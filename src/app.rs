//! Flow's shell: native window lifecycle plus the fixed navigation rail and
//! main-pane frame from `docs/PRODUCT_REQUIREMENTS.md` §5.
//!
//! Milestone 0 strips every Flow coding-agent concept — daemon, sessions,
//! transcript, composer, provider tooling — down to this: seven fixed
//! destinations and a placeholder main pane. See the milestone-0 wayfinder
//! ticket for what was removed and why.

use std::array;

use gpui::{AnyElement, App, Context, Entity, FocusHandle, Window, div, prelude::*};

use crate::db::{Bucket, Db, Task};
use crate::input::{ComposerEvent, ComposerInput};
use crate::query::QueryCache;
use crate::theme::Theme;
use crate::{CancelTurn, NewTask, ToggleCommandPalette};

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
    /// Whether the sidebar's Capture row currently shows the text field
    /// instead of the "+ Capture" button. Stays open across submissions so
    /// rapid successive captures don't need to reopen it each time.
    capturing: bool,
    /// A single long-lived composer, reused across every capture rather
    /// than recreated on each open — cheaper and keeps its own focus/blink
    /// state stable. PRD §6.1's title-only capture; note/schedule/parent
    /// fields are a later Milestone 1 step (see docs/HANDOFF.md).
    capture_input: Entity<ComposerInput>,
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

        let flow = cx.new(|cx| {
            let capture_input = cx.new(|cx| {
                ComposerInput::new(window, cx).placeholder(tr!("input.capture_a_task"))
            });
            cx.subscribe(&capture_input, Self::on_capture_event).detach();

            Self {
                destination: Destination::Inbox,
                new_task_focus: cx.focus_handle(),
                nav_focuses: array::from_fn(|_| cx.focus_handle()),
                mode_tasks_focus: cx.focus_handle(),
                db,
                tasks: QueryCache::new(8),
                capturing: false,
                capture_input,
            }
        });
        window.set_window_title(&window_title(Destination::Inbox));
        flow
    }

    /// `⌘N` / the sidebar's "+ Capture" row: opens the composer and focuses
    /// it. PRD §6.1: "always-available task composer focused in the current
    /// view."
    pub(super) fn open_capture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.capturing = true;
        let focus = self.capture_input.read(cx).focus();
        window.focus(&focus, cx);
        cx.notify();
    }

    /// Escape while capturing. PRD §6.1 asks for a confirmation before
    /// discarding unsaved text; skipped here since a bare title field has
    /// nothing more to lose than what Escape already discards — revisit
    /// once the composer grows a note field.
    fn close_capture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.capturing {
            return;
        }
        self.capturing = false;
        self.capture_input.update(cx, |input, cx| input.clear(cx));
        window.focus(&self.new_task_focus, cx);
        cx.notify();
    }

    fn on_capture_event(
        &mut self,
        _input: Entity<ComposerInput>,
        event: &ComposerEvent,
        cx: &mut Context<Self>,
    ) {
        let ComposerEvent::Submit(title) = event else {
            return;
        };
        let Some(db) = self.db.clone() else { return };
        let title = title.clone();
        cx.spawn(async move |flow, cx| {
            let Ok(_task) = cx
                .background_executor()
                .spawn(async move { db.create_task(title) })
                .await
            else {
                return;
            };
            let _ = flow.update(cx, |flow, cx| {
                flow.tasks.invalidate(&Bucket::Inbox);
                cx.notify();
            });
        })
        .detach();
        // Stay open for rapid successive captures; the field already
        // cleared itself (ComposerInput::enter's Composer-mode branch).
    }

    fn handle_new_task_action(&mut self, _: &NewTask, window: &mut Window, cx: &mut Context<Self>) {
        self.open_capture(window, cx);
    }

    fn handle_cancel_turn_action(&mut self, _: &CancelTurn, window: &mut Window, cx: &mut Context<Self>) {
        self.close_capture(window, cx);
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
