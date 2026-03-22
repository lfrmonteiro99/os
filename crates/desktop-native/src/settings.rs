use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// Centralized user preferences for AuroraOS
pub struct AppSettings {
    pub wallpaper_idx: usize,
    pub volume: f32,
    pub brightness: f32,
    pub wifi_enabled: bool,
    pub bluetooth_enabled: bool,
    pub airdrop_enabled: bool,
    pub focus_mode: bool,
    pub use_real_terminal: bool,
    pub auto_save_interval_secs: u64,
    pub font_size: f32,
    pub show_fps: bool,
    pub dock_magnification: bool,
    pub dock_icon_size: f32,
    pub dock_auto_hide: bool,
    pub dock_position: String,
    pub dock_show_running_indicators: bool,
    pub low_power_mode: bool,
    pub dark_mode: bool,
    /// Accent color RGB (default: macOS blue 0,122,255)
    pub accent_r: u8,
    pub accent_g: u8,
    pub accent_b: u8,
    /// Custom wallpaper file path (empty = use built-in)
    pub custom_wallpaper: String,
    /// Which windows were open on last quit (comma-separated indices)
    pub open_windows: String,
    /// Z-order of windows (comma-separated indices, front-to-back)
    pub z_order: String,
    pub user_name: String,
    pub password_hash: String,
    pub idle_lock_minutes: u64,
    pub favorite_paths: String,
    pub desktop_use_stacks: bool,
    pub show_file_path_bar: bool,
    pub show_file_status_bar: bool,
    pub tag_labels: String,
    pub custom_smart_folders: String,
    pub recent_emojis: String,
    pub music_track_idx: usize,
    pub music_library_query: String,
    pub music_shuffle: bool,
    pub music_repeat_all: bool,
    pub music_elapsed_seconds: f32,
    pub color_picker_saved_colors: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            wallpaper_idx: 0,
            volume: 0.5,
            brightness: 0.7,
            wifi_enabled: true,
            bluetooth_enabled: true,
            airdrop_enabled: false,
            focus_mode: false,
            use_real_terminal: true,
            auto_save_interval_secs: 30,
            font_size: 13.0,
            show_fps: false,
            dock_magnification: true,
            dock_icon_size: 48.0,
            dock_auto_hide: false,
            dock_position: "bottom".to_string(),
            dock_show_running_indicators: true,
            low_power_mode: false,
            dark_mode: false,
            accent_r: 0,
            accent_g: 122,
            accent_b: 255,
            custom_wallpaper: String::new(),
            open_windows: String::new(),
            z_order: String::new(),
            user_name: String::new(),
            password_hash: String::new(),
            idle_lock_minutes: 5,
            favorite_paths: String::new(),
            desktop_use_stacks: false,
            show_file_path_bar: true,
            show_file_status_bar: true,
            tag_labels: String::new(),
            custom_smart_folders: String::new(),
            recent_emojis: String::new(),
            music_track_idx: 0,
            music_library_query: String::new(),
            music_shuffle: false,
            music_repeat_all: true,
            music_elapsed_seconds: 0.0,
            color_picker_saved_colors: String::new(),
        }
    }
}

impl AppSettings {
    /// Path to settings file in user home
    pub fn settings_path() -> PathBuf {
        let home = if cfg!(windows) {
            std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into())
        } else {
            std::env::var("HOME").unwrap_or_else(|_| ".".into())
        };
        PathBuf::from(home).join(".aurora_settings.json")
    }

    /// Save settings to disk
    pub fn save(&self) -> Result<(), String> {
        let json = self.to_json();
        std::fs::write(Self::settings_path(), json).map_err(|e| e.to_string())
    }

    /// Load settings from disk, falling back to defaults
    pub fn load() -> Self {
        match std::fs::read_to_string(Self::settings_path()) {
            Ok(content) => Self::from_json(&content),
            Err(_) => Self::default(),
        }
    }
}

impl AppSettings {
    pub fn clamp(&mut self) {
        self.volume = self.volume.clamp(0.0, 1.0);
        self.brightness = self.brightness.clamp(0.0, 1.0);
        self.font_size = self.font_size.clamp(8.0, 32.0);
        self.dock_icon_size = self.dock_icon_size.clamp(32.0, 72.0);
        self.idle_lock_minutes = self.idle_lock_minutes.clamp(1, 120);
        self.music_elapsed_seconds = self.music_elapsed_seconds.clamp(0.0, 3600.0);
    }

    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"wallpaper\": {},\n",
                "  \"volume\": {:.2},\n",
                "  \"brightness\": {:.2},\n",
                "  \"wifi\": {},\n",
                "  \"bluetooth\": {},\n",
                "  \"airdrop\": {},\n",
                "  \"focus_mode\": {},\n",
                "  \"real_terminal\": {},\n",
                "  \"auto_save_secs\": {},\n",
                "  \"font_size\": {:.1},\n",
                "  \"show_fps\": {},\n",
                "  \"dock_magnification\": {},\n",
                "  \"dock_icon_size\": {:.1},\n",
                "  \"dock_auto_hide\": {},\n",
                "  \"dock_position\": \"{}\",\n",
                "  \"dock_show_running_indicators\": {},\n",
                "  \"low_power_mode\": {},\n",
                "  \"dark_mode\": {},\n",
                "  \"accent_r\": {},\n",
                "  \"accent_g\": {},\n",
                "  \"accent_b\": {},\n",
                "  \"custom_wallpaper\": \"{}\",\n",
                "  \"open_windows\": \"{}\",\n",
                "  \"z_order\": \"{}\",\n",
                "  \"user_name\": \"{}\",\n",
                "  \"password_hash\": \"{}\",\n",
                "  \"idle_lock_minutes\": {},\n",
                "  \"favorite_paths\": \"{}\",\n",
                "  \"desktop_use_stacks\": {},\n",
                "  \"show_file_path_bar\": {},\n",
                "  \"show_file_status_bar\": {},\n",
                "  \"tag_labels\": \"{}\",\n",
                "  \"custom_smart_folders\": \"{}\",\n",
                "  \"recent_emojis\": \"{}\",\n",
                "  \"music_track_idx\": {},\n",
                "  \"music_library_query\": \"{}\",\n",
                "  \"music_shuffle\": {},\n",
                "  \"music_repeat_all\": {},\n",
                "  \"music_elapsed_seconds\": {:.2},\n",
                "  \"color_picker_saved_colors\": \"{}\"\n",
                "}}"
            ),
            self.wallpaper_idx,
            self.volume,
            self.brightness,
            self.wifi_enabled,
            self.bluetooth_enabled,
            self.airdrop_enabled,
            self.focus_mode,
            self.use_real_terminal,
            self.auto_save_interval_secs,
            self.font_size,
            self.show_fps,
            self.dock_magnification,
            self.dock_icon_size,
            self.dock_auto_hide,
            self.dock_position,
            self.dock_show_running_indicators,
            self.low_power_mode,
            self.dark_mode,
            self.accent_r,
            self.accent_g,
            self.accent_b,
            self.custom_wallpaper,
            self.open_windows,
            self.z_order,
            self.user_name,
            self.password_hash,
            self.idle_lock_minutes,
            self.favorite_paths,
            self.desktop_use_stacks,
            self.show_file_path_bar,
            self.show_file_status_bar,
            self.tag_labels,
            self.custom_smart_folders,
            self.recent_emojis,
            self.music_track_idx,
            self.music_library_query,
            self.music_shuffle,
            self.music_repeat_all,
            self.music_elapsed_seconds,
            self.color_picker_saved_colors,
        )
    }

    pub fn from_json(json: &str) -> Self {
        let mut s = Self::default();
        if let Some(v) = parse_json_usize(json, "wallpaper") {
            s.wallpaper_idx = v;
        }
        if let Some(v) = parse_json_f32(json, "volume") {
            s.volume = v;
        }
        if let Some(v) = parse_json_f32(json, "brightness") {
            s.brightness = v;
        }
        if let Some(v) = parse_json_bool(json, "wifi") {
            s.wifi_enabled = v;
        }
        if let Some(v) = parse_json_bool(json, "bluetooth") {
            s.bluetooth_enabled = v;
        }
        if let Some(v) = parse_json_bool(json, "airdrop") {
            s.airdrop_enabled = v;
        }
        if let Some(v) = parse_json_bool(json, "focus_mode") {
            s.focus_mode = v;
        }
        if let Some(v) = parse_json_bool(json, "real_terminal") {
            s.use_real_terminal = v;
        }
        if let Some(v) = parse_json_usize(json, "auto_save_secs") {
            s.auto_save_interval_secs = v as u64;
        }
        if let Some(v) = parse_json_f32(json, "font_size") {
            s.font_size = v;
        }
        if let Some(v) = parse_json_bool(json, "show_fps") {
            s.show_fps = v;
        }
        if let Some(v) = parse_json_bool(json, "dock_magnification") {
            s.dock_magnification = v;
        }
        if let Some(v) = parse_json_f32(json, "dock_icon_size") {
            s.dock_icon_size = v;
        }
        if let Some(v) = parse_json_bool(json, "dock_auto_hide") {
            s.dock_auto_hide = v;
        }
        if let Some(v) = parse_json_string(json, "dock_position") {
            s.dock_position = v;
        }
        if let Some(v) = parse_json_bool(json, "dock_show_running_indicators") {
            s.dock_show_running_indicators = v;
        }
        if let Some(v) = parse_json_bool(json, "low_power_mode") {
            s.low_power_mode = v;
        }
        if let Some(v) = parse_json_bool(json, "dark_mode") {
            s.dark_mode = v;
        }
        if let Some(v) = parse_json_usize(json, "accent_r") {
            s.accent_r = v as u8;
        }
        if let Some(v) = parse_json_usize(json, "accent_g") {
            s.accent_g = v as u8;
        }
        if let Some(v) = parse_json_usize(json, "accent_b") {
            s.accent_b = v as u8;
        }
        if let Some(v) = parse_json_string(json, "custom_wallpaper") {
            s.custom_wallpaper = v;
        }
        if let Some(v) = parse_json_string(json, "open_windows") {
            s.open_windows = v;
        }
        if let Some(v) = parse_json_string(json, "z_order") {
            s.z_order = v;
        }
        if let Some(v) = parse_json_string(json, "user_name") {
            s.user_name = v;
        }
        if let Some(v) = parse_json_string(json, "password_hash") {
            s.password_hash = v;
        }
        if let Some(v) = parse_json_usize(json, "idle_lock_minutes") {
            s.idle_lock_minutes = v as u64;
        }
        if let Some(v) = parse_json_string(json, "favorite_paths") {
            s.favorite_paths = v;
        }
        if let Some(v) = parse_json_bool(json, "desktop_use_stacks") {
            s.desktop_use_stacks = v;
        }
        if let Some(v) = parse_json_bool(json, "show_file_path_bar") {
            s.show_file_path_bar = v;
        }
        if let Some(v) = parse_json_bool(json, "show_file_status_bar") {
            s.show_file_status_bar = v;
        }
        if let Some(v) = parse_json_string(json, "tag_labels") {
            s.tag_labels = v;
        }
        if let Some(v) = parse_json_string(json, "custom_smart_folders") {
            s.custom_smart_folders = v;
        }
        if let Some(v) = parse_json_string(json, "recent_emojis") {
            s.recent_emojis = v;
        }
        if let Some(v) = parse_json_usize(json, "music_track_idx") {
            s.music_track_idx = v;
        }
        if let Some(v) = parse_json_string(json, "music_library_query") {
            s.music_library_query = v;
        }
        if let Some(v) = parse_json_bool(json, "music_shuffle") {
            s.music_shuffle = v;
        }
        if let Some(v) = parse_json_bool(json, "music_repeat_all") {
            s.music_repeat_all = v;
        }
        if let Some(v) = parse_json_f32(json, "music_elapsed_seconds") {
            s.music_elapsed_seconds = v;
        }
        if let Some(v) = parse_json_string(json, "color_picker_saved_colors") {
            s.color_picker_saved_colors = v;
        }
        s.clamp();
        s
    }

    pub fn has_user_profile(&self) -> bool {
        !self.user_name.trim().is_empty() && !self.password_hash.is_empty()
    }

    pub fn set_password(&mut self, password: &str) {
        self.password_hash = hash_password(password);
    }

    pub fn verify_password(&self, password: &str) -> bool {
        self.password_hash == hash_password(password)
    }
}

pub fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn parse_json_f32(json: &str, key: &str) -> Option<f32> {
    let pattern = format!("\"{}\":", key);
    let idx = json.find(&pattern)?;
    let rest = json[idx + pattern.len()..].trim();
    rest.split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .next()?
        .parse()
        .ok()
}

fn parse_json_usize(json: &str, key: &str) -> Option<usize> {
    let pattern = format!("\"{}\":", key);
    let idx = json.find(&pattern)?;
    let rest = json[idx + pattern.len()..].trim();
    rest.split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

fn parse_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":", key);
    let idx = json.find(&pattern)?;
    let rest = json[idx + pattern.len()..].trim();
    if !rest.starts_with('"') {
        return None;
    }
    let inner = &rest[1..];
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

fn parse_json_bool(json: &str, key: &str) -> Option<bool> {
    let pattern = format!("\"{}\":", key);
    let idx = json.find(&pattern)?;
    let rest = json[idx + pattern.len()..].trim();
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn default_settings_are_sane() {
        let s = AppSettings::default();
        assert_eq!(s.wallpaper_idx, 0);
        assert!((s.volume - 0.5).abs() < 0.01);
        assert!((s.brightness - 0.7).abs() < 0.01);
        assert!(s.wifi_enabled);
        assert!(s.bluetooth_enabled);
        assert!(!s.airdrop_enabled);
        assert!(!s.focus_mode);
        assert!(s.use_real_terminal);
        assert_eq!(s.auto_save_interval_secs, 30);
        assert!((s.font_size - 13.0).abs() < 0.1);
        assert!(!s.show_fps);
        assert!(s.dock_magnification);
        assert!(!s.low_power_mode);
        assert!(s.user_name.is_empty());
        assert!(s.password_hash.is_empty());
        assert_eq!(s.idle_lock_minutes, 5);
        assert!(s.favorite_paths.is_empty());
        assert!(!s.desktop_use_stacks);
        assert!(s.recent_emojis.is_empty());
        assert_eq!(s.music_track_idx, 0);
        assert!(s.music_library_query.is_empty());
        assert!(!s.music_shuffle);
        assert!(s.music_repeat_all);
        assert_eq!(s.music_elapsed_seconds, 0.0);
        assert!(s.color_picker_saved_colors.is_empty());
    }

    #[test]
    fn clamp_enforces_bounds() {
        let mut s = AppSettings::default();
        s.volume = 2.0;
        s.brightness = -1.0;
        s.font_size = 100.0;
        s.dock_icon_size = 5.0;
        s.idle_lock_minutes = 0;
        s.clamp();
        assert_eq!(s.volume, 1.0);
        assert_eq!(s.brightness, 0.0);
        assert_eq!(s.font_size, 32.0);
        assert_eq!(s.dock_icon_size, 32.0);
        assert_eq!(s.idle_lock_minutes, 1);
    }

    #[test]
    fn roundtrip_json() {
        let mut original = AppSettings::default();
        original.wallpaper_idx = 3;
        original.volume = 0.8;
        original.brightness = 0.4;
        original.wifi_enabled = false;
        original.focus_mode = true;
        original.font_size = 16.0;
        original.show_fps = true;
        original.user_name = "Aurora User".to_string();
        original.set_password("secret");
        original.idle_lock_minutes = 10;
        original.favorite_paths = "C:/Users/test/Desktop|C:/Users/test/Documents".to_string();
        original.music_track_idx = 4;
        original.music_library_query = "drive".to_string();
        original.music_shuffle = true;
        original.music_repeat_all = false;
        original.music_elapsed_seconds = 87.5;

        let json = original.to_json();
        let parsed = AppSettings::from_json(&json);

        assert_eq!(parsed.wallpaper_idx, 3);
        assert!((parsed.volume - 0.8).abs() < 0.01);
        assert!((parsed.brightness - 0.4).abs() < 0.01);
        assert!(!parsed.wifi_enabled);
        assert!(parsed.focus_mode);
        assert!((parsed.font_size - 16.0).abs() < 0.1);
        assert!(parsed.show_fps);
        assert_eq!(parsed.user_name, "Aurora User");
        assert!(parsed.verify_password("secret"));
        assert_eq!(parsed.idle_lock_minutes, 10);
        assert_eq!(
            parsed.favorite_paths,
            "C:/Users/test/Desktop|C:/Users/test/Documents"
        );
        assert_eq!(parsed.music_track_idx, 4);
        assert_eq!(parsed.music_library_query, "drive");
        assert!(parsed.music_shuffle);
        assert!(!parsed.music_repeat_all);
        assert!((parsed.music_elapsed_seconds - 87.5).abs() < 0.01);
    }

    #[test]
    fn from_json_missing_keys_use_defaults() {
        let s = AppSettings::from_json("{}");
        let d = AppSettings::default();
        assert_eq!(s.wallpaper_idx, d.wallpaper_idx);
        assert!((s.volume - d.volume).abs() < 0.01);
    }

    #[test]
    fn from_json_invalid_values_use_defaults() {
        let json = r#"{"volume": "not_a_number", "wallpaper": "abc"}"#;
        let s = AppSettings::from_json(json);
        let d = AppSettings::default();
        assert!((s.volume - d.volume).abs() < 0.01);
        assert_eq!(s.wallpaper_idx, d.wallpaper_idx);
    }

    #[test]
    fn to_json_contains_all_keys() {
        let s = AppSettings::default();
        let json = s.to_json();
        assert!(json.contains("\"wallpaper\""));
        assert!(json.contains("\"volume\""));
        assert!(json.contains("\"brightness\""));
        assert!(json.contains("\"wifi\""));
        assert!(json.contains("\"bluetooth\""));
        assert!(json.contains("\"focus_mode\""));
        assert!(json.contains("\"real_terminal\""));
        assert!(json.contains("\"auto_save_secs\""));
        assert!(json.contains("\"font_size\""));
        assert!(json.contains("\"show_fps\""));
        assert!(json.contains("\"dock_magnification\""));
        assert!(json.contains("\"dock_icon_size\""));
        assert!(json.contains("\"low_power_mode\""));
        assert!(json.contains("\"user_name\""));
        assert!(json.contains("\"password_hash\""));
        assert!(json.contains("\"idle_lock_minutes\""));
        assert!(json.contains("\"favorite_paths\""));
        assert!(json.contains("\"recent_emojis\""));
        assert!(json.contains("\"music_track_idx\""));
        assert!(json.contains("\"music_library_query\""));
        assert!(json.contains("\"music_shuffle\""));
        assert!(json.contains("\"music_repeat_all\""));
        assert!(json.contains("\"music_elapsed_seconds\""));
        assert!(json.contains("\"color_picker_saved_colors\""));
    }

    // ── JSON parsers ────────────────────────────────────────────────

    #[test]
    fn parse_json_f32_works() {
        assert_eq!(parse_json_f32(r#"{"volume": 0.75}"#, "volume"), Some(0.75));
    }

    #[test]
    fn parse_json_usize_works() {
        assert_eq!(
            parse_json_usize(r#"{"wallpaper": 2}"#, "wallpaper"),
            Some(2)
        );
    }

    #[test]
    fn parse_json_bool_works() {
        assert_eq!(parse_json_bool(r#"{"wifi": true}"#, "wifi"), Some(true));
        assert_eq!(parse_json_bool(r#"{"wifi": false}"#, "wifi"), Some(false));
    }

    #[test]
    fn parse_json_missing_key_returns_none() {
        assert_eq!(parse_json_f32(r#"{"other": 1.0}"#, "volume"), None);
    }

    // ── dark_mode ───────────────────────────────────────────────────

    #[test]
    fn dark_mode_default_is_false() {
        let s = AppSettings::default();
        assert!(!s.dark_mode);
    }

    #[test]
    fn dark_mode_roundtrip() {
        let mut s = AppSettings::default();
        s.dark_mode = true;
        let json = s.to_json();
        let parsed = AppSettings::from_json(&json);
        assert!(parsed.dark_mode);
    }

    #[test]
    fn to_json_contains_dark_mode() {
        let s = AppSettings::default();
        let json = s.to_json();
        assert!(json.contains("\"dark_mode\""));
    }

    // ── persistence ─────────────────────────────────────────────────

    #[test]
    fn save_and_load_roundtrip() {
        // Use a temp dir to avoid polluting user home
        let dir = unique_temp_dir("aurora_test_settings");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(".aurora_settings_test.json");

        let mut original = AppSettings::default();
        original.volume = 0.3;
        original.dark_mode = true;
        original.wallpaper_idx = 2;

        // Manual save to temp path
        let json = original.to_json();
        std::fs::write(&path, &json).unwrap();

        // Manual load
        let content = std::fs::read_to_string(&path).unwrap();
        let loaded = AppSettings::from_json(&content);

        assert!((loaded.volume - 0.3).abs() < 0.01);
        assert!(loaded.dark_mode);
        assert_eq!(loaded.wallpaper_idx, 2);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        // from_json with empty string gives defaults
        let s = AppSettings::from_json("");
        let d = AppSettings::default();
        assert_eq!(s.wallpaper_idx, d.wallpaper_idx);
        assert!(!s.dark_mode);
    }

    #[test]
    fn settings_path_is_not_empty() {
        let path = AppSettings::settings_path();
        assert!(!path.to_string_lossy().is_empty());
        assert!(path.to_string_lossy().contains("aurora_settings"));
    }

    // ── accent color ─────────────────────────────────────────────────

    #[test]
    fn accent_color_defaults_blue() {
        let s = AppSettings::default();
        assert_eq!((s.accent_r, s.accent_g, s.accent_b), (0, 122, 255));
    }

    #[test]
    fn accent_color_roundtrip() {
        let mut s = AppSettings::default();
        s.accent_r = 255;
        s.accent_g = 59;
        s.accent_b = 48;
        let json = s.to_json();
        let parsed = AppSettings::from_json(&json);
        assert_eq!(
            (parsed.accent_r, parsed.accent_g, parsed.accent_b),
            (255, 59, 48)
        );
    }

    // ── custom wallpaper ────────────────────────────────────────────

    #[test]
    fn custom_wallpaper_default_empty() {
        let s = AppSettings::default();
        assert!(s.custom_wallpaper.is_empty());
    }

    #[test]
    fn custom_wallpaper_roundtrip() {
        let mut s = AppSettings::default();
        s.custom_wallpaper = "C:/wallpapers/sunset.png".to_string();
        let json = s.to_json();
        let parsed = AppSettings::from_json(&json);
        assert_eq!(parsed.custom_wallpaper, "C:/wallpapers/sunset.png");
    }

    // ── window restore ──────────────────────────────────────────────

    #[test]
    fn open_windows_default_empty() {
        let s = AppSettings::default();
        assert!(s.open_windows.is_empty());
    }

    #[test]
    fn open_windows_roundtrip() {
        let mut s = AppSettings::default();
        s.open_windows = "0,2,4,7".to_string();
        s.z_order = "7,4,2,0".to_string();
        let json = s.to_json();
        let parsed = AppSettings::from_json(&json);
        assert_eq!(parsed.open_windows, "0,2,4,7");
        assert_eq!(parsed.z_order, "7,4,2,0");
    }

    #[test]
    fn password_hash_is_not_plaintext() {
        let hashed = hash_password("secret");
        assert_ne!(hashed, "secret");
        assert_eq!(hashed.len(), 64);
    }

    #[test]
    fn verify_password_matches_hash() {
        let mut s = AppSettings::default();
        s.user_name = "Aurora User".to_string();
        s.set_password("correct horse battery staple");
        assert!(s.has_user_profile());
        assert!(s.verify_password("correct horse battery staple"));
        assert!(!s.verify_password("wrong password"));
    }

    #[test]
    fn has_user_profile_requires_name_and_password() {
        let mut s = AppSettings::default();
        assert!(!s.has_user_profile());
        s.user_name = "Aurora User".to_string();
        assert!(!s.has_user_profile());
        s.set_password("secret");
        assert!(s.has_user_profile());
    }

    #[test]
    fn favorite_paths_roundtrip() {
        let mut s = AppSettings::default();
        s.favorite_paths = "C:/One|C:/Two".to_string();
        let parsed = AppSettings::from_json(&s.to_json());
        assert_eq!(parsed.favorite_paths, "C:/One|C:/Two");
    }

    #[test]
    fn file_bars_roundtrip() {
        let mut s = AppSettings::default();
        s.show_file_path_bar = false;
        s.show_file_status_bar = false;
        let parsed = AppSettings::from_json(&s.to_json());
        assert!(!parsed.show_file_path_bar);
        assert!(!parsed.show_file_status_bar);
    }

    #[test]
    fn dock_preferences_roundtrip() {
        let mut s = AppSettings::default();
        s.dock_auto_hide = true;
        s.dock_position = "left".to_string();
        s.dock_show_running_indicators = false;
        s.low_power_mode = true;
        let parsed = AppSettings::from_json(&s.to_json());
        assert!(parsed.dock_auto_hide);
        assert_eq!(parsed.dock_position, "left");
        assert!(!parsed.dock_show_running_indicators);
        assert!(parsed.low_power_mode);
    }

    #[test]
    fn tag_labels_roundtrip() {
        let mut s = AppSettings::default();
        s.tag_labels = "red:Urgent|blue:Work".to_string();
        let parsed = AppSettings::from_json(&s.to_json());
        assert_eq!(parsed.tag_labels, "red:Urgent|blue:Work");
    }

    #[test]
    fn custom_smart_folders_roundtrip() {
        let mut s = AppSettings::default();
        s.custom_smart_folders = "Big Logs;log;1;red".to_string();
        let parsed = AppSettings::from_json(&s.to_json());
        assert_eq!(parsed.custom_smart_folders, "Big Logs;log;1;red");
    }

    #[test]
    fn recent_emojis_roundtrip() {
        let mut s = AppSettings::default();
        s.recent_emojis = "😀|🚀|👍".to_string();
        let parsed = AppSettings::from_json(&s.to_json());
        assert_eq!(parsed.recent_emojis, "😀|🚀|👍");
    }

    #[test]
    fn music_player_state_roundtrip() {
        let mut s = AppSettings::default();
        s.music_track_idx = 3;
        s.music_library_query = "focus".to_string();
        s.music_shuffle = true;
        s.music_repeat_all = false;
        s.music_elapsed_seconds = 42.25;
        let parsed = AppSettings::from_json(&s.to_json());
        assert_eq!(parsed.music_track_idx, 3);
        assert_eq!(parsed.music_library_query, "focus");
        assert!(parsed.music_shuffle);
        assert!(!parsed.music_repeat_all);
        assert!((parsed.music_elapsed_seconds - 42.25).abs() < 0.01);
    }

    // ── parse_json_string ───────────────────────────────────────────

    #[test]
    fn parse_json_string_works() {
        assert_eq!(
            parse_json_string(r#"{"path": "hello"}"#, "path"),
            Some("hello".to_string())
        );
    }

    #[test]
    fn parse_json_string_empty() {
        assert_eq!(
            parse_json_string(r#"{"path": ""}"#, "path"),
            Some("".to_string())
        );
    }

    #[test]
    fn parse_json_string_missing() {
        assert_eq!(parse_json_string(r#"{"other": "x"}"#, "path"), None);
    }

    #[test]
    fn color_picker_saved_colors_roundtrip() {
        let mut s = AppSettings::default();
        s.color_picker_saved_colors = "#FF0000|#00FF00".to_string();
        let parsed = AppSettings::from_json(&s.to_json());
        assert_eq!(parsed.color_picker_saved_colors, "#FF0000|#00FF00");
    }
}
