use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewKind {
    Text,
    Folder,
    Image,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewData {
    pub path: PathBuf,
    pub title: String,
    pub subtitle: String,
    pub body: String,
    pub kind: PreviewKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileInfo {
    pub name: String,
    pub kind: String,
    pub size_label: String,
    pub location: String,
    pub modified_label: String,
}

pub fn build_preview(path: &Path) -> PreviewData {
    let title = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Item")
        .to_string();

    if path.is_dir() {
        let count = fs::read_dir(path)
            .map(|rd| rd.filter_map(|e| e.ok()).count())
            .unwrap_or(0);
        return PreviewData {
            path: path.to_path_buf(),
            title,
            subtitle: "Folder".to_string(),
            body: format!("{count} items"),
            kind: PreviewKind::Folder,
        };
    }

    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if is_text_extension(&ext) {
        let text =
            fs::read_to_string(path).unwrap_or_else(|_| "Unable to preview text file.".to_string());
        let lines = text.lines().take(100).collect::<Vec<_>>();
        let body = if lines.is_empty() {
            "(empty file)".to_string()
        } else {
            lines.join("\n")
        };
        return PreviewData {
            path: path.to_path_buf(),
            title,
            subtitle: ext_label(&ext, "Text File"),
            body,
            kind: PreviewKind::Text,
        };
    }

    if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp") {
        let size = path.metadata().map(|meta| meta.len()).unwrap_or(0);
        return PreviewData {
            path: path.to_path_buf(),
            title,
            subtitle: ext_label(&ext, "Image"),
            body: format!("Image preview unavailable in this build.\nSize: {size} bytes"),
            kind: PreviewKind::Image,
        };
    }

    let size = path.metadata().map(|meta| meta.len()).unwrap_or(0);
    PreviewData {
        path: path.to_path_buf(),
        title,
        subtitle: ext_label(&ext, "File"),
        body: format!("No preview available.\nSize: {size} bytes"),
        kind: PreviewKind::Unknown,
    }
}

pub fn read_file_info(path: &Path) -> FileInfo {
    let metadata = path.metadata().ok();
    let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
    let modified_label = metadata
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| format!("Modified {}", duration.as_secs()))
        .unwrap_or_else(|| "Modified unknown".to_string());

    FileInfo {
        name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Item")
            .to_string(),
        kind: if is_dir {
            "Folder".to_string()
        } else {
            let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
            ext_label(&ext.to_ascii_lowercase(), "File")
        },
        size_label: if is_dir {
            "Size unknown".to_string()
        } else {
            format!("{size} bytes")
        },
        location: path.parent().unwrap_or(path).to_string_lossy().to_string(),
        modified_label,
    }
}

pub fn move_preview_index(current: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let current = current.min(len - 1) as isize;
    (current + delta).clamp(0, len as isize - 1) as usize
}

fn ext_label(ext: &str, fallback: &str) -> String {
    if ext.is_empty() {
        fallback.to_string()
    } else {
        format!("{}.{}", fallback, ext)
    }
}

fn is_text_extension(ext: &str) -> bool {
    matches!(
        ext,
        "rs" | "py"
            | "js"
            | "ts"
            | "c"
            | "cpp"
            | "h"
            | "go"
            | "java"
            | "md"
            | "txt"
            | "log"
            | "csv"
            | "json"
            | "toml"
            | "yaml"
            | "yml"
            | "html"
            | "css"
            | "xml"
            | "sh"
            | "bat"
            | "cmd"
            | "ps1"
            | "cfg"
            | "ini"
            | "conf"
            | "env"
            | "sql"
            | "lua"
            | "rb"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir() -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("aurora_quick_look_tests_{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn text_preview_reads_first_lines() {
        let dir = test_dir();
        let path = dir.join("note.txt");
        std::fs::write(&path, "hello\nworld\npreview").unwrap();
        let preview = build_preview(&path);
        assert_eq!(preview.kind, PreviewKind::Text);
        assert!(preview.body.contains("hello"));
        assert!(preview.body.contains("world"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn folder_preview_counts_items() {
        let dir = test_dir();
        let folder = dir.join("Folder");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("a.txt"), "a").unwrap();
        std::fs::write(folder.join("b.txt"), "b").unwrap();
        let preview = build_preview(&folder);
        assert_eq!(preview.kind, PreviewKind::Folder);
        assert!(preview.body.contains("2 items"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn image_preview_uses_image_kind() {
        let dir = test_dir();
        let path = dir.join("photo.png");
        std::fs::write(&path, [1_u8, 2, 3, 4]).unwrap();
        let preview = build_preview(&path);
        assert_eq!(preview.kind, PreviewKind::Image);
        assert!(preview.body.contains("Image preview unavailable"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn preview_navigation_clamps_within_bounds() {
        assert_eq!(move_preview_index(0, -1, 3), 0);
        assert_eq!(move_preview_index(1, 1, 3), 2);
        assert_eq!(move_preview_index(2, 1, 3), 2);
    }

    #[test]
    fn read_file_info_reports_file_metadata() {
        let dir = test_dir();
        let path = dir.join("note.txt");
        std::fs::write(&path, "hello").unwrap();
        let info = read_file_info(&path);
        assert_eq!(info.name, "note.txt");
        assert!(info.kind.contains(".txt"));
        assert_eq!(info.location, dir.to_string_lossy());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_file_info_reports_folder_metadata() {
        let dir = test_dir();
        let folder = dir.join("Projects");
        std::fs::create_dir_all(&folder).unwrap();
        let info = read_file_info(&folder);
        assert_eq!(info.name, "Projects");
        assert_eq!(info.kind, "Folder");
        assert_eq!(info.location, dir.to_string_lossy());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
