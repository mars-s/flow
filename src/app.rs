//! Flow's shell: native window lifecycle plus the fixed navigation rail and
//! main-pane frame from `docs/PRODUCT_REQUIREMENTS.md` §5.
//!
//! Milestone 0 strips every Flow coding-agent concept — daemon, sessions,
//! transcript, composer, provider tooling — down to this: seven fixed
//! destinations and a placeholder main pane. See the milestone-0 wayfinder
//! ticket for what was removed and why.

use std::array;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use gpui::{AnyElement, App, Context, Entity, FocusHandle, ListState, Window, div, prelude::*};

use crate::db::{Bucket, Db, Task, View};
use crate::input::{ComposerEvent, ComposerInput};
use crate::query::QueryCache;
use crate::theme::Theme;
use crate::{CancelTurn, NewTask, SpaceCapture, ToggleCommandPalette, ToggleInspector};

mod calendar;
mod command_palette;
mod components;
mod inspector;
mod render;
mod settings;
mod sidebar;
mod tasks;
#[cfg(test)]
mod tests;
mod window_chrome;

pub use inspector::init as init_inspector;

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
    /// The Tasks/Calendar pill's sliding thumb, mid-flight — `None` at
    /// rest (thumb sitting exactly at the active mode's position, nothing
    /// to evaluate every render). `ui::sidebar::render_mode_switch`
    /// evaluates it by hand each frame via `ui::motion::Tween` rather than
    /// `with_animation`, since a fast double-toggle needs to resume from
    /// wherever the thumb currently sits, not replay from a fixed start.
    mode_thumb: Option<crate::ui::motion::Tween>,
    /// `None` when opening the local database failed at startup (e.g. an
    /// unwritable data directory) — task views degrade to an error state
    /// rather than panicking. See `tasks.rs`.
    db: Option<Db>,
    /// Loaded lazily per bucket from `render`, per this repo's
    /// `cx.background_executor().spawn` + `cx.notify()` convention
    /// (`query.rs`'s own doc comment is this exact pattern).
    tasks: QueryCache<View, Vec<Task>>,
    /// The last `Query::Ready` value seen per view, kept around so a
    /// mutation's `invalidate_view` (which evicts the cache entry outright)
    /// doesn't blank the whole list to a loading skeleton for the refetch's
    /// round trip to the DB thread. `render_task_view` draws this stale
    /// value instead on a `Pending`/`Missing` read, matching `query.rs`'s
    /// own doc comment ("`Query::Pending => None, // draw the last known
    /// value`") — this is that fallback actually being kept, not a new
    /// pattern. Falls back to the skeleton only on a genuine first load,
    /// when nothing has ever rendered for that view yet.
    last_tasks: HashMap<View, Arc<Vec<Task>>>,
    /// Same stale-while-revalidate fallback as `last_tasks`, for
    /// `completed_tasks`.
    last_completed: HashMap<View, Arc<Vec<Task>>>,
    /// One virtualized-list state per flat (non-Upcoming) view — GPUI's
    /// own `list()`/`ListState`, the primitive `CLAUDE.md`'s performance
    /// section names ("Long collections are virtualized with `list()`"),
    /// proven to bound render cost regardless of item count in
    /// `ui::virtualized_list`'s tests. Created lazily and re-spliced
    /// (`app/tasks.rs::sync_task_list_state`) whenever `last_tasks`' Arc
    /// for that view changes identity — a real refetch, not the same
    /// stale snapshot redrawn across renders. **Known simplification**:
    /// the resplice always replaces the whole range rather than diffing
    /// old against new, so any data change (not just an unrelated one
    /// elsewhere) resets scroll position to the top — acceptable for now
    /// since it only fires on an actual mutation, not on every render;
    /// a minimal-diff splice is the natural follow-up if that's felt in
    /// practice. Upcoming isn't covered — its date-grouped sections don't
    /// fit `list()`'s flat item-index model without materially more work.
    task_list_states: HashMap<View, ListState>,
    /// The overlay scrollbar paired with each view's `task_list_states`
    /// entry — `ui::scrollbar` already had a `Scrollable` impl for
    /// `ListState` built and ready, just never used until now.
    task_list_scrollbars: HashMap<View, Rc<crate::ui::scrollbar::ScrollbarState>>,
    /// Per-task keyboard focus handles, keyed by task id — the same
    /// pattern `nav_focuses` uses for the sidebar's fixed seven
    /// destinations, but as a map instead of a fixed array since the task
    /// list is dynamic. Created lazily (`Flow::row_focus`) and pruned in
    /// `render_task_view` whenever a fresh task list arrives (same spot
    /// that already prunes `completing_ids`), so this stays bounded to
    /// currently-visible tasks rather than growing forever across a
    /// session. First entry in the `CLAUDE.md`/PRD keyboard-accessibility
    /// gap this project's own audit found — see `docs/HANDOFF.md`.
    row_focuses: HashMap<String, FocusHandle>,
    /// Same pattern as `row_focuses`, for the collapsed Completed section's
    /// own rows — a *separate* map, not shared, since `row_focuses`'
    /// pruning runs against a view's active task list and would delete
    /// every completed-row handle on the very next unrelated refetch
    /// (identical reasoning to why `subtask_focuses` is its own map too).
    /// Pruned in `render_task_view` against whichever completed list was
    /// just fetched.
    completed_row_focuses: HashMap<String, FocusHandle>,
    /// Per-subtask keyboard focus handles, keyed by subtask id — a
    /// separate map from `row_focuses` even though both hold task-id-
    /// keyed `FocusHandle`s, since `row_focuses`' pruning runs against
    /// the flat top-level task list, which never contains a subtask id
    /// (every view query filters `parent_id IS NULL`); reusing that map
    /// would delete every subtask handle on the next unrelated refetch.
    /// Pruned instead in `render_task_view` against whichever task's
    /// subtasks are actually loaded, alongside `SubtaskContext`'s own
    /// construction.
    subtask_focuses: HashMap<String, FocusHandle>,
    /// The expanded card's "+ Add subtask" row — same single-stable-
    /// handle reasoning as `detail_delete_focus` (only one card, so only
    /// one add-subtask row, at a time).
    add_subtask_focus: FocusHandle,
    /// One keyboard focus handle per view for the collapsed "Completed"
    /// section's "Clear" button — bounded to the five task views (no
    /// pruning needed, unlike `row_focuses`), created lazily the same way.
    completed_clear_focuses: HashMap<View, FocusHandle>,
    /// The Undo toast's own focus handle. A single stable field (like
    /// `new_task_focus`) rather than a fresh handle per render, since a
    /// fresh handle each frame would drop tab focus the instant a render
    /// happened to land while the toast had it — GPUI's tab order only
    /// includes handles actually `track_focus`'d in the current frame, so
    /// this being unused whenever no toast is showing is harmless.
    undo_toast_focus: FocusHandle,
    /// The expanded detail card's delete button — a single stable field,
    /// same reasoning as `undo_toast_focus`: only one task can ever be
    /// expanded at a time (`expanded_task_id: Option<String>`), so there's
    /// only ever one card's delete button to reach, and reusing one handle
    /// across every task avoids needing a `row_focuses`-style per-task map
    /// for something that's never actually rendered more than once.
    detail_delete_focus: FocusHandle,
    /// The expanded detail card's static title heading, when not being
    /// edited — same single-stable-handle reasoning as
    /// `detail_delete_focus`. Enter/Space starts editing (see
    /// `editing_title`); `title_input` itself gets its own Tab
    /// reachability for free once editing starts, from every
    /// `ComposerInput`'s `.tab_index(0)`.
    title_focus: FocusHandle,
    /// The expanded detail card's schedule pill — same single-stable-
    /// handle reasoning as `detail_delete_focus`.
    schedule_pill_focus: FocusHandle,
    /// The open schedule picker's Today/Anytime/Someday quick-pick pills,
    /// in `ProcessTarget::ALL`'s order — same single-stable-handle
    /// reasoning as `schedule_pill_focus` (only one picker open at a
    /// time), three named fields rather than a map since `ProcessTarget`
    /// is a small fixed enum, not dynamic per-task data.
    process_pill_focuses: [FocusHandle; 3],
    /// The open schedule picker's "Clear" pill (only shown when the task
    /// already has a schedule) — same reasoning, kept separate from
    /// `process_pill_focuses` since it isn't a `ProcessTarget` variant.
    process_clear_focus: FocusHandle,
    /// The "Complete parent and all subtasks?" inline confirm's two
    /// buttons — same single-stable-handle reasoning (only one confirm
    /// shown at a time, tied to the one expanded task).
    confirm_cancel_focus: FocusHandle,
    confirm_yes_focus: FocusHandle,
    /// What `render_task_view` last saw for the expanded task's row —
    /// `(id, schedule_picker_open, scheduling, adding_subtask,
    /// pending_complete_confirm, subtask_count)`. Compared against each
    /// render so `remeasure_task_row` runs only on an actual change, not
    /// every frame — see `render_task_view`'s own comment for why the
    /// expanded row specifically needs this at all.
    last_expanded_signature: Option<(String, bool, bool, bool, Option<String>, usize)>,
    /// Same stale-while-revalidate fallback as `last_tasks`, keyed by
    /// parent task id — without it, ticking a subtask flickers its own
    /// count ("Subtasks (1/3)" → "Subtasks" → back) and its indented list
    /// briefly disappears on the same invalidate-then-refetch gap.
    last_subtasks: HashMap<String, Arc<Vec<Task>>>,
    /// Whether the sidebar's Capture row currently shows the text field
    /// instead of the "+ Capture" button. Stays open across submissions so
    /// rapid successive captures don't need to reopen it each time.
    capturing: bool,
    /// A single long-lived composer, reused across every capture rather
    /// than recreated on each open — cheaper and keeps its own focus/blink
    /// state stable. PRD §6.1's title-only capture; note/schedule/parent
    /// fields are a later Milestone 1 step (see docs/HANDOFF.md).
    capture_input: Entity<ComposerInput>,
    /// Set when `Db::create_task`/`schedule` fails on submit. PRD §6.1:
    /// "show a non-blocking error with Retry on failure, and never discard
    /// typed content" — the failed title is restored into `capture_input`
    /// (see `on_capture_event`) rather than left cleared, and this drives
    /// the inline message + Retry affordance under the field.
    capture_error: Option<gpui::SharedString>,
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
    /// Whether the detail card's title is currently showing `title_input`
    /// instead of the static heading — PRD §11's "edit" verb had no path
    /// at all until this (found by re-checking the acceptance criteria
    /// against the shipped app, the same way the delete-Undo-toast and
    /// Capture-failure gaps were found earlier). A toggle, like
    /// `scheduling`, rather than always-editable like `note_input`: the
    /// title reads as a heading until deliberately clicked/activated,
    /// matching the schedule pill's own click-to-edit convention already
    /// established in this card, instead of inventing a second "always
    /// looks like a text field" treatment.
    editing_title: bool,
    /// Which task `title_input`'s content belongs to — same reasoning as
    /// `note_task_id`.
    title_task_id: Option<String>,
    /// A single long-lived composer for the detail card's title field,
    /// same reuse-not-recreate reasoning as `capture_input`.
    title_input: Entity<ComposerInput>,
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
    /// PRD §6.5 (revised 2026-08-19: EventKit, not Google OAuth). Read at
    /// startup and refreshed after Settings' "Connect Calendar" completes —
    /// there's no EventKit change notification wired up yet (a `NSNotification`
    /// observer, the natural follow-up), so a permission revoked from System
    /// Settings while Flow is already running isn't noticed until relaunch.
    calendar_auth: crate::platform::CalendarAuth,
    /// Set to `true` while `Settings::connect_calendar` awaits the system
    /// permission prompt, so the button can show a busy state instead of
    /// being clickable twice.
    calendar_connecting: bool,
    /// Today's calendar-glance events (PRD §6.5/§6.3), fetched once per
    /// Today-view mount rather than in `calendar_glance`'s own render path —
    /// EventKit's query is synchronous system-framework I/O, so it belongs
    /// behind the same `cx.background_executor().spawn` + `cx.notify()`
    /// convention as every other Flow data fetch, not called fresh on every
    /// frame `render_task_view` draws Today.
    today_calendar_events: Option<Arc<Vec<crate::platform::CalendarEvent>>>,
    /// The Calendar tab's Day/Week switch — Month/Year are
    /// `wayfinder/tickets/eventkit-calendar-tab.md`'s own next slice, not
    /// built yet.
    calendar_view_mode: CalendarViewMode,
    /// The day the Calendar tab is centered on — Day mode shows exactly
    /// this date, Week mode shows the Monday-start week it falls in
    /// (matches the reference screenshot's own "Mon 17 ... Sun 23" header).
    /// Navigated by the tab's Today/‹/› controls.
    calendar_cursor: chrono::NaiveDate,
    /// Calendar ids toggled off in the tab's own sidebar — visible (shown)
    /// is the default, matching every calendar being on the first time the
    /// tab is opened. Not persisted across launches: there's no settings-
    /// persistence file anywhere in this codebase yet (`theme.rs`'s own
    /// `ThemePreference` plumbing is unused dead code), so adding one just
    /// for this would be new infrastructure, not a natural extension of
    /// existing code — a real, disclosed scope cut
    /// (`wayfinder/tickets/eventkit-calendar-tab.md`'s own open question).
    calendar_hidden_ids: HashSet<String>,
    /// The tab's own sidebar list — fetched once when the tab is first
    /// opened (calendars don't change with the visible date range, unlike
    /// `calendar_range_events`), not refetched on every Day/Week/navigate.
    calendar_list: Option<Arc<Vec<crate::platform::CalendarInfo>>>,
    /// Events for whatever range the tab is currently showing, refetched on
    /// every mode switch or navigation — same
    /// `cx.background_executor().spawn` + `cx.notify()` convention as
    /// `today_calendar_events`, for the same render-path-I/O reason.
    calendar_range_events: Option<Arc<Vec<crate::platform::CalendarEvent>>>,
    /// Bumped by every `refresh_calendar_tab` call and captured by its own
    /// async fetch — a superseded fetch (a slower earlier request, e.g. a
    /// full-year query, landing after a faster later one, e.g. flipping
    /// straight back to Day) checks its captured value against the current
    /// one before applying its result, so rapid navigation/mode-switching
    /// can't let stale data overwrite what's actually being shown. Same
    /// superseded-result problem `query.rs`'s own generation counter
    /// solves for `tasks`/`subtasks`, just not routed through
    /// `QueryCache` itself since a calendar fetch's key (mode + cursor)
    /// isn't naturally `View`-shaped.
    calendar_fetch_generation: u64,
    /// One focus handle per calendar row in the tab's sidebar, keyed by
    /// calendar id — same pruned-map pattern as `row_focuses`, pruned
    /// against `calendar_list` whenever it's refetched.
    calendar_row_focuses: HashMap<String, FocusHandle>,
    /// Single stable handles for the tab's own fixed controls (Day/Week
    /// switch, Today/‹/›) — same reasoning as `undo_toast_focus`: at most
    /// one Calendar tab is ever visible at a time.
    calendar_day_focus: FocusHandle,
    calendar_week_focus: FocusHandle,
    calendar_month_focus: FocusHandle,
    calendar_year_focus: FocusHandle,
    calendar_today_focus: FocusHandle,
    calendar_prev_focus: FocusHandle,
    calendar_next_focus: FocusHandle,
    /// Settings' calendar section — "Connect Calendar" and the denied-state
    /// "Open System Settings" link, single stable handles since only one is
    /// ever shown at a time (mutually exclusive on `calendar_auth`).
    settings_connect_calendar_focus: FocusHandle,
    settings_open_privacy_focus: FocusHandle,
}

/// The Calendar tab's Day/Week switch (`wayfinder/tickets/eventkit-calendar-tab.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CalendarViewMode {
    Day,
    Week,
    Month,
    Year,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UndoKind {
    Complete,
    Delete,
}

impl Flow {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        // A session-boundary marker in the debug log, not just a state
        // transition — makes it obvious where one run ends and the next
        // (post-rebuild) run starts when reading the log after several
        // watcher relaunches.
        crate::debug_log!("--- Flow starting ---");
        // Opening a local file is a one-time, sub-millisecond startup cost,
        // not a per-frame one — unlike render-path I/O, this is exactly the
        // "one-shot user action" CLAUDE.md carves out as fine to do
        // synchronously. A failure degrades task views instead of crashing.
        let db = match Db::open() {
            Ok(db) => Some(db),
            Err(error) => {
                // `eprintln!` here has never reliably reached anything: a
                // GUI app launched via macOS's `open -n -W`
                // (`scripts/dev.ts`) doesn't deliver its own stderr back to
                // a watching terminal, in a release build there's no
                // terminal at all. `debug_log!` guarantees at least a
                // debug-build capture; `eprintln!` stays too since it's
                // free and might still reach Console.app's unified log.
                eprintln!("Flow: failed to open the local database: {error:#}");
                crate::debug_log!("failed to open the local database: {error:#}");
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

            // Single-line, Enter-submits — same shape as `subtask_input`,
            // not `note_input`'s multi-line/blur-saves code-editor mode.
            // No placeholder text: it's only ever shown pre-filled with
            // the task's current title, never empty.
            let title_input = cx.new(|cx| ComposerInput::new(window, cx));
            cx.subscribe(&title_input, Self::on_title_event).detach();
            // Enter isn't the only way focus can leave this field — see
            // `on_title_event`'s doc for why a blur handler is also
            // required, not just belt-and-suspenders.
            let title_focus_handle = title_input.read(cx).focus();
            cx.on_blur(&title_focus_handle, window, Self::on_title_blur).detach();

            Self {
                destination: Destination::Inbox,
                new_task_focus: cx.focus_handle(),
                completed_clear_focuses: HashMap::new(),
                undo_toast_focus: cx.focus_handle(),
                detail_delete_focus: cx.focus_handle(),
                title_focus: cx.focus_handle(),
                schedule_pill_focus: cx.focus_handle(),
                process_pill_focuses: array::from_fn(|_| cx.focus_handle()),
                process_clear_focus: cx.focus_handle(),
                confirm_cancel_focus: cx.focus_handle(),
                confirm_yes_focus: cx.focus_handle(),
                nav_focuses: array::from_fn(|_| cx.focus_handle()),
                mode_tasks_focus: cx.focus_handle(),
                mode_thumb: None,
                db,
                tasks: QueryCache::new(8),
                last_tasks: HashMap::new(),
                task_list_states: HashMap::new(),
                task_list_scrollbars: HashMap::new(),
                row_focuses: HashMap::new(),
                completed_row_focuses: HashMap::new(),
                subtask_focuses: HashMap::new(),
                add_subtask_focus: cx.focus_handle(),
                last_expanded_signature: None,
                last_completed: HashMap::new(),
                last_subtasks: HashMap::new(),
                capturing: false,
                capture_input,
                capture_error: None,
                expanded_task_id: None,
                note_task_id: None,
                note_input,
                editing_title: false,
                title_task_id: None,
                title_input,
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
                calendar_auth: crate::platform::calendar_authorization_status(),
                calendar_connecting: false,
                today_calendar_events: None,
                calendar_view_mode: CalendarViewMode::Week,
                calendar_cursor: chrono::Local::now().date_naive(),
                calendar_hidden_ids: HashSet::new(),
                calendar_list: None,
                calendar_range_events: None,
                calendar_fetch_generation: 0,
                calendar_row_focuses: HashMap::new(),
                calendar_day_focus: cx.focus_handle(),
                calendar_week_focus: cx.focus_handle(),
                calendar_month_focus: cx.focus_handle(),
                calendar_year_focus: cx.focus_handle(),
                calendar_today_focus: cx.focus_handle(),
                calendar_prev_focus: cx.focus_handle(),
                calendar_next_focus: cx.focus_handle(),
                settings_connect_calendar_focus: cx.focus_handle(),
                settings_open_privacy_focus: cx.focus_handle(),
            }
        });
        window.set_window_title(&window_title(Destination::Inbox));
        #[cfg(debug_assertions)]
        cx.set_global(FlowDebugHandle(flow.clone()));
        flow
    }

    /// `⌘N` / the sidebar's "+ Capture" row: opens the composer and focuses
    /// it. PRD §6.1: "always-available task composer focused in the current
    /// view."
    pub(super) fn open_capture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.capturing = true;
        self.capture_error = None;
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
        self.capture_error = None;
        self.capture_input.update(cx, |input, cx| input.clear(cx));
        window.focus(&self.new_task_focus, cx);
        cx.notify();
    }

    /// The Retry affordance under a failed capture (`capture_error`): the
    /// original text is already sitting in `capture_input` (restored by
    /// `submit_capture` on failure), so this just resubmits it exactly
    /// like pressing Enter again would.
    pub(super) fn retry_capture(&mut self, cx: &mut Context<Self>) {
        let title = self.capture_input.read(cx).content().to_string();
        if title.is_empty() {
            return;
        }
        self.submit_capture(title, cx);
    }

    fn on_capture_event(
        &mut self,
        _input: Entity<ComposerInput>,
        event: &ComposerEvent,
        cx: &mut Context<Self>,
    ) {
        if matches!(event, ComposerEvent::Edited) {
            // Deliberately does *not* clear `capture_error` here: a failed
            // submit's own restore (`submit_capture`) programmatically
            // calls `set_content`, which emits this same `Edited` event —
            // clearing the error right here would erase it in the same
            // beat it was set. The error instead clears on the next
            // successful submit, or when the field closes.
            self.highlight_capture_date_phrase(cx);
            return;
        }
        let ComposerEvent::Submit(title) = event else {
            return;
        };
        self.submit_capture(title.clone(), cx);
    }

    /// Shared by a fresh Enter submit and the failed-capture Retry click —
    /// same write, same failure handling either way.
    fn submit_capture(&mut self, original_title: String, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else { return };

        // Parsing is pure and cheap (no I/O), so it runs inline rather than
        // adding a second background hop before the write below.
        let parsed = crate::parse::parse(&original_title, chrono::Local::now().date_naive());
        // A title that parsed down to nothing but the date phrase itself
        // (the user typed only "tomorrow") has no title left; PRD §6.1
        // requires a nonempty title, so treat that as unrecognized rather
        // than saving a blank one.
        let (title, date, time) = if parsed.cleaned_title.is_empty() {
            (original_title.clone(), None, None)
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
            if let Err(error) = &created {
                // PRD §10: never log a task's actual title — see
                // `delete_task`'s identical fix for the full reasoning.
                // No task id exists yet to log instead (creation is what
                // just failed), so this line just records that *a*
                // capture failed, not which one.
                crate::debug_log!("capture: FAILED: {error:#}");
                // PRD §6.1: "show a non-blocking error with Retry on
                // failure, and never discard typed content." The field
                // already self-cleared on submit (Composer-mode Enter), so
                // put the user's exact original text back rather than
                // leaving them to retype it.
                let _ = flow.update(cx, |flow, cx| {
                    flow.capture_error = Some("Couldn't save. Try again.".into());
                    flow.capture_input.update(cx, |input, cx| {
                        input.set_content(original_title.clone(), cx)
                    });
                    cx.notify();
                });
                return;
            }
            crate::debug_log!("capture: created");
            let _ = flow.update(cx, |flow, cx| {
                flow.capture_error = None;
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

    /// The detail card's title heading, once clicked or activated (Enter/
    /// Space via `title_focus`): swaps in `title_input`, pre-filled and
    /// focused, for the task already in `expanded_task_id` — same
    /// swap-in-place idiom as `open_add_subtask`/`focus_schedule_field`.
    pub(super) fn start_editing_title(
        &mut self,
        id: String,
        title: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.editing_title = true;
        self.title_task_id = Some(id);
        self.title_input.update(cx, |input, cx| input.set_content(title, cx));
        let focus = self.title_input.read(cx).focus();
        window.focus(&focus, cx);
        cx.notify();
    }

    /// PRD §11's "edit" verb — found missing entirely (no way to rename a
    /// task anywhere in the app) by re-checking the acceptance criteria
    /// against the shipped app, the same way earlier gaps were found.
    /// Enter-submits, matching `on_subtask_event`'s shape rather than the
    /// note field's blur-only one — but that alone reintroduced the exact
    /// bug `set_expanded_task` already fixed for `note_input`: clicking
    /// anywhere that isn't itself `track_focus`'d (the detail card's own
    /// completion checkbox, for instance) does not blur the previously
    /// focused element in GPUI, so a typed-but-unsubmitted title could
    /// still be silently discarded on collapse/task-switch. Fixed the
    /// same way notes were: `flush_title` below runs proactively from
    /// `set_expanded_task`, plus a real blur handler for the cases that
    /// do land on another focusable control (Tab, or the delete button).
    fn on_title_event(&mut self, _input: Entity<ComposerInput>, event: &ComposerEvent, cx: &mut Context<Self>) {
        let ComposerEvent::Submit(title) = event else {
            return;
        };
        let Some(id) = self.title_task_id.clone() else {
            return;
        };
        let title = title.trim().to_string();
        if title.is_empty() {
            // PRD §6.1: a task needs a nonempty title. Reject rather than
            // save blank — leaves editing_title on so the user sees their
            // (empty) field and can fix it, instead of silently reverting
            // to the old title with no explanation.
            return;
        }
        let Some(db) = self.db.clone() else { return };
        self.editing_title = false;
        cx.spawn(async move |flow, cx| {
            let result = cx.background_executor().spawn(async move { db.set_title(id, title) }).await;
            if result.is_err() {
                return;
            }
            let _ = flow.update(cx, |flow, cx| {
                // A rename changes what every row already shows, unlike a
                // note edit — every view's cache needs to see the new
                // title, not just the closing detail card, so this
                // invalidates broadly rather than just the origin view
                // (same reasoning `delete_task` already uses).
                flow.invalidate_all_views();
                cx.notify();
            });
        })
        .detach();
    }

    /// `title_input` lost focus: the safety-net save trigger, same role
    /// `on_note_blur` plays for the note field. See `on_title_event`'s doc
    /// for why Submit alone wasn't enough.
    fn on_title_blur(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.flush_title(cx);
    }

    /// Commits whatever `title_input` currently holds if it's a valid
    /// nonempty title, and always leaves edit mode either way — called
    /// proactively from `set_expanded_task` (collapsing or switching
    /// tasks) and from `on_title_blur`. Deliberately different from
    /// `on_title_event`'s own Submit handling: Submit leaves an empty
    /// field open so the user — who is still actively there — can fix
    /// it, but by the time this runs the user has already moved on
    /// (clicked away, switched tasks), so there's no field left to leave
    /// open; an empty result here just abandons the edit and keeps the
    /// task's existing title, the same "harmless to no-op" reasoning
    /// `flush_note` already follows.
    fn flush_title(&mut self, cx: &mut Context<Self>) {
        let was_editing = self.editing_title;
        self.editing_title = false;
        if !was_editing {
            return;
        }
        let Some(id) = self.title_task_id.clone() else { return };
        let title = self.title_input.read(cx).content().trim().to_string();
        if title.is_empty() {
            return; // Abandon: PRD §6.1 forbids saving a blank title.
        }
        let Some(db) = self.db.clone() else { return };
        cx.spawn(async move |flow, cx| {
            let result = cx.background_executor().spawn(async move { db.set_title(id, title) }).await;
            if result.is_err() {
                return;
            }
            let _ = flow.update(cx, |flow, cx| {
                flow.invalidate_all_views();
                cx.notify();
            });
        })
        .detach();
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

    /// Hidden dev shortcut (Cmd-Option-I), no menu item. GPUI's element
    /// inspector only compiles under `debug_assertions`
    /// (`gpui::Window::toggle_inspector`), so the release build keeps the
    /// keybinding wired but no-ops rather than forking the keymap — see
    /// `lib.rs`'s binding comment.
    #[allow(unused_variables)]
    fn handle_toggle_inspector_action(
        &mut self,
        _: &ToggleInspector,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        #[cfg(debug_assertions)]
        window.toggle_inspector(cx);
    }

    fn handle_cancel_turn_action(&mut self, _: &CancelTurn, window: &mut Window, cx: &mut Context<Self>) {
        if self.scheduling || self.expanded_task_id.is_some() {
            self.scheduling = false;
            self.schedule_picker_open = false;
            self.adding_subtask = false;
            self.editing_title = false;
            self.pending_complete_confirm = None;
            self.set_expanded_task(None, cx);
            self.schedule_input.update(cx, |input, cx| input.clear(cx));
            self.subtask_input.update(cx, |input, cx| input.clear(cx));
            self.title_input.update(cx, |input, cx| input.clear(cx));
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
            self.flush_title(cx);
        }
        self.expanded_task_id = id;
    }

    /// Settings' "Connect Calendar" button (PRD §6.5). Triggers the system
    /// EventKit permission prompt; a no-op re-prompt if the user already
    /// decided (granted or denied) — EventKit only ever prompts once, same
    /// as `platform::calendar_request_access`'s own doc says.
    pub(super) fn connect_calendar(&mut self, cx: &mut Context<Self>) {
        if self.calendar_connecting {
            return;
        }
        self.calendar_connecting = true;
        cx.notify();
        cx.spawn(async move |flow, cx| {
            let status = crate::platform::calendar_request_access().await;
            let _ = flow.update(cx, |flow, cx| {
                flow.calendar_connecting = false;
                flow.calendar_auth = status;
                flow.today_calendar_events = None;
                flow.calendar_list = None;
                flow.calendar_range_events = None;
                // Settings' own destination doesn't change here, so
                // `set_destination`'s Today/Calendar-arrival fetches never
                // run for this grant — fetch directly if either is already
                // open behind Settings (e.g. reopened from a previous
                // session).
                if flow.destination == Destination::Today {
                    flow.refresh_today_calendar_events(cx);
                }
                if flow.destination == Destination::Calendar {
                    flow.refresh_calendar_tab(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Fetches today's events once per Today-view mount (`set_destination`'s
    /// own call site) rather than from `calendar_glance`'s render path —
    /// see `today_calendar_events`'s own field doc for why.
    pub(super) fn refresh_today_calendar_events(&mut self, cx: &mut Context<Self>) {
        if self.calendar_auth != crate::platform::CalendarAuth::Granted {
            self.today_calendar_events = None;
            return;
        }
        cx.spawn(async move |flow, cx| {
            let events = cx
                .background_executor()
                .spawn(async move {
                    let start = crate::platform::local_midnight(chrono::Local::now().date_naive());
                    let end = start + chrono::Duration::days(1);
                    crate::platform::calendar_events_between(start, end)
                })
                .await;
            let _ = flow.update(cx, |flow, cx| {
                flow.today_calendar_events = Some(Arc::new(events));
                cx.notify();
            });
        })
        .detach();
    }

    /// The Calendar tab's own date range for its current mode/cursor — Day:
    /// just `calendar_cursor`; Week: the Monday-start week it falls in,
    /// matching the reference screenshot's own "Mon 17 ... Sun 23" header
    /// (chrono's ISO-week default, not a separate convention invented for
    /// this).
    pub(super) fn calendar_visible_range(&self) -> (chrono::NaiveDate, chrono::NaiveDate) {
        use chrono::Datelike;
        match self.calendar_view_mode {
            CalendarViewMode::Day => (self.calendar_cursor, self.calendar_cursor),
            CalendarViewMode::Week => {
                let week = self.calendar_cursor.week(chrono::Weekday::Mon);
                (week.first_day(), week.last_day())
            }
            // The visible month expanded to the full Monday-start weeks
            // shown at its edges, matching Apple Calendar's own month grid
            // (the ticket's own stated requirement) rather than stopping
            // mid-week at the 1st/last day.
            CalendarViewMode::Month => {
                let first_of_month = self.calendar_cursor.with_day(1).unwrap_or(self.calendar_cursor);
                let last_of_month = first_of_month
                    .checked_add_months(chrono::Months::new(1))
                    .and_then(|next| next.pred_opt())
                    .unwrap_or(first_of_month);
                (
                    first_of_month.week(chrono::Weekday::Mon).first_day(),
                    last_of_month.week(chrono::Weekday::Mon).last_day(),
                )
            }
            CalendarViewMode::Year => (
                chrono::NaiveDate::from_ymd_opt(self.calendar_cursor.year(), 1, 1)
                    .unwrap_or(self.calendar_cursor),
                chrono::NaiveDate::from_ymd_opt(self.calendar_cursor.year(), 12, 31)
                    .unwrap_or(self.calendar_cursor),
            ),
        }
    }

    /// Fetches the Calendar tab's own sidebar list (once — see
    /// `calendar_list`'s field doc) and this range's events. Called from
    /// `set_destination` on arrival at the tab and after every mode switch
    /// or navigation — never from the tab's own render path, same
    /// render-path-I/O reasoning as `refresh_today_calendar_events`.
    pub(super) fn refresh_calendar_tab(&mut self, cx: &mut Context<Self>) {
        if self.calendar_auth != crate::platform::CalendarAuth::Granted {
            self.calendar_list = None;
            self.calendar_range_events = None;
            return;
        }
        let (start_date, end_date) = self.calendar_visible_range();
        let fetch_list = self.calendar_list.is_none();
        self.calendar_fetch_generation += 1;
        let generation = self.calendar_fetch_generation;
        cx.spawn(async move |flow, cx| {
            let (list, events) = cx
                .background_executor()
                .spawn(async move {
                    let list = fetch_list.then(crate::platform::calendar_list);
                    let start = crate::platform::local_midnight(start_date);
                    let end = crate::platform::local_midnight(end_date) + chrono::Duration::days(1);
                    let events = crate::platform::calendar_events_between(start, end);
                    (list, events)
                })
                .await;
            let _ = flow.update(cx, |flow, cx| {
                // A newer navigation/mode-switch already started its own
                // fetch since this one began — this result is for a range
                // nobody's looking at anymore, so it's dropped rather than
                // clobbering whatever the newer fetch lands with.
                if flow.calendar_fetch_generation != generation {
                    return;
                }
                if let Some(list) = list {
                    flow.prune_calendar_row_focuses(&list);
                    flow.calendar_list = Some(Arc::new(list));
                }
                flow.calendar_range_events = Some(Arc::new(events));
                cx.notify();
            });
        })
        .detach();
    }

    /// The Calendar tab's Day/Week switch.
    pub(super) fn set_calendar_view_mode(&mut self, mode: CalendarViewMode, cx: &mut Context<Self>) {
        if self.calendar_view_mode == mode {
            return;
        }
        self.calendar_view_mode = mode;
        self.refresh_calendar_tab(cx);
        cx.notify();
    }

    /// The tab's ‹/Today/› controls. `step` is `-1`/`0`/`1` (back/today/
    /// forward) — the actual jump size depends on the current mode (a day,
    /// a week, a month, or a year), decided here rather than by each
    /// caller, so `calendar.rs`'s nav buttons don't need to know the
    /// mode-to-jump-size mapping themselves.
    pub(super) fn navigate_calendar(&mut self, step: i64, cx: &mut Context<Self>) {
        self.calendar_cursor = if step == 0 {
            chrono::Local::now().date_naive()
        } else {
            match self.calendar_view_mode {
                CalendarViewMode::Day => self.calendar_cursor + chrono::Duration::days(step),
                CalendarViewMode::Week => self.calendar_cursor + chrono::Duration::days(step * 7),
                CalendarViewMode::Month => {
                    let months = chrono::Months::new(1);
                    if step > 0 {
                        self.calendar_cursor.checked_add_months(months)
                    } else {
                        self.calendar_cursor.checked_sub_months(months)
                    }
                    .unwrap_or(self.calendar_cursor)
                }
                CalendarViewMode::Year => {
                    let years = chrono::Months::new(12);
                    if step > 0 {
                        self.calendar_cursor.checked_add_months(years)
                    } else {
                        self.calendar_cursor.checked_sub_months(years)
                    }
                    .unwrap_or(self.calendar_cursor)
                }
            }
        };
        self.refresh_calendar_tab(cx);
        cx.notify();
    }

    /// The Year grid's click-to-drill-down: jumps straight to Month mode
    /// for the clicked month, rather than making someone switch modes and
    /// navigate month-by-month to get there.
    pub(super) fn jump_to_month(&mut self, first_day: chrono::NaiveDate, cx: &mut Context<Self>) {
        self.calendar_view_mode = CalendarViewMode::Month;
        self.calendar_cursor = first_day;
        self.refresh_calendar_tab(cx);
        cx.notify();
    }

    /// The sidebar's per-calendar visibility toggle.
    pub(super) fn toggle_calendar_visibility(&mut self, id: String, cx: &mut Context<Self>) {
        if !self.calendar_hidden_ids.remove(&id) {
            self.calendar_hidden_ids.insert(id);
        }
        cx.notify();
    }

    /// Gets or creates the keyboard focus handle for one calendar-tab
    /// sidebar row — same pattern as `Flow::row_focus`.
    pub(super) fn calendar_row_focus(&mut self, id: &str, cx: &mut Context<Self>) -> FocusHandle {
        self.calendar_row_focuses.entry(id.to_string()).or_insert_with(|| cx.focus_handle()).clone()
    }

    fn prune_calendar_row_focuses(&mut self, list: &[crate::platform::CalendarInfo]) {
        self.calendar_row_focuses.retain(|id, _| list.iter().any(|calendar| &calendar.id == id));
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
        crate::debug_log!("destination: {:?} -> {destination:?}", self.destination);
        self.destination = destination;
        window.set_window_title(&window_title(destination));
        if destination == Destination::Today {
            self.refresh_today_calendar_events(cx);
        }
        if destination == Destination::Calendar {
            self.refresh_calendar_tab(cx);
        }
        cx.notify();
    }

    /// A plain-text dump of the fields most worth seeing while debugging
    /// blind (no screen-recording access): what's open, what's mid-flight,
    /// what's cached. Read by `app::inspector`'s Cmd-Option-I panel via
    /// [`FlowDebugHandle`] — this is the "app state" half of that panel,
    /// alongside GPUI's own per-element style dump.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_snapshot(&self) -> String {
        format!(
            "destination: {:?}\n\
             capturing: {} (error: {:?})\n\
             expanded_task_id: {:?}\n\
             schedule_picker_open: {} scheduling: {} adding_subtask: {}\n\
             pending_complete_confirm: {:?}\n\
             selected_task_ids: {}\n\
             completing_ids: {}\n\
             undo_toast: {}\n\
             cached views (tasks/completed/last_tasks/last_completed): {}/{}/{}/{}\n\
             cached subtask parents: {}\n\
             virtualized list states/scrollbars: {}/{}\n\
             keyboard focus handles (row/subtask/completed): {}/{}/{}\n\
             calendar_auth: {:?} (connecting: {}, today events cached: {})",
            self.destination,
            self.capturing,
            self.capture_error,
            self.expanded_task_id,
            self.schedule_picker_open,
            self.scheduling,
            self.adding_subtask,
            self.pending_complete_confirm,
            self.selected_task_ids.len(),
            self.completing_ids.len(),
            match &self.undo_toast {
                // PRD §10: task titles are private, "in UI, logs, and
                // telemetry" — the toast's own task id is enough to
                // identify it here without also echoing its title into a
                // panel whose whole purpose is being read (or
                // screenshotted) by whoever's debugging.
                Some(toast) => format!(
                    "{:?} of {} ({:?})",
                    toast.kind, toast.task_id, toast.origin_view
                ),
                None => "none".to_string(),
            },
            self.tasks.len(),
            self.completed_tasks.len(),
            self.last_tasks.len(),
            self.last_completed.len(),
            self.last_subtasks.len(),
            // Added alongside the virtualized-list and keyboard-
            // accessibility work later this session — kept here so this
            // snapshot doesn't quietly go stale against Flow's actual
            // field list again. task_list_states/scrollbars are bounded
            // to the 4 flat views (Upcoming excluded, see their own field
            // docs); row_focuses is bounded to visible rows, subtask_focuses
            // to the one expanded task's subtasks — see each field's doc
            // for the exact pruning contract if either grows unexpectedly.
            self.task_list_states.len(),
            self.task_list_scrollbars.len(),
            self.row_focuses.len(),
            self.subtask_focuses.len(),
            self.completed_row_focuses.len(),
            self.calendar_auth,
            self.calendar_connecting,
            self.today_calendar_events.is_some(),
        )
    }
}

/// A handle to the running `Flow` entity, set once in `Flow::new` — lets a
/// debug-only surface with no `Flow` of its own (the Cmd-Option-I inspector
/// is a separate GPUI overlay entity) read live app state for display. Not
/// read by any product code path.
#[cfg(debug_assertions)]
pub(crate) struct FlowDebugHandle(pub Entity<Flow>);
#[cfg(debug_assertions)]
impl gpui::Global for FlowDebugHandle {}

fn window_title(destination: Destination) -> String {
    format!("{} — Flow", destination.label())
}
