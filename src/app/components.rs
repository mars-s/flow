//! Shared rendering primitives for Flow's placeholder shell.
//!
//! Milestone 0 has no task list yet: every destination renders the same
//! "coming soon" body. Kept as its own module so Milestone 1 can grow real
//! per-destination views without churning `render.rs`.

use gpui::{AnyElement, Div, Hsla, IntoElement, ParentElement, Rgba, Styled, div, prelude::*, px};

use super::Destination;
use crate::platform::CalendarEvent;
use crate::theme::Theme;

/// Title + "coming soon" placeholder body for a not-yet-built destination.
pub(super) fn placeholder_pane(theme: Theme, destination: Destination) -> Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .bg(theme.canvas)
        .child(
            div()
                .text_size(px(15.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text)
                .child(destination.label()),
        )
        .child(
            div()
                .text_size(px(11.5))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.text_tertiary)
                .child("Coming soon"),
        )
        .child(
            div()
                .mt(px(2.0))
                .max_w(px(320.0))
                .text_center()
                .text_size(px(12.0))
                .line_height(px(18.0))
                .text_color(theme.text_ghost)
                .child(destination.placeholder_copy()),
        )
}

/// Today's calendar glance (PRD §6.3, §6.5 revised 2026-08-19). Only ever
/// called once `calendar_auth == Granted` — see `tasks.rs`'s call site —
/// so there's no "not connected" state to render here; that state lives in
/// the card simply not appearing (PRD §6.5: "the Today calendar-glance
/// card is hidden entirely rather than showing an empty or disconnected
/// state").
///
/// `events` is `None` while the first fetch for this Today mount is still
/// in flight (`Flow::refresh_today_calendar_events`) — shows just the date
/// header rather than a loading skeleton, since a calendar with genuinely
/// zero events today looks identical and this is a small, low-stakes
/// glance rather than a primary content area.
pub(super) fn calendar_glance(theme: Theme, events: Option<&[CalendarEvent]>) -> AnyElement {
    let mut events: Vec<&CalendarEvent> = events.unwrap_or(&[]).iter().collect();
    events.sort_by_key(|event| (event.all_day, event.start));

    div()
        .flex_none()
        .mb(px(10.0))
        .p(px(10.0))
        .rounded(px(10.0))
        .bg(theme.raised)
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.text_tertiary)
                // `%b %-d` (not `%-d %b`) to match `tasks.rs::day_label`'s
                // established short-date word order ("Aug 23") elsewhere
                // in the app, rather than a second convention.
                .child(chrono::Local::now().format("%A, %b %-d").to_string()),
        )
        .when(events.is_empty(), |card| {
            card.child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.text_ghost)
                    .child("No events today"),
            )
        })
        .children(events.into_iter().map(|event| {
            // Never a `theme` token: per `DESIGN_DIRECTION.md`'s "Calendar
            // colors remain calendar colors, never Flow status colors,"
            // an event's color is calendar data, not app state.
            let (r, g, b, a) = event.color;
            let color: Hsla = Rgba { r, g, b, a }.into();
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().flex_none().size(px(6.0)).rounded_full().bg(color))
                .child(
                    div()
                        .flex_none()
                        .w(px(64.0))
                        .text_size(px(12.0))
                        .text_color(theme.text_secondary)
                        .child(if event.all_day {
                            "All day".to_string()
                        } else {
                            event.start.format("%-I:%M %p").to_string()
                        }),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(12.5))
                        .text_color(theme.text)
                        .child(event.title.clone()),
                )
        }))
        .into_any_element()
}
