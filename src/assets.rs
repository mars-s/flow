use std::borrow::Cow;

use anyhow::Result;
use gpui::{App, AssetSource, SharedString};

/// Icons embedded in the binary so the app stays a single artifact.
pub struct Assets;

macro_rules! icons {
    ($($name:literal),+ $(,)?) => {
        &[$((
            concat!("icons/", $name, ".svg"),
            include_bytes!(concat!("../assets/icons/", $name, ".svg")).as_slice(),
        )),+]
    };
}

const ICONS: &[(&str, &[u8])] = icons![
    "alert",
    "appearance",
    "arrow-down",
    "arrow-left",
    "arrow-right",
    "arrow-up",
    "arrow-up-right",
    "block",
    "bot",
    "case-sensitive",
    "chart-column",
    "check",
    "changes",
    "cloud-upload",
    "chevron-down",
    "chevron-right",
    "chevron-up",
    "chevrons-up-down",
    "command",
    "compose",
    "copy",
    "corner-down-right",
    "cursor-spark",
    "download",
    "ellipsis",
    "eye",
    "eye-off",
    "external-link",
    "file",
    "folder",
    "folder-new",
    "file-bottom-left-arrow",
    "fork",
    "globe",
    "github",
    "hexagon",
    "info",
    "laptop",
    "list",
    "loader-circle",
    "lock",
    "lock-open",
    "package",
    "panel-left",
    "panel-right",
    "pencil",
    "plus",
    "queue",
    "regex",
    "replace",
    "replace-all",
    "rewind",
    "rotate-cw",
    "search",
    "server",
    "settings",
    "slash",
    "sparkle",
    "star",
    "star-filled",
    "stop",
    "stop-filled",
    "trash",
    "whole-word",
    "wrench",
    "window-maximize",
    "window-minimize",
    "window-restore",
    "x",
    "zap",
];

const TEXT_FONTS: &[&[u8]] = &[
    include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Italic.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-BoldItalic.ttf"),
];

/// Symbols-only icon face resolved via CoreText cascade (`FontFallbacks`),
/// never as a primary GPUI family; see `register_fonts_with_coretext`.
const SYMBOLS_FONT: &[u8] = include_bytes!("../assets/fonts/SymbolsNerdFontMono-Regular.ttf");

/// Family name of [`SYMBOLS_FONT`] for `FontFallbacks` lists.
pub const SYMBOLS_FONT_FAMILY: &str = "Symbols Nerd Font Mono";

pub fn register_fonts(cx: &App) -> Result<()> {
    cx.text_system().add_fonts(
        TEXT_FONTS
            .iter()
            .map(|font| Cow::Borrowed(*font))
            .collect::<Vec<_>>(),
    )?;
    crate::platform::register_fonts_with_coretext(&[SYMBOLS_FONT])
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .filter(|(name, _)| name.starts_with(path))
            .map(|(name, _)| SharedString::from(*name))
            .collect())
    }
}
