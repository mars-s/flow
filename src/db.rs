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

use anyhow::{Context, Result};

/// Applied once on open. Real migrations arrive with Milestone 1's task
/// schema; for now this only proves the connection and schema application
/// work end to end.
const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)";

enum Command {
    Ping { reply: mpsc::Sender<Result<bool>> },
    Execute { sql: String, reply: mpsc::Sender<Result<()>> },
}

/// A cheaply cloneable handle to the database thread.
#[derive(Clone)]
pub struct Db {
    commands: mpsc::Sender<Command>,
}

impl Db {
    /// Open (or create) the local database file in Flow's app data
    /// directory and spawn the dedicated thread that owns it. Blocks until
    /// the connection is open and the schema is applied.
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

    /// Run one parameterless statement.
    pub fn execute(&self, sql: impl Into<String>) -> Result<()> {
        let (reply, rx) = mpsc::channel();
        self.commands
            .send(Command::Execute {
                sql: sql.into(),
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
                let result = runtime.block_on(ping(&conn));
                let _ = reply.send(result);
            }
            Command::Execute { sql, reply } => {
                let result = runtime.block_on(async {
                    conn.execute(&sql, ())
                        .await
                        .map(|_| ())
                        .with_context(|| format!("running statement: {sql}"))
                });
                let _ = reply.send(result);
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
    conn.execute(SCHEMA, ())
        .await
        .context("applying the schema")?;
    Ok(conn)
}

async fn ping(conn: &turso::Connection) -> Result<bool> {
    let mut rows = conn.query("SELECT 1", ()).await.context("querying")?;
    Ok(rows.next().await.context("reading a row")?.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_a_database_and_answers_a_ping() {
        let dir = std::env::temp_dir().join(format!("flow-db-test-{}", uuid::Uuid::new_v4()));
        let db = Db::open_at(dir.join("flow.db")).expect("database should open");

        assert!(db.ping().expect("ping should succeed"));
        db.execute("INSERT INTO schema_version (version) VALUES (1)")
            .expect("execute should succeed");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
