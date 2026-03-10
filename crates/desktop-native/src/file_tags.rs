use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::file_index::FmEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagColor {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Gray,
}

impl TagColor {
    pub const ALL: [Self; 7] = [
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::Blue,
        Self::Purple,
        Self::Gray,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Orange => "orange",
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Gray => "gray",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "red" => Some(Self::Red),
            "orange" => Some(Self::Orange),
            "yellow" => Some(Self::Yellow),
            "green" => Some(Self::Green),
            "blue" => Some(Self::Blue),
            "purple" => Some(Self::Purple),
            "gray" => Some(Self::Gray),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct FileTags {
    tags: BTreeMap<PathBuf, Vec<TagColor>>,
}

impl FileTags {
    pub fn path() -> PathBuf {
        let home = if cfg!(windows) {
            std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into())
        } else {
            std::env::var("HOME").unwrap_or_else(|_| ".".into())
        };
        PathBuf::from(home).join(".aurora_tags.json")
    }

    pub fn load() -> Self {
        let Ok(content) = std::fs::read_to_string(Self::path()) else {
            return Self::default();
        };
        Self::from_json(&content)
    }

    pub fn save(&self) -> Result<(), String> {
        std::fs::write(Self::path(), self.to_json()).map_err(|e| e.to_string())
    }

    pub fn assign(&mut self, path: &Path, color: TagColor) {
        let tags = self.tags.entry(path.to_path_buf()).or_default();
        if !tags.contains(&color) {
            tags.push(color);
        }
    }

    pub fn clear(&mut self, path: &Path) {
        self.tags.remove(path);
    }

    pub fn get(&self, path: &Path) -> &[TagColor] {
        self.tags.get(path).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn entries_with_tag(&self, color: TagColor, root: &Path) -> Vec<FmEntry> {
        self.entries_with_tags(&[color], false, root)
    }

    pub fn entries_with_tags(
        &self,
        colors: &[TagColor],
        match_all: bool,
        root: &Path,
    ) -> Vec<FmEntry> {
        if colors.is_empty() {
            return Vec::new();
        }
        let mut entries = self
            .tags
            .iter()
            .filter(|(path, tags)| {
                path.starts_with(root)
                    && if match_all {
                        colors.iter().all(|color| tags.contains(color))
                    } else {
                        colors.iter().any(|color| tags.contains(color))
                    }
            })
            .filter_map(|(path, _)| {
                let metadata = path.metadata().ok()?;
                if !metadata.is_file() {
                    return None;
                }
                Some(FmEntry {
                    name: path.file_name()?.to_string_lossy().to_string(),
                    is_dir: false,
                    size: metadata.len(),
                    path: path.clone(),
                })
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        entries
    }

    pub fn to_json(&self) -> String {
        self.tags
            .iter()
            .map(|(path, tags)| {
                let tag_list = tags
                    .iter()
                    .map(|tag| tag.as_str())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{}|{}", path.to_string_lossy(), tag_list)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn from_json(content: &str) -> Self {
        let mut tags = BTreeMap::new();
        for line in content.lines() {
            let mut parts = line.splitn(2, '|');
            let Some(path) = parts.next() else { continue };
            let Some(tag_blob) = parts.next() else {
                continue;
            };
            let parsed = tag_blob
                .split(',')
                .filter_map(TagColor::from_str)
                .collect::<Vec<_>>();
            if !parsed.is_empty() {
                tags.insert(PathBuf::from(path), parsed);
            }
        }
        Self { tags }
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
    fn assign_adds_tag_once() {
        let path = PathBuf::from("C:/test/file.txt");
        let mut tags = FileTags::default();
        tags.assign(&path, TagColor::Red);
        tags.assign(&path, TagColor::Red);
        assert_eq!(tags.get(&path), &[TagColor::Red]);
    }

    #[test]
    fn clear_removes_tags() {
        let path = PathBuf::from("C:/test/file.txt");
        let mut tags = FileTags::default();
        tags.assign(&path, TagColor::Blue);
        tags.clear(&path);
        assert!(tags.get(&path).is_empty());
    }

    #[test]
    fn json_roundtrip_preserves_tags() {
        let path = PathBuf::from("C:/test/file.txt");
        let mut tags = FileTags::default();
        tags.assign(&path, TagColor::Red);
        tags.assign(&path, TagColor::Blue);
        let parsed = FileTags::from_json(&tags.to_json());
        assert_eq!(parsed.get(&path), &[TagColor::Red, TagColor::Blue]);
    }

    #[test]
    fn entries_with_tag_filters_existing_files_under_root() {
        let root = unique_temp_dir("aurora_tag_entries");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("nested")).unwrap();

        let keep = root.join("nested").join("b.txt");
        let keep2 = root.join("a.txt");
        let skip = root.join("skip.txt");
        std::fs::write(&keep, "b").unwrap();
        std::fs::write(&keep2, "a").unwrap();
        std::fs::write(&skip, "x").unwrap();

        let mut tags = FileTags::default();
        tags.assign(&keep, TagColor::Blue);
        tags.assign(&keep2, TagColor::Blue);
        tags.assign(&skip, TagColor::Red);

        let entries = tags.entries_with_tag(TagColor::Blue, &root);
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["a.txt", "b.txt"]
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn entries_with_tags_support_any_and_all_matching() {
        let root = unique_temp_dir("aurora_tag_entries_multi");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let both = root.join("both.txt");
        let red_only = root.join("red.txt");
        let blue_only = root.join("blue.txt");
        std::fs::write(&both, "x").unwrap();
        std::fs::write(&red_only, "x").unwrap();
        std::fs::write(&blue_only, "x").unwrap();

        let mut tags = FileTags::default();
        tags.assign(&both, TagColor::Red);
        tags.assign(&both, TagColor::Blue);
        tags.assign(&red_only, TagColor::Red);
        tags.assign(&blue_only, TagColor::Blue);

        let any_entries = tags.entries_with_tags(&[TagColor::Red, TagColor::Blue], false, &root);
        assert_eq!(any_entries.len(), 3);

        let all_entries = tags.entries_with_tags(&[TagColor::Red, TagColor::Blue], true, &root);
        assert_eq!(all_entries.len(), 1);
        assert_eq!(all_entries[0].name, "both.txt");

        let _ = std::fs::remove_dir_all(&root);
    }
}
