//! Browser window state and mock page rendering.

use eframe::egui::Color32;

// ── Data model ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum BrowserPage {
    Favorites,
    MockSite { title: String, sections: Vec<PageSection> },
    NotFound { url: String },
}

#[derive(Clone, Debug)]
pub struct PageSection {
    pub kind: SectionKind,
    pub text: String,
}

#[derive(Clone, Debug)]
pub enum SectionKind {
    Heading,
    Paragraph,
    Image { color: Color32, height: f32 },
    Link { url: String },
    SearchBar,
    CodeBlock,
}

pub struct BrowserState {
    pub url: String,
    pub history: Vec<String>,
    pub history_idx: usize,
    pub page: BrowserPage,
    pub bookmarks: Vec<Bookmark>,
}

#[derive(Clone)]
pub struct Bookmark {
    pub name: String,
    pub url: String,
    pub color: Color32,
    pub abbrev: String,
}

impl BrowserState {
    pub fn new() -> Self {
        Self {
            url: "auroraos://favorites".to_string(),
            history: vec!["auroraos://favorites".to_string()],
            history_idx: 0,
            page: BrowserPage::Favorites,
            bookmarks: default_bookmarks(),
        }
    }

    pub fn navigate(&mut self, url: &str) {
        let url = normalize_url(url);
        self.page = resolve_page(&url);
        self.url = url.clone();

        // Truncate forward history
        if self.history_idx + 1 < self.history.len() {
            self.history.truncate(self.history_idx + 1);
        }
        self.history.push(url);
        self.history_idx = self.history.len() - 1;
    }

    pub fn can_go_back(&self) -> bool {
        self.history_idx > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.history_idx + 1 < self.history.len()
    }

    pub fn go_back(&mut self) {
        if self.can_go_back() {
            self.history_idx -= 1;
            let url = self.history[self.history_idx].clone();
            self.page = resolve_page(&url);
            self.url = url;
        }
    }

    pub fn go_forward(&mut self) {
        if self.can_go_forward() {
            self.history_idx += 1;
            let url = self.history[self.history_idx].clone();
            self.page = resolve_page(&url);
            self.url = url;
        }
    }

    pub fn add_bookmark(&mut self, name: &str, url: &str, color: Color32) {
        let abbrev = name.chars().take(2).collect::<String>();
        self.bookmarks.push(Bookmark {
            name: name.to_string(),
            url: url.to_string(),
            color,
            abbrev,
        });
    }
}

fn normalize_url(url: &str) -> String {
    let url = url.trim().to_lowercase();
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("auroraos://") {
        url
    } else if url.contains('.') {
        format!("https://{}", url)
    } else {
        // Treat as search
        format!("https://search.auroraos.local/?q={}", url.replace(' ', "+"))
    }
}

fn resolve_page(url: &str) -> BrowserPage {
    if url.contains("auroraos://favorites") {
        return BrowserPage::Favorites;
    }

    let domain = url
        .replace("https://", "")
        .replace("http://", "")
        .split('/')
        .next()
        .unwrap_or("")
        .to_string();

    match domain.as_str() {
        "google.com" | "www.google.com" => BrowserPage::MockSite {
            title: "Google".to_string(),
            sections: vec![
                PageSection { kind: SectionKind::Heading, text: "Google".to_string() },
                PageSection { kind: SectionKind::SearchBar, text: String::new() },
                PageSection { kind: SectionKind::Paragraph, text: "Search the world's information".to_string() },
            ],
        },
        "github.com" | "www.github.com" => BrowserPage::MockSite {
            title: "GitHub".to_string(),
            sections: vec![
                PageSection { kind: SectionKind::Image { color: Color32::from_rgb(36, 41, 46), height: 50.0 }, text: "GitHub - Where the world builds software".to_string() },
                PageSection { kind: SectionKind::Heading, text: "Popular repositories".to_string() },
                PageSection { kind: SectionKind::Link { url: "https://github.com/AuroraOS".to_string() }, text: "AuroraOS/desktop - macOS-like desktop in Rust".to_string() },
                PageSection { kind: SectionKind::Link { url: "https://github.com/AuroraOS".to_string() }, text: "AuroraOS/kernel - Microkernel foundation".to_string() },
                PageSection { kind: SectionKind::CodeBlock, text: "cargo build --release\ncargo run -p desktop-native".to_string() },
            ],
        },
        "reddit.com" | "www.reddit.com" => BrowserPage::MockSite {
            title: "Reddit".to_string(),
            sections: vec![
                PageSection { kind: SectionKind::Image { color: Color32::from_rgb(255, 69, 0), height: 40.0 }, text: "reddit".to_string() },
                PageSection { kind: SectionKind::Heading, text: "r/programming".to_string() },
                PageSection { kind: SectionKind::Link { url: "#".to_string() }, text: "Show HN: AuroraOS - A desktop OS written in Rust".to_string() },
                PageSection { kind: SectionKind::Paragraph, text: "1.2k upvotes - 342 comments".to_string() },
                PageSection { kind: SectionKind::Link { url: "#".to_string() }, text: "Why Rust is the future of systems programming".to_string() },
                PageSection { kind: SectionKind::Paragraph, text: "856 upvotes - 198 comments".to_string() },
            ],
        },
        d if d.contains("search.auroraos") => {
            let query = url.split("q=").nth(1).unwrap_or("").replace('+', " ");
            BrowserPage::MockSite {
                title: format!("Search: {}", query),
                sections: vec![
                    PageSection { kind: SectionKind::SearchBar, text: query.clone() },
                    PageSection { kind: SectionKind::Heading, text: format!("Results for \"{}\"", query) },
                    PageSection { kind: SectionKind::Link { url: "#".to_string() }, text: format!("{} - Wikipedia", query) },
                    PageSection { kind: SectionKind::Paragraph, text: "The free encyclopedia article about this topic...".to_string() },
                    PageSection { kind: SectionKind::Link { url: "#".to_string() }, text: format!("{} documentation", query) },
                    PageSection { kind: SectionKind::Paragraph, text: "Official documentation and reference guide.".to_string() },
                ],
            }
        }
        _ => BrowserPage::NotFound { url: url.to_string() },
    }
}

fn default_bookmarks() -> Vec<Bookmark> {
    vec![
        Bookmark { name: "Apple".into(), url: "https://apple.com".into(), color: Color32::from_rgb(142, 142, 147), abbrev: "A".into() },
        Bookmark { name: "Google".into(), url: "https://google.com".into(), color: Color32::from_rgb(66, 133, 244), abbrev: "G".into() },
        Bookmark { name: "GitHub".into(), url: "https://github.com".into(), color: Color32::from_rgb(36, 41, 46), abbrev: "GH".into() },
        Bookmark { name: "Reddit".into(), url: "https://reddit.com".into(), color: Color32::from_rgb(255, 69, 0), abbrev: "R".into() },
        Bookmark { name: "Netflix".into(), url: "https://netflix.com".into(), color: Color32::from_rgb(229, 9, 20), abbrev: "N".into() },
        Bookmark { name: "Rust".into(), url: "https://rust-lang.org".into(), color: Color32::from_rgb(222, 165, 88), abbrev: "Rs".into() },
        Bookmark { name: "Twitter".into(), url: "https://twitter.com".into(), color: Color32::from_rgb(29, 161, 242), abbrev: "T".into() },
        Bookmark { name: "LinkedIn".into(), url: "https://linkedin.com".into(), color: Color32::from_rgb(0, 119, 181), abbrev: "Li".into() },
    ]
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_at_favorites() {
        let b = BrowserState::new();
        assert!(matches!(b.page, BrowserPage::Favorites));
        assert_eq!(b.url, "auroraos://favorites");
    }

    #[test]
    fn navigate_to_google() {
        let mut b = BrowserState::new();
        b.navigate("google.com");
        assert!(matches!(b.page, BrowserPage::MockSite { .. }));
        if let BrowserPage::MockSite { title, .. } = &b.page {
            assert_eq!(title, "Google");
        }
    }

    #[test]
    fn navigate_to_github() {
        let mut b = BrowserState::new();
        b.navigate("github.com");
        if let BrowserPage::MockSite { title, sections, .. } = &b.page {
            assert_eq!(title, "GitHub");
            assert!(sections.len() >= 3);
        } else {
            panic!("Expected MockSite for github.com");
        }
    }

    #[test]
    fn navigate_unknown_is_not_found() {
        let mut b = BrowserState::new();
        b.navigate("unknown-site-xyz.example");
        assert!(matches!(b.page, BrowserPage::NotFound { .. }));
    }

    #[test]
    fn search_query_generates_results() {
        let mut b = BrowserState::new();
        b.navigate("rust programming");
        if let BrowserPage::MockSite { title, .. } = &b.page {
            assert!(title.contains("rust programming"));
        } else {
            panic!("Expected MockSite for search query");
        }
    }

    #[test]
    fn history_back_forward() {
        let mut b = BrowserState::new();
        b.navigate("google.com");
        b.navigate("github.com");
        assert!(!b.can_go_forward());
        assert!(b.can_go_back());

        b.go_back();
        assert!(b.url.contains("google.com"));
        assert!(b.can_go_forward());

        b.go_forward();
        assert!(b.url.contains("github.com"));
    }

    #[test]
    fn history_truncates_on_new_nav() {
        let mut b = BrowserState::new();
        b.navigate("google.com");
        b.navigate("github.com");
        b.go_back(); // at google.com
        b.navigate("reddit.com"); // should truncate github.com
        assert!(!b.can_go_forward());
        assert_eq!(b.history.len(), 3); // favorites, google, reddit
    }

    #[test]
    fn normalize_adds_https() {
        assert_eq!(normalize_url("google.com"), "https://google.com");
    }

    #[test]
    fn normalize_preserves_scheme() {
        assert_eq!(normalize_url("http://test.com"), "http://test.com");
        assert!(normalize_url("auroraos://favorites").contains("auroraos://"));
    }

    #[test]
    fn normalize_search_no_dot() {
        let url = normalize_url("rust programming");
        assert!(url.contains("search.auroraos"));
        assert!(url.contains("rust+programming"));
    }

    #[test]
    fn default_bookmarks_not_empty() {
        let b = BrowserState::new();
        assert!(b.bookmarks.len() >= 6);
    }

    #[test]
    fn add_bookmark() {
        let mut b = BrowserState::new();
        let before = b.bookmarks.len();
        b.add_bookmark("Test", "https://test.com", Color32::RED);
        assert_eq!(b.bookmarks.len(), before + 1);
        assert_eq!(b.bookmarks.last().unwrap().name, "Test");
    }

    #[test]
    fn go_back_at_start_does_nothing() {
        let mut b = BrowserState::new();
        let url_before = b.url.clone();
        b.go_back();
        assert_eq!(b.url, url_before);
    }

    #[test]
    fn go_forward_at_end_does_nothing() {
        let mut b = BrowserState::new();
        b.navigate("google.com");
        let url_before = b.url.clone();
        b.go_forward();
        assert_eq!(b.url, url_before);
    }
}
