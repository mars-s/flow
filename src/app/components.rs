//! Shared rendering primitives for Flow's placeholder shell.
//!
//! Milestone 0 has no task list yet: every destination renders the same
//! "coming soon" body. Kept as its own module so Milestone 1 can grow real
//! per-destination views without churning `render.rs`.

use gpui::{AnyElement, Div, Hsla, IntoElement, ParentElement, Styled, div, hsla, px};

use super::Destination;
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

/// One event on the fixture-backed calendar glance below.
struct GlanceEvent {
    time: &'static str,
    title: &'static str,
    /// Stands in for a real per-calendar color (`calendar_events.color`,
    /// PRD §8) until Milestone 3 wires up an actual Google Calendar
    /// connection. Deliberately not a `theme` token: per
    /// `DESIGN_DIRECTION.md`'s "Calendar colors remain calendar colors,
    /// never Flow status colors," an event's color is calendar data, not
    /// app state, so it never comes from the semantic palette.
    color: Hsla,
}

/// Milestone 1's PRD §12 exit scope: "a small fixture-backed calendar-
/// glance component only to prove layout" — real Google Calendar data is
/// Milestone 3. Scoped to Today only, matching §6.3's literal text ("A
/// compact calendar-glance card precedes the tasks" appears in Today's own
/// paragraph, not Upcoming's or any other view's).
///
/// This intentionally skips the connected/loading/error states
/// `DESIGN_DIRECTION.md`'s required-states table lists for the calendar
/// rail — there is no real "is a calendar connected" concept yet to be in
/// any of those states, so showing the populated fixture unconditionally
/// is what actually proves the layout; a fake "not connected" empty state
/// would need to be swapped for a real one anyway once Milestone 3 lands.
pub(super) fn calendar_glance(theme: Theme) -> AnyElement {
    // The same three example events from `DESIGN_DIRECTION.md`'s own
    // calendar-rail mockup, reused rather than invented, so the fixture
    // matches the approved visual reference instead of drifting from it.
    let events = [
        GlanceEvent { time: "8:00 AM", title: "Laundry", color: hsla(38.0 / 360.0, 0.75, 0.62, 1.0) },
        GlanceEvent { time: "10:00 AM", title: "Research", color: hsla(200.0 / 360.0, 0.55, 0.60, 1.0) },
        GlanceEvent { time: "3:30 PM", title: "Design sync", color: hsla(280.0 / 360.0, 0.45, 0.68, 1.0) },
    ];

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
                .child(chrono::Local::now().format("%A, %-d %b").to_string()),
        )
        .children(events.into_iter().map(|event| {
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().flex_none().size(px(6.0)).rounded_full().bg(event.color))
                .child(
                    div()
                        .flex_none()
                        .w(px(64.0))
                        .text_size(px(12.0))
                        .text_color(theme.text_secondary)
                        .child(event.time),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(12.5))
                        .text_color(theme.text)
                        .child(event.title),
                )
        }))
        .into_any_element()
}
