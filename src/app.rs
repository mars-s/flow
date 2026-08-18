//! Flow's shell: native window lifecycle plus the fixed navigation rail and
//! main-pane frame from `docs/PRODUCT_REQUIREMENTS.md` §5.
//!
//! Milestone 0 strips every Flow coding-agent concept — daemon, sessions,
//! transcript, composer, provider tooling — down to this: seven fixed
//! destinations and a placeholder main pane. See the milestone-0 wayfinder
//! ticket for what was removed and why.

use std::array;
use std::collections::HashSet;

use gpui::{AnyElement, App, Context, Entity, FocusHandle, Window, div, prelude::*};

use crate::db::{Bucket, Db, Task, View};
use crate::input::{ComposerEvent, ComposerInput};
use crate::query::QueryCache;
use crate::theme::Theme;
use crate::{CancelTurn, NewTask, SpaceCapture, ToggleCommandPalette};

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
    /// The task row currently showing its detail card (note, schedule
    /// status, delete), if any — `docs/DESIGN_DIRECTION.md`'s "Task detail"
    /// component, available from every task view. Only one row expands at a
    /// time, matching how `capturing` gates the sidebar to one open field.
    expanded_task_id: Option<String>,
    /// Which task's note `note_input`'s content belongs to. Kept separate
    /// from `expanded_task_id` so a blur that lands after the card has
    /// already closed still saves against the right task.
    note_task_id: Option<String>,
    /// A single long-lived composer for the detail card's note field, same
    /// reuse-not-recreate reasoning as `capture_input`.
    note_input: Entity<ComposerInput>,
    /// Whether the expanded card's schedule pill has opened the
    /// Today/Anytime/Someday/"Schedule…" quick-picker.
    schedule_picker_open: bool,
    /// Whether the picker is showing the free-text "Schedule…" field
    /// instead of the three quick buttons. PRD §6.3 names "schedule" (an
    /// arbitrary date) as the fourth Process option; rather than building a
    /// calendar widget, this reuses `parse.rs` on whatever the user types,
    /// the same way Capture does.
    scheduling: bool,
    /// A single long-lived composer for the "Schedule…" field, same
    /// reuse-not-recreate reasoning as `capture_input`.
    schedule_input: Entity<ComposerInput>,
    /// Cmd+click multi-select: task ids toggled in without opening their
    /// detail card. A plain click clears this and falls back to opening/
    /// closing the card as usual.
    selected_task_ids: HashSet<String>,
    /// A second `QueryCache`, keyed the same as `tasks`, for each view's
    /// completed tasks — kept separate rather than widening `tasks`'s value
    /// since the two listings have different SQL, ordering, and visibility
    /// (collapsed by default), and a shared cache entry couldn't answer
    /// "how many completed" without always fetching both anyway.
    completed_tasks: QueryCache<View, Vec<Task>>,
    /// Which views currently show their collapsed "Completed" section
    /// expanded. A set rather than one bool per view, matching this
    /// codebase's `HashSet<View>` idiom for per-view UI flags.
    completed_expanded: HashSet<View>,
    /// Tasks mid-way through the 180 ms fade/collapse
    /// `docs/DESIGN_DIRECTION.md` names for completing a row. The checkbox
    /// fills immediately, but the actual `Db::set_completed` write (and the
    /// row's removal from the list) waits for the animation to finish, so
    /// the row has something to animate instead of vanishing on the spot.
    completing_ids: HashSet<String>,
    /// The most recent "Completed" toast, if its 10-second window hasn't
    /// elapsed. Only one at a time — a second completion while one is
    /// showing simply replaces it, matching the single-slot toast most task
    /// apps use rather than a queue nobody asked for.
    undo_toast: Option<UndoToast>,
    /// Distinguishes a stale dismiss timer from the current toast — see
    /// `show_undo_toast`.
    undo_token: u64,
    /// Subtasks, keyed by parent task id — a third `QueryCache` alongside
    /// `tasks`/`completed_tasks` rather than folding into either, since a
    /// key here is a task id, not a `View`. Only fetched for the currently
    /// expanded task (`Flow::toggle_expanded`), the same lazy-on-demand
    /// reasoning as `note_input`'s content: a compact row never shows
    /// subtask progress, so nothing here is reachable from a frame that
    /// only has the list open, matching CLAUDE.md's render-path I/O rule.
    subtasks: QueryCache<String, Vec<Task>>,
    /// Whether the expanded card's "+ Add subtask" affordance has swapped
    /// in `subtask_input`, mirroring `scheduling`'s relationship to the
    /// schedule picker.
    adding_subtask: bool,
    /// A single long-lived composer for adding a subtask, same reuse-not-
    /// recreate reasoning as `capture_input`/`schedule_input`.
    subtask_input: Entity<ComposerInput>,
    /// The task id currently showing the inline "Complete parent and all
    /// subtasks?" prompt (PRD §6.2), if any — set instead of completing
    /// immediately when the expanded card's checkbox is clicked on a task
    /// with at least one incomplete subtask.
    pending_complete_confirm: Option<String>,
}

/// What `render_undo_toast` (`app/tasks.rs`) shows and what `Flow::undo`
/// reverses.
struct UndoToast {
    task_id: String,
    title: gpui::SharedString,
    origin_view: View,
    token: u64,
    kind: UndoKind,
}

/// What a toast's Undo button actually reverses — completion and deletion
/// share the toast UI and the 10-second dismiss timer (PRD §6.1 names that
/// window for deletion specifically; completion's own spec doesn't state a
/// different one), but need different DB writes to undo.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum UndoKind {
    Complete,
    Delete,
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

            let note_input = cx.new(|cx| {
                ComposerInput::new(window, cx)
                    .code_editor(None)
                    .placeholder(tr!("input.notes"))
            });
            // The note field has no Submit event (code-mode Enter inserts a
            // newline, per notes wanting multiple lines) — blur is the only
            // save trigger, so it's wired directly rather than through
            // `cx.subscribe`.
            let note_focus = note_input.read(cx).focus();
            cx.on_blur(&note_focus, window, Self::on_note_blur).detach();

            let subtask_input = cx.new(|cx| {
                ComposerInput::new(window, cx).placeholder(tr!("input.add_subtask"))
            });
            cx.subscribe(&subtask_input, Self::on_subtask_event).detach();

            Self {
                destination: Destination::Inbox,
                new_task_focus: cx.focus_handle(),
                nav_focuses: array::from_fn(|_| cx.focus_handle()),
                mode_tasks_focus: cx.focus_handle(),
                db,
                tasks: QueryCache::new(8),
                capturing: false,
                capture_input,
                expanded_task_id: None,
                note_task_id: None,
                note_input,
                schedule_picker_open: false,
                scheduling: false,
                schedule_input,
                selected_task_ids: HashSet::new(),
                completed_tasks: QueryCache::new(8),
                completed_expanded: HashSet::new(),
                completing_ids: HashSet::new(),
                undo_toast: None,
                undo_token: 0,
                subtasks: QueryCache::new(8),
                adding_subtask: false,
                subtask_input,
                pending_complete_confirm: None,
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
        if matches!(event, ComposerEvent::Edited) {
            self.highlight_capture_date_phrase(cx);
            return;
        }
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

        let has_schedule = date.is_some() || time.is_some();
        cx.spawn(async move |flow, cx| {
            let created = cx
                .background_executor()
                .spawn(async move {
                    let task = db.create_task(title)?;
                    // A parsed date activates the task immediately — it
                    // moves straight to Today/Upcoming instead of sitting
                    // in Inbox with a schedule attached. Explicit product
                    // decision overriding this project's earlier PRD §14
                    // reading ("a parsed date does not activate the task").
                    if has_schedule {
                        db.schedule(
                            &task.id,
                            Bucket::Active,
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
                for view in [View::Inbox, View::Today, View::Upcoming, View::Anytime] {
                    flow.tasks.invalidate(&view);
                }
                cx.notify();
            });
        })
        .detach();
        // Stay open for rapid successive captures; the field already
        // cleared itself (ComposerInput::enter's Composer-mode branch).
    }

    /// Live preview for Capture: re-parses on every keystroke (pure and
    /// cheap, same reasoning `on_capture_event`'s submit-time parse already
    /// documents) and paints whatever date/time phrase it recognizes using
    /// the composer's existing find-match highlight
    /// (`ComposerInput::set_search_matches`, built for search but just as
    /// suited to washing a live-parsed range) instead of a second highlight
    /// mechanism.
    fn highlight_capture_date_phrase(&mut self, cx: &mut Context<Self>) {
        let content = self.capture_input.read(cx).content().to_string();
        let parsed = crate::parse::parse(&content, chrono::Local::now().date_naive());
        let (matches, active) = match parsed.source_range {
            Some(range) => (vec![range], Some(0)),
            None => (Vec::new(), None),
        };
        self.capture_input
            .update(cx, |input, cx| input.set_search_matches(matches, active, cx));
    }

    /// Focuses `schedule_input` for the row already in `expanded_task_id` —
    /// called by `Flow::toggle_schedule_picker` (`tasks.rs`) the moment the
    /// picker opens, so the NLP field is ready to type into immediately
    /// rather than needing a separate "Schedule…" click first.
    pub(super) fn focus_schedule_field(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.schedule_input.read(cx).focus();
        window.focus(&focus, cx);
    }

    fn on_schedule_event(
        &mut self,
        _input: Entity<ComposerInput>,
        event: &ComposerEvent,
        cx: &mut Context<Self>,
    ) {
        if matches!(event, ComposerEvent::Edited) {
            self.highlight_schedule_date_phrase(cx);
            return;
        }
        let ComposerEvent::Submit(text) = event else {
            return;
        };
        let Some(id) = self.expanded_task_id.clone() else {
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
        self.schedule_picker_open = false;
        self.set_expanded_task(None, cx);
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

    /// Live preview for the "Schedule…" field, same reasoning as
    /// `highlight_capture_date_phrase` — cheap enough that establishing the
    /// pattern once made this second field free.
    fn highlight_schedule_date_phrase(&mut self, cx: &mut Context<Self>) {
        let content = self.schedule_input.read(cx).content().to_string();
        let parsed = crate::parse::parse(&content, chrono::Local::now().date_naive());
        let (matches, active) = match parsed.source_range {
            Some(range) => (vec![range], Some(0)),
            None => (Vec::new(), None),
        };
        self.schedule_input
            .update(cx, |input, cx| input.set_search_matches(matches, active, cx));
    }

    /// The detail card's "+ Add subtask" row: swaps in `subtask_input`,
    /// focused, for the task already in `expanded_task_id` — same
    /// swap-in-place idiom as `focus_schedule_field`.
    pub(super) fn open_add_subtask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.adding_subtask = true;
        let focus = self.subtask_input.read(cx).focus();
        window.focus(&focus, cx);
        cx.notify();
    }

    fn on_subtask_event(
        &mut self,
        _input: Entity<ComposerInput>,
        event: &ComposerEvent,
        cx: &mut Context<Self>,
    ) {
        let ComposerEvent::Submit(title) = event else {
            return;
        };
        let Some(parent_id) = self.expanded_task_id.clone() else {
            return;
        };
        let Some(db) = self.db.clone() else { return };
        if title.trim().is_empty() {
            return; // PRD §6.1: a task needs a nonempty title.
        }
        let title = title.clone();

        cx.spawn(async move |flow, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { db.create_subtask(parent_id.clone(), title).map(|_| parent_id) })
                .await;
            let Ok(parent_id) = result else { return };
            let _ = flow.update(cx, |flow, cx| {
                flow.subtasks.invalidate(&parent_id);
                cx.notify();
            });
        })
        .detach();
        // Stay open for adding several subtasks in a row, same reasoning
        // as Capture; the field already cleared itself on submit.
    }

    fn handle_new_task_action(&mut self, _: &NewTask, window: &mut Window, cx: &mut Context<Self>) {
        self.open_capture(window, cx);
    }

    /// Bare `space`, scoped to task views only (`Destination::view()` is
    /// `None` for Calendar/Settings) so it doesn't hijack space's native
    /// meaning there. The keymap context (`lib.rs`'s `!ComposerInput`
    /// predicate) already keeps this from firing while a composer has
    /// focus; the `capturing`/`scheduling`/`expanded_task_id` check here is
    /// a defensive second layer in case a future field reuses `ComposerInput`
    /// without leaning on the same key context.
    fn handle_space_capture_action(
        &mut self,
        _: &SpaceCapture,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.capturing || self.scheduling || self.expanded_task_id.is_some() {
            return;
        }
        if self.destination.view().is_none() {
            return;
        }
        self.open_capture(window, cx);
    }

    fn handle_cancel_turn_action(&mut self, _: &CancelTurn, window: &mut Window, cx: &mut Context<Self>) {
        if self.scheduling || self.expanded_task_id.is_some() {
            self.scheduling = false;
            self.schedule_picker_open = false;
            self.adding_subtask = false;
            self.pending_complete_confirm = None;
            self.set_expanded_task(None, cx);
            self.schedule_input.update(cx, |input, cx| input.clear(cx));
            self.subtask_input.update(cx, |input, cx| input.clear(cx));
            cx.notify();
            return;
        }
        self.close_capture(window, cx);
    }

    /// The detail card's note field lost focus: the only save trigger for
    /// notes (code-mode Enter inserts a newline instead of submitting).
    /// Writing the same content twice is harmless, so this fires
    /// unconditionally rather than tracking a dirty flag.
    fn on_note_blur(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.flush_note(cx);
    }

    /// The actual note-saving write, factored out of `on_note_blur` so
    /// `set_expanded_task` can call it proactively too — see that method's
    /// doc comment for why relying on blur alone silently dropped notes.
    /// Writing the same content twice is harmless, so this fires
    /// unconditionally rather than tracking a dirty flag.
    fn flush_note(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.note_task_id.clone() else {
            return;
        };
        let Some(db) = self.db.clone() else { return };
        let content = self.note_input.read(cx).content().to_string();
        let note = (!content.trim().is_empty()).then_some(content);

        cx.spawn(async move |_flow, cx| {
            let _ = cx
                .background_executor()
                .spawn(async move { db.set_note(id, note) })
                .await;
        })
        .detach();
    }

    /// Every direct write to `expanded_task_id` goes through here so a note
    /// typed into the shared `note_input` is never lost. GPUI's blur
    /// signal (the note field's only other save trigger) fires only when
    /// keyboard focus explicitly moves to another focusable element —
    /// clicking a plain button or checkbox that doesn't take focus, or
    /// switching straight from one task's card to another's, never blurs
    /// the field, so relying on blur alone silently dropped whatever was
    /// typed (user-reported: "notes don't work"). This flushes proactively
    /// instead, whenever the card is about to close or switch to a
    /// different task.
    pub(super) fn set_expanded_task(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        if self.expanded_task_id.is_some() && self.expanded_task_id != id {
            self.flush_note(cx);
        }
        self.expanded_task_id = id;
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
