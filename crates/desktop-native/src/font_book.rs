use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui::{self, Align, Color32, CornerRadius, FontFamily, FontId, RichText, Stroke};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FontCollection {
    All,
    RecentlyAdded,
    System,
    User,
    Serif,
    SansSerif,
    Monospace,
    Script,
    Decorative,
}

impl FontCollection {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All Fonts",
            Self::RecentlyAdded => "Recently Added",
            Self::System => "System",
            Self::User => "User",
            Self::Serif => "Serif",
            Self::SansSerif => "Sans-Serif",
            Self::Monospace => "Monospace",
            Self::Script => "Script",
            Self::Decorative => "Decorative",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FontKind {
    Serif,
    SansSerif,
    Monospace,
    Script,
    Decorative,
}

impl FontKind {
    fn label(self) -> &'static str {
        match self {
            Self::Serif => "Serif",
            Self::SansSerif => "Sans-Serif",
            Self::Monospace => "Monospace",
            Self::Script => "Script",
            Self::Decorative => "Decorative",
        }
    }

    fn family(self) -> FontFamily {
        match self {
            Self::Monospace => FontFamily::Monospace,
            _ => FontFamily::Proportional,
        }
    }
}

#[derive(Clone, Debug)]
struct FontEntry {
    family: String,
    style: String,
    source: String,
    file_name: String,
    path: Option<PathBuf>,
    kind: FontKind,
    languages: &'static str,
    designer: &'static str,
    version: &'static str,
}

pub struct FontBookApp {
    query: String,
    sample_text: String,
    selected_collection: FontCollection,
    selected_font: usize,
    preview_size: f32,
    disabled_fonts: HashSet<String>,
    custom_collection: Vec<String>,
    install_path: String,
    install_to_user: bool,
    status: String,
}

impl FontBookApp {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            sample_text: "The quick brown fox jumps over the lazy dog 0123456789".to_string(),
            selected_collection: FontCollection::All,
            selected_font: 0,
            preview_size: 36.0,
            disabled_fonts: HashSet::new(),
            custom_collection: Vec::new(),
            install_path: String::new(),
            install_to_user: true,
            status: "Ready to preview and organize fonts.".to_string(),
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        let white = Color32::from_gray(235);
        let gray = Color32::from_gray(150);
        let panel = Color32::from_rgba_unmultiplied(255, 255, 255, 10);
        let fonts = available_fonts();
        let filtered = filtered_font_indices(&fonts, self.selected_collection, &self.query);
        if filtered.is_empty() {
            self.selected_font = 0;
        } else if self.selected_font >= filtered.len() {
            self.selected_font = 0;
        }
        let selected_entry = filtered
            .get(self.selected_font)
            .and_then(|idx| fonts.get(*idx))
            .or_else(|| fonts.first());

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Font Book").size(16.0).strong().color(white));
                        ui.label(
                            RichText::new(
                                "Preview installed fonts, build lightweight collections, and validate installs.",
                            )
                            .size(11.0)
                            .color(gray),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.preview_size, 12.0..=72.0)
                                .text("Preview Size"),
                        );
                    });
                });

                ui.add_space(8.0);
                ui.columns(3, |columns| {
                    columns[0].vertical(|ui| {
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("Collections").size(12.0).strong().color(white),
                                );
                                ui.add_space(4.0);
                                for collection in [
                                    FontCollection::All,
                                    FontCollection::RecentlyAdded,
                                    FontCollection::System,
                                    FontCollection::User,
                                    FontCollection::Serif,
                                    FontCollection::SansSerif,
                                    FontCollection::Monospace,
                                    FontCollection::Script,
                                    FontCollection::Decorative,
                                ] {
                                    let active = self.selected_collection == collection;
                                    let fill = if active {
                                        Color32::from_rgba_unmultiplied(52, 120, 246, 52)
                                    } else {
                                        Color32::TRANSPARENT
                                    };
                                    let response = egui::Frame::default()
                                        .fill(fill)
                                        .corner_radius(CornerRadius::same(8))
                                        .inner_margin(egui::Margin::symmetric(8, 6))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(collection.label())
                                                    .size(11.0)
                                                    .color(Color32::WHITE),
                                            );
                                        })
                                        .response;
                                    if response.interact(egui::Sense::click()).clicked() {
                                        self.selected_collection = collection;
                                        self.selected_font = 0;
                                    }
                                }

                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new("Custom Collection")
                                        .size(12.0)
                                        .strong()
                                        .color(white),
                                );
                                if let Some(entry) = selected_entry {
                                    if ui.button("Add Selected Font").clicked()
                                        && !self.custom_collection.contains(&entry.family)
                                    {
                                        self.custom_collection.push(entry.family.clone());
                                        self.status =
                                            format!("Added {} to custom collection.", entry.family);
                                    }
                                }
                                for family in &self.custom_collection {
                                    ui.label(RichText::new(family).size(10.0).color(gray));
                                }
                            });

                        ui.add_space(8.0);
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("Install Font").size(12.0).strong().color(white),
                                );
                                ui.add_space(4.0);
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.install_path)
                                        .hint_text("Path to .ttf / .otf / .ttc")
                                        .desired_width(f32::INFINITY),
                                );
                                ui.checkbox(&mut self.install_to_user, "Install for current user");
                                if ui.button("Validate & Install").clicked() {
                                    self.status = validate_install_path(
                                        &self.install_path,
                                        self.install_to_user,
                                    );
                                }
                                ui.add_space(4.0);
                                ui.label(RichText::new(&self.status).size(10.0).color(gray));
                            });
                    });

                    columns[1].vertical(|ui| {
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("Families").size(12.0).strong().color(white),
                                    );
                                    ui.add_space(8.0);
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.query)
                                            .hint_text("Search family, style, or source")
                                            .desired_width(f32::INFINITY),
                                    );
                                });
                                ui.add_space(6.0);
                                egui::ScrollArea::vertical()
                                    .max_height(420.0)
                                    .show(ui, |ui| {
                                        if filtered.is_empty() {
                                            ui.label(
                                                RichText::new("No fonts match this filter.")
                                                    .size(11.0)
                                                    .color(gray),
                                            );
                                        }
                                        for (visible_idx, idx) in filtered.iter().enumerate() {
                                            let entry = &fonts[*idx];
                                            let active = visible_idx == self.selected_font;
                                            let disabled = self.disabled_fonts.contains(&entry.family);
                                            let fill = if active {
                                                Color32::from_rgba_unmultiplied(52, 120, 246, 52)
                                            } else {
                                                Color32::TRANSPARENT
                                            };
                                            let response = egui::Frame::default()
                                                .fill(fill)
                                                .corner_radius(CornerRadius::same(8))
                                                .inner_margin(egui::Margin::symmetric(8, 6))
                                                .show(ui, |ui| {
                                                    ui.label(
                                                        RichText::new(&entry.family)
                                                            .size(13.0)
                                                            .strong()
                                                            .color(if disabled {
                                                                Color32::from_gray(120)
                                                            } else {
                                                                Color32::WHITE
                                                            }),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{}  •  {}  •  {}",
                                                            entry.style,
                                                            entry.kind.label(),
                                                            entry.source
                                                        ))
                                                        .size(10.0)
                                                        .color(gray),
                                                    );
                                                    ui.label(
                                                        RichText::new(&self.sample_text)
                                                            .font(FontId::new(
                                                                16.0,
                                                                entry.kind.family(),
                                                            ))
                                                            .color(if disabled {
                                                                Color32::from_gray(110)
                                                            } else {
                                                                Color32::from_gray(210)
                                                            }),
                                                    );
                                                })
                                                .response;
                                            if response.interact(egui::Sense::click()).clicked() {
                                                self.selected_font = visible_idx;
                                            }
                                        }
                                    });
                            });
                    });

                    columns[2].vertical(|ui| {
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                if let Some(entry) = selected_entry {
                                    let disabled = self.disabled_fonts.contains(&entry.family);
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(&entry.family)
                                                    .size(20.0)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}  •  {}  •  {}",
                                                    entry.style,
                                                    entry.kind.label(),
                                                    entry.version
                                                ))
                                                .size(11.0)
                                                .color(gray),
                                            );
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(Align::Center),
                                            |ui| {
                                                let label =
                                                    if disabled { "Enable" } else { "Disable" };
                                                if ui.button(label).clicked() {
                                                    if disabled {
                                                        self.disabled_fonts.remove(&entry.family);
                                                        self.status = format!(
                                                            "{} is visible again.",
                                                            entry.family
                                                        );
                                                    } else {
                                                        self.disabled_fonts
                                                            .insert(entry.family.clone());
                                                        self.status = format!(
                                                            "{} hidden from previews.",
                                                            entry.family
                                                        );
                                                    }
                                                }
                                            },
                                        );
                                    });

                                    ui.add_space(8.0);
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.sample_text)
                                            .desired_rows(3)
                                            .hint_text("Sample text"),
                                    );
                                    ui.add_space(8.0);
                                    for size in [12.0, 18.0, self.preview_size.max(24.0)] {
                                        ui.label(
                                            RichText::new(&self.sample_text)
                                                .font(FontId::new(size, entry.kind.family()))
                                                .color(if disabled {
                                                    Color32::from_gray(120)
                                                } else {
                                                    Color32::WHITE
                                                }),
                                        );
                                        ui.add_space(4.0);
                                    }

                                    ui.separator();
                                    info_row(ui, "Designer", entry.designer, white, gray);
                                    info_row(ui, "Languages", entry.languages, white, gray);
                                    info_row(ui, "Source", &entry.source, white, gray);
                                    info_row(ui, "File", &entry.file_name, white, gray);
                                    if let Some(path) = &entry.path {
                                        info_row(
                                            ui,
                                            "Location",
                                            &path.display().to_string(),
                                            white,
                                            gray,
                                        );
                                    }

                                    ui.add_space(10.0);
                                    ui.label(
                                        RichText::new("Glyph Grid").size(12.0).strong().color(white),
                                    );
                                    render_glyph_grid(ui, entry.kind, disabled);
                                } else {
                                    ui.label(
                                        RichText::new("No fonts available to preview.")
                                            .size(11.0)
                                            .color(gray),
                                    );
                                }
                            });
                    });
                });
            });
    }
}

fn filtered_font_indices(
    fonts: &[FontEntry],
    collection: FontCollection,
    query: &str,
) -> Vec<usize> {
    let query = query.trim().to_ascii_lowercase();
    let mut matches = fonts
        .iter()
        .enumerate()
        .filter(|(_, entry)| collection_matches(collection, entry))
        .filter(|(_, entry)| {
            query.is_empty()
                || entry.family.to_ascii_lowercase().contains(&query)
                || entry.style.to_ascii_lowercase().contains(&query)
                || entry.source.to_ascii_lowercase().contains(&query)
        })
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    if collection == FontCollection::RecentlyAdded {
        matches.reverse();
    }
    matches
}

fn collection_matches(collection: FontCollection, entry: &FontEntry) -> bool {
    match collection {
        FontCollection::All | FontCollection::RecentlyAdded => true,
        FontCollection::System => entry.source == "System",
        FontCollection::User => entry.source == "User",
        FontCollection::Serif => entry.kind == FontKind::Serif,
        FontCollection::SansSerif => entry.kind == FontKind::SansSerif,
        FontCollection::Monospace => entry.kind == FontKind::Monospace,
        FontCollection::Script => entry.kind == FontKind::Script,
        FontCollection::Decorative => entry.kind == FontKind::Decorative,
    }
}

fn available_fonts() -> Vec<FontEntry> {
    let mut fonts = scan_font_paths();
    if fonts.is_empty() {
        fonts = fallback_fonts();
    }
    fonts.sort_by(|a, b| a.family.cmp(&b.family).then_with(|| a.style.cmp(&b.style)));
    fonts
}

fn scan_font_paths() -> Vec<FontEntry> {
    let mut fonts = Vec::new();
    let mut seen = HashSet::new();
    for (source, dir) in font_search_roots() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_font_file(&path) {
                continue;
            }
            let family = path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("Unknown Font")
                .replace(['_', '-'], " ");
            let key = family.to_ascii_lowercase();
            if !seen.insert(key) {
                continue;
            }
            fonts.push(FontEntry {
                style: infer_style(&family).to_string(),
                kind: infer_font_kind(&family),
                languages: infer_languages(&family),
                designer: infer_designer(&family),
                version: "1.0",
                file_name: path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .to_string(),
                path: Some(path),
                family,
                source: source.clone(),
            });
        }
    }
    fonts
}

fn font_search_roots() -> Vec<(String, PathBuf)> {
    let mut roots = Vec::new();
    #[cfg(windows)]
    {
        roots.push(("System".to_string(), PathBuf::from(r"C:\Windows\Fonts")));
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            roots.push((
                "User".to_string(),
                PathBuf::from(local_app_data).join("Microsoft\\Windows\\Fonts"),
            ));
        }
    }
    #[cfg(target_os = "linux")]
    {
        roots.push(("System".to_string(), PathBuf::from("/usr/share/fonts")));
        if let Ok(home) = std::env::var("HOME") {
            roots.push(("User".to_string(), PathBuf::from(home).join(".fonts")));
        }
    }
    #[cfg(target_os = "macos")]
    {
        roots.push(("System".to_string(), PathBuf::from("/System/Library/Fonts")));
        if let Ok(home) = std::env::var("HOME") {
            roots.push((
                "User".to_string(),
                PathBuf::from(home).join("Library/Fonts"),
            ));
        }
    }
    roots
}

fn fallback_fonts() -> Vec<FontEntry> {
    vec![
        fallback_entry("SF Pro Display", FontKind::SansSerif, "System"),
        fallback_entry("New York", FontKind::Serif, "System"),
        fallback_entry("JetBrains Mono", FontKind::Monospace, "User"),
        fallback_entry("Snell Roundhand", FontKind::Script, "System"),
        fallback_entry("Cooper Black", FontKind::Decorative, "System"),
    ]
}

fn fallback_entry(family: &str, kind: FontKind, source: &str) -> FontEntry {
    FontEntry {
        family: family.to_string(),
        style: "Regular".to_string(),
        source: source.to_string(),
        file_name: format!("{}.ttf", family.replace(' ', "")),
        path: None,
        kind,
        languages: infer_languages(family),
        designer: infer_designer(family),
        version: "1.0",
    }
}

fn infer_style(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.contains("bold") {
        "Bold"
    } else if lower.contains("italic") || lower.contains("oblique") {
        "Italic"
    } else if lower.contains("light") {
        "Light"
    } else {
        "Regular"
    }
}

fn infer_font_kind(name: &str) -> FontKind {
    let lower = name.to_ascii_lowercase();
    if lower.contains("mono") || lower.contains("code") || lower.contains("console") {
        FontKind::Monospace
    } else if lower.contains("script") || lower.contains("hand") || lower.contains("brush") {
        FontKind::Script
    } else if lower.contains("display")
        || lower.contains("black")
        || lower.contains("impact")
        || lower.contains("decor")
    {
        FontKind::Decorative
    } else if lower.contains("serif") || lower.contains("times") || lower.contains("georgia") {
        FontKind::Serif
    } else {
        FontKind::SansSerif
    }
}

fn infer_languages(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.contains("noto") {
        "Latin, Greek, Cyrillic, CJK"
    } else if lower.contains("mono") {
        "Latin, symbols"
    } else {
        "Latin, Western European"
    }
}

fn infer_designer(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.contains("jetbrains") {
        "JetBrains"
    } else if lower.contains("sf") {
        "Apple"
    } else if lower.contains("noto") {
        "Google"
    } else {
        "Unknown Foundry"
    }
}

fn is_font_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref(),
        Some("ttf" | "otf" | "ttc")
    )
}

fn validate_install_path(path: &str, install_to_user: bool) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "Choose a font file before installing.".to_string();
    }
    let candidate = Path::new(trimmed);
    if !is_font_file(candidate) {
        return "Unsupported font format. Use .ttf, .otf, or .ttc.".to_string();
    }
    if !candidate.exists() {
        return "Font file not found on disk.".to_string();
    }
    let scope = if install_to_user {
        "current user"
    } else {
        "system-wide"
    };
    format!(
        "Validated {} for {} installation. Prototype does not copy files yet.",
        candidate
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("font"),
        scope
    )
}

fn info_row(ui: &mut egui::Ui, label: &str, value: &str, white: Color32, gray: Color32) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(88.0, 18.0),
            egui::Layout::left_to_right(Align::Center),
            |ui| {
                ui.label(RichText::new(label).size(10.0).color(gray));
            },
        );
        ui.label(RichText::new(value).size(11.0).color(white));
    });
}

fn render_glyph_grid(ui: &mut egui::Ui, kind: FontKind, disabled: bool) {
    let color = if disabled {
        Color32::from_gray(120)
    } else {
        Color32::from_gray(230)
    };
    let glyphs = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!?@#&";
    egui::Grid::new("font_glyph_grid")
        .num_columns(6)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            for (idx, glyph) in glyphs.chars().enumerate() {
                ui.label(
                    RichText::new(glyph.to_string())
                        .font(FontId::new(22.0, kind.family()))
                        .color(color),
                );
                if idx % 6 == 5 {
                    ui.end_row();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_font_kind_matches_common_names() {
        assert_eq!(infer_font_kind("JetBrains Mono"), FontKind::Monospace);
        assert_eq!(infer_font_kind("Times New Roman"), FontKind::Serif);
        assert_eq!(infer_font_kind("Brush Script"), FontKind::Script);
    }

    #[test]
    fn filtered_font_indices_respects_collection_and_query() {
        let fonts = vec![
            fallback_entry("JetBrains Mono", FontKind::Monospace, "User"),
            fallback_entry("SF Pro Display", FontKind::SansSerif, "System"),
            fallback_entry("New York Serif", FontKind::Serif, "System"),
        ];
        assert_eq!(
            filtered_font_indices(&fonts, FontCollection::Monospace, ""),
            vec![0]
        );
        assert_eq!(
            filtered_font_indices(&fonts, FontCollection::All, "display"),
            vec![1]
        );
    }

    #[test]
    fn validate_install_path_requires_supported_extension() {
        assert!(validate_install_path("", true).contains("Choose"));
        assert!(validate_install_path("C:/tmp/font.txt", true).contains("Unsupported"));
    }
}
