use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use chrono::Local;
use eframe::egui::{self, Align, Color32, CornerRadius, RichText, Stroke};

use crate::notifications::AppNotification;
use crate::process_manager::ProcessInfo;
use crate::toast::Toast;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConsoleLevel {
    Error,
    Warning,
    Info,
    Debug,
}

impl ConsoleLevel {
    fn label(self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::Warning => "Warning",
            Self::Info => "Info",
            Self::Debug => "Debug",
        }
    }

    fn color(self) -> Color32 {
        match self {
            Self::Error => Color32::from_rgb(255, 99, 99),
            Self::Warning => Color32::from_rgb(255, 204, 0),
            Self::Info => Color32::from_gray(235),
            Self::Debug => Color32::from_gray(135),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConsoleSource {
    All,
    System,
    Applications,
    User,
    Custom,
}

impl ConsoleSource {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All Sources",
            Self::System => "System",
            Self::Applications => "Applications",
            Self::User => "User",
            Self::Custom => "Custom Logs",
        }
    }
}

#[derive(Clone, Debug)]
struct ConsoleEntry {
    id: String,
    timestamp: String,
    source: ConsoleSource,
    process: String,
    pid: Option<u32>,
    level: ConsoleLevel,
    message: String,
    detail: String,
}

#[derive(Clone, Debug)]
pub struct ConsoleTelemetrySnapshot {
    pub connected: bool,
    pub status: String,
    pub health: String,
    pub uptime: String,
    pub last_error: Option<String>,
    pub network_name: String,
    pub process_count: usize,
}

pub struct ConsoleApp {
    pub search: String,
    pub process_filter: String,
    selected_source: ConsoleSource,
    show_error: bool,
    show_warning: bool,
    show_info: bool,
    show_debug: bool,
    paused: bool,
    auto_scroll: bool,
    cleared: bool,
    cached_entries: Vec<ConsoleEntry>,
    expanded: HashSet<String>,
    bookmarked: HashSet<String>,
    last_export_path: Option<PathBuf>,
}

impl ConsoleApp {
    pub fn new() -> Self {
        Self {
            search: String::new(),
            process_filter: String::new(),
            selected_source: ConsoleSource::All,
            show_error: true,
            show_warning: true,
            show_info: true,
            show_debug: false,
            paused: false,
            auto_scroll: true,
            cleared: false,
            cached_entries: Vec::new(),
            expanded: HashSet::new(),
            bookmarked: HashSet::new(),
            last_export_path: None,
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        telemetry: &ConsoleTelemetrySnapshot,
        notifications: &[AppNotification],
        toasts: &[&Toast],
        processes: &[ProcessInfo],
        custom_log_root: &std::path::Path,
    ) {
        if !self.paused && !self.cleared {
            self.cached_entries =
                Self::collect_entries(telemetry, notifications, toasts, processes, custom_log_root);
        }
        let filtered = self.filtered_entries();
        let panel = Color32::from_rgba_unmultiplied(255, 255, 255, 10);
        let white = Color32::from_gray(235);
        let gray = Color32::from_gray(150);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Console").size(16.0).strong().color(white));
                        ui.label(
                            RichText::new(
                                "System, app, and custom log streams with real-time filters.",
                            )
                            .size(11.0)
                            .color(gray),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Export").clicked() {
                            self.export_filtered(&filtered);
                        }
                        if ui.button("Refresh").clicked() {
                            self.cleared = false;
                            self.paused = false;
                            self.cached_entries = Self::collect_entries(
                                telemetry,
                                notifications,
                                toasts,
                                processes,
                                custom_log_root,
                            );
                        }
                        let pause_label = if self.paused { "Resume" } else { "Pause" };
                        if ui.button(pause_label).clicked() {
                            self.paused = !self.paused;
                        }
                        if ui.button("Clear").clicked() {
                            self.cleared = true;
                            self.cached_entries.clear();
                        }
                    });
                });

                if let Some(path) = &self.last_export_path {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("Last export: {}", path.display()))
                            .size(10.0)
                            .color(Color32::from_gray(110)),
                    );
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Search messages...")
                            .desired_width(200.0),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.process_filter)
                            .hint_text("Filter process/app...")
                            .desired_width(160.0),
                    );
                    for source in [
                        ConsoleSource::All,
                        ConsoleSource::System,
                        ConsoleSource::Applications,
                        ConsoleSource::User,
                        ConsoleSource::Custom,
                    ] {
                        let selected = self.selected_source == source;
                        let fill = if selected {
                            Color32::from_rgb(0, 122, 255)
                        } else {
                            Color32::from_gray(55)
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(source.label())
                                        .size(10.0)
                                        .color(Color32::WHITE),
                                )
                                .fill(fill)
                                .corner_radius(CornerRadius::same(8)),
                            )
                            .clicked()
                        {
                            self.selected_source = source;
                        }
                    }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_error, "Error");
                    ui.checkbox(&mut self.show_warning, "Warning");
                    ui.checkbox(&mut self.show_info, "Info");
                    ui.checkbox(&mut self.show_debug, "Debug");
                    ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
                });

                ui.add_space(10.0);
                egui::Frame::default()
                    .fill(panel)
                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.allocate_ui_with_layout(
                                egui::Vec2::new(78.0, 18.0),
                                egui::Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.label(RichText::new("Time").size(10.0).strong().color(gray));
                                },
                            );
                            ui.allocate_ui_with_layout(
                                egui::Vec2::new(86.0, 18.0),
                                egui::Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new("Level").size(10.0).strong().color(gray),
                                    );
                                },
                            );
                            ui.allocate_ui_with_layout(
                                egui::Vec2::new(90.0, 18.0),
                                egui::Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new("Source").size(10.0).strong().color(gray),
                                    );
                                },
                            );
                            ui.allocate_ui_with_layout(
                                egui::Vec2::new(170.0, 18.0),
                                egui::Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new("Process").size(10.0).strong().color(gray),
                                    );
                                },
                            );
                            ui.label(RichText::new("Message").size(10.0).strong().color(gray));
                        });
                        ui.add_space(4.0);

                        egui::ScrollArea::vertical()
                            .max_height(320.0)
                            .stick_to_bottom(self.auto_scroll)
                            .show(ui, |ui| {
                                if filtered.is_empty() {
                                    ui.label(
                                        RichText::new("No log entries match the current filters.")
                                            .size(11.0)
                                            .color(gray),
                                    );
                                }
                                for entry in &filtered {
                                    let expanded = self.expanded.contains(&entry.id);
                                    let bookmarked = self.bookmarked.contains(&entry.id);
                                    let response = egui::Frame::default()
                                        .fill(if bookmarked {
                                            Color32::from_rgba_unmultiplied(255, 214, 10, 28)
                                        } else {
                                            Color32::from_rgba_unmultiplied(255, 255, 255, 6)
                                        })
                                        .corner_radius(CornerRadius::same(8))
                                        .inner_margin(egui::Margin::symmetric(8, 6))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(&entry.timestamp)
                                                        .size(10.0)
                                                        .monospace()
                                                        .color(Color32::from_gray(120)),
                                                );
                                                ui.label(
                                                    RichText::new(entry.level.label())
                                                        .size(10.0)
                                                        .strong()
                                                        .color(entry.level.color()),
                                                );
                                                ui.label(
                                                    RichText::new(entry.source.label())
                                                        .size(10.0)
                                                        .color(Color32::from_gray(150)),
                                                );
                                                let process_label = if let Some(pid) = entry.pid {
                                                    format!("{} ({pid})", entry.process)
                                                } else {
                                                    entry.process.clone()
                                                };
                                                ui.label(
                                                    RichText::new(process_label)
                                                        .size(10.0)
                                                        .monospace()
                                                        .color(Color32::from_gray(190)),
                                                );
                                                ui.label(
                                                    RichText::new(&entry.message)
                                                        .size(10.0)
                                                        .monospace()
                                                        .color(Color32::from_gray(220)),
                                                );
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        let star = if bookmarked {
                                                            "Bookmarked"
                                                        } else {
                                                            "Bookmark"
                                                        };
                                                        if ui.small_button(star).clicked() {
                                                            if bookmarked {
                                                                self.bookmarked.remove(&entry.id);
                                                            } else {
                                                                self.bookmarked
                                                                    .insert(entry.id.clone());
                                                            }
                                                        }
                                                    },
                                                );
                                            });
                                            if expanded {
                                                ui.add_space(4.0);
                                                ui.label(
                                                    RichText::new(&entry.detail)
                                                        .size(10.0)
                                                        .monospace()
                                                        .color(Color32::from_gray(170)),
                                                );
                                            }
                                        })
                                        .response;
                                    if response.interact(egui::Sense::click()).clicked() {
                                        if expanded {
                                            self.expanded.remove(&entry.id);
                                        } else {
                                            self.expanded.insert(entry.id.clone());
                                        }
                                    }
                                }
                            });

                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!(
                                "{} visible entries  |  {} bookmarked  |  auto-scroll {}",
                                filtered.len(),
                                self.bookmarked.len(),
                                if self.auto_scroll { "on" } else { "off" }
                            ))
                            .size(10.0)
                            .color(Color32::from_gray(130)),
                        );
                    });
            });
    }

    fn filtered_entries(&self) -> Vec<ConsoleEntry> {
        let query = self.search.trim().to_ascii_lowercase();
        let process_query = self.process_filter.trim().to_ascii_lowercase();
        self.cached_entries
            .iter()
            .filter(|entry| match self.selected_source {
                ConsoleSource::All => true,
                selected => entry.source == selected,
            })
            .filter(|entry| match entry.level {
                ConsoleLevel::Error => self.show_error,
                ConsoleLevel::Warning => self.show_warning,
                ConsoleLevel::Info => self.show_info,
                ConsoleLevel::Debug => self.show_debug,
            })
            .filter(|entry| {
                query.is_empty()
                    || entry.message.to_ascii_lowercase().contains(&query)
                    || entry.detail.to_ascii_lowercase().contains(&query)
                    || entry.process.to_ascii_lowercase().contains(&query)
            })
            .filter(|entry| {
                process_query.is_empty()
                    || entry.process.to_ascii_lowercase().contains(&process_query)
            })
            .cloned()
            .collect()
    }

    fn collect_entries(
        telemetry: &ConsoleTelemetrySnapshot,
        notifications: &[AppNotification],
        toasts: &[&Toast],
        processes: &[ProcessInfo],
        custom_log_root: &std::path::Path,
    ) -> Vec<ConsoleEntry> {
        let mut entries = Vec::new();
        let now = Local::now().format("%H:%M:%S").to_string();

        entries.push(ConsoleEntry {
            id: "telemetry_status".to_string(),
            timestamp: now.clone(),
            source: ConsoleSource::System,
            process: "telemetryd".to_string(),
            pid: None,
            level: if telemetry.connected {
                ConsoleLevel::Info
            } else {
                ConsoleLevel::Warning
            },
            message: format!(
                "Daemon {} | health {} | network {} | processes {}",
                telemetry.status, telemetry.health, telemetry.network_name, telemetry.process_count
            ),
            detail: format!(
                "Connected: {}\nUptime: {}",
                telemetry.connected, telemetry.uptime
            ),
        });

        if let Some(error) = &telemetry.last_error {
            entries.push(ConsoleEntry {
                id: "telemetry_error".to_string(),
                timestamp: now.clone(),
                source: ConsoleSource::System,
                process: "telemetryd".to_string(),
                pid: None,
                level: ConsoleLevel::Error,
                message: error.clone(),
                detail: error.clone(),
            });
        }

        for notification in notifications.iter().take(25) {
            entries.push(ConsoleEntry {
                id: format!("notif_{}_{}", notification.app, notification.title),
                timestamp: notification.time_ago(),
                source: ConsoleSource::Applications,
                process: notification.app.clone(),
                pid: None,
                level: classify_color(notification.color),
                message: notification.title.clone(),
                detail: notification.body.clone(),
            });
        }

        for toast in toasts.iter().take(8) {
            entries.push(ConsoleEntry {
                id: format!("toast_{}_{}", toast.title, toast.body),
                timestamp: "now".to_string(),
                source: ConsoleSource::User,
                process: toast.title.clone(),
                pid: None,
                level: classify_color(toast.color),
                message: toast.body.clone(),
                detail: format!(
                    "Toast '{}' visible with {:.0}% progress.",
                    toast.title,
                    toast.progress() * 100.0
                ),
            });
        }

        for process in processes.iter().take(12) {
            entries.push(ConsoleEntry {
                id: format!("proc_{}", process.pid),
                timestamp: now.clone(),
                source: ConsoleSource::System,
                process: process.name.clone(),
                pid: Some(process.pid),
                level: if process.cpu_usage > 20.0 {
                    ConsoleLevel::Warning
                } else {
                    ConsoleLevel::Debug
                },
                message: format!(
                    "CPU {:.1}% | Memory {}",
                    process.cpu_usage,
                    crate::process_manager::ProcessManager::format_memory(process.memory_bytes)
                ),
                detail: format!(
                    "pid={} cpu={:.1}% memory_bytes={}",
                    process.pid, process.cpu_usage, process.memory_bytes
                ),
            });
        }

        entries.extend(read_custom_logs(custom_log_root));
        entries
    }

    fn export_filtered(&mut self, filtered: &[ConsoleEntry]) {
        let path = std::env::temp_dir().join("aurora_console_export.log");
        let content = filtered
            .iter()
            .map(|entry| {
                format!(
                    "{} [{}] {} {} {}\n{}",
                    entry.timestamp,
                    entry.level.label(),
                    entry.source.label(),
                    entry.process,
                    entry.message,
                    entry.detail
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        if fs::write(&path, content).is_ok() {
            self.last_export_path = Some(path);
        }
    }
}

fn classify_color(color: Color32) -> ConsoleLevel {
    if color.r() > 220 && color.g() < 140 {
        ConsoleLevel::Error
    } else if color.r() > 220 && color.g() > 180 {
        ConsoleLevel::Warning
    } else if color.g() > 180 {
        ConsoleLevel::Info
    } else {
        ConsoleLevel::Debug
    }
}

fn read_custom_logs(root: &std::path::Path) -> Vec<ConsoleEntry> {
    let mut entries = Vec::new();
    if !root.exists() {
        return entries;
    }
    let Ok(read_dir) = fs::read_dir(root) else {
        return entries;
    };
    for file in read_dir.flatten().take(5) {
        let path = file.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("log") {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in content.lines().take(6).enumerate() {
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("custom.log");
            entries.push(ConsoleEntry {
                id: format!("custom_{}_{}", file_name, index),
                timestamp: "file".to_string(),
                source: ConsoleSource::Custom,
                process: file_name.to_string(),
                pid: None,
                level: ConsoleLevel::Info,
                message: line.to_string(),
                detail: path.to_string_lossy().to_string(),
            });
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_color_detects_error_and_warning() {
        assert_eq!(
            classify_color(Color32::from_rgb(255, 59, 48)),
            ConsoleLevel::Error
        );
        assert_eq!(
            classify_color(Color32::from_rgb(255, 204, 0)),
            ConsoleLevel::Warning
        );
    }

    #[test]
    fn custom_logs_are_loaded_from_log_files() {
        let root = std::env::temp_dir().join(format!(
            "aurora_console_logs_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("session.log"), "line one\nline two").unwrap();
        let entries = read_custom_logs(&root);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].source, ConsoleSource::Custom);
        let _ = fs::remove_dir_all(&root);
    }
}
