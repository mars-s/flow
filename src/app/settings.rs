//! Flow has no user-facing settings in Milestone 0.
//!
//! The coding-agent configuration UI (see the milestone-0 wayfinder ticket
//! for scope) was stripped — none of it is a Flow product surface. This is
//! a placeholder for `rebuild-shell-frame` to register real Flow settings
//! content in later; it intentionally has none of its own.

use super::*;

/// No-op: there is no settings search field to bind keys for anymore.
pub fn init(_cx: &mut App) {}

impl Flow {
    pub(super) fn render_settings(&self, _window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::current(cx);
        div().size_full().bg(theme.canvas).into_any_element()
    }

    pub(super) fn apply_daemon_exposure_fields(&mut self, _cx: &mut Context<Self>) {}

    pub(super) fn request_computer_permissions(&mut self, _prompt: bool, _cx: &mut Context<Self>) {}
}
