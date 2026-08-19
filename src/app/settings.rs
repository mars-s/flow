//! Flow's Settings destination: currently just the calendar connection
//! (PRD §6.5, revised 2026-08-19 — EventKit, not Google OAuth).
//!
//! The coding-agent configuration UI (see the milestone-0 wayfinder ticket
//! for scope) was stripped — none of it is a Flow product surface. This
//! grew its first real content alongside the EventKit calendar work; more
//! Flow settings (appearance, account) land here as they're built.

use gpui::{App, AnyElement, Context, IntoElement, ParentElement, Styled, div, prelude::*, px};

use super::Flow;
use crate::platform::CalendarAuth;
use crate::theme::Theme;

/// No-op: there is no settings search field to bind keys for anymore.
pub fn init(_cx: &mut App) {}

impl Flow {
    pub(super) fn render_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::current(cx);
        div()
            .size_full()
            .bg(theme.canvas)
            .p(px(24.0))
            .child(self.render_calendar_section(theme, cx))
            .into_any_element()
    }

    fn render_calendar_section(&self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        let auth = self.calendar_auth;
        let connecting = self.calendar_connecting;

        div()
            .max_w(px(480.0))
            .p(px(16.0))
            .rounded(px(10.0))
            .bg(theme.raised)
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.text)
                    .child("Calendar"),
            )
            .child(
                // PRD §6.5: "a clear read-only disclosure" — stated
                // unconditionally, not just before connecting, since the
                // access it describes stays read-only for as long as it's
                // granted, not only at the moment of asking for it.
                div()
                    .text_size(px(12.0))
                    .line_height(px(18.0))
                    .text_color(theme.text_secondary)
                    .child(
                        "Flow reads your local macOS Calendar app — whatever \
                         calendars you already have set up (iCloud, Google, \
                         Exchange, and more) — to show today's events \
                         alongside your tasks. Read-only: Flow never creates, \
                         edits, or deletes anything in your calendar.",
                    ),
            )
            .child(match auth {
                CalendarAuth::Granted => div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .flex_none()
                            .size(px(6.0))
                            .rounded_full()
                            .bg(theme.accent),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.text)
                            .child("Connected"),
                    )
                    .into_any_element(),
                CalendarAuth::Denied => div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.text_secondary)
                            .child(
                                "Calendar access was denied. Flow can't ask \
                                 again — grant it from System Settings, then \
                                 relaunch Flow.",
                            ),
                    )
                    .child(open_system_settings_button(
                        theme,
                        self.settings_open_privacy_focus.clone(),
                        cx,
                    ))
                    .into_any_element(),
                CalendarAuth::Unavailable => div()
                    .text_size(px(12.0))
                    .text_color(theme.text_ghost)
                    .child("Calendar access isn't available on this platform yet.")
                    .into_any_element(),
                CalendarAuth::NotDetermined => connect_calendar_button(
                    theme,
                    connecting,
                    self.settings_connect_calendar_focus.clone(),
                    cx,
                )
                .into_any_element(),
            })
            .into_any_element()
    }
}

fn connect_calendar_button(
    theme: Theme,
    connecting: bool,
    focus: gpui::FocusHandle,
    cx: &mut Context<Flow>,
) -> impl IntoElement {
    div()
        .id("connect-calendar")
        .flex_none()
        // PRD §7's 28px minimum hit target — explicit rather than relying
        // on padding-plus-line-height to clear it by chance.
        .h(px(28.0))
        .px(px(12.0))
        .rounded(px(6.0))
        .bg(theme.accent)
        .flex()
        .items_center()
        .text_size(px(12.5))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(theme.canvas)
        .when(!connecting, |button| {
            button
                .cursor_pointer()
                .hover(|el| el.opacity(0.9))
                .track_focus(&focus)
                .tab_index(0)
                .focus_visible(|style| style.border_2().border_color(theme.text))
                .on_click(cx.listener(|flow, _, _, cx| flow.connect_calendar(cx)))
                .on_key_down(cx.listener(|flow, event: &gpui::KeyDownEvent, _, cx| {
                    if event.keystroke.modifiers.modified() {
                        return;
                    }
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        flow.connect_calendar(cx);
                        cx.stop_propagation();
                    }
                }))
        })
        .when(connecting, |button| button.opacity(0.6))
        .child(if connecting { "Connecting…" } else { "Connect Calendar" })
}

/// PRD §6.5: "a way to reach macOS's own System Settings → Privacy &
/// Security → Calendars pane to revoke it — Flow cannot revoke a system
/// permission grant programmatically." `platform::open_calendar_privacy_pane`
/// is the macOS/non-macOS split for that deep link, matching this file's own
/// `calendar_authorization_status`-shaped pattern.
fn open_system_settings_button(
    theme: Theme,
    focus: gpui::FocusHandle,
    cx: &mut Context<Flow>,
) -> impl IntoElement {
    div()
        .id("open-calendar-privacy-settings")
        .flex_none()
        // PRD §7's 28px minimum hit target.
        .h(px(28.0))
        .px(px(12.0))
        .rounded(px(6.0))
        .flex()
        .items_center()
        .border_1()
        .border_color(theme.border_strong)
        .cursor_pointer()
        .hover(|el| el.border_color(theme.accent))
        .track_focus(&focus)
        .tab_index(0)
        .focus_visible(|style| style.border_color(theme.accent))
        .text_size(px(12.5))
        .text_color(theme.text)
        .child("Open System Settings")
        .on_click(cx.listener(|_flow, _, _, _cx| crate::platform::open_calendar_privacy_pane()))
        .on_key_down(cx.listener(|_flow, event: &gpui::KeyDownEvent, _, cx| {
            if event.keystroke.modifiers.modified() {
                return;
            }
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                crate::platform::open_calendar_privacy_pane();
                cx.stop_propagation();
            }
        }))
}
