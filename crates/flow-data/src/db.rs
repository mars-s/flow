//! Flow's embedded local store: Turso on a dedicated tokio runtime, isolated
//! from GPUI's smol-backed executor the same way `analytics.rs` isolates its
//! own tokio runtime on a background OS thread. See `docs/turso.md` for the
//! API research this module is built against, including the open questions
//! it flags — several of the choices below exist specifically to sidestep
//! them rather than assume an answer.
//!
//! The runtime is `current_thread`, and every `block_on` call happens on the
//! one dedicated thread that owns the `Connection` — so the connection never
//! crosses threads and its `Send`/`Sync` bounds (undocumented upstream, per
//! `docs/turso.md` §6) never come into question here.
//!
//! `Db::open` blocks the calling thread until the connection is ready, and
//! every method on `Db` blocks the calling thread for its reply. Only call
//! these from `cx.background_executor().spawn`, never from a render path.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use turso::Row;
use uuid::Uuid;

/// The three persisted placements from `docs/PRODUCT_REQUIREMENTS.md` §5.
/// Today/Upcoming/Anytime are computed views over `Active`, not stored.
///
/// `Serialize`/`Deserialize` here (and on `View`/`Task` below) exist for the
/// Tauri migration's IPC boundary (`wayfinder/tickets/migrate-to-tauri.md`)
/// — the GPUI app never serializes these, so this costs it nothing.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Bucket {
    Inbox,
    Active,
    Someday,
}

impl Bucket {
    fn as_str(self) -> &'static str {
        match self {
            Bucket::Inbox => "inbox",
            Bucket::Active => "active",
            Bucket::Someday => "someday",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "inbox" => Ok(Bucket::Inbox),
            "active" => Ok(Bucket::Active),
            "someday" => Ok(Bucket::Someday),
            other => Err(anyhow!("unknown bucket: {other}")),
        }
    }
}

/// The five task surfaces from `docs/PRODUCT_REQUIREMENTS.md` §5/§6.3 — the
/// UI-facing address for "what should this view show," distinct from
/// `Bucket`, the storage placement underneath. Inbox and Someday map to one
/// bucket each; Today/Upcoming/Anytime all read `Bucket::Active`, sliced by
/// `scheduled_date`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum View {
    Inbox,
    Today,
    Upcoming,
    Anytime,
    Someday,
}

/// Mirrors the `tasks` table from `docs/PRODUCT_REQUIREMENTS.md` §8. No
/// `user_id`: Flow's local phase is single-user, so a `users` table would be
/// speculative multi-tenancy nothing here needs yet.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub note: Option<String>,
    pub bucket: Bucket,
    pub scheduled_date: Option<String>,
    pub scheduled_time: Option<String>,
    pub scheduled_timezone: Option<String>,
    pub position: f64,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Task {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row.get::<String>(0)?,
            parent_id: row.get::<Option<String>>(1)?,
            title: row.get::<String>(2)?,
            note: row.get::<Option<String>>(3)?,
            bucket: Bucket::parse(&row.get::<String>(4)?)?,
            scheduled_date: row.get::<Option<String>>(5)?,
            scheduled_time: row.get::<Option<String>>(6)?,
            scheduled_timezone: row.get::<Option<String>>(7)?,
            position: row.get::<f64>(8)?,
            completed_at: row.get::<Option<String>>(9)?,
            created_at: row.get::<String>(10)?,
            updated_at: row.get::<String>(11)?,
        })
    }
}

const TASK_COLUMNS: &str = "id, parent_id, title, note, bucket, scheduled_date, \
    scheduled_time, scheduled_timezone, position, completed_at, created_at, updated_at";

/// Applied in order, tracked by a single-row `schema_version` table. Add new
/// entries to the end; never edit an already-shipped one.
const MIGRATIONS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS tasks (
        id TEXT PRIMARY KEY,
        parent_id TEXT REFERENCES tasks(id),
        title TEXT NOT NULL,
        note TEXT,
        bucket TEXT NOT NULL,
        scheduled_date TEXT,
        scheduled_time TEXT,
        scheduled_timezone TEXT,
        position REAL NOT NULL,
        completed_at TEXT,
        deleted_at TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS tasks_bucket_idx ON tasks(bucket)",
    "CREATE INDEX IF NOT EXISTS tasks_parent_idx ON tasks(parent_id)",
];

enum Command {
    Ping {
        reply: mpsc::Sender<Result<bool>>,
    },
    CreateTask {
        title: String,
        reply: mpsc::Sender<Result<Task>>,
    },
    CreateTaskScheduled {
        title: String,
        scheduled_date: Option<String>,
        scheduled_time: Option<String>,
        reply: mpsc::Sender<Result<Task>>,
    },
    ListView {
        view: View,
        reply: mpsc::Sender<Result<Vec<Task>>>,
    },
    ListCompleted {
        view: View,
        reply: mpsc::Sender<Result<Vec<Task>>>,
    },
    SetCompleted {
        id: String,
        completed: bool,
        reply: mpsc::Sender<Result<()>>,
    },
    Schedule {
        id: String,
        bucket: Bucket,
        scheduled_date: Option<String>,
        scheduled_time: Option<String>,
        reply: mpsc::Sender<Result<()>>,
    },
    SetNote {
        id: String,
        note: Option<String>,
        reply: mpsc::Sender<Result<()>>,
    },
    SetTitle {
        id: String,
        title: String,
        reply: mpsc::Sender<Result<()>>,
    },
    DeleteTask {
        id: String,
        reply: mpsc::Sender<Result<()>>,
    },
    RestoreTask {
        id: String,
        reply: mpsc::Sender<Result<()>>,
    },
    CreateSubtask {
        parent_id: String,
        title: String,
        reply: mpsc::Sender<Result<Task>>,
    },
    ListSubtasks {
        parent_id: String,
        reply: mpsc::Sender<Result<Vec<Task>>>,
    },
}

/// A cheaply cloneable handle to the database thread.
#[derive(Clone)]
pub struct Db {
    commands: mpsc::Sender<Command>,
}

impl Db {
    /// Open (or create) the local database file in Flow's app data
    /// directory, spawn the dedicated thread that owns it, and apply any
    /// pending migrations. Blocks until ready.
    pub fn open() -> Result<Self> {
        Self::open_at(database_path()?)
    }

    fn open_at(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }

        let (commands_tx, commands_rx) = mpsc::channel::<Command>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<()>>();

        thread::Builder::new()
            .name("flow-db".into())
            .spawn(move || run(path, commands_rx, ready_tx))
            .context("spawning the database thread")?;

        ready_rx
            .recv()
            .context("database thread exited before it was ready")??;

        Ok(Self {
            commands: commands_tx,
        })
    }

    /// `SELECT 1`, to prove the connection works.
    pub fn ping(&self) -> Result<bool> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::Ping { reply })
            .context("database thread is gone")?;
        rx.recv().context("database thread dropped the reply")?
    }

    /// Create a new Inbox task with just a title. Milestone 1's capture flow
    /// starts here; note/schedule/parent arrive once the composer and NLP
    /// parser exist.
    pub fn create_task(&self, title: impl Into<String>) -> Result<Task> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::CreateTask {
                title: title.into(),
                reply,
            })
            .context("database thread is gone")?;
        rx.recv().context("database thread dropped the reply")?
    }

    /// Create a task and schedule it in one atomic step — Capture's own
    /// write path when a phrase parses a date/time. See
    /// `create_task_scheduled`'s own doc for why this needs to be atomic
    /// rather than two separate `create_task`/`schedule` calls.
    pub fn create_task_scheduled(
        &self,
        title: impl Into<String>,
        scheduled_date: Option<impl Into<String>>,
        scheduled_time: Option<impl Into<String>>,
    ) -> Result<Task> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::CreateTaskScheduled {
                title: title.into(),
                scheduled_date: scheduled_date.map(Into::into),
                scheduled_time: scheduled_time.map(Into::into),
                reply,
            })
            .context("database thread is gone")?;
        rx.recv().context("database thread dropped the reply")?
    }

    /// Open, undeleted, uncompleted tasks for one of the five task views.
    pub fn list_view(&self, view: View) -> Result<Vec<Task>> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::ListView { view, reply })
            .context("database thread is gone")?;
        rx.recv().context("database thread dropped the reply")?
    }

    /// Completed tasks for one of the five task views — the same
    /// bucket/date-range slice `list_view` uses, but `completed_at IS NOT
    /// NULL` and newest-completed-first, for the collapsed "Completed"
    /// section at the bottom of each view rather than a shared logbook.
    pub fn list_completed(&self, view: View) -> Result<Vec<Task>> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::ListCompleted { view, reply })
            .context("database thread is gone")?;
        rx.recv().context("database thread dropped the reply")?
    }

    /// Complete or reopen a task.
    pub fn set_completed(&self, id: impl Into<String>, completed: bool) -> Result<()> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::SetCompleted {
                id: id.into(),
                completed,
                reply,
            })
            .context("database thread is gone")?;
        rx.recv().context("database thread dropped the reply")?
    }

    /// Moves a task into `bucket` with an optional schedule — the "Move to
    /// active" / "Schedule and activate" actions from
    /// `docs/PRODUCT_REQUIREMENTS.md` §5. `scheduled_time` without
    /// `scheduled_date` is nonsensical per §8's constraints; callers must
    /// clear both together when clearing a schedule.
    pub fn schedule(
        &self,
        id: impl Into<String>,
        bucket: Bucket,
        scheduled_date: Option<impl Into<String>>,
        scheduled_time: Option<impl Into<String>>,
    ) -> Result<()> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::Schedule {
                id: id.into(),
                bucket,
                scheduled_date: scheduled_date.map(Into::into),
                scheduled_time: scheduled_time.map(Into::into),
                reply,
            })
            .context("database thread is gone")?;
        rx.recv().context("database thread dropped the reply")?
    }

    /// Sets or clears a task's note. `None` clears it.
    pub fn set_note(&self, id: impl Into<String>, note: Option<impl Into<String>>) -> Result<()> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::SetNote {
                id: id.into(),
                note: note.map(Into::into),
                reply,
            })
            .context("database thread is gone")?;
        rx.recv().context("database thread dropped the reply")?
    }

    /// Renames a task. PRD §6.1's "required nonempty title" is enforced by
    /// the caller (the composer field this backs never submits blank), not
    /// re-checked here — matches `create_task`'s own division of labor.
    pub fn set_title(&self, id: impl Into<String>, title: impl Into<String>) -> Result<()> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::SetTitle {
                id: id.into(),
                title: title.into(),
                reply,
            })
            .context("database thread is gone")?;
        rx.recv().context("database thread dropped the reply")?
    }

    /// Soft-deletes a task by stamping `deleted_at`. Every `list_view`/
    /// `list_bucket` query already filters `deleted_at IS NULL`, so a
    /// deleted task disappears from every view without a separate query
    /// change.
    pub fn delete_task(&self, id: impl Into<String>) -> Result<()> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::DeleteTask {
                id: id.into(),
                reply,
            })
            .context("database thread is gone")?;
        rx.recv().context("database thread dropped the reply")?
    }

    /// Clears `deleted_at`, the undo half of `delete_task` — PRD §6.1:
    /// "Deletion shows an undo toast for 10 seconds; storage uses a
    /// soft-delete timestamp until a future permanent-delete policy
    /// exists." A soft-deleted row was never actually removed, so undo is
    /// just clearing the same column `delete_task` set.
    pub fn restore_task(&self, id: impl Into<String>) -> Result<()> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::RestoreTask {
                id: id.into(),
                reply,
            })
            .context("database thread is gone")?;
        rx.recv().context("database thread dropped the reply")?
    }

    /// Adds a subtask under `parent_id`. Rejects a parent that is itself a
    /// subtask — `docs/PRODUCT_REQUIREMENTS.md` §6.2's one-level ceiling
    /// ("a subtask cannot have children in v1"), enforced here rather than
    /// only in the UI since the UI already hides the affordance but a
    /// caller shouldn't be able to bypass it. Inherits the parent's bucket
    /// (a subtask has no independent placement of its own) and no
    /// schedule, per the same section: "a child inherits no schedule
    /// automatically."
    pub fn create_subtask(&self, parent_id: impl Into<String>, title: impl Into<String>) -> Result<Task> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::CreateSubtask {
                parent_id: parent_id.into(),
                title: title.into(),
                reply,
            })
            .context("database thread is gone")?;
        rx.recv().context("database thread dropped the reply")?
    }

    /// A parent's direct subtasks, in manual order. Unlike `list_view`, this
    /// does not filter out completed ones — a subtask stays visible
    /// (checked off) under its expanded parent rather than moving to a
    /// separate collapsed section, since the parent's own progress count
    /// needs the completed ones counted, not hidden.
    pub fn list_subtasks(&self, parent_id: impl Into<String>) -> Result<Vec<Task>> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::ListSubtasks {
                parent_id: parent_id.into(),
                reply,
            })
            .context("database thread is gone")?;
        rx.recv().context("database thread dropped the reply")?
    }
}

fn database_path() -> Result<PathBuf> {
    let base = dirs::data_dir().context("no platform data directory")?;
    Ok(base
        .join(flow_core::identity::DATA_DIRECTORY_NAME)
        .join("flow.db"))
}

/// The database thread's body: one tokio runtime, one Turso connection,
/// commands processed one at a time. Flow is single-user and single-window,
/// so sequential processing is enough for now — Turso's engine supports
/// concurrent writers (MVCC) if that ever becomes a real bottleneck.
fn run(path: PathBuf, commands: mpsc::Receiver<Command>, ready: mpsc::Sender<Result<()>>) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready.send(Err(error).context("building the database's tokio runtime"));
            return;
        }
    };

    let conn = runtime.block_on(open_connection(&path));
    let conn = match conn {
        Ok(conn) => conn,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };

    let _ = ready.send(Ok(()));

    while let Ok(command) = commands.recv() {
        match command {
            Command::Ping { reply } => {
                let _ = reply.send(runtime.block_on(ping(&conn)));
            }
            Command::CreateTask { title, reply } => {
                let _ = reply.send(runtime.block_on(create_task(&conn, title)));
            }
            Command::CreateTaskScheduled { title, scheduled_date, scheduled_time, reply } => {
                let _ = reply.send(runtime.block_on(create_task_scheduled(
                    &conn,
                    title,
                    scheduled_date,
                    scheduled_time,
                )));
            }
            Command::ListView { view, reply } => {
                let _ = reply.send(runtime.block_on(list_view(&conn, view)));
            }
            Command::ListCompleted { view, reply } => {
                let _ = reply.send(runtime.block_on(list_completed(&conn, view)));
            }
            Command::SetCompleted {
                id,
                completed,
                reply,
            } => {
                let _ = reply.send(runtime.block_on(set_completed(&conn, id, completed)));
            }
            Command::Schedule {
                id,
                bucket,
                scheduled_date,
                scheduled_time,
                reply,
            } => {
                let _ = reply.send(runtime.block_on(schedule(
                    &conn,
                    id,
                    bucket,
                    scheduled_date,
                    scheduled_time,
                )));
            }
            Command::SetNote { id, note, reply } => {
                let _ = reply.send(runtime.block_on(set_note(&conn, id, note)));
            }
            Command::SetTitle { id, title, reply } => {
                let _ = reply.send(runtime.block_on(set_title(&conn, id, title)));
            }
            Command::DeleteTask { id, reply } => {
                let _ = reply.send(runtime.block_on(delete_task(&conn, id)));
            }
            Command::RestoreTask { id, reply } => {
                let _ = reply.send(runtime.block_on(restore_task(&conn, id)));
            }
            Command::CreateSubtask {
                parent_id,
                title,
                reply,
            } => {
                let _ = reply.send(runtime.block_on(create_subtask(&conn, parent_id, title)));
            }
            Command::ListSubtasks { parent_id, reply } => {
                let _ = reply.send(runtime.block_on(list_subtasks(&conn, parent_id)));
            }
        }
    }
}

async fn open_connection(path: &Path) -> Result<turso::Connection> {
    let db = turso::Builder::new_local(&path.to_string_lossy())
        .build()
        .await
        .with_context(|| format!("opening {}", path.display()))?;
    let conn = db.connect().context("opening a database connection")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)",
        (),
    )
    .await
    .context("creating schema_version")?;

    let mut rows = conn
        .query("SELECT version FROM schema_version LIMIT 1", ())
        .await
        .context("reading schema_version")?;
    let current_version = match rows.next().await.context("reading schema_version row")? {
        Some(row) => row.get::<i64>(0).context("reading version column")?,
        None => {
            conn.execute("INSERT INTO schema_version (version) VALUES (0)", ())
                .await
                .context("seeding schema_version")?;
            0
        }
    };
    drop(rows);

    for (index, migration) in MIGRATIONS.iter().enumerate() {
        let version = (index + 1) as i64;
        if version <= current_version {
            continue;
        }
        conn.execute(*migration, ())
            .await
            .with_context(|| format!("applying migration {version}"))?;
        conn.execute("UPDATE schema_version SET version = ?1", (version,))
            .await
            .with_context(|| format!("recording migration {version}"))?;
    }

    Ok(conn)
}

async fn ping(conn: &turso::Connection) -> Result<bool> {
    let mut rows = conn.query("SELECT 1", ()).await.context("querying")?;
    Ok(rows.next().await.context("reading a row")?.is_some())
}

async fn create_task(conn: &turso::Connection, title: String) -> Result<Task> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let position = Utc::now().timestamp_millis() as f64;

    conn.execute(
        "INSERT INTO tasks (id, title, bucket, position, created_at, updated_at) \
         VALUES (?1, ?2, 'inbox', ?3, ?4, ?4)",
        (id.as_str(), title.as_str(), position, now.as_str()),
    )
    .await
    .context("inserting task")?;

    Ok(Task {
        id,
        parent_id: None,
        title,
        note: None,
        bucket: Bucket::Inbox,
        scheduled_date: None,
        scheduled_time: None,
        scheduled_timezone: None,
        position,
        completed_at: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// Create-then-schedule as one transaction — Capture's own write path.
/// Without this, a title that parses a schedule (`create_task` succeeds,
/// the follow-up `schedule` call fails for any reason) left a real,
/// already-committed task sitting unscheduled in Inbox while the caller
/// still reported the whole capture as failed. The UI's own failure path
/// restores the typed title and offers Retry — which would then create a
/// second, genuinely duplicate task on top of the first one nobody knew
/// existed. `BEGIN`/`COMMIT`/`ROLLBACK` make the two writes atomic: a
/// `schedule` failure now rolls the `create_task` back too, so a reported
/// failure means nothing was saved, matching what the UI already tells
/// the user. Found via a PRD §10 idempotency audit, not a user report —
/// this is the actual live duplicate-creation path that audit was
/// initially (wrongly) dismissed as not existing yet; see
/// `docs/HANDOFF.md` for the full story.
async fn create_task_scheduled(
    conn: &turso::Connection,
    title: String,
    scheduled_date: Option<String>,
    scheduled_time: Option<String>,
) -> Result<Task> {
    let has_schedule = scheduled_date.is_some() || scheduled_time.is_some();
    if !has_schedule {
        // Nothing to make atomic with — skip the transaction wrapper
        // entirely for the common unscheduled-capture case.
        return create_task(conn, title).await;
    }
    conn.execute("BEGIN", ()).await.context("beginning capture transaction")?;
    let result = async {
        let task = create_task(conn, title).await?;
        schedule(
            conn,
            task.id.clone(),
            Bucket::Active,
            scheduled_date.clone(),
            scheduled_time.clone(),
        )
        .await?;
        Ok(Task { bucket: Bucket::Active, scheduled_date, scheduled_time, ..task })
    }
    .await;
    match result {
        Ok(task) => {
            conn.execute("COMMIT", ()).await.context("committing capture transaction")?;
            Ok(task)
        }
        Err(error) => {
            // Best-effort: if the rollback itself fails there is nothing
            // more this function can do about it, and reporting the
            // original error is more useful than a rollback-failure one.
            let _ = conn.execute("ROLLBACK", ()).await;
            Err(error)
        }
    }
}

/// Open, undeleted, uncompleted tasks in one bucket, ordered by position.
/// Completed items are hidden from primary views by default per
/// `docs/PRODUCT_REQUIREMENTS.md` §5 — the future Logbook view will query
/// completed tasks separately rather than this function gaining a flag.
async fn list_bucket(conn: &turso::Connection, bucket: Bucket) -> Result<Vec<Task>> {
    let sql = format!(
        "SELECT {TASK_COLUMNS} FROM tasks \
         WHERE bucket = ?1 AND parent_id IS NULL \
         AND deleted_at IS NULL AND completed_at IS NULL \
         ORDER BY position ASC"
    );
    run_task_query(conn, &sql, (bucket.as_str(),)).await
}

/// Dispatches to the SQL for each of the five task views, per
/// `docs/PRODUCT_REQUIREMENTS.md` §5's placement table. Today/Upcoming/
/// Anytime all read `Bucket::Active`, sliced by `scheduled_date` against
/// the caller's local "today" — computed here rather than accepted as a
/// parameter, since every caller wants "right now" and a stale date would
/// be a bug, not a feature, for a single-window desktop app.
async fn list_view(conn: &turso::Connection, view: View) -> Result<Vec<Task>> {
    match view {
        View::Inbox => list_bucket(conn, Bucket::Inbox).await,
        View::Someday => list_bucket(conn, Bucket::Someday).await,
        View::Anytime => {
            let sql = format!(
                "SELECT {TASK_COLUMNS} FROM tasks \
                 WHERE bucket = 'active' AND scheduled_date IS NULL AND parent_id IS NULL \
                 AND deleted_at IS NULL AND completed_at IS NULL \
                 ORDER BY position ASC"
            );
            run_task_query(conn, &sql, ()).await
        }
        View::Today => {
            let today = chrono::Local::now().date_naive().to_string();
            let sql = format!(
                "SELECT {TASK_COLUMNS} FROM tasks \
                 WHERE bucket = 'active' AND scheduled_date IS NOT NULL \
                 AND scheduled_date <= ?1 AND parent_id IS NULL \
                 AND deleted_at IS NULL AND completed_at IS NULL \
                 ORDER BY scheduled_date ASC, scheduled_time ASC"
            );
            run_task_query(conn, &sql, (today,)).await
        }
        View::Upcoming => {
            let today = chrono::Local::now().date_naive().to_string();
            let sql = format!(
                "SELECT {TASK_COLUMNS} FROM tasks \
                 WHERE bucket = 'active' AND scheduled_date IS NOT NULL \
                 AND scheduled_date > ?1 AND parent_id IS NULL \
                 AND deleted_at IS NULL AND completed_at IS NULL \
                 ORDER BY scheduled_date ASC, scheduled_time ASC"
            );
            run_task_query(conn, &sql, (today,)).await
        }
    }
}

/// Mirrors `list_view`'s per-view bucket/date-range slice, but for completed
/// tasks (`completed_at IS NOT NULL`) ordered most-recently-completed-first —
/// `position` is meaningless once a task is done and out of the active
/// ordering it was set for.
async fn list_completed(conn: &turso::Connection, view: View) -> Result<Vec<Task>> {
    let bucket = match view {
        View::Inbox => Bucket::Inbox,
        View::Someday => Bucket::Someday,
        View::Today | View::Upcoming | View::Anytime => Bucket::Active,
    };
    let sql = match view {
        View::Inbox | View::Someday => format!(
            "SELECT {TASK_COLUMNS} FROM tasks \
             WHERE bucket = ?1 AND parent_id IS NULL \
             AND deleted_at IS NULL AND completed_at IS NOT NULL \
             ORDER BY updated_at DESC"
        ),
        View::Anytime => format!(
            "SELECT {TASK_COLUMNS} FROM tasks \
             WHERE bucket = ?1 AND scheduled_date IS NULL AND parent_id IS NULL \
             AND deleted_at IS NULL AND completed_at IS NOT NULL \
             ORDER BY updated_at DESC"
        ),
        View::Today => format!(
            "SELECT {TASK_COLUMNS} FROM tasks \
             WHERE bucket = ?1 AND scheduled_date IS NOT NULL AND scheduled_date <= ?2 \
             AND parent_id IS NULL \
             AND deleted_at IS NULL AND completed_at IS NOT NULL \
             ORDER BY updated_at DESC"
        ),
        View::Upcoming => format!(
            "SELECT {TASK_COLUMNS} FROM tasks \
             WHERE bucket = ?1 AND scheduled_date IS NOT NULL AND scheduled_date > ?2 \
             AND parent_id IS NULL \
             AND deleted_at IS NULL AND completed_at IS NOT NULL \
             ORDER BY updated_at DESC"
        ),
    };
    match view {
        View::Inbox | View::Someday | View::Anytime => {
            run_task_query(conn, &sql, (bucket.as_str(),)).await
        }
        View::Today | View::Upcoming => {
            let today = chrono::Local::now().date_naive().to_string();
            run_task_query(conn, &sql, (bucket.as_str(), today)).await
        }
    }
}

async fn run_task_query(
    conn: &turso::Connection,
    sql: &str,
    params: impl turso::IntoParams,
) -> Result<Vec<Task>> {
    let mut rows = conn.query(sql, params).await.context("listing tasks")?;
    let mut tasks = Vec::new();
    while let Some(row) = rows.next().await.context("reading a task row")? {
        tasks.push(Task::from_row(&row)?);
    }
    Ok(tasks)
}

async fn set_completed(conn: &turso::Connection, id: String, completed: bool) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let completed_at = completed.then(|| now.clone());
    conn.execute(
        "UPDATE tasks SET completed_at = ?1, updated_at = ?2 WHERE id = ?3",
        (completed_at, now, id),
    )
    .await
    .context("updating completion")?;
    Ok(())
}

async fn schedule(
    conn: &turso::Connection,
    id: String,
    bucket: Bucket,
    scheduled_date: Option<String>,
    scheduled_time: Option<String>,
) -> Result<()> {
    // PRD §8: "scheduled_time requires scheduled_date." Real, not
    // theoretical — parse.rs's TIME_ONLY pattern ("call mom at 3pm", no
    // date phrase) produces exactly `date: None, time: Some(_)`, and
    // Capture's own submit path passes that straight through with no
    // guard before this fix. Rejecting it here (not just at the call
    // site) covers every path that reaches `schedule`, present or future
    // — the same "root cause, not symptom" reasoning `create_subtask`'s
    // own parent-relationship checks already follow.
    if scheduled_time.is_some() && scheduled_date.is_none() {
        return Err(anyhow!(
            "a scheduled time requires a scheduled date (PRD §8)"
        ));
    }
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE tasks SET bucket = ?1, scheduled_date = ?2, scheduled_time = ?3, \
         updated_at = ?4 WHERE id = ?5",
        (bucket.as_str(), scheduled_date, scheduled_time, now, id),
    )
    .await
    .context("scheduling task")?;
    Ok(())
}

async fn set_note(conn: &turso::Connection, id: String, note: Option<String>) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE tasks SET note = ?1, updated_at = ?2 WHERE id = ?3",
        (note, now, id),
    )
    .await
    .context("updating note")?;
    Ok(())
}

async fn set_title(conn: &turso::Connection, id: String, title: String) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE tasks SET title = ?1, updated_at = ?2 WHERE id = ?3",
        (title, now, id),
    )
    .await
    .context("updating title")?;
    Ok(())
}

async fn delete_task(conn: &turso::Connection, id: String) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    // Both statements are one transaction — same reasoning as
    // create_task_scheduled's own atomicity fix a few commits ago: without
    // it, the parent's own UPDATE landing while the subtask-cascade UPDATE
    // fails would recreate exactly the orphaning bug this cascade exists to
    // fix, just narrowed to a rarer failure window (two same-connection
    // UPDATEs back to back, not a whole created-task validation). Found
    // via a self-review of that exact fix, not a new report.
    conn.execute("BEGIN", ()).await.context("beginning delete transaction")?;
    let result = async {
        conn.execute(
            "UPDATE tasks SET deleted_at = ?1, updated_at = ?2 WHERE id = ?3",
            (now.clone(), now.clone(), id.clone()),
        )
        .await
        .context("deleting task")?;
        // Cascades to subtasks — without this, a parent with open subtasks
        // just vanished from every view (list_view always filters
        // `parent_id IS NULL`, so a subtask never appears independently)
        // while its subtask rows stayed `deleted_at IS NULL` in the
        // database forever: not shown anywhere, not actually deleted,
        // unreachable by any UI path (the only way to see a subtask is
        // through its parent's own detail card, which can never open
        // again once the parent's gone). Found via a data-integrity
        // audit, not a user report.
        conn.execute(
            "UPDATE tasks SET deleted_at = ?1, updated_at = ?2 \
             WHERE parent_id = ?3 AND deleted_at IS NULL",
            (now.clone(), now, id),
        )
        .await
        .context("deleting subtasks")
    }
    .await;
    match result {
        Ok(_) => {
            conn.execute("COMMIT", ()).await.context("committing delete transaction")?;
            Ok(())
        }
        Err(error) => {
            let _ = conn.execute("ROLLBACK", ()).await;
            Err(error)
        }
    }
}

async fn restore_task(conn: &turso::Connection, id: String) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    // Same transaction-wrapping reasoning as delete_task's own cascade.
    conn.execute("BEGIN", ()).await.context("beginning restore transaction")?;
    let result = async {
        conn.execute(
            "UPDATE tasks SET deleted_at = NULL, updated_at = ?1 WHERE id = ?2",
            (now.clone(), id.clone()),
        )
        .await
        .context("restoring task")?;
        // Symmetric with delete_task's own cascade: Undo on a parent that
        // had subtasks must bring the whole family back, not just the
        // parent — otherwise the subtasks a moment ago would come back as
        // gone-but-not-deleted zombies the instant Undo's own 10-second
        // window expired, an inconsistent state relative to what the user
        // just clicked Undo to reverse. Unconditional (no deleted_at-
        // timestamp match against the parent's own delete) is safe only
        // because there is currently no UI path to delete a single
        // subtask independently of its parent — every non-NULL subtask
        // deleted_at was set by this exact cascade. If an independent
        // subtask-delete is ever added, this needs to stop being
        // unconditional or it will incorrectly resurrect an unrelated
        // deletion.
        conn.execute(
            "UPDATE tasks SET deleted_at = NULL, updated_at = ?1 WHERE parent_id = ?2",
            (now, id),
        )
        .await
        .context("restoring subtasks")
    }
    .await;
    match result {
        Ok(_) => {
            conn.execute("COMMIT", ()).await.context("committing restore transaction")?;
            Ok(())
        }
        Err(error) => {
            let _ = conn.execute("ROLLBACK", ()).await;
            Err(error)
        }
    }
}

async fn create_subtask(conn: &turso::Connection, parent_id: String, title: String) -> Result<Task> {
    // PRD §8: "A deleted or completed parent cannot accept a new open
    // subtask" — both halves enforced here, not just the deleted one.
    let mut parent_rows = conn
        .query(
            "SELECT bucket, parent_id FROM tasks \
             WHERE id = ?1 AND deleted_at IS NULL AND completed_at IS NULL",
            (parent_id.as_str(),),
        )
        .await
        .context("looking up the parent task")?;
    let parent_row = parent_rows
        .next()
        .await
        .context("reading the parent task row")?
        .ok_or_else(|| anyhow!("parent task not found, deleted, or completed"))?;
    let parent_bucket = Bucket::parse(&parent_row.get::<String>(0)?)?;
    if parent_row.get::<Option<String>>(1)?.is_some() {
        return Err(anyhow!(
            "cannot add a subtask to a subtask (one-level ceiling, PRD §6.2)"
        ));
    }
    drop(parent_rows);

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let position = Utc::now().timestamp_millis() as f64;

    conn.execute(
        "INSERT INTO tasks (id, parent_id, title, bucket, position, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        (
            id.as_str(),
            parent_id.as_str(),
            title.as_str(),
            parent_bucket.as_str(),
            position,
            now.as_str(),
        ),
    )
    .await
    .context("inserting subtask")?;

    Ok(Task {
        id,
        parent_id: Some(parent_id),
        title,
        note: None,
        bucket: parent_bucket,
        scheduled_date: None,
        scheduled_time: None,
        scheduled_timezone: None,
        position,
        completed_at: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// A parent's direct subtasks in manual order — completed ones included
/// (see `Db::list_subtasks`'s doc comment for why).
async fn list_subtasks(conn: &turso::Connection, parent_id: String) -> Result<Vec<Task>> {
    let sql = format!(
        "SELECT {TASK_COLUMNS} FROM tasks \
         WHERE parent_id = ?1 AND deleted_at IS NULL \
         ORDER BY position ASC"
    );
    run_task_query(conn, &sql, (parent_id,)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_test_db() -> Db {
        let dir = std::env::temp_dir().join(format!("flow-db-test-{}", Uuid::new_v4()));
        Db::open_at(dir.join("flow.db")).expect("database should open")
    }

    #[test]
    fn opens_a_database_and_answers_a_ping() {
        let db = open_test_db();
        assert!(db.ping().expect("ping should succeed"));
    }

    #[test]
    fn creates_and_lists_an_inbox_task() {
        let db = open_test_db();
        let created = db
            .create_task("Take out laundry")
            .expect("create should succeed");
        assert_eq!(created.bucket, Bucket::Inbox);
        assert!(created.completed_at.is_none());

        let inbox = db.list_view(View::Inbox).expect("list should succeed");
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].id, created.id);
        assert_eq!(inbox[0].title, "Take out laundry");
    }

    #[test]
    fn completing_a_task_removes_it_from_its_open_listing() {
        let db = open_test_db();
        let created = db.create_task("Ship it").expect("create should succeed");

        db.set_completed(&created.id, true)
            .expect("complete should succeed");
        let inbox = db.list_view(View::Inbox).expect("list should succeed");
        assert!(
            inbox.is_empty(),
            "completed tasks should be hidden from primary views by default"
        );

        db.set_completed(&created.id, false)
            .expect("reopen should succeed");
        let inbox = db.list_view(View::Inbox).expect("list should succeed");
        assert_eq!(inbox.len(), 1);
        assert!(inbox[0].completed_at.is_none());
    }

    #[test]
    fn scheduling_moves_a_task_between_views() {
        let db = open_test_db();
        let task = db.create_task("Bring Mya cake").expect("create");

        // Active + no date → Anytime only.
        db.schedule(&task.id, Bucket::Active, None::<String>, None::<String>)
            .expect("schedule");
        assert_eq!(db.list_view(View::Inbox).expect("list").len(), 0);
        assert_eq!(db.list_view(View::Anytime).expect("list").len(), 1);
        assert_eq!(db.list_view(View::Today).expect("list").len(), 0);
        assert_eq!(db.list_view(View::Upcoming).expect("list").len(), 0);

        // Active + today's date → Today, not Anytime or Upcoming.
        let today = chrono::Local::now().date_naive().to_string();
        db.schedule(&task.id, Bucket::Active, Some(today), None::<String>)
            .expect("schedule");
        assert_eq!(db.list_view(View::Today).expect("list").len(), 1);
        assert_eq!(db.list_view(View::Anytime).expect("list").len(), 0);
        assert_eq!(db.list_view(View::Upcoming).expect("list").len(), 0);

        // Active + a future date → Upcoming, not Today.
        let next_year = (chrono::Local::now().date_naive() + chrono::Days::new(365)).to_string();
        db.schedule(&task.id, Bucket::Active, Some(next_year), None::<String>)
            .expect("schedule");
        assert_eq!(db.list_view(View::Upcoming).expect("list").len(), 1);
        assert_eq!(db.list_view(View::Today).expect("list").len(), 0);

        // Clearing the date (same bucket, date/time both None) drops it
        // back to Anytime — the "clear schedule" action's underlying write.
        db.schedule(&task.id, Bucket::Active, None::<String>, None::<String>)
            .expect("schedule");
        assert_eq!(db.list_view(View::Anytime).expect("list").len(), 1);
        assert_eq!(db.list_view(View::Upcoming).expect("list").len(), 0);
        assert!(db.list_view(View::Anytime).expect("list")[0].scheduled_date.is_none());
    }

    #[test]
    fn scheduling_a_time_with_no_date_is_rejected() {
        // PRD §8: "scheduled_time requires scheduled_date." Real path, not
        // theoretical: parse.rs's TIME_ONLY pattern ("call mom at 3pm", no
        // date phrase) used to produce exactly this combination with no
        // guard anywhere before it reached the database.
        let db = open_test_db();
        let task = db.create_task("Call mom").expect("create");
        let result = db.schedule(&task.id, Bucket::Active, None::<String>, Some("15:00"));
        assert!(result.is_err(), "a time with no date should be rejected");
    }

    #[test]
    fn create_task_scheduled_is_atomic() {
        // Found via a PRD §10 idempotency audit: submit_capture used to do
        // create_task then schedule as two separate calls. If schedule
        // failed after create_task already landed, the caller reported the
        // whole capture as failed (and restored the typed title for
        // Retry) while a real, already-committed task sat unscheduled in
        // Inbox — invisible to the user, and duplicated on the next Retry.
        // create_task_scheduled wraps both in a transaction so a schedule
        // failure rolls the create back too: a reported failure now
        // really means nothing was saved.
        let db = open_test_db();
        let before = db.list_view(View::Inbox).expect("list").len();

        // A time with no date is a guaranteed schedule() failure (see the
        // test above) — the exact combination that used to leak a task.
        let result = db.create_task_scheduled("Call mom", None::<String>, Some("15:00"));
        assert!(result.is_err(), "the whole operation should fail");

        let after = db.list_view(View::Inbox).expect("list").len();
        assert_eq!(before, after, "a failed schedule should not leave an orphaned task behind");
    }

    #[test]
    fn create_task_scheduled_activates_the_task_when_it_succeeds() {
        let db = open_test_db();
        let today = chrono::Local::now().date_naive().to_string();
        let task = db
            .create_task_scheduled("Take out laundry", Some(today.clone()), Some("08:00"))
            .expect("create_task_scheduled should succeed");
        assert_eq!(task.bucket, Bucket::Active);
        assert_eq!(task.scheduled_date.as_deref(), Some(today.as_str()));
        assert_eq!(task.scheduled_time.as_deref(), Some("08:00"));

        let today_view = db.list_view(View::Today).expect("list");
        assert_eq!(today_view.len(), 1);
        assert_eq!(today_view[0].id, task.id);
    }

    #[test]
    fn someday_tasks_are_isolated_from_other_views() {
        let db = open_test_db();
        let task = db.create_task("Learn pottery").expect("create");
        db.schedule(&task.id, Bucket::Someday, None::<String>, None::<String>)
            .expect("schedule");

        assert_eq!(db.list_view(View::Someday).expect("list").len(), 1);
        assert_eq!(db.list_view(View::Inbox).expect("list").len(), 0);
        assert_eq!(db.list_view(View::Anytime).expect("list").len(), 0);
    }

    /// PRD §14: a parsed date attached to a captured task does not
    /// activate it — it stays in Inbox as a review queue, just with its
    /// schedule already attached. This is exactly what
    /// `Flow::on_capture_event` (`app.rs`) relies on: it calls
    /// `schedule(id, Bucket::Inbox, ...)` after `create_task`, never
    /// `Bucket::Active`.
    #[test]
    fn a_scheduled_inbox_task_stays_in_inbox_not_today() {
        let db = open_test_db();
        let task = db.create_task("Bring Mya cake").expect("create");
        let today = chrono::Local::now().date_naive().to_string();

        db.schedule(&task.id, Bucket::Inbox, Some(today), None::<String>)
            .expect("schedule");

        let inbox = db.list_view(View::Inbox).expect("list");
        assert_eq!(inbox.len(), 1);
        assert!(inbox[0].scheduled_date.is_some());
        assert_eq!(db.list_view(View::Today).expect("list").len(), 0);
    }

    #[test]
    fn set_note_updates_and_clears_a_tasks_note() {
        let db = open_test_db();
        let task = db.create_task("Write notes").expect("create");
        assert!(task.note.is_none());

        db.set_note(&task.id, Some("Buy oat milk"))
            .expect("set_note should succeed");
        let inbox = db.list_view(View::Inbox).expect("list");
        assert_eq!(inbox[0].note.as_deref(), Some("Buy oat milk"));

        db.set_note(&task.id, None::<String>)
            .expect("clearing the note should succeed");
        let inbox = db.list_view(View::Inbox).expect("list");
        assert!(inbox[0].note.is_none());
    }

    #[test]
    fn set_title_renames_a_task() {
        let db = open_test_db();
        let task = db.create_task("Wrte notes").expect("create");

        db.set_title(&task.id, "Write notes").expect("set_title should succeed");
        let inbox = db.list_view(View::Inbox).expect("list");
        assert_eq!(inbox[0].title, "Write notes");
    }

    #[test]
    fn completing_a_task_moves_it_from_list_view_to_list_completed() {
        let db = open_test_db();
        let task = db.create_task("Ship it").expect("create should succeed");

        assert_eq!(db.list_view(View::Inbox).expect("list").len(), 1);
        assert!(db.list_completed(View::Inbox).expect("list").is_empty());

        db.set_completed(&task.id, true)
            .expect("complete should succeed");

        assert!(db.list_view(View::Inbox).expect("list").is_empty());
        let completed = db.list_completed(View::Inbox).expect("list");
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].id, task.id);
        assert!(completed[0].completed_at.is_some());
    }

    #[test]
    fn delete_task_removes_it_from_its_view() {
        let db = open_test_db();
        let task = db.create_task("Throwaway").expect("create");
        assert_eq!(db.list_view(View::Inbox).expect("list").len(), 1);

        db.delete_task(&task.id).expect("delete should succeed");
        assert_eq!(db.list_view(View::Inbox).expect("list").len(), 0);
    }

    #[test]
    fn restore_task_undoes_a_delete() {
        let db = open_test_db();
        let task = db.create_task("Bring it back").expect("create");
        db.delete_task(&task.id).expect("delete should succeed");
        assert_eq!(db.list_view(View::Inbox).expect("list").len(), 0);

        db.restore_task(&task.id).expect("restore should succeed");
        let inbox = db.list_view(View::Inbox).expect("list");
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].id, task.id);
    }

    #[test]
    fn deleting_a_parent_cascades_to_its_subtasks() {
        // Found via a data-integrity audit: delete_task only ever touched
        // the one row by id, orphaning subtasks as invisible-but-not-
        // deleted zombies — a subtask never appears in any top-level view
        // (list_view always filters parent_id IS NULL), and the only path
        // to see one is through the parent's own detail card, which can
        // never open again once the parent's gone.
        let db = open_test_db();
        let parent = db.create_task("Plan trip").expect("create");
        let child = db.create_subtask(&parent.id, "Book flights").expect("create_subtask");

        db.delete_task(&parent.id).expect("delete should succeed");

        let subtasks = db.list_subtasks(&parent.id).expect("list_subtasks");
        assert!(
            subtasks.iter().all(|task| task.id != child.id),
            "the subtask should be deleted along with its parent"
        );
    }

    #[test]
    fn undoing_a_parent_delete_restores_its_subtasks_too() {
        let db = open_test_db();
        let parent = db.create_task("Plan trip").expect("create");
        let child = db.create_subtask(&parent.id, "Book flights").expect("create_subtask");

        db.delete_task(&parent.id).expect("delete should succeed");
        db.restore_task(&parent.id).expect("restore should succeed");

        let subtasks = db.list_subtasks(&parent.id).expect("list_subtasks");
        assert!(
            subtasks.iter().any(|task| task.id == child.id),
            "undoing the parent's delete should bring its subtask back too"
        );
    }

    #[test]
    fn a_subtask_does_not_appear_as_its_own_top_level_task() {
        let db = open_test_db();
        let parent = db.create_task("Plan trip").expect("create");
        db.create_subtask(&parent.id, "Book flights")
            .expect("create_subtask should succeed");

        // The subtask must not leak into the parent's own view as a
        // second, independent Inbox row.
        assert_eq!(db.list_view(View::Inbox).expect("list").len(), 1);

        let subtasks = db.list_subtasks(&parent.id).expect("list_subtasks");
        assert_eq!(subtasks.len(), 1);
        assert_eq!(subtasks[0].title, "Book flights");
        assert_eq!(subtasks[0].parent_id.as_deref(), Some(parent.id.as_str()));
    }

    #[test]
    fn a_subtask_cannot_have_its_own_subtask() {
        let db = open_test_db();
        let parent = db.create_task("Plan trip").expect("create");
        let child = db
            .create_subtask(&parent.id, "Book flights")
            .expect("create_subtask should succeed");

        let grandchild = db.create_subtask(&child.id, "Pick a seat");
        assert!(grandchild.is_err(), "one-level ceiling should reject this");
    }

    #[test]
    fn a_completed_parent_cannot_accept_a_new_subtask() {
        let db = open_test_db();
        let parent = db.create_task("Plan trip").expect("create");
        db.set_completed(&parent.id, true).expect("complete parent");

        let child = db.create_subtask(&parent.id, "Book flights");
        assert!(
            child.is_err(),
            "a completed parent should reject a new open subtask (PRD §8)"
        );
    }

    #[test]
    fn a_subtask_inherits_the_parents_bucket_and_no_schedule() {
        let db = open_test_db();
        let parent = db.create_task("Plan trip").expect("create");
        db.schedule(&parent.id, Bucket::Active, None::<String>, None::<String>)
            .expect("schedule");

        let child = db
            .create_subtask(&parent.id, "Book flights")
            .expect("create_subtask should succeed");
        assert_eq!(child.bucket, Bucket::Active);
        assert!(child.scheduled_date.is_none());
    }

    #[test]
    fn list_subtasks_keeps_completed_children_visible() {
        let db = open_test_db();
        let parent = db.create_task("Plan trip").expect("create");
        let child = db
            .create_subtask(&parent.id, "Book flights")
            .expect("create_subtask should succeed");

        db.set_completed(&child.id, true)
            .expect("complete should succeed");

        let subtasks = db.list_subtasks(&parent.id).expect("list_subtasks");
        assert_eq!(subtasks.len(), 1, "completed subtasks stay visible under the parent");
        assert!(subtasks[0].completed_at.is_some());
    }
}
