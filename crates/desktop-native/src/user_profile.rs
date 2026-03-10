use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserProfile {
    pub username: String,
    pub display_name: String,
    pub avatar_r: u8,
    pub avatar_g: u8,
    pub avatar_b: u8,
    pub avatar_initials: String,
    pub created_at: u64,
    pub is_admin: bool,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            username: String::new(),
            display_name: String::new(),
            avatar_r: 0,
            avatar_g: 122,
            avatar_b: 255,
            avatar_initials: String::new(),
            created_at: 0,
            is_admin: true,
        }
    }
}

impl UserProfile {
    pub fn profile_path() -> PathBuf {
        let home = if cfg!(windows) {
            std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into())
        } else {
            std::env::var("HOME").unwrap_or_else(|_| ".".into())
        };
        PathBuf::from(home).join(".aurora_profile.json")
    }

    pub fn from_display_name(display_name: &str, avatar_rgb: (u8, u8, u8)) -> Self {
        let trimmed = display_name.trim();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            username: slugify(trimmed),
            display_name: trimmed.to_string(),
            avatar_r: avatar_rgb.0,
            avatar_g: avatar_rgb.1,
            avatar_b: avatar_rgb.2,
            avatar_initials: derive_initials(trimmed),
            created_at: now,
            is_admin: true,
        }
    }

    pub fn load() -> Option<Self> {
        let content = std::fs::read_to_string(Self::profile_path()).ok()?;
        Some(Self::from_json(&content))
    }

    pub fn save(&self) -> Result<(), String> {
        std::fs::write(Self::profile_path(), self.to_json()).map_err(|e| e.to_string())
    }

    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"username\": \"{}\",\n",
                "  \"display_name\": \"{}\",\n",
                "  \"avatar_r\": {},\n",
                "  \"avatar_g\": {},\n",
                "  \"avatar_b\": {},\n",
                "  \"avatar_initials\": \"{}\",\n",
                "  \"created_at\": {},\n",
                "  \"is_admin\": {}\n",
                "}}"
            ),
            self.username,
            self.display_name,
            self.avatar_r,
            self.avatar_g,
            self.avatar_b,
            self.avatar_initials,
            self.created_at,
            self.is_admin,
        )
    }

    pub fn from_json(json: &str) -> Self {
        let mut profile = Self::default();
        if let Some(v) = parse_json_string(json, "username") {
            profile.username = v;
        }
        if let Some(v) = parse_json_string(json, "display_name") {
            profile.display_name = v;
        }
        if let Some(v) = parse_json_usize(json, "avatar_r") {
            profile.avatar_r = v as u8;
        }
        if let Some(v) = parse_json_usize(json, "avatar_g") {
            profile.avatar_g = v as u8;
        }
        if let Some(v) = parse_json_usize(json, "avatar_b") {
            profile.avatar_b = v as u8;
        }
        if let Some(v) = parse_json_string(json, "avatar_initials") {
            profile.avatar_initials = v;
        }
        if let Some(v) = parse_json_usize(json, "created_at") {
            profile.created_at = v as u64;
        }
        if let Some(v) = parse_json_bool(json, "is_admin") {
            profile.is_admin = v;
        }
        if profile.avatar_initials.is_empty() && !profile.display_name.is_empty() {
            profile.avatar_initials = derive_initials(&profile.display_name);
        }
        if profile.username.is_empty() && !profile.display_name.is_empty() {
            profile.username = slugify(&profile.display_name);
        }
        profile
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, self.to_json()).map_err(|e| e.to_string())
    }

    pub fn load_from_path(path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        Some(Self::from_json(&content))
    }
}

pub fn derive_initials(name: &str) -> String {
    let parts: Vec<_> = name
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .collect();
    match parts.as_slice() {
        [] => String::new(),
        [single] => single.chars().take(2).collect::<String>().to_uppercase(),
        [first, second, ..] => {
            let first_char = first.chars().next().unwrap_or(' ');
            let second_char = second.chars().next().unwrap_or(' ');
            format!("{first_char}{second_char}").to_uppercase()
        }
    }
}

fn slugify(name: &str) -> String {
    let slug = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    slug.split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
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

    #[test]
    fn derive_initials_handles_empty() {
        assert_eq!(derive_initials(""), "");
    }

    #[test]
    fn derive_initials_uses_two_letters_for_single_word() {
        assert_eq!(derive_initials("aurora"), "AU");
    }

    #[test]
    fn derive_initials_uses_first_letters_for_multiple_words() {
        assert_eq!(derive_initials("Aurora User"), "AU");
    }

    #[test]
    fn from_display_name_derives_profile_fields() {
        let profile = UserProfile::from_display_name("Aurora User", (10, 20, 30));
        assert_eq!(profile.username, "aurora-user");
        assert_eq!(profile.display_name, "Aurora User");
        assert_eq!(profile.avatar_initials, "AU");
        assert_eq!(
            (profile.avatar_r, profile.avatar_g, profile.avatar_b),
            (10, 20, 30)
        );
        assert!(profile.created_at > 0);
        assert!(profile.is_admin);
    }

    #[test]
    fn json_roundtrip_preserves_profile() {
        let original = UserProfile::from_display_name("Aurora User", (10, 20, 30));
        let parsed = UserProfile::from_json(&original.to_json());
        assert_eq!(parsed.username, original.username);
        assert_eq!(parsed.display_name, original.display_name);
        assert_eq!(parsed.avatar_initials, original.avatar_initials);
        assert_eq!(
            (parsed.avatar_r, parsed.avatar_g, parsed.avatar_b),
            (10, 20, 30)
        );
    }

    #[test]
    fn save_and_load_from_path_roundtrip() {
        let dir = std::env::temp_dir().join("aurora_profile_tests");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(".aurora_profile_test.json");
        let profile = UserProfile::from_display_name("Aurora User", (44, 55, 66));
        profile.save_to_path(&path).unwrap();
        let loaded = UserProfile::load_from_path(&path).unwrap();
        assert_eq!(loaded.display_name, "Aurora User");
        assert_eq!(loaded.avatar_initials, "AU");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
