//! Shared rendering primitives for Flow's placeholder shell.
//!
//! Milestone 0 has no task list yet: every destination renders the same
//! "coming soon" body. Kept as its own module so Milestone 1 can grow real
//! per-destination views without churning `render.rs`.

use gpui::{Div, ParentElement, Styled, div, px};

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
