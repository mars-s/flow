//! Milestone 1: the five task views (Inbox, Today, Upcoming, Anytime,
//! Someday), all backed by `crate::db::View` and sharing one row renderer.

use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, ClickEvent, Context, Entity, FocusHandle, IntoElement,
    KeyDownEvent, ListAlignment, ListState, ParentElement, SharedString, Styled, Window, div,
    ease_out_quint, list, prelude::*, px,
};

use super::Flow;
use super::{UndoKind, UndoToast};
use super::sidebar::Destination;
use crate::db::{Bucket, Task, View};
use crate::input::ComposerInput;
use crate::query::Query;
use crate::theme::Theme;
use crate::ui::motion;

/// PRD §7's completion-collapse timing; reused here for the row's fade-in,
/// the completed-section reveal, and the undo toast too, since nothing in
/// the direction doc distinguishes them — see `crate::ui::motion::TRANSITION`.
const ROW_TRANSITION: Duration = motion::TRANSITION;

/// How far the expanded "Completed" section grows upward before it scrolls
/// internally instead of pushing the open-task list any further.
const COMPLETED_MAX_HEIGHT: f32 = 280.0;

/// PRD §6.1 names this window for delete's undo toast; completion's own
/// spec doesn't state a duration, so it reuses the same one for consistency.
const UNDO_TOAST_DURATION: Duration = Duration::from_secs(10);

/// The three quick fixed destinations PRD §6.3 names for the task detail
/// card's schedule picker. The fourth, "Schedule" (an arbitrary date), is a
/// free-text field parsed by `parse.rs` rather than one of these — see
/// `render_process_row`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ProcessTarget {
    Today,
    Anytime,
    Someday,
}

impl ProcessTarget {
    const ALL: [ProcessTarget; 3] = [Self::Today, Self::Anytime, Self::Someday];

    fn label(self) -> &'static str {
        match self {
            ProcessTarget::Today => "Today",
            ProcessTarget::Anytime => "Anytime",
            ProcessTarget::Someday => "Someday",
        }
    }

    fn bucket(self) -> Bucket {
        match self {
            ProcessTarget::Today | ProcessTarget::Anytime => Bucket::Active,
            ProcessTarget::Someday => Bucket::Someday,
        }
    }

    /// Reuses the sidebar's icon for the same concept (`sidebar.rs`'s
    /// `Destination::icon_path`) so Today/Anytime/Someday read as the same
    /// idea in both places — no new icon assets, no per-item color per
    /// `docs/DESIGN_DIRECTION.md`'s single-accent rule.
    fn icon_path(self) -> &'static str {
        match self {
            ProcessTarget::Today => "icons/star.svg",
            ProcessTarget::Anytime => "icons/layers.svg",
            ProcessTarget::Someday => "icons/archive.svg",
        }
    }
}

impl Flow {
    /// Reads a cached view, kicking off a background fetch on a miss. Safe
    /// to call from `render` — the miss path only spawns work, per
    /// `query.rs`'s own doc comment (the pattern this follows exactly).
    pub(super) fn read_view(&mut self, view: View, cx: &mut Context<Self>) -> Query<View, Vec<Task>> {
        let query = self.tasks.read(&view);
        if let Query::Missing(token) = &query {
            let Some(db) = self.db.clone() else {
                return query;
            };
            let token = token.clone();
            cx.spawn(async move |flow, cx| {
                let Ok(tasks) = cx
                    .background_executor()
                    .spawn(async move { db.list_view(view) })
                    .await
                else {
                    return;
                };
                let _ = flow.update(cx, |flow, cx| {
                    if flow.tasks.fulfill(token, tasks) {
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        query
    }

    /// Invalidates every task view's cache. Placement/deletion writes can
    /// move a task between any of the five, and the write is a one-shot
    /// user action rather than a per-frame cost, so invalidating all five
    /// beats threading the exact affected set through every call site.
    /// `pub(super)` (not private) so `app.rs`'s `on_title_event` can reuse
    /// it too — a rename can touch a task in any of the five views, same
    /// reasoning as delete/schedule, so it needs the same broad
    /// invalidation rather than `submit_capture`'s narrower one.
    pub(super) fn invalidate_all_views(&mut self) {
        // Routes through `invalidate_view` (not a direct `self.tasks`
        // touch) specifically so this also clears `completed_tasks` — a
        // delete or bulk-delete can remove a row from either cache, and
        // the previous version only cleared `tasks`, so a deleted task
        // that was already completed stayed visible in its "Completed"
        // section forever (the cache never refetched to notice it was
        // gone).
        for view in [
            View::Inbox,
            View::Today,
            View::Upcoming,
            View::Anytime,
            View::Someday,
        ] {
            self.invalidate_view(view);
        }
    }

    /// Invalidates both an open view and its collapsed "Completed" section —
    /// every write that moves a task in or out of one can also move it in or
    /// out of the other, so the two caches always travel together.
    fn invalidate_view(&mut self, view: View) {
        crate::debug_log!("cache: invalidate {view:?}");
        self.tasks.invalidate(&view);
        self.completed_tasks.invalidate(&view);
    }

    /// Creates `view`'s virtualized-list state on first use, or tells GPUI
    /// its item count changed on every later call — see the field doc on
    /// `Flow::task_list_states` for the scroll-position tradeoff this
    /// whole-range splice accepts.
    fn sync_task_list_state(&mut self, view: View, tasks: &[Task]) {
        match self.task_list_states.get(&view) {
            Some(state) => state.splice(0..state.item_count(), tasks.len()),
            None => {
                self.task_list_states.insert(
                    view,
                    ListState::new(tasks.len(), ListAlignment::Top, px(600.0)),
                );
            }
        }
    }

    /// Finds which flat (non-Upcoming) view currently shows `task_id` and
    /// tells that view's `ListState` to remeasure just that one row — see
    /// `render_task_view`'s expanded-row-signature comment for why this
    /// gets called. A subtask, or a task in Upcoming (not virtualized),
    /// simply has no entry in `last_tasks`/`task_list_states` and is a
    /// harmless no-op here.
    fn remeasure_task_row(&self, task_id: &str) {
        for (view, tasks) in &self.last_tasks {
            let Some(ix) = tasks.iter().position(|task| task.id == task_id) else { continue };
            if let Some(state) = self.task_list_states.get(view) {
                state.remeasure_items(ix..ix + 1);
            }
            return;
        }
    }

    /// Gets or creates the keyboard focus handle for one task row —
    /// `Flow::row_focuses`'s field doc has the full reasoning.
    pub(super) fn row_focus(&mut self, task_id: &str, cx: &mut Context<Self>) -> FocusHandle {
        self.row_focuses
            .entry(task_id.to_string())
            .or_insert_with(|| cx.focus_handle())
            .clone()
    }

    /// Gets or creates the "Clear completed" button's focus handle for one
    /// view — `Flow::completed_clear_focuses`'s field doc has the full
    /// reasoning.
    fn completed_clear_focus(&mut self, view: View, cx: &mut Context<Self>) -> FocusHandle {
        self.completed_clear_focuses.entry(view).or_insert_with(|| cx.focus_handle()).clone()
    }

    /// Drops focus handles for any task id no longer present in `tasks` —
    /// called from `render_task_view` alongside its other per-list-refetch
    /// bookkeeping (`completing_ids` pruning, `sync_task_list_state`), so
    /// `row_focuses` stays bounded to what's actually visible rather than
    /// accumulating one entry per task ever seen in a session. The pruned
    /// row was already gone from the render tree the same frame this runs
    /// (it's no longer in `tasks`), so this can't blur something still on
    /// screen — dropping its handle here is bookkeeping cleanup, not a
    /// focus-movement decision in its own right.
    fn prune_row_focuses(&mut self, tasks: &[Task]) {
        self.row_focuses.retain(|id, _| tasks.iter().any(|task| &task.id == id));
    }

    /// Reads a view's completed tasks, kicking off a background fetch on a
    /// miss — same `QueryCache` read-through pattern as `read_view`, against
    /// the parallel `completed_tasks` cache.
    pub(super) fn read_completed(&mut self, view: View, cx: &mut Context<Self>) -> Query<View, Vec<Task>> {
        let query = self.completed_tasks.read(&view);
        if let Query::Missing(token) = &query {
            let Some(db) = self.db.clone() else {
                return query;
            };
            let token = token.clone();
            cx.spawn(async move |flow, cx| {
                let Ok(tasks) = cx
                    .background_executor()
                    .spawn(async move { db.list_completed(view) })
                    .await
                else {
                    return;
                };
                let _ = flow.update(cx, |flow, cx| {
                    if flow.completed_tasks.fulfill(token, tasks) {
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        query
    }

    /// A task's direct subtasks, kicking off a background fetch on a miss —
    /// same `QueryCache` read-through pattern as `read_view`/`read_completed`,
    /// keyed by parent task id rather than `View`. Only ever called for the
    /// currently expanded task (see `render_detail_card`), not per row.
    pub(super) fn read_subtasks(&mut self, parent_id: &str, cx: &mut Context<Self>) -> Query<String, Vec<Task>> {
        let query = self.subtasks.read(&parent_id.to_string());
        if let Query::Missing(token) = &query {
            let Some(db) = self.db.clone() else {
                return query;
            };
            let token = token.clone();
            let parent_id = parent_id.to_string();
            cx.spawn(async move |flow, cx| {
                let Ok(subtasks) = cx
                    .background_executor()
                    .spawn(async move { db.list_subtasks(parent_id) })
                    .await
                else {
                    return;
                };
                let _ = flow.update(cx, |flow, cx| {
                    if flow.subtasks.fulfill(token, subtasks) {
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        query
    }

    /// The "Completed (N)" row's click: expands or collapses that view's
    /// section. Collapsed by default (`Flow::new`'s empty `completed_expanded`).
    fn toggle_completed_expanded(&mut self, view: View, cx: &mut Context<Self>) {
        if !self.completed_expanded.remove(&view) {
            self.completed_expanded.insert(view);
        }
        cx.notify();
    }

    /// Reopening (unchecking) writes immediately — nothing to animate out,
    /// the row is already visible and just needs to update. Completing
    /// instead waits for `ROW_TRANSITION`'s fade/collapse (`toggle_completed`)
    /// so the row has something to animate before it actually disappears.
    /// Which cache entries to invalidate is decided by `origin_view` rather
    /// than guessed from the task's bucket, since Today and Upcoming both
    /// read `Bucket::Active` and either could be the row the click came from.
    fn write_completed(&mut self, id: String, completed: bool, origin_view: View, cx: &mut Context<Self>) {
        crate::debug_log!(
            "task {id}: write completed={completed} (origin {origin_view:?})"
        );
        // Reopening — whether from the checkbox or the Undo toast — is
        // always an immediate, deliberate reversal, so it clears
        // `completing_ids` synchronously here rather than waiting for
        // `render_task_view`'s fresh-fetch pruning to notice. That pruning
        // alone isn't enough for this path: if Undo lands before the
        // completing write's own refetch has resolved, a fresh `Ready`
        // will show the task present again (reopened) — which the pruning
        // reads as "still there, keep waiting" — and the row would be
        // stuck showing its collapsed/checked state forever.
        if !completed {
            self.completing_ids.remove(&id);
        }
        let Some(db) = self.db.clone() else { return };
        cx.spawn(async move |flow, cx| {
            let write_id = id.clone();
            let Ok(()) = cx
                .background_executor()
                .spawn(async move { db.set_completed(write_id, completed) })
                .await
            else {
                crate::debug_log!("task {id}: write completed={completed} FAILED");
                if completed {
                    // Without this, a failed completing-write leaves the
                    // row permanently stuck showing as collapsed/checked —
                    // the pruning in render_task_view only clears
                    // completing_ids once a fresh fetch confirms the write
                    // landed, which never happens if it didn't.
                    let _ = flow.update(cx, |flow, cx| {
                        flow.completing_ids.remove(&id);
                        cx.notify();
                    });
                }
                return;
            };
            let _ = flow.update(cx, |flow, cx| {
                // Deliberately NOT cleared here. `invalidate_view` below
                // evicts the cache entry, but the replacement fetch is
                // async — for at least one more render, `render_task_view`
                // falls back to `last_tasks`' stale snapshot, which was
                // captured *before* this write and still has the task
                // un-completed. Clearing `completing_ids` here (a version
                // of this fix tried exactly that) meant `is_completing`
                // went false one frame before the stale fallback's data
                // caught up, so the row flashed back to a normal, unchecked,
                // full-height row — a fresh mount fading itself back in —
                // for that gap. The single source of truth for "is this row
                // actually gone yet" is a fresh `Query::Ready` that no
                // longer contains the id, pruned in `render_task_view`
                // itself once that arrives, not a guess made here about
                // how long the round trip will take.
                match origin_view {
                    View::Inbox | View::Someday => flow.invalidate_view(origin_view),
                    View::Today | View::Upcoming | View::Anytime => {
                        for view in [View::Today, View::Upcoming, View::Anytime] {
                            flow.invalidate_view(view);
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The completion checkbox. Reopening reverses immediately
    /// (`docs/PRODUCT_REQUIREMENTS.md` §7: "Reopening reverses it").
    /// Completing checks the control immediately (the caller's optimistic
    /// styling) but delays the actual write until `render_task_row`'s
    /// 180 ms fade/collapse animation finishes, then shows a 10-second Undo
    /// toast — matching the deletion undo window PRD §6.1 already names,
    /// since completion's own spec doesn't state a different one.
    fn toggle_completed(
        &mut self,
        id: String,
        title: String,
        completed: bool,
        origin_view: View,
        cx: &mut Context<Self>,
    ) {
        if !completed {
            self.write_completed(id, false, origin_view, cx);
            return;
        }
        if !self.completing_ids.insert(id.clone()) {
            return; // Already animating out from an earlier click.
        }
        cx.notify();
        cx.spawn(async move |flow, cx| {
            cx.background_executor().timer(ROW_TRANSITION).await;
            let _ = flow.update(cx, |flow, cx| {
                // `completing_ids` stays set until `write_completed`'s own
                // write actually lands — see the comment there.
                flow.write_completed(id.clone(), true, origin_view, cx);
                flow.show_undo_toast(id, title, origin_view, UndoKind::Complete, cx);
            });
        })
        .detach();
    }

    /// Shows (or replaces) the single-slot toast and schedules its own
    /// dismissal — shared by completion and deletion (PRD §6.1 names the
    /// 10-second window for deletion specifically; completion's own spec
    /// doesn't state a different one, so it reuses it). `token`
    /// distinguishes this toast's timer from an earlier one still in
    /// flight — without it, a second action inside the first one's window
    /// would let the first timer clear the second toast early.
    fn show_undo_toast(
        &mut self,
        task_id: String,
        title: String,
        origin_view: View,
        kind: UndoKind,
        cx: &mut Context<Self>,
    ) {
        self.undo_token += 1;
        let token = self.undo_token;
        self.undo_toast = Some(UndoToast { task_id, title: title.into(), origin_view, token, kind });
        cx.notify();
        cx.spawn(async move |flow, cx| {
            cx.background_executor().timer(UNDO_TOAST_DURATION).await;
            let _ = flow.update(cx, |flow, cx| {
                if flow.undo_toast.as_ref().is_some_and(|toast| toast.token == token) {
                    flow.undo_toast = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// The toast's "Undo" button: reverses whichever action showed it
    /// (reopens a completed task, or restores a deleted one) and dismisses
    /// the toast.
    fn undo_last_action(&mut self, cx: &mut Context<Self>) {
        let Some(toast) = self.undo_toast.take() else { return };
        cx.notify();
        match toast.kind {
            UndoKind::Complete => self.write_completed(toast.task_id, false, toast.origin_view, cx),
            UndoKind::Delete => {
                let Some(db) = self.db.clone() else { return };
                let task_id = toast.task_id.clone();
                cx.spawn(async move |flow, cx| {
                    let result = cx
                        .background_executor()
                        .spawn(async move { db.restore_task(toast.task_id) })
                        .await;
                    if let Err(error) = result {
                        crate::debug_log!("task {task_id}: undo (restore) FAILED: {error:#}");
                        return;
                    }
                    let _ = flow.update(cx, |flow, cx| {
                        flow.invalidate_all_views();
                        cx.notify();
                    });
                })
                .detach();
            }
        }
    }

    /// The detail card's checkbox, when the task has subtasks: gates on
    /// whether any are still open. PRD §6.2: "Completing a parent with
    /// incomplete children asks: 'Complete parent and all subtasks' or
    /// 'Cancel.' It never leaves a completed parent with open children."
    /// `has_open_subtasks` is decided by the caller (`render_detail_card`)
    /// from the subtasks it already has loaded, rather than this method
    /// re-reading the cache.
    fn request_complete(
        &mut self,
        id: String,
        title: String,
        has_open_subtasks: bool,
        origin_view: View,
        cx: &mut Context<Self>,
    ) {
        if has_open_subtasks {
            self.pending_complete_confirm = Some(id);
            cx.notify();
            return;
        }
        self.toggle_completed(id, title, true, origin_view, cx);
    }

    /// The inline confirm's "Cancel" — leaves the parent and its subtasks
    /// untouched.
    fn cancel_complete_confirm(&mut self, cx: &mut Context<Self>) {
        self.pending_complete_confirm = None;
        cx.notify();
    }

    /// The inline confirm's "Complete parent and all subtasks" — completes
    /// every still-open subtask (immediate writes, same as
    /// `toggle_subtask_completed`, no collapse animation since they stay
    /// visible under the parent either way) and the parent itself through
    /// the normal animated path.
    fn confirm_complete_with_subtasks(
        &mut self,
        id: String,
        title: String,
        open_subtask_ids: Vec<String>,
        origin_view: View,
        cx: &mut Context<Self>,
    ) {
        self.pending_complete_confirm = None;
        if let Some(db) = self.db.clone() {
            let parent_id = id.clone();
            cx.spawn(async move |flow, cx| {
                let _ = cx
                    .background_executor()
                    .spawn(async move {
                        for subtask_id in open_subtask_ids {
                            if let Err(error) = db.set_completed(subtask_id.clone(), true) {
                                eprintln!(
                                    "Flow: confirm_complete_with_subtasks failed for subtask {subtask_id}: {error:#}"
                                );
                                crate::debug_log!(
                                    "subtask {subtask_id}: confirm_complete_with_subtasks FAILED: {error:#}"
                                );
                            }
                        }
                    })
                    .await;
                let _ = flow.update(cx, |flow, cx| {
                    flow.subtasks.invalidate(&parent_id);
                    cx.notify();
                });
            })
            .detach();
        }
        self.toggle_completed(id, title, true, origin_view, cx);
    }

    /// A subtask's own completion circle — unlike a top-level task, there's
    /// no collapse-and-disappear here (`list_subtasks` keeps completed
    /// children visible under their expanded parent), so this just writes
    /// immediately.
    fn toggle_subtask_completed(&mut self, id: String, parent_id: String, completed: bool, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else { return };
        cx.spawn(async move |flow, cx| {
            let write_id = id.clone();
            let Ok(()) = cx
                .background_executor()
                .spawn(async move { db.set_completed(write_id, completed) })
                .await
            else {
                crate::debug_log!("subtask {id}: write completed={completed} FAILED");
                return;
            };
            let _ = flow.update(cx, |flow, cx| {
                flow.subtasks.invalidate(&parent_id);
                cx.notify();
            });
        })
        .detach();
    }

    /// Opens or closes a task's detail card — `docs/DESIGN_DIRECTION.md`'s
    /// "Task detail" component, available from every task view now, not
    /// only Inbox. Loads the task's current note into `note_input`.
    fn toggle_expanded(&mut self, id: String, note: Option<String>, cx: &mut Context<Self>) {
        if self.expanded_task_id.as_deref() == Some(id.as_str()) {
            self.schedule_picker_open = false;
            self.scheduling = false;
            self.adding_subtask = false;
            self.pending_complete_confirm = None;
            // Deliberately NOT `self.editing_title = false` here — it has
            // to still read `true` when `set_expanded_task` runs below,
            // or `flush_title` (which checks it to decide whether there's
            // anything to save) sees a false positive "nothing was being
            // edited" and silently drops a typed-but-unsubmitted rename.
            // `flush_title` clears the flag itself once it's actually run.
            self.set_expanded_task(None, cx);
            cx.notify();
            return;
        }
        self.schedule_picker_open = false;
        self.scheduling = false;
        self.adding_subtask = false;
        self.pending_complete_confirm = None;
        // Flushes the previous task's note and title (if either was being
        // edited) before switching `note_task_id`/`title_task_id` out from
        // under them — same ordering reasoning as the branch above.
        self.set_expanded_task(Some(id.clone()), cx);
        self.note_task_id = Some(id);
        self.note_input
            .update(cx, |input, cx| input.set_content(note.unwrap_or_default(), cx));
        cx.notify();
    }

    /// Cmd+click: toggles a row into the multi-select set instead of
    /// opening/closing its detail card.
    fn toggle_selected(&mut self, id: String, cx: &mut Context<Self>) {
        if !self.selected_task_ids.remove(&id) {
            self.selected_task_ids.insert(id);
        }
        cx.notify();
    }

    /// The detail card's schedule pill: opens/closes the picker. Opening it
    /// also focuses the free-text NLP field immediately — `scheduling` and
    /// `schedule_picker_open` now always move together, since the picker no
    /// longer has a separate button-only state before the field appears.
    fn toggle_schedule_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.schedule_picker_open = !self.schedule_picker_open;
        self.scheduling = self.schedule_picker_open;
        if self.schedule_picker_open {
            self.focus_schedule_field(window, cx);
        }
        cx.notify();
    }

    /// Removes a task's scheduled date/time while leaving its bucket
    /// unchanged, e.g. an Active-bucket task drops back into Anytime (which
    /// is exactly "Active with no `scheduled_date`" — see `list_view`), an
    /// Inbox task just loses its date. Closes the picker
    /// `docs/HANDOFF.md` gap could only change a schedule, never remove one.
    fn clear_schedule(&mut self, id: String, bucket: Bucket, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else { return };
        self.set_expanded_task(None, cx);
        self.schedule_picker_open = false;
        cx.spawn(async move |flow, cx| {
            let write_id = id.clone();
            let result = cx
                .background_executor()
                .spawn(async move { db.schedule(write_id, bucket, None::<String>, None::<String>) })
                .await;
            if let Err(error) = result {
                crate::debug_log!("task {id}: clear_schedule FAILED: {error:#}");
                return;
            }
            let _ = flow.update(cx, |flow, cx| {
                flow.invalidate_all_views();
                cx.notify();
            });
        })
        .detach();
    }

    /// Moves a task to `target` via `Db::schedule` and refreshes every task
    /// view — the quick-picker is now reachable from any of the five, so
    /// the exact set of affected views is no longer just Inbox + one
    /// destination.
    fn process_task(&mut self, id: String, target: ProcessTarget, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else { return };
        self.set_expanded_task(None, cx);
        self.schedule_picker_open = false;
        let today = (target == ProcessTarget::Today)
            .then(|| chrono::Local::now().date_naive().to_string());

        cx.spawn(async move |flow, cx| {
            let write_id = id.clone();
            let result = cx
                .background_executor()
                .spawn(async move { db.schedule(write_id, target.bucket(), today, None::<String>) })
                .await;
            if let Err(error) = result {
                crate::debug_log!("task {id}: process_task({target:?}) FAILED: {error:#}");
                return;
            }
            let _ = flow.update(cx, |flow, cx| {
                flow.invalidate_all_views();
                cx.notify();
            });
        })
        .detach();
    }

    /// `docs/DESIGN_DIRECTION.md`'s task detail spec names "delete" as an
    /// exposed action. Soft-deletes via `Db::delete_task`, which every
    /// `list_view` query already filters out.
    fn delete_task(&mut self, id: String, title: String, origin_view: View, cx: &mut Context<Self>) {
        crate::debug_log!("task {id} ({title:?}): delete requested");
        let Some(db) = self.db.clone() else { return };
        self.set_expanded_task(None, cx);
        self.schedule_picker_open = false;
        cx.spawn(async move |flow, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { db.delete_task(id.clone()).map(|()| id) })
                .await;
            let Ok(id) = result else {
                crate::debug_log!("task delete FAILED (origin {origin_view:?})");
                return;
            };
            let _ = flow.update(cx, |flow, cx| {
                flow.invalidate_all_views();
                flow.show_undo_toast(id, title, origin_view, UndoKind::Delete, cx);
            });
        })
        .detach();
    }

    /// Applies `target` to every task in `selected_task_ids`, then clears
    /// the selection. The bulk-action bar's Today/Anytime/Someday buttons.
    fn bulk_process(&mut self, target: ProcessTarget, cx: &mut Context<Self>) {
        let ids: Vec<String> = self.selected_task_ids.drain().collect();
        if ids.is_empty() {
            return;
        }
        let Some(db) = self.db.clone() else { return };
        let today = (target == ProcessTarget::Today)
            .then(|| chrono::Local::now().date_naive().to_string());

        cx.spawn(async move |flow, cx| {
            let _ = cx
                .background_executor()
                .spawn(async move {
                    for id in ids {
                        if let Err(error) =
                            db.schedule(id.clone(), target.bucket(), today.clone(), None::<String>)
                        {
                            // No error-surface UI exists yet (see
                            // docs/HANDOFF.md) — logging beats silently
                            // dropping a failure mid-batch, which the
                            // previous `let _ =` did.
                            eprintln!("Flow: bulk_process failed for task {id}: {error:#}");
                            crate::debug_log!("task {id}: bulk_process FAILED: {error:#}");
                        }
                    }
                })
                .await;
            let _ = flow.update(cx, |flow, cx| {
                flow.invalidate_all_views();
                cx.notify();
            });
        })
        .detach();
    }

    /// The bulk-action bar's Delete button.
    fn bulk_delete(&mut self, cx: &mut Context<Self>) {
        let ids: Vec<String> = self.selected_task_ids.drain().collect();
        if ids.is_empty() {
            return;
        }
        let Some(db) = self.db.clone() else { return };

        cx.spawn(async move |flow, cx| {
            let _ = cx
                .background_executor()
                .spawn(async move {
                    for id in ids {
                        if let Err(error) = db.delete_task(id.clone()) {
                            eprintln!("Flow: bulk_delete failed for task {id}: {error:#}");
                            crate::debug_log!("task {id}: bulk_delete FAILED: {error:#}");
                        }
                    }
                })
                .await;
            let _ = flow.update(cx, |flow, cx| {
                flow.invalidate_all_views();
                cx.notify();
            });
        })
        .detach();
    }

    /// The "Completed" section's "Clear" button: soft-deletes every
    /// completed task currently shown there, same loop-in-background
    /// pattern as `bulk_delete`. Reads straight off the already-loaded
    /// `completed_tasks` cache rather than re-fetching — the button only
    /// renders once that cache is `Ready` (the section itself is gated on
    /// `has_completed` in `task_list`), so a read here is never a genuine
    /// miss.
    fn clear_completed(&mut self, view: View, cx: &mut Context<Self>) {
        let Query::Ready(completed) = self.completed_tasks.read(&view) else {
            return;
        };
        let ids: Vec<String> = completed.iter().map(|task| task.id.clone()).collect();
        if ids.is_empty() {
            return;
        }
        let Some(db) = self.db.clone() else { return };

        cx.spawn(async move |flow, cx| {
            let _ = cx
                .background_executor()
                .spawn(async move {
                    for id in ids {
                        if let Err(error) = db.delete_task(id.clone()) {
                            eprintln!("Flow: clear_completed failed for task {id}: {error:#}");
                            crate::debug_log!("task {id}: clear_completed FAILED: {error:#}");
                        }
                    }
                })
                .await;
            let _ = flow.update(cx, |flow, cx| {
                flow.invalidate_view(view);
                cx.notify();
            });
        })
        .detach();
    }

    /// The floating "Completed \"…\"" banner shown after a task finishes its
    /// collapse animation. Rendered once at the window level (`render.rs`),
    /// not per view, so it survives navigating away from the view the
    /// completion happened in — the same reason `origin_view` travels with
    /// it instead of being read back off `self.destination`.
    pub(super) fn render_undo_toast(&mut self, theme: Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        let toast = self.undo_toast.as_ref()?;
        let title = toast.title.clone();
        let verb = match toast.kind {
            UndoKind::Complete => "Completed",
            UndoKind::Delete => "Deleted",
        };

        Some(
            div()
                .id("undo-toast")
                .absolute()
                .bottom(px(20.0))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .px(px(14.0))
                        .py(px(10.0))
                        .rounded(px(8.0))
                        .bg(theme.raised)
                        .border_1()
                        .border_color(theme.border_strong)
                        .shadow(vec![
                            gpui::BoxShadow::new(
                                px(0.0),
                                px(4.0),
                                gpui::Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.35 },
                            )
                            .blur_radius(px(16.0)),
                        ])
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(theme.text)
                                .child(format!("{verb} \u{201c}{title}\u{201d}")),
                        )
                        .child(
                            div()
                                .id("undo-toast-button")
                                .track_focus(&self.undo_toast_focus)
                                .tab_index(0)
                                .px(px(8.0))
                                .py(px(3.0))
                                .rounded(px(5.0))
                                .cursor_pointer()
                                .text_size(px(12.5))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.accent)
                                .hover(|el| el.bg(theme.overlay))
                                .focus_visible(|style| style.border_1().border_color(theme.accent))
                                .on_click(cx.listener(|flow, _, _, cx| flow.undo_last_action(cx)))
                                .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, _, cx| {
                                    if event.keystroke.modifiers.modified() {
                                        return;
                                    }
                                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                        flow.undo_last_action(cx);
                                        cx.stop_propagation();
                                    }
                                }))
                                .child("Undo"),
                        ),
                )
                .with_animation(
                    // Keyed to `token` (not a fixed id) so each new toast
                    // gets its own fresh 0→1 timeline instead of reusing an
                    // earlier toast's already-elapsed one.
                    gpui::SharedString::from(format!("undo-toast-fade-{}", toast.token)),
                    Animation::new(ROW_TRANSITION).with_easing(ease_out_quint()),
                    |element, delta| element.opacity(delta),
                )
                .into_any_element(),
        )
    }

    pub(super) fn render_task_view(
        &mut self,
        destination: Destination,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let view = destination
            .view()
            .expect("render_task_view is only called for task destinations");

        if self.db.is_none() {
            return database_unavailable(theme).into_any_element();
        }

        let expanded = self.expanded_task_id.clone();
        let schedule_picker_open = self.schedule_picker_open;
        let scheduling = self.scheduling;
        let schedule_input = self.schedule_input.clone();
        let note_input = self.note_input.clone();
        let editing_title = self.editing_title;
        let title_input = self.title_input.clone();
        let selected = self.selected_task_ids.clone();
        let completed_expanded = self.completed_expanded.contains(&view);
        // Only the expanded task's card ever reads this — fetched here
        // (not per row) since only one task can be expanded at a time, the
        // same reasoning `note_input` already follows.
        let subtask_context = expanded.as_ref().map(|id| {
            let subtasks = match self.read_subtasks(id, cx) {
                Query::Ready(subtasks) => {
                    self.last_subtasks.insert(id.clone(), subtasks.clone());
                    subtasks
                }
                // Same stale-while-revalidate reasoning as `read_view`
                // above: every subtask completion invalidates this cache
                // entry, so a bare empty fallback here flickered the
                // "Subtasks (N/M)" count and indented list away on every
                // toggle. Empty only for a genuinely un-fetched parent.
                Query::Pending | Query::Missing(_) => {
                    self.last_subtasks.get(id).cloned().unwrap_or_default()
                }
            };
            // A dedicated map, not `row_focuses` — that one's pruned
            // against the flat top-level task list on every refetch
            // (`prune_row_focuses`), which never contains subtask ids
            // (they're filtered out of every top-level view query), so
            // reusing it would have every subtask handle deleted the
            // very next prune after being created. Fetched here (not in
            // `render_subtask_row`, which has no `&mut Flow` access) for
            // the same reason `subtasks` itself is: only reachable from
            // `render_task_view`'s own `&mut self` scope.
            self.subtask_focuses.retain(|id, _| subtasks.iter().any(|subtask| &subtask.id == id));
            let subtask_focuses = subtasks
                .iter()
                .map(|subtask| {
                    self.subtask_focuses
                        .entry(subtask.id.clone())
                        .or_insert_with(|| cx.focus_handle())
                        .clone()
                })
                .collect();
            SubtaskContext {
                adding: self.adding_subtask,
                input: self.subtask_input.clone(),
                pending_confirm: self.pending_complete_confirm.as_deref() == Some(id.as_str()),
                subtask_count: subtasks.len(),
                subtask_focuses,
                subtasks,
            }
        });
        // The expanded task's row is the one row in a virtualized flat
        // view whose height isn't a fixed 40px — everything that can
        // change it (which task is expanded, the schedule picker, the
        // add-subtask row, the complete-with-subtasks confirm banner, and
        // the subtask list's own length) is folded into one signature
        // compared against last render. `gpui::ListState`'s own doc is
        // explicit that a virtualized list needs telling when an item's
        // height changes at a fixed index; `remeasure_items` (not
        // `splice`) preserves scroll position, since this is a size
        // change, not a membership one.
        let expanded_signature = expanded.clone().map(|id| {
            (
                id,
                schedule_picker_open,
                scheduling,
                self.adding_subtask,
                self.pending_complete_confirm.clone(),
                subtask_context.as_ref().map_or(0, |context| context.subtask_count),
            )
        });
        if expanded_signature != self.last_expanded_signature {
            for changed_id in [
                self.last_expanded_signature.as_ref().map(|signature| signature.0.clone()),
                expanded_signature.as_ref().map(|signature| signature.0.clone()),
            ]
            .into_iter()
            .flatten()
            {
                self.remeasure_task_row(&changed_id);
            }
            self.last_expanded_signature = expanded_signature;
        }
        // Always fetched, not only once expanded — the collapsed row still
        // needs a count, and this is the same "resolve the whole collection
        // up front, render degrades on a miss" pattern `read_view` already
        // follows rather than a second, gated fetch.
        let completed = match self.read_completed(view, cx) {
            Query::Ready(tasks) => {
                self.last_completed.insert(view, tasks.clone());
                tasks
            }
            // Stale-while-revalidate: an empty completed section briefly
            // reappearing mid-refetch is harmless (it's collapsed by
            // default and has no animation of its own to glitch), so this
            // one can stay simple rather than also carrying a last-known
            // fallback.
            Query::Pending | Query::Missing(_) => Arc::new(Vec::new()),
        };
        let tasks_query = self.read_view(view, cx);
        let tasks = match tasks_query {
            Query::Ready(tasks) => {
                // Only resplice on an actual Arc identity change (a real
                // refetch), not every render this Ready value happens to
                // be read on — a bare `Arc::ptr_eq` miss against whatever
                // was last stored is the cheapest correct check available,
                // since `query.rs` hands back a fresh Arc per fetch.
                let changed =
                    self.last_tasks.get(&view).is_none_or(|prev| !Arc::ptr_eq(prev, &tasks));
                self.last_tasks.insert(view, tasks.clone());
                // `changed` alone isn't enough to gate this: `sidebar.rs`'s
                // `inbox_count` reads this exact same query and updates
                // `last_tasks` too (for the badge), independently of
                // whether `render_task_view` has ever synced a ListState —
                // sidebar renders first, so by the time this runs the Arc
                // can already match with no ListState ever having been
                // created. Missing-entry is therefore checked too, or the
                // `list_state.expect(...)` below panics on first load.
                if view != View::Upcoming
                    && (changed || !self.task_list_states.contains_key(&view))
                {
                    self.sync_task_list_state(view, &tasks);
                }
                // The single source of truth for "is this row actually
                // gone yet" — see `write_completed`'s comment on why it
                // deliberately doesn't clear this itself. Once a fresh
                // fetch confirms an id isn't in the view anymore, drop it;
                // until then `is_completing` keeps the row visually
                // collapsed/checked, covering the whole write+refetch round
                // trip with no gap for it to flash back into view.
                self.completing_ids.retain(|id| tasks.iter().any(|task| &task.id == id));
                self.prune_row_focuses(&tasks);
                Some(tasks)
            }
            // Every mutation (complete, delete, schedule, ...) invalidates
            // the cache outright and refetches, so `Pending`/`Missing` here
            // usually means "the list you're already looking at, one
            // round-trip away from confirming itself" rather than "unknown
            // data." Drawing the stale list instead of a loading skeleton
            // is what stops every tick/delete/schedule from blanking the
            // whole view for a frame — the skeleton is reserved for a
            // view's genuine first load, when there's nothing to fall
            // back to yet.
            Query::Pending | Query::Missing(_) => self.last_tasks.get(&view).cloned(),
        };
        // Captured after the prune above, not before it — otherwise this
        // render would still carry an id the fresh fetch just confirmed
        // gone, for one extra frame.
        let completing_ids = self.completing_ids.clone();
        // Upcoming isn't virtualized (see `task_list_states`'s field doc),
        // so it has neither a list state nor a scrollbar to hand down.
        let list_state = (view != View::Upcoming)
            .then(|| self.task_list_states.get(&view).cloned())
            .flatten();
        let scrollbar_state = (view != View::Upcoming).then(|| {
            self.task_list_scrollbars.entry(view).or_default().clone()
        });
        let completed_clear_focus = self.completed_clear_focus(view, cx);
        let detail_delete_focus = self.detail_delete_focus.clone();
        let title_focus = self.title_focus.clone();
        let schedule_pill_focus = self.schedule_pill_focus.clone();
        let process_pill_focuses = self.process_pill_focuses.clone();
        let process_clear_focus = self.process_clear_focus.clone();
        let confirm_cancel_focus = self.confirm_cancel_focus.clone();
        let confirm_yes_focus = self.confirm_yes_focus.clone();
        let add_subtask_focus = self.add_subtask_focus.clone();
        match tasks {
            Some(tasks) => task_list(
                view,
                tasks,
                completed,
                completed_expanded,
                completing_ids,
                expanded,
                schedule_picker_open,
                scheduling,
                note_input,
                editing_title,
                title_input,
                title_focus,
                schedule_input,
                subtask_context,
                selected,
                list_state,
                scrollbar_state,
                completed_clear_focus,
                detail_delete_focus,
                schedule_pill_focus,
                process_pill_focuses,
                process_clear_focus,
                confirm_cancel_focus,
                confirm_yes_focus,
                add_subtask_focus,
                theme,
                cx,
            )
            .into_any_element(),
            None => loading_skeleton(theme).into_any_element(),
        }
    }
}

/// Everything only the expanded task's detail card needs for its subtasks
/// section — bundled into one struct rather than four separate parameters
/// threaded through every row-rendering function in this file, when only
/// one row (the expanded one) ever actually uses any of it.
#[derive(Clone)]
struct SubtaskContext {
    subtasks: Arc<Vec<Task>>,
    adding: bool,
    input: Entity<ComposerInput>,
    pending_confirm: bool,
    /// `subtasks.len()`, kept alongside it so `render_task_view` can fold
    /// it into the expanded row's remeasure signature without cloning the
    /// whole `Arc<Vec<Task>>` just to read a length.
    subtask_count: usize,
    /// One focus handle per entry in `subtasks`, same order — see
    /// `Flow::subtask_focuses`'s field doc.
    subtask_focuses: Vec<FocusHandle>,
}

#[allow(clippy::too_many_arguments)]
fn task_list(
    view: View,
    tasks: Arc<Vec<Task>>,
    completed: Arc<Vec<Task>>,
    completed_expanded: bool,
    completing_ids: HashSet<String>,
    expanded: Option<String>,
    schedule_picker_open: bool,
    scheduling: bool,
    note_input: Entity<ComposerInput>,
    editing_title: bool,
    title_input: Entity<ComposerInput>,
    title_focus: FocusHandle,
    schedule_input: Entity<ComposerInput>,
    subtask_context: Option<SubtaskContext>,
    selected: HashSet<String>,
    list_state: Option<ListState>,
    scrollbar_state: Option<Rc<crate::ui::scrollbar::ScrollbarState>>,
    completed_clear_focus: FocusHandle,
    detail_delete_focus: FocusHandle,
    schedule_pill_focus: FocusHandle,
    process_pill_focuses: [FocusHandle; 3],
    process_clear_focus: FocusHandle,
    confirm_cancel_focus: FocusHandle,
    confirm_yes_focus: FocusHandle,
    add_subtask_focus: FocusHandle,
    theme: Theme,
    cx: &mut Context<Flow>,
) -> AnyElement {
    if tasks.is_empty() && completed.is_empty() {
        return empty_state(view, theme).into_any_element();
    }

    let selected_count = selected.len();
    let has_completed = !completed.is_empty();

    div()
        .id("task-list")
        .size_full()
        .flex()
        .flex_col()
        .when(selected_count >= 2, |list| {
            list.child(
                div()
                    .flex_none()
                    .px(px(24.0))
                    .pt(px(40.0))
                    .child(bulk_action_bar(selected_count, theme, cx)),
            )
        })
        .child(
            // PRD §6.3: "Upcoming groups active tasks by local date from
            // tomorrow onward" — every other view stays the flat list it's
            // always been, now virtualized via GPUI's own `list()`
            // (`Flow::task_list_states`'s field doc has the full reasoning
            // and its one known limitation). Upcoming's date-grouped
            // sections don't fit `list()`'s flat item-index model without
            // materially more work, so it keeps the plain scrollable
            // `.children()` render it's always had.
            if view == View::Upcoming {
                div()
                    .id("task-list-open")
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .px(px(24.0))
                    // Same reasoning as the flat-list branch's own
                    // conditional pt: avoid doubling the top inset above
                    // the pinned bulk bar when it's shown.
                    .when(selected_count < 2, |list| list.pt(px(40.0)))
                    .pb(px(20.0))
                    .gap(px(1.0))
                    .children(group_by_scheduled_date(&tasks).into_iter().map(
                        |(date, group)| {
                            render_upcoming_section(
                                date,
                                group,
                                view,
                                &expanded,
                                &selected,
                                &completing_ids,
                                schedule_picker_open,
                                scheduling,
                                note_input.clone(),
                                editing_title,
                                title_input.clone(),
                                title_focus.clone(),
                                schedule_input.clone(),
                                subtask_context.clone(),
                                detail_delete_focus.clone(),
                                schedule_pill_focus.clone(),
                                process_pill_focuses.clone(),
                                process_clear_focus.clone(),
                                confirm_cancel_focus.clone(),
                                confirm_yes_focus.clone(),
                                add_subtask_focus.clone(),
                                theme,
                                cx,
                            )
                        },
                    ))
                    .into_any_element()
            } else {
                let list_state = list_state
                    .expect("a non-Upcoming view always has a synced ListState by the time task_list is reached — render_task_view only skips syncing it for Upcoming");
                let entity = cx.entity();
                // Cloned rather than moved: `note_input`/`schedule_input`/
                // `subtask_context`/`expanded` are all still needed below
                // for `completed_section`, which this closure — built once
                // but called on every visible row — must not consume.
                let list_note_input = note_input.clone();
                let list_title_input = title_input.clone();
                let list_title_focus = title_focus.clone();
                let list_schedule_input = schedule_input.clone();
                let list_subtask_context = subtask_context.clone();
                let list_expanded = expanded.clone();
                let list_detail_delete_focus = detail_delete_focus.clone();
                let list_schedule_pill_focus = schedule_pill_focus.clone();
                let list_process_pill_focuses = process_pill_focuses.clone();
                let list_process_clear_focus = process_clear_focus.clone();
                let list_confirm_cancel_focus = confirm_cancel_focus.clone();
                let list_confirm_yes_focus = confirm_yes_focus.clone();
                let list_add_subtask_focus = add_subtask_focus.clone();
                div()
                    .id("task-list-open")
                    .relative()
                    .flex_1()
                    .min_h(px(0.0))
                    .child(
                        // `gpui::list()`'s own padding is a no-op for
                        // horizontal inset: it positions each item at
                        // `bounds.origin.x + 0` regardless of `.px()`
                        // (confirmed by reading its `prepaint_items`
                        // source — only vertical padding actually offsets
                        // item position there). A plain padded wrapper
                        // around `list()` gets normal box-model layout
                        // instead: the wrapper's padding genuinely shrinks
                        // the space `list()` fills, so its bounds — and
                        // every row's `.w_full()` inside it — are already
                        // correctly inset by the time GPUI lays them out.
                        div()
                            .w_full()
                            .h_full()
                            .px(px(24.0))
                            // The 40px top inset belongs to whichever
                            // element is visually first in this region —
                            // the pinned bulk bar (which carries its own
                            // pt(40) above) when 2+ are selected, the list
                            // itself otherwise. Applying both
                            // unconditionally would double the gap above
                            // the first row whenever the bulk bar shows.
                            .when(selected_count < 2, |wrapper| wrapper.pt(px(40.0)))
                            .pb(px(20.0))
                            // PRD §6.3: "A compact calendar-glance card
                            // precedes the tasks" in Today specifically —
                            // fixture-backed per Milestone 1's exit scope
                            // (§12), see `components::calendar_glance`'s
                            // own doc for the full reasoning.
                            .when(view == View::Today, |wrapper| {
                                wrapper.child(super::components::calendar_glance(theme))
                            })
                            .child(
                                list(list_state.clone(), move |ix, _window, cx| {
                                    let task = tasks[ix].clone();
                                    let is_expanded =
                                        list_expanded.as_deref() == Some(task.id.as_str());
                                    let is_selected = selected.contains(&task.id);
                                    let is_completing = completing_ids.contains(&task.id);
                                    entity.update(cx, |flow, cx| {
                                        // The compact row's own focus
                                        // handle is only needed (and only
                                        // created — see `row_focus`'s
                                        // pruning reasoning) when this row
                                        // isn't the expanded one; the
                                        // expanded card instead reuses the
                                        // single stable `detail_delete_focus`
                                        // for its delete button, threaded
                                        // in below regardless of expansion
                                        // state since it's cheap (one
                                        // clone, not a map entry) and only
                                        // ever bound by `render_detail_card`
                                        // when this row actually is the
                                        // expanded one.
                                        let focus = (!is_expanded)
                                            .then(|| flow.row_focus(&task.id, cx));
                                        render_task_row(
                                            task,
                                            view,
                                            is_expanded,
                                            is_selected,
                                            is_completing,
                                            schedule_picker_open,
                                            scheduling,
                                            list_note_input.clone(),
                                            editing_title,
                                            list_title_input.clone(),
                                            list_title_focus.clone(),
                                            list_schedule_input.clone(),
                                            list_subtask_context.clone(),
                                            focus,
                                            list_detail_delete_focus.clone(),
                                            list_schedule_pill_focus.clone(),
                                            list_process_pill_focuses.clone(),
                                            list_process_clear_focus.clone(),
                                            list_confirm_cancel_focus.clone(),
                                            list_confirm_yes_focus.clone(),
                                            list_add_subtask_focus.clone(),
                                            theme,
                                            cx,
                                        )
                                    })
                                })
                                .w_full()
                                .h_full(),
                            ),
                    )
                    // Scrollbar is a sibling of the padded wrapper, not a
                    // child of it — it hugs `#task-list-open`'s true right
                    // edge (per `ui::scrollbar::vertical`'s own doc: pinned
                    // to the *parent's* edge), sitting in the gutter past
                    // the content's own inset rather than inside it.
                    .when_some(scrollbar_state, |el, scrollbar_state| {
                        el.child(crate::ui::scrollbar::vertical(&list_state, &scrollbar_state))
                    })
                    .into_any_element()
            },
        )
        .when(has_completed, |list| {
            list.child(
                div()
                    .id("task-list-completed-dock")
                    .flex_none()
                    .px(px(24.0))
                    .pb(px(20.0))
                    .child(completed_section(
                        view,
                        completed,
                        completed_expanded,
                        expanded,
                        schedule_picker_open,
                        scheduling,
                        note_input,
                        editing_title,
                        title_input,
                        title_focus,
                        schedule_input,
                        subtask_context,
                        completed_clear_focus,
                        detail_delete_focus,
                        schedule_pill_focus,
                        process_pill_focuses,
                        process_clear_focus,
                        confirm_cancel_focus,
                        confirm_yes_focus,
                        add_subtask_focus,
                        theme,
                        cx,
                    )),
            )
        })
        .into_any_element()
}

/// Groups Upcoming's tasks by `scheduled_date`, preserving arrival order —
/// `list_view`'s own SQL already sorts by `scheduled_date ASC, scheduled_time
/// ASC` (see `db.rs`), so this only needs to watch for the date changing,
/// never re-sort. Every task here is guaranteed a `scheduled_date` by the
/// view's own definition (`bucket = active AND scheduled_date > today`).
fn group_by_scheduled_date(tasks: &[Task]) -> Vec<(String, Vec<Task>)> {
    let mut groups: Vec<(String, Vec<Task>)> = Vec::new();
    for task in tasks {
        let date = task.scheduled_date.clone().unwrap_or_default();
        match groups.last_mut() {
            Some((last_date, group)) if *last_date == date => group.push(task.clone()),
            _ => groups.push((date, vec![task.clone()])),
        }
    }
    groups
}

/// One of Upcoming's date sections (PRD §6.3: "groups active tasks by local
/// date from tomorrow onward"). No calendar events yet — Flow's read-only
/// Google Calendar glance is a later milestone — so this only ever shows
/// task-bearing days, not the PRD's "empty days with events still show"
/// case, which has nothing to populate it with yet.
#[allow(clippy::too_many_arguments)]
fn render_upcoming_section(
    date: String,
    tasks: Vec<Task>,
    view: View,
    expanded: &Option<String>,
    selected: &HashSet<String>,
    completing_ids: &HashSet<String>,
    schedule_picker_open: bool,
    scheduling: bool,
    note_input: Entity<ComposerInput>,
    editing_title: bool,
    title_input: Entity<ComposerInput>,
    title_focus: FocusHandle,
    schedule_input: Entity<ComposerInput>,
    subtask_context: Option<SubtaskContext>,
    detail_delete_focus: FocusHandle,
    schedule_pill_focus: FocusHandle,
    process_pill_focuses: [FocusHandle; 3],
    process_clear_focus: FocusHandle,
    confirm_cancel_focus: FocusHandle,
    confirm_yes_focus: FocusHandle,
    add_subtask_focus: FocusHandle,
    theme: Theme,
    cx: &mut Context<Flow>,
) -> AnyElement {
    let today = chrono::Local::now().date_naive();
    let label = day_label(&date, today);

    div()
        .flex()
        .flex_col()
        .child(
            div()
                .id(SharedString::from(format!("upcoming-section-{date}")))
                .h(px(28.0))
                .flex()
                .items_end()
                .pb(px(4.0))
                .mt(px(4.0))
                .border_b_1()
                .border_color(theme.sidebar_border)
                .text_size(px(12.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .child(label),
        )
        .children(tasks.into_iter().map(|task| {
            let is_expanded = expanded.as_deref() == Some(task.id.as_str());
            let is_selected = selected.contains(&task.id);
            let is_completing = completing_ids.contains(&task.id);
            render_task_row(
                task,
                view,
                is_expanded,
                is_selected,
                is_completing,
                schedule_picker_open,
                scheduling,
                note_input.clone(),
                editing_title,
                title_input.clone(),
                title_focus.clone(),
                schedule_input.clone(),
                subtask_context.clone(),
                // Upcoming's rows aren't in this first keyboard-access
                // pass's scope — see `render_task_row`'s `focus` param doc.
                None,
                detail_delete_focus.clone(),
                schedule_pill_focus.clone(),
                process_pill_focuses.clone(),
                process_clear_focus.clone(),
                confirm_cancel_focus.clone(),
                confirm_yes_focus.clone(),
                add_subtask_focus.clone(),
                theme,
                cx,
            )
        }))
        .into_any_element()
}

/// The collapsed-by-default "Completed" section at the bottom of a task
/// list — its own row per view rather than one shared logbook, since a
/// completed task stays associated with wherever it was completed from.
#[allow(clippy::too_many_arguments)]
fn completed_section(
    view: View,
    completed: Arc<Vec<Task>>,
    expanded: bool,
    task_expanded: Option<String>,
    schedule_picker_open: bool,
    scheduling: bool,
    note_input: Entity<ComposerInput>,
    editing_title: bool,
    title_input: Entity<ComposerInput>,
    title_focus: FocusHandle,
    schedule_input: Entity<ComposerInput>,
    subtask_context: Option<SubtaskContext>,
    completed_clear_focus: FocusHandle,
    detail_delete_focus: FocusHandle,
    schedule_pill_focus: FocusHandle,
    process_pill_focuses: [FocusHandle; 3],
    process_clear_focus: FocusHandle,
    confirm_cancel_focus: FocusHandle,
    confirm_yes_focus: FocusHandle,
    add_subtask_focus: FocusHandle,
    theme: Theme,
    cx: &mut Context<Flow>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .mt(px(4.0))
        // The scrollable rows come before the toggle in document order so
        // the toggle stays pinned at the bottom of the dock and the section
        // reads as growing upward out of it, capped at `COMPLETED_MAX_HEIGHT`
        // before it scrolls internally instead of pushing the open list
        // further up.
        .when(expanded, |section| {
            section.child(
                div()
                    .id(SharedString::from(format!("completed-rows-{view:?}")))
                    .max_h(px(COMPLETED_MAX_HEIGHT))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .children(completed.iter().cloned().map(|task| {
                        let is_expanded = task_expanded.as_deref() == Some(task.id.as_str());
                        render_task_row(
                            task,
                            view,
                            is_expanded,
                            false,
                            false,
                            schedule_picker_open,
                            scheduling,
                            note_input.clone(),
                            editing_title,
                            title_input.clone(),
                            title_focus.clone(),
                            schedule_input.clone(),
                            subtask_context.clone(),
                            // Completed-section rows aren't in this first
                            // keyboard-access pass's scope — see
                            // `render_task_row`'s `focus` param doc.
                            None,
                            detail_delete_focus.clone(),
                            schedule_pill_focus.clone(),
                            process_pill_focuses.clone(),
                            process_clear_focus.clone(),
                            confirm_cancel_focus.clone(),
                            confirm_yes_focus.clone(),
                            add_subtask_focus.clone(),
                            theme,
                            cx,
                        )
                    }))
                    .with_animation(
                        // Unmounts entirely on collapse (this whole child is
                        // behind `.when(expanded, ...)`), so every mount is a
                        // fresh reveal — the disclosure opening rather than
                        // rows silently appearing.
                        SharedString::from(format!("completed-reveal-{view:?}")),
                        Animation::new(motion::TRANSITION).with_easing(ease_out_quint()),
                        |element, delta| element.opacity(delta),
                    ),
            )
        })
        .child(
            div()
                .h(px(28.0))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .id(SharedString::from(format!("completed-toggle-{view:?}")))
                        .h_full()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .px(px(8.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .hover(|el| el.bg(theme.overlay))
                        .on_click(
                            cx.listener(move |flow, _, _, cx| flow.toggle_completed_expanded(view, cx)),
                        )
                        .child(crate::ui::icon(
                            if expanded { "icons/chevron-down.svg" } else { "icons/chevron-right.svg" },
                            12.0,
                            theme.text_tertiary,
                        ))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.text_secondary)
                                .child(format!("Completed ({})", completed.len())),
                        ),
                )
                .when(expanded, |row| {
                    row.child(
                        div()
                            .id(SharedString::from(format!("completed-clear-{view:?}")))
                            .track_focus(&completed_clear_focus)
                            .tab_index(0)
                            .px(px(8.0))
                            .py(px(3.0))
                            .rounded(px(5.0))
                            .cursor_pointer()
                            .text_size(px(11.5))
                            .text_color(theme.text_tertiary)
                            .hover(|el| el.bg(theme.overlay).text_color(theme.danger))
                            .focus_visible(|style| style.border_1().border_color(theme.accent))
                            .on_click(cx.listener(move |flow, _, _, cx| {
                                flow.clear_completed(view, cx);
                                cx.stop_propagation();
                            }))
                            .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, _, cx| {
                                if event.keystroke.modifiers.modified() {
                                    return;
                                }
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    flow.clear_completed(view, cx);
                                    cx.stop_propagation();
                                }
                            }))
                            .child("Clear"),
                    )
                }),
        )
        .into_any_element()
}

/// A minimal bar shown above the list once 2+ tasks are selected, reusing
/// the same Today/Anytime/Someday/Delete actions the detail card exposes.
fn bulk_action_bar(count: usize, theme: Theme, cx: &mut Context<Flow>) -> AnyElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .px(px(12.0))
        .py(px(8.0))
        .mb(px(8.0))
        .rounded(px(10.0))
        .bg(theme.raised)
        .child(
            div()
                .text_size(px(12.0))
                .text_color(theme.text_secondary)
                .child(format!("{count} selected")),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .children(ProcessTarget::ALL.into_iter().map(|target| {
                    div()
                        .id(SharedString::from(format!("bulk-{}", target.label())))
                        .px(px(8.0))
                        .py(px(3.0))
                        .rounded(px(5.0))
                        .cursor_pointer()
                        .text_size(px(11.5))
                        .text_color(theme.text_secondary)
                        .bg(theme.overlay)
                        .hover(|el| el.bg(theme.overlay_strong).text_color(theme.text))
                        .on_click(cx.listener(move |flow, _, _, cx| {
                            flow.bulk_process(target, cx);
                        }))
                        .child(target.label())
                }))
                .child(
                    div()
                        .id("bulk-delete")
                        .px(px(8.0))
                        .py(px(3.0))
                        .rounded(px(5.0))
                        .cursor_pointer()
                        .text_size(px(11.5))
                        .text_color(theme.danger)
                        .bg(theme.overlay)
                        .hover(|el| el.bg(theme.overlay_strong))
                        .on_click(cx.listener(move |flow, _, _, cx| flow.bulk_delete(cx)))
                        .child("Delete"),
                ),
        )
        .with_animation(
            // Mounted via `.when(selected_count >= 2, ...)` — unmounts once
            // the selection drops back below 2, so a stable id is enough;
            // every appearance is a fresh mount.
            "bulk-action-bar-reveal",
            Animation::new(motion::TRANSITION).with_easing(ease_out_quint()),
            |element, delta| element.opacity(delta),
        )
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn render_task_row(
    task: Task,
    origin_view: View,
    is_expanded: bool,
    is_selected: bool,
    is_completing: bool,
    schedule_picker_open: bool,
    scheduling: bool,
    note_input: Entity<ComposerInput>,
    editing_title: bool,
    title_input: Entity<ComposerInput>,
    title_focus: FocusHandle,
    schedule_input: Entity<ComposerInput>,
    subtask_context: Option<SubtaskContext>,
    // `Some` only from the virtualized flat-list path (Inbox/Today/
    // Anytime/Someday's compact rows), which is the one call site that
    // already runs inside `entity.update` and so can cheaply fetch a
    // handle per visible row without the O(n) cost of doing it for every
    // task up front. `None` from the Completed section and Upcoming's
    // rows (still mouse-only) — see `Flow::row_focuses`'s field doc and
    // `docs/HANDOFF.md` for the full scope of what this first pass covers
    // and what's deliberately left for later.
    focus: Option<FocusHandle>,
    // Bound to the expanded card's delete button, schedule pill, and
    // (once the picker is open) its quick-pick pills, when this row
    // happens to be the expanded one — see `Flow::detail_delete_focus`'s
    // field doc for why one stable handle set covers every task instead
    // of a map.
    detail_delete_focus: FocusHandle,
    schedule_pill_focus: FocusHandle,
    process_pill_focuses: [FocusHandle; 3],
    process_clear_focus: FocusHandle,
    confirm_cancel_focus: FocusHandle,
    confirm_yes_focus: FocusHandle,
    add_subtask_focus: FocusHandle,
    theme: Theme,
    cx: &mut Context<Flow>,
) -> AnyElement {
    let id_for_row_click = task.id.clone();
    let note_for_click = task.note.clone();
    let id_for_row_key = task.id.clone();
    let note_for_row_key = task.note.clone();

    // A row is either its compact one-line form or its expanded detail
    // card — never both. Stacking a plain row on top of a card that
    // repeats the same checkbox and title (the previous shape) duplicated
    // the task's name on screen; the card now *is* the row when expanded.
    if is_expanded {
        return render_detail_card(
            &task,
            origin_view,
            schedule_picker_open,
            scheduling,
            note_input,
            editing_title,
            title_input,
            title_focus,
            schedule_input,
            subtask_context,
            detail_delete_focus,
            schedule_pill_focus,
            process_pill_focuses,
            process_clear_focus,
            confirm_cancel_focus,
            confirm_yes_focus,
            add_subtask_focus,
            theme,
            cx,
        );
    }

    let completed = task.completed_at.is_some();
    // Checked visually the instant it's clicked, even though the actual
    // `Db::set_completed` write (and the row's removal from this list)
    // waits for the collapse animation below — the optimistic-check half of
    // PRD §7's "checks the control immediately, fades and collapses...".
    let checked = completed || is_completing;
    let id_for_click = task.id.clone();
    let title_for_click = task.title.clone();
    let title_for_row_key = task.title.clone();
    // The schedule metadata is a fact about the task, not the view it's
    // being read from — a scheduled Inbox task (PRD §14) shows its date the
    // same as a scheduled Today/Upcoming one.
    let schedule = schedule_label(&task);

    div()
        .id(gpui::SharedString::from(format!("task-{}", task.id)))
        // `gpui::list()` lays each row out as the root of its own layout
        // tree (`layout_as_root`) rather than as a flex child of a
        // stretch-by-default container the way the old plain `.children()`
        // list did — so unlike every other row-shaped element in this
        // file, this one has to claim its own width explicitly or it
        // shrinks to its content's natural width instead of filling the
        // list's viewport (a real regression this virtualization
        // introduced, caught from a screenshot: rows collapsed to a narrow
        // column).
        .w_full()
        .h(px(40.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(8.0))
        .rounded(px(6.0))
        .overflow_hidden()
        .cursor_pointer()
        .hover(|el| el.bg(theme.overlay))
        .when(is_selected, |row| row.bg(theme.sidebar_item_background))
        // First entry in the keyboard-accessibility pass this codebase's
        // own audit flagged (`docs/HANDOFF.md`) — tab reaches the row.
        // Enter opens it, matching a plain click on the row body. Space
        // toggles completion instead of also opening the row: this app
        // already treats bare Space as "act on the task" at the app level
        // (`SpaceCapture` opens Capture when nothing's focused), and giving
        // the checkbox its own tab stop here would double the tab stops
        // per row across what can be a long list — a real cost, not just
        // a style choice, given this file's own performance discipline
        // around per-row work. Scoped to just these two activations for
        // now; cmd+select and arrow-key navigation between rows are
        // deliberately not attempted here — see the `focus` parameter's
        // doc for the exact boundary.
        .when_some(focus, |row, handle| {
            row.track_focus(&handle)
                .tab_index(0)
                .focus_visible(|style| style.border_1().border_color(theme.accent))
                .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, _, cx| {
                    if event.keystroke.modifiers.modified() {
                        return;
                    }
                    match event.keystroke.key.as_str() {
                        "enter" => {
                            flow.selected_task_ids.clear();
                            flow.toggle_expanded(
                                id_for_row_key.clone(),
                                note_for_row_key.clone(),
                                cx,
                            );
                            cx.stop_propagation();
                        }
                        "space" => {
                            // Same `is_completing` re-click guard the
                            // checkbox's own `on_click` uses — a held or
                            // repeated Space must not restart the
                            // collapse animation mid-flight.
                            if !is_completing {
                                flow.toggle_completed(
                                    id_for_row_key.clone(),
                                    title_for_row_key.clone(),
                                    !completed,
                                    origin_view,
                                    cx,
                                );
                            }
                            cx.stop_propagation();
                        }
                        _ => {}
                    }
                }))
        })
        .on_click(cx.listener(move |flow, event: &ClickEvent, _, cx| {
            if event.modifiers().secondary() {
                flow.toggle_selected(id_for_row_click.clone(), cx);
            } else {
                flow.selected_task_ids.clear();
                flow.toggle_expanded(id_for_row_click.clone(), note_for_click.clone(), cx);
            }
        }))
        .child(
            div()
                .id(gpui::SharedString::from(format!("task-{}-complete", task.id)))
                .w(px(17.0))
                .h(px(17.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .border_1()
                .border_color(if checked { theme.accent } else { theme.border_strong })
                .cursor_default()
                .hover(|el| el.border_color(theme.accent))
                .on_click(cx.listener(move |flow, _, _, cx| {
                    if is_completing {
                        return; // Already animating out from an earlier click.
                    }
                    flow.toggle_completed(id_for_click.clone(), title_for_click.clone(), !completed, origin_view, cx);
                    cx.stop_propagation();
                }))
                .when(checked, |circle| {
                    circle.child(crate::ui::icon("icons/check.svg", 11.0, theme.accent))
                }),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(13.0))
                .text_color(theme.text)
                .child(task.title.clone()),
        )
        .when_some(schedule, |row, label| {
            row.child(
                div()
                    .flex_none()
                    .text_size(px(11.5))
                    .text_color(theme.text_tertiary)
                    .child(label),
            )
        })
        .with_animation(
            // A distinct id for the completing state rather than reusing
            // the mount fade-in's — `with_animation` times an id from when
            // it first appears, so switching `is_completing` on a row that
            // already finished its fade-in needs a fresh id to get a fresh
            // 0→1 timeline instead of jumping straight to the fade-in's
            // already-elapsed delta of 1.
            gpui::SharedString::from(if is_completing {
                format!("task-collapse-{}", task.id)
            } else {
                format!("task-fade-{}", task.id)
            }),
            Animation::new(ROW_TRANSITION).with_easing(ease_out_quint()),
            move |element, delta| {
                if is_completing {
                    // Opacity has to hit 0 well before the collapse finishes,
                    // or `overflow_hidden` starts hard-clipping a checkbox
                    // and title that are still half-visible — a chopped
                    // glyph mid-shrink, not a clean collapse. Racing fade
                    // ahead of height (out by 45% of the timeline, height
                    // still running the full duration) means there's nothing
                    // left to clip by the time the row is short enough for
                    // it to show.
                    let fade = (delta / 0.45).min(1.0);
                    element.opacity(1.0 - fade).h(px(40.0 * (1.0 - delta)))
                } else {
                    element.opacity(delta)
                }
            },
        )
        .into_any_element()
}

/// `docs/DESIGN_DIRECTION.md`'s "Task detail" component: a 10 px raised
/// surface, not a modal, exposing note, subtasks, schedule, move, and
/// delete in that order. A subtask itself shows no Subtasks section of its
/// own — PRD §6.2's one-level ceiling ("a subtask cannot have children").
#[allow(clippy::too_many_arguments)]
fn render_detail_card(
    task: &Task,
    origin_view: View,
    schedule_picker_open: bool,
    scheduling: bool,
    note_input: Entity<ComposerInput>,
    editing_title: bool,
    title_input: Entity<ComposerInput>,
    title_focus: FocusHandle,
    schedule_input: Entity<ComposerInput>,
    subtask_context: Option<SubtaskContext>,
    detail_delete_focus: FocusHandle,
    schedule_pill_focus: FocusHandle,
    process_pill_focuses: [FocusHandle; 3],
    process_clear_focus: FocusHandle,
    confirm_cancel_focus: FocusHandle,
    confirm_yes_focus: FocusHandle,
    add_subtask_focus: FocusHandle,
    theme: Theme,
    cx: &mut Context<Flow>,
) -> AnyElement {
    // Always `Some` in practice — `render_task_view` only computes it when
    // a task is expanded, and this function is only reached for that exact
    // task (see `render_task_row`'s `is_expanded` branch).
    let subtask_context = subtask_context
        .expect("render_detail_card is only reached for the expanded task, which always has a SubtaskContext");
    let open_subtask_ids: Vec<String> = subtask_context
        .subtasks
        .iter()
        .filter(|subtask| subtask.completed_at.is_none())
        .map(|subtask| subtask.id.clone())
        .collect();

    let completed = task.completed_at.is_some();
    let id_for_complete = task.id.clone();
    let title_for_complete = task.title.clone();
    let has_open_subtasks_for_complete = !open_subtask_ids.is_empty();
    let id_for_delete = task.id.clone();
    let title_for_delete = task.title.clone();
    let id_for_delete_key = task.id.clone();
    let title_for_delete_key = task.title.clone();
    let id_for_collapse = task.id.clone();
    let note_for_collapse = task.note.clone();
    let id_for_edit = task.id.clone();
    let title_for_edit = task.title.clone();
    let id_for_edit_key = task.id.clone();
    let title_for_edit_key = task.title.clone();
    let placement = placement_label(task);

    div()
        // Same `gpui::list()` per-item-root reasoning as the compact row
        // above — this is the other of the two shapes `render_task_row`
        // can return, so it needs the same explicit width.
        .w_full()
        .my(px(1.0))
        .p(px(12.0))
        .rounded(px(10.0))
        .bg(theme.raised)
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .id(gpui::SharedString::from(format!("task-{}-detail-header", task.id)))
                .cursor_pointer()
                // The title's own click now starts editing it rather than
                // collapsing the card (an editable title needs its own
                // activation — see the title child's own comment), so the
                // header row itself keeps the previous "click to collapse"
                // behavior on whatever background space isn't the
                // checkbox or the title text — both of those already
                // `stop_propagation()` their own clicks, so this only
                // fires on genuine background clicks, the same area that
                // already read as "the title" before it became editable.
                .on_click(cx.listener(move |flow, _, _, cx| {
                    flow.toggle_expanded(id_for_collapse.clone(), note_for_collapse.clone(), cx);
                }))
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(
                    div()
                        .id(gpui::SharedString::from(format!("task-{}-detail-complete", task.id)))
                        .w(px(17.0))
                        .h(px(17.0))
                        .flex_none()
                        .rounded_full()
                        .border_1()
                        .border_color(theme.border_strong)
                        .cursor_default()
                        .hover(|el| el.border_color(theme.accent))
                        .on_click(cx.listener(move |flow, _, _, cx| {
                            if completed {
                                flow.toggle_completed(
                                    id_for_complete.clone(),
                                    title_for_complete.clone(),
                                    false,
                                    origin_view,
                                    cx,
                                );
                            } else {
                                flow.request_complete(
                                    id_for_complete.clone(),
                                    title_for_complete.clone(),
                                    has_open_subtasks_for_complete,
                                    origin_view,
                                    cx,
                                );
                            }
                            cx.stop_propagation();
                        })),
                )
                .child(if editing_title {
                    // PRD §11's "edit" verb, previously missing entirely
                    // (found by re-checking the acceptance criteria
                    // against the shipped app). `title_input` is
                    // pre-filled and focused by `start_editing_title`
                    // before this ever renders — see that method's doc.
                    div()
                        .id(gpui::SharedString::from(format!("task-{}-detail-title-edit", task.id)))
                        .flex_1()
                        .min_w_0()
                        .child(title_input)
                        .into_any_element()
                } else {
                    div()
                        .id(gpui::SharedString::from(format!("task-{}-detail-title", task.id)))
                        .track_focus(&title_focus)
                        .tab_index(0)
                        .flex_1()
                        .min_w_0()
                        .cursor_pointer()
                        .text_size(px(15.0))
                        .text_color(theme.text)
                        .focus_visible(|style| style.border_1().border_color(theme.accent))
                        // A dedicated click target for editing, taking
                        // over from the old "click the title to collapse
                        // the card" behavior the header row's blank space
                        // still has — an editable title needs its own
                        // activation, and "click text to edit it" is the
                        // more expected reading once it's editable at all.
                        .on_click(cx.listener(move |flow, _, window, cx| {
                            flow.start_editing_title(id_for_edit.clone(), title_for_edit.clone(), window, cx);
                            cx.stop_propagation();
                        }))
                        .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, window, cx| {
                            if event.keystroke.modifiers.modified() {
                                return;
                            }
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                flow.start_editing_title(
                                    id_for_edit_key.clone(),
                                    title_for_edit_key.clone(),
                                    window,
                                    cx,
                                );
                                cx.stop_propagation();
                            }
                        }))
                        .child(task.title.clone())
                        .into_any_element()
                }),
        )
        .child(note_input)
        .when(subtask_context.pending_confirm, |card| {
            let id_for_confirm = task.id.clone();
            let title_for_confirm = task.title.clone();
            let confirm_subtask_ids = open_subtask_ids.clone();
            let id_for_confirm_key = task.id.clone();
            let title_for_confirm_key = task.title.clone();
            let confirm_subtask_ids_key = open_subtask_ids.clone();
            card.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.0))
                    .p(px(8.0))
                    .rounded(px(6.0))
                    .bg(theme.overlay)
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(11.5))
                            .text_color(theme.text)
                            .child("Complete parent and all subtasks?"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .id("complete-confirm-cancel")
                                    .track_focus(&confirm_cancel_focus)
                                    .tab_index(0)
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .rounded(px(5.0))
                                    .cursor_pointer()
                                    .text_size(px(11.5))
                                    .text_color(theme.text_secondary)
                                    .hover(|el| el.bg(theme.overlay_strong))
                                    .focus_visible(|style| style.border_1().border_color(theme.accent))
                                    .on_click(cx.listener(move |flow, _, _, cx| {
                                        flow.cancel_complete_confirm(cx);
                                        cx.stop_propagation();
                                    }))
                                    .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, _, cx| {
                                        if event.keystroke.modifiers.modified() {
                                            return;
                                        }
                                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                            flow.cancel_complete_confirm(cx);
                                            cx.stop_propagation();
                                        }
                                    }))
                                    .child("Cancel"),
                            )
                            .child(
                                div()
                                    .id("complete-confirm-yes")
                                    .track_focus(&confirm_yes_focus)
                                    .tab_index(0)
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .rounded(px(5.0))
                                    .cursor_pointer()
                                    .text_size(px(11.5))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.accent)
                                    .hover(|el| el.bg(theme.overlay_strong))
                                    .focus_visible(|style| style.border_1().border_color(theme.accent))
                                    .on_click(cx.listener(move |flow, _, _, cx| {
                                        flow.confirm_complete_with_subtasks(
                                            id_for_confirm.clone(),
                                            title_for_confirm.clone(),
                                            confirm_subtask_ids.clone(),
                                            origin_view,
                                            cx,
                                        );
                                        cx.stop_propagation();
                                    }))
                                    .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, _, cx| {
                                        if event.keystroke.modifiers.modified() {
                                            return;
                                        }
                                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                            flow.confirm_complete_with_subtasks(
                                                id_for_confirm_key.clone(),
                                                title_for_confirm_key.clone(),
                                                confirm_subtask_ids_key.clone(),
                                                origin_view,
                                                cx,
                                            );
                                            cx.stop_propagation();
                                        }
                                    }))
                                    .child("Complete all"),
                            ),
                    )
                    .with_animation(
                        // Mounted via `.when(pending_confirm, ...)` —
                        // unmounts once cancelled or confirmed, so a
                        // per-task id is enough; every appearance is a
                        // fresh mount.
                        gpui::SharedString::from(format!("complete-confirm-{}-reveal", task.id)),
                        Animation::new(motion::TRANSITION).with_easing(ease_out_quint()),
                        |element, delta| element.opacity(delta),
                    ),
            )
        })
        .when(task.parent_id.is_none(), |card| {
            card.child(render_subtasks_section(
                task.id.clone(),
                &subtask_context,
                add_subtask_focus,
                theme,
                cx,
            ))
        })
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(render_schedule_pill(
                    task.id.clone(),
                    placement,
                    schedule_pill_focus,
                    theme,
                    cx,
                ))
                .child(
                    crate::ui::icon_button(
                        gpui::SharedString::from(format!("task-{}-delete", task.id)),
                        "icons/trash.svg",
                        theme,
                    )
                    .track_focus(&detail_delete_focus)
                    .tab_index(0)
                    .focus_visible(|style| style.border_1().border_color(theme.accent))
                    .on_click(cx.listener(move |flow, _, _, cx| {
                        flow.delete_task(id_for_delete.clone(), title_for_delete.clone(), origin_view, cx);
                        cx.stop_propagation();
                    }))
                    .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, _, cx| {
                        if event.keystroke.modifiers.modified() {
                            return;
                        }
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            flow.delete_task(
                                id_for_delete_key.clone(),
                                title_for_delete_key.clone(),
                                origin_view,
                                cx,
                            );
                            cx.stop_propagation();
                        }
                    })),
                ),
        )
        .when(schedule_picker_open, |card| {
            card.child(render_process_row(
                task.id.clone(),
                task.bucket,
                task.scheduled_date.is_some(),
                scheduling,
                schedule_input,
                process_pill_focuses,
                process_clear_focus,
                theme,
                cx,
            ))
        })
        .with_animation(
            // The row unmounts entirely when collapsed (`render_task_row`'s
            // `is_expanded` branch), so a stable id per task is enough —
            // every mount is a fresh appearance, unlike the row's own
            // fade/collapse which needs two ids to distinguish in-place
            // states on an element that never leaves the tree.
            gpui::SharedString::from(format!("task-{}-detail-reveal", task.id)),
            Animation::new(motion::REVEAL).with_easing(ease_out_quint()),
            |element, delta| element.opacity(delta),
        )
        .into_any_element()
}

/// PRD §6.2's subtasks: "shown indented beneath an expanded parent."
/// `docs/DESIGN_DIRECTION.md`: "one indentation level and a slender left
/// guide that ends at the last child" — a plain `border_l_1()` around just
/// the row list achieves that for free, since a border only ever wraps its
/// own box. The header's "(done/total)" is the progress PRD §6.2 asks for,
/// as a plain count rather than a literal ring — no chart/ring primitive
/// exists in this codebase yet to justify building one for a single spot.
fn render_subtasks_section(
    parent_id: String,
    context: &SubtaskContext,
    add_subtask_focus: FocusHandle,
    theme: Theme,
    cx: &mut Context<Flow>,
) -> AnyElement {
    let total = context.subtasks.len();
    let done = context.subtasks.iter().filter(|t| t.completed_at.is_some()).count();

    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(11.5))
                .text_color(theme.text_secondary)
                .child(if total > 0 {
                    format!("Subtasks ({done}/{total})")
                } else {
                    "Subtasks".to_string()
                }),
        )
        .when(!context.subtasks.is_empty(), |section| {
            let parent_id = parent_id.clone();
            section.child(
                div()
                    .flex()
                    .flex_col()
                    .pl(px(10.0))
                    .border_l_1()
                    .border_color(theme.border)
                    .children(
                        context.subtasks.iter().cloned().zip(context.subtask_focuses.iter().cloned()).map(
                            |(subtask, focus)| render_subtask_row(subtask, parent_id.clone(), focus, theme, cx),
                        ),
                    ),
            )
        })
        .child(if context.adding {
            div().pl(px(10.0)).child(context.input.clone()).into_any_element()
        } else {
            div()
                .id(SharedString::from(format!("add-subtask-{parent_id}")))
                .track_focus(&add_subtask_focus)
                .tab_index(0)
                .pl(px(10.0))
                .h(px(22.0))
                .flex()
                .items_center()
                .gap(px(4.0))
                .cursor_pointer()
                .text_size(px(11.5))
                .text_color(theme.text_tertiary)
                .hover(|el| el.text_color(theme.text_secondary))
                .focus_visible(|style| style.border_1().border_color(theme.accent))
                .on_click(cx.listener(move |flow, _, window, cx| {
                    flow.open_add_subtask(window, cx);
                    cx.stop_propagation();
                }))
                .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.modifiers.modified() {
                        return;
                    }
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        flow.open_add_subtask(window, cx);
                        cx.stop_propagation();
                    }
                }))
                .child(crate::ui::icon("icons/plus.svg", 11.0, theme.text_tertiary))
                .child("Add subtask")
                .into_any_element()
        })
        .into_any_element()
}

/// One subtask row: a smaller completion circle + title, no schedule
/// metadata (subtasks don't show one — PRD §6.2: "a child inherits no
/// schedule automatically" — and no click-to-expand, since a subtask has
/// no detail card of its own under the one-level ceiling).
fn render_subtask_row(
    subtask: Task,
    parent_id: String,
    focus: FocusHandle,
    theme: Theme,
    cx: &mut Context<Flow>,
) -> AnyElement {
    let checked = subtask.completed_at.is_some();
    let id = subtask.id.clone();
    let id_for_key = subtask.id.clone();
    let parent_id_for_key = parent_id.clone();

    div()
        .h(px(24.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        // Same Space-toggles-completion convention as the top-level task
        // row (`render_task_row`) — no Enter action here since a subtask
        // has no detail card of its own to open under the one-level
        // ceiling, so the row's only keyboard verb is completion.
        .track_focus(&focus)
        .tab_index(0)
        .focus_visible(|style| style.border_1().border_color(theme.accent))
        .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, _, cx| {
            if event.keystroke.modifiers.modified() {
                return;
            }
            if event.keystroke.key == "space" {
                flow.toggle_subtask_completed(id_for_key.clone(), parent_id_for_key.clone(), !checked, cx);
                cx.stop_propagation();
            }
        }))
        .child(
            div()
                .id(SharedString::from(format!("subtask-{}-complete", subtask.id)))
                .w(px(13.0))
                .h(px(13.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .border_1()
                .border_color(if checked { theme.accent } else { theme.border_strong })
                .cursor_default()
                .hover(|el| el.border_color(theme.accent))
                .on_click(cx.listener(move |flow, _, _, cx| {
                    flow.toggle_subtask_completed(id.clone(), parent_id.clone(), !checked, cx);
                    cx.stop_propagation();
                }))
                .when(checked, |circle| {
                    circle.child(crate::ui::icon("icons/check.svg", 9.0, theme.accent))
                }),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(12.5))
                .text_color(if checked { theme.text_tertiary } else { theme.text })
                .child(subtask.title.clone()),
        )
        .into_any_element()
}

/// The detail card's schedule status: the task's current placement in the
/// same four buckets the quick-picker offers. Clicking it opens that
/// picker so placement can be changed without leaving the card.
fn render_schedule_pill(
    id: String,
    label: String,
    focus: FocusHandle,
    theme: Theme,
    cx: &mut Context<Flow>,
) -> AnyElement {
    div()
        .id(gpui::SharedString::from(format!("schedule-pill-{id}")))
        .track_focus(&focus)
        .tab_index(0)
        .px(px(8.0))
        .py(px(3.0))
        .rounded(px(5.0))
        .cursor_pointer()
        .text_size(px(11.5))
        .text_color(theme.text_secondary)
        .bg(theme.overlay)
        .hover(|el| el.bg(theme.overlay_strong).text_color(theme.text))
        .focus_visible(|style| style.border_1().border_color(theme.accent))
        .on_click(cx.listener(move |flow, _, window, cx| {
            flow.toggle_schedule_picker(window, cx);
            cx.stop_propagation();
        }))
        .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, window, cx| {
            if event.keystroke.modifiers.modified() {
                return;
            }
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                flow.toggle_schedule_picker(window, cx);
                cx.stop_propagation();
            }
        }))
        .child(label)
        .into_any_element()
}

/// PRD §6.3's "inline 'Process' action". The free-text NLP field is now the
/// primary surface — opening the picker (`Flow::toggle_schedule_picker`)
/// focuses it immediately rather than gating it behind a separate
/// "Schedule…" click — with Today/Anytime/Someday as a quick-pick list
/// underneath for the common cases that don't need typing. A "Clear"
/// option appears only when the task actually has a schedule to remove.
/// `scheduling` is threaded through purely for signature consistency with
/// the rest of this file's render chain; it's always true by the time this
/// renders, since nothing opens the picker without also opening the field.
#[allow(clippy::too_many_arguments)]
fn render_process_row(
    task_id: String,
    bucket: Bucket,
    has_schedule: bool,
    scheduling: bool,
    schedule_input: Entity<ComposerInput>,
    process_pill_focuses: [FocusHandle; 3],
    process_clear_focus: FocusHandle,
    theme: Theme,
    cx: &mut Context<Flow>,
) -> AnyElement {
    let _ = scheduling;
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(schedule_input)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                // `ProcessTarget::ALL`'s fixed order (Today, Anytime,
                // Someday) matches `process_pill_focuses`'s field doc —
                // zipping keeps that correspondence explicit at the use
                // site instead of relying on array-index arithmetic.
                .children(ProcessTarget::ALL.into_iter().zip(process_pill_focuses).map(
                    |(target, focus)| {
                    let id = task_id.clone();
                    let id_for_key = task_id.clone();
                    div()
                        .id(gpui::SharedString::from(format!("process-{task_id}-{}", target.label())))
                        .track_focus(&focus)
                        .tab_index(0)
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .px(px(8.0))
                        .py(px(3.0))
                        .rounded(px(5.0))
                        .cursor_pointer()
                        .text_size(px(11.5))
                        .text_color(theme.text_secondary)
                        .bg(theme.overlay)
                        .hover(|el| el.bg(theme.overlay_strong).text_color(theme.text))
                        .focus_visible(|style| style.border_1().border_color(theme.accent))
                        .on_click(cx.listener(move |flow, _, _, cx| {
                            flow.process_task(id.clone(), target, cx);
                            cx.stop_propagation();
                        }))
                        .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, _, cx| {
                            if event.keystroke.modifiers.modified() {
                                return;
                            }
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                flow.process_task(id_for_key.clone(), target, cx);
                                cx.stop_propagation();
                            }
                        }))
                        .child(crate::ui::icon(target.icon_path(), 12.0, theme.text_secondary))
                        .child(target.label())
                }))
                .when(has_schedule, |row| {
                    let id = task_id.clone();
                    let id_for_key = task_id.clone();
                    row.child(
                        div()
                            .id(gpui::SharedString::from(format!("process-{task_id}-clear")))
                            .track_focus(&process_clear_focus)
                            .tab_index(0)
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(8.0))
                            .py(px(3.0))
                            .rounded(px(5.0))
                            .cursor_pointer()
                            .text_size(px(11.5))
                            .text_color(theme.text_secondary)
                            .bg(theme.overlay)
                            .hover(|el| el.bg(theme.overlay_strong).text_color(theme.text))
                            .focus_visible(|style| style.border_1().border_color(theme.accent))
                            .on_click(cx.listener(move |flow, _, _, cx| {
                                flow.clear_schedule(id.clone(), bucket, cx);
                                cx.stop_propagation();
                            }))
                            .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, _, cx| {
                                if event.keystroke.modifiers.modified() {
                                    return;
                                }
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    flow.clear_schedule(id_for_key.clone(), bucket, cx);
                                    cx.stop_propagation();
                                }
                            }))
                            .child(crate::ui::icon("icons/x.svg", 12.0, theme.text_secondary))
                            .child("Clear"),
                    )
                }),
        )
        .with_animation(
            // Mounted via `.when(schedule_picker_open, ...)` — unmounts
            // entirely on close, so a stable id per task is enough; every
            // mount is a fresh reveal, matching the detail card's own
            // pattern.
            gpui::SharedString::from(format!("process-{task_id}-reveal")),
            Animation::new(motion::TRANSITION).with_easing(ease_out_quint()),
            |element, delta| element.opacity(delta),
        )
        .into_any_element()
}

/// Human-friendly rendering of a stored `YYYY-MM-DD`/`HH:mm` schedule pair:
/// "Today"/"Tomorrow" for the two nearest days, a bare weekday name for the
/// rest of the week, else a short date (`"Aug 23"`) — with the time appended
/// in 12-hour form when present (`"Tomorrow 6:00 PM"`). Shared by
/// `schedule_label` (the row's trailing label) and `placement_label` (the
/// detail card's status pill) so both read the same.
/// The "Today"/"Tomorrow"/weekday/short-date half of `format_schedule`, kept
/// separate so Upcoming's date-section headers (which have no time to
/// append) can call it directly instead of duplicating the day-name logic.
fn day_label(date: &str, today: chrono::NaiveDate) -> String {
    match chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(parsed) => match (parsed - today).num_days() {
            0 => "Today".to_string(),
            1 => "Tomorrow".to_string(),
            2..=6 => parsed.format("%A").to_string(),
            _ => parsed.format("%b %-d").to_string(),
        },
        // An unparseable stored date is not expected, but showing the raw
        // string beats panicking or hiding the schedule entirely.
        Err(_) => date.to_string(),
    }
}

fn format_schedule(date: &str, time: Option<&str>, today: chrono::NaiveDate) -> String {
    let day_part = day_label(date, today);
    let Some(time) = time else {
        return day_part;
    };
    match chrono::NaiveTime::parse_from_str(time, "%H:%M") {
        Ok(parsed) => format!("{day_part} {}", parsed.format("%-I:%M %p")),
        Err(_) => day_part,
    }
}

/// `scheduled_date`/`scheduled_time` are stored as plain `YYYY-MM-DD`/
/// `HH:mm` strings (`docs/PRODUCT_REQUIREMENTS.md` §6.4); `format_schedule`
/// renders them for display.
fn schedule_label(task: &Task) -> Option<String> {
    let date = task.scheduled_date.as_deref()?;
    let today = chrono::Local::now().date_naive();
    Some(format_schedule(date, task.scheduled_time.as_deref(), today))
}

/// The detail card's schedule-pill label: the task's formatted schedule if
/// it has one, else "Anytime"/"Someday"/"Inbox" for its unscheduled bucket.
fn placement_label(task: &Task) -> String {
    if let Some(label) = schedule_label(task) {
        return label;
    }
    match task.bucket {
        Bucket::Someday => "Someday".to_string(),
        Bucket::Active => "Anytime".to_string(),
        Bucket::Inbox => "Inbox".to_string(),
    }
}

/// PRD §7's "Required UI states" table plus a placement-specific line for
/// the two views the table doesn't cover (Upcoming, Anytime).
fn empty_state(view: View, theme: Theme) -> impl IntoElement {
    let copy = match view {
        View::Inbox => "Nothing to process. Capture the next thing.",
        View::Today => "Your day is clear.",
        View::Upcoming => "Nothing scheduled ahead.",
        View::Anytime => "No active work waiting.",
        View::Someday => "Nothing deferred.",
    };
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(13.0))
                .text_color(theme.text_secondary)
                .child(copy),
        )
}

fn loading_skeleton(theme: Theme) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .px(px(24.0))
        .py(px(20.0))
        .gap(px(8.0))
        .children((0..3).map(|index| {
            div()
                .id(gpui::SharedString::from(format!("task-skeleton-{index}")))
                .h(px(40.0))
                .rounded(px(6.0))
                .bg(theme.overlay)
        }))
}

fn database_unavailable(theme: Theme) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(13.0))
                .text_color(theme.danger)
                .child("Local database unavailable."),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `group_by_scheduled_date` and `day_label` are the two pure pieces of
    /// Upcoming's PRD §6.3 date grouping — everything else in this file is
    /// GPUI element construction, which this codebase doesn't unit-test
    /// (see `db.rs`/`parse.rs` for where the real test coverage lives).
    fn task(id: &str, scheduled_date: &str) -> Task {
        Task {
            id: id.to_string(),
            parent_id: None,
            title: "Task".to_string(),
            note: None,
            bucket: Bucket::Active,
            scheduled_date: Some(scheduled_date.to_string()),
            scheduled_time: None,
            scheduled_timezone: None,
            position: 0.0,
            completed_at: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn consecutive_same_date_tasks_land_in_one_group() {
        let tasks = vec![
            task("a", "2026-08-20"),
            task("b", "2026-08-20"),
            task("c", "2026-08-21"),
        ];
        let groups = group_by_scheduled_date(&tasks);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "2026-08-20");
        assert_eq!(groups[0].1.len(), 2);
        assert_eq!(groups[1].0, "2026-08-21");
        assert_eq!(groups[1].1.len(), 1);
    }

    #[test]
    fn a_repeated_date_after_a_gap_starts_a_new_group_not_merges_back() {
        // `list_view`'s ORDER BY guarantees this never happens in practice
        // (dates only ever increase), but the grouping itself only compares
        // against the immediately preceding group, not the whole history —
        // worth locking in that it doesn't silently merge non-adjacent runs.
        let tasks = vec![
            task("a", "2026-08-20"),
            task("b", "2026-08-21"),
            task("c", "2026-08-20"),
        ];
        let groups = group_by_scheduled_date(&tasks);
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn day_label_matches_format_schedules_day_half() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 8, 18).unwrap();
        assert_eq!(day_label("2026-08-19", today), "Tomorrow");
        assert_eq!(day_label("2026-08-21", today), "Friday");
        assert_eq!(day_label("2026-09-01", today), "Sep 1");
    }
}
