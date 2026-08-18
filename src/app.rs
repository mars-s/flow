//! Flow's shell: native window lifecycle plus the fixed navigation rail and
//! main-pane frame from `docs/PRODUCT_REQUIREMENTS.md` §5.
//!
//! Milestone 0 strips every Flow coding-agent concept — daemon, sessions,
//! transcript, composer, provider tooling — down to this: seven fixed
//! destinations and a placeholder main pane. See the milestone-0 wayfinder
//! ticket for what was removed and why.

use std::array;

use gpui::{AnyElement, App, Context, Entity, FocusHandle, Window, div, prelude::*};

use crate::ToggleCommandPalette;
use crate::db::{Bucket, Db, Task};
use crate::input::ComposerInput;
use crate::query::QueryCache;
use crate::theme::Theme;

mod command_palette;
mod components;
mod render;
mod settings;
mod sidebar;
mod tasks;
#[cfg(test)]
mod tests;
mod window_chrome;

pub use command_palette::init as init_command_palette;
pub use settings::init as init_settings_keys;
pub use sidebar::init as init_sidebar_keys;
use sidebar::Destination;

pub struct Flow {
    destination: Destination,
    new_task_focus: FocusHandle,
    /// One stable handle per destination, indexed by `Destination::index`.
    nav_focuses: [FocusHandle; Destination::COUNT],
    /// The sidebar's "Tasks" mode-switch segment. The "Calendar" segment
    /// reuses `nav_focuses[Destination::Calendar.index()]` since it and the
    /// Calendar destination are the same thing; Tasks has no destination of
    /// its own, so it needs a handle that isn't one of the seven above.
    mode_tasks_focus: FocusHandle,
    /// `None` when opening the local database failed at startup (e.g. an
    /// unwritable data directory) — task views degrade to an error state
    /// rather than panicking. See `tasks.rs`.
    db: Option<Db>,
    /// Loaded lazily per bucket from `render`, per this repo's
    /// `cx.background_executor().spawn` + `cx.notify()` convention
    /// (`query.rs`'s own doc comment is this exact pattern).
    tasks: QueryCache<Bucket, Vec<Task>>,
}

impl Flow {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        // Opening a local file is a one-time, sub-millisecond startup cost,
        // not a per-frame one — unlike render-path I/O, this is exactly the
        // "one-shot user action" CLAUDE.md carves out as fine to do
        // synchronously. A failure degrades task views instead of crashing.
        let db = match Db::open() {
            Ok(db) => Some(db),
            Err(error) => {
                eprintln!("Flow: failed to open the local database: {error:#}");
                None
            }
        };

        let flow = cx.new(|cx| Self {
            destination: Destination::Inbox,
            new_task_focus: cx.focus_handle(),
            nav_focuses: array::from_fn(|_| cx.focus_handle()),
            mode_tasks_focus: cx.focus_handle(),
            db,
            tasks: QueryCache::new(8),
        });
        window.set_window_title(&window_title(Destination::Inbox));
        flow
    }

    /// Where the window should land keyboard focus on open, so arrow keys
    /// work in the sidebar immediately without an extra tab press.
    pub(crate) fn initial_focus(&self, _cx: &App) -> FocusHandle {
        self.nav_focuses[self.destination.index()].clone()
    }

    pub(super) fn set_destination(
        &mut self,
        destination: Destination,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.destination == destination {
            return;
        }
        self.destination = destination;
        window.set_window_title(&window_title(destination));
        cx.notify();
    }
}

fn window_title(destination: Destination) -> String {
    format!("{} — Flow", destination.label())
}
