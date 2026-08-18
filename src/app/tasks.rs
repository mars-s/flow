//! Milestone 1: task views backed by `crate::db`. Currently just Inbox —
//! Today/Upcoming/Anytime/Someday reuse `read_bucket`/`render_task_row` once
//! their own bucket-filtering views exist (see `docs/HANDOFF.md`).

use std::sync::Arc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, Context, IntoElement, ParentElement, Styled, Window,
    div, ease_out_quint, prelude::*, px,
};

use super::Flow;
use crate::db::{Bucket, Task};
use crate::query::Query;
use crate::theme::Theme;

/// PRD §7's completion-collapse timing; reused here for the row's fade-in
/// too since nothing in the direction doc distinguishes them.
const ROW_TRANSITION: Duration = Duration::from_millis(180);

impl Flow {
    /// Reads a cached bucket, kicking off a background fetch on a miss.
    /// Safe to call from `render` — the miss path only spawns work, per
    /// `query.rs`'s own doc comment (the pattern this follows exactly).
    pub(super) fn read_bucket(&mut self, bucket: Bucket, cx: &mut Context<Self>) -> Query<Bucket, Vec<Task>> {
        let query = self.tasks.read(&bucket);
        if let Query::Missing(token) = &query {
            let Some(db) = self.db.clone() else {
                return query;
            };
            let token = token.clone();
            cx.spawn(async move |flow, cx| {
                let Ok(tasks) = cx
                    .background_executor()
                    .spawn(async move { db.list_bucket(bucket) })
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

    /// Drops the cached value for `bucket` so the next `read_bucket` call
    /// refetches it. Called after any write that could change its contents.
    fn invalidate_bucket(&mut self, bucket: Bucket) {
        self.tasks.invalidate(&bucket);
    }

    /// Toggles a task's completion and refreshes whatever bucket is
    /// currently showing it. Fire-and-forget from the caller's point of
    /// view — the row updates when `cx.notify()` lands, same as a fetch.
    fn toggle_completed(&mut self, id: String, completed: bool, cx: &mut Context<Self>) {
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
                flow.invalidate_bucket(Bucket::Inbox);
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn render_inbox(&mut self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        if self.db.is_none() {
            return database_unavailable(theme).into_any_element();
        }

        match self.read_bucket(Bucket::Inbox, cx) {
            Query::Ready(tasks) => task_list(tasks, theme, cx).into_any_element(),
            Query::Pending | Query::Missing(_) => loading_skeleton(theme).into_any_element(),
        }
    }
}

fn task_list(tasks: Arc<Vec<Task>>, theme: Theme, cx: &mut Context<Flow>) -> AnyElement {
    if tasks.is_empty() {
        return empty_inbox(theme).into_any_element();
    }

    div()
        .id("inbox-list")
        .size_full()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .px(px(24.0))
        .py(px(20.0))
        .gap(px(1.0))
        .children(tasks.iter().cloned().map(|task| render_task_row(task, theme, cx)))
        .into_any_element()
}

fn render_task_row(task: Task, theme: Theme, cx: &mut Context<Flow>) -> AnyElement {
    let completed = task.completed_at.is_some();
    let id_for_click = task.id.clone();

    div()
        .id(gpui::SharedString::from(format!("task-{}", task.id)))
        .h(px(40.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(8.0))
        .rounded(px(6.0))
        .hover(|el| el.bg(theme.overlay))
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
                    flow.toggle_completed(id_for_click.clone(), !completed, cx);
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
        .with_animation(
            gpui::SharedString::from(format!("task-fade-{}", task.id)),
            Animation::new(ROW_TRANSITION).with_easing(ease_out_quint()),
            |element, delta| element.opacity(delta),
        )
        .into_any_element()
}

/// PRD §7's "Required UI states" table, Inbox row: "Nothing to process.
/// Capture the next thing."
fn empty_inbox(theme: Theme) -> impl IntoElement {
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
                .child("Nothing to process. Capture the next thing."),
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
                .id(gpui::SharedString::from(format!("inbox-skeleton-{index}")))
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
