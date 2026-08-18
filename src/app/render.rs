//! Top-level frame: native window chrome, the fixed sidebar, and the main
//! pane that cross-fades between destination placeholders.

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, Context, IntoElement, ParentElement, Render,
    SharedString, Styled, Window, div, ease_out_quint, prelude::*,
};

use super::Flow;
use super::components::placeholder_pane;
use super::sidebar::Destination;
use crate::theme::Theme;

/// PRD §7: "Navigation cross-fades or slides only the main pane (120–160
/// ms)". `with_animation` already resolves instantly under
/// `cx.reduce_motion()` (see the toast fade below, the pattern this reuses).
const MAIN_PANE_TRANSITION: Duration = Duration::from_millis(140);

impl Render for Flow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);

        let content = div()
            .id("flow-shell")
            .key_context("Flow")
            .relative()
            .size_full()
            .flex()
            .bg(theme.canvas)
            .text_color(theme.text)
            .on_action(cx.listener(Self::handle_new_task_action))
            .on_action(cx.listener(Self::handle_space_capture_action))
            .on_action(cx.listener(Self::handle_cancel_turn_action))
            .child(self.render_sidebar(cx))
            .child(self.render_main_pane(theme, cx))
            .children(self.render_undo_toast(theme, cx))
            // Layered on top rather than reserving its own row, so the
            // sidebar's border keeps running the full window height instead
            // of stopping at a row boundary above it.
            .child(self.render_drag_bar(cx))
            .into_any_element();

        self.render_window_frame(content, window, cx)
    }
}

impl Flow {
    fn render_main_pane(&mut self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        let destination = self.destination;
        let body = match destination.view() {
            Some(_) => self.render_task_view(destination, theme, cx),
            None => placeholder_pane(theme, destination).into_any_element(),
        };
        div()
            .id("main-pane")
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_hidden()
            .child(
                div()
                    .id(SharedString::from(format!("pane-{}", destination.index())))
                    .size_full()
                    .child(body)
                    .with_animation(
                        SharedString::from(format!("pane-fade-{}", destination.index())),
                        Animation::new(MAIN_PANE_TRANSITION).with_easing(ease_out_quint()),
                        |element, delta| element.opacity(delta),
                    ),
            )
            .into_any_element()
    }
}
