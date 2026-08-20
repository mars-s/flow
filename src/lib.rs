#![recursion_limit = "256"]

rust_i18n::i18n!("locales", fallback = "en");

// rust-i18n expands locale data in a proc macro, which Cargo does not always
// discover as an input when only a YAML file changes. Keep explicit source
// dependencies so the watcher rebuilds the translation registry itself.
const _LOCALE_SOURCES: [&str; 3] = [
    include_str!("../locales/app.yml"),
    include_str!("../locales/zh-CN.yml"),
    include_str!("../locales/ja.yml"),
];

macro_rules! tr {
    ($key:expr) => {
        crate::i18n::translate($key)
    };
    ($key:expr, $($args:tt)*) => {
        rust_i18n::t!($key, $($args)*).into_owned()
    };
}

mod analytics;
mod app;
mod assets;
mod browser;
// Lives in the `flow-data` crate now, not this one — it never depended on
// GPUI, so it's reusable by any frontend
// (`wayfinder/tickets/migrate-to-tauri.md`). This re-export keeps every
// existing `crate::db::…` reference in the app working unchanged.
use flow_data::db;
mod debug_log;
mod input;
mod md;
// Same as `db` above — moved to `flow-data`, re-exported so every existing
// `crate::parse::…` reference keeps working unchanged.
use flow_data::parse;
mod platform;
mod query;
mod theme;
mod ui;
mod updater;

// Milestone 0 has no daemon, driver, or review-diff surface, and
// `cargo-workspace-cleanup` has correspondingly slimmed `flow-core` down to
// just locale plumbing and app identity — see the rebuild-shell-frame
// wayfinder ticket. Only re-export what the crate still has; `tr!` needs
// `i18n`, and the window/menu setup below needs `identity`.
pub use flow_core::{i18n, identity};

use gpui::{
    App, Application, Bounds, KeyBinding, Menu, MenuItem, TitlebarOptions,
    WindowBackgroundAppearance, WindowBounds, WindowOptions, actions, point, px, size,
};

use crate::app::Flow;
use crate::identity::{APP_ID, APP_NAME};
actions!(
    flow,
    [
        Quit,
        About,
        CloseWindow,
        NewTask,
        SpaceCapture,
        NewProject,
        OpenSettings,
        CheckForUpdates,
        ToggleSidebar,
        ToggleRightPanel,
        ToggleCommandPalette,
        ToggleFpsCounter,
        ToggleInspector,
        NavigateBack,
        NavigateForward,
        FocusComposer,
        ToggleModelPicker,
        ToggleUsagePanel,
        SaveFile,
        CancelTurn,
        CopySelection,
        OpenFind,
        OpenFindReplace,
        CloseFind,
        FindNext,
        FindPrevious,
        ToggleFindCaseSensitive,
        ToggleFindWholeWord,
        ToggleFindRegex,
        ReplaceAllMatches,
        BrowserBack,
        BrowserForward,
        BrowserReload,
        BrowserHardReload,
        BrowserStop,
        BrowserDevtools,
        FocusBrowserAddress,
        BrowserAddressCancel,
        WebviewCopy,
        WebviewCut,
        WebviewPaste,
        WebviewSelectAll
    ]
);

const DEFAULT_WINDOW_WIDTH: f32 = 1380.0;
const DEFAULT_WINDOW_HEIGHT: f32 = 880.0;
const MIN_WINDOW_WIDTH: f32 = 980.0;
const MIN_WINDOW_HEIGHT: f32 = 680.0;

/// Where the main window opens.
///
/// Flow restored the last-used frame from `crate::persistence`; that module
/// left with the daemon/workspace strip (see the rebuild-shell-frame
/// wayfinder ticket) and Flow has no persistence layer yet in Milestone 0,
/// so every launch centers the window. Restore-to-last-position can come
/// back once a Flow-owned settings/window-state store exists.
fn restored_window_placement(cx: &App) -> (WindowBounds, Option<gpui::DisplayId>) {
    (
        WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(DEFAULT_WINDOW_WIDTH), px(DEFAULT_WINDOW_HEIGHT)),
            cx,
        )),
        None,
    )
}

trait FlowApplicationExt {
    fn with_main_window_reopen(self) -> Self;
}

impl FlowApplicationExt for Application {
    fn with_main_window_reopen(self) -> Self {
        self.on_reopen(|cx| {
            if let Some(window) = cx.windows().into_iter().next() {
                window
                    .update(cx, |_, window, _| window.activate_window())
                    .ok();
            }
            cx.activate(true);
        });
        self
    }
}

pub fn run() {
    gpui_platform::application()
        .with_assets(crate::assets::Assets)
        .with_main_window_reopen()
        .run(move |cx: &mut App| {
            // Linux uses this for Wayland app_id/X11 WM_CLASS and notification
            // attribution. Other platforms also benefit from one stable
            // process identity.
            cx.set_app_identity(APP_ID, APP_NAME);
            crate::assets::register_fonts(cx).expect("failed to register bundled fonts");
            crate::input::init(cx);
            crate::ui::menu::init(cx);
            crate::app::init_settings_keys(cx);
            crate::app::init_command_palette(cx);
            crate::app::init_sidebar_keys(cx);
            crate::app::init_inspector(cx);
            crate::theme::init(cx);
            crate::platform::init_reduce_motion(cx);

            // Sparkle only runs from a bundled release build (or when forced
            // via FLOW_FORCE_UPDATER=1); everywhere else the menu item is
            // omitted along with the updater itself.
            let updater = crate::updater::Updater::init();
            let updater_available = updater.is_some();
            cx.set_global(crate::updater::UpdaterState(updater));
            cx.on_action(|_: &CheckForUpdates, cx| {
                if let Some(updater) = &cx.global::<crate::updater::UpdaterState>().0 {
                    updater.check_for_updates();
                }
            });
            cx.on_action(|_: &About, _| crate::platform::show_about_panel());

            cx.bind_keys([
                // `secondary` is Command on macOS and Control elsewhere.
                KeyBinding::new("secondary-q", Quit, None),
                KeyBinding::new("secondary-w", CloseWindow, None),
                KeyBinding::new("secondary-n", NewTask, None),
                KeyBinding::new("secondary-o", NewProject, None),
                KeyBinding::new("secondary-,", OpenSettings, None),
                KeyBinding::new("secondary-b", ToggleSidebar, None),
                KeyBinding::new("secondary-shift-b", ToggleRightPanel, None),
                KeyBinding::new("secondary-k", ToggleCommandPalette, None),
                KeyBinding::new("secondary-alt-shift-f", ToggleFpsCounter, None),
                // Hidden on purpose: no menu item, like Ghostty/Zed's own
                // element inspector. GPUI's inspector only compiles in debug
                // builds (`cfg(debug_assertions)`); the handler in app.rs
                // no-ops in release rather than omitting the binding, so the
                // keymap doesn't fork between build kinds.
                KeyBinding::new("secondary-alt-i", ToggleInspector, None),
                KeyBinding::new("secondary-[", NavigateBack, Some("Flow")),
                KeyBinding::new("secondary-]", NavigateForward, Some("Flow")),
                KeyBinding::new("secondary-l", FocusComposer, None),
                KeyBinding::new("secondary-/", ToggleModelPicker, None),
                KeyBinding::new("secondary-u", ToggleUsagePanel, None),
                KeyBinding::new("secondary-s", SaveFile, None),
                KeyBinding::new("escape", CancelTurn, Some("Flow")),
                // Bare space opens Capture from a task view, but never while
                // a composer has focus — `!ComposerInput` excludes Capture,
                // Schedule…, and the note field, all of which set that key
                // context (`input.rs`), so a literal typed space keeps
                // reaching the text field instead of being swallowed here.
                KeyBinding::new("space", SpaceCapture, Some("Flow && !ComposerInput")),
                KeyBinding::new("secondary-c", CopySelection, Some("Flow")),
                // Find and replace in the right panel's file editor, on the
                // conventional VS Code bindings. The primary shortcut + G cycles matches from
                // the editor without moving focus to the bar.
                KeyBinding::new("secondary-f", OpenFind, Some("Flow")),
                KeyBinding::new("secondary-alt-f", OpenFindReplace, Some("Flow")),
                KeyBinding::new("secondary-g", FindNext, Some("Flow")),
                KeyBinding::new("secondary-shift-g", FindPrevious, Some("Flow")),
                // Scoped to the editor pane: escape closes the bar there and
                // falls through to CancelTurn anywhere else.
                KeyBinding::new("escape", CloseFind, Some("FileEditorPane")),
                KeyBinding::new(
                    "secondary-alt-c",
                    ToggleFindCaseSensitive,
                    Some("FileEditorPane"),
                ),
                KeyBinding::new(
                    "secondary-alt-w",
                    ToggleFindWholeWord,
                    Some("FileEditorPane"),
                ),
                KeyBinding::new("secondary-alt-r", ToggleFindRegex, Some("FileEditorPane")),
                KeyBinding::new("shift-enter", FindPrevious, Some("FindBar")),
                KeyBinding::new("secondary-alt-enter", ReplaceAllMatches, Some("FindBar")),
                // Browser surface. Deeper than "Flow", so while focus is on the
                // page or its address bar the browser reads the platform's
                // conventional navigation shortcuts; the same keys elsewhere
                // keep their app meanings. The clipboard trio is rebound
                // because GPUI's window view claims key equivalents before
                // AppKit can walk the responder chain into the webview.
                KeyBinding::new("secondary-l", FocusBrowserAddress, Some("Browser")),
                KeyBinding::new("secondary-r", BrowserReload, Some("Browser")),
                KeyBinding::new("secondary-shift-r", BrowserHardReload, Some("Browser")),
                KeyBinding::new("secondary-[", BrowserBack, Some("Browser")),
                KeyBinding::new("secondary-]", BrowserForward, Some("Browser")),
                KeyBinding::new("escape", BrowserStop, Some("Browser")),
                KeyBinding::new("secondary-alt-i", BrowserDevtools, Some("Browser")),
                KeyBinding::new("secondary-c", WebviewCopy, Some("Browser")),
                KeyBinding::new("secondary-x", WebviewCut, Some("Browser")),
                KeyBinding::new("secondary-v", WebviewPaste, Some("Browser")),
                KeyBinding::new("secondary-a", WebviewSelectAll, Some("Browser")),
                KeyBinding::new("escape", BrowserAddressCancel, Some("BrowserAddress")),
            ]);

            cx.on_action(|_: &Quit, cx| cx.quit());

            // Unlike AppKit, Linux has no Dock activation path that can
            // restore a hidden last window. Follow Zed's GPUI precedent and
            // terminate when the final window closes.
            #[cfg(not(target_os = "macos"))]
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let (window_bounds, display_id) = restored_window_placement(cx);
            let window = cx
                .open_window(
                    WindowOptions {
                        titlebar: Some(TitlebarOptions {
                            title: Some(APP_NAME.into()),
                            appears_transparent: cfg!(target_os = "macos"),
                            traffic_light_position: cfg!(target_os = "macos")
                                .then(|| point(px(16.0), px(17.0))),
                        }),
                        // Flow moves its custom macOS titlebar explicitly. Keep
                        // the NSWindow movable so native controls and Window-menu
                        // tiling remain enabled.
                        is_movable: true,
                        app_owns_titlebar_drag: cfg!(target_os = "macos"),
                        window_background: if cfg!(target_os = "macos") {
                            WindowBackgroundAppearance::Blurred
                        } else {
                            WindowBackgroundAppearance::Opaque
                        },
                        app_id: Some(APP_ID.to_owned()),
                        // GPUI defaults to compositor/server decorations. If a
                        // Wayland compositor declines them, it reports the
                        // client fallback and Flow renders that frame itself.
                        #[cfg(target_os = "linux")]
                        icon: crate::platform::linux_app_icon(),
                        window_bounds: Some(window_bounds),
                        display_id,
                        window_min_size: Some(size(
                            px(MIN_WINDOW_WIDTH),
                            px(MIN_WINDOW_HEIGHT),
                        )),
                        ..Default::default()
                    },
                    move |window, cx| {
                        crate::platform::configure_main_window_close_behavior(window, cx);
                        let flow = Flow::new(window, cx);
                        let focus = flow.read(cx).initial_focus(cx);
                        window.focus(&focus, cx);
                        flow
                    },
                )
                .expect("failed to open Flow window");

            window
                .update(cx, |_, window, cx| {
                    crate::platform::configure_sidebar_material(
                        window,
                        crate::theme::Theme::current(cx).is_dark,
                    );
                    cx.activate(true);
                })
                .ok();

            set_app_menus(cx, updater_available);
        });
}

/// Rebuild the native menu bar in the active locale. GPUI menus own their
/// labels, so changing language must replace the model as well as redraw the
/// window.
pub(crate) fn set_app_menus(cx: &mut App, updater_available: bool) {
    cx.set_menus(vec![
        Menu {
            name: APP_NAME.into(),
            disabled: false,
            items: {
                let mut items = vec![MenuItem::action(tr!("menu.about", app = APP_NAME), About)];
                if updater_available {
                    items.push(MenuItem::action(
                        tr!("menu.check_for_updates"),
                        CheckForUpdates,
                    ));
                }
                items.push(MenuItem::separator());
                items.extend([
                    MenuItem::action(tr!("menu.settings"), OpenSettings),
                    MenuItem::separator(),
                    MenuItem::action(tr!("menu.quit", app = APP_NAME), Quit),
                ]);
                items
            },
        },
        Menu {
            name: tr!("menu.file").into(),
            disabled: false,
            items: vec![
                MenuItem::action(tr!("menu.new_task"), NewTask),
                MenuItem::action(tr!("menu.new_project"), NewProject),
            ],
        },
        Menu {
            name: tr!("menu.edit").into(),
            disabled: false,
            items: vec![
                MenuItem::action(tr!("menu.undo"), input::Undo),
                MenuItem::action(tr!("menu.redo"), input::Redo),
                MenuItem::separator(),
                MenuItem::action(tr!("menu.cut"), input::Cut),
                MenuItem::action(tr!("menu.copy"), input::Copy),
                MenuItem::action(tr!("menu.paste"), input::Paste),
                MenuItem::action(tr!("menu.select_all"), input::SelectAll),
            ],
        },
        Menu {
            name: tr!("menu.view").into(),
            disabled: false,
            items: vec![
                MenuItem::action(tr!("menu.command_palette"), ToggleCommandPalette),
                MenuItem::separator(),
                MenuItem::action(tr!("menu.toggle_sidebar"), ToggleSidebar),
                MenuItem::action(tr!("menu.toggle_right_panel"), ToggleRightPanel),
                MenuItem::action(tr!("menu.focus_composer"), FocusComposer),
                MenuItem::action(tr!("menu.toggle_model_picker"), ToggleModelPicker),
                MenuItem::action(tr!("menu.toggle_usage_panel"), ToggleUsagePanel),
            ],
        },
        Menu {
            name: tr!("menu.window").into(),
            disabled: false,
            items: vec![
                MenuItem::action(tr!("menu.toggle_fps_counter"), ToggleFpsCounter),
                MenuItem::action(tr!("menu.close_window"), CloseWindow),
            ],
        },
    ]);
}
