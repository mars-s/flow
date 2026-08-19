//! Hidden dev overlay: Cmd-Option-I toggles GPUI's built-in element
//! inspector (picking, hit-testing, and per-element state capture all live
//! in `gpui::Inspector`/`DivInspectorState` — see the pinned fork's
//! `crates/gpui/src/inspector.rs`). This file only supplies the panel UI
//! that renders whatever element is currently picked: bounds, content size,
//! and a full style dump, so a human or an agent reading a screenshot can
//! see actual layout/color state instead of guessing from source.
//!
//! No menu item on purpose, same as Zed's own inspector — it's a debug
//! tool, not a product surface. The whole thing only compiles under
//! `debug_assertions` (matching gpui's own gate), so it never touches a
//! release build.

use gpui::App;

#[cfg(debug_assertions)]
use gpui::{
    AnyElement, Context, DivInspectorState, Inspector, InspectorElementId, IntoElement,
    ParentElement, Styled, Window, div, prelude::*, px,
};

#[cfg(debug_assertions)]
use crate::theme::Theme;

pub fn init(_cx: &mut App) {
    #[cfg(debug_assertions)]
    {
        _cx.register_inspector_element(|id, state: &DivInspectorState, window, cx| {
            render_element_state(id, state, window, cx)
        });
        _cx.set_inspector_renderer(Box::new(render_panel));
    }
}

#[cfg(debug_assertions)]
fn render_panel(
    inspector: &mut Inspector,
    window: &mut Window,
    cx: &mut Context<Inspector>,
) -> AnyElement {
    let theme = Theme::current(cx);
    let picking = inspector.is_picking();
    let inspector_id = inspector.active_element_id().cloned();

    div()
        .id("flow-inspector")
        .size_full()
        .flex()
        .flex_col()
        .bg(theme.raised)
        .text_color(theme.text)
        .border_l_1()
        .border_color(theme.border)
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .justify_between()
                .h(px(36.0))
                .px(px(10.0))
                .border_b_1()
                .border_color(theme.border)
                .child(div().text_size(px(12.0)).child("Inspector"))
                .child(
                    div()
                        .id("inspector-pick")
                        .tab_index(0)
                        .px(px(8.0))
                        .py(px(3.0))
                        .rounded(px(5.0))
                        .cursor_default()
                        .text_size(px(11.0))
                        .when(picking, |el| el.bg(theme.accent).text_color(theme.on_inverse))
                        .when(!picking, |el| {
                            el.text_color(theme.text_secondary)
                                .hover(|el| el.bg(theme.overlay))
                        })
                        .on_click(cx.listener(|inspector, _, window, _cx| {
                            inspector.start_picking();
                            window.refresh();
                        }))
                        .child(if picking { "Picking…" } else { "Pick" }),
                ),
        )
        .child(
            div()
                .id("flow-inspector-content")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(px(10.0))
                .py(px(8.0))
                .text_size(px(11.0))
                .line_height(px(16.0))
                .when_some(inspector_id, |el, id| el.child(render_element_id(&id, theme)))
                .children(inspector.render_inspector_states(window, cx)),
        )
        .into_any_element()
}

#[cfg(debug_assertions)]
fn render_element_id(id: &InspectorElementId, theme: Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .pb(px(8.0))
        .mb(px(8.0))
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .text_color(theme.text_secondary)
                .child(format!("instance {}", id.instance_id)),
        )
        .child(
            div()
                .text_color(theme.text_tertiary)
                .child(id.path.source_location.to_string()),
        )
}

/// Registered per `DivInspectorState` (every `div()` captures one, whether
/// or not the caller opted into anything special) — GPUI hands this the
/// state for the currently-picked element. Style is dumped via `Debug`
/// rather than hand-picking fields: `StyleRefinement` derives it already,
/// and reflecting fields by hand would silently go stale as gpui's style
/// surface grows.
#[cfg(debug_assertions)]
fn render_element_state(
    _id: InspectorElementId,
    state: &DivInspectorState,
    _window: &mut Window,
    cx: &mut gpui::App,
) -> AnyElement {
    let theme = Theme::current(cx);
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_color(theme.text_secondary)
                .child(format!("bounds: {:?}", state.bounds)),
        )
        .child(
            div()
                .text_color(theme.text_secondary)
                .child(format!("content_size: {:?}", state.content_size)),
        )
        .child(
            div()
                .text_color(theme.text)
                .whitespace_normal()
                .child(format!("{:#?}", state.base_style)),
        )
        .into_any_element()
}
