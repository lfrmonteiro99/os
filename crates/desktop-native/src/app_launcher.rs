//! Application discovery and Launchpad data model.
//!
//! Scans the system for installed applications (Start Menu shortcuts on Windows,
//! /usr/share/applications on Linux, /Applications on macOS) and provides a
//! searchable catalog for the Launchpad UI.

use std::path::PathBuf;

/// A discovered application entry.
#[derive(Clone, Debug)]
pub struct AppEntry {
    pub name: String,
    pub path: PathBuf,
    /// Category for grouping (e.g., "Utilities", "Games", "Development")
    pub category: String,
}

/// The application catalog — scanned once, then filtered by query.
pub struct AppCatalog {
    pub apps: Vec<AppEntry>,
    pub last_scan: Option<std::time::Instant>,
}

impl AppCatalog {
    pub fn new() -> Self {
        let mut catalog = Self {
            apps: Vec::new(),
            last_scan: None,
        };
        catalog.scan();
        catalog
    }

    /// Re-scan for installed applications.
    pub fn scan(&mut self) {
        self.apps.clear();

        #[cfg(windows)]
        self.scan_windows();

        #[cfg(target_os = "linux")]
        self.scan_linux();

        #[cfg(target_os = "macos")]
        self.scan_macos();

        // Add AuroraOS built-in apps
        self.add_builtin_apps();

        // Sort by name
        self.apps
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        // Deduplicate by name (keep first occurrence)
        self.apps
            .dedup_by(|a, b| a.name.to_lowercase() == b.name.to_lowercase());

        self.last_scan = Some(std::time::Instant::now());
    }

    /// Search/filter apps by query (case-insensitive).
    pub fn search(&self, query: &str) -> Vec<&AppEntry> {
        if query.is_empty() {
            return self.apps.iter().collect();
        }
        let q = query.to_lowercase();
        self.apps
            .iter()
            .filter(|app| {
                app.name.to_lowercase().contains(&q) || app.category.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Get apps grouped by category.
    #[allow(dead_code)]
    pub fn by_category(&self) -> Vec<(&str, Vec<&AppEntry>)> {
        let mut categories: std::collections::BTreeMap<&str, Vec<&AppEntry>> =
            std::collections::BTreeMap::new();
        for app in &self.apps {
            categories.entry(&app.category).or_default().push(app);
        }
        categories.into_iter().collect()
    }

    fn add_builtin_apps(&mut self) {
        let builtins = [
            ("System Overview", "System"),
            ("Terminal", "Utilities"),
            ("Files", "Utilities"),
            ("Browser", "Internet"),
            ("Calculator", "Utilities"),
            ("Notes", "Productivity"),
            ("Music", "Media"),
            ("Photos", "Media"),
            ("Calendar", "Productivity"),
            ("TextEdit", "Productivity"),
            ("Settings", "System"),
            ("Activity Monitor", "System"),
            ("Network Diagnostics", "System"),
            ("Disk Utility", "Utilities"),
            ("Font Book", "Productivity"),
            ("Dictionary", "Productivity"),
            ("Console", "Utilities"),
            ("Messages", "Communication"),
            ("Quick Controls", "System"),
        ];
        for (name, cat) in builtins {
            self.apps.push(AppEntry {
                name: name.to_string(),
                path: PathBuf::from(format!(
                    "aurora://{}",
                    name.to_lowercase().replace(' ', "-")
                )),
                category: cat.to_string(),
            });
        }
    }

    #[cfg(windows)]
    fn scan_windows(&mut self) {
        // Scan Start Menu shortcuts (both per-user and all-users)
        let start_menu_paths: Vec<PathBuf> = [
            std::env::var("APPDATA")
                .ok()
                .map(|p| PathBuf::from(p).join("Microsoft\\Windows\\Start Menu\\Programs")),
            std::env::var("ProgramData")
                .ok()
                .map(|p| PathBuf::from(p).join("Microsoft\\Windows\\Start Menu\\Programs")),
        ]
        .into_iter()
        .flatten()
        .collect();

        for start_dir in &start_menu_paths {
            if !start_dir.exists() {
                continue;
            }
            self.scan_directory_recursive(start_dir, 3);
        }
    }

    #[cfg(windows)]
    fn scan_directory_recursive(&mut self, dir: &std::path::Path, max_depth: u32) {
        if max_depth == 0 {
            return;
        }
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        let folder_name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                self.scan_directory_recursive(&path, max_depth - 1);
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "lnk" && ext != "exe" {
                continue;
            }

            let name = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip uninstall shortcuts and other noise
            if name.to_lowercase().contains("uninstall")
                || name.to_lowercase().contains("readme")
                || name.to_lowercase().contains("website")
                || name.to_lowercase().contains("license")
                || name.to_lowercase().contains("help")
                || name.is_empty()
            {
                continue;
            }

            let category = categorize_app(&name, &folder_name);

            self.apps.push(AppEntry {
                name,
                path,
                category,
            });
        }
    }

    #[cfg(target_os = "linux")]
    fn scan_linux(&mut self) {
        let app_dirs = ["/usr/share/applications", "/usr/local/share/applications"];
        for dir in &app_dirs {
            let path = std::path::Path::new(dir);
            if !path.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().and_then(|e| e.to_str()) != Some("desktop") {
                        continue;
                    }
                    if let Ok(content) = std::fs::read_to_string(&p) {
                        let name = content
                            .lines()
                            .find(|l| l.starts_with("Name="))
                            .map(|l| l[5..].to_string())
                            .unwrap_or_default();
                        let cat = content
                            .lines()
                            .find(|l| l.starts_with("Categories="))
                            .map(|l| l[11..].split(';').next().unwrap_or("Other").to_string())
                            .unwrap_or_else(|| "Other".to_string());
                        if !name.is_empty() {
                            self.apps.push(AppEntry {
                                name,
                                path: p,
                                category: cat,
                            });
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn scan_macos(&mut self) {
        let app_dir = std::path::Path::new("/Applications");
        if let Ok(entries) = std::fs::read_dir(app_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("app") {
                    let name = p
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    if !name.is_empty() {
                        self.apps.push(AppEntry {
                            name,
                            path: p,
                            category: "Applications".to_string(),
                        });
                    }
                }
            }
        }
    }
}

/// Heuristic category assignment based on app name and folder.
fn categorize_app(name: &str, folder: &str) -> String {
    let lower_name = name.to_lowercase();
    let lower_folder = folder.to_lowercase();

    // Check folder name first
    if lower_folder.contains("game") {
        return "Games".to_string();
    }
    if lower_folder.contains("develop") || lower_folder.contains("programming") {
        return "Development".to_string();
    }
    if lower_folder.contains("office") || lower_folder.contains("microsoft office") {
        return "Productivity".to_string();
    }
    if lower_folder.contains("system") || lower_folder.contains("admin") {
        return "System".to_string();
    }
    if lower_folder.contains("startup") {
        return "Startup".to_string();
    }
    if lower_folder.contains("access") {
        return "Accessibility".to_string();
    }

    // Check app name
    if lower_name.contains("code")
        || lower_name.contains("studio")
        || lower_name.contains("ide")
        || lower_name.contains("git")
        || lower_name.contains("terminal")
        || lower_name.contains("python")
        || lower_name.contains("node")
        || lower_name.contains("rust")
        || lower_name.contains("docker")
    {
        return "Development".to_string();
    }
    if lower_name.contains("chrome")
        || lower_name.contains("firefox")
        || lower_name.contains("edge")
        || lower_name.contains("browser")
        || lower_name.contains("opera")
        || lower_name.contains("brave")
    {
        return "Internet".to_string();
    }
    if lower_name.contains("word")
        || lower_name.contains("excel")
        || lower_name.contains("powerpoint")
        || lower_name.contains("outlook")
        || lower_name.contains("onenote")
        || lower_name.contains("notion")
    {
        return "Productivity".to_string();
    }
    if lower_name.contains("spotify")
        || lower_name.contains("music")
        || lower_name.contains("vlc")
        || lower_name.contains("media")
        || lower_name.contains("player")
        || lower_name.contains("photo")
        || lower_name.contains("video")
    {
        return "Media".to_string();
    }
    if lower_name.contains("discord")
        || lower_name.contains("slack")
        || lower_name.contains("teams")
        || lower_name.contains("zoom")
        || lower_name.contains("telegram")
        || lower_name.contains("whatsapp")
        || lower_name.contains("skype")
    {
        return "Communication".to_string();
    }
    if lower_name.contains("steam") || lower_name.contains("epic") || lower_name.contains("game") {
        return "Games".to_string();
    }
    if lower_name.contains("paint")
        || lower_name.contains("photoshop")
        || lower_name.contains("gimp")
        || lower_name.contains("blender")
        || lower_name.contains("figma")
        || lower_name.contains("inkscape")
    {
        return "Graphics".to_string();
    }
    if lower_name.contains("notepad")
        || lower_name.contains("calc")
        || lower_name.contains("clock")
        || lower_name.contains("snip")
        || lower_name.contains("task manager")
        || lower_name.contains("cmd")
        || lower_name.contains("powershell")
    {
        return "Utilities".to_string();
    }

    "Other".to_string()
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_builtin_apps() {
        let catalog = AppCatalog::new();
        let names: Vec<&str> = catalog.apps.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"Terminal"), "Should have Terminal");
        assert!(names.contains(&"Files"), "Should have Files");
        assert!(names.contains(&"Calculator"), "Should have Calculator");
        assert!(names.contains(&"Settings"), "Should have Settings");
    }

    #[test]
    fn search_empty_returns_all() {
        let catalog = AppCatalog::new();
        let results = catalog.search("");
        assert_eq!(results.len(), catalog.apps.len());
    }

    #[test]
    fn search_filters_by_name() {
        let catalog = AppCatalog::new();
        let results = catalog.search("Terminal");
        assert!(results.iter().any(|a| a.name == "Terminal"));
    }

    #[test]
    fn search_is_case_insensitive() {
        let catalog = AppCatalog::new();
        let r1 = catalog.search("terminal");
        let r2 = catalog.search("TERMINAL");
        assert_eq!(r1.len(), r2.len());
    }

    #[test]
    fn search_by_category() {
        let catalog = AppCatalog::new();
        let results = catalog.search("Utilities");
        assert!(
            !results.is_empty(),
            "Should find apps in Utilities category"
        );
    }

    #[test]
    fn by_category_groups_correctly() {
        let catalog = AppCatalog::new();
        let groups = catalog.by_category();
        assert!(!groups.is_empty(), "Should have at least one category");
        // System category should exist (builtins include System Overview, Settings, etc.)
        assert!(
            groups.iter().any(|(cat, _)| *cat == "System"),
            "Should have System category"
        );
    }

    #[test]
    fn apps_are_sorted_by_name() {
        let catalog = AppCatalog::new();
        for window in catalog.apps.windows(2) {
            assert!(
                window[0].name.to_lowercase() <= window[1].name.to_lowercase(),
                "'{}' should come before '{}'",
                window[0].name,
                window[1].name,
            );
        }
    }

    #[test]
    fn no_duplicate_names() {
        let catalog = AppCatalog::new();
        let mut seen = std::collections::HashSet::new();
        for app in &catalog.apps {
            let lower = app.name.to_lowercase();
            assert!(
                seen.insert(lower.clone()),
                "Duplicate app name: {}",
                app.name
            );
        }
    }

    #[test]
    fn categorize_app_dev_tools() {
        assert_eq!(
            categorize_app("Visual Studio Code", "Programs"),
            "Development"
        );
        assert_eq!(categorize_app("Git Bash", "Git"), "Development");
    }

    #[test]
    fn categorize_app_browsers() {
        assert_eq!(categorize_app("Google Chrome", "Programs"), "Internet");
        assert_eq!(categorize_app("Firefox", "Mozilla Firefox"), "Internet");
    }

    #[test]
    fn categorize_app_games_folder() {
        assert_eq!(categorize_app("Something", "Games"), "Games");
    }

    #[test]
    fn categorize_app_unknown() {
        assert_eq!(categorize_app("SomeRandomApp", "SomeFolder"), "Other");
    }

    #[cfg(windows)]
    #[test]
    fn scan_finds_system_apps() {
        let catalog = AppCatalog::new();
        // On any Windows system there should be Start Menu entries
        let non_builtin: Vec<_> = catalog
            .apps
            .iter()
            .filter(|a| !a.path.to_string_lossy().starts_with("aurora://"))
            .collect();
        assert!(
            !non_builtin.is_empty(),
            "Should discover at least some installed Windows applications"
        );
    }

    #[test]
    fn builtin_apps_have_aurora_scheme() {
        let catalog = AppCatalog::new();
        let builtins: Vec<_> = catalog
            .apps
            .iter()
            .filter(|a| a.path.to_string_lossy().starts_with("aurora://"))
            .collect();
        assert!(
            builtins.len() >= 10,
            "Should have at least 10 builtin apps, got {}",
            builtins.len()
        );
    }

    #[test]
    fn rescan_refreshes() {
        let mut catalog = AppCatalog::new();
        let count1 = catalog.apps.len();
        catalog.scan();
        let count2 = catalog.apps.len();
        // Should be the same (deterministic scan)
        assert_eq!(count1, count2);
        assert!(catalog.last_scan.is_some());
    }
}
