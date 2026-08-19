//! A tiny always-on event log for debug builds — appended to a plain text
//! file next to the local database (`~/Library/Application Support/Flow
//! Dev/debug.log` on macOS), independent of any UI. Exists so a session
//! with no screen-recording access (this one, most of the time this
//! project has been worked on) can read *what actually happened* in the
//! running app as text, instead of needing a screenshot to debug a
//! sequence of events. See the `flow-debug` skill for how this fits
//! alongside the Cmd-Option-I inspector (`src/app/inspector.rs`).
//!
//! Deliberately not routed through stdout/`println!`: the app is launched
//! via macOS's `open -n -W` (`scripts/dev.ts`), which does not reliably
//! deliver a GUI subprocess's own stdout back to the watching terminal —
//! writing to a real file sidesteps that entirely.
//!
//! No-ops completely in release builds via `crate::debug_log!`'s two
//! cfg-gated bodies below — this is a development aid, not a product
//! feature, and never touches a machine that isn't actively being
//! developed against.

#[cfg(debug_assertions)]
use std::io::Write;

/// Append one timestamped line. Failure (no data dir, disk full, ...) is
/// swallowed — a missing debug log must never be the reason the app
/// misbehaves or panics.
#[cfg(debug_assertions)]
pub fn log(args: std::fmt::Arguments) {
    let Some(base) = dirs::data_dir() else { return };
    let path = base
        .join(flow_core::identity::DATA_DIRECTORY_NAME)
        .join("debug.log");
    let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let now = chrono::Local::now().format("%H:%M:%S%.3f");
    let _ = writeln!(file, "[{now}] {args}");
}

#[cfg(not(debug_assertions))]
pub fn log(_args: std::fmt::Arguments) {}

/// `debug_log!("task {id} completed")` — same call shape as `println!`,
/// routed through [`log`] rather than stdout. Exported at the crate root
/// (not just this module) so call sites elsewhere in the crate can use it
/// as `crate::debug_log!(...)` without an extra `use`.
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        $crate::debug_log::log(format_args!($($arg)*))
    };
}
