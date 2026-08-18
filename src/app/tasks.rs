//! Milestone 1: the five task views (Inbox, Today, Upcoming, Anytime,
//! Someday), all backed by `crate::db::View` and sharing one row renderer.

use std::sync::Arc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, Context, IntoElement, ParentElement, Styled, Window,
    div, ease_out_quint, prelude::*, px,
};

use super::Flow;
use super::sidebar::Destination;
use crate::db::{Bucket, Task, View};
use crate::query::Query;
use crate::theme::Theme;

/// PRD §7's completion-collapse timing; reused here for the row's fade-in
/// too since nothing in the direction doc distinguishes them.
const ROW_TRANSITION: Duration = Duration::from_millis(180);

/// The three quick destinations PRD §6.3 names for Inbox's inline "Process"
/// action. "Schedule" (an arbitrary date, not just "today") is the fourth
/// option the PRD names but needs a real date picker — not built yet, see
/// docs/HANDOFF.md.
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

    fn view(self) -> View {
        match self {
            ProcessTarget::Today => View::Today,
            ProcessTarget::Anytime => View::Anytime,
            ProcessTarget::Someday => View::Someday,
        }
    }

    fn bucket(self) -> Bucket {
        match self {
            ProcessTarget::Today | ProcessTarget::Anytime => Bucket::Active,
            ProcessTarget::Someday => Bucket::Someday,
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

    /// Every view that reads `Bucket::Active` is invalidated together,
    /// since a write to one (e.g. completing a Today task) can change what
    /// Upcoming/Anytime show too (a rescheduled task moves between them).
    /// Inbox and Someday are unaffected by an active-task write and are
    /// invalidated individually by their own call sites.
    fn invalidate_active_views(&mut self) {
        for view in [View::Today, View::Upcoming, View::Anytime] {
            self.tasks.invalidate(&view);
        }
    }

    fn invalidate_view(&mut self, view: View) {
        self.tasks.invalidate(&view);
    }

    /// Toggles a task's completion and refreshes whatever view is currently
    /// showing it. Fire-and-forget from the caller's point of view — the
    /// row updates when `cx.notify()` lands, same as a fetch. Which cache
    /// entries to invalidate is decided by `origin_view` rather than
    /// guessed from the task's bucket, since Today and Upcoming both read
    /// `Bucket::Active` and either could be the row the click came from.
    fn toggle_completed(&mut self, id: String, completed: bool, origin_view: View, cx: &mut Context<Self>) {
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
                    View::Today | View::Upcoming | View::Anytime => flow.invalidate_active_views(),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Opens or closes Inbox's inline Process row for `id`. Only one row
    /// processes at a time — opening a second closes whichever was open.
    fn toggle_processing(&mut self, id: String, cx: &mut Context<Self>) {
        self.processing_task_id = if self.processing_task_id.as_deref() == Some(id.as_str()) {
            None
        } else {
            Some(id)
        };
        cx.notify();
    }

    /// Moves an Inbox task to `target` via `Db::schedule` and refreshes
    /// both the Inbox listing (the task leaves it) and whatever view it
    /// lands in.
    fn process_task(&mut self, id: String, target: ProcessTarget, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else { return };
        self.processing_task_id = None;
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
                flow.invalidate_view(View::Inbox);
                flow.invalidate_view(target.view());
                cx.notify();
            });
        })
        .detach();
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

        let processing = self.processing_task_id.clone();
        match self.read_view(view, cx) {
            Query::Ready(tasks) => task_list(view, tasks, processing, theme, cx).into_any_element(),
            Query::Pending | Query::Missing(_) => loading_skeleton(theme).into_any_element(),
        }
    }
}

fn task_list(
    view: View,
    tasks: Arc<Vec<Task>>,
    processing: Option<String>,
    theme: Theme,
    cx: &mut Context<Flow>,
) -> AnyElement {
    if tasks.is_empty() {
        return empty_state(view, theme).into_any_element();
    }

    let show_schedule = matches!(view, View::Today | View::Upcoming);
    // PRD §6.3: Process is an Inbox-only affordance. Today/Upcoming/Anytime
    // tasks are already placed; Someday intentionally stays quiet.
    let processable = view == View::Inbox;

    div()
        .id("task-list")
        .size_full()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .px(px(24.0))
        .py(px(20.0))
        .gap(px(1.0))
        .children(tasks.iter().cloned().map(|task| {
            let is_processing = processable && processing.as_deref() == Some(task.id.as_str());
            render_task_row(task, view, show_schedule, processable, is_processing, theme, cx)
        }))
        .into_any_element()
}

fn render_task_row(
    task: Task,
    origin_view: View,
    show_schedule: bool,
    processable: bool,
    is_processing: bool,
    theme: Theme,
    cx: &mut Context<Flow>,
) -> AnyElement {
    let completed = task.completed_at.is_some();
    let id_for_click = task.id.clone();
    let id_for_row_click = task.id.clone();
    let schedule = show_schedule.then(|| schedule_label(&task)).flatten();

    div()
        .flex()
        .flex_col()
        .child(
            div()
                .id(gpui::SharedString::from(format!("task-{}", task.id)))
                .h(px(40.0))
                .flex()
                .items_center()
                .gap(px(10.0))
                .px(px(8.0))
                .rounded(px(6.0))
                .when(processable, |row| row.cursor_pointer())
                .hover(|el| el.bg(theme.overlay))
                .when(processable, |row| {
                    row.on_click(cx.listener(move |flow, _, _, cx| {
                        flow.toggle_processing(id_for_row_click.clone(), cx);
                    }))
                })
                .child(
                    div()
                        .id(gpui::SharedString::from(format!("task-{}-complete", task.id)))
                        .w(px(17.0))
                        .h(px(17.0))
                        .flex_none()
                        .rounded_full()
                        .border_1()
                        .border_color(theme.border_strong)
                        .cursor_default()
                        .hover(|el| el.border_color(theme.accent))
                        .on_click(cx.listener(move |flow, _, _, cx| {
                            flow.toggle_completed(id_for_click.clone(), !completed, origin_view, cx);
                            cx.stop_propagation();
                        })),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.0))
                        .text_color(theme.text)
                        .child(task.title),
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
                    gpui::SharedString::from(format!("task-fade-{}", task.id)),
                    Animation::new(ROW_TRANSITION).with_easing(ease_out_quint()),
                    |element, delta| element.opacity(delta),
                ),
        )
        .when(is_processing, |column| {
            column.child(render_process_row(task.id, theme, cx))
        })
        .into_any_element()
}

/// PRD §6.3's "inline 'Process' action" on an Inbox row: three quick
/// destinations. "Schedule" (an arbitrary date) needs a real date picker
/// and isn't built yet — see docs/HANDOFF.md.
fn render_process_row(task_id: String, theme: Theme, cx: &mut Context<Flow>) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .pl(px(35.0))
        .pb(px(6.0))
        .children(ProcessTarget::ALL.into_iter().map(|target| {
            let id = task_id.clone();
            div()
                .id(gpui::SharedString::from(format!("process-{task_id}-{}", target.label())))
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
                .child(target.label())
        }))
        .into_any_element()
}

/// `scheduled_date`/`scheduled_time` are stored as plain `YYYY-MM-DD`/
/// `HH:mm` strings (`docs/PRODUCT_REQUIREMENTS.md` §6.4) — shown as-is for
/// now. A friendlier "Tomorrow · 8:00 AM" rendering belongs to the NLP
/// parser work, which owns date formatting throughout the app.
fn schedule_label(task: &Task) -> Option<String> {
    let date = task.scheduled_date.as_deref()?;
    Some(match &task.scheduled_time {
        Some(time) => format!("{date} · {time}"),
        None => date.to_string(),
    })
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

