use eframe::egui::Color32;
use std::time::Duration;

// ── Constants ────────────────────────────────────────────────────────────────

pub const POLL_EVERY: Duration = Duration::from_millis(900);
pub const SYSINFO_INTERVAL: Duration = Duration::from_secs(2);
pub const MENU_BAR_HEIGHT: f32 = 34.0;
pub const DOCK_HEIGHT: f32 = 96.0;
pub const DOCK_ICON_BASE: f32 = 48.0;
pub const DOCK_ICON_MAX_SCALE: f32 = 1.35;
pub const DOCK_EFFECT_DIST: f32 = 120.0;
pub const WINDOW_COUNT: usize = 18;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppScreenState {
    Setup,
    Login,
    Locked,
    Desktop,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppPhase {
    Booting,
    Ready,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DockPosition {
    Bottom,
    Left,
    Right,
}

impl DockPosition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "left" => Self::Left,
            "right" => Self::Right,
            _ => Self::Bottom,
        }
    }
}

// ── Window types ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum WindowKind {
    Overview = 0,
    Terminal = 1,
    FileManager = 2,
    Controls = 3,
    Messages = 4,
    Browser = 5,
    Calculator = 6,
    Notes = 7,
    MusicPlayer = 8,
    Photos = 9,
    Calendar = 10,
    TextEditor = 11,
    Settings = 12,
    ProcessManager = 13,
    Trash = 14,
    NetworkDiagnostics = 15,
    Dictionary = 16,
    Console = 17,
}

impl WindowKind {
    pub fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(Self::Overview),
            1 => Some(Self::Terminal),
            2 => Some(Self::FileManager),
            3 => Some(Self::Controls),
            4 => Some(Self::Messages),
            5 => Some(Self::Browser),
            6 => Some(Self::Calculator),
            7 => Some(Self::Notes),
            8 => Some(Self::MusicPlayer),
            9 => Some(Self::Photos),
            10 => Some(Self::Calendar),
            11 => Some(Self::TextEditor),
            12 => Some(Self::Settings),
            13 => Some(Self::ProcessManager),
            14 => Some(Self::Trash),
            15 => Some(Self::NetworkDiagnostics),
            16 => Some(Self::Dictionary),
            17 => Some(Self::Console),
            _ => None,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Overview => "System Overview",
            Self::Terminal => "Terminal",
            Self::FileManager => "Files",
            Self::Controls => "Quick Controls",
            Self::Messages => "Messages",
            Self::Browser => "Browser",
            Self::Calculator => "Calculator",
            Self::Notes => "Notes",
            Self::MusicPlayer => "Music",
            Self::Photos => "Photos",
            Self::Calendar => "Calendar",
            Self::TextEditor => "TextEdit",
            Self::Settings => "Settings",
            Self::ProcessManager => "Activity Monitor",
            Self::Trash => "Trash",
            Self::NetworkDiagnostics => "Network Diagnostics",
            Self::Dictionary => "Dictionary",
            Self::Console => "Console",
        }
    }
}

// ── Dock icon types ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DockIcon {
    Files,
    Terminal,
    Browser,
    Overview,
    Launchpad,
    Messages,
    Notes,
    Calendar,
    Music,
    Photos,
    Calculator,
    Settings,
    Store,
    Separator,
    Controls,
    Info,
    Trash,
}

impl DockIcon {
    pub fn all() -> &'static [Self] {
        &[
            Self::Files,
            Self::Terminal,
            Self::Browser,
            Self::Messages,
            Self::Overview,
            Self::Launchpad,
            Self::Notes,
            Self::Calendar,
            Self::Music,
            Self::Photos,
            Self::Calculator,
            Self::Settings,
            Self::Store,
            Self::Separator,
            Self::Controls,
            Self::Info,
            Self::Trash,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Files => "Files",
            Self::Terminal => "Terminal",
            Self::Browser => "Browser",
            Self::Overview => "System Overview",
            Self::Launchpad => "Launchpad",
            Self::Messages => "Messages",
            Self::Notes => "Notes",
            Self::Calendar => "Calendar",
            Self::Music => "Music",
            Self::Photos => "Photos",
            Self::Calculator => "Calculator",
            Self::Settings => "Settings",
            Self::Store => "App Store",
            Self::Separator => "",
            Self::Controls => "Quick Controls",
            Self::Info => "System Info",
            Self::Trash => "Trash",
        }
    }

    pub fn bg_color(self) -> Color32 {
        match self {
            Self::Files => Color32::from_rgb(0, 122, 255),
            Self::Terminal => Color32::from_rgb(30, 30, 46),
            Self::Browser => Color32::from_rgb(0, 180, 216),
            Self::Overview => Color32::from_rgb(52, 199, 89),
            Self::Launchpad => Color32::from_rgb(88, 86, 214),
            Self::Messages => Color32::from_rgb(76, 217, 100),
            Self::Notes => Color32::from_rgb(255, 214, 10),
            Self::Calendar => Color32::WHITE,
            Self::Music => Color32::from_rgb(255, 55, 95),
            Self::Photos => Color32::from_rgb(255, 107, 107),
            Self::Calculator => Color32::from_rgb(255, 149, 0),
            Self::Settings => Color32::from_rgb(142, 142, 147),
            Self::Store => Color32::from_rgb(0, 122, 255),
            Self::Separator => Color32::TRANSPARENT,
            Self::Controls => Color32::from_rgb(88, 86, 214),
            Self::Info => Color32::from_rgb(175, 82, 222),
            Self::Trash => Color32::from_rgb(120, 120, 128),
        }
    }

    pub fn window_kind(self) -> Option<WindowKind> {
        match self {
            Self::Files => Some(WindowKind::FileManager),
            Self::Terminal => Some(WindowKind::Terminal),
            Self::Overview => Some(WindowKind::Overview),
            Self::Controls => Some(WindowKind::Controls),
            Self::Messages => Some(WindowKind::Messages),
            Self::Browser => Some(WindowKind::Browser),
            Self::Calculator => Some(WindowKind::Calculator),
            Self::Notes => Some(WindowKind::Notes),
            Self::Music => Some(WindowKind::MusicPlayer),
            Self::Photos => Some(WindowKind::Photos),
            Self::Calendar => Some(WindowKind::Calendar),
            Self::Settings => Some(WindowKind::Settings),
            Self::Info => Some(WindowKind::ProcessManager),
            Self::Trash => Some(WindowKind::Trash),
            _ => None,
        }
    }

    pub fn is_separator(self) -> bool {
        matches!(self, Self::Separator)
    }
}

// ── Menu ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuDropdown {
    File,
    Edit,
    View,
    Window,
    Help,
}

impl MenuDropdown {
    pub fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Edit => "Edit",
            Self::View => "View",
            Self::Window => "Window",
            Self::Help => "Help",
        }
    }

    pub fn items(self) -> &'static [&'static str] {
        match self {
            Self::File => &[
                "New Window",
                "Open...",
                "Save",
                "---",
                "Close Window  Ctrl+W",
                "---",
                "Quit  Ctrl+Q",
            ],
            Self::Edit => &[
                "Undo  Ctrl+Z",
                "Redo  Ctrl+Y",
                "---",
                "Cut  Ctrl+X",
                "Copy  Ctrl+C",
                "Paste  Ctrl+V",
                "---",
                "Select All  Ctrl+A",
            ],
            Self::View => &[
                "Enter Full Screen",
                "---",
                "Show Sidebar",
                "Show Path Bar",
                "Show Status Bar",
                "Show Preview",
            ],
            Self::Window => &[
                "Minimize  Ctrl+M",
                "Zoom",
                "Start Screen Saver",
                "---",
                "Tile Left",
                "Tile Right",
                "---",
                "Tile Left Third",
                "Tile Center Third",
                "Tile Right Third",
                "---",
                "Bring All to Front",
            ],
            Self::Help => &["AuroraOS Help", "---", "About AuroraOS"],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuAction {
    Quit,
    CloseWindow,
    Minimize,
    Maximize,
    TileLeft,
    TileRight,
    TileLeftThird,
    TileCenterThird,
    TileRightThird,
    BringAllToFront,
    Copy,
    Cut,
    Paste,
    SelectAll,
    Undo,
    Redo,
    Save,
    StartScreenSaver,
    ToggleFullScreen,
    ToggleSidebar,
    TogglePathBar,
    ToggleStatusBar,
    TogglePreview,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Constants ────────────────────────────────────────────────────

    #[test]
    fn constants_are_reasonable() {
        assert!(MENU_BAR_HEIGHT > 0.0);
        assert!(DOCK_HEIGHT > 0.0);
        assert!(DOCK_ICON_BASE > 0.0);
        assert!(DOCK_ICON_MAX_SCALE > 1.0);
        assert!(DOCK_EFFECT_DIST > 0.0);
        assert!(POLL_EVERY.as_millis() > 0);
        assert!(SYSINFO_INTERVAL.as_secs() > 0);
    }

    #[test]
    fn dock_position_string_roundtrip() {
        assert_eq!(DockPosition::from_str("bottom"), DockPosition::Bottom);
        assert_eq!(DockPosition::from_str("left"), DockPosition::Left);
        assert_eq!(DockPosition::from_str("right"), DockPosition::Right);
        assert_eq!(DockPosition::Right.as_str(), "right");
    }

    #[test]
    fn app_screen_state_variants_exist() {
        assert!(matches!(AppScreenState::Setup, AppScreenState::Setup));
        assert!(matches!(AppScreenState::Login, AppScreenState::Login));
        assert!(matches!(AppScreenState::Locked, AppScreenState::Locked));
        assert!(matches!(AppScreenState::Desktop, AppScreenState::Desktop));
    }

    #[test]
    fn app_phase_variants_exist() {
        assert!(matches!(AppPhase::Booting, AppPhase::Booting));
        assert!(matches!(AppPhase::Ready, AppPhase::Ready));
    }

    // ── WindowKind ───────────────────────────────────────────────────

    #[test]
    fn window_kind_titles_nonempty() {
        let kinds = [
            WindowKind::Overview,
            WindowKind::Terminal,
            WindowKind::FileManager,
            WindowKind::Controls,
            WindowKind::Messages,
            WindowKind::Browser,
            WindowKind::Calculator,
            WindowKind::Notes,
            WindowKind::MusicPlayer,
            WindowKind::Photos,
            WindowKind::Calendar,
            WindowKind::TextEditor,
            WindowKind::Settings,
            WindowKind::ProcessManager,
            WindowKind::Trash,
            WindowKind::NetworkDiagnostics,
            WindowKind::Dictionary,
            WindowKind::Console,
        ];
        for kind in kinds {
            assert!(!kind.title().is_empty(), "{kind:?} has empty title");
        }
    }

    #[test]
    fn window_count_matches_enum() {
        assert_eq!(WINDOW_COUNT, 18);
        assert_eq!(WindowKind::Console as usize, WINDOW_COUNT - 1);
    }

    #[test]
    fn window_kind_indices_are_contiguous() {
        assert_eq!(WindowKind::Overview as usize, 0);
        assert_eq!(WindowKind::Terminal as usize, 1);
        assert_eq!(WindowKind::TextEditor as usize, 11);
        assert_eq!(WindowKind::Settings as usize, 12);
        assert_eq!(WindowKind::ProcessManager as usize, 13);
        assert_eq!(WindowKind::Trash as usize, 14);
        assert_eq!(WindowKind::NetworkDiagnostics as usize, 15);
        assert_eq!(WindowKind::Dictionary as usize, 16);
        assert_eq!(WindowKind::Console as usize, 17);
    }

    #[test]
    fn from_index_roundtrip() {
        for i in 0..WINDOW_COUNT {
            let kind = WindowKind::from_index(i);
            assert!(kind.is_some(), "from_index({i}) should return Some");
            assert_eq!(kind.unwrap() as usize, i);
        }
    }

    #[test]
    fn from_index_out_of_bounds() {
        assert!(WindowKind::from_index(18).is_none());
        assert!(WindowKind::from_index(999).is_none());
    }

    // ── DockIcon ─────────────────────────────────────────────────────

    #[test]
    fn dock_icon_all_has_items() {
        assert!(DockIcon::all().len() >= 10);
    }

    #[test]
    fn dock_icon_separator_has_no_window() {
        assert!(DockIcon::Separator.window_kind().is_none());
        assert!(DockIcon::Separator.is_separator());
    }

    #[test]
    fn dock_icon_files_maps_to_file_manager() {
        assert_eq!(DockIcon::Files.window_kind(), Some(WindowKind::FileManager));
    }

    #[test]
    fn dock_icon_labels_nonempty_except_separator() {
        for icon in DockIcon::all() {
            if !icon.is_separator() {
                assert!(!icon.label().is_empty(), "{icon:?} has empty label");
            }
        }
    }

    #[test]
    fn dock_icon_bg_colors_not_all_same() {
        let colors: Vec<_> = DockIcon::all()
            .iter()
            .filter(|i| !i.is_separator())
            .map(|i| i.bg_color())
            .collect();
        // At least 5 distinct colors
        let mut unique = colors.clone();
        unique.dedup();
        assert!(unique.len() >= 5, "dock icons should have diverse colors");
    }

    // ── MenuDropdown ─────────────────────────────────────────────────

    #[test]
    fn menu_dropdown_labels_nonempty() {
        let menus = [
            MenuDropdown::File,
            MenuDropdown::Edit,
            MenuDropdown::View,
            MenuDropdown::Window,
            MenuDropdown::Help,
        ];
        for m in menus {
            assert!(!m.label().is_empty());
        }
    }

    #[test]
    fn menu_dropdown_items_nonempty() {
        let menus = [
            MenuDropdown::File,
            MenuDropdown::Edit,
            MenuDropdown::View,
            MenuDropdown::Window,
            MenuDropdown::Help,
        ];
        for m in menus {
            assert!(!m.items().is_empty(), "{m:?} has no items");
        }
    }

    #[test]
    fn menu_file_has_quit() {
        assert!(MenuDropdown::File
            .items()
            .iter()
            .any(|i| i.contains("Quit")));
    }

    #[test]
    fn menu_window_has_minimize() {
        assert!(MenuDropdown::Window
            .items()
            .iter()
            .any(|i| i.contains("Minimize")));
    }

    #[test]
    fn menu_window_has_start_screen_saver() {
        assert!(MenuDropdown::Window
            .items()
            .iter()
            .any(|i| i.contains("Start Screen Saver")));
    }
}
