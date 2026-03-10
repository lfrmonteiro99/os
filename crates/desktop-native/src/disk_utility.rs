use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, CornerRadius, RichText, Stroke};
use sysinfo::{DiskKind, Disks};

#[derive(Clone, Debug, PartialEq)]
struct DiskUsageSlice {
    label: &'static str,
    bytes: u64,
    color: Color32,
}

#[derive(Clone, Debug, PartialEq)]
struct DiskVolume {
    name: String,
    mount_point: String,
    file_system: String,
    kind: String,
    total_bytes: u64,
    available_bytes: u64,
    removable: bool,
    read_only: bool,
    smart_status: &'static str,
    usage: Vec<DiskUsageSlice>,
}

#[derive(Clone, Debug)]
struct FirstAidRun {
    started_at: Instant,
    duration: Duration,
}

pub struct DiskUtilityApp {
    selected_volume: usize,
    first_aid: Option<FirstAidRun>,
    erase_dialog_open: bool,
    partition_editor_open: bool,
    info_panel_open: bool,
    last_action: String,
}

impl DiskUtilityApp {
    pub fn new() -> Self {
        Self {
            selected_volume: 0,
            first_aid: None,
            erase_dialog_open: false,
            partition_editor_open: false,
            info_panel_open: false,
            last_action: "Ready to inspect mounted volumes.".to_string(),
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        let white = Color32::from_gray(235);
        let gray = Color32::from_gray(150);
        let panel = Color32::from_rgba_unmultiplied(255, 255, 255, 10);
        let volumes = collect_volumes();
        if volumes.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("No mounted disks found.").color(white));
            });
            return;
        }
        if self.selected_volume >= volumes.len() {
            self.selected_volume = 0;
        }
        let selected = &volumes[self.selected_volume];
        let used_bytes = selected
            .total_bytes
            .saturating_sub(selected.available_bytes);
        let used_pct = percent_of(selected.total_bytes, used_bytes);
        let free_pct = percent_of(selected.total_bytes, selected.available_bytes);
        let first_aid_progress = self.first_aid_progress();
        if self.first_aid.is_some() && first_aid_progress >= 1.0 {
            self.first_aid = None;
            self.last_action = format!(
                "First Aid completed for {}. No blocking issues found.",
                selected.name
            );
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Disk Utility").size(16.0).strong().color(white));
                        ui.label(
                            RichText::new(
                                "Mounted volumes, health checks, and safe mock maintenance actions.",
                            )
                            .size(11.0)
                            .color(gray),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if selected.removable && ui.button("Eject").clicked() {
                            self.last_action =
                                format!("Eject requested for {}. Simulated only.", selected.name);
                        }
                        if ui.button("Info").clicked() {
                            self.info_panel_open = !self.info_panel_open;
                        }
                        if ui.button("Partition").clicked() {
                            self.partition_editor_open = !self.partition_editor_open;
                        }
                        if ui.button("Erase").clicked() {
                            self.erase_dialog_open = !self.erase_dialog_open;
                        }
                        let aid_label = if self.first_aid.is_some() {
                            "Running First Aid..."
                        } else {
                            "First Aid"
                        };
                        if ui.button(aid_label).clicked() && self.first_aid.is_none() {
                            self.first_aid = Some(FirstAidRun {
                                started_at: Instant::now(),
                                duration: Duration::from_secs(5),
                            });
                            self.last_action =
                                format!("Scanning {} for file system inconsistencies...", selected.name);
                        }
                    });
                });

                ui.add_space(8.0);
                ui.columns(2, |columns| {
                    columns[0].vertical(|ui| {
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.label(RichText::new("Volumes").size(12.0).strong().color(white));
                                ui.add_space(4.0);
                                for (idx, volume) in volumes.iter().enumerate() {
                                    let selected_row = idx == self.selected_volume;
                                    let fill = if selected_row {
                                        Color32::from_rgba_unmultiplied(52, 120, 246, 52)
                                    } else {
                                        Color32::TRANSPARENT
                                    };
                                    let response = egui::Frame::default()
                                        .fill(fill)
                                        .corner_radius(CornerRadius::same(8))
                                        .inner_margin(egui::Margin::symmetric(8, 6))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(if volume.removable { "◉" } else { "●" })
                                                        .size(12.0)
                                                        .color(if volume.removable {
                                                            Color32::from_rgb(48, 209, 88)
                                                        } else {
                                                            Color32::from_rgb(10, 132, 255)
                                                        }),
                                                );
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        RichText::new(&volume.name)
                                                            .size(12.0)
                                                            .strong()
                                                            .color(Color32::WHITE),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{}  {} free",
                                                            volume.mount_point,
                                                            format_bytes(volume.available_bytes)
                                                        ))
                                                        .size(10.0)
                                                        .color(gray),
                                                    );
                                                });
                                            });
                                        })
                                        .response;
                                    if response.interact(egui::Sense::click()).clicked() {
                                        self.selected_volume = idx;
                                    }
                                }
                            });

                        ui.add_space(8.0);
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.label(RichText::new("Status").size(12.0).strong().color(white));
                                ui.add_space(4.0);
                                ui.label(RichText::new(&self.last_action).size(11.0).color(gray));
                                if self.first_aid.is_some() {
                                    ui.add_space(6.0);
                                    ui.add(
                                        egui::ProgressBar::new(first_aid_progress)
                                            .text(format!(
                                                "First Aid {:.0}%",
                                                first_aid_progress * 100.0
                                            )),
                                    );
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
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(&selected.name)
                                                .size(20.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "{}  {}  SMART {}",
                                                selected.kind, selected.file_system, selected.smart_status
                                            ))
                                            .size(11.0)
                                            .color(gray),
                                        );
                                    });
                                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(format!("{used_pct:.0}% used"))
                                                .size(13.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        );
                                    });
                                });

                                ui.add_space(10.0);
                                render_usage_bar(ui, &selected.usage, selected.total_bytes);
                                ui.add_space(8.0);
                                ui.horizontal_wrapped(|ui| {
                                    for slice in &selected.usage {
                                        ui.label(
                                            RichText::new(format!(
                                                "■ {} {}",
                                                slice.label,
                                                format_bytes(slice.bytes)
                                            ))
                                            .size(10.0)
                                            .color(slice.color),
                                        );
                                        ui.add_space(8.0);
                                    }
                                });

                                ui.add_space(10.0);
                                ui.columns(3, |columns| {
                                    metric_card(
                                        &mut columns[0],
                                        "Capacity",
                                        &format_bytes(selected.total_bytes),
                                        white,
                                        gray,
                                    );
                                    metric_card(
                                        &mut columns[1],
                                        "Available",
                                        &format!("{free_pct:.0}%"),
                                        white,
                                        gray,
                                    );
                                    metric_card(
                                        &mut columns[2],
                                        "Mount Point",
                                        &selected.mount_point,
                                        white,
                                        gray,
                                    );
                                });

                                if self.info_panel_open {
                                    ui.add_space(10.0);
                                    egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                        .corner_radius(CornerRadius::same(8))
                                        .inner_margin(egui::Margin::same(10))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new("Detailed Properties")
                                                    .size(12.0)
                                                    .strong()
                                                    .color(white),
                                            );
                                            detail_row(ui, "Device", &selected.name, white, gray);
                                            detail_row(ui, "Type", &selected.kind, white, gray);
                                            detail_row(
                                                ui,
                                                "File System",
                                                &selected.file_system,
                                                white,
                                                gray,
                                            );
                                            detail_row(
                                                ui,
                                                "Mounted At",
                                                &selected.mount_point,
                                                white,
                                                gray,
                                            );
                                            detail_row(
                                                ui,
                                                "Writable",
                                                if selected.read_only { "No" } else { "Yes" },
                                                white,
                                                gray,
                                            );
                                            detail_row(
                                                ui,
                                                "Removable",
                                                if selected.removable { "Yes" } else { "No" },
                                                white,
                                                gray,
                                            );
                                            detail_row(
                                                ui,
                                                "SMART",
                                                selected.smart_status,
                                                white,
                                                gray,
                                            );
                                        });
                                }

                                if self.erase_dialog_open {
                                    ui.add_space(10.0);
                                    warning_box(
                                        ui,
                                        "Erase is a mock confirmation only. No disk formatting will be performed.",
                                    );
                                }
                                if self.partition_editor_open {
                                    ui.add_space(10.0);
                                    warning_box(
                                        ui,
                                        "Partition editing is visual-only in this prototype. Drag handles and apply are intentionally disabled.",
                                    );
                                }
                            });
                    });
                });
            });
    }

    fn first_aid_progress(&self) -> f32 {
        self.first_aid
            .as_ref()
            .map(|run| {
                let elapsed = Instant::now().saturating_duration_since(run.started_at);
                (elapsed.as_secs_f32() / run.duration.as_secs_f32()).clamp(0.0, 1.0)
            })
            .unwrap_or(0.0)
    }
}

fn collect_volumes() -> Vec<DiskVolume> {
    let disks = Disks::new_with_refreshed_list();
    let mut volumes = disks
        .list()
        .iter()
        .map(|disk| {
            let total = disk.total_space();
            let available = disk.available_space();
            let name = disk.name().to_string_lossy().trim().to_string();
            let mount = disk.mount_point().to_string_lossy().to_string();
            let display_name = if name.is_empty() { mount.clone() } else { name };
            DiskVolume {
                name: display_name,
                mount_point: mount,
                file_system: disk.file_system().to_string_lossy().to_string(),
                kind: disk.kind().to_string(),
                total_bytes: total,
                available_bytes: available,
                removable: disk.is_removable(),
                read_only: disk.is_read_only(),
                smart_status: smart_status_for_disk(disk.kind(), available),
                usage: build_usage_slices(total, available),
            }
        })
        .collect::<Vec<_>>();
    volumes.sort_by(|a, b| {
        b.total_bytes
            .cmp(&a.total_bytes)
            .then_with(|| a.name.cmp(&b.name))
    });
    volumes
}

fn build_usage_slices(total_bytes: u64, available_bytes: u64) -> Vec<DiskUsageSlice> {
    let used = total_bytes.saturating_sub(available_bytes);
    if total_bytes == 0 {
        return vec![DiskUsageSlice {
            label: "Free",
            bytes: 0,
            color: Color32::from_gray(200),
        }];
    }
    let apps = used.saturating_mul(32) / 100;
    let documents = used.saturating_mul(24) / 100;
    let system = used.saturating_mul(18) / 100;
    let other = used.saturating_sub(apps + documents + system);
    vec![
        DiskUsageSlice {
            label: "Apps",
            bytes: apps,
            color: Color32::from_rgb(10, 132, 255),
        },
        DiskUsageSlice {
            label: "Documents",
            bytes: documents,
            color: Color32::from_rgb(48, 209, 88),
        },
        DiskUsageSlice {
            label: "System",
            bytes: system,
            color: Color32::from_rgb(255, 159, 10),
        },
        DiskUsageSlice {
            label: "Other",
            bytes: other,
            color: Color32::from_rgb(142, 142, 147),
        },
        DiskUsageSlice {
            label: "Free",
            bytes: available_bytes,
            color: Color32::from_gray(215),
        },
    ]
}

fn smart_status_for_disk(kind: DiskKind, available_bytes: u64) -> &'static str {
    match kind {
        DiskKind::SSD if available_bytes < 20 * 1024 * 1024 * 1024 => "Warning",
        DiskKind::HDD if available_bytes < 10 * 1024 * 1024 * 1024 => "Warning",
        _ => "Verified",
    }
}

fn render_usage_bar(ui: &mut egui::Ui, slices: &[DiskUsageSlice], total_bytes: u64) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 20.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(10), Color32::from_gray(35));
    if total_bytes == 0 {
        return;
    }
    let mut left = rect.left();
    for slice in slices {
        let fraction = slice.bytes as f32 / total_bytes as f32;
        let slice_width = if slice == slices.last().unwrap() {
            rect.right() - left
        } else {
            rect.width() * fraction
        };
        let slice_rect = egui::Rect::from_min_max(
            egui::pos2(left, rect.top()),
            egui::pos2((left + slice_width).min(rect.right()), rect.bottom()),
        );
        painter.rect_filled(slice_rect, CornerRadius::same(10), slice.color);
        left = slice_rect.right();
    }
}

fn metric_card(ui: &mut egui::Ui, label: &str, value: &str, white: Color32, gray: Color32) {
    egui::Frame::default()
        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(10.0).color(gray));
            ui.label(RichText::new(value).size(12.0).strong().color(white));
        });
}

fn detail_row(ui: &mut egui::Ui, label: &str, value: &str, white: Color32, gray: Color32) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(96.0, 18.0),
            egui::Layout::left_to_right(Align::Center),
            |ui| {
                ui.label(RichText::new(label).size(10.0).color(gray));
            },
        );
        ui.label(RichText::new(value).size(11.0).color(white));
    });
}

fn warning_box(ui: &mut egui::Ui, text: &str) {
    egui::Frame::default()
        .fill(Color32::from_rgba_unmultiplied(255, 159, 10, 18))
        .stroke(Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(255, 159, 10, 80),
        ))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(11.0)
                    .color(Color32::from_gray(240)),
            );
        });
}

fn percent_of(total: u64, part: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (part as f32 / total as f32) * 100.0
    }
}

fn format_bytes(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes >= 100 * 1024 * 1024 * 1024 {
        format!("{:.0} GB", bytes as f64 / GIB)
    } else if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / GIB)
    } else {
        format!("{:.0} MB", bytes as f64 / MIB)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_usage_slices_covers_total_space() {
        let slices = build_usage_slices(1_000, 250);
        let total = slices.iter().map(|slice| slice.bytes).sum::<u64>();
        assert_eq!(total, 1_000);
        assert_eq!(slices.last().unwrap().label, "Free");
        assert_eq!(slices.last().unwrap().bytes, 250);
    }

    #[test]
    fn smart_status_warns_when_space_is_low() {
        assert_eq!(
            smart_status_for_disk(DiskKind::SSD, 5 * 1024 * 1024 * 1024),
            "Warning"
        );
        assert_eq!(
            smart_status_for_disk(DiskKind::HDD, 50 * 1024 * 1024 * 1024),
            "Verified"
        );
    }

    #[test]
    fn format_bytes_uses_expected_units() {
        assert_eq!(format_bytes(512 * 1024 * 1024), "512 MB");
        assert_eq!(format_bytes(5 * 1024 * 1024 * 1024), "5.0 GB");
        assert_eq!(format_bytes(250 * 1024 * 1024 * 1024), "250 GB");
    }
}
