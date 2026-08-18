//! The fixed navigation rail: the seven Flow destinations plus "+ New task".
//!
//! Milestone 0 has no task store yet, so selecting a destination only swaps
//! the main pane's placeholder (see `render.rs`/`components.rs`). Every row
//! is reachable and operable by keyboard per this repo's accessibility
//! conventions: `tab` moves focus without changing the selection, arrow keys
//! move focus and select (a conventional listbox), and `enter`/`space`
//! select whatever row currently has focus.

use gpui::{App, Context, Div, KeyDownEvent, SharedString, Stateful, Window, div, prelude::*, px};

use super::Flow;
use crate::theme::Theme;

/// No global keybindings: the rail's arrow/enter handling is scoped to its
/// own rows via `on_key_down`, not registered as app-wide actions.
pub fn init(_cx: &mut App) {}

const SIDEBAR_WIDTH: f32 = 272.0;
const ROW_HEIGHT: f32 = 30.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Destination {
    Inbox,
    Today,
    Upcoming,
    Anytime,
    Someday,
    Calendar,
    Settings,
}

impl Destination {
    pub(super) const ALL: [Destination; 7] = [
        Destination::Inbox,
        Destination::Today,
        Destination::Upcoming,
        Destination::Anytime,
        Destination::Someday,
        Destination::Calendar,
        Destination::Settings,
    ];

    pub(super) const COUNT: usize = Self::ALL.len();

    pub(super) fn label(self) -> &'static str {
        match self {
            Destination::Inbox => "Inbox",
            Destination::Today => "Today",
            Destination::Upcoming => "Upcoming",
            Destination::Anytime => "Anytime",
            Destination::Someday => "Someday",
            Destination::Calendar => "Calendar",
            Destination::Settings => "Settings",
        }
    }

    /// One sentence of what this destination will show once it is built.
    pub(super) fn placeholder_copy(self) -> &'static str {
        match self {
            Destination::Inbox => {
                "New tasks land here the moment they're captured, waiting to be processed."
            }
            Destination::Today => "Overdue and today's active tasks, with a calendar glance.",
            Destination::Upcoming => "Dated tasks ahead, grouped by day.",
            Destination::Anytime => "Active work with no date yet.",
            Destination::Someday => "Work you've deliberately deferred.",
            Destination::Calendar => "A read-only glance at your connected calendar.",
            Destination::Settings => "Account, calendar, and appearance settings.",
        }
    }

    pub(super) fn index(self) -> usize {
        self as usize
    }

    pub(super) fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::COUNT]
    }
}

impl Flow {
    /// Move keyboard focus to `destination`'s row and select it. Used by
    /// arrow-key navigation, which — unlike `tab` — both moves focus and
    /// changes the active destination.
    fn focus_and_select_destination(
        &mut self,
        destination: Destination,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.nav_focuses[destination.index()], cx);
        self.set_destination(destination, window, cx);
    }

    pub(super) fn render_sidebar(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::current(cx);

        div()
            .id("sidebar")
            .flex_none()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .px(px(12.0))
            .py(px(16.0))
            .bg(theme.sidebar)
            .border_r_1()
            .border_color(theme.sidebar_border)
            .child(
                div()
                    .px(px(4.0))
                    .pb(px(10.0))
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.text)
                    .child("Flow"),
            )
            .child(self.render_new_task_button(theme))
            .child(
                div()
                    .id("sidebar-nav")
                    .tab_group()
                    .tab_index(1)
                    .tab_stop(false)
                    .mt(px(6.0))
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .children(
                        Destination::ALL
                            .into_iter()
                            .map(|destination| self.render_nav_row(destination, theme, cx)),
                    ),
            )
    }

    fn render_new_task_button(&self, theme: Theme) -> Stateful<Div> {
        // ponytail: capture has no composer yet (Milestone 1). The row is
        // fully keyboard/mouse operable so the affordance is real, it just
        // has nothing to do until the task store exists.
        div()
            .id("new-task")
            .track_focus(&self.new_task_focus)
            .tab_index(0)
            .h(px(ROW_HEIGHT))
            .px(px(8.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .cursor_default()
            .text_size(px(12.5))
            .text_color(theme.text_secondary)
            .hover(|el| el.bg(theme.overlay))
            .active(|el| el.bg(theme.overlay_strong))
            .focus_visible(|style| style.border_1().border_color(theme.accent))
            .child("+")
            .child("New task")
    }

    fn render_nav_row(
        &self,
        destination: Destination,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let index = destination.index();
        let focus = self.nav_focuses[index].clone();
        let selected = self.destination == destination;
        let badge = (destination == Destination::Inbox).then_some(0u32);

        div()
            .id(SharedString::from(format!("nav-{index}")))
            .track_focus(&focus)
            .tab_index(0)
            .h(px(ROW_HEIGHT))
            .px(px(8.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_between()
            .cursor_default()
            .text_size(px(12.5))
            .when(selected, |row| {
                row.bg(theme.sidebar_item_background).text_color(theme.text)
            })
            .when(!selected, |row| {
                row.text_color(theme.text_secondary)
                    .hover(|el| el.bg(theme.overlay))
            })
            .active(|el| el.bg(theme.overlay_strong))
            .focus_visible(|style| style.border_1().border_color(theme.accent))
            .on_click(cx.listener(move |this, _, window, cx| {
                window.focus(&this.nav_focuses[index], cx);
                this.set_destination(destination, window, cx);
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "enter" | "space" => {
                        this.set_destination(destination, window, cx);
                        cx.stop_propagation();
                    }
                    "down" => {
                        this.focus_and_select_destination(
                            Destination::from_index(index + 1),
                            window,
                            cx,
                        );
                        cx.stop_propagation();
                    }
                    "up" => {
                        let previous = if index == 0 {
                            Destination::COUNT - 1
                        } else {
                            index - 1
                        };
                        this.focus_and_select_destination(
                            Destination::from_index(previous),
                            window,
                            cx,
                        );
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(destination.label())
            .when_some(badge, |row, count| {
                row.child(
                    div()
                        .min_w(px(16.0))
                        .h(px(16.0))
                        .px(px(4.0))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(theme.overlay_strong)
                        .text_size(px(10.0))
                        .text_color(theme.text_tertiary)
                        .child(count.to_string()),
                )
            })
    }
}
