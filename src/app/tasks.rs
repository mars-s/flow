//! Milestone 1: the five task views (Inbox, Today, Upcoming, Anytime,
//! Someday), all backed by `crate::db::View` and sharing one row renderer.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, ClickEvent, Context, Entity, IntoElement, ParentElement,
    SharedString, Styled, div, ease_out_quint, prelude::*, px,
};

use super::Flow;
use super::UndoToast;
use super::sidebar::Destination;
use crate::db::{Bucket, Task, View};
use crate::input::ComposerInput;
use crate::query::Query;
use crate::theme::Theme;

/// PRD §7's completion-collapse timing; reused here for the row's fade-in
/// too since nothing in the direction doc distinguishes them.
const ROW_TRANSITION: Duration = Duration::from_millis(180);

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
    fn invalidate_all_views(&mut self) {
        for view in [
            View::Inbox,
            View::Today,
            View::Upcoming,
            View::Anytime,
            View::Someday,
        ] {
            self.tasks.invalidate(&view);
        }
    }

    /// Invalidates both an open view and its collapsed "Completed" section —
    /// every write that moves a task in or out of one can also move it in or
    /// out of the other, so the two caches always travel together.
    fn invalidate_view(&mut self, view: View) {
        self.tasks.invalidate(&view);
        self.completed_tasks.invalidate(&view);
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
        let Some(db) = self.db.clone() else { return };
        cx.spawn(async move |flow, cx| {
            let Ok(()) = cx
                .background_executor()
                .spawn(async move { db.set_completed(id, completed) })
                .await
            else {
                return;
            };
            let _ = flow.update(cx, |flow, cx| {
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
                flow.completing_ids.remove(&id);
                flow.write_completed(id.clone(), true, origin_view, cx);
                flow.show_undo_toast(id, title, origin_view, cx);
            });
        })
        .detach();
    }

    /// Shows (or replaces) the single-slot "Completed" toast and schedules
    /// its own dismissal. `token` distinguishes this toast's timer from an
    /// earlier one still in flight — without it, completing a second task
    /// inside the first one's 10-second window would let the first timer
    /// clear the second toast early.
    fn show_undo_toast(&mut self, task_id: String, title: String, origin_view: View, cx: &mut Context<Self>) {
        self.undo_token += 1;
        let token = self.undo_token;
        self.undo_toast = Some(UndoToast { task_id, title: title.into(), origin_view, token });
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

    /// The toast's "Undo" button: reopens the task and dismisses the toast.
    fn undo_complete(&mut self, cx: &mut Context<Self>) {
        let Some(toast) = self.undo_toast.take() else { return };
        cx.notify();
        self.write_completed(toast.task_id, false, toast.origin_view, cx);
    }

    /// Opens or closes a task's detail card — `docs/DESIGN_DIRECTION.md`'s
    /// "Task detail" component, available from every task view now, not
    /// only Inbox. Loads the task's current note into `note_input`.
    fn toggle_expanded(&mut self, id: String, note: Option<String>, cx: &mut Context<Self>) {
        if self.expanded_task_id.as_deref() == Some(id.as_str()) {
            self.expanded_task_id = None;
            self.schedule_picker_open = false;
            self.scheduling = false;
            cx.notify();
            return;
        }
        self.expanded_task_id = Some(id.clone());
        self.note_task_id = Some(id);
        self.schedule_picker_open = false;
        self.scheduling = false;
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

    /// The detail card's schedule pill: opens/closes the
    /// Today/Anytime/Someday/"Schedule…" quick-picker.
    fn toggle_schedule_picker(&mut self, cx: &mut Context<Self>) {
        self.schedule_picker_open = !self.schedule_picker_open;
        cx.notify();
    }

    /// Removes a task's scheduled date/time while leaving its bucket
    /// unchanged, e.g. an Active-bucket task drops back into Anytime (which
    /// is exactly "Active with no `scheduled_date`" — see `list_view`), an
    /// Inbox task just loses its date. Closes the picker
    /// `docs/HANDOFF.md` gap could only change a schedule, never remove one.
    fn clear_schedule(&mut self, id: String, bucket: Bucket, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else { return };
        self.expanded_task_id = None;
        self.schedule_picker_open = false;
        cx.spawn(async move |flow, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { db.schedule(id, bucket, None::<String>, None::<String>) })
                .await;
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

    /// Moves a task to `target` via `Db::schedule` and refreshes every task
    /// view — the quick-picker is now reachable from any of the five, so
    /// the exact set of affected views is no longer just Inbox + one
    /// destination.
    fn process_task(&mut self, id: String, target: ProcessTarget, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else { return };
        self.expanded_task_id = None;
        self.schedule_picker_open = false;
        let today = (target == ProcessTarget::Today)
            .then(|| chrono::Local::now().date_naive().to_string());

        cx.spawn(async move |flow, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { db.schedule(id, target.bucket(), today, None::<String>) })
                .await;
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

    /// `docs/DESIGN_DIRECTION.md`'s task detail spec names "delete" as an
    /// exposed action. Soft-deletes via `Db::delete_task`, which every
    /// `list_view` query already filters out.
    fn delete_task(&mut self, id: String, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else { return };
        self.expanded_task_id = None;
        self.schedule_picker_open = false;
        cx.spawn(async move |flow, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { db.delete_task(id) })
                .await;
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

    /// The floating "Completed \"…\"" banner shown after a task finishes its
    /// collapse animation. Rendered once at the window level (`render.rs`),
    /// not per view, so it survives navigating away from the view the
    /// completion happened in — the same reason `origin_view` travels with
    /// it instead of being read back off `self.destination`.
    pub(super) fn render_undo_toast(&mut self, theme: Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        let toast = self.undo_toast.as_ref()?;
        let title = toast.title.clone();

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
                                .child(format!("Completed \u{201c}{title}\u{201d}")),
                        )
                        .child(
                            div()
                                .id("undo-toast-button")
                                .px(px(8.0))
                                .py(px(3.0))
                                .rounded(px(5.0))
                                .cursor_pointer()
                                .text_size(px(12.5))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.accent)
                                .hover(|el| el.bg(theme.overlay))
                                .on_click(cx.listener(|flow, _, _, cx| flow.undo_complete(cx)))
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
        let selected = self.selected_task_ids.clone();
        let completed_expanded = self.completed_expanded.contains(&view);
        let completing_ids = self.completing_ids.clone();
        // Always fetched, not only once expanded — the collapsed row still
        // needs a count, and this is the same "resolve the whole collection
        // up front, render degrades on a miss" pattern `read_view` already
        // follows rather than a second, gated fetch.
        let completed = match self.read_completed(view, cx) {
            Query::Ready(tasks) => tasks,
            Query::Pending | Query::Missing(_) => Arc::new(Vec::new()),
        };
        match self.read_view(view, cx) {
            Query::Ready(tasks) => task_list(
                view,
                tasks,
                completed,
                completed_expanded,
                completing_ids,
                expanded,
                schedule_picker_open,
                scheduling,
                note_input,
                schedule_input,
                selected,
                theme,
                cx,
            )
            .into_any_element(),
            Query::Pending | Query::Missing(_) => loading_skeleton(theme).into_any_element(),
        }
    }
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
    schedule_input: Entity<ComposerInput>,
    selected: HashSet<String>,
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
        .child(
            div()
                .id("task-list-open")
                .flex_1()
                .min_h(px(0.0))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .px(px(24.0))
                .pt(px(40.0))
                .pb(px(20.0))
                .gap(px(1.0))
                .when(selected_count >= 2, |list| {
                    list.child(bulk_action_bar(selected_count, theme, cx))
                })
                .children(tasks.iter().cloned().map(|task| {
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
                        schedule_input.clone(),
                        theme,
                        cx,
                    )
                })),
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
                        schedule_input,
                        theme,
                        cx,
                    )),
            )
        })
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
    schedule_input: Entity<ComposerInput>,
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
                            schedule_input.clone(),
                            theme,
                            cx,
                        )
                    })),
            )
        })
        .child(
            div()
                .id(SharedString::from(format!("completed-toggle-{view:?}")))
                .h(px(28.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .px(px(8.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .hover(|el| el.bg(theme.overlay))
                .on_click(cx.listener(move |flow, _, _, cx| flow.toggle_completed_expanded(view, cx)))
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
    schedule_input: Entity<ComposerInput>,
    theme: Theme,
    cx: &mut Context<Flow>,
) -> AnyElement {
    let id_for_row_click = task.id.clone();
    let note_for_click = task.note.clone();

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
            schedule_input,
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
    // The schedule metadata is a fact about the task, not the view it's
    // being read from — a scheduled Inbox task (PRD §14) shows its date the
    // same as a scheduled Today/Upcoming one.
    let schedule = schedule_label(&task);

    div()
        .id(gpui::SharedString::from(format!("task-{}", task.id)))
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
                    element.opacity(1.0 - delta).h(px(40.0 * (1.0 - delta)))
                } else {
                    element.opacity(delta)
                }
            },
        )
        .into_any_element()
}

/// `docs/DESIGN_DIRECTION.md`'s "Task detail" component: a 10 px raised
/// surface, not a modal, exposing note, schedule, move, and delete in that
/// order. Subtasks are skipped — no UI or `parent_id` queries exist yet.
#[allow(clippy::too_many_arguments)]
fn render_detail_card(
    task: &Task,
    origin_view: View,
    schedule_picker_open: bool,
    scheduling: bool,
    note_input: Entity<ComposerInput>,
    schedule_input: Entity<ComposerInput>,
    theme: Theme,
    cx: &mut Context<Flow>,
) -> AnyElement {
    let completed = task.completed_at.is_some();
    let id_for_complete = task.id.clone();
    let title_for_complete = task.title.clone();
    let id_for_delete = task.id.clone();
    let id_for_collapse = task.id.clone();
    let note_for_collapse = task.note.clone();
    let placement = placement_label(task);

    div()
        .my(px(1.0))
        .p(px(12.0))
        .rounded(px(10.0))
        .bg(theme.raised)
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
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
                            flow.toggle_completed(
                                id_for_complete.clone(),
                                title_for_complete.clone(),
                                !completed,
                                origin_view,
                                cx,
                            );
                            cx.stop_propagation();
                        })),
                )
                .child(
                    div()
                        .id(gpui::SharedString::from(format!("task-{}-detail-title", task.id)))
                        .flex_1()
                        .min_w_0()
                        .cursor_pointer()
                        .text_size(px(15.0))
                        .text_color(theme.text)
                        .on_click(cx.listener(move |flow, _, _, cx| {
                            flow.toggle_expanded(id_for_collapse.clone(), note_for_collapse.clone(), cx);
                        }))
                        .child(task.title.clone()),
                ),
        )
        .child(note_input)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(render_schedule_pill(task.id.clone(), placement, theme, cx))
                .child(
                    crate::ui::icon_button(
                        gpui::SharedString::from(format!("task-{}-delete", task.id)),
                        "icons/trash.svg",
                        theme,
                    )
                    .on_click(cx.listener(move |flow, _, _, cx| {
                        flow.delete_task(id_for_delete.clone(), cx);
                        cx.stop_propagation();
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
                theme,
                cx,
            ))
        })
        .into_any_element()
}

/// The detail card's schedule status: the task's current placement in the
/// same four buckets the quick-picker offers. Clicking it opens that
/// picker so placement can be changed without leaving the card.
fn render_schedule_pill(id: String, label: String, theme: Theme, cx: &mut Context<Flow>) -> AnyElement {
    div()
        .id(gpui::SharedString::from(format!("schedule-pill-{id}")))
        .px(px(8.0))
        .py(px(3.0))
        .rounded(px(5.0))
        .cursor_pointer()
        .text_size(px(11.5))
        .text_color(theme.text_secondary)
        .bg(theme.overlay)
        .hover(|el| el.bg(theme.overlay_strong).text_color(theme.text))
        .on_click(cx.listener(move |flow, _, _, cx| {
            flow.toggle_schedule_picker(cx);
            cx.stop_propagation();
        }))
        .child(label)
        .into_any_element()
}

/// PRD §6.3's "inline 'Process' action": three quick destinations plus
/// "Schedule…", which swaps in a free-text field parsed the same way
/// Capture parses a title (see `Flow::on_schedule_event`, `app.rs`) rather
/// than a calendar widget. Lives inside the task detail card's schedule
/// picker now, relocated from the old bare-pill row. A fifth "Clear" option
/// appears only when the task actually has a schedule to remove.
#[allow(clippy::too_many_arguments)]
fn render_process_row(
    task_id: String,
    bucket: Bucket,
    has_schedule: bool,
    scheduling: bool,
    schedule_input: Entity<ComposerInput>,
    theme: Theme,
    cx: &mut Context<Flow>,
) -> AnyElement {
    if scheduling {
        return div().child(schedule_input).into_any_element();
    }

    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .children(ProcessTarget::ALL.into_iter().map(|target| {
            let id = task_id.clone();
            div()
                .id(gpui::SharedString::from(format!("process-{task_id}-{}", target.label())))
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
                .on_click(cx.listener(move |flow, _, _, cx| {
                    flow.process_task(id.clone(), target, cx);
                    cx.stop_propagation();
                }))
                .child(crate::ui::icon(target.icon_path(), 12.0, theme.text_secondary))
                .child(target.label())
        }))
        .child(
            div()
                .id(gpui::SharedString::from(format!("process-{task_id}-schedule")))
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
                .on_click(cx.listener(move |flow, _, window, cx| {
                    flow.open_schedule_field(window, cx);
                    cx.stop_propagation();
                }))
                .child(crate::ui::icon("icons/calendar.svg", 12.0, theme.text_secondary))
                .child("Schedule…"),
        )
        .when(has_schedule, |row| {
            let id = task_id.clone();
            row.child(
                div()
                    .id(gpui::SharedString::from(format!("process-{task_id}-clear")))
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
                    .on_click(cx.listener(move |flow, _, _, cx| {
                        flow.clear_schedule(id.clone(), bucket, cx);
                        cx.stop_propagation();
                    }))
                    .child(crate::ui::icon("icons/x.svg", 12.0, theme.text_secondary))
                    .child("Clear"),
            )
        })
        .into_any_element()
}

/// Human-friendly rendering of a stored `YYYY-MM-DD`/`HH:mm` schedule pair:
/// "Today"/"Tomorrow" for the two nearest days, a bare weekday name for the
/// rest of the week, else a short date (`"Aug 23"`) — with the time appended
/// in 12-hour form when present (`"Tomorrow 6:00 PM"`). Shared by
/// `schedule_label` (the row's trailing label) and `placement_label` (the
/// detail card's status pill) so both read the same.
fn format_schedule(date: &str, time: Option<&str>, today: chrono::NaiveDate) -> String {
    let day_part = match chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(parsed) => match (parsed - today).num_days() {
            0 => "Today".to_string(),
            1 => "Tomorrow".to_string(),
            2..=6 => parsed.format("%A").to_string(),
            _ => parsed.format("%b %-d").to_string(),
        },
        // An unparseable stored date is not expected, but showing the raw
        // string beats panicking or hiding the schedule entirely.
        Err(_) => date.to_string(),
    };
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
