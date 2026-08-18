//! TODO(milestone-0): rebuild command palette for Flow.
//!
//! The coding-agent command palette (task switcher, model-picking commands,
//! settings jump list, transcript search) was stripped for Milestone 0 —
//! see the milestone-0 wayfinder ticket for scope. This stub only keeps
//! `⌘K` wiring in `src/app.rs`, `src/app/render.rs`, `src/app/sidebar.rs`, and
//! `src/app/right_panel.rs` compiling; it renders nothing. Flow should
//! repopulate it with task-scoped commands (new task, jump to view).

use super::*;

/// No-op: the palette has no keybindings to register anymore.
pub fn init(_cx: &mut App) {}

pub(super) struct CommandPaletteUi {
    open: bool,
}

impl CommandPaletteUi {
    pub(super) fn new(_search: Entity<ComposerInput>) -> Self {
        Self { open: false }
    }

    pub(super) fn is_open(&self) -> bool {
        self.open
    }
}

impl Flow {
    pub(super) fn toggle_command_palette_action(
        &mut self,
        _: &ToggleCommandPalette,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // ponytail: palette has no content to show yet; keep the toggle a
        // no-op rather than opening an empty overlay.
    }

    pub(super) fn command_palette_query_edited(&mut self, _query: &str, _cx: &mut Context<Self>) {}

    pub(super) fn render_command_palette(
        &self,
        _window: &Window,
        _cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        None
    }
}
