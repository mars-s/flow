//! Flow's shell: native window lifecycle plus the fixed navigation rail and
//! main-pane frame from `docs/PRODUCT_REQUIREMENTS.md` §5.
//!
//! Milestone 0 strips every Flow coding-agent concept — daemon, sessions,
//! transcript, composer, provider tooling — down to this: seven fixed
//! destinations and a placeholder main pane. See the milestone-0 wayfinder
//! ticket for what was removed and why.

use std::array;

use gpui::{AnyElement, App, Context, Entity, FocusHandle, Window, div, prelude::*};

use crate::db::{Bucket, Db, Task, View};
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
    tasks: QueryCache<View, Vec<Task>>,
    /// Whether the sidebar's Capture row currently shows the text field
    /// instead of the "+ Capture" button. Stays open across submissions so
    /// rapid successive captures don't need to reopen it each time.
    capturing: bool,
    /// A single long-lived composer, reused across every capture rather
    /// than recreated on each open — cheaper and keeps its own focus/blink
    /// state stable. PRD §6.1's title-only capture; note/schedule/parent
    /// fields are a later Milestone 1 step (see docs/HANDOFF.md).
    capture_input: Entity<ComposerInput>,
    /// The Inbox row currently showing its inline "Process" actions
    /// (Today/Anytime/Someday), if any — PRD §6.3's "inline Process action"
    /// on Inbox. Only one row processes at a time, matching how
    /// `capturing` gates the sidebar to one open field.
    processing_task_id: Option<String>,
    /// Whether `processing_task_id`'s row is showing the free-text
    /// "Schedule…" field instead of the three quick buttons. PRD §6.3
    /// names "schedule" (an arbitrary date) as the fourth Process option;
    /// rather than building a calendar widget, this reuses `parse.rs` on
    /// whatever the user types, the same way Capture does.
    scheduling: bool,
    /// A single long-lived composer for the "Schedule…" field, same
    /// reuse-not-recreate reasoning as `capture_input`.
    schedule_input: Entity<ComposerInput>,
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

            let schedule_input = cx.new(|cx| {
                ComposerInput::new(window, cx).placeholder(tr!("input.schedule_when"))
            });
            cx.subscribe(&schedule_input, Self::on_schedule_event).detach();

            Self {
                destination: Destination::Inbox,
                new_task_focus: cx.focus_handle(),
                nav_focuses: array::from_fn(|_| cx.focus_handle()),
                mode_tasks_focus: cx.focus_handle(),
                db,
                tasks: QueryCache::new(8),
                capturing: false,
                capture_input,
                processing_task_id: None,
                scheduling: false,
                schedule_input,
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

        // Parsing is pure and cheap (no I/O), so it runs inline rather than
        // adding a second background hop before the write below.
        let parsed = crate::parse::parse(title, chrono::Local::now().date_naive());
        // A title that parsed down to nothing but the date phrase itself
        // (the user typed only "tomorrow") has no title left; PRD §6.1
        // requires a nonempty title, so treat that as unrecognized rather
        // than saving a blank one.
        let (title, date, time) = if parsed.cleaned_title.is_empty() {
            (title.clone(), None, None)
        } else {
            (parsed.cleaned_title, parsed.date, parsed.time)
        };

        cx.spawn(async move |flow, cx| {
            let created = cx
                .background_executor()
                .spawn(async move {
                    let task = db.create_task(title)?;
                    // Per PRD §5/§14, a parsed date does not activate the
                    // task — it stays in Inbox as a review queue, just with
                    // its schedule already attached.
                    if date.is_some() || time.is_some() {
                        db.schedule(
                            &task.id,
                            Bucket::Inbox,
                            date.map(|d| d.to_string()),
                            time.map(|t| t.format("%H:%M").to_string()),
                        )?;
                    }
                    anyhow::Ok(())
                })
                .await;
            if created.is_err() {
                return;
            }
            let _ = flow.update(cx, |flow, cx| {
                flow.tasks.invalidate(&View::Inbox);
                cx.notify();
            });
        })
        .detach();
        // Stay open for rapid successive captures; the field already
        // cleared itself (ComposerInput::enter's Composer-mode branch).
    }

    /// The Process row's "Schedule…" button: swaps the three quick buttons
    /// for a free-text field, focused, for the row already in
    /// `processing_task_id`.
    pub(super) fn open_schedule_field(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.scheduling = true;
        let focus = self.schedule_input.read(cx).focus();
        window.focus(&focus, cx);
        cx.notify();
    }

    fn on_schedule_event(
        &mut self,
        _input: Entity<ComposerInput>,
        event: &ComposerEvent,
        cx: &mut Context<Self>,
    ) {
        let ComposerEvent::Submit(text) = event else {
            return;
        };
        let Some(id) = self.processing_task_id.clone() else {
            return;
        };
        let Some(db) = self.db.clone() else { return };

        let parsed = crate::parse::parse(text, chrono::Local::now().date_naive());
        // Nothing recognized: leave the field open so the user can correct
        // it, same as Capture leaves an unrecognized phrase in the title
        // rather than guessing.
        let Some(date) = parsed.date else { return };
        let time = parsed.time;

        self.scheduling = false;
        self.processing_task_id = None;
        self.schedule_input.update(cx, |input, cx| input.clear(cx));

        cx.spawn(async move |flow, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    db.schedule(
                        id,
                        Bucket::Active,
                        Some(date.to_string()),
                        time.map(|t| t.format("%H:%M").to_string()),
                    )
                })
                .await;
            if result.is_err() {
                return;
            }
            let _ = flow.update(cx, |flow, cx| {
                for view in [View::Inbox, View::Today, View::Upcoming, View::Anytime] {
                    flow.tasks.invalidate(&view);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn handle_new_task_action(&mut self, _: &NewTask, window: &mut Window, cx: &mut Context<Self>) {
        self.open_capture(window, cx);
    }

    fn handle_cancel_turn_action(&mut self, _: &CancelTurn, window: &mut Window, cx: &mut Context<Self>) {
        if self.scheduling || self.processing_task_id.is_some() {
            self.scheduling = false;
            self.processing_task_id = None;
            self.schedule_input.update(cx, |input, cx| input.clear(cx));
            cx.notify();
            return;
        }
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
