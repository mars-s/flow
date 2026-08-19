//! The Calendar tab (PRD §6.5, `wayfinder/tickets/eventkit-calendar-tab.md`).
//! Day/Week only for now — Month/Year are that ticket's own next slice.
//!
//! **Deliberate, disclosed simplification**: this is an agenda-per-day
//! layout (each visible day's events listed top-to-bottom, sorted
//! all-day-then-by-start-time), not Apple Calendar's pixel-accurate hour
//! grid with true time-of-day vertical positioning and overlap resolution —
//! that's a separate absolute-position layout algorithm the ticket doesn't
//! scope into this first pass. A multi-day event is filed under its start
//! date's column only, not spanned across every day it covers.

use gpui::{
    AnyElement, Context, FocusHandle, Hsla, IntoElement, KeyDownEvent, ParentElement, Rgba,
    Styled, div, prelude::*, px,
};

use super::Flow;
use super::sidebar::Destination;
use crate::app::CalendarViewMode;
use crate::platform::{CalendarAuth, CalendarEvent, CalendarInfo};
use crate::theme::Theme;

impl Flow {
    pub(super) fn render_calendar_tab(&mut self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        if self.calendar_auth != CalendarAuth::Granted {
            return render_calendar_not_connected(theme, cx);
        }

        let (start, end) = self.calendar_visible_range();
        let days: Vec<chrono::NaiveDate> = {
            let mut day = start;
            let mut days = Vec::new();
            while day <= end {
                days.push(day);
                day += chrono::Duration::days(1);
            }
            days
        };
        let hidden = self.calendar_hidden_ids.clone();
        let events = self.calendar_range_events.clone().unwrap_or_default();
        let calendars = self.calendar_list.clone();

        div()
            .size_full()
            .bg(theme.canvas)
            .flex()
            .child(self.render_calendar_sidebar(calendars, theme, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(self.render_calendar_header(theme, cx))
                    .child(render_calendar_body(days, &events, &hidden, theme)),
            )
            .into_any_element()
    }

    fn render_calendar_sidebar(
        &mut self,
        calendars: Option<std::sync::Arc<Vec<CalendarInfo>>>,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(calendars) = calendars else {
            return div().flex_none().w(px(200.0)).h_full().bg(theme.raised).into_any_element();
        };

        // Grouped by account (`source_title`), matching Apple Calendar's own
        // sidebar sectioning — the reference screenshot's own grouping.
        let mut groups: Vec<(String, Vec<CalendarInfo>)> = Vec::new();
        for calendar in calendars.iter() {
            match groups.iter_mut().find(|(source, _)| source == &calendar.source_title) {
                Some((_, list)) => list.push(calendar.clone()),
                None => groups.push((calendar.source_title.clone(), vec![calendar.clone()])),
            }
        }

        div()
            .id("calendar-sidebar")
            .flex_none()
            .w(px(200.0))
            .h_full()
            .overflow_y_scroll()
            .bg(theme.raised)
            .border_r_1()
            .border_color(theme.border)
            .p(px(12.0))
            .flex()
            .flex_col()
            .gap(px(14.0))
            .children(groups.into_iter().map(|(source, calendars)| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(10.5))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.text_ghost)
                            .child(if source.is_empty() { "Other".to_string() } else { source }),
                    )
                    .children(calendars.into_iter().map(|calendar| {
                        self.render_calendar_toggle_row(calendar, theme, cx)
                    }))
            }))
            .into_any_element()
    }

    fn render_calendar_toggle_row(
        &mut self,
        calendar: CalendarInfo,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let hidden = self.calendar_hidden_ids.contains(&calendar.id);
        let focus = self.calendar_row_focus(&calendar.id, cx);
        let id_for_click = calendar.id.clone();
        let id_for_key = calendar.id.clone();
        let (r, g, b, a) = calendar.color;
        let color: Hsla = Rgba { r, g, b, a }.into();

        div()
            .id(gpui::SharedString::from(format!("calendar-toggle-{}", calendar.id)))
            .track_focus(&focus)
            .tab_index(0)
            .h(px(24.0))
            .px(px(4.0))
            .rounded(px(5.0))
            .cursor_pointer()
            .flex()
            .items_center()
            .gap(px(8.0))
            .hover(|el| el.bg(theme.overlay))
            .focus_visible(|style| style.border_1().border_color(theme.accent))
            .when(hidden, |row| row.opacity(0.4))
            .on_click(cx.listener(move |flow, _, _, cx| {
                flow.toggle_calendar_visibility(id_for_click.clone(), cx);
            }))
            .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, _, cx| {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    flow.toggle_calendar_visibility(id_for_key.clone(), cx);
                    cx.stop_propagation();
                }
            }))
            .child(div().flex_none().size(px(8.0)).rounded_full().bg(color))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.5))
                    .text_color(theme.text)
                    .child(calendar.title),
            )
            .into_any_element()
    }

    fn render_calendar_header(&mut self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        let mode = self.calendar_view_mode;
        let (start, end) = self.calendar_visible_range();
        let label = if mode == CalendarViewMode::Day {
            start.format("%A, %B %-d").to_string()
        } else if start.format("%b").to_string() == end.format("%b").to_string() {
            format!("{} {}\u{2013}{}, {}", start.format("%b"), start.format("%-d"), end.format("%-d"), end.format("%Y"))
        } else {
            format!("{} \u{2013} {}", start.format("%b %-d"), end.format("%b %-d, %Y"))
        };

        let day_focus = self.calendar_day_focus.clone();
        let week_focus = self.calendar_week_focus.clone();
        let today_focus = self.calendar_today_focus.clone();
        let prev_focus = self.calendar_prev_focus.clone();
        let next_focus = self.calendar_next_focus.clone();

        div()
            .flex_none()
            .h(px(48.0))
            .px(px(16.0))
            .flex()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .text_size(px(14.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.text)
                    .child(label),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(calendar_nav_button("‹", "calendar-prev", prev_focus, theme, cx, |flow, cx| {
                        let delta = if flow.calendar_view_mode == CalendarViewMode::Day { -1 } else { -7 };
                        flow.navigate_calendar(delta, cx);
                    }))
                    .child(calendar_text_button("Today", "calendar-today", today_focus, theme, cx, |flow, cx| {
                        flow.navigate_calendar(0, cx);
                    }))
                    .child(calendar_nav_button("›", "calendar-next", next_focus, theme, cx, |flow, cx| {
                        let delta = if flow.calendar_view_mode == CalendarViewMode::Day { 1 } else { 7 };
                        flow.navigate_calendar(delta, cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .rounded(px(6.0))
                            .bg(theme.overlay)
                            .p(px(2.0))
                            .child(calendar_mode_button(
                                "Day",
                                mode == CalendarViewMode::Day,
                                day_focus,
                                theme,
                                cx,
                                CalendarViewMode::Day,
                            ))
                            .child(calendar_mode_button(
                                "Week",
                                mode == CalendarViewMode::Week,
                                week_focus,
                                theme,
                                cx,
                                CalendarViewMode::Week,
                            )),
                    ),
            )
            .into_any_element()
    }
}

fn calendar_nav_button(
    label: &'static str,
    id: &'static str,
    focus: FocusHandle,
    theme: Theme,
    cx: &mut Context<Flow>,
    on_click: impl Fn(&mut Flow, &mut Context<Flow>) + 'static,
) -> AnyElement {
    let on_click = std::rc::Rc::new(on_click);
    let for_click = on_click.clone();
    div()
        .id(id)
        .track_focus(&focus)
        .tab_index(0)
        .size(px(24.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(14.0))
        .text_color(theme.text_secondary)
        .hover(|el| el.bg(theme.overlay))
        .focus_visible(|style| style.border_1().border_color(theme.accent))
        .on_click(cx.listener(move |flow, _, _, cx| for_click(flow, cx)))
        .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, _, cx| {
            if event.keystroke.modifiers.modified() {
                return;
            }
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                on_click(flow, cx);
                cx.stop_propagation();
            }
        }))
        .child(label)
        .into_any_element()
}

fn calendar_text_button(
    label: &'static str,
    id: &'static str,
    focus: FocusHandle,
    theme: Theme,
    cx: &mut Context<Flow>,
    on_click: impl Fn(&mut Flow, &mut Context<Flow>) + 'static,
) -> AnyElement {
    let on_click = std::rc::Rc::new(on_click);
    let for_click = on_click.clone();
    div()
        .id(id)
        .track_focus(&focus)
        .tab_index(0)
        .h(px(24.0))
        .px(px(10.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .flex()
        .items_center()
        .text_size(px(12.5))
        .text_color(theme.text_secondary)
        .border_1()
        .border_color(theme.border_strong)
        .hover(|el| el.border_color(theme.accent))
        .focus_visible(|style| style.border_color(theme.accent))
        .on_click(cx.listener(move |flow, _, _, cx| for_click(flow, cx)))
        .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, _, cx| {
            if event.keystroke.modifiers.modified() {
                return;
            }
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                on_click(flow, cx);
                cx.stop_propagation();
            }
        }))
        .child(label)
        .into_any_element()
}

fn calendar_mode_button(
    label: &'static str,
    active: bool,
    focus: FocusHandle,
    theme: Theme,
    cx: &mut Context<Flow>,
    mode: CalendarViewMode,
) -> AnyElement {
    div()
        .id(gpui::SharedString::from(format!("calendar-mode-{label}")))
        .track_focus(&focus)
        .tab_index(0)
        .h(px(20.0))
        .px(px(10.0))
        .rounded(px(4.0))
        .cursor_pointer()
        .flex()
        .items_center()
        .text_size(px(12.0))
        .font_weight(gpui::FontWeight::MEDIUM)
        .when(active, |el| el.bg(theme.raised).text_color(theme.text))
        .when(!active, |el| el.text_color(theme.text_secondary))
        .focus_visible(|style| style.border_1().border_color(theme.accent))
        .on_click(cx.listener(move |flow, _, _, cx| flow.set_calendar_view_mode(mode, cx)))
        .on_key_down(cx.listener(move |flow, event: &KeyDownEvent, _, cx| {
            if event.keystroke.modifiers.modified() {
                return;
            }
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                flow.set_calendar_view_mode(mode, cx);
                cx.stop_propagation();
            }
        }))
        .child(label)
        .into_any_element()
}

fn render_calendar_body(
    days: Vec<chrono::NaiveDate>,
    events: &[CalendarEvent],
    hidden: &std::collections::HashSet<String>,
    theme: Theme,
) -> AnyElement {
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .children(days.into_iter().map(|day| {
            let mut day_events: Vec<&CalendarEvent> = events
                .iter()
                .filter(|event| !hidden.contains(&event.calendar_id))
                .filter(|event| event.start.date_naive() == day)
                .collect();
            day_events.sort_by_key(|event| (!event.all_day, event.start));
            render_calendar_day_column(day, day_events, theme)
        }))
        .into_any_element()
}

fn render_calendar_day_column(day: chrono::NaiveDate, events: Vec<&CalendarEvent>, theme: Theme) -> AnyElement {
    let is_today = day == chrono::Local::now().date_naive();
    div()
        .flex_1()
        .min_w_0()
        .h_full()
        .border_r_1()
        .border_color(theme.border)
        .flex()
        .flex_col()
        .child(
            div()
                .flex_none()
                .h(px(36.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .border_b_1()
                .border_color(theme.border)
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.text_secondary)
                        .child(day.format("%a").to_string()),
                )
                .child(
                    div()
                        .when(is_today, |el| {
                            el.rounded_full().bg(theme.accent).text_color(theme.canvas)
                        })
                        .when(!is_today, |el| el.text_color(theme.text))
                        .px(px(6.0))
                        .text_size(px(12.5))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(day.format("%-d").to_string()),
                ),
        )
        .child(
            div()
                .id(gpui::SharedString::from(format!("calendar-day-{day}")))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .p(px(6.0))
                .flex()
                .flex_col()
                .gap(px(4.0))
                .when(events.is_empty(), |col| {
                    col.child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.text_ghost)
                            .child(""),
                    )
                })
                .children(events.into_iter().map(|event| render_calendar_event_card(event, theme))),
        )
        .into_any_element()
}

fn render_calendar_event_card(event: &CalendarEvent, theme: Theme) -> AnyElement {
    let (r, g, b, a) = event.color;
    let color: Hsla = Rgba { r, g, b, a }.into();
    div()
        .id(gpui::SharedString::from(format!("calendar-event-{}", event.id)))
        .p(px(6.0))
        .rounded(px(5.0))
        .bg(theme.raised)
        .border_l_2()
        .border_color(color)
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .text_size(px(11.5))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.text)
                .truncate()
                .child(event.title.clone()),
        )
        .when(!event.all_day, |card| {
            card.child(
                div()
                    .text_size(px(10.5))
                    .text_color(theme.text_secondary)
                    .child(format!(
                        "{} \u{2013} {}",
                        event.start.format("%-I:%M %p"),
                        event.end.format("%-I:%M %p")
                    )),
            )
        })
        .into_any_element()
}

fn render_calendar_not_connected(theme: Theme, cx: &mut Context<Flow>) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(8.0))
        .bg(theme.canvas)
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text)
                .child("No calendar connected"),
        )
        .child(
            div()
                .max_w(px(320.0))
                .text_center()
                .text_size(px(12.0))
                .line_height(px(18.0))
                .text_color(theme.text_ghost)
                .child("Connect your macOS Calendar in Settings to see your events here."),
        )
        .child(
            div()
                .id("calendar-tab-go-to-settings")
                .mt(px(6.0))
                .px(px(12.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .bg(theme.accent)
                .text_size(px(12.5))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.canvas)
                .cursor_pointer()
                .hover(|el| el.opacity(0.9))
                .child("Open Settings")
                .on_click(cx.listener(|flow, _, window, cx| {
                    flow.set_destination(Destination::Settings, window, cx);
                })),
        )
        .into_any_element()
}
