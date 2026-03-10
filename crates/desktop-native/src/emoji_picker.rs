#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmojiCategory {
    Smileys,
    People,
    Animals,
    Food,
    Travel,
    Activities,
    Objects,
    Symbols,
    SpecialChars,
}

impl EmojiCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Smileys => "Smileys",
            Self::People => "People",
            Self::Animals => "Animals",
            Self::Food => "Food",
            Self::Travel => "Travel",
            Self::Activities => "Activities",
            Self::Objects => "Objects",
            Self::Symbols => "Symbols",
            Self::SpecialChars => "Chars",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Smileys => "😀",
            Self::People => "👍",
            Self::Animals => "🐱",
            Self::Food => "🍎",
            Self::Travel => "🚀",
            Self::Activities => "⚽",
            Self::Objects => "💡",
            Self::Symbols => "∞",
            Self::SpecialChars => "æ",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Smileys,
            Self::People,
            Self::Animals,
            Self::Food,
            Self::Travel,
            Self::Activities,
            Self::Objects,
            Self::Symbols,
            Self::SpecialChars,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmojiEntry {
    pub symbol: &'static str,
    pub name: &'static str,
    pub category: EmojiCategory,
}

const EMOJI_ENTRIES: &[EmojiEntry] = &[
    EmojiEntry {
        symbol: "😀",
        name: "grinning face",
        category: EmojiCategory::Smileys,
    },
    EmojiEntry {
        symbol: "😂",
        name: "face with tears of joy",
        category: EmojiCategory::Smileys,
    },
    EmojiEntry {
        symbol: "😍",
        name: "smiling heart eyes",
        category: EmojiCategory::Smileys,
    },
    EmojiEntry {
        symbol: "😎",
        name: "cool sunglasses",
        category: EmojiCategory::Smileys,
    },
    EmojiEntry {
        symbol: "🤔",
        name: "thinking face",
        category: EmojiCategory::Smileys,
    },
    EmojiEntry {
        symbol: "👍",
        name: "thumbs up",
        category: EmojiCategory::People,
    },
    EmojiEntry {
        symbol: "👏",
        name: "clapping hands",
        category: EmojiCategory::People,
    },
    EmojiEntry {
        symbol: "🙏",
        name: "folded hands",
        category: EmojiCategory::People,
    },
    EmojiEntry {
        symbol: "🧑‍💻",
        name: "technologist",
        category: EmojiCategory::People,
    },
    EmojiEntry {
        symbol: "🐱",
        name: "cat face",
        category: EmojiCategory::Animals,
    },
    EmojiEntry {
        symbol: "🐶",
        name: "dog face",
        category: EmojiCategory::Animals,
    },
    EmojiEntry {
        symbol: "🦊",
        name: "fox face",
        category: EmojiCategory::Animals,
    },
    EmojiEntry {
        symbol: "🐼",
        name: "panda",
        category: EmojiCategory::Animals,
    },
    EmojiEntry {
        symbol: "🍎",
        name: "red apple",
        category: EmojiCategory::Food,
    },
    EmojiEntry {
        symbol: "🍕",
        name: "pizza",
        category: EmojiCategory::Food,
    },
    EmojiEntry {
        symbol: "☕",
        name: "hot coffee",
        category: EmojiCategory::Food,
    },
    EmojiEntry {
        symbol: "🍰",
        name: "cake",
        category: EmojiCategory::Food,
    },
    EmojiEntry {
        symbol: "🚀",
        name: "rocket",
        category: EmojiCategory::Travel,
    },
    EmojiEntry {
        symbol: "✈️",
        name: "airplane",
        category: EmojiCategory::Travel,
    },
    EmojiEntry {
        symbol: "🚗",
        name: "car",
        category: EmojiCategory::Travel,
    },
    EmojiEntry {
        symbol: "🗺️",
        name: "world map",
        category: EmojiCategory::Travel,
    },
    EmojiEntry {
        symbol: "⚽",
        name: "soccer ball",
        category: EmojiCategory::Activities,
    },
    EmojiEntry {
        symbol: "🎮",
        name: "game controller",
        category: EmojiCategory::Activities,
    },
    EmojiEntry {
        symbol: "🎵",
        name: "music note",
        category: EmojiCategory::Activities,
    },
    EmojiEntry {
        symbol: "🎯",
        name: "dart target",
        category: EmojiCategory::Activities,
    },
    EmojiEntry {
        symbol: "💡",
        name: "light bulb",
        category: EmojiCategory::Objects,
    },
    EmojiEntry {
        symbol: "📌",
        name: "push pin",
        category: EmojiCategory::Objects,
    },
    EmojiEntry {
        symbol: "🔒",
        name: "lock",
        category: EmojiCategory::Objects,
    },
    EmojiEntry {
        symbol: "💻",
        name: "laptop",
        category: EmojiCategory::Objects,
    },
    EmojiEntry {
        symbol: "∞",
        name: "infinity",
        category: EmojiCategory::Symbols,
    },
    EmojiEntry {
        symbol: "→",
        name: "right arrow",
        category: EmojiCategory::Symbols,
    },
    EmojiEntry {
        symbol: "€",
        name: "euro sign",
        category: EmojiCategory::Symbols,
    },
    EmojiEntry {
        symbol: "™",
        name: "trademark",
        category: EmojiCategory::Symbols,
    },
    EmojiEntry {
        symbol: "é",
        name: "e acute",
        category: EmojiCategory::SpecialChars,
    },
    EmojiEntry {
        symbol: "à",
        name: "a grave",
        category: EmojiCategory::SpecialChars,
    },
    EmojiEntry {
        symbol: "ñ",
        name: "n tilde",
        category: EmojiCategory::SpecialChars,
    },
    EmojiEntry {
        symbol: "ü",
        name: "u umlaut",
        category: EmojiCategory::SpecialChars,
    },
];

pub fn filtered_entries(query: &str, category: EmojiCategory) -> Vec<EmojiEntry> {
    let trimmed = query.trim().to_ascii_lowercase();
    EMOJI_ENTRIES
        .iter()
        .copied()
        .filter(|entry| entry.category == category)
        .filter(|entry| {
            trimmed.is_empty() || entry.name.contains(&trimmed) || entry.symbol.contains(&trimmed)
        })
        .collect()
}

pub fn find_by_symbol(symbol: &str) -> Option<EmojiEntry> {
    EMOJI_ENTRIES
        .iter()
        .copied()
        .find(|entry| entry.symbol == symbol)
}

pub fn push_recent(recent: &mut Vec<String>, symbol: &str, max: usize) {
    recent.retain(|existing| existing != symbol);
    recent.insert(0, symbol.to_string());
    if recent.len() > max {
        recent.truncate(max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtered_entries_respect_category_and_query() {
        let smileys = filtered_entries("", EmojiCategory::Smileys);
        assert!(smileys
            .iter()
            .all(|entry| entry.category == EmojiCategory::Smileys));
        let rocket = filtered_entries("rocket", EmojiCategory::Travel);
        assert_eq!(rocket.len(), 1);
        assert_eq!(rocket[0].symbol, "🚀");
    }

    #[test]
    fn push_recent_moves_latest_to_front_and_caps() {
        let mut recent = vec!["😀".to_string(), "👍".to_string(), "🎮".to_string()];
        push_recent(&mut recent, "👍", 3);
        assert_eq!(recent, vec!["👍", "😀", "🎮"]);
        push_recent(&mut recent, "🚀", 3);
        assert_eq!(recent, vec!["🚀", "👍", "😀"]);
    }
}
