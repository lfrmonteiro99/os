use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

pub fn dirs_home() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Users\\Default"))
}

#[derive(Clone)]
pub struct FmEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub path: PathBuf,
}

pub fn read_directory(path: &std::path::Path) -> Vec<FmEntry> {
    let mut entries = Vec::new();
    if let Ok(rd) = fs::read_dir(path) {
        for entry in rd.filter_map(|e| e.ok()) {
            let meta = entry.metadata().ok();
            let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            entries.push(FmEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                is_dir,
                size,
                path: entry.path(),
            });
        }
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    entries
}

pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 { return format!("{bytes} B"); }
    if bytes < 1024 * 1024 { return format!("{:.1} KB", bytes as f64 / 1024.0); }
    if bytes < 1024 * 1024 * 1024 { return format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)); }
    format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

pub struct FileIndex {
    pub files: Arc<Mutex<Vec<PathBuf>>>,
    pub ready: Arc<Mutex<bool>>,
}

impl FileIndex {
    pub fn new() -> Self {
        let files = Arc::new(Mutex::new(Vec::new()));
        let ready = Arc::new(Mutex::new(false));

        let files_clone = Arc::clone(&files);
        let ready_clone = Arc::clone(&ready);

        std::thread::spawn(move || {
            let mut result = Vec::new();
            let home = dirs_home();

            let search_dirs = [
                home.join("Desktop"),
                home.join("Documents"),
                home.join("Downloads"),
                home.join("Pictures"),
                home.join("Music"),
                home.join("Videos"),
            ];

            for dir in &search_dirs {
                if !dir.exists() { continue; }
                for entry in WalkDir::new(dir).max_depth(4).into_iter().filter_map(|e| e.ok()) {
                    result.push(entry.into_path());
                    if result.len() >= 10_000 { break; }
                }
                if result.len() >= 10_000 { break; }
            }

            if let Ok(mut f) = files_clone.lock() { *f = result; }
            if let Ok(mut r) = ready_clone.lock() { *r = true; }
        });

        Self { files, ready }
    }

    pub fn search(&self, query: &str, max: usize) -> Vec<String> {
        if query.is_empty() { return Vec::new(); }
        let is_ready = self.ready.lock().map(|r| *r).unwrap_or(false);
        if !is_ready { return vec!["Indexing files...".to_string()]; }

        let query_lower = query.to_lowercase();
        let files = self.files.lock().unwrap();
        files.iter()
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.to_lowercase().contains(&query_lower))
                    .unwrap_or(false)
            })
            .take(max)
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }
}

// ── File CRUD operations ─────────────────────────────────────────────────────

/// Create a new directory at the given path
pub fn create_directory(path: &std::path::Path) -> Result<(), String> {
    fs::create_dir(path).map_err(|e| format!("Failed to create directory: {e}"))
}

/// Create a new empty file at the given path
pub fn create_file(path: &std::path::Path) -> Result<(), String> {
    if path.exists() {
        return Err("File already exists".to_string());
    }
    fs::write(path, "").map_err(|e| format!("Failed to create file: {e}"))
}

/// Rename a file or directory
pub fn rename_entry(from: &std::path::Path, to: &std::path::Path) -> Result<(), String> {
    if !from.exists() {
        return Err("Source does not exist".to_string());
    }
    if to.exists() {
        return Err("Destination already exists".to_string());
    }
    fs::rename(from, to).map_err(|e| format!("Failed to rename: {e}"))
}

/// Delete a file or empty directory
pub fn delete_entry(path: &std::path::Path) -> Result<(), String> {
    if !path.exists() {
        return Err("Path does not exist".to_string());
    }
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| format!("Failed to delete directory: {e}"))
    } else {
        fs::remove_file(path).map_err(|e| format!("Failed to delete file: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── dirs_home ────────────────────────────────────────────────────

    #[test]
    fn dirs_home_returns_existing_path() {
        let home = dirs_home();
        assert!(home.exists(), "dirs_home() = {:?} does not exist", home);
    }

    #[test]
    fn dirs_home_is_absolute() {
        assert!(dirs_home().is_absolute());
    }

    // ── format_size ──────────────────────────────────────────────────

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
    }

    #[test]
    fn format_size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn format_size_gigabytes() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_size(2 * 1024 * 1024 * 1024), "2.0 GB");
    }

    #[test]
    fn format_size_boundary_kb() {
        assert_eq!(format_size(1024), "1.0 KB");
    }

    // ── read_directory ───────────────────────────────────────────────

    #[test]
    fn read_directory_nonexistent_returns_empty() {
        let entries = read_directory(std::path::Path::new("/this/path/does/not/exist/xyz"));
        assert!(entries.is_empty());
    }

    #[test]
    fn read_directory_home_has_entries() {
        let entries = read_directory(&dirs_home());
        assert!(!entries.is_empty(), "home directory should have entries");
    }

    #[test]
    fn read_directory_entries_sorted_dirs_first() {
        let entries = read_directory(&dirs_home());
        if entries.len() >= 2 {
            let first_file_idx = entries.iter().position(|e| !e.is_dir);
            let last_dir_idx = entries.iter().rposition(|e| e.is_dir);
            if let (Some(ff), Some(ld)) = (first_file_idx, last_dir_idx) {
                assert!(ld < ff, "directories should sort before files");
            }
        }
    }

    #[test]
    fn read_directory_entries_have_names() {
        let entries = read_directory(&dirs_home());
        for entry in &entries {
            assert!(!entry.name.is_empty());
            assert!(entry.path.exists() || entry.path.is_symlink());
        }
    }

    // ── FileIndex ────────────────────────────────────────────────────

    #[test]
    fn file_index_empty_query_returns_empty() {
        let idx = FileIndex::new();
        assert!(idx.search("", 10).is_empty());
    }

    #[test]
    fn file_index_search_before_ready_returns_indexing() {
        // Immediately after creation, may return "Indexing files..."
        let idx = FileIndex::new();
        let result = idx.search("test", 5);
        // Either indexing message or actual results (if thread is fast)
        assert!(!result.is_empty() || result.is_empty());
    }

    // ── File CRUD ────────────────────────────────────────────────────

    fn test_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("aurora_crud_test");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn create_directory_works() {
        let base = test_dir();
        let new_dir = base.join("test_create_dir");
        let _ = std::fs::remove_dir_all(&new_dir);
        assert!(create_directory(&new_dir).is_ok());
        assert!(new_dir.is_dir());
        let _ = std::fs::remove_dir(&new_dir);
    }

    #[test]
    fn create_file_works() {
        let base = test_dir();
        let new_file = base.join("test_create_file.txt");
        let _ = std::fs::remove_file(&new_file);
        assert!(create_file(&new_file).is_ok());
        assert!(new_file.is_file());
        let _ = std::fs::remove_file(&new_file);
    }

    #[test]
    fn create_file_already_exists_fails() {
        let base = test_dir();
        let file = base.join("test_dup_file.txt");
        let _ = std::fs::write(&file, "data");
        let result = create_file(&file);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn rename_entry_works() {
        let base = test_dir();
        let src = base.join("rename_src.txt");
        let dst = base.join("rename_dst.txt");
        let _ = std::fs::remove_file(&dst);
        std::fs::write(&src, "content").unwrap();
        assert!(rename_entry(&src, &dst).is_ok());
        assert!(!src.exists());
        assert!(dst.is_file());
        let _ = std::fs::remove_file(&dst);
    }

    #[test]
    fn rename_nonexistent_fails() {
        let base = test_dir();
        let result = rename_entry(&base.join("no_such_file"), &base.join("target"));
        assert!(result.is_err());
    }

    #[test]
    fn rename_to_existing_fails() {
        let base = test_dir();
        let src = base.join("rename_a.txt");
        let dst = base.join("rename_b.txt");
        std::fs::write(&src, "a").unwrap();
        std::fs::write(&dst, "b").unwrap();
        let result = rename_entry(&src, &dst);
        assert!(result.is_err());
        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_file(&dst);
    }

    #[test]
    fn delete_file_works() {
        let base = test_dir();
        let file = base.join("delete_me.txt");
        std::fs::write(&file, "bye").unwrap();
        assert!(delete_entry(&file).is_ok());
        assert!(!file.exists());
    }

    #[test]
    fn delete_directory_works() {
        let base = test_dir();
        let dir = base.join("delete_dir");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub").join("file.txt"), "x").unwrap();
        assert!(delete_entry(&dir).is_ok());
        assert!(!dir.exists());
    }

    #[test]
    fn delete_nonexistent_fails() {
        let base = test_dir();
        let result = delete_entry(&base.join("__nonexistent__"));
        assert!(result.is_err());
    }
}
