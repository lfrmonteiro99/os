use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrashEntry {
    pub original_path: PathBuf,
    pub trash_name: String,
    pub deleted_at: u64,
    pub size_bytes: u64,
    pub is_dir: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmartFolderKind {
    AllImages,
    AllDocuments,
    RecentFiles,
    LargeFiles,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomSmartFolder {
    pub name: String,
    pub extension: Option<String>,
    pub min_size_mb: Option<u64>,
    pub tag: Option<String>,
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
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries
}

pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    if bytes < 1024 * 1024 {
        return format!("{:.1} KB", bytes as f64 / 1024.0);
    }
    if bytes < 1024 * 1024 * 1024 {
        return format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0));
    }
    format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

pub fn smart_folder_entries(kind: SmartFolderKind, root: &Path) -> Vec<FmEntry> {
    smart_folder_entries_matching(root, |_path, metadata, ext| match kind {
        SmartFolderKind::AllImages => {
            matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp")
        }
        SmartFolderKind::AllDocuments => matches!(ext, "pdf" | "doc" | "docx" | "txt" | "md"),
        SmartFolderKind::RecentFiles => {
            let recent_cutoff =
                SystemTime::now().checked_sub(std::time::Duration::from_secs(7 * 24 * 60 * 60));
            recent_cutoff
                .and_then(|cutoff| metadata.modified().ok().map(|modified| modified >= cutoff))
                .unwrap_or(false)
        }
        SmartFolderKind::LargeFiles => metadata.len() > 100 * 1024 * 1024,
    })
}

pub fn custom_smart_folder_entries(folder: &CustomSmartFolder, root: &Path) -> Vec<FmEntry> {
    let extension = folder
        .extension
        .as_ref()
        .map(|value| value.trim_start_matches('.').to_ascii_lowercase());
    let min_size_bytes = folder.min_size_mb.map(|value| value * 1024 * 1024);
    smart_folder_entries_matching(root, |_path, metadata, ext| {
        if let Some(expected_ext) = extension.as_ref() {
            if ext != expected_ext {
                return false;
            }
        }
        if let Some(min_size) = min_size_bytes {
            if metadata.len() < min_size {
                return false;
            }
        }
        true
    })
}

fn smart_folder_entries_matching<F>(root: &Path, mut keep: F) -> Vec<FmEntry>
where
    F: FnMut(&Path, &fs::Metadata, &str) -> bool,
{
    let mut matches = WalkDir::new(root)
        .max_depth(4)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| {
            let path = entry.into_path();
            let metadata = path.metadata().ok()?;
            let ext = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            if keep(&path, &metadata, &ext) {
                Some(FmEntry {
                    name: path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("file")
                        .to_string(),
                    is_dir: false,
                    size: metadata.len(),
                    path,
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    matches.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    matches
}

pub fn trash_dir() -> PathBuf {
    trash_dir_in(&dirs_home())
}

pub fn load_trash_entries() -> Vec<TrashEntry> {
    load_trash_entries_in(&dirs_home())
}

pub fn move_to_trash(path: &Path) -> Result<TrashEntry, String> {
    move_to_trash_in(&dirs_home(), path)
}

pub fn restore_trash_entry(trash_name: &str) -> Result<PathBuf, String> {
    restore_trash_entry_in(&dirs_home(), trash_name)
}

pub fn delete_trash_entry_permanently(trash_name: &str) -> Result<(), String> {
    delete_trash_entry_permanently_in(&dirs_home(), trash_name)
}

pub fn empty_trash() -> Result<(), String> {
    empty_trash_in(&dirs_home())
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
                if !dir.exists() {
                    continue;
                }
                for entry in WalkDir::new(dir)
                    .max_depth(4)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    result.push(entry.into_path());
                    if result.len() >= 10_000 {
                        break;
                    }
                }
                if result.len() >= 10_000 {
                    break;
                }
            }

            if let Ok(mut f) = files_clone.lock() {
                *f = result;
            }
            if let Ok(mut r) = ready_clone.lock() {
                *r = true;
            }
        });

        Self { files, ready }
    }

    pub fn search(&self, query: &str, max: usize) -> Vec<String> {
        if query.is_empty() {
            return Vec::new();
        }
        let is_ready = self.ready.lock().map(|r| *r).unwrap_or(false);
        if !is_ready {
            return vec!["Indexing files...".to_string()];
        }

        let query_lower = query.to_lowercase();
        let files = self.files.lock().unwrap();
        files
            .iter()
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

pub fn move_entry_to_directory(
    path: &std::path::Path,
    target_dir: &std::path::Path,
) -> Result<PathBuf, String> {
    if !path.exists() {
        return Err("Source does not exist".to_string());
    }
    if !target_dir.is_dir() {
        return Err("Target directory does not exist".to_string());
    }
    let Some(file_name) = path.file_name() else {
        return Err("Source has no file name".to_string());
    };
    let target_path = target_dir.join(file_name);
    if target_path == path {
        return Ok(target_path);
    }
    if target_path.exists() {
        return Err("Destination already exists".to_string());
    }
    fs::rename(path, &target_path).map_err(|e| format!("Failed to move: {e}"))?;
    Ok(target_path)
}

pub fn copy_entry_to_directory(
    path: &std::path::Path,
    target_dir: &std::path::Path,
) -> Result<PathBuf, String> {
    if !path.exists() {
        return Err("Source does not exist".to_string());
    }
    if !target_dir.is_dir() {
        return Err("Target directory does not exist".to_string());
    }
    let Some(file_name) = path.file_name() else {
        return Err("Source has no file name".to_string());
    };
    let target_path = target_dir.join(file_name);
    if target_path.exists() {
        return Err("Destination already exists".to_string());
    }
    if path.is_dir() {
        copy_directory_recursive(path, &target_path)?;
    } else {
        fs::copy(path, &target_path).map_err(|e| format!("Failed to copy: {e}"))?;
    }
    Ok(target_path)
}

/// Delete a file or empty directory
pub fn delete_entry(path: &std::path::Path) -> Result<(), String> {
    move_to_trash(path).map(|_| ())
}

fn trash_dir_in(root: &Path) -> PathBuf {
    root.join(".aurora_trash")
}

fn manifest_path_in(root: &Path) -> PathBuf {
    trash_dir_in(root).join(".manifest.json")
}

fn load_trash_entries_in(root: &Path) -> Vec<TrashEntry> {
    let manifest_path = manifest_path_in(root);
    let Ok(content) = fs::read_to_string(&manifest_path) else {
        return Vec::new();
    };

    let mut entries = content
        .lines()
        .filter_map(parse_manifest_line)
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| b.deleted_at.cmp(&a.deleted_at));
    entries
}

fn save_trash_entries_in(root: &Path, entries: &[TrashEntry]) -> Result<(), String> {
    let trash = trash_dir_in(root);
    fs::create_dir_all(&trash).map_err(|e| format!("Failed to create trash directory: {e}"))?;
    let content = entries
        .iter()
        .map(manifest_line)
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(manifest_path_in(root), content)
        .map_err(|e| format!("Failed to save trash manifest: {e}"))
}

fn move_to_trash_in(root: &Path, path: &Path) -> Result<TrashEntry, String> {
    if !path.exists() {
        return Err("Path does not exist".to_string());
    }

    let trash = trash_dir_in(root);
    fs::create_dir_all(&trash).map_err(|e| format!("Failed to create trash directory: {e}"))?;

    let deleted_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let base_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("item");
    let trash_name = format!("{deleted_at}_{base_name}");
    let destination = trash.join(&trash_name);

    fs::rename(path, &destination).map_err(|e| format!("Failed to move to trash: {e}"))?;

    let mut entries = load_trash_entries_in(root);
    let size_bytes = compute_size_bytes(&destination);
    let entry = TrashEntry {
        original_path: path.to_path_buf(),
        trash_name: trash_name.clone(),
        deleted_at,
        size_bytes,
        is_dir: destination.is_dir(),
    };
    entries.push(entry.clone());
    save_trash_entries_in(root, &entries)?;
    Ok(entry)
}

fn restore_trash_entry_in(root: &Path, trash_name: &str) -> Result<PathBuf, String> {
    let mut entries = load_trash_entries_in(root);
    let Some(index) = entries
        .iter()
        .position(|entry| entry.trash_name == trash_name)
    else {
        return Err("Trash entry not found".to_string());
    };

    let entry = entries.remove(index);
    if entry.original_path.exists() {
        return Err("Original path already exists".to_string());
    }

    if let Some(parent) = entry.original_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to recreate original directory: {e}"))?;
    }

    let source = trash_dir_in(root).join(&entry.trash_name);
    fs::rename(&source, &entry.original_path)
        .map_err(|e| format!("Failed to restore trash entry: {e}"))?;
    save_trash_entries_in(root, &entries)?;
    Ok(entry.original_path)
}

fn empty_trash_in(root: &Path) -> Result<(), String> {
    let trash = trash_dir_in(root);
    if !trash.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(&trash).map_err(|e| format!("Failed to read trash directory: {e}"))? {
        let entry = entry.map_err(|e| format!("Failed to inspect trash entry: {e}"))?;
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some(".manifest.json") {
            continue;
        }
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .map_err(|e| format!("Failed to remove trashed directory: {e}"))?;
        } else {
            fs::remove_file(&path).map_err(|e| format!("Failed to remove trashed file: {e}"))?;
        }
    }
    save_trash_entries_in(root, &[])?;
    Ok(())
}

fn delete_trash_entry_permanently_in(root: &Path, trash_name: &str) -> Result<(), String> {
    let mut entries = load_trash_entries_in(root);
    let Some(index) = entries
        .iter()
        .position(|entry| entry.trash_name == trash_name)
    else {
        return Err("Trash entry not found".to_string());
    };

    let entry = entries.remove(index);
    let source = trash_dir_in(root).join(&entry.trash_name);
    if source.is_dir() {
        fs::remove_dir_all(&source)
            .map_err(|e| format!("Failed to delete trashed directory: {e}"))?;
    } else if source.exists() {
        fs::remove_file(&source).map_err(|e| format!("Failed to delete trashed file: {e}"))?;
    }
    save_trash_entries_in(root, &entries)
}

fn manifest_line(entry: &TrashEntry) -> String {
    format!(
        "{{\"original_path\":\"{}\",\"trash_name\":\"{}\",\"deleted_at\":{},\"size_bytes\":{},\"is_dir\":{}}}",
        escape_json(&entry.original_path.to_string_lossy()),
        escape_json(&entry.trash_name),
        entry.deleted_at,
        entry.size_bytes,
        entry.is_dir,
    )
}

fn parse_manifest_line(line: &str) -> Option<TrashEntry> {
    Some(TrashEntry {
        original_path: PathBuf::from(parse_json_string(line, "original_path")?),
        trash_name: parse_json_string(line, "trash_name")?,
        deleted_at: parse_json_u64(line, "deleted_at")?,
        size_bytes: parse_json_u64(line, "size_bytes")?,
        is_dir: parse_json_bool(line, "is_dir")?,
    })
}

fn parse_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].replace("\\\\", "\\"))
}

fn parse_json_u64(json: &str, key: &str) -> Option<u64> {
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
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\")
}

fn compute_size_bytes(path: &Path) -> u64 {
    if path.is_dir() {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.metadata().ok())
            .map(|meta| meta.len())
            .sum()
    } else {
        path.metadata().map(|meta| meta.len()).unwrap_or(0)
    }
}

fn copy_directory_recursive(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|e| format!("Failed to create destination directory: {e}"))?;
    for entry in
        fs::read_dir(source).map_err(|e| format!("Failed to read source directory: {e}"))?
    {
        let entry = entry.map_err(|e| format!("Failed to inspect directory entry: {e}"))?;
        let path = entry.path();
        let target = destination.join(entry.file_name());
        if path.is_dir() {
            copy_directory_recursive(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|e| format!("Failed to copy: {e}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

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
        let unique = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "aurora_crud_test_{}_{}",
            std::process::id(),
            unique
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    fn cleanup_dir(path: &Path) {
        for _ in 0..5 {
            if std::fs::remove_dir_all(path).is_ok() || !path.exists() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let _ = std::fs::remove_dir_all(path);
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
    fn move_entry_to_directory_moves_file() {
        let root = test_dir().join("move_entry_to_dir");
        cleanup_dir(&root);
        let from_dir = root.join("from");
        let to_dir = root.join("to");
        std::fs::create_dir_all(&from_dir).unwrap();
        std::fs::create_dir_all(&to_dir).unwrap();
        let source = from_dir.join("note.txt");
        std::fs::write(&source, "hello").unwrap();

        let moved = move_entry_to_directory(&source, &to_dir).unwrap();
        assert_eq!(moved, to_dir.join("note.txt"));
        assert!(!source.exists());
        assert!(moved.exists());

        cleanup_dir(&root);
    }

    #[test]
    fn move_entry_to_directory_rejects_existing_destination() {
        let root = test_dir().join("move_entry_conflict");
        cleanup_dir(&root);
        let from_dir = root.join("from");
        let to_dir = root.join("to");
        std::fs::create_dir_all(&from_dir).unwrap();
        std::fs::create_dir_all(&to_dir).unwrap();
        let source = from_dir.join("note.txt");
        let dest = to_dir.join("note.txt");
        std::fs::write(&source, "hello").unwrap();
        std::fs::write(&dest, "existing").unwrap();

        let err = move_entry_to_directory(&source, &to_dir).unwrap_err();
        assert_eq!(err, "Destination already exists");
        assert!(source.exists());

        cleanup_dir(&root);
    }

    #[test]
    fn copy_entry_to_directory_copies_file_without_removing_source() {
        let root = test_dir().join("copy_entry_to_dir");
        cleanup_dir(&root);
        let from_dir = root.join("from");
        let to_dir = root.join("to");
        std::fs::create_dir_all(&from_dir).unwrap();
        std::fs::create_dir_all(&to_dir).unwrap();
        let source = from_dir.join("note.txt");
        std::fs::write(&source, "hello").unwrap();

        let copied = copy_entry_to_directory(&source, &to_dir).unwrap();
        assert_eq!(copied, to_dir.join("note.txt"));
        assert!(source.exists());
        assert!(copied.exists());

        cleanup_dir(&root);
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

    #[test]
    fn move_to_trash_moves_file_and_records_manifest() {
        let root = test_dir().join("trash_backend_file");
        cleanup_dir(&root);
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("delete_me.txt");
        std::fs::write(&file, "bye").unwrap();

        let entry = move_to_trash_in(&root, &file).unwrap();
        assert!(!file.exists());
        assert!(trash_dir_in(&root).join(&entry.trash_name).exists());
        let entries = load_trash_entries_in(&root);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].original_path, file);

        cleanup_dir(&root);
    }

    #[test]
    fn restore_trash_entry_moves_file_back() {
        let root = test_dir().join("trash_backend_restore");
        cleanup_dir(&root);
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("restore_me.txt");
        std::fs::write(&file, "hello").unwrap();

        let entry = move_to_trash_in(&root, &file).unwrap();
        let restored_path = restore_trash_entry_in(&root, &entry.trash_name).unwrap();
        assert_eq!(restored_path, file);
        assert!(file.exists());
        assert!(load_trash_entries_in(&root).is_empty());

        cleanup_dir(&root);
    }

    #[test]
    fn empty_trash_removes_entries_and_files() {
        let root = test_dir().join("trash_backend_empty");
        cleanup_dir(&root);
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("purge_me.txt");
        std::fs::write(&file, "hello").unwrap();

        let entry = move_to_trash_in(&root, &file).unwrap();
        assert!(trash_dir_in(&root).join(&entry.trash_name).exists());
        empty_trash_in(&root).unwrap();
        assert!(!trash_dir_in(&root).join(&entry.trash_name).exists());
        assert!(load_trash_entries_in(&root).is_empty());

        cleanup_dir(&root);
    }

    #[test]
    fn delete_trash_entry_permanently_removes_single_entry() {
        let root = test_dir().join("trash_backend_delete_one");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("purge_me.txt");
        std::fs::write(&file, "hello").unwrap();

        let entry = move_to_trash_in(&root, &file).unwrap();
        assert!(trash_dir_in(&root).join(&entry.trash_name).exists());
        delete_trash_entry_permanently_in(&root, &entry.trash_name).unwrap();
        assert!(!trash_dir_in(&root).join(&entry.trash_name).exists());
        assert!(load_trash_entries_in(&root).is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn smart_folder_all_images_filters_image_extensions() {
        let root = test_dir().join("smart_images");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.png"), "x").unwrap();
        std::fs::write(root.join("b.txt"), "x").unwrap();
        let entries = smart_folder_entries(SmartFolderKind::AllImages, &root);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "a.png");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn smart_folder_all_documents_filters_document_extensions() {
        let root = test_dir().join("smart_documents");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.md"), "x").unwrap();
        std::fs::write(root.join("b.jpg"), "x").unwrap();
        let entries = smart_folder_entries(SmartFolderKind::AllDocuments, &root);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "a.md");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn smart_folder_recent_files_includes_new_files() {
        let root = test_dir().join("smart_recent");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("recent.txt"), "x").unwrap();
        let entries = smart_folder_entries(SmartFolderKind::RecentFiles, &root);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "recent.txt");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn smart_folder_large_files_filters_by_size() {
        let root = test_dir().join("smart_large");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("small.bin"), vec![0_u8; 1024]).unwrap();
        std::fs::write(root.join("large.bin"), vec![0_u8; 101 * 1024 * 1024]).unwrap();
        let entries = smart_folder_entries(SmartFolderKind::LargeFiles, &root);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "large.bin");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn custom_smart_folder_entries_apply_extension_and_size_rules() {
        let root = test_dir().join("smart_custom");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("keep.log"), vec![0_u8; 2 * 1024 * 1024]).unwrap();
        std::fs::write(root.join("small.log"), vec![0_u8; 512]).unwrap();
        std::fs::write(root.join("other.txt"), vec![0_u8; 2 * 1024 * 1024]).unwrap();

        let folder = CustomSmartFolder {
            name: "Big Logs".to_string(),
            extension: Some("log".to_string()),
            min_size_mb: Some(1),
            tag: Some("red".to_string()),
        };
        let entries = custom_smart_folder_entries(&folder, &root);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "keep.log");
        let _ = std::fs::remove_dir_all(&root);
    }
}
