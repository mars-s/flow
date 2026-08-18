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
use turso::Row;
use uuid::Uuid;

/// The three persisted placements from `docs/PRODUCT_REQUIREMENTS.md` §5.
/// Today/Upcoming/Anytime are computed views over `Active`, not stored.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

/// Mirrors the `tasks` table from `docs/PRODUCT_REQUIREMENTS.md` §8. No
/// `user_id`: Flow's local phase is single-user, so a `users` table would be
/// speculative multi-tenancy nothing here needs yet.
#[derive(Clone, Debug)]
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
    ListBucket {
        bucket: Bucket,
        reply: mpsc::Sender<Result<Vec<Task>>>,
    },
    SetCompleted {
        id: String,
        completed: bool,
        reply: mpsc::Sender<Result<()>>,
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

    /// Open, undeleted tasks in one bucket, ordered by position.
    pub fn list_bucket(&self, bucket: Bucket) -> Result<Vec<Task>> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::ListBucket { bucket, reply })
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
            Command::ListBucket { bucket, reply } => {
                let _ = reply.send(runtime.block_on(list_bucket(&conn, bucket)));
            }
            Command::SetCompleted {
                id,
                completed,
                reply,
            } => {
                let _ = reply.send(runtime.block_on(set_completed(&conn, id, completed)));
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

/// Open, undeleted, uncompleted tasks in one bucket. Completed items are
/// hidden from primary views by default per
/// `docs/PRODUCT_REQUIREMENTS.md` §5 — the future Logbook view will query
/// completed tasks separately rather than this function gaining a flag.
async fn list_bucket(conn: &turso::Connection, bucket: Bucket) -> Result<Vec<Task>> {
    let sql = format!(
        "SELECT {TASK_COLUMNS} FROM tasks \
         WHERE bucket = ?1 AND deleted_at IS NULL AND completed_at IS NULL \
         ORDER BY position ASC"
    );
    let mut rows = conn
        .query(&sql, (bucket.as_str(),))
        .await
        .context("listing bucket")?;
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

        let inbox = db.list_bucket(Bucket::Inbox).expect("list should succeed");
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
        let inbox = db.list_bucket(Bucket::Inbox).expect("list should succeed");
        assert!(
            inbox.is_empty(),
            "completed tasks should be hidden from primary views by default"
        );

        db.set_completed(&created.id, false)
            .expect("reopen should succeed");
        let inbox = db.list_bucket(Bucket::Inbox).expect("list should succeed");
        assert_eq!(inbox.len(), 1);
        assert!(inbox[0].completed_at.is_none());
    }
}
