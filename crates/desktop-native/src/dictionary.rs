use eframe::egui::{self, Align, Color32, CornerRadius, RichText, Stroke};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryEntry {
    pub word: &'static str,
    pub pronunciation: &'static str,
    pub part_of_speech: &'static str,
    pub definitions: &'static [&'static str],
    pub examples: &'static [&'static str],
    pub synonyms: &'static [&'static str],
    pub antonyms: &'static [&'static str],
}

pub struct DictionaryApp {
    pub query: String,
    selected_word: String,
    history: Vec<String>,
    bookmarks: Vec<String>,
    thesaurus_mode: bool,
    word_of_day_index: usize,
}

impl DictionaryApp {
    pub fn new() -> Self {
        let word_of_day_index = 7;
        let word_of_day = ENTRIES[word_of_day_index].word.to_string();
        Self {
            query: word_of_day.clone(),
            selected_word: word_of_day,
            history: Vec::new(),
            bookmarks: vec!["aurora".to_string(), "kernel".to_string()],
            thesaurus_mode: false,
            word_of_day_index,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        let panel = Color32::from_rgba_unmultiplied(255, 255, 255, 10);
        let white = Color32::from_gray(235);
        let gray = Color32::from_gray(150);
        let query = self.query.trim().to_ascii_lowercase();
        let results = search_entries(&query);
        let selected = find_entry(&self.selected_word).or_else(|| results.first().copied());
        if selected.is_none() && !query.is_empty() {
            self.selected_word = query.clone();
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Dictionary").size(16.0).strong().color(white));
                        ui.label(
                            RichText::new("Definitions, examples, and quick thesaurus lookup.")
                                .size(11.0)
                                .color(gray),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.checkbox(&mut self.thesaurus_mode, "Thesaurus");
                    });
                });

                ui.add_space(8.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Search for a word")
                        .desired_width(f32::INFINITY),
                );

                ui.add_space(10.0);
                ui.columns(2, |columns| {
                    columns[0].vertical(|ui| {
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                let featured = &ENTRIES[self.word_of_day_index % ENTRIES.len()];
                                ui.label(
                                    RichText::new("Word of the Day")
                                        .size(12.0)
                                        .strong()
                                        .color(white),
                                );
                                ui.label(
                                    RichText::new(featured.word)
                                        .size(14.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                );
                                ui.label(
                                    RichText::new(featured.definitions[0])
                                        .size(11.0)
                                        .color(gray),
                                );
                                if ui.small_button("Open").clicked() {
                                    self.open_word(featured.word);
                                }
                            });

                        ui.add_space(8.0);
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.label(RichText::new("Matches").size(12.0).strong().color(white));
                                ui.add_space(4.0);
                                egui::ScrollArea::vertical()
                                    .max_height(220.0)
                                    .show(ui, |ui| {
                                        if results.is_empty() {
                                            ui.label(
                                                RichText::new("No local entry found.")
                                                    .size(11.0)
                                                    .color(gray),
                                            );
                                        }
                                        for entry in &results {
                                            let selected_word = self.selected_word == entry.word;
                                            let fill = if selected_word {
                                                Color32::from_rgba_unmultiplied(0, 122, 255, 50)
                                            } else {
                                                Color32::TRANSPARENT
                                            };
                                            let response = egui::Frame::default()
                                                .fill(fill)
                                                .corner_radius(CornerRadius::same(8))
                                                .inner_margin(egui::Margin::symmetric(8, 6))
                                                .show(ui, |ui| {
                                                    ui.label(
                                                        RichText::new(entry.word)
                                                            .size(12.0)
                                                            .strong()
                                                            .color(Color32::WHITE),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{}  {}",
                                                            entry.pronunciation,
                                                            entry.part_of_speech
                                                        ))
                                                        .size(10.0)
                                                        .color(gray),
                                                    );
                                                })
                                                .response;
                                            if response.interact(egui::Sense::click()).clicked() {
                                                self.open_word(entry.word);
                                            }
                                        }
                                    });
                            });

                        ui.add_space(8.0);
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.label(RichText::new("Recent").size(12.0).strong().color(white));
                                let recent_words = self.history.clone();
                                for word in recent_words {
                                    if ui.small_button(&word).clicked() {
                                        self.open_word(&word);
                                    }
                                }
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new("Bookmarks").size(12.0).strong().color(white),
                                );
                                let bookmarked_words = self.bookmarks.clone();
                                for word in bookmarked_words {
                                    if ui.small_button(&word).clicked() {
                                        self.open_word(&word);
                                    }
                                }
                            });
                    });

                    columns[1].vertical(|ui| {
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                if let Some(entry) = selected {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(entry.word)
                                                    .size(22.0)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}  {}",
                                                    entry.pronunciation, entry.part_of_speech
                                                ))
                                                .size(11.0)
                                                .color(gray),
                                            );
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(Align::Center),
                                            |ui| {
                                                let saved = self
                                                    .bookmarks
                                                    .iter()
                                                    .any(|word| word == entry.word);
                                                let label =
                                                    if saved { "Bookmarked" } else { "Bookmark" };
                                                if ui.button(label).clicked() && !saved {
                                                    self.bookmarks
                                                        .insert(0, entry.word.to_string());
                                                    self.bookmarks.truncate(8);
                                                }
                                            },
                                        );
                                    });

                                    ui.add_space(8.0);
                                    if self.thesaurus_mode {
                                        ui.label(
                                            RichText::new("Synonyms")
                                                .size(12.0)
                                                .strong()
                                                .color(white),
                                        );
                                        ui.label(
                                            RichText::new(if entry.synonyms.is_empty() {
                                                "No local synonyms recorded.".to_string()
                                            } else {
                                                entry.synonyms.join(", ")
                                            })
                                            .size(11.0)
                                            .color(gray),
                                        );
                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new("Antonyms")
                                                .size(12.0)
                                                .strong()
                                                .color(white),
                                        );
                                        ui.label(
                                            RichText::new(if entry.antonyms.is_empty() {
                                                "No local antonyms recorded.".to_string()
                                            } else {
                                                entry.antonyms.join(", ")
                                            })
                                            .size(11.0)
                                            .color(gray),
                                        );
                                    } else {
                                        ui.label(
                                            RichText::new("Definitions")
                                                .size(12.0)
                                                .strong()
                                                .color(white),
                                        );
                                        for (index, definition) in
                                            entry.definitions.iter().enumerate()
                                        {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}. {}",
                                                    index + 1,
                                                    definition
                                                ))
                                                .size(11.0)
                                                .color(gray),
                                            );
                                        }
                                        if !entry.examples.is_empty() {
                                            ui.add_space(8.0);
                                            ui.label(
                                                RichText::new("Examples")
                                                    .size(12.0)
                                                    .strong()
                                                    .color(white),
                                            );
                                            for example in entry.examples {
                                                ui.label(
                                                    RichText::new(format!("\"{}\"", example))
                                                        .size(11.0)
                                                        .italics()
                                                        .color(Color32::from_gray(175)),
                                                );
                                            }
                                        }
                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new("Related")
                                                .size(12.0)
                                                .strong()
                                                .color(white),
                                        );
                                        ui.label(
                                            RichText::new(if entry.synonyms.is_empty() {
                                                "No related terms recorded.".to_string()
                                            } else {
                                                entry.synonyms.join(", ")
                                            })
                                            .size(11.0)
                                            .color(gray),
                                        );
                                    }
                                } else {
                                    ui.label(
                                        RichText::new("No dictionary entry selected.")
                                            .size(12.0)
                                            .color(gray),
                                    );
                                }
                            });
                    });
                });
            });
    }

    pub fn open_word(&mut self, word: &str) {
        self.query = word.to_string();
        self.selected_word = word.to_string();
        self.history.retain(|existing| existing != word);
        self.history.insert(0, word.to_string());
        self.history.truncate(10);
    }
}

pub fn find_entry(word: &str) -> Option<&'static DictionaryEntry> {
    let query = word.trim().to_ascii_lowercase();
    ENTRIES.iter().find(|entry| entry.word == query)
}

pub fn search_entries(query: &str) -> Vec<&'static DictionaryEntry> {
    if query.is_empty() {
        return ENTRIES.iter().take(10).collect();
    }
    ENTRIES
        .iter()
        .filter(|entry| {
            entry.word.contains(query)
                || entry.synonyms.iter().any(|value| value.contains(&query))
                || entry
                    .definitions
                    .iter()
                    .any(|value| value.to_ascii_lowercase().contains(query))
        })
        .take(24)
        .collect()
}

pub fn inline_definition(word: &str) -> Option<String> {
    let entry = find_entry(word)?;
    Some(entry.definitions[0].to_string())
}

const ENTRIES: &[DictionaryEntry] = &[
    DictionaryEntry {
        word: "aurora",
        pronunciation: "/uh-roar-uh/",
        part_of_speech: "noun",
        definitions: &["A natural light display in the sky, often visible near polar regions."],
        examples: &["The aurora shimmered above the winter coastline."],
        synonyms: &["polar lights", "northern lights"],
        antonyms: &[],
    },
    DictionaryEntry {
        word: "widget",
        pronunciation: "/wij-it/",
        part_of_speech: "noun",
        definitions: &["A small interface element that surfaces focused information or actions."],
        examples: &["The weather widget showed rain by noon."],
        synonyms: &["panel", "tile", "component"],
        antonyms: &[],
    },
    DictionaryEntry {
        word: "kernel",
        pronunciation: "/kur-nuhl/",
        part_of_speech: "noun",
        definitions: &[
            "The core component of an operating system that manages hardware and processes.",
        ],
        examples: &["The kernel brokers access to memory and devices."],
        synonyms: &["core", "runtime"],
        antonyms: &[],
    },
    DictionaryEntry {
        word: "latency",
        pronunciation: "/lay-tuhn-see/",
        part_of_speech: "noun",
        definitions: &["The delay between an action and its visible or measurable response."],
        examples: &["Lower latency made the interface feel immediate."],
        synonyms: &["delay", "lag"],
        antonyms: &["responsiveness"],
    },
    DictionaryEntry {
        word: "throughput",
        pronunciation: "/throo-put/",
        part_of_speech: "noun",
        definitions: &["The amount of work or data processed in a given period of time."],
        examples: &["The network monitor estimated peak throughput at 800 Mbps."],
        synonyms: &["bandwidth", "capacity"],
        antonyms: &["bottleneck"],
    },
    DictionaryEntry {
        word: "sandbox",
        pronunciation: "/sand-boks/",
        part_of_speech: "noun",
        definitions: &["An isolated execution environment with restricted permissions."],
        examples: &["The app ran inside a sandbox with no file-system write access."],
        synonyms: &["isolation", "container"],
        antonyms: &["host system"],
    },
    DictionaryEntry {
        word: "compositor",
        pronunciation: "/kuhm-poz-i-ter/",
        part_of_speech: "noun",
        definitions: &["Software that combines application surfaces into a final desktop scene."],
        examples: &["The compositor applied blur before presenting the frame."],
        synonyms: &["renderer", "scene manager"],
        antonyms: &[],
    },
    DictionaryEntry {
        word: "daemon",
        pronunciation: "/dee-muhn/",
        part_of_speech: "noun",
        definitions: &["A background service process that runs without direct user interaction."],
        examples: &["A logging daemon collected status updates from each service."],
        synonyms: &["service", "background process"],
        antonyms: &["foreground app"],
    },
    DictionaryEntry {
        word: "clipboard",
        pronunciation: "/klip-bord/",
        part_of_speech: "noun",
        definitions: &["Temporary storage used when copying and pasting data."],
        examples: &["The clipboard kept the latest copied report snippet."],
        synonyms: &["copy buffer"],
        antonyms: &[],
    },
    DictionaryEntry {
        word: "gesture",
        pronunciation: "/jes-cher/",
        part_of_speech: "noun",
        definitions: &["A touch or pointer movement interpreted as a command."],
        examples: &["A three-finger gesture switched desktops."],
        synonyms: &["motion", "swipe"],
        antonyms: &["click"],
    },
    DictionaryEntry {
        word: "focus",
        pronunciation: "/foh-kus/",
        part_of_speech: "noun",
        definitions: &["The element or window currently receiving user input."],
        examples: &["Keyboard focus moved to the search field."],
        synonyms: &["attention", "active target"],
        antonyms: &["background"],
    },
    DictionaryEntry {
        word: "render",
        pronunciation: "/ren-der/",
        part_of_speech: "verb",
        definitions: &["To draw or compute visible output for display."],
        examples: &["The engine renders each window into the scene graph."],
        synonyms: &["draw", "paint"],
        antonyms: &["hide"],
    },
    DictionaryEntry {
        word: "process",
        pronunciation: "/prah-ses/",
        part_of_speech: "noun",
        definitions: &["A running instance of a program with its own memory and execution state."],
        examples: &["The process manager listed CPU usage per process."],
        synonyms: &["task", "program"],
        antonyms: &[],
    },
    DictionaryEntry {
        word: "shortcut",
        pronunciation: "/short-kuht/",
        part_of_speech: "noun",
        definitions: &["A quick command, key combination, or alias that triggers an action."],
        examples: &["A shortcut opened Spotlight instantly."],
        synonyms: &["hotkey", "alias"],
        antonyms: &["long route"],
    },
    DictionaryEntry {
        word: "preview",
        pronunciation: "/pree-vyoo/",
        part_of_speech: "noun",
        definitions: &["A temporary or reduced view of content before opening it fully."],
        examples: &["Quick Look showed a preview of the selected image."],
        synonyms: &["sample", "glimpse"],
        antonyms: &["final output"],
    },
    DictionaryEntry {
        word: "index",
        pronunciation: "/in-deks/",
        part_of_speech: "noun",
        definitions: &["A structured lookup used to locate data efficiently."],
        examples: &["Spotlight queries the file index for results."],
        synonyms: &["catalog", "lookup table"],
        antonyms: &["scan"],
    },
    DictionaryEntry {
        word: "notification",
        pronunciation: "/noh-tuh-fuh-kay-shun/",
        part_of_speech: "noun",
        definitions: &["A message surfaced by the system to alert the user to an event."],
        examples: &["The notification center grouped alerts by app."],
        synonyms: &["alert", "notice"],
        antonyms: &["silence"],
    },
    DictionaryEntry {
        word: "tab",
        pronunciation: "/tab/",
        part_of_speech: "noun",
        definitions: &["A selectable strip representing an open document, view, or session."],
        examples: &["The terminal opened a new tab for the deployment logs."],
        synonyms: &["pane", "session"],
        antonyms: &["window"],
    },
    DictionaryEntry {
        word: "window",
        pronunciation: "/win-doh/",
        part_of_speech: "noun",
        definitions: &["A framed region of the desktop containing an application or document."],
        examples: &["The settings window opened near the center of the screen."],
        synonyms: &["panel", "frame"],
        antonyms: &["fullscreen"],
    },
    DictionaryEntry {
        word: "diagnostics",
        pronunciation: "/dai-ug-nos-tiks/",
        part_of_speech: "noun",
        definitions: &["Information and tools used to inspect, test, and troubleshoot a system."],
        examples: &["Network diagnostics reported packet loss on the Wi-Fi link."],
        synonyms: &["analysis", "inspection"],
        antonyms: &["guesswork"],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_definition_returns_known_word() {
        assert!(inline_definition("aurora")
            .unwrap()
            .contains("light display"));
    }

    #[test]
    fn search_matches_definitions_and_synonyms() {
        let results = search_entries("bandwidth");
        assert!(results.iter().any(|entry| entry.word == "throughput"));
    }

    #[test]
    fn open_word_updates_history() {
        let mut app = DictionaryApp::new();
        app.open_word("widget");
        app.open_word("kernel");
        assert_eq!(app.history.first().map(String::as_str), Some("kernel"));
    }
}
