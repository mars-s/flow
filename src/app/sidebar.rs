//! The fixed navigation rail: a Tasks/Calendar mode switch, the five task
//! views inside Tasks mode, and Settings pinned to the bottom.
//!
//! Follows docs/DESIGN_DIRECTION.md's navigation-rail spec: 252px fixed
//! width, icon-plus-label rows, Inbox's count in a soft pill aligned right,
//! the active row filled with a focus-soft background. Tasks and Calendar
//! are Flow's two whole-app modes, switched with a segmented control at the
//! top of the rail; Settings is a single row pinned to the bottom, reachable
//! from either mode.
//!
//! Milestone 0 has no task store yet, so selecting a destination only swaps
//! the main pane's placeholder (see `render.rs`/`components.rs`). Every row
//! is reachable and operable by keyboard per this repo's accessibility
//! conventions: `tab` moves focus without changing the selection, arrow keys
//! move focus and select within the task list (a conventional listbox), and
//! `enter`/`space` select whatever row currently has focus.

use gpui::{
    Animation, AnimationExt, AnyElement, App, Context, Div, IntoElement, KeyDownEvent,
    SharedString, Stateful, Window, div, ease_out_quint, prelude::*, px,
};

use super::Flow;
use crate::theme::Theme;
use crate::ui::icon;
use crate::ui::motion;

/// No global keybindings: the rail's arrow/enter handling is scoped to its
/// own rows via `on_key_down`, not registered as app-wide actions.
pub fn init(_cx: &mut App) {}

const SIDEBAR_WIDTH: f32 = 252.0;
const ROW_HEIGHT: f32 = 32.0;

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

/// The sidebar's two whole-app modes. Settings is reachable from either and
/// does not belong to a mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Tasks,
    Calendar,
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

    /// The rows Tasks mode lists, in display order.
    const TASK_VIEWS: [Destination; 5] = [
        Destination::Inbox,
        Destination::Today,
        Destination::Upcoming,
        Destination::Anytime,
        Destination::Someday,
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

    /// The database view this destination reads, or `None` for the two
    /// destinations with no task list of their own (Calendar, Settings).
    pub(super) fn view(self) -> Option<crate::db::View> {
        match self {
            Destination::Inbox => Some(crate::db::View::Inbox),
            Destination::Today => Some(crate::db::View::Today),
            Destination::Upcoming => Some(crate::db::View::Upcoming),
            Destination::Anytime => Some(crate::db::View::Anytime),
            Destination::Someday => Some(crate::db::View::Someday),
            Destination::Calendar | Destination::Settings => None,
        }
    }

    fn icon_path(self) -> &'static str {
        match self {
            Destination::Inbox => "icons/inbox.svg",
            Destination::Today => "icons/star.svg",
            Destination::Upcoming => "icons/list.svg",
            Destination::Anytime => "icons/layers.svg",
            Destination::Someday => "icons/archive.svg",
            Destination::Calendar => "icons/calendar.svg",
            Destination::Settings => "icons/settings.svg",
        }
    }

    pub(super) fn index(self) -> usize {
        self as usize
    }

    pub(super) fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::COUNT]
    }

    /// `TASK_VIEWS[task_index]`'s position within `ALL`, for arrow-key wrap.
    fn task_view_index(self) -> Option<usize> {
        Self::TASK_VIEWS.iter().position(|&d| d == self)
    }
}

impl Mode {
    fn icon_path(self) -> &'static str {
        match self {
            Mode::Tasks => "icons/home.svg",
            Mode::Calendar => "icons/calendar.svg",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Mode::Tasks => "Tasks",
            Mode::Calendar => "Calendar",
        }
    }
}

impl Flow {
    /// The mode the segmented control currently highlights. Calendar is the
    /// only destination that belongs to Calendar mode; everything else,
    /// Settings included, reads as Tasks mode.
    fn sidebar_mode(&self) -> Mode {
        if self.destination == Destination::Calendar {
            Mode::Calendar
        } else {
            Mode::Tasks
        }
    }

    /// Move keyboard focus to `destination`'s row and select it. Used by
    /// arrow-key navigation, which — unlike `tab` — both moves focus and
    /// changes the active destination. Wraps within the five task views.
    fn focus_and_select_task_view(
        &mut self,
        task_index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let destination = Destination::TASK_VIEWS[task_index % Destination::TASK_VIEWS.len()];
        window.focus(&self.nav_focuses[destination.index()], cx);
        self.set_destination(destination, window, cx);
    }

    pub(super) fn render_sidebar(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::current(cx);
        let mode = self.sidebar_mode();

        div()
            .id("sidebar")
            .flex_none()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .px(px(12.0))
            .pt(px(super::window_chrome::DRAG_BAR_HEIGHT + 2.0))
            .pb(px(16.0))
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
            .child(self.render_capture_row(theme, cx))
            .child(self.render_mode_switch(mode, theme, cx))
            .child(
                div()
                    .id("sidebar-nav")
                    .tab_group()
                    .tab_index(1)
                    .tab_stop(false)
                    .mt(px(6.0))
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .when(mode == Mode::Tasks, |list| {
                        list.children(
                            Destination::TASK_VIEWS
                                .into_iter()
                                .map(|destination| self.render_nav_row(destination, theme, cx)),
                        )
                    }),
            )
            .child(
                div()
                    .mx(px(4.0))
                    .my(px(8.0))
                    .h(px(1.0))
                    .bg(theme.sidebar_border),
            )
            .child(self.render_nav_row(Destination::Settings, theme, cx))
    }

    /// The button when idle, or the real composer field once `capturing` is
    /// true (opened via a click/enter/space on the button, or `⌘N` from
    /// anywhere — see `Flow::open_capture` in `app.rs`).
    fn render_capture_row(&mut self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        if self.capturing {
            let error = self.capture_error.clone();
            return div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .h(px(ROW_HEIGHT))
                        .px(px(4.0))
                        .flex()
                        .items_center()
                        .child(self.capture_input.clone()),
                )
                // PRD §6.1: "show a non-blocking error with Retry on
                // failure" — `Flow::submit_capture` sets this on a failed
                // `Db::create_task`/`schedule` and restores the typed text
                // rather than losing it.
                .when_some(error, |row, message| {
                    row.child(
                        div()
                            .px(px(8.0))
                            .pb(px(4.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.danger)
                                    .child(message),
                            )
                            .child(
                                div()
                                    .id("capture-retry")
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.accent)
                                    .hover(|el| el.bg(theme.overlay))
                                    .on_click(cx.listener(|flow, _, _, cx| flow.retry_capture(cx)))
                                    .child("Retry"),
                            )
                            .with_animation(
                                // Mounts fresh each failure (unmounts when
                                // `capture_error` clears), matching the rest
                                // of the app's reveal-on-mount vocabulary.
                                "capture-error-reveal",
                                Animation::new(motion::TRANSITION).with_easing(ease_out_quint()),
                                |element, delta| element.opacity(delta),
                            ),
                    )
                })
                .into_any_element();
        }

        div()
            .id("capture")
            .track_focus(&self.new_task_focus)
            .tab_index(0)
            .h(px(ROW_HEIGHT))
            .px(px(8.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .cursor_default()
            .text_size(px(12.5))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(theme.text_secondary)
            .hover(|el| el.bg(theme.overlay))
            .active(|el| el.bg(theme.overlay_strong))
            .focus_visible(|style| style.border_1().border_color(theme.accent))
            .on_click(cx.listener(|flow, _, window, cx| flow.open_capture(window, cx)))
            .on_key_down(cx.listener(|flow, event: &KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    flow.open_capture(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(icon("icons/plus.svg", 14.0, theme.text_secondary))
            .child("Capture")
            .into_any_element()
    }

    /// The Tasks/Calendar pill: Flow's two whole-app modes. Each segment
    /// fills half the pill's width; the active segment gets a raised
    /// surface, matching the reference's "Home | Code" treatment but within
    /// the monochrome focus-blue system (no per-segment color).
    fn render_mode_switch(
        &self,
        mode: Mode,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        div()
            .id("mode-switch")
            .mt(px(6.0))
            .p(px(2.0))
            .rounded(px(8.0))
            .bg(theme.inset)
            .flex()
            .child(self.render_mode_segment(Mode::Tasks, mode, theme, cx))
            .child(self.render_mode_segment(Mode::Calendar, mode, theme, cx))
    }

    fn render_mode_segment(
        &self,
        segment: Mode,
        active_mode: Mode,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let selected = segment == active_mode;
        let focus = if segment == Mode::Calendar {
            self.nav_focuses[Destination::Calendar.index()].clone()
        } else {
            self.mode_tasks_focus.clone()
        };

        div()
            .id(SharedString::from(format!("mode-{}", segment.label())))
            .track_focus(&focus)
            .tab_index(0)
            .flex_1()
            .h(px(ROW_HEIGHT - 4.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .cursor_default()
            .text_size(px(12.5))
            .font_weight(gpui::FontWeight::MEDIUM)
            .when(selected, |row| {
                row.bg(theme.raised).text_color(theme.text)
            })
            .when(!selected, |row| {
                row.text_color(theme.text_secondary)
                    .hover(|el| el.bg(theme.overlay))
            })
            .focus_visible(|style| style.border_1().border_color(theme.accent))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.select_mode(segment, window, cx);
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.select_mode(segment, window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(icon(
                segment.icon_path(),
                14.0,
                if selected { theme.text } else { theme.text_secondary },
            ))
            .child(segment.label())
    }

    /// Live Inbox count for the badge. `0` while the first load is still in
    /// flight or the database is unavailable — an undercount is a better
    /// default than blocking the row on a fetch.
    fn inbox_count(&mut self, cx: &mut Context<Self>) -> usize {
        match self.read_view(crate::db::View::Inbox, cx) {
            crate::query::Query::Ready(tasks) => tasks.len(),
            crate::query::Query::Pending | crate::query::Query::Missing(_) => 0,
        }
    }

    fn select_mode(&mut self, mode: Mode, window: &mut Window, cx: &mut Context<Self>) {
        let destination = match mode {
            Mode::Calendar => Destination::Calendar,
            // Tasks mode has no destination of its own; landing on Inbox
            // matches "+ Capture" and the rest of the app's default entry
            // point into task review.
            Mode::Tasks => Destination::Inbox,
        };
        self.set_destination(destination, window, cx);
    }

    fn render_nav_row(
        &mut self,
        destination: Destination,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let index = destination.index();
        let focus = self.nav_focuses[index].clone();
        let selected = self.destination == destination;
        let badge = (destination == Destination::Inbox).then(|| self.inbox_count(cx));
        let row_icon_color = if selected {
            theme.text
        } else {
            theme.text_secondary
        };

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
            .gap(px(8.0))
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
                let Some(task_index) = destination.task_view_index() else {
                    // Settings isn't part of the arrow-navigable task list;
                    // it only supports enter/space, like the mode segments.
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.set_destination(destination, window, cx);
                        cx.stop_propagation();
                    }
                    return;
                };
                match event.keystroke.key.as_str() {
                    "enter" | "space" => {
                        this.set_destination(destination, window, cx);
                        cx.stop_propagation();
                    }
                    "down" => {
                        this.focus_and_select_task_view(task_index + 1, window, cx);
                        cx.stop_propagation();
                    }
                    "up" => {
                        let previous = if task_index == 0 {
                            Destination::TASK_VIEWS.len() - 1
                        } else {
                            task_index - 1
                        };
                        this.focus_and_select_task_view(previous, window, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon(destination.icon_path(), 14.0, row_icon_color))
                    .child(destination.label()),
            )
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
