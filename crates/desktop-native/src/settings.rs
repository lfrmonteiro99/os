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
    pub dark_mode: bool,
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
            dark_mode: false,
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
                "  \"dark_mode\": {}\n",
                "}}"
            ),
            self.wallpaper_idx, self.volume, self.brightness,
            self.wifi_enabled, self.bluetooth_enabled, self.airdrop_enabled,
            self.focus_mode, self.use_real_terminal, self.auto_save_interval_secs,
            self.font_size, self.show_fps, self.dock_magnification, self.dock_icon_size,
            self.dark_mode,
        )
    }

    pub fn from_json(json: &str) -> Self {
        let mut s = Self::default();
        if let Some(v) = parse_json_usize(json, "wallpaper") { s.wallpaper_idx = v; }
        if let Some(v) = parse_json_f32(json, "volume") { s.volume = v; }
        if let Some(v) = parse_json_f32(json, "brightness") { s.brightness = v; }
        if let Some(v) = parse_json_bool(json, "wifi") { s.wifi_enabled = v; }
        if let Some(v) = parse_json_bool(json, "bluetooth") { s.bluetooth_enabled = v; }
        if let Some(v) = parse_json_bool(json, "airdrop") { s.airdrop_enabled = v; }
        if let Some(v) = parse_json_bool(json, "focus_mode") { s.focus_mode = v; }
        if let Some(v) = parse_json_bool(json, "real_terminal") { s.use_real_terminal = v; }
        if let Some(v) = parse_json_usize(json, "auto_save_secs") { s.auto_save_interval_secs = v as u64; }
        if let Some(v) = parse_json_f32(json, "font_size") { s.font_size = v; }
        if let Some(v) = parse_json_bool(json, "show_fps") { s.show_fps = v; }
        if let Some(v) = parse_json_bool(json, "dock_magnification") { s.dock_magnification = v; }
        if let Some(v) = parse_json_f32(json, "dock_icon_size") { s.dock_icon_size = v; }
        if let Some(v) = parse_json_bool(json, "dark_mode") { s.dark_mode = v; }
        s.clamp();
        s
    }
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

fn parse_json_bool(json: &str, key: &str) -> Option<bool> {
    let pattern = format!("\"{}\":", key);
    let idx = json.find(&pattern)?;
    let rest = json[idx + pattern.len()..].trim();
    if rest.starts_with("true") { Some(true) }
    else if rest.starts_with("false") { Some(false) }
    else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }

    #[test]
    fn clamp_enforces_bounds() {
        let mut s = AppSettings::default();
        s.volume = 2.0;
        s.brightness = -1.0;
        s.font_size = 100.0;
        s.dock_icon_size = 5.0;
        s.clamp();
        assert_eq!(s.volume, 1.0);
        assert_eq!(s.brightness, 0.0);
        assert_eq!(s.font_size, 32.0);
        assert_eq!(s.dock_icon_size, 32.0);
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

        let json = original.to_json();
        let parsed = AppSettings::from_json(&json);

        assert_eq!(parsed.wallpaper_idx, 3);
        assert!((parsed.volume - 0.8).abs() < 0.01);
        assert!((parsed.brightness - 0.4).abs() < 0.01);
        assert!(!parsed.wifi_enabled);
        assert!(parsed.focus_mode);
        assert!((parsed.font_size - 16.0).abs() < 0.1);
        assert!(parsed.show_fps);
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
    }

    // ── JSON parsers ────────────────────────────────────────────────

    #[test]
    fn parse_json_f32_works() {
        assert_eq!(parse_json_f32(r#"{"volume": 0.75}"#, "volume"), Some(0.75));
    }

    #[test]
    fn parse_json_usize_works() {
        assert_eq!(parse_json_usize(r#"{"wallpaper": 2}"#, "wallpaper"), Some(2));
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
        let dir = std::env::temp_dir().join("aurora_test_settings");
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
        let _ = std::fs::remove_dir(&dir);
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
}
