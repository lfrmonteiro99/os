mod app_launcher;
mod assistant;
mod auto_save;
mod browser;
mod calculator;
mod clipboard;
mod color_picker;
mod console;
mod dictionary;
mod disk_utility;
mod embedded_app;
mod emoji_picker;
mod file_index;
mod file_tags;
mod font_book;
mod icons;
mod messages;
mod music_audio;
mod network_diag;
mod notifications;
mod process_manager;
mod quick_look;
mod screensaver;
mod settings;
mod terminal;
mod toast;
mod types;
mod user_profile;
mod window;

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::{BufRead, BufReader, Write as IoWrite};
use std::net::TcpStream;
use std::path::PathBuf;

use std::time::{Duration, Instant};

use battery::Manager as BatteryManager;
use chrono::{Datelike, Local, Timelike};
use eframe::egui::{
    self, Align, Align2, Color32, ColorImage, CornerRadius, FontId, Id, Layout, Order, Pos2, Rect,
    RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2,
};
use ipc::{decode_response, encode_command, CommandFrame};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};

use app_launcher::AppCatalog;
use assistant::{
    parse_query as parse_assistant_query, suggestion_chips as assistant_suggestion_chips,
    AssistantIntent, AssistantMessage,
};
use auto_save::AutoSave;
use browser::BrowserState;
use calculator::{
    calc_eval, format_calc, format_programmer_value, parse_programmer_value, programmer_ascii,
    programmer_eval, programmer_not, programmer_representations, scientific_eval, CalculatorMode,
    ProgrammerBase,
};
use clipboard::AppClipboard;
use color_picker::{sample_color_from_position, ColorPickerApp};
use console::{ConsoleApp, ConsoleTelemetrySnapshot};
use dictionary::{inline_definition as dictionary_inline_definition, DictionaryApp};
use disk_utility::DiskUtilityApp;
use embedded_app::EmbeddedApp;
use emoji_picker::{
    filtered_entries as filtered_emoji_entries, find_by_symbol as find_emoji_by_symbol,
    push_recent as push_recent_emoji, EmojiCategory,
};
use file_index::{
    copy_entry_to_directory, create_directory, create_file, custom_smart_folder_entries,
    delete_entry, delete_trash_entry_permanently, dirs_home, empty_trash, format_size,
    load_trash_entries, move_entry_to_directory, read_directory, rename_entry, restore_trash_entry,
    smart_folder_entries, trash_dir, CustomSmartFolder, FileIndex, FmEntry, SmartFolderKind,
};
use file_tags::{FileTags, TagColor};
use font_book::FontBookApp;
use messages::MessagesState;
use music_audio::MusicAudioEngine;
use network_diag::NetworkDiagnostics;
use notifications::NotificationCenter;
use process_manager::ProcessManager;
use quick_look::{build_preview, move_preview_index, read_file_info, PreviewKind};
use screensaver::{photo_slideshow_paths, render_screensaver_overlay, ScreenSaverKind};
use settings::AppSettings;
use terminal::{launch_program, open_file_with_system, PtyTerminal};
use toast::Toast;
use toast::ToastManager;
use types::*;
use user_profile::UserProfile;
use window::{ManagedWindow, SnapSide};

// ── Telemetry ────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Telemetry {
    connected: bool,
    status: String,
    health: String,
    uptime: String,
    boot: String,
    last_error: Option<String>,
    last_poll: Option<Instant>,
}

// ── Real system info ─────────────────────────────────────────────────────────

struct RealSystemInfo {
    sys: System,
    networks: Networks,
    cpu_usage: f32,
    total_memory_gb: f64,
    used_memory_gb: f64,
    memory_pct: f32,
    battery_pct: f32,
    battery_charging: bool,
    battery_available: bool,
    network_up: bool,
    network_name: String,
    disk_total_gb: f64,
    disk_used_gb: f64,
    process_count: usize,
    last_refresh: Option<Instant>,
}

impl RealSystemInfo {
    fn new() -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        sys.refresh_cpu_all();
        sys.refresh_memory();

        let networks = Networks::new_with_refreshed_list();

        let mut info = Self {
            sys,
            networks,
            cpu_usage: 0.0,
            total_memory_gb: 0.0,
            used_memory_gb: 0.0,
            memory_pct: 0.0,
            battery_pct: 100.0,
            battery_charging: false,
            battery_available: false,
            network_up: false,
            network_name: String::new(),
            disk_total_gb: 0.0,
            disk_used_gb: 0.0,
            process_count: 0,
            last_refresh: None,
        };
        info.refresh();
        info
    }

    fn refresh(&mut self) {
        // CPU
        self.sys.refresh_cpu_all();
        let cpus = self.sys.cpus();
        if !cpus.is_empty() {
            self.cpu_usage = cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32;
        }

        // Memory
        self.sys.refresh_memory();
        let total = self.sys.total_memory();
        let used = self.sys.used_memory();
        self.total_memory_gb = total as f64 / 1_073_741_824.0;
        self.used_memory_gb = used as f64 / 1_073_741_824.0;
        self.memory_pct = if total > 0 {
            (used as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        // Processes
        self.sys
            .refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        self.process_count = self.sys.processes().len();

        // Disks
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let mut total_disk: u64 = 0;
        let mut used_disk: u64 = 0;
        for disk in disks.list() {
            total_disk += disk.total_space();
            used_disk += disk.total_space() - disk.available_space();
        }
        self.disk_total_gb = total_disk as f64 / 1_073_741_824.0;
        self.disk_used_gb = used_disk as f64 / 1_073_741_824.0;

        // Network
        self.networks.refresh(false);
        self.network_up = false;
        self.network_name.clear();
        for (name, data) in self.networks.iter() {
            let received = data.total_received();
            let transmitted = data.total_transmitted();
            if received > 0 || transmitted > 0 {
                self.network_up = true;
                if self.network_name.is_empty() {
                    self.network_name = name.clone();
                }
            }
        }

        // Battery
        if let Ok(manager) = BatteryManager::new() {
            if let Ok(mut batteries) = manager.batteries() {
                if let Some(Ok(batt)) = batteries.next() {
                    self.battery_available = true;
                    self.battery_pct = (batt.state_of_charge().value * 100.0).min(100.0);
                    self.battery_charging = matches!(
                        batt.state(),
                        battery::State::Charging | battery::State::Full
                    );
                }
            }
        }

        self.last_refresh = Some(Instant::now());
    }

    fn should_refresh(&self) -> bool {
        self.last_refresh
            .map(|t| t.elapsed() >= SYSINFO_INTERVAL)
            .unwrap_or(true)
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn gradient_rect(painter: &egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = egui::epaint::Mesh::default();
    let i = mesh.vertices.len() as u32;
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i, i + 2, i + 3);
    painter.add(Shape::mesh(mesh));
}

fn paint_hill(
    painter: &egui::Painter,
    screen: Rect,
    center_x_ratio: f32,
    peak_height: f32,
    spread: f32,
    color: Color32,
) {
    let cx = screen.left() + center_x_ratio * screen.width();
    let bottom = screen.bottom();
    let n = 40;
    let mut mesh = egui::epaint::Mesh::default();
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let x = screen.left() + t * screen.width();
        let dx = (x - cx) / spread;
        let h = peak_height * (-dx * dx * 0.5).exp();
        let idx = mesh.vertices.len() as u32;
        mesh.colored_vertex(Pos2::new(x, bottom - h), color);
        mesh.colored_vertex(Pos2::new(x, bottom), color);
        if i > 0 {
            mesh.add_triangle(idx - 2, idx, idx + 1);
            mesh.add_triangle(idx - 2, idx + 1, idx - 1);
        }
    }
    painter.add(Shape::mesh(mesh));
}

fn desktop_directory() -> PathBuf {
    let desktop = dirs_home().join("Desktop");
    if desktop.exists() {
        desktop
    } else {
        dirs_home()
    }
}

// ── Dock icon painting ───────────────────────────────────────────────────────

fn category_color(category: &str) -> Color32 {
    match category {
        "System" => Color32::from_rgb(142, 142, 147),
        "Utilities" => Color32::from_rgb(90, 200, 250),
        "Internet" => Color32::from_rgb(0, 122, 255),
        "Productivity" => Color32::from_rgb(255, 149, 0),
        "Media" => Color32::from_rgb(255, 55, 95),
        "Communication" => Color32::from_rgb(76, 217, 100),
        "Development" => Color32::from_rgb(88, 86, 214),
        "Games" => Color32::from_rgb(255, 45, 85),
        "Graphics" => Color32::from_rgb(175, 82, 222),
        _ => Color32::from_rgb(100, 100, 120),
    }
}

fn paint_dock_icon(painter: &egui::Painter, rect: Rect, icon: DockIcon) {
    let rounding = CornerRadius::same((rect.width() * 0.22) as u8);
    let s = rect.width();
    let c = rect.center();

    match icon {
        DockIcon::Calendar => {
            painter.rect_filled(rect, rounding, Color32::WHITE);
            let header = Rect::from_min_size(rect.min, Vec2::new(s, s * 0.3));
            painter.rect_filled(header, rounding, Color32::from_rgb(255, 59, 48));
        }
        _ => {
            painter.rect_filled(rect, rounding, icon.bg_color());
        }
    }

    let white = Color32::WHITE;
    let line_w = (s * 0.05).max(1.5);
    let inner = rect.shrink(s * 0.2);

    match icon {
        DockIcon::Files => {
            let body = Rect::from_min_max(
                Pos2::new(inner.left(), inner.top() + inner.height() * 0.2),
                inner.max,
            );
            painter.rect_filled(body, CornerRadius::same(2), white);
            let tab = Rect::from_min_size(
                inner.left_top(),
                Vec2::new(inner.width() * 0.45, inner.height() * 0.25),
            );
            painter.rect_filled(tab, CornerRadius::same(2), white);
        }
        DockIcon::Terminal => {
            let left = inner.left();
            let mid_y = inner.center().y;
            let cw = inner.width() * 0.35;
            let ch = inner.height() * 0.3;
            let stroke = Stroke::new(line_w * 1.5, Color32::from_rgb(166, 227, 161));
            painter.line_segment(
                [Pos2::new(left, mid_y - ch), Pos2::new(left + cw, mid_y)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(left + cw, mid_y), Pos2::new(left, mid_y + ch)],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(left + cw + 4.0, mid_y + ch),
                    Pos2::new(inner.right(), mid_y + ch),
                ],
                Stroke::new(line_w * 1.5, white),
            );
        }
        DockIcon::Browser => {
            let r = inner.width() * 0.42;
            painter.circle_stroke(c, r, Stroke::new(line_w, white));
            painter.line_segment(
                [Pos2::new(c.x - r, c.y), Pos2::new(c.x + r, c.y)],
                Stroke::new(line_w * 0.8, white),
            );
            painter.line_segment(
                [Pos2::new(c.x, c.y - r), Pos2::new(c.x, c.y + r)],
                Stroke::new(line_w * 0.8, white),
            );
            painter.circle_stroke(
                c,
                r * 0.5,
                Stroke::new(line_w * 0.6, Color32::from_white_alpha(160)),
            );
        }
        DockIcon::Overview => {
            let bar_h = inner.height() * 0.16;
            let gap = inner.height() * 0.12;
            for (i, w_ratio) in [0.8f32, 0.55, 0.7].iter().enumerate() {
                let y = inner.top() + i as f32 * (bar_h + gap);
                let bar = Rect::from_min_size(
                    Pos2::new(inner.left(), y),
                    Vec2::new(inner.width() * w_ratio, bar_h),
                );
                painter.rect_filled(bar, CornerRadius::same((bar_h * 0.5) as u8), white);
            }
        }
        DockIcon::Launchpad => {
            // 3x3 grid of circles (like macOS Launchpad icon)
            let grid_size = inner.width() * 0.7;
            let origin = Pos2::new(c.x - grid_size * 0.5, c.y - grid_size * 0.5);
            let dot_r = grid_size * 0.12;
            let spacing = grid_size * 0.35;
            for row in 0..3 {
                for col in 0..3 {
                    let cx = origin.x + col as f32 * spacing + spacing * 0.5;
                    let cy = origin.y + row as f32 * spacing + spacing * 0.5;
                    painter.circle_filled(Pos2::new(cx, cy), dot_r, white);
                }
            }
        }
        DockIcon::Messages => {
            let r = inner.width() * 0.38;
            painter.circle_filled(c, r, white);
            painter.add(Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - r * 0.3, c.y + r * 0.7),
                    Pos2::new(c.x - r * 0.9, c.y + r * 1.2),
                    Pos2::new(c.x + r * 0.1, c.y + r * 0.9),
                ],
                white,
                Stroke::NONE,
            ));
        }
        DockIcon::Notes => {
            painter.rect_stroke(
                inner,
                CornerRadius::same(2),
                Stroke::new(line_w * 0.8, Color32::from_rgb(120, 100, 10)),
                StrokeKind::Outside,
            );
            for i in 0..3 {
                let y = inner.top() + inner.height() * (0.3 + i as f32 * 0.22);
                painter.line_segment(
                    [
                        Pos2::new(inner.left() + 3.0, y),
                        Pos2::new(inner.right() - 3.0, y),
                    ],
                    Stroke::new(line_w * 0.7, Color32::from_rgb(140, 120, 20)),
                );
            }
        }
        DockIcon::Calendar => {
            // Show real day of month
            let day = Local::now().format("%d").to_string();
            painter.text(
                Pos2::new(c.x, c.y + s * 0.08),
                Align2::CENTER_CENTER,
                &day,
                FontId::proportional(s * 0.38),
                Color32::from_rgb(30, 30, 30),
            );
        }
        DockIcon::Music => {
            let nr = inner.width() * 0.18;
            let nc = Pos2::new(c.x - nr * 0.5, inner.bottom() - nr);
            painter.circle_filled(nc, nr, white);
            let st = Pos2::new(nc.x + nr, inner.top() + 2.0);
            let sb = Pos2::new(nc.x + nr, nc.y);
            painter.line_segment([st, sb], Stroke::new(line_w * 1.2, white));
            painter.line_segment(
                [
                    st,
                    Pos2::new(st.x + inner.width() * 0.25, st.y + inner.height() * 0.2),
                ],
                Stroke::new(line_w * 1.5, white),
            );
        }
        DockIcon::Photos => {
            let sr = inner.width() * 0.15;
            painter.circle_filled(
                Pos2::new(inner.right() - sr - 2.0, inner.top() + sr + 2.0),
                sr,
                white,
            );
            painter.add(Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - 2.0, inner.top() + inner.height() * 0.3),
                    Pos2::new(inner.right(), inner.bottom()),
                    Pos2::new(inner.left(), inner.bottom()),
                ],
                white,
                Stroke::NONE,
            ));
        }
        DockIcon::Calculator => {
            let cw = inner.width() * 0.38;
            let ch = inner.height() * 0.25;
            let gx = inner.width() * 0.24;
            let gy = inner.height() * 0.12;
            for row in 0..3 {
                for col in 0..2 {
                    let x = inner.left() + col as f32 * (cw + gx);
                    let y = inner.top() + row as f32 * (ch + gy);
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(x, y), Vec2::new(cw, ch)),
                        CornerRadius::same(2),
                        white,
                    );
                }
            }
        }
        DockIcon::Settings => {
            let r = inner.width() * 0.3;
            painter.circle_stroke(c, r, Stroke::new(line_w * 1.2, white));
            painter.circle_filled(c, r * 0.4, white);
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                painter.line_segment(
                    [
                        Pos2::new(c.x + angle.cos() * r * 0.8, c.y + angle.sin() * r * 0.8),
                        Pos2::new(c.x + angle.cos() * r * 1.35, c.y + angle.sin() * r * 1.35),
                    ],
                    Stroke::new(line_w * 1.5, white),
                );
            }
        }
        DockIcon::Store => {
            painter.text(
                c,
                Align2::CENTER_CENTER,
                "A",
                FontId::proportional(s * 0.45),
                white,
            );
        }
        DockIcon::Controls => {
            let cell = inner.width() * 0.38;
            let gap = inner.width() * 0.24;
            for row in 0..2 {
                for col in 0..2 {
                    let x = inner.left() + col as f32 * (cell + gap);
                    let y = inner.top() + row as f32 * (cell + gap);
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(x, y), Vec2::splat(cell)),
                        CornerRadius::same(3),
                        white,
                    );
                }
            }
        }
        DockIcon::Info => {
            let r = inner.width() * 0.4;
            painter.circle_stroke(c, r, Stroke::new(line_w * 1.2, white));
            painter.text(
                c,
                Align2::CENTER_CENTER,
                "i",
                FontId::proportional(s * 0.35),
                white,
            );
        }
        DockIcon::Trash => {
            let body = Rect::from_min_max(
                Pos2::new(
                    inner.left() + inner.width() * 0.18,
                    inner.top() + inner.height() * 0.28,
                ),
                Pos2::new(
                    inner.right() - inner.width() * 0.18,
                    inner.bottom() - inner.height() * 0.08,
                ),
            );
            painter.rect_stroke(
                body,
                CornerRadius::same(3),
                Stroke::new(line_w, white),
                StrokeKind::Outside,
            );
            let lid_y = inner.top() + inner.height() * 0.22;
            painter.line_segment(
                [
                    Pos2::new(body.left() - 2.0, lid_y),
                    Pos2::new(body.right() + 2.0, lid_y),
                ],
                Stroke::new(line_w * 1.2, white),
            );
            painter.line_segment(
                [
                    Pos2::new(
                        c.x - inner.width() * 0.08,
                        inner.top() + inner.height() * 0.12,
                    ),
                    Pos2::new(
                        c.x + inner.width() * 0.08,
                        inner.top() + inner.height() * 0.12,
                    ),
                ],
                Stroke::new(line_w * 1.2, white),
            );
            for offset in [-0.12_f32, 0.0, 0.12] {
                let x = c.x + inner.width() * offset;
                painter.line_segment(
                    [
                        Pos2::new(x, body.top() + 3.0),
                        Pos2::new(x, body.bottom() - 3.0),
                    ],
                    Stroke::new(line_w * 0.8, white),
                );
            }
        }
        _ => {}
    }
}

// ── App state ────────────────────────────────────────────────────────────────

// ── Wallpaper presets ─────────────────────────────────────────────────────────

struct WallpaperPreset {
    name: &'static str,
    bands: &'static [(f32, [u8; 3])],
    hills: &'static [(f32, f32, f32, [u8; 3])], // (center_x_ratio, peak, spread, rgb)
}

const WALLPAPERS: &[WallpaperPreset] = &[
    WallpaperPreset {
        name: "Big Sur Sunset",
        bands: &[
            (0.00, [27, 43, 94]),
            (0.15, [45, 74, 140]),
            (0.28, [107, 91, 149]),
            (0.40, [192, 108, 132]),
            (0.52, [246, 114, 128]),
            (0.62, [248, 181, 149]),
            (0.72, [255, 207, 135]),
            (0.82, [248, 168, 96]),
            (0.92, [208, 112, 80]),
            (1.00, [42, 26, 58]),
        ],
        hills: &[
            (0.50, 160.0, 500.0, [74, 53, 112]),
            (0.75, 180.0, 400.0, [46, 37, 83]),
            (0.25, 200.0, 450.0, [26, 31, 58]),
            (0.55, 130.0, 600.0, [10, 14, 26]),
        ],
    },
    WallpaperPreset {
        name: "Ocean Blue",
        bands: &[
            (0.00, [10, 15, 40]),
            (0.20, [15, 40, 80]),
            (0.40, [20, 80, 140]),
            (0.55, [40, 120, 180]),
            (0.70, [80, 170, 210]),
            (0.85, [140, 210, 230]),
            (1.00, [20, 50, 90]),
        ],
        hills: &[
            (0.40, 140.0, 500.0, [10, 30, 60]),
            (0.70, 170.0, 400.0, [8, 20, 50]),
            (0.20, 190.0, 550.0, [5, 15, 35]),
        ],
    },
    WallpaperPreset {
        name: "Aurora Borealis",
        bands: &[
            (0.00, [5, 10, 20]),
            (0.15, [10, 25, 45]),
            (0.30, [15, 60, 50]),
            (0.45, [20, 100, 80]),
            (0.55, [30, 140, 100]),
            (0.65, [20, 100, 90]),
            (0.75, [15, 60, 60]),
            (0.90, [10, 30, 40]),
            (1.00, [5, 10, 20]),
        ],
        hills: &[
            (0.45, 150.0, 500.0, [5, 20, 15]),
            (0.70, 180.0, 400.0, [3, 12, 10]),
            (0.25, 200.0, 500.0, [2, 8, 8]),
        ],
    },
    WallpaperPreset {
        name: "Warm Desert",
        bands: &[
            (0.00, [40, 20, 50]),
            (0.15, [80, 40, 60]),
            (0.30, [160, 80, 60]),
            (0.45, [220, 140, 80]),
            (0.55, [240, 180, 100]),
            (0.65, [250, 210, 140]),
            (0.80, [230, 170, 90]),
            (0.90, [180, 110, 60]),
            (1.00, [60, 30, 30]),
        ],
        hills: &[
            (0.50, 120.0, 600.0, [120, 70, 40]),
            (0.30, 160.0, 450.0, [80, 45, 25]),
            (0.75, 140.0, 500.0, [50, 25, 15]),
        ],
    },
];

// ── Toast notifications ──────────────────────────────────────────────────────

// ── App state ────────────────────────────────────────────────────────────────

struct AuroraDesktopApp {
    daemon_addr: String,
    auth_token: Option<String>,
    next_frame_id: u64,
    telemetry: Telemetry,
    sysinfo: RealSystemInfo,
    file_index: FileIndex,
    windows: [ManagedWindow; WINDOW_COUNT],
    z_order: Vec<WindowKind>,
    focused: Option<WindowKind>,
    show_control_center: bool,
    show_spotlight: bool,
    show_assistant: bool,
    show_notifications: bool,
    show_emoji_picker: bool,
    notification_panel_opened_at: Option<Instant>,
    dock_hover_since: Option<Instant>,
    dock_last_exit_at: Option<Instant>,
    spotlight_query: String,
    assistant_query: String,
    cc_wifi: bool,
    cc_bluetooth: bool,
    cc_airdrop: bool,
    cc_focus: bool,
    cc_brightness: f32,
    cc_volume: f32,
    fps_smoothed: f32,
    active_menu: Option<MenuDropdown>,
    context_menu_pos: Option<Pos2>,
    desktop_context_target: Option<PathBuf>,
    desktop_rename_target: Option<PathBuf>,
    desktop_rename_buffer: String,
    dock_bounce: Option<(DockIcon, Instant)>,
    fm_current_dir: PathBuf,
    fm_entries: Vec<FmEntry>,
    fm_selected_path: Option<PathBuf>,
    fm_tabs: Vec<FileManagerTab>,
    fm_active_tab: usize,
    menu_action: Option<MenuAction>,
    spotlight_open_window: Option<WindowKind>,
    should_quit: bool,
    terminal_output: Vec<(String, Color32)>,
    terminal_input: String,
    pty_terminal: Option<PtyTerminal>,
    terminal_tabs: Vec<TerminalTab>,
    terminal_active_tab: usize,
    use_real_terminal: bool,
    cpu_history: VecDeque<f32>,
    // Calculator state
    calc_display: String,
    calc_operand: Option<f64>,
    calc_operator: Option<char>,
    calc_reset_next: bool,
    calc_mode: CalculatorMode,
    calc_history: Vec<String>,
    calc_memory: f64,
    calc_degrees: bool,
    calc_programmer_base: ProgrammerBase,
    calc_programmer_operand: Option<i64>,
    calc_programmer_operator: Option<String>,
    // Notes state
    notes_text: String,
    notes_tabs: Vec<NotesTab>,
    notes_active_tab: usize,
    // Music player state
    music_playing: bool,
    music_track_idx: usize,
    music_library_query: String,
    music_shuffle: bool,
    music_repeat_all: bool,
    music_elapsed_seconds: f32,
    music_last_tick: Instant,
    music_audio: Option<MusicAudioEngine>,
    music_override_path: Option<PathBuf>,
    // Text editor state
    editor_file_path: Option<PathBuf>,
    editor_content: String,
    editor_modified: bool,
    editor_tabs: Vec<EditorTab>,
    editor_active_tab: usize,
    // Browser state
    browser_state: BrowserState,
    browser_url_input: String,
    // Messages state
    messages_state: MessagesState,
    // Toast notifications
    toast_manager: ToastManager,
    // Snap preview animation
    // Calendar navigation
    calendar_month_offset: i32,
    // Photo viewer lightbox
    photo_viewer_idx: Option<usize>,
    pip_state: Option<PipState>,
    pip_dragging: bool,
    pip_resizing: bool,
    pip_last_pos: Option<Pos2>,
    pip_last_size: Option<Vec2>,
    // App Switcher (Ctrl+Tab)
    show_app_switcher: bool,
    app_switcher_idx: usize,
    // Keyboard shortcuts overlay
    show_shortcuts_overlay: bool,
    // Downloads stack
    show_downloads_stack: bool,
    recent_downloads: Vec<String>,
    // Mission Control drag
    mc_dragging_window: Option<WindowKind>,
    // Spotlight: deferred file open
    spotlight_open_file: Option<PathBuf>,
    // Wallpaper
    wallpaper_idx: usize,
    desktop_entries: Vec<FmEntry>,
    desktop_selected_paths: Vec<PathBuf>,
    desktop_stack_expanded: Option<String>,
    desktop_selection_drag_start: Option<Pos2>,
    desktop_last_refresh: Instant,
    file_drag_path: Option<PathBuf>,
    app_phase: AppPhase,
    boot_started_at: Instant,
    // Session auth
    screen_state: AppScreenState,
    auth_focus_pending: bool,
    auth_password: String,
    auth_error: Option<String>,
    setup_user_name: String,
    setup_password: String,
    setup_password_confirm: String,
    setup_step: usize,
    settings_current_password: String,
    settings_new_password: String,
    settings_confirm_password: String,
    settings_password_message: Option<String>,
    settings_custom_folder_name: String,
    settings_custom_folder_extension: String,
    settings_custom_folder_min_size_mb: String,
    settings_custom_folder_tag: String,
    selected_tag_filters: Vec<TagColor>,
    tag_filter_match_all: bool,
    login_shake: Option<Instant>,
    last_input_at: Instant,
    screensaver_active: bool,
    screensaver_kind: ScreenSaverKind,
    screensaver_started_at: Option<Instant>,
    // Mission Control
    show_mission_control: bool,
    mission_control_anim: f32, // 0.0 = hidden, 1.0 = fully shown
    // Edge snap preview
    drag_snap_preview: Option<SnapSide>,
    drag_snap_maximize: bool,
    // Menu bar popups
    show_wifi_popup: bool,
    show_battery_popup: bool,
    show_volume_popup: bool,
    show_bluetooth_popup: bool,
    last_battery_alert_level: u8,
    // Multi-desktop
    current_desktop: usize,
    desktop_count: usize,
    // Clipboard
    clipboard: AppClipboard,
    // Process manager
    proc_manager: Option<ProcessManager>,
    network_diagnostics: NetworkDiagnostics,
    disk_utility: DiskUtilityApp,
    dictionary_app: DictionaryApp,
    console_app: ConsoleApp,
    font_book: FontBookApp,
    color_picker: ColorPickerApp,
    proc_search: String,
    proc_sort_by_cpu: bool,
    // Auto-save
    auto_save: AutoSave,
    // Wallpaper transition
    wallpaper_prev_idx: usize,
    wallpaper_transition: f32, // 0.0 = old, 1.0 = new, animated
    wallpaper_changing: bool,
    // Unsaved changes guard
    confirm_close_window: Option<WindowKind>,
    // Dynamic notifications
    notification_center: NotificationCenter,
    collapsed_notification_apps: HashSet<String>,
    // App settings (persisted)
    app_settings: AppSettings,
    sidebar_favorites: Vec<PathBuf>,
    file_tags: FileTags,
    user_profile: UserProfile,
    profile_name_buffer: String,
    quick_look_open: bool,
    quick_look_paths: Vec<PathBuf>,
    quick_look_index: usize,
    emoji_query: String,
    emoji_category: EmojiCategory,
    recent_emojis: Vec<String>,
    assistant_history: Vec<AssistantMessage>,
    assistant_state: AssistantOverlayState,
    pending_assistant_query: Option<(String, Instant)>,
    photo_texture_cache: HashMap<PathBuf, TextureHandle>,
    show_file_sidebar: bool,
    show_file_preview_pane: bool,
    show_file_path_bar: bool,
    show_file_status_bar: bool,
    fm_toolbar_search: String,
    fm_view_mode: FileManagerViewMode,
    fm_view_modes_by_path: HashMap<PathBuf, FileManagerViewMode>,
    fm_sort_field: FileManagerSortField,
    fm_icon_scale: f32,
    fm_back_history: Vec<PathBuf>,
    fm_forward_history: Vec<PathBuf>,
    file_info_target: Option<PathBuf>,
    fm_drag_tab_index: Option<usize>,
    fm_tab_scroll: usize,
    // File manager state
    fm_rename_target: Option<PathBuf>,
    fm_rename_buffer: String,
    fm_show_new_dialog: bool,
    fm_new_name: String,
    fm_new_is_dir: bool,
    // Launchpad (Application Grid)
    show_launchpad: bool,
    launchpad_query: String,
    app_catalog: AppCatalog,
    launchpad_page: usize,
    // Embedded Windows apps
    embedded_apps: Vec<EmbeddedApp>,
    embed_launch_input: String,
    show_embed_launcher: bool,
    /// Our own HWND (cached on first frame) — stored as raw pointer on Windows
    #[cfg(windows)]
    own_hwnd: Option<*mut std::ffi::c_void>,
}

#[derive(Clone)]
struct FileManagerTab {
    path: PathBuf,
    entries: Vec<FmEntry>,
    selected_path: Option<PathBuf>,
}

#[derive(Clone)]
struct TerminalTab {
    title: String,
    output: Vec<(String, Color32)>,
    input: String,
}

#[derive(Clone)]
struct NotesTab {
    title: String,
    text: String,
}

#[derive(Clone)]
struct EditorTab {
    title: String,
    file_path: Option<PathBuf>,
    content: String,
    modified: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum PipSource {
    Music,
    Photo(usize),
    Browser { title: String, url: String },
}

#[derive(Clone, Debug)]
struct PipState {
    source: PipSource,
    pos: Pos2,
    size: Vec2,
    last_interaction: Instant,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AssistantOverlayState {
    Idle,
    Listening,
    Thinking,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MusicAudioAction {
    Stop,
    Pause,
    Resume,
    PlayFromOffset,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SpotlightInlineKind {
    Calculation,
    Conversion,
    Definition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpotlightInlineResult {
    kind: SpotlightInlineKind,
    title: String,
    subtitle: String,
    body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpotlightContactHit {
    name: String,
    email: String,
    phone: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpotlightMessageHit {
    contact_name: String,
    snippet: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpotlightEventHit {
    title: String,
    time: String,
    details: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpotlightReminderHit {
    title: String,
    details: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpotlightPreferenceHit {
    title: String,
    subtitle: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FileManagerViewMode {
    Icon,
    List,
    Column,
    Gallery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FileManagerSortField {
    Name,
    Size,
    Kind,
}

impl AuroraDesktopApp {
    fn new() -> Self {
        let daemon_addr =
            std::env::var("AURORA_DAEMON").unwrap_or_else(|_| "127.0.0.1:7878".to_string());
        let auth_token = std::env::var("AURORA_TOKEN").ok().filter(|v| !v.is_empty());
        let loaded_settings = AppSettings::load();
        let show_file_path_bar = loaded_settings.show_file_path_bar;
        let show_file_status_bar = loaded_settings.show_file_status_bar;
        let initial_screen_state = Self::initial_screen_state(&loaded_settings);
        let loaded_profile = UserProfile::load().unwrap_or_else(|| {
            if loaded_settings.user_name.is_empty() {
                UserProfile::default()
            } else {
                UserProfile::from_display_name(
                    &loaded_settings.user_name,
                    (
                        loaded_settings.accent_r,
                        loaded_settings.accent_g,
                        loaded_settings.accent_b,
                    ),
                )
            }
        });
        let sidebar_favorites = Self::load_sidebar_favorites(&loaded_settings);
        let recent_emojis = Self::load_recent_emojis(&loaded_settings);

        let mut app = Self {
            daemon_addr,
            auth_token,
            next_frame_id: 1,
            telemetry: Telemetry::default(),
            sysinfo: RealSystemInfo::new(),
            file_index: FileIndex::new(),
            windows: [
                ManagedWindow::new(Pos2::new(40.0, 60.0), Vec2::new(520.0, 420.0)),   // Overview
                ManagedWindow::new(Pos2::new(280.0, 100.0), Vec2::new(540.0, 360.0)),  // Terminal
                ManagedWindow::new(Pos2::new(620.0, 80.0), Vec2::new(400.0, 380.0)),   // FileManager
                ManagedWindow::new(Pos2::new(340.0, 240.0), Vec2::new(340.0, 280.0)),  // Controls
                ManagedWindow::new(Pos2::new(180.0, 160.0), Vec2::new(380.0, 400.0)),  // Messages
                ManagedWindow::new(Pos2::new(460.0, 50.0), Vec2::new(560.0, 420.0)),   // Browser
                ManagedWindow::new(Pos2::new(460.0, 120.0), Vec2::new(520.0, 520.0)),  // Calculator
                ManagedWindow::new(Pos2::new(100.0, 120.0), Vec2::new(400.0, 360.0)),  // Notes
                ManagedWindow::new(Pos2::new(350.0, 180.0), Vec2::new(340.0, 200.0)),  // MusicPlayer
                ManagedWindow::new(Pos2::new(200.0, 70.0), Vec2::new(480.0, 400.0)),   // Photos
                ManagedWindow::new(Pos2::new(420.0, 90.0), Vec2::new(300.0, 340.0)),   // Calendar
                ManagedWindow::new(Pos2::new(150.0, 80.0), Vec2::new(600.0, 450.0)),  // TextEditor
                ManagedWindow::new(Pos2::new(300.0, 100.0), Vec2::new(500.0, 450.0)), // Settings
                ManagedWindow::new(Pos2::new(250.0, 70.0), Vec2::new(560.0, 420.0)),  // ProcessManager
                ManagedWindow::new(Pos2::new(500.0, 90.0), Vec2::new(520.0, 420.0)),  // Trash
                ManagedWindow::new(Pos2::new(420.0, 110.0), Vec2::new(520.0, 360.0)), // NetworkDiagnostics
                ManagedWindow::new(Pos2::new(280.0, 90.0), Vec2::new(820.0, 520.0)),  // DiskUtility
                ManagedWindow::new(Pos2::new(260.0, 90.0), Vec2::new(700.0, 520.0)),  // Dictionary
                ManagedWindow::new(Pos2::new(220.0, 80.0), Vec2::new(860.0, 560.0)),  // Console
                ManagedWindow::new(Pos2::new(240.0, 70.0), Vec2::new(960.0, 600.0)),  // FontBook
                ManagedWindow::new(Pos2::new(980.0, 90.0), Vec2::new(360.0, 540.0)),  // ColorPicker
            ],
            z_order: vec![
                WindowKind::Overview, WindowKind::FileManager, WindowKind::Browser,
                WindowKind::Messages, WindowKind::Terminal, WindowKind::Controls,
            ],
            focused: Some(WindowKind::Terminal),
            show_control_center: false,
            show_spotlight: false,
            show_assistant: false,
            show_notifications: false,
            show_emoji_picker: false,
            notification_panel_opened_at: None,
            dock_hover_since: None,
            dock_last_exit_at: None,
            spotlight_query: String::new(),
            assistant_query: String::new(),
            cc_wifi: true,
            cc_bluetooth: true,
            cc_airdrop: false,
            cc_focus: false,
            cc_brightness: 0.7,
            cc_volume: 0.5,
            fps_smoothed: 0.0,
            active_menu: None,
            context_menu_pos: None,
            desktop_context_target: None,
            desktop_rename_target: None,
            desktop_rename_buffer: String::new(),
            dock_bounce: None,
            fm_current_dir: dirs_home(),
            fm_entries: read_directory(&dirs_home()),
            fm_selected_path: None,
            fm_tabs: vec![FileManagerTab {
                path: dirs_home(),
                entries: read_directory(&dirs_home()),
                selected_path: None,
            }],
            fm_active_tab: 0,
            menu_action: None,
            spotlight_open_window: None,
            should_quit: false,
            terminal_output: Vec::new(),
            terminal_input: String::new(),
            pty_terminal: None,
            terminal_tabs: vec![TerminalTab {
                title: "Shell 1".to_string(),
                output: Vec::new(),
                input: String::new(),
            }],
            terminal_active_tab: 0,
            use_real_terminal: true,
            cpu_history: VecDeque::with_capacity(120),
            calc_display: "0".to_string(),
            calc_operand: None,
            calc_operator: None,
            calc_reset_next: false,
            calc_mode: CalculatorMode::Basic,
            calc_history: Vec::new(),
            calc_memory: 0.0,
            calc_degrees: true,
            calc_programmer_base: ProgrammerBase::Dec,
            calc_programmer_operand: None,
            calc_programmer_operator: None,
            notes_text: "Welcome to AuroraOS Notes!\n\nYou can type anything here.\nThis is a simple scratchpad.\n\n- Todo: finish the desktop shell\n- Todo: add more apps\n- Todo: write documentation".to_string(),
            notes_tabs: vec![NotesTab {
                title: "Note 1".to_string(),
                text: "Welcome to AuroraOS Notes!\n\nYou can type anything here.\nThis is a simple scratchpad.\n\n- Todo: finish the desktop shell\n- Todo: add more apps\n- Todo: write documentation".to_string(),
            }],
            notes_active_tab: 0,
            music_playing: false,
            music_track_idx: loaded_settings.music_track_idx,
            music_library_query: loaded_settings.music_library_query.clone(),
            music_shuffle: loaded_settings.music_shuffle,
            music_repeat_all: loaded_settings.music_repeat_all,
            music_elapsed_seconds: loaded_settings.music_elapsed_seconds,
            music_last_tick: Instant::now(),
            music_audio: MusicAudioEngine::new().ok(),
            music_override_path: None,
            editor_file_path: None,
            editor_content: String::new(),
            editor_modified: false,
            editor_tabs: vec![EditorTab {
                title: "Untitled".to_string(),
                file_path: None,
                content: String::new(),
                modified: false,
            }],
            editor_active_tab: 0,
            browser_state: BrowserState::new(),
            browser_url_input: String::new(),
            messages_state: MessagesState::new(),
            toast_manager: ToastManager::new(4),
            calendar_month_offset: 0,
            photo_viewer_idx: None,
            pip_state: None,
            pip_dragging: false,
            pip_resizing: false,
            pip_last_pos: None,
            pip_last_size: None,
            show_app_switcher: false,
            app_switcher_idx: 0,
            show_shortcuts_overlay: false,
            show_downloads_stack: false,
            recent_downloads: Vec::new(),
            mc_dragging_window: None,
            spotlight_open_file: None,
            wallpaper_idx: 0,
            desktop_entries: read_directory(&desktop_directory()),
            desktop_selected_paths: Vec::new(),
            desktop_stack_expanded: None,
            desktop_selection_drag_start: None,
            desktop_last_refresh: Instant::now(),
            file_drag_path: None,
            app_phase: AppPhase::Booting,
            boot_started_at: Instant::now(),
            screen_state: initial_screen_state,
            auth_focus_pending: true,
            auth_password: String::new(),
            auth_error: None,
            setup_user_name: loaded_settings.user_name.clone(),
            setup_password: String::new(),
            setup_password_confirm: String::new(),
            setup_step: 0,
            settings_current_password: String::new(),
            settings_new_password: String::new(),
            settings_confirm_password: String::new(),
            settings_password_message: None,
            settings_custom_folder_name: String::new(),
            settings_custom_folder_extension: String::new(),
            settings_custom_folder_min_size_mb: String::new(),
            settings_custom_folder_tag: String::new(),
            selected_tag_filters: Vec::new(),
            tag_filter_match_all: false,
            login_shake: None,
            last_input_at: Instant::now(),
            screensaver_active: false,
            screensaver_kind: ScreenSaverKind::FloatingClock,
            screensaver_started_at: None,
            show_mission_control: false,
            mission_control_anim: 0.0,
            drag_snap_preview: None,
            drag_snap_maximize: false,
            show_wifi_popup: false,
            show_battery_popup: false,
            show_volume_popup: false,
            show_bluetooth_popup: false,
            last_battery_alert_level: 0,
            current_desktop: 0,
            desktop_count: 2,
            clipboard: AppClipboard::new(),
            proc_manager: None,
            network_diagnostics: NetworkDiagnostics::new(),
            disk_utility: DiskUtilityApp::new(),
            dictionary_app: DictionaryApp::new(),
            console_app: ConsoleApp::new(),
            font_book: FontBookApp::new(),
            color_picker: ColorPickerApp::new(
                &loaded_settings.color_picker_saved_colors,
                Color32::from_rgb(
                    loaded_settings.accent_r,
                    loaded_settings.accent_g,
                    loaded_settings.accent_b,
                ),
            ),
            proc_search: String::new(),
            proc_sort_by_cpu: true,
            auto_save: AutoSave::new(30, dirs_home()),
            wallpaper_prev_idx: 0,
            wallpaper_transition: 1.0,
            wallpaper_changing: false,
            confirm_close_window: None,
            notification_center: NotificationCenter::new(),
            collapsed_notification_apps: HashSet::new(),
            app_settings: loaded_settings,
            sidebar_favorites,
            file_tags: FileTags::load(),
            profile_name_buffer: loaded_profile.display_name.clone(),
            user_profile: loaded_profile,
            quick_look_open: false,
            quick_look_paths: Vec::new(),
            quick_look_index: 0,
            emoji_query: String::new(),
            emoji_category: EmojiCategory::Smileys,
            recent_emojis,
            assistant_history: Vec::new(),
            assistant_state: AssistantOverlayState::Idle,
            pending_assistant_query: None,
            photo_texture_cache: HashMap::new(),
            show_file_sidebar: true,
            show_file_preview_pane: true,
            show_file_path_bar,
            show_file_status_bar,
            fm_toolbar_search: String::new(),
            fm_view_mode: FileManagerViewMode::List,
            fm_view_modes_by_path: HashMap::new(),
            fm_sort_field: FileManagerSortField::Name,
            fm_icon_scale: 1.0,
            fm_back_history: Vec::new(),
            fm_forward_history: Vec::new(),
            file_info_target: None,
            fm_drag_tab_index: None,
            fm_tab_scroll: 0,
            fm_rename_target: None,
            fm_rename_buffer: String::new(),
            fm_show_new_dialog: false,
            fm_new_name: String::new(),
            fm_new_is_dir: true,
            show_launchpad: false,
            launchpad_query: String::new(),
            app_catalog: AppCatalog::new(),
            launchpad_page: 0,
            embedded_apps: Vec::new(),
            embed_launch_input: String::new(),
            show_embed_launcher: false,
            #[cfg(windows)]
            own_hwnd: None,
        };
        // New app windows start closed — opened via dock clicks
        app.windows[WindowKind::Calculator as usize].open = false;
        app.windows[WindowKind::Notes as usize].open = false;
        app.windows[WindowKind::MusicPlayer as usize].open = false;
        app.windows[WindowKind::Photos as usize].open = false;
        app.windows[WindowKind::Calendar as usize].open = false;
        app.windows[WindowKind::TextEditor as usize].open = false;
        app.windows[WindowKind::Settings as usize].open = false;
        app.windows[WindowKind::ProcessManager as usize].open = false;
        app.windows[WindowKind::NetworkDiagnostics as usize].open = false;
        app.windows[WindowKind::DiskUtility as usize].open = false;
        app.windows[WindowKind::Dictionary as usize].open = false;
        app.windows[WindowKind::Console as usize].open = false;
        app.windows[WindowKind::FontBook as usize].open = false;
        app.windows[WindowKind::ColorPicker as usize].open = false;
        app.load_state();
        // Apply persisted settings
        app.cc_volume = app.app_settings.volume;
        app.cc_brightness = app.app_settings.brightness;
        app.cc_wifi = app.app_settings.wifi_enabled;
        app.cc_bluetooth = app.app_settings.bluetooth_enabled;
        app.cc_airdrop = app.app_settings.airdrop_enabled;
        app.wallpaper_idx = app.app_settings.wallpaper_idx;
        // Seed notification center
        app.notification_center.seed_defaults();
        // Check for crash recovery files
        if let Some(recovered_notes) = app.auto_save.load_recovery("notes") {
            if !recovered_notes.is_empty() && recovered_notes != app.notes_text {
                app.notes_text = recovered_notes;
                app.sync_active_tab_from_globals(WindowKind::Notes);
                app.toast_manager.push(Toast::new(
                    "Recovery",
                    "Notes restored from auto-save",
                    Color32::from_rgb(255, 149, 0),
                ));
            }
        }
        if let Some(recovered_editor) = app.auto_save.load_recovery("editor") {
            if !recovered_editor.is_empty() {
                app.editor_content = recovered_editor;
                app.editor_modified = true;
                app.sync_active_tab_from_globals(WindowKind::TextEditor);
                app.toast_manager.push(Toast::new(
                    "Recovery",
                    "Editor content restored from auto-save",
                    Color32::from_rgb(255, 149, 0),
                ));
            }
        }
        // Initialize real PTY terminal
        if app.use_real_terminal {
            match PtyTerminal::new() {
                Some(pty) => {
                    app.pty_terminal = Some(pty);
                    app.toast_manager.push(Toast::new(
                        "Terminal Ready",
                        "Real shell connected",
                        Color32::from_rgb(52, 199, 89),
                    ));
                    app.notification_center.notify(
                        "System",
                        "Terminal Ready",
                        "Real shell connected",
                        Color32::from_rgb(52, 199, 89),
                    );
                }
                None => {
                    app.toast_manager.push(Toast::new(
                        "Terminal",
                        "Using built-in shell (PTY unavailable)",
                        Color32::from_rgb(255, 149, 0),
                    ));
                }
            }
        }
        app
    }

    fn avatar_color(&self) -> Color32 {
        Color32::from_rgb(
            self.user_profile.avatar_r,
            self.user_profile.avatar_g,
            self.user_profile.avatar_b,
        )
    }

    fn avatar_initials(&self) -> &str {
        if self.user_profile.avatar_initials.is_empty() {
            "A"
        } else {
            self.user_profile.avatar_initials.as_str()
        }
    }

    fn profile_display_name(&self) -> &str {
        if self.user_profile.display_name.is_empty() {
            if self.app_settings.user_name.is_empty() {
                "Aurora User"
            } else {
                self.app_settings.user_name.as_str()
            }
        } else {
            self.user_profile.display_name.as_str()
        }
    }

    fn default_sidebar_favorites() -> Vec<PathBuf> {
        let home = dirs_home();
        vec![
            home.clone(),
            desktop_directory(),
            home.join("Documents"),
            home.join("Downloads"),
            home.join("Pictures"),
            trash_dir(),
        ]
    }

    fn load_sidebar_favorites(settings: &AppSettings) -> Vec<PathBuf> {
        if settings.favorite_paths.is_empty() {
            return Self::default_sidebar_favorites();
        }
        let parsed = settings
            .favorite_paths
            .split('|')
            .filter(|entry| !entry.trim().is_empty())
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        if parsed.is_empty() {
            Self::default_sidebar_favorites()
        } else {
            parsed
        }
    }

    fn persist_sidebar_favorites(&mut self) {
        self.app_settings.favorite_paths = self
            .sidebar_favorites
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("|");
    }

    fn load_recent_emojis(settings: &AppSettings) -> Vec<String> {
        settings
            .recent_emojis
            .split('|')
            .filter(|entry| !entry.trim().is_empty())
            .map(|entry| entry.to_string())
            .collect()
    }

    fn persist_recent_emojis(&mut self) {
        self.app_settings.recent_emojis = self.recent_emojis.join("|");
    }

    fn persist_music_state(&mut self) {
        self.app_settings.music_track_idx = self.music_track_idx;
        self.app_settings.music_library_query = self.music_library_query.clone();
        self.app_settings.music_shuffle = self.music_shuffle;
        self.app_settings.music_repeat_all = self.music_repeat_all;
        self.app_settings.music_elapsed_seconds = self.music_elapsed_seconds;
    }

    fn desired_music_audio_action(
        active_path: Option<&std::path::Path>,
        target_path: Option<&std::path::Path>,
        should_play: bool,
        force_restart: bool,
    ) -> MusicAudioAction {
        match (active_path, target_path, should_play, force_restart) {
            (_, None, _, _) => MusicAudioAction::Stop,
            (_, Some(_), false, _) => MusicAudioAction::Pause,
            (_, Some(_), true, true) => MusicAudioAction::PlayFromOffset,
            (Some(active), Some(target), true, false) if active == target => {
                MusicAudioAction::Resume
            }
            (Some(_), Some(_), true, false) | (None, Some(_), true, false) => {
                MusicAudioAction::PlayFromOffset
            }
        }
    }

    fn parse_tag_labels(settings: &AppSettings) -> Vec<(TagColor, String)> {
        settings
            .tag_labels
            .split('|')
            .filter_map(|entry| {
                let (color, label) = entry.split_once(':')?;
                Some((TagColor::from_str(color)?, label.to_string()))
            })
            .collect()
    }

    fn tag_label(&self, color: TagColor) -> String {
        Self::parse_tag_labels(&self.app_settings)
            .into_iter()
            .find(|(stored_color, _)| *stored_color == color)
            .map(|(_, label)| label)
            .filter(|label| !label.trim().is_empty())
            .unwrap_or_else(|| {
                match color {
                    TagColor::Red => "Red",
                    TagColor::Orange => "Orange",
                    TagColor::Yellow => "Yellow",
                    TagColor::Green => "Green",
                    TagColor::Blue => "Blue",
                    TagColor::Purple => "Purple",
                    TagColor::Gray => "Gray",
                }
                .to_string()
            })
    }

    fn set_tag_label(&mut self, color: TagColor, label: &str) {
        let mut labels = Self::parse_tag_labels(&self.app_settings);
        if let Some((_, current)) = labels
            .iter_mut()
            .find(|(stored_color, _)| *stored_color == color)
        {
            *current = label.trim().to_string();
        } else {
            labels.push((color, label.trim().to_string()));
        }
        self.app_settings.tag_labels = labels
            .into_iter()
            .filter(|(_, stored_label)| !stored_label.is_empty())
            .map(|(stored_color, stored_label)| {
                format!("{}:{}", stored_color.as_str(), stored_label)
            })
            .collect::<Vec<_>>()
            .join("|");
    }

    fn load_custom_smart_folders(settings: &AppSettings) -> Vec<CustomSmartFolder> {
        settings
            .custom_smart_folders
            .split('|')
            .filter(|entry| !entry.trim().is_empty())
            .filter_map(|entry| {
                let parts = entry.splitn(4, ';').collect::<Vec<_>>();
                if parts.len() != 4 || parts[0].trim().is_empty() {
                    return None;
                }
                Some(CustomSmartFolder {
                    name: parts[0].trim().to_string(),
                    extension: (!parts[1].trim().is_empty()).then(|| parts[1].trim().to_string()),
                    min_size_mb: parts[2].trim().parse::<u64>().ok(),
                    tag: (!parts[3].trim().is_empty()).then(|| parts[3].trim().to_string()),
                })
            })
            .collect()
    }

    fn save_custom_smart_folders(&mut self, folders: &[CustomSmartFolder]) {
        self.app_settings.custom_smart_folders = folders
            .iter()
            .map(|folder| {
                format!(
                    "{};{};{};{}",
                    folder.name,
                    folder.extension.clone().unwrap_or_default(),
                    folder
                        .min_size_mb
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                    folder.tag.clone().unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>()
            .join("|");
    }

    fn smart_folder_title(&self, token: &str) -> String {
        if let Some(name) = token.strip_prefix("custom_") {
            return name.replace('_', " ");
        }
        match token {
            "images" => "All Images".to_string(),
            "documents" => "All Documents".to_string(),
            "recent" => "Recent Files".to_string(),
            "large" => "Large Files".to_string(),
            "tag_red" => format!("Tagged {}", self.tag_label(TagColor::Red)),
            "tag_orange" => format!("Tagged {}", self.tag_label(TagColor::Orange)),
            "tag_yellow" => format!("Tagged {}", self.tag_label(TagColor::Yellow)),
            "tag_green" => format!("Tagged {}", self.tag_label(TagColor::Green)),
            "tag_blue" => format!("Tagged {}", self.tag_label(TagColor::Blue)),
            "tag_purple" => format!("Tagged {}", self.tag_label(TagColor::Purple)),
            "tag_gray" => format!("Tagged {}", self.tag_label(TagColor::Gray)),
            _ if token.starts_with("tags_any_") => "Tagged Any".to_string(),
            _ if token.starts_with("tags_all_") => "Tagged All".to_string(),
            _ => "Smart Folder".to_string(),
        }
    }

    fn smart_folder_entries_for_token(
        &self,
        token: &str,
        root: &std::path::Path,
        file_tags: &FileTags,
    ) -> Vec<FmEntry> {
        match token {
            "images" => smart_folder_entries(SmartFolderKind::AllImages, root),
            "documents" => smart_folder_entries(SmartFolderKind::AllDocuments, root),
            "recent" => smart_folder_entries(SmartFolderKind::RecentFiles, root),
            "large" => smart_folder_entries(SmartFolderKind::LargeFiles, root),
            "tag_red" => file_tags.entries_with_tag(TagColor::Red, root),
            "tag_orange" => file_tags.entries_with_tag(TagColor::Orange, root),
            "tag_yellow" => file_tags.entries_with_tag(TagColor::Yellow, root),
            "tag_green" => file_tags.entries_with_tag(TagColor::Green, root),
            "tag_blue" => file_tags.entries_with_tag(TagColor::Blue, root),
            "tag_purple" => file_tags.entries_with_tag(TagColor::Purple, root),
            "tag_gray" => file_tags.entries_with_tag(TagColor::Gray, root),
            _ if token.starts_with("tags_any_") || token.starts_with("tags_all_") => {
                let match_all = token.starts_with("tags_all_");
                let tag_blob = token
                    .strip_prefix("tags_any_")
                    .or_else(|| token.strip_prefix("tags_all_"))
                    .unwrap_or("");
                let colors = tag_blob
                    .split(',')
                    .filter_map(TagColor::from_str)
                    .collect::<Vec<_>>();
                file_tags.entries_with_tags(&colors, match_all, root)
            }
            _ if token.starts_with("custom_") => {
                let custom_name = token.trim_start_matches("custom_").replace('_', " ");
                let folder = Self::load_custom_smart_folders(&self.app_settings)
                    .into_iter()
                    .find(|folder| folder.name == custom_name);
                if let Some(folder) = folder {
                    let mut entries = custom_smart_folder_entries(&folder, root);
                    if let Some(tag) = folder.tag.as_deref().and_then(TagColor::from_str) {
                        entries.retain(|entry| file_tags.get(&entry.path).contains(&tag));
                    }
                    entries
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }

    fn tag_filter_token(colors: &[TagColor], match_all: bool) -> Option<String> {
        if colors.is_empty() {
            return None;
        }
        let prefix = if match_all { "tags_all_" } else { "tags_any_" };
        let blob = colors
            .iter()
            .map(|color| color.as_str())
            .collect::<Vec<_>>()
            .join(",");
        Some(format!("{prefix}{blob}"))
    }

    fn drag_uses_copy_modifier(modifiers: egui::Modifiers) -> bool {
        modifiers.ctrl
    }

    fn active_drag_count(&self, drag_path: &std::path::Path) -> usize {
        if self
            .desktop_selected_paths
            .iter()
            .any(|selected| selected == drag_path)
        {
            self.desktop_selected_paths.len().max(1)
        } else {
            1
        }
    }

    fn dock_icon_accepts_file_drop(icon: DockIcon) -> bool {
        !icon.is_separator()
    }

    fn dock_position(settings: &AppSettings) -> DockPosition {
        DockPosition::from_str(&settings.dock_position)
    }

    fn dock_hovered_for_position(
        position: DockPosition,
        screen: Rect,
        pointer: Option<Pos2>,
    ) -> bool {
        let Some(pointer) = pointer else {
            return false;
        };
        match position {
            DockPosition::Bottom => pointer.y >= screen.bottom() - DOCK_HEIGHT - 20.0,
            DockPosition::Left => pointer.x <= screen.left() + DOCK_HEIGHT,
            DockPosition::Right => pointer.x >= screen.right() - DOCK_HEIGHT,
        }
    }

    fn dock_hidden_offset(auto_hide: bool, hovered: bool, elapsed: Duration) -> f32 {
        if !auto_hide {
            return 0.0;
        }
        if hovered {
            let t = (elapsed.as_secs_f32() / 0.2).clamp(0.0, 1.0);
            1.0 - t
        } else {
            1.0
        }
    }

    fn update_dock_hover_state(&mut self, hovered: bool, now: Instant) -> Duration {
        if hovered {
            let since = self.dock_hover_since.get_or_insert(now);
            self.dock_last_exit_at = None;
            now.saturating_duration_since(*since)
        } else {
            self.dock_hover_since = None;
            let since = self.dock_last_exit_at.get_or_insert(now);
            now.saturating_duration_since(*since)
        }
    }

    fn dock_panel_rect(screen: Rect, position: DockPosition, hidden_factor: f32) -> Rect {
        match position {
            DockPosition::Bottom => {
                let y = screen.bottom() - DOCK_HEIGHT * (1.0 - hidden_factor * 0.85);
                Rect::from_min_size(
                    Pos2::new(screen.left(), y),
                    Vec2::new(screen.width(), DOCK_HEIGHT),
                )
            }
            DockPosition::Left => {
                let x = screen.left() - DOCK_HEIGHT * hidden_factor * 0.85;
                Rect::from_min_size(
                    Pos2::new(x, screen.top() + MENU_BAR_HEIGHT),
                    Vec2::new(DOCK_HEIGHT, screen.height() - MENU_BAR_HEIGHT),
                )
            }
            DockPosition::Right => {
                let x = screen.right() - DOCK_HEIGHT * (1.0 - hidden_factor * 0.85);
                Rect::from_min_size(
                    Pos2::new(x, screen.top() + MENU_BAR_HEIGHT),
                    Vec2::new(DOCK_HEIGHT, screen.height() - MENU_BAR_HEIGHT),
                )
            }
        }
    }

    fn spotlight_inline_result(query: &str) -> Option<SpotlightInlineResult> {
        Self::spotlight_calc_result(query)
            .or_else(|| Self::spotlight_conversion_result(query))
            .or_else(|| Self::spotlight_definition_result(query))
    }

    fn spotlight_calc_result(query: &str) -> Option<SpotlightInlineResult> {
        let trimmed = query.trim();
        if let Some(rest) = trimmed.strip_suffix("%") {
            let value = rest.trim().parse::<f64>().ok()?;
            return Some(SpotlightInlineResult {
                kind: SpotlightInlineKind::Calculation,
                title: format!("{}%", format_calc(value)),
                subtitle: "Percentage".to_string(),
                body: format_calc(value / 100.0),
            });
        }
        if let Some((percent, of_value)) = trimmed.split_once("% of ") {
            let percent = percent.trim().parse::<f64>().ok()?;
            let of_value = of_value.trim().parse::<f64>().ok()?;
            let result = (percent / 100.0) * of_value;
            return Some(SpotlightInlineResult {
                kind: SpotlightInlineKind::Calculation,
                title: trimmed.to_string(),
                subtitle: "Calculation".to_string(),
                body: format_calc(result),
            });
        }
        if let Some(inner) = trimmed
            .strip_prefix("sqrt(")
            .and_then(|value| value.strip_suffix(')'))
        {
            let value = inner.trim().parse::<f64>().ok()?;
            return Some(SpotlightInlineResult {
                kind: SpotlightInlineKind::Calculation,
                title: trimmed.to_string(),
                subtitle: "Square Root".to_string(),
                body: format_calc(value.sqrt()),
            });
        }
        if let Some(inner) = trimmed
            .strip_prefix("pow(")
            .and_then(|value| value.strip_suffix(')'))
        {
            let (base, exp) = inner.split_once(',')?;
            let base = base.trim().parse::<f64>().ok()?;
            let exp = exp.trim().parse::<f64>().ok()?;
            return Some(SpotlightInlineResult {
                kind: SpotlightInlineKind::Calculation,
                title: trimmed.to_string(),
                subtitle: "Power".to_string(),
                body: format_calc(base.powf(exp)),
            });
        }
        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        if parts.len() == 3 {
            let left = parts[0].parse::<f64>().ok()?;
            let op = parts[1].chars().next()?;
            let right = parts[2].parse::<f64>().ok()?;
            if matches!(op, '+' | '-' | '*' | '/') {
                return Some(SpotlightInlineResult {
                    kind: SpotlightInlineKind::Calculation,
                    title: trimmed.to_string(),
                    subtitle: "Calculation".to_string(),
                    body: format_calc(calc_eval(left, op, right)),
                });
            }
        }
        None
    }

    fn spotlight_conversion_result(query: &str) -> Option<SpotlightInlineResult> {
        let parts = query.trim().split_whitespace().collect::<Vec<_>>();
        if parts.len() != 4 || parts[2].to_ascii_lowercase() != "in" {
            return None;
        }
        let value = parts[0].parse::<f64>().ok()?;
        let from = parts[1].to_ascii_lowercase();
        let to = parts[3].to_ascii_lowercase();
        let result = match (from.as_str(), to.as_str()) {
            ("km", "miles") | ("kilometers", "miles") => value * 0.621371,
            ("miles", "km") => value / 0.621371,
            ("lbs", "kg") | ("pounds", "kg") => value * 0.453592,
            ("kg", "lbs") => value / 0.453592,
            ("f", "c") | ("°f", "celsius") | ("f°", "celsius") => (value - 32.0) * 5.0 / 9.0,
            ("c", "f") | ("celsius", "°f") | ("celsius", "f") => (value * 9.0 / 5.0) + 32.0,
            _ => return None,
        };
        Some(SpotlightInlineResult {
            kind: SpotlightInlineKind::Conversion,
            title: query.trim().to_string(),
            subtitle: "Conversion".to_string(),
            body: format_calc(result),
        })
    }

    fn spotlight_definition_result(query: &str) -> Option<SpotlightInlineResult> {
        let word = query
            .trim()
            .strip_prefix("define ")?
            .trim()
            .to_ascii_lowercase();
        let definition = dictionary_inline_definition(&word).unwrap_or_else(|| {
            "No local definition found. Open Dictionary for a full entry.".to_string()
        });
        Some(SpotlightInlineResult {
            kind: SpotlightInlineKind::Definition,
            title: format!("define {}", word),
            subtitle: "Definition".to_string(),
            body: definition,
        })
    }

    fn spotlight_top_hit_label(
        query: &str,
        app_hits: usize,
        file_hits: usize,
        inline_hit: bool,
    ) -> Option<&'static str> {
        if inline_hit {
            Some("Top Hit")
        } else if app_hits > 0 {
            Some("Top Hit")
        } else if file_hits > 0 || !query.trim().is_empty() {
            Some("Top Hit")
        } else {
            None
        }
    }

    fn spotlight_contact_hits(&self, query: &str) -> Vec<SpotlightContactHit> {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        self.messages_state
            .conversations
            .iter()
            .enumerate()
            .filter_map(|(idx, conversation)| {
                let email = format!(
                    "{}@aurora.local",
                    conversation
                        .contact_name
                        .to_ascii_lowercase()
                        .replace(' ', ".")
                );
                let phone = format!("555-01{:02}", idx + 1);
                let matches = conversation
                    .contact_name
                    .to_ascii_lowercase()
                    .contains(&query)
                    || email.contains(&query)
                    || phone.contains(&query);
                matches.then(|| SpotlightContactHit {
                    name: conversation.contact_name.clone(),
                    email,
                    phone,
                })
            })
            .take(4)
            .collect()
    }

    fn spotlight_message_hits(&self, query: &str) -> Vec<SpotlightMessageHit> {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        for conversation in &self.messages_state.conversations {
            for message in conversation.messages.iter().rev() {
                if message.text.to_ascii_lowercase().contains(&query)
                    || conversation
                        .contact_name
                        .to_ascii_lowercase()
                        .contains(&query)
                {
                    hits.push(SpotlightMessageHit {
                        contact_name: conversation.contact_name.clone(),
                        snippet: message.text.clone(),
                    });
                    break;
                }
            }
            if hits.len() >= 4 {
                break;
            }
        }
        hits
    }

    fn spotlight_calendar_hits(&self, query: &str) -> Vec<SpotlightEventHit> {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        [
            ("Team Standup", "9:00 AM", "Conference Room B"),
            ("Code Review", "2:00 PM", "Desktop shell milestone"),
            ("Gym", "5:30 PM", "Downtown Fitness Club"),
        ]
        .into_iter()
        .filter(|(title, time, details)| {
            title.to_ascii_lowercase().contains(&query)
                || time.to_ascii_lowercase().contains(&query)
                || details.to_ascii_lowercase().contains(&query)
        })
        .map(|(title, time, details)| SpotlightEventHit {
            title: title.to_string(),
            time: time.to_string(),
            details: details.to_string(),
        })
        .collect()
    }

    fn spotlight_reminder_hits(query: &str) -> Vec<SpotlightReminderHit> {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        [
            ("Review pull requests", "Engineering"),
            ("Send prototype build", "Alice"),
            ("Refill coffee beans", "Office"),
        ]
        .into_iter()
        .filter(|(title, details)| {
            title.to_ascii_lowercase().contains(&query)
                || details.to_ascii_lowercase().contains(&query)
        })
        .map(|(title, details)| SpotlightReminderHit {
            title: title.to_string(),
            details: details.to_string(),
        })
        .collect()
    }

    fn spotlight_system_preference_hits(query: &str) -> Vec<SpotlightPreferenceHit> {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        [
            ("Appearance", "Theme, accent color, wallpaper"),
            ("Security & Privacy", "Lock screen, password, permissions"),
            ("Notifications", "Alerts, badges, notification grouping"),
            ("Mouse & Trackpad", "Pointer speed and gestures"),
            ("Energy Saver", "Auto-lock and power management"),
        ]
        .into_iter()
        .filter(|(title, subtitle)| {
            title.to_ascii_lowercase().contains(&query)
                || subtitle.to_ascii_lowercase().contains(&query)
        })
        .map(|(title, subtitle)| SpotlightPreferenceHit {
            title: title.to_string(),
            subtitle: subtitle.to_string(),
        })
        .collect()
    }

    fn desktop_drop_target(
        drag_path: &std::path::Path,
        pointer: Option<Pos2>,
        work_rect: Rect,
    ) -> Option<PathBuf> {
        let pointer = pointer?;
        if !work_rect.contains(pointer) {
            return None;
        }
        let target = desktop_directory();
        if drag_path.parent() == Some(target.as_path()) {
            return None;
        }
        Some(target)
    }

    fn favorite_drop_target(
        drag_path: &std::path::Path,
        favorite_path: &std::path::Path,
        trash_path: &std::path::Path,
    ) -> Option<PathBuf> {
        if favorite_path == trash_path {
            return Some(favorite_path.to_path_buf());
        }
        if !favorite_path.is_dir() || drag_path.parent() == Some(favorite_path) {
            return None;
        }
        Some(favorite_path.to_path_buf())
    }

    fn current_folder_drop_target(
        drag_path: &std::path::Path,
        current_dir: &std::path::Path,
    ) -> Option<PathBuf> {
        if !current_dir.is_dir() || drag_path.parent() == Some(current_dir) {
            return None;
        }
        Some(current_dir.to_path_buf())
    }

    fn directory_row_drop_target(
        drag_path: &std::path::Path,
        target_dir: &std::path::Path,
    ) -> Option<PathBuf> {
        if !target_dir.is_dir() || drag_path.parent() == Some(target_dir) {
            return None;
        }
        Some(target_dir.to_path_buf())
    }

    fn desktop_context_menu_items(has_item_target: bool) -> &'static [&'static str] {
        if has_item_target {
            &["Open", "Rename", "Copy", "Move to Trash", "---", "Get Info"]
        } else {
            &[
                "New Folder",
                "New File",
                "---",
                "Get Info",
                "Change Wallpaper",
                "Use Stacks",
                "---",
                "Show Desktop",
                "Mission Control",
                "---",
                "Open Terminal Here",
                "Start Screen Saver",
                "Keyboard Shortcuts",
            ]
        }
    }

    fn screensaver_photo_candidates(home: &std::path::Path) -> Vec<PathBuf> {
        let pictures = home.join("Pictures");
        if !pictures.exists() {
            return Vec::new();
        }
        let mut paths = read_directory(&pictures)
            .into_iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| entry.path)
            .collect::<Vec<_>>();
        paths.sort();
        photo_slideshow_paths(&paths)
    }

    fn photo_library_paths(home: &std::path::Path) -> Vec<PathBuf> {
        let pictures = home.join("Pictures");
        if !pictures.exists() {
            return Vec::new();
        }
        let mut paths = read_directory(&pictures)
            .into_iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| entry.path)
            .collect::<Vec<_>>();
        paths.sort();
        photo_slideshow_paths(&paths)
    }

    fn music_library_paths(home: &std::path::Path) -> Vec<PathBuf> {
        let music = home.join("Music");
        if !music.exists() {
            return Vec::new();
        }
        let mut paths = read_directory(&music)
            .into_iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| entry.path)
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| {
                        matches!(
                            ext.to_ascii_lowercase().as_str(),
                            "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac"
                        )
                    })
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    fn is_supported_audio_path(path: &std::path::Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac"
                )
            })
            .unwrap_or(false)
    }

    fn is_supported_video_path(path: &std::path::Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "mp4" | "mov" | "mkv" | "avi" | "wmv" | "webm" | "m4v"
                )
            })
            .unwrap_or(false)
    }

    fn music_library_track_idx_for_path(
        real_tracks: &[PathBuf],
        path: &std::path::Path,
    ) -> Option<usize> {
        real_tracks.iter().position(|track| track == path)
    }

    fn current_music_path(&self, real_tracks: &[PathBuf]) -> Option<PathBuf> {
        if let Some(path) = &self.music_override_path {
            Some(path.clone())
        } else if real_tracks.is_empty() {
            None
        } else {
            Some(
                real_tracks[Self::normalize_music_track_idx(self.music_track_idx, real_tracks.len())]
                    .clone(),
            )
        }
    }

    fn open_audio_path_in_music_player(&mut self, path: PathBuf) {
        let real_tracks = Self::music_library_paths(&dirs_home());
        if let Some(idx) = Self::music_library_track_idx_for_path(&real_tracks, &path) {
            self.music_track_idx = idx;
            self.music_override_path = None;
        } else {
            self.music_override_path = Some(path);
        }
        self.music_playing = true;
        self.music_elapsed_seconds = 0.0;
        self.music_last_tick = Instant::now();
        self.persist_music_state();
        self.sync_music_audio(true);
        let win = self.window_mut(WindowKind::MusicPlayer);
        win.open = true;
        win.minimized = false;
        self.bring_to_front(WindowKind::MusicPlayer);
    }

    fn open_video_path_in_quick_look(&mut self, path: PathBuf) {
        self.quick_look_paths = vec![path];
        self.quick_look_index = 0;
        self.quick_look_open = true;
    }

    fn open_url_in_browser(&mut self, url: &str) {
        self.browser_state.navigate(url);
        self.browser_url_input = self.browser_state.url.clone();
        let win = self.window_mut(WindowKind::Browser);
        win.open = true;
        win.minimized = false;
        self.bring_to_front(WindowKind::Browser);
    }

    fn looks_like_browser_target(target: &str) -> bool {
        let trimmed = target.trim();
        trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("auroraos://")
            || trimmed.contains('.')
    }

    fn open_path_in_aurora_if_supported(&mut self, path: &std::path::Path) -> bool {
        if Self::is_supported_audio_path(path) {
            self.open_audio_path_in_music_player(path.to_path_buf());
            true
        } else if Self::is_supported_video_path(path) {
            self.open_video_path_in_quick_look(path.to_path_buf());
            true
        } else {
            false
        }
    }

    fn decode_photo_color_image(path: &std::path::Path) -> Option<ColorImage> {
        let image = image::open(path).ok()?;
        let image = if image.width() > 512 || image.height() > 512 {
            image.thumbnail(512, 512)
        } else {
            image
        }
        .to_rgba8();
        let size = [image.width() as usize, image.height() as usize];
        let pixels = image.as_flat_samples();
        Some(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
    }

    fn photo_texture_for_path(
        &mut self,
        ctx: &egui::Context,
        path: &std::path::Path,
    ) -> Option<TextureHandle> {
        if let Some(texture) = self.photo_texture_cache.get(path) {
            return Some(texture.clone());
        }
        let image = Self::decode_photo_color_image(path)?;
        let texture = ctx.load_texture(
            format!("photo:{}", path.display()),
            image,
            TextureOptions::LINEAR,
        );
        self.photo_texture_cache
            .insert(path.to_path_buf(), texture.clone());
        Some(texture)
    }

    fn photo_metadata_label(path: &std::path::Path) -> Option<String> {
        let (width, height) = image::image_dimensions(path).ok()?;
        let bytes = std::fs::metadata(path).ok()?.len();
        Some(format!("{width}x{height}  {}", format_size(bytes)))
    }

    fn music_track_title(path: &std::path::Path) -> String {
        path.file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Audio Track")
            .to_string()
    }

    fn music_track_metadata_label(path: &std::path::Path) -> Option<String> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_uppercase())
            .unwrap_or_else(|| "AUDIO".to_string());
        let bytes = std::fs::metadata(path).ok()?.len();
        Some(format!("{extension}  {}", format_size(bytes)))
    }

    fn music_track_color_for_path(path: &std::path::Path) -> Color32 {
        let mut hash = 0_u32;
        for byte in path.to_string_lossy().bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
        }
        Color32::from_rgb(
            80 + (hash & 0x3F) as u8,
            90 + ((hash >> 6) & 0x4F) as u8,
            140 + ((hash >> 12) & 0x5F) as u8,
        )
    }

    fn desktop_rename_destination(target: &std::path::Path, new_name: &str) -> Option<PathBuf> {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(target.parent()?.join(trimmed))
    }

    fn toggle_desktop_selection(selected_paths: &mut Vec<PathBuf>, path: &std::path::Path) {
        if let Some(index) = selected_paths.iter().position(|selected| selected == path) {
            selected_paths.remove(index);
        } else {
            selected_paths.push(path.to_path_buf());
        }
    }

    fn replace_desktop_selection(selected_paths: &mut Vec<PathBuf>, path: &std::path::Path) {
        selected_paths.clear();
        selected_paths.push(path.to_path_buf());
    }

    fn select_all_desktop_entries(entries: &[FmEntry]) -> Vec<PathBuf> {
        entries.iter().map(|entry| entry.path.clone()).collect()
    }

    fn desktop_icon_rects(work_rect: Rect, count: usize) -> Vec<Rect> {
        Self::desktop_icon_positions(work_rect, count)
            .into_iter()
            .map(|pos| Rect::from_min_size(pos, Vec2::new(72.0, 76.0)))
            .collect()
    }

    fn desktop_selection_rect(start: Pos2, current: Pos2) -> Rect {
        Rect::from_two_pos(start, current)
    }

    fn desktop_paths_in_selection_rect(
        entries: &[FmEntry],
        icon_rects: &[Rect],
        selection_rect: Rect,
    ) -> Vec<PathBuf> {
        entries
            .iter()
            .zip(icon_rects.iter())
            .filter(|(_, rect)| rect.intersects(selection_rect))
            .map(|(entry, _)| entry.path.clone())
            .collect()
    }

    fn desktop_stack_name(entry: &FmEntry) -> String {
        if entry.is_dir {
            return "Folders".to_string();
        }
        let ext = entry
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" => "Images".to_string(),
            "pdf" | "doc" | "docx" | "txt" | "md" | "rtf" => "Documents".to_string(),
            "mp4" | "mov" | "mkv" | "avi" => "Videos".to_string(),
            "mp3" | "wav" | "flac" => "Audio".to_string(),
            "zip" | "rar" | "7z" => "Archives".to_string(),
            _ => "Other".to_string(),
        }
    }

    fn desktop_stacks(entries: &[FmEntry]) -> Vec<(String, Vec<FmEntry>)> {
        let mut groups = std::collections::BTreeMap::<String, Vec<FmEntry>>::new();
        for entry in entries {
            groups
                .entry(Self::desktop_stack_name(entry))
                .or_default()
                .push(entry.clone());
        }
        groups.into_iter().collect()
    }

    fn make_file_manager_tab(path: PathBuf) -> FileManagerTab {
        let entries = read_directory(&path);
        FileManagerTab {
            path,
            entries,
            selected_path: None,
        }
    }

    fn sync_file_manager_from_active_tab(&mut self) {
        if let Some(tab) = self.fm_tabs.get(self.fm_active_tab).cloned() {
            self.fm_current_dir = tab.path;
            self.fm_entries = tab.entries;
            self.fm_selected_path = tab.selected_path;
            self.sync_file_manager_view_mode_for_current_dir();
        }
    }

    fn sync_active_tab_from_file_manager(&mut self) {
        if let Some(tab) = self.fm_tabs.get_mut(self.fm_active_tab) {
            tab.path = self.fm_current_dir.clone();
            tab.entries = self.fm_entries.clone();
            tab.selected_path = self.fm_selected_path.clone();
        }
    }

    fn open_file_manager_tab(&mut self, path: PathBuf) {
        self.sync_active_tab_from_file_manager();
        self.fm_tabs.push(Self::make_file_manager_tab(path));
        self.fm_active_tab = self.fm_tabs.len().saturating_sub(1);
        self.fm_tab_scroll = Self::clamp_tab_scroll(self.fm_active_tab, self.fm_tabs.len(), 6);
        self.sync_file_manager_from_active_tab();
    }

    fn close_file_manager_tab(&mut self, index: usize) {
        if self.fm_tabs.len() <= 1 || index >= self.fm_tabs.len() {
            return;
        }
        self.fm_tabs.remove(index);
        if self.fm_active_tab >= self.fm_tabs.len() {
            self.fm_active_tab = self.fm_tabs.len().saturating_sub(1);
        } else if index < self.fm_active_tab {
            self.fm_active_tab = self.fm_active_tab.saturating_sub(1);
        }
        self.fm_tab_scroll = Self::clamp_tab_scroll(
            self.fm_tab_scroll.min(self.fm_active_tab),
            self.fm_tabs.len(),
            6,
        );
        self.sync_file_manager_from_active_tab();
    }

    fn switch_file_manager_tab(&mut self, index: usize) {
        if index >= self.fm_tabs.len() {
            return;
        }
        self.sync_active_tab_from_file_manager();
        self.fm_active_tab = index;
        self.fm_tab_scroll = Self::clamp_tab_scroll(self.fm_active_tab, self.fm_tabs.len(), 6);
        self.sync_file_manager_from_active_tab();
    }

    fn move_file_manager_tab(&mut self, from: usize, to: usize) {
        if from >= self.fm_tabs.len() || to >= self.fm_tabs.len() || from == to {
            return;
        }
        self.sync_active_tab_from_file_manager();
        let tab = self.fm_tabs.remove(from);
        self.fm_tabs.insert(to, tab);
        self.fm_active_tab = if self.fm_active_tab == from {
            to
        } else if from < self.fm_active_tab && to >= self.fm_active_tab {
            self.fm_active_tab.saturating_sub(1)
        } else if from > self.fm_active_tab && to <= self.fm_active_tab {
            self.fm_active_tab.saturating_add(1)
        } else {
            self.fm_active_tab
        };
        self.sync_file_manager_from_active_tab();
        self.fm_tab_scroll = Self::clamp_tab_scroll(self.fm_tab_scroll, self.fm_tabs.len(), 6);
    }

    fn file_manager_directory_navigation(path: &std::path::Path, open_in_new_tab: bool) -> PathBuf {
        if open_in_new_tab {
            PathBuf::from(format!("__OPEN_TAB__{}", path.display()))
        } else {
            path.to_path_buf()
        }
    }

    fn file_manager_open_entry_navigation(
        entry: &FmEntry,
        open_in_new_tab: bool,
    ) -> Option<PathBuf> {
        if entry.is_dir {
            Some(Self::file_manager_directory_navigation(
                &entry.path,
                open_in_new_tab,
            ))
        } else {
            let ext = entry
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let is_text = matches!(
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
                    | "gitignore"
                    | "lock"
                    | "sql"
                    | "lua"
                    | "rb"
            );
            if is_text {
                Some(PathBuf::from(format!(
                    "__OPEN_EDITOR__{}",
                    entry.path.display()
                )))
            } else if Self::is_supported_audio_path(&entry.path) {
                Some(PathBuf::from(format!(
                    "__OPEN_MUSIC__{}",
                    entry.path.display()
                )))
            } else if Self::is_supported_video_path(&entry.path) {
                Some(PathBuf::from(format!(
                    "__OPEN_VIDEO__{}",
                    entry.path.display()
                )))
            } else {
                open_file_with_system(&entry.path);
                None
            }
        }
    }

    fn visible_tab_range(
        tab_count: usize,
        scroll: usize,
        visible_count: usize,
    ) -> std::ops::Range<usize> {
        let start = Self::clamp_tab_scroll(scroll, tab_count, visible_count);
        let end = (start + visible_count).min(tab_count);
        start..end
    }

    fn clamp_tab_scroll(scroll: usize, tab_count: usize, visible_count: usize) -> usize {
        tab_count.saturating_sub(visible_count).min(scroll)
    }

    fn initial_screen_state(settings: &AppSettings) -> AppScreenState {
        if settings.has_user_profile() {
            AppScreenState::Login
        } else {
            AppScreenState::Setup
        }
    }

    fn boot_progress(elapsed: Duration) -> f32 {
        (elapsed.as_secs_f32() / 2.0).clamp(0.0, 1.0)
    }

    fn should_finish_boot(elapsed: Duration) -> bool {
        elapsed >= Duration::from_secs(2)
    }

    fn desktop_icon_positions(work_rect: Rect, count: usize) -> Vec<Pos2> {
        let cell_w = 90.0;
        let cell_h = 90.0;
        let columns = ((work_rect.width() / cell_w).floor() as usize).max(1);
        (0..count)
            .map(|idx| {
                let col = idx / ((work_rect.height() / cell_h).floor() as usize).max(1);
                let row = idx % ((work_rect.height() / cell_h).floor() as usize).max(1);
                let x = work_rect.right() - 70.0 - col as f32 * cell_w;
                let y = work_rect.top() + 18.0 + row as f32 * cell_h;
                let bounded_x = x.max(work_rect.left() + 10.0);
                let max_x = work_rect.right() - 70.0;
                Pos2::new(bounded_x.min(max_x), y)
            })
            .take(
                columns
                    * ((work_rect.height() / cell_h).floor() as usize)
                        .max(1)
                        .max(count),
            )
            .collect()
    }

    fn should_auto_lock(
        idle_for: Duration,
        idle_lock_minutes: u64,
        screen_state: AppScreenState,
    ) -> bool {
        if screen_state != AppScreenState::Desktop {
            return false;
        }
        idle_for >= Duration::from_secs(idle_lock_minutes.saturating_mul(60))
    }

    fn should_start_screensaver(
        idle_for: Duration,
        idle_lock_minutes: u64,
        screen_state: AppScreenState,
    ) -> bool {
        if screen_state != AppScreenState::Desktop {
            return false;
        }
        let lock_secs = idle_lock_minutes.saturating_mul(60);
        if lock_secs == 0 {
            return false;
        }
        let threshold_secs = lock_secs.saturating_sub(30).max(15);
        idle_for >= Duration::from_secs(threshold_secs)
    }

    fn consume_auth_focus(pending: &mut bool, enabled: bool) -> bool {
        if enabled && *pending {
            *pending = false;
            true
        } else {
            false
        }
    }

    fn toggle_file_preview_pane(&mut self) {
        self.show_file_preview_pane = !self.show_file_preview_pane;
    }

    fn toggle_file_sidebar(&mut self) {
        self.show_file_sidebar = !self.show_file_sidebar;
    }

    fn toggle_file_path_bar(&mut self) {
        self.show_file_path_bar = !self.show_file_path_bar;
    }

    fn toggle_file_status_bar(&mut self) {
        self.show_file_status_bar = !self.show_file_status_bar;
    }

    fn file_manager_path_segments(path: &std::path::Path) -> Vec<(String, PathBuf)> {
        let mut current = PathBuf::new();
        let mut segments = Vec::new();
        for component in path.components() {
            current.push(component.as_os_str());
            let label = component
                .as_os_str()
                .to_string_lossy()
                .trim_end_matches('\\')
                .trim_end_matches('/')
                .to_string();
            if label.is_empty() {
                continue;
            }
            segments.push((label, current.clone()));
        }
        if segments.is_empty() {
            segments.push(("Home".to_string(), path.to_path_buf()));
        }
        segments
    }

    fn filter_file_manager_entries(entries: &[FmEntry], query: &str) -> Vec<FmEntry> {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return entries.to_vec();
        }
        entries
            .iter()
            .filter(|entry| {
                entry.name.to_ascii_lowercase().contains(&query)
                    || entry
                        .path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext.to_ascii_lowercase().contains(&query))
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    fn file_manager_view_mode_from_shortcut(index: u8) -> Option<FileManagerViewMode> {
        match index {
            1 => Some(FileManagerViewMode::Icon),
            2 => Some(FileManagerViewMode::List),
            3 => Some(FileManagerViewMode::Column),
            4 => Some(FileManagerViewMode::Gallery),
            _ => None,
        }
    }

    fn set_file_manager_view_mode(&mut self, mode: FileManagerViewMode) {
        self.fm_view_mode = mode;
        self.fm_view_modes_by_path
            .insert(self.fm_current_dir.clone(), mode);
    }

    fn sync_file_manager_view_mode_for_current_dir(&mut self) {
        self.fm_view_mode = self
            .fm_view_modes_by_path
            .get(&self.fm_current_dir)
            .copied()
            .unwrap_or(FileManagerViewMode::List);
    }

    fn sort_file_manager_entries(entries: &[FmEntry], field: FileManagerSortField) -> Vec<FmEntry> {
        let mut sorted = entries.to_vec();
        sorted.sort_by(|a, b| {
            if a.is_dir != b.is_dir {
                return b.is_dir.cmp(&a.is_dir);
            }
            match field {
                FileManagerSortField::Name => a
                    .name
                    .to_ascii_lowercase()
                    .cmp(&b.name.to_ascii_lowercase()),
                FileManagerSortField::Size => a.size.cmp(&b.size).then_with(|| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                }),
                FileManagerSortField::Kind => {
                    let a_kind = a
                        .path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("");
                    let b_kind = b
                        .path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("");
                    a_kind.cmp(b_kind).then_with(|| {
                        a.name
                            .to_ascii_lowercase()
                            .cmp(&b.name.to_ascii_lowercase())
                    })
                }
            }
        });
        sorted
    }

    fn gallery_selection_index(entries: &[FmEntry], selected_path: Option<&PathBuf>) -> usize {
        selected_path
            .and_then(|selected| entries.iter().position(|entry| &entry.path == selected))
            .unwrap_or(0)
    }

    fn gallery_next_index(len: usize, current: usize, delta: isize) -> usize {
        if len == 0 {
            return 0;
        }
        current
            .saturating_add_signed(delta)
            .min(len.saturating_sub(1))
    }

    fn navigate_file_manager_to(&mut self, path: PathBuf, push_history: bool) {
        if push_history && self.fm_current_dir != path {
            self.fm_back_history.push(self.fm_current_dir.clone());
            self.fm_forward_history.clear();
        }
        self.fm_current_dir = path.clone();
        self.fm_entries = read_directory(&path);
        self.fm_selected_path = None;
        self.sync_file_manager_view_mode_for_current_dir();
        self.sync_active_tab_from_file_manager();
    }

    fn request_file_info(&mut self, path: Option<PathBuf>) {
        self.file_info_target = path;
    }

    fn window_supports_tabs(kind: WindowKind) -> bool {
        matches!(
            kind,
            WindowKind::Terminal | WindowKind::Notes | WindowKind::TextEditor
        )
    }

    fn window_tab_count(&self, kind: WindowKind) -> usize {
        match kind {
            WindowKind::Terminal => self.terminal_tabs.len(),
            WindowKind::Notes => self.notes_tabs.len(),
            WindowKind::TextEditor => self.editor_tabs.len(),
            _ => 0,
        }
    }

    fn sync_tabbed_window_globals(&mut self, kind: WindowKind) {
        match kind {
            WindowKind::Terminal => {
                if let Some(tab) = self.terminal_tabs.get(self.terminal_active_tab) {
                    self.terminal_output = tab.output.clone();
                    self.terminal_input = tab.input.clone();
                }
            }
            WindowKind::Notes => {
                if let Some(tab) = self.notes_tabs.get(self.notes_active_tab) {
                    self.notes_text = tab.text.clone();
                }
            }
            WindowKind::TextEditor => {
                if let Some(tab) = self.editor_tabs.get(self.editor_active_tab) {
                    self.editor_file_path = tab.file_path.clone();
                    self.editor_content = tab.content.clone();
                    self.editor_modified = tab.modified;
                }
            }
            _ => {}
        }
    }

    fn sync_active_tab_from_globals(&mut self, kind: WindowKind) {
        match kind {
            WindowKind::Terminal => {
                if let Some(tab) = self.terminal_tabs.get_mut(self.terminal_active_tab) {
                    tab.output = self.terminal_output.clone();
                    tab.input = self.terminal_input.clone();
                }
            }
            WindowKind::Notes => {
                if let Some(tab) = self.notes_tabs.get_mut(self.notes_active_tab) {
                    tab.text = self.notes_text.clone();
                }
            }
            WindowKind::TextEditor => {
                if let Some(tab) = self.editor_tabs.get_mut(self.editor_active_tab) {
                    tab.file_path = self.editor_file_path.clone();
                    tab.content = self.editor_content.clone();
                    tab.modified = self.editor_modified;
                    tab.title = self
                        .editor_file_path
                        .as_ref()
                        .and_then(|path| path.file_name().and_then(|name| name.to_str()))
                        .unwrap_or("Untitled")
                        .to_string();
                }
            }
            _ => {}
        }
    }

    fn open_window_tab(&mut self, kind: WindowKind) {
        match kind {
            WindowKind::Terminal => {
                let next = self.terminal_tabs.len() + 1;
                self.terminal_tabs.push(TerminalTab {
                    title: format!("Shell {next}"),
                    output: Vec::new(),
                    input: String::new(),
                });
                self.terminal_active_tab = self.terminal_tabs.len() - 1;
            }
            WindowKind::Notes => {
                let next = self.notes_tabs.len() + 1;
                self.notes_tabs.push(NotesTab {
                    title: format!("Note {next}"),
                    text: String::new(),
                });
                self.notes_active_tab = self.notes_tabs.len() - 1;
            }
            WindowKind::TextEditor => {
                self.editor_tabs.push(EditorTab {
                    title: "Untitled".to_string(),
                    file_path: None,
                    content: String::new(),
                    modified: false,
                });
                self.editor_active_tab = self.editor_tabs.len() - 1;
            }
            _ => return,
        }
        self.sync_tabbed_window_globals(kind);
    }

    fn close_window_tab(&mut self, kind: WindowKind) {
        match kind {
            WindowKind::Terminal if self.terminal_tabs.len() > 1 => {
                self.terminal_tabs.remove(self.terminal_active_tab);
                self.terminal_active_tab =
                    self.terminal_active_tab.min(self.terminal_tabs.len() - 1);
            }
            WindowKind::Notes if self.notes_tabs.len() > 1 => {
                self.notes_tabs.remove(self.notes_active_tab);
                self.notes_active_tab = self.notes_active_tab.min(self.notes_tabs.len() - 1);
            }
            WindowKind::TextEditor if self.editor_tabs.len() > 1 => {
                self.editor_tabs.remove(self.editor_active_tab);
                self.editor_active_tab = self.editor_active_tab.min(self.editor_tabs.len() - 1);
            }
            _ => return,
        }
        self.sync_tabbed_window_globals(kind);
    }

    fn cycle_window_tab(&mut self, kind: WindowKind, delta: isize) {
        match kind {
            WindowKind::Terminal if !self.terminal_tabs.is_empty() => {
                self.terminal_active_tab = self
                    .terminal_active_tab
                    .saturating_add_signed(delta)
                    .min(self.terminal_tabs.len() - 1);
            }
            WindowKind::Notes if !self.notes_tabs.is_empty() => {
                self.notes_active_tab = self
                    .notes_active_tab
                    .saturating_add_signed(delta)
                    .min(self.notes_tabs.len() - 1);
            }
            WindowKind::TextEditor if !self.editor_tabs.is_empty() => {
                self.editor_active_tab = self
                    .editor_active_tab
                    .saturating_add_signed(delta)
                    .min(self.editor_tabs.len() - 1);
            }
            _ => return,
        }
        self.sync_tabbed_window_globals(kind);
    }

    fn render_window_tabs(&mut self, ui: &mut egui::Ui, kind: WindowKind) {
        let labels: Vec<String> = match kind {
            WindowKind::Terminal => self
                .terminal_tabs
                .iter()
                .map(|tab| tab.title.clone())
                .collect(),
            WindowKind::Notes => self
                .notes_tabs
                .iter()
                .map(|tab| tab.title.clone())
                .collect(),
            WindowKind::TextEditor => self
                .editor_tabs
                .iter()
                .map(|tab| tab.title.clone())
                .collect(),
            _ => Vec::new(),
        };
        let active_index = match kind {
            WindowKind::Terminal => self.terminal_active_tab,
            WindowKind::Notes => self.notes_active_tab,
            WindowKind::TextEditor => self.editor_active_tab,
            _ => 0,
        };

        ui.horizontal_wrapped(|ui| {
            for (index, label) in labels.iter().enumerate() {
                let active = index == active_index;
                let fill = if active {
                    Color32::from_rgba_unmultiplied(0, 122, 255, 110)
                } else {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 14)
                };
                if ui
                    .add(
                        egui::Button::new(RichText::new(label).size(11.0).color(Color32::WHITE))
                            .fill(fill),
                    )
                    .clicked()
                {
                    match kind {
                        WindowKind::Terminal => self.terminal_active_tab = index,
                        WindowKind::Notes => self.notes_active_tab = index,
                        WindowKind::TextEditor => self.editor_active_tab = index,
                        _ => {}
                    }
                    self.sync_tabbed_window_globals(kind);
                }
            }
            if ui.small_button("+").clicked() {
                self.open_window_tab(kind);
            }
        });
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(8.0);
    }

    fn pip_small_size() -> Vec2 {
        Vec2::new(280.0, 157.5)
    }

    fn pip_medium_size() -> Vec2 {
        Vec2::new(420.0, 236.25)
    }

    fn pip_resized_size(size: Vec2, delta: Vec2) -> Vec2 {
        let width = (size.x + delta.x).clamp(220.0, 560.0);
        Vec2::new(width, width / (16.0 / 9.0))
    }

    fn pip_toggle_size(size: Vec2) -> Vec2 {
        if size.x > 320.0 {
            Self::pip_small_size()
        } else {
            Self::pip_medium_size()
        }
    }

    fn pip_clamp_position(work_rect: Rect, size: Vec2, pos: Pos2) -> Pos2 {
        let min_x = work_rect.left() + 12.0;
        let min_y = work_rect.top() + 12.0;
        let max_x = work_rect.right() - size.x - 12.0;
        let max_y = work_rect.bottom() - size.y - 12.0;
        Pos2::new(pos.x.clamp(min_x, max_x), pos.y.clamp(min_y, max_y))
    }

    fn pip_snapped_position(work_rect: Rect, size: Vec2, pos: Pos2) -> Pos2 {
        let candidates = [
            Pos2::new(work_rect.left() + 16.0, work_rect.top() + 16.0),
            Pos2::new(work_rect.right() - size.x - 16.0, work_rect.top() + 16.0),
            Pos2::new(work_rect.left() + 16.0, work_rect.bottom() - size.y - 16.0),
            Pos2::new(
                work_rect.right() - size.x - 16.0,
                work_rect.bottom() - size.y - 16.0,
            ),
        ];
        candidates
            .into_iter()
            .min_by(|a, b| {
                let da = (a.x - pos.x).powi(2) + (a.y - pos.y).powi(2);
                let db = (b.x - pos.x).powi(2) + (b.y - pos.y).powi(2);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(pos)
    }

    fn music_track_info(track_idx: usize) -> (&'static str, &'static str, Color32) {
        let tracks = [
            (
                "Aurora Ambient",
                "System Sounds",
                Color32::from_rgb(255, 107, 157),
            ),
            ("Neon Waves", "Synthwave FM", Color32::from_rgb(88, 86, 214)),
            (
                "Mountain Breeze",
                "Nature Sounds",
                Color32::from_rgb(52, 199, 89),
            ),
            ("Deep Focus", "Lo-Fi Beats", Color32::from_rgb(255, 149, 0)),
            ("Night Drive", "Electronic", Color32::from_rgb(0, 122, 255)),
        ];
        tracks[track_idx % tracks.len()]
    }

    fn normalize_music_track_idx(track_idx: usize, real_track_count: usize) -> usize {
        if real_track_count > 0 {
            track_idx % real_track_count
        } else {
            track_idx % 5
        }
    }

    fn current_music_track_title(track_idx: usize, real_tracks: &[PathBuf]) -> String {
        if real_tracks.is_empty() {
            let (name, _, _) = Self::music_track_info(track_idx);
            name.to_string()
        } else {
            Self::music_track_title(
                &real_tracks[Self::normalize_music_track_idx(track_idx, real_tracks.len())],
            )
        }
    }

    fn music_track_duration_seconds(track_idx: usize, real_track: Option<&std::path::Path>) -> f32 {
        if let Some(path) = real_track {
            let bytes = std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
            let ext_bonus = match path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str()
            {
                "flac" | "wav" => 45.0,
                "m4a" | "aac" => 25.0,
                _ => 0.0,
            };
            ((bytes as f32 / 48_000.0) + 90.0 + ext_bonus).clamp(90.0, 420.0)
        } else {
            const MOCK_DURATIONS: [f32; 5] = [208.0, 194.0, 236.0, 221.0, 245.0];
            MOCK_DURATIONS[track_idx % MOCK_DURATIONS.len()]
        }
    }

    fn format_music_time(seconds: f32) -> String {
        let total = seconds.max(0.0).round() as u32;
        format!("{}:{:02}", total / 60, total % 60)
    }

    fn advance_music_elapsed(elapsed: f32, delta: f32, duration: f32) -> (f32, bool) {
        let next = elapsed + delta.max(0.0);
        if next >= duration {
            (duration, true)
        } else {
            (next, false)
        }
    }

    fn music_seek_fraction(rect: Rect, pointer_x: f32) -> f32 {
        ((pointer_x - rect.left()) / rect.width()).clamp(0.0, 1.0)
    }

    fn music_seek_seconds(duration: f32, fraction: f32) -> f32 {
        (duration * fraction.clamp(0.0, 1.0)).clamp(0.0, duration.max(0.0))
    }

    fn photo_color(idx: usize) -> Color32 {
        let colors = [
            Color32::from_rgb(255, 107, 107),
            Color32::from_rgb(78, 205, 196),
            Color32::from_rgb(255, 230, 109),
            Color32::from_rgb(162, 155, 254),
            Color32::from_rgb(255, 159, 243),
            Color32::from_rgb(69, 183, 209),
            Color32::from_rgb(255, 179, 71),
            Color32::from_rgb(119, 221, 119),
            Color32::from_rgb(207, 159, 255),
            Color32::from_rgb(255, 105, 180),
            Color32::from_rgb(100, 149, 237),
            Color32::from_rgb(255, 218, 185),
            Color32::from_rgb(144, 238, 144),
            Color32::from_rgb(255, 160, 122),
            Color32::from_rgb(173, 216, 230),
            Color32::from_rgb(221, 160, 221),
            Color32::from_rgb(245, 222, 179),
            Color32::from_rgb(176, 224, 230),
            Color32::from_rgb(255, 182, 193),
            Color32::from_rgb(152, 251, 152),
            Color32::from_rgb(135, 206, 250),
            Color32::from_rgb(255, 228, 196),
            Color32::from_rgb(230, 230, 250),
            Color32::from_rgb(250, 128, 114),
        ];
        colors[idx % colors.len()]
    }

    fn open_pip(&mut self, source: PipSource, work_rect: Rect) {
        let size = self.pip_last_size.unwrap_or_else(Self::pip_small_size);
        let default_pos = Self::pip_snapped_position(
            work_rect,
            size,
            Pos2::new(
                work_rect.right() - size.x - 24.0,
                work_rect.bottom() - size.y - 24.0,
            ),
        );
        self.pip_state = Some(PipState {
            source,
            pos: self
                .pip_last_pos
                .map(|pos| Self::pip_clamp_position(work_rect, size, pos))
                .unwrap_or(default_pos),
            size,
            last_interaction: Instant::now(),
        });
        self.pip_dragging = false;
        self.pip_resizing = false;
    }

    fn assistant_weather_summary() -> &'static str {
        let hour = Local::now().hour();
        match hour {
            6..=11 => "Clear morning, 18°C with light coastal breeze.",
            12..=17 => "Sunny afternoon, 22°C and dry conditions.",
            18..=21 => "Mild evening, 19°C with scattered clouds.",
            _ => "Cool night, 15°C and calm skies.",
        }
    }

    fn assistant_track_match(query: &str) -> Option<usize> {
        let lower = query.to_lowercase();
        (0..5).find(|idx| {
            let (name, artist, _) = Self::music_track_info(*idx);
            name.to_lowercase().contains(&lower) || artist.to_lowercase().contains(&lower)
        })
    }

    fn assistant_track_match_with_paths(query: &str, real_tracks: &[PathBuf]) -> Option<usize> {
        let lower = query.to_lowercase();
        real_tracks.iter().position(|path| {
            Self::music_track_title(path)
                .to_lowercase()
                .contains(&lower)
        })
    }

    fn filtered_music_track_indices(real_tracks: &[PathBuf], query: &str) -> Vec<usize> {
        let trimmed = query.trim().to_lowercase();
        if trimmed.is_empty() {
            return (0..real_tracks.len()).collect();
        }
        real_tracks
            .iter()
            .enumerate()
            .filter_map(|(idx, path)| {
                let title = Self::music_track_title(path).to_lowercase();
                let ext = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if title.contains(&trimmed) || ext.contains(&trimmed) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }

    fn music_queue_indices(real_tracks: &[PathBuf], query: &str, shuffle: bool) -> Vec<usize> {
        let mut queue = Self::filtered_music_track_indices(real_tracks, query);
        if shuffle {
            queue.sort_by_key(|idx| {
                let path = &real_tracks[*idx];
                let mut hash = 0_u64;
                for byte in path.to_string_lossy().bytes() {
                    hash = hash.wrapping_mul(131).wrapping_add(byte as u64);
                }
                hash
            });
        }
        queue
    }

    fn music_filtered_queue_position(queue: &[usize], current_idx: usize) -> Option<usize> {
        queue.iter().position(|idx| *idx == current_idx)
    }

    fn step_music_track_idx(current_idx: usize, queue: &[usize], delta: isize) -> usize {
        if queue.is_empty() {
            return current_idx;
        }
        let current_pos = Self::music_filtered_queue_position(queue, current_idx).unwrap_or(0);
        let len = queue.len() as isize;
        let next_pos = (current_pos as isize + delta).rem_euclid(len) as usize;
        queue[next_pos]
    }

    fn step_music_track_idx_with_repeat(
        current_idx: usize,
        queue: &[usize],
        delta: isize,
        repeat_all: bool,
    ) -> Option<usize> {
        if queue.is_empty() {
            return None;
        }
        let current_pos = Self::music_filtered_queue_position(queue, current_idx).unwrap_or(0);
        let next_pos = current_pos as isize + delta;
        if repeat_all {
            Some(Self::step_music_track_idx(current_idx, queue, delta))
        } else if next_pos < 0 || next_pos >= queue.len() as isize {
            None
        } else {
            Some(queue[next_pos as usize])
        }
    }

    fn execute_assistant_query(&mut self, query: &str, record_user: bool) {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return;
        }

        if record_user {
            self.assistant_history.push(AssistantMessage {
                is_user: true,
                text: trimmed.to_string(),
            });
        }

        let response = match parse_assistant_query(trimmed) {
            AssistantIntent::OpenApp(kind) => {
                self.open_builtin_app(kind.title());
                format!("Opening {}.", kind.title())
            }
            AssistantIntent::AskTime => format!("It is {}.", Local::now().format("%-I:%M %p")),
            AssistantIntent::SetReminder(reminder) => {
                let note_line = format!("\n- [ ] {reminder}");
                if !self.notes_text.contains(&note_line) {
                    self.notes_text.push_str(&note_line);
                    self.sync_active_tab_from_globals(WindowKind::Notes);
                }
                self.notification_center.notify(
                    "Assistant",
                    "Reminder added",
                    &reminder,
                    Color32::from_rgb(255, 149, 0),
                );
                format!("Reminder saved: {reminder}.")
            }
            AssistantIntent::Search(search) => {
                self.show_spotlight = true;
                self.spotlight_query = search.clone();
                format!("Searching for \"{search}\".")
            }
            AssistantIntent::ToggleDarkMode(enabled) => {
                self.app_settings.dark_mode = enabled;
                if enabled { "Dark Mode enabled.".to_string() } else { "Dark Mode disabled.".to_string() }
            }
            AssistantIntent::ToggleWifi(enabled) => {
                self.cc_wifi = enabled;
                self.app_settings.wifi_enabled = enabled;
                if enabled { "Wi-Fi enabled.".to_string() } else { "Wi-Fi disabled.".to_string() }
            }
            AssistantIntent::ToggleBluetooth(enabled) => {
                self.cc_bluetooth = enabled;
                self.app_settings.bluetooth_enabled = enabled;
                if enabled { "Bluetooth enabled.".to_string() } else { "Bluetooth disabled.".to_string() }
            }
            AssistantIntent::BatteryStatus => {
                if self.sysinfo.battery_available {
                    if self.sysinfo.battery_charging {
                        format!("Battery is at {:.0}% and charging.", self.sysinfo.battery_pct)
                    } else {
                        format!("Battery is at {:.0}%.", self.sysinfo.battery_pct)
                    }
                } else {
                    "Battery data is not available on this device.".to_string()
                }
            }
            AssistantIntent::Weather => Self::assistant_weather_summary().to_string(),
            AssistantIntent::PlaySong(song) => {
                let real_tracks = Self::music_library_paths(&dirs_home());
                self.music_override_path = None;
                if let Some(song_name) = song {
                    if let Some(idx) = Self::assistant_track_match_with_paths(&song_name, &real_tracks)
                    {
                        self.music_track_idx = idx;
                    } else if let Some(idx) = Self::assistant_track_match(&song_name) {
                        self.music_track_idx = idx;
                    }
                }
                self.music_playing = true;
                self.reset_music_progress();
                let win = self.window_mut(WindowKind::MusicPlayer);
                win.open = true;
                win.minimized = false;
                self.bring_to_front(WindowKind::MusicPlayer);
                let track = Self::current_music_track_title(self.music_track_idx, &real_tracks);
                format!("Playing {track}.")
            }
            AssistantIntent::Fallback => {
                "I can help open apps, search, toggle Wi-Fi/Bluetooth/Dark Mode, create reminders, report battery, weather, and play music.".to_string()
            }
        };

        self.assistant_history.push(AssistantMessage {
            is_user: false,
            text: response,
        });
        self.assistant_state = AssistantOverlayState::Idle;
        self.assistant_query.clear();
    }

    fn run_assistant_query(&mut self, query: &str) {
        self.execute_assistant_query(query, true);
    }

    fn queue_assistant_query(&mut self, query: &str) {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return;
        }
        self.assistant_history.push(AssistantMessage {
            is_user: true,
            text: trimmed.to_string(),
        });
        self.pending_assistant_query = Some((
            trimmed.to_string(),
            Instant::now() + Duration::from_millis(450),
        ));
        self.assistant_state = AssistantOverlayState::Thinking;
        self.assistant_query.clear();
    }

    fn maybe_process_assistant_query(&mut self) {
        if let Some((query, ready_at)) = self.pending_assistant_query.clone() {
            if Instant::now() >= ready_at {
                self.pending_assistant_query = None;
                self.execute_assistant_query(&query, false);
            }
        }
    }

    fn reset_music_progress(&mut self) {
        self.music_elapsed_seconds = 0.0;
        self.music_last_tick = Instant::now();
        self.persist_music_state();
        self.sync_music_audio(true);
    }

    fn maybe_tick_music_playback(&mut self) {
        let now = Instant::now();
        let delta = now.duration_since(self.music_last_tick).as_secs_f32();
        self.music_last_tick = now;
        if !self.music_playing {
            return;
        }

        let real_tracks = Self::music_library_paths(&dirs_home());
        let current_track_path = self.current_music_path(&real_tracks);
        let using_real_tracks = !real_tracks.is_empty() && self.music_override_path.is_none();
        let filtered_indices = if using_real_tracks {
            Self::music_queue_indices(&real_tracks, &self.music_library_query, self.music_shuffle)
        } else {
            Vec::new()
        };
        let duration = Self::music_track_duration_seconds(
            self.music_track_idx,
            current_track_path.as_deref(),
        );
        let (elapsed, finished) =
            Self::advance_music_elapsed(self.music_elapsed_seconds, delta, duration);
        if !finished {
            self.music_elapsed_seconds = elapsed;
            self.persist_music_state();
            return;
        }

        let mut should_restart_audio = false;
        let next_idx = if using_real_tracks {
            Self::step_music_track_idx_with_repeat(
                self.music_track_idx,
                &filtered_indices,
                1,
                self.music_repeat_all,
            )
        } else if self.music_override_path.is_some() {
            if self.music_repeat_all {
                should_restart_audio = true;
            }
            None
        } else if self.music_repeat_all {
            Some((self.music_track_idx + 1) % 5)
        } else if self.music_track_idx + 1 < 5 {
            Some(self.music_track_idx + 1)
        } else {
            None
        };

        if let Some(idx) = next_idx {
            self.music_track_idx = idx;
            self.music_override_path = None;
            self.music_elapsed_seconds = 0.0;
            should_restart_audio = true;
        } else if should_restart_audio {
            self.music_elapsed_seconds = 0.0;
        } else {
            self.music_elapsed_seconds = duration;
            self.music_playing = false;
        }
        self.persist_music_state();
        if should_restart_audio && self.music_playing {
            self.music_last_tick = Instant::now();
            self.sync_music_audio(true);
        }
    }

    fn sync_music_audio(&mut self, force_restart: bool) {
        let real_tracks = Self::music_library_paths(&dirs_home());
        let target_path = self.current_music_path(&real_tracks);

        let action = Self::desired_music_audio_action(
            self.music_audio
                .as_ref()
                .and_then(|engine| engine.active_path()),
            target_path.as_deref(),
            self.music_playing,
            force_restart,
        );

        let Some(engine) = self.music_audio.as_mut() else {
            return;
        };

        match action {
            MusicAudioAction::Stop => engine.stop(),
            MusicAudioAction::Pause => engine.pause(),
            MusicAudioAction::Resume => engine.resume(),
            MusicAudioAction::PlayFromOffset => {
                if let Some(path) = target_path.as_deref() {
                    let _ = engine.play_file(path, self.music_elapsed_seconds);
                } else {
                    engine.stop();
                }
            }
        }
    }

    fn render_assistant_overlay(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            Order::Foreground,
            Id::new("assistant_bg"),
        ));
        painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 120));

        let pulse = ((ctx.input(|i| i.time) as f32 * 2.2).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
        let orb_start = match self.assistant_state {
            AssistantOverlayState::Listening => Color32::from_rgb(90, 220, 255),
            AssistantOverlayState::Thinking => Color32::from_rgb(255, 184, 92),
            AssistantOverlayState::Idle => Color32::from_rgb(128, 190, 255),
        };
        let orb_end = match self.assistant_state {
            AssistantOverlayState::Listening => Color32::from_rgb(82, 129, 255),
            AssistantOverlayState::Thinking => Color32::from_rgb(255, 122, 0),
            AssistantOverlayState::Idle => Color32::from_rgb(112, 102, 255),
        };
        egui::Area::new(Id::new("assistant_overlay"))
            .order(Order::Foreground)
            .fixed_pos(Pos2::new(screen.center().x - 220.0, screen.top() + 88.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(18, 20, 28, 242))
                    .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 35)))
                    .corner_radius(CornerRadius::same(18))
                    .inner_margin(egui::Margin::same(16))
                    .show(ui, |ui| {
                        ui.set_min_width(440.0);
                        ui.horizontal(|ui| {
                            let orb_rect = Rect::from_center_size(
                                Pos2::new(ui.min_rect().left() + 18.0, ui.cursor().top() + 14.0),
                                Vec2::splat(28.0 + pulse * 8.0),
                            );
                            gradient_rect(ui.painter(), orb_rect, orb_start, orb_end);
                            ui.add_space(30.0);
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Aurora Assistant").size(16.0).strong().color(Color32::WHITE));
                                let subtitle = match self.assistant_state {
                                    AssistantOverlayState::Listening => "Listening for a request...",
                                    AssistantOverlayState::Thinking => "Thinking...",
                                    AssistantOverlayState::Idle => "Type a request or use a suggestion below.",
                                };
                                ui.label(RichText::new(subtitle).size(11.0).color(Color32::from_gray(170)));
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.small_button("Close").clicked() {
                                    self.show_assistant = false;
                                    self.assistant_state = AssistantOverlayState::Idle;
                                    self.pending_assistant_query = None;
                                }
                            });
                        });

                        ui.add_space(12.0);
                        egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                            if self.assistant_history.is_empty() {
                                ui.label(RichText::new("Ask me to open apps, search the system, create reminders, or play music.").size(12.0).color(Color32::from_gray(185)));
                            }
                            for message in &self.assistant_history {
                                let fill = if message.is_user {
                                    Color32::from_rgba_unmultiplied(0, 122, 255, 70)
                                } else {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 10)
                                };
                                egui::Frame::new()
                                    .fill(fill)
                                    .corner_radius(CornerRadius::same(12))
                                    .inner_margin(egui::Margin::same(10))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(&message.text)
                                                .size(12.0)
                                                .color(Color32::from_gray(235)),
                                        );
                                    });
                                ui.add_space(6.0);
                            }
                            if self.assistant_state == AssistantOverlayState::Thinking {
                                let dots = match ((ctx.input(|i| i.time) * 3.0) as i32).rem_euclid(3) {
                                    0 => ".",
                                    1 => "..",
                                    _ => "...",
                                };
                                egui::Frame::new()
                                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                                    .corner_radius(CornerRadius::same(12))
                                    .inner_margin(egui::Margin::same(10))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new(format!("Thinking{dots}")).size(12.0).color(Color32::from_gray(220)));
                                    });
                            }
                        });

                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            for chip in assistant_suggestion_chips() {
                                if ui.button(*chip).clicked() {
                                    self.assistant_query = chip.to_string();
                                    let query = self.assistant_query.clone();
                                    self.queue_assistant_query(&query);
                                }
                            }
                        });

                        ui.add_space(10.0);
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.assistant_query)
                                .hint_text("Open Files, remind me to..., play..., search for...")
                                .desired_width(408.0),
                        );
                        if !response.has_focus() && self.assistant_query.is_empty() {
                            response.request_focus();
                        }
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let query = self.assistant_query.clone();
                            self.queue_assistant_query(&query);
                        }
                    });
            });
    }

    fn insert_text_into_active_input(&mut self, text: &str) -> bool {
        match self.active_window() {
            Some(WindowKind::TextEditor) => {
                self.editor_content.push_str(text);
                self.editor_modified = true;
                self.sync_active_tab_from_globals(WindowKind::TextEditor);
                true
            }
            Some(WindowKind::Notes) => {
                self.notes_text.push_str(text);
                self.sync_active_tab_from_globals(WindowKind::Notes);
                true
            }
            Some(WindowKind::Messages) => {
                self.messages_state.input_text.push_str(text);
                true
            }
            _ => false,
        }
    }

    fn commit_emoji_pick(&mut self, symbol: &str) -> bool {
        if self.insert_text_into_active_input(symbol) {
            push_recent_emoji(&mut self.recent_emojis, symbol, 30);
            self.persist_recent_emojis();
            let _ = self.app_settings.save();
            self.show_emoji_picker = false;
            true
        } else {
            false
        }
    }

    fn setup_step_count() -> usize {
        4
    }

    fn validate_setup_step(
        step: usize,
        user_name: &str,
        password: &str,
        confirm_password: &str,
    ) -> Result<(), &'static str> {
        match step {
            0 | 3 => Ok(()),
            1 => {
                if user_name.trim().is_empty() {
                    Err("Enter a user name.")
                } else {
                    Ok(())
                }
            }
            2 => {
                if password.is_empty() {
                    Err("Enter a password.")
                } else if password != confirm_password {
                    Err("Passwords do not match.")
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    fn try_advance_setup(&mut self) -> Result<bool, &'static str> {
        Self::validate_setup_step(
            self.setup_step,
            &self.setup_user_name,
            &self.setup_password,
            &self.setup_password_confirm,
        )?;

        if self.setup_step + 1 < Self::setup_step_count() {
            self.setup_step += 1;
            return Ok(false);
        }

        Self::try_create_profile(
            &mut self.app_settings,
            &self.setup_user_name,
            &self.setup_password,
            &self.setup_password_confirm,
        )?;
        Ok(true)
    }

    fn try_create_profile(
        settings: &mut AppSettings,
        user_name: &str,
        password: &str,
        confirm_password: &str,
    ) -> Result<(), &'static str> {
        let trimmed_name = user_name.trim();
        if trimmed_name.is_empty() {
            return Err("Enter a user name.");
        }
        if password.is_empty() {
            return Err("Enter a password.");
        }
        if password != confirm_password {
            return Err("Passwords do not match.");
        }
        settings.user_name = trimmed_name.to_string();
        settings.set_password(password);
        Ok(())
    }

    fn try_unlock(settings: &AppSettings, password: &str) -> Result<(), &'static str> {
        if password.is_empty() {
            return Err("Enter your password.");
        }
        if settings.verify_password(password) {
            Ok(())
        } else {
            Err("Incorrect password.")
        }
    }

    fn try_change_password(
        settings: &mut AppSettings,
        current_password: &str,
        new_password: &str,
        confirm_password: &str,
    ) -> Result<(), &'static str> {
        if current_password.is_empty() {
            return Err("Enter your current password.");
        }
        if !settings.verify_password(current_password) {
            return Err("Current password is incorrect.");
        }
        if new_password.is_empty() {
            return Err("Enter a new password.");
        }
        if new_password != confirm_password {
            return Err("New passwords do not match.");
        }
        settings.set_password(new_password);
        Ok(())
    }

    fn profile_created_label(profile: &UserProfile) -> String {
        if profile.created_at == 0 {
            "Created unknown".to_string()
        } else {
            format!("Created {}", profile.created_at)
        }
    }

    fn lock_screen(&mut self, reason: &str) {
        if self.screen_state == AppScreenState::Desktop && self.app_settings.has_user_profile() {
            self.screensaver_active = false;
            self.screensaver_started_at = None;
            self.screen_state = AppScreenState::Locked;
            self.auth_focus_pending = true;
            self.auth_password.clear();
            self.auth_error = Some(reason.to_string());
            self.login_shake = None;
        }
    }

    fn start_screensaver(&mut self) {
        if self.screen_state == AppScreenState::Desktop && !self.screensaver_active {
            self.screensaver_active = true;
            self.screensaver_started_at = Some(Instant::now());
            self.screensaver_kind = self.screensaver_kind.next();
        }
    }

    fn submit_auth(&mut self) {
        match self.screen_state {
            AppScreenState::Setup => match self.try_advance_setup() {
                Ok(false) => {
                    self.auth_error = None;
                }
                Ok(true) => {
                    self.user_profile = UserProfile::from_display_name(
                        &self.setup_user_name,
                        (
                            self.app_settings.accent_r,
                            self.app_settings.accent_g,
                            self.app_settings.accent_b,
                        ),
                    );
                    self.profile_name_buffer = self.user_profile.display_name.clone();
                    self.screen_state = AppScreenState::Desktop;
                    self.setup_step = 0;
                    self.auth_focus_pending = false;
                    self.auth_error = None;
                    self.setup_password.clear();
                    self.setup_password_confirm.clear();
                    self.auth_password.clear();
                    self.last_input_at = Instant::now();
                    let _ = self.app_settings.save();
                    let _ = self.user_profile.save();
                    self.toast_manager.push(Toast::new(
                        "Welcome",
                        "Profile created",
                        Color32::from_rgb(52, 199, 89),
                    ));
                }
                Err(message) => {
                    self.auth_error = Some(message.to_string());
                    self.login_shake = Some(Instant::now());
                }
            },
            AppScreenState::Login | AppScreenState::Locked => {
                match Self::try_unlock(&self.app_settings, &self.auth_password) {
                    Ok(()) => {
                        self.screen_state = AppScreenState::Desktop;
                        self.auth_focus_pending = false;
                        self.auth_password.clear();
                        self.auth_error = None;
                        self.login_shake = None;
                        self.last_input_at = Instant::now();
                    }
                    Err(message) => {
                        self.auth_error = Some(message.to_string());
                        self.login_shake = Some(Instant::now());
                    }
                }
            }
            AppScreenState::Desktop => {}
        }
    }

    fn note_input_activity(&mut self, ctx: &egui::Context) {
        let has_activity = ctx.input(|i| {
            !i.events.is_empty()
                || i.pointer.delta() != Vec2::ZERO
                || i.raw_scroll_delta != Vec2::ZERO
                || i.pointer.any_down()
        });
        if has_activity && self.screensaver_active {
            self.screensaver_active = false;
            self.screensaver_started_at = None;
            self.lock_screen("Locked");
            return;
        }
        if has_activity && self.screen_state == AppScreenState::Desktop {
            self.last_input_at = Instant::now();
        }
    }

    fn handle_session_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Q) && i.modifiers.command && i.modifiers.ctrl) {
            self.lock_screen("Locked");
        }
    }

    fn handle_emoji_picker_shortcuts(&mut self, ctx: &egui::Context) {
        let toggle = ctx.input(|i| {
            (i.key_pressed(egui::Key::Period) && i.modifiers.ctrl)
                || (i.key_pressed(egui::Key::Space) && i.modifiers.ctrl && i.modifiers.command)
        });
        if toggle {
            self.show_emoji_picker = !self.show_emoji_picker;
        }
        if self.show_emoji_picker && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_emoji_picker = false;
        }
    }

    fn battery_time_remaining_label(
        battery_pct: f32,
        charging: bool,
        low_power_mode: bool,
    ) -> String {
        if charging {
            let minutes = ((100.0 - battery_pct).max(0.0) * 1.2).round() as i32;
            return format!("About {}h {}m until full", minutes / 60, minutes % 60);
        }
        let hours_per_percent = if low_power_mode { 0.06 } else { 0.045 };
        let minutes = (battery_pct.max(0.0) * hours_per_percent * 60.0).round() as i32;
        format!("About {}h {}m remaining", minutes / 60, minutes % 60)
    }

    fn battery_alert_level(battery_available: bool, battery_pct: f32, charging: bool) -> u8 {
        if !battery_available || charging {
            0
        } else if battery_pct <= 5.0 {
            3
        } else if battery_pct <= 10.0 {
            2
        } else if battery_pct <= 20.0 {
            1
        } else {
            0
        }
    }

    fn handle_battery_alerts(&mut self) {
        let level = Self::battery_alert_level(
            self.sysinfo.battery_available,
            self.sysinfo.battery_pct,
            self.sysinfo.battery_charging,
        );
        if level == 0 {
            self.last_battery_alert_level = 0;
            return;
        }
        if level <= self.last_battery_alert_level {
            return;
        }

        self.last_battery_alert_level = level;
        match level {
            1 => {
                self.notification_center.notify(
                    "Battery",
                    "Low Battery",
                    "Battery is below 20%. Consider enabling Low Power Mode.",
                    Color32::from_rgb(255, 149, 0),
                );
            }
            2 => {
                self.notification_center.notify(
                    "Battery",
                    "Critical Battery",
                    "Battery is below 10%. Save your work soon.",
                    Color32::from_rgb(255, 59, 48),
                );
            }
            3 => {
                self.notification_center.notify(
                    "Battery",
                    "Reserve Power",
                    "Battery is below 5%. Save your work now.",
                    Color32::from_rgb(255, 59, 48),
                );
            }
            _ => {}
        }
    }

    fn handle_idle_lock(&mut self) {
        let idle_for = self.last_input_at.elapsed();
        if Self::should_start_screensaver(
            idle_for,
            self.app_settings.idle_lock_minutes,
            self.screen_state,
        ) && !self.screensaver_active
        {
            self.start_screensaver();
        }
        if Self::should_auto_lock(
            idle_for,
            self.app_settings.idle_lock_minutes,
            self.screen_state,
        ) {
            self.lock_screen("Locked after inactivity");
        }
    }

    fn refresh_desktop_entries_if_needed(&mut self) {
        if self.desktop_last_refresh.elapsed() >= Duration::from_secs(5) {
            self.desktop_entries = read_directory(&desktop_directory());
            self.desktop_last_refresh = Instant::now();
        }
    }

    fn open_quick_look(&mut self, paths: Vec<PathBuf>, selected_path: &PathBuf) {
        if let Some(index) = paths.iter().position(|path| path == selected_path) {
            self.quick_look_paths = paths;
            self.quick_look_index = index;
            self.quick_look_open = true;
        }
    }

    fn handle_quick_look_shortcuts(&mut self, ctx: &egui::Context) {
        if self.quick_look_open {
            if ctx.input(|i| i.key_pressed(egui::Key::Space) || i.key_pressed(egui::Key::Escape)) {
                self.quick_look_open = false;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                self.quick_look_index =
                    move_preview_index(self.quick_look_index, 1, self.quick_look_paths.len());
            }
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                self.quick_look_index =
                    move_preview_index(self.quick_look_index, -1, self.quick_look_paths.len());
            }
            return;
        }

        if !ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            return;
        }

        if self.focused == Some(WindowKind::FileManager) {
            if let Some(selected) = self.fm_selected_path.clone() {
                let paths = self
                    .fm_entries
                    .iter()
                    .map(|entry| entry.path.clone())
                    .collect::<Vec<_>>();
                self.open_quick_look(paths, &selected);
            }
        } else if let Some(selected) = self.desktop_selected_paths.first().cloned() {
            let paths = self
                .desktop_entries
                .iter()
                .map(|entry| entry.path.clone())
                .collect::<Vec<_>>();
            self.open_quick_look(paths, &selected);
        }
    }

    fn render_quick_look(&mut self, ctx: &egui::Context) {
        if !self.quick_look_open || self.quick_look_paths.is_empty() {
            return;
        }

        let path = self.quick_look_paths[self.quick_look_index].clone();
        let preview = build_preview(&path);
        let screen = ctx.viewport_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            Order::Foreground,
            Id::new("quick_look_bg"),
        ));
        painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 170));

        egui::Area::new(Id::new("quick_look_modal"))
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(25, 27, 34, 240))
                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(40)))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(egui::Margin::same(18))
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(620.0, 420.0));
                        ui.label(
                            RichText::new(preview.title)
                                .size(20.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.label(
                            RichText::new(preview.subtitle)
                                .size(11.0)
                                .color(Color32::from_gray(160)),
                        );
                        ui.add_space(14.0);
                        match preview.kind {
                            PreviewKind::Text => {
                                egui::ScrollArea::vertical()
                                    .max_height(320.0)
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(preview.body)
                                                .size(12.0)
                                                .monospace()
                                                .color(Color32::from_gray(220)),
                                        );
                                    });
                            }
                            _ => {
                                ui.label(
                                    RichText::new(preview.body)
                                        .size(12.0)
                                        .color(Color32::from_gray(210)),
                                );
                            }
                        }
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("Space/Esc to close, arrows to navigate")
                                .size(10.0)
                                .color(Color32::from_gray(130)),
                        );
                    });
            });
    }

    fn render_file_info_panel(&mut self, ctx: &egui::Context) {
        let Some(path) = self.file_info_target.clone() else {
            return;
        };
        let info = read_file_info(&path);

        egui::Window::new("Get Info")
            .id(Id::new("file_info_panel"))
            .collapsible(false)
            .resizable(false)
            .default_width(360.0)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(&info.name)
                        .size(18.0)
                        .strong()
                        .color(Color32::WHITE),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("Kind: {}", info.kind))
                        .size(12.0)
                        .color(Color32::from_gray(210)),
                );
                ui.label(
                    RichText::new(format!("Size: {}", info.size_label))
                        .size(12.0)
                        .color(Color32::from_gray(210)),
                );
                ui.label(
                    RichText::new(format!("Location: {}", info.location))
                        .size(12.0)
                        .color(Color32::from_gray(210)),
                );
                ui.label(
                    RichText::new(info.modified_label)
                        .size(12.0)
                        .color(Color32::from_gray(210)),
                );
                ui.add_space(10.0);
                if ui.button("Close").clicked() {
                    self.file_info_target = None;
                }
            });
    }

    fn render_emoji_picker(&mut self, ctx: &egui::Context) {
        if !self.show_emoji_picker {
            return;
        }

        let mut picked_symbol: Option<String> = None;
        let screen = ctx.content_rect();
        let recents = self
            .recent_emojis
            .iter()
            .filter_map(|symbol| find_emoji_by_symbol(symbol))
            .collect::<Vec<_>>();
        let filtered = filtered_emoji_entries(&self.emoji_query, self.emoji_category);

        egui::Area::new(Id::new("emoji_picker"))
            .fixed_pos(Pos2::new(
                screen.center().x - 220.0,
                screen.bottom() - 360.0,
            ))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(28, 30, 36, 244))
                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(36)))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(440.0, 320.0));
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Emoji & Symbols")
                                    .size(15.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new("Ctrl+.")
                                        .size(10.0)
                                        .color(Color32::from_gray(150)),
                                );
                            });
                        });
                        ui.add_space(8.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.emoji_query)
                                .desired_width(ui.available_width())
                                .hint_text("Search emoji or symbols"),
                        );
                        if !recents.is_empty() {
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new("Recently Used")
                                    .size(10.0)
                                    .color(Color32::from_gray(150)),
                            );
                            ui.horizontal_wrapped(|ui| {
                                for entry in recents {
                                    if ui.button(entry.symbol).on_hover_text(entry.name).clicked() {
                                        picked_symbol = Some(entry.symbol.to_string());
                                    }
                                }
                            });
                        }
                        ui.add_space(10.0);
                        ui.horizontal_wrapped(|ui| {
                            for category in EmojiCategory::all() {
                                let active = *category == self.emoji_category;
                                let fill = if active {
                                    Color32::from_rgba_unmultiplied(0, 122, 255, 110)
                                } else {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 18)
                                };
                                let label = format!("{} {}", category.icon(), category.label());
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(label).size(10.0).color(Color32::WHITE),
                                        )
                                        .fill(fill),
                                    )
                                    .clicked()
                                {
                                    self.emoji_category = *category;
                                }
                            }
                        });
                        ui.add_space(10.0);
                        egui::ScrollArea::vertical()
                            .max_height(180.0)
                            .show(ui, |ui| {
                                egui::Grid::new("emoji_picker_grid")
                                    .num_columns(8)
                                    .spacing(Vec2::new(8.0, 8.0))
                                    .show(ui, |ui| {
                                        for (index, entry) in filtered.iter().enumerate() {
                                            if ui
                                                .button(entry.symbol)
                                                .on_hover_text(entry.name)
                                                .clicked()
                                            {
                                                picked_symbol = Some(entry.symbol.to_string());
                                            }
                                            if (index + 1) % 8 == 0 {
                                                ui.end_row();
                                            }
                                        }
                                    });
                                if filtered.is_empty() {
                                    ui.label(
                                        RichText::new("No matches")
                                            .size(11.0)
                                            .color(Color32::from_gray(140)),
                                    );
                                }
                            });
                    });
            });

        if let Some(symbol) = picked_symbol {
            let _ = self.commit_emoji_pick(&symbol);
        }
    }

    fn render_boot_splash(&mut self, ctx: &egui::Context) {
        let screen = ctx.viewport_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            Order::Background,
            Id::new("boot_splash"),
        ));
        painter.rect_filled(screen, 0.0, Color32::from_rgb(8, 10, 18));

        let elapsed = self.boot_started_at.elapsed();
        let progress = Self::boot_progress(elapsed);
        let fade_in = (elapsed.as_secs_f32() / 0.3).clamp(0.0, 1.0);
        let alpha = (fade_in * 255.0) as u8;

        let center = screen.center();
        let logo_rect =
            Rect::from_center_size(center + Vec2::new(0.0, -40.0), Vec2::new(140.0, 140.0));
        let glow = Color32::from_rgba_unmultiplied(70, 160, 255, alpha.saturating_sub(80));
        painter.circle_filled(logo_rect.center(), 72.0, glow);
        painter.circle_stroke(
            logo_rect.center(),
            52.0,
            Stroke::new(4.0, Color32::from_rgba_unmultiplied(180, 225, 255, alpha)),
        );
        painter.circle_stroke(
            logo_rect.center() + Vec2::new(10.0, -8.0),
            34.0,
            Stroke::new(6.0, Color32::from_rgba_unmultiplied(96, 210, 255, alpha)),
        );
        painter.circle_stroke(
            logo_rect.center() + Vec2::new(-12.0, 12.0),
            24.0,
            Stroke::new(5.0, Color32::from_rgba_unmultiplied(110, 255, 190, alpha)),
        );

        painter.text(
            center + Vec2::new(0.0, 58.0),
            Align2::CENTER_CENTER,
            "AuroraOS",
            FontId::proportional(28.0),
            Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
        );

        let bar_rect =
            Rect::from_center_size(center + Vec2::new(0.0, 102.0), Vec2::new(220.0, 10.0));
        painter.rect_filled(
            bar_rect,
            CornerRadius::same(8),
            Color32::from_rgba_unmultiplied(255, 255, 255, 18),
        );
        let fill_rect = Rect::from_min_max(
            bar_rect.min,
            Pos2::new(
                bar_rect.left() + bar_rect.width() * progress,
                bar_rect.bottom(),
            ),
        );
        painter.rect_filled(
            fill_rect,
            CornerRadius::same(8),
            Color32::from_rgba_unmultiplied(96, 210, 255, 220),
        );

        painter.text(
            center + Vec2::new(0.0, 126.0),
            Align2::CENTER_CENTER,
            if self.telemetry.connected {
                "Connecting services"
            } else {
                "Starting desktop shell"
            },
            FontId::proportional(13.0),
            Color32::from_rgba_unmultiplied(180, 186, 204, alpha),
        );
    }

    fn window_ref(&self, kind: WindowKind) -> &ManagedWindow {
        &self.windows[kind as usize]
    }
    fn window_mut(&mut self, kind: WindowKind) -> &mut ManagedWindow {
        &mut self.windows[kind as usize]
    }

    fn bring_to_front(&mut self, kind: WindowKind) {
        self.z_order.retain(|k| *k != kind);
        self.z_order.push(kind);
        self.focused = Some(kind);
    }

    fn desktop_work_rect(ctx: &egui::Context) -> Rect {
        // Use screen_rect (not content_rect) because content_rect may shrink
        // after TopBottomPanel is drawn, which would double-subtract menu height.
        let full = ctx.viewport_rect();
        Rect::from_min_max(
            Pos2::new(full.left(), full.top() + MENU_BAR_HEIGHT),
            Pos2::new(full.right(), full.bottom() - DOCK_HEIGHT),
        )
    }

    fn snap_rect(work_rect: Rect, side: SnapSide) -> Rect {
        let hw = work_rect.width() * 0.5;
        let tw = work_rect.width() / 3.0;
        let h = work_rect.height();
        match side {
            SnapSide::Left => Rect::from_min_size(work_rect.left_top(), Vec2::new(hw, h)),
            SnapSide::Right => Rect::from_min_size(
                Pos2::new(work_rect.left() + hw, work_rect.top()),
                Vec2::new(hw, h),
            ),
            SnapSide::LeftThird => Rect::from_min_size(work_rect.left_top(), Vec2::new(tw, h)),
            SnapSide::CenterThird => Rect::from_min_size(
                Pos2::new(work_rect.left() + tw, work_rect.top()),
                Vec2::new(tw, h),
            ),
            SnapSide::RightThird => Rect::from_min_size(
                Pos2::new(work_rect.left() + 2.0 * tw, work_rect.top()),
                Vec2::new(tw, h),
            ),
        }
    }

    fn active_window(&self) -> Option<WindowKind> {
        if let Some(kind) = self.focused {
            let w = self.window_ref(kind);
            if w.open && !w.minimized {
                return Some(kind);
            }
        }
        self.z_order.iter().rev().copied().find(|k| {
            let w = self.window_ref(*k);
            w.open && !w.minimized
        })
    }

    // ── IPC ──────────────────────────────────────────────────────────────────

    fn next_frame_id(&mut self) -> u64 {
        let id = self.next_frame_id;
        self.next_frame_id = self.next_frame_id.saturating_add(1);
        id
    }

    fn send_command(&mut self, command: &str) -> Result<String, String> {
        let stream = TcpStream::connect(&self.daemon_addr)
            .map_err(|e| format!("connect {} failed: {e}", self.daemon_addr))?;
        let reader_stream = stream
            .try_clone()
            .map_err(|e| format!("clone failed: {e}"))?;
        let mut reader = BufReader::new(reader_stream);
        let mut writer = stream;
        let frame = CommandFrame::with_auth(self.next_frame_id(), self.auth_token.clone(), command);
        let mut encoded = encode_command(&frame);
        encoded.push('\n');
        writer
            .write_all(encoded.as_bytes())
            .map_err(|e| format!("write: {e}"))?;
        writer.flush().map_err(|e| format!("flush: {e}"))?;
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| format!("read: {e}"))?;
        if line.trim().is_empty() {
            return Err("empty response".to_string());
        }
        let resp = decode_response(line.trim()).map_err(|e| format!("decode: {e}"))?;
        Ok(resp.payload)
    }

    fn refresh_telemetry(&mut self) {
        let status = self.send_command("status");
        let health = self.send_command("health");
        let uptime = self.send_command("uptime");
        let boot = self.send_command("boot");
        match (status, health, uptime, boot) {
            (Ok(s), Ok(h), Ok(u), Ok(b)) => {
                self.telemetry.connected = true;
                self.telemetry.status = s;
                self.telemetry.health = h;
                self.telemetry.uptime = u;
                self.telemetry.boot = b;
                self.telemetry.last_error = None;
                self.telemetry.last_poll = Some(Instant::now());
            }
            (s, h, u, b) => {
                self.telemetry.connected = false;
                let mut errs = Vec::new();
                if let Err(e) = s {
                    errs.push(format!("status: {e}"));
                }
                if let Err(e) = h {
                    errs.push(format!("health: {e}"));
                }
                if let Err(e) = u {
                    errs.push(format!("uptime: {e}"));
                }
                if let Err(e) = b {
                    errs.push(format!("boot: {e}"));
                }
                self.telemetry.last_error = Some(errs.join(" | "));
                self.telemetry.last_poll = Some(Instant::now());
            }
        }
    }

    fn maybe_poll(&mut self) {
        let should = self
            .telemetry
            .last_poll
            .map(|l| l.elapsed() >= POLL_EVERY)
            .unwrap_or(true);
        if should {
            self.refresh_telemetry();
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context, work_rect: Rect) {
        // Cmd/Ctrl+Q = quit
        if ctx.input(|i| i.key_pressed(egui::Key::Q) && i.modifiers.command) {
            self.save_state();
            self.should_quit = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // F11 = toggle fullscreen
        if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
            let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
            return;
        }

        // Cmd/Ctrl+W = close focused window
        if ctx.input(|i| i.key_pressed(egui::Key::W) && i.modifiers.command) {
            if self.active_window() == Some(WindowKind::FileManager) && self.fm_tabs.len() > 1 {
                self.close_file_manager_tab(self.fm_active_tab);
                return;
            }
            if let Some(kind) = self.active_window() {
                if Self::window_supports_tabs(kind) && self.window_tab_count(kind) > 1 {
                    self.close_window_tab(kind);
                    return;
                }
            }
            if let Some(kind) = self.active_window() {
                let win = self.window_mut(kind);
                win.open = false;
                win.id_epoch = win.id_epoch.saturating_add(1);
            }
            return;
        }

        // Cmd/Ctrl+S = save in text editor
        if ctx.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command) {
            if self.active_window() == Some(WindowKind::TextEditor) && self.editor_modified {
                if let Some(ref path) = self.editor_file_path {
                    if fs::write(path, self.editor_content.as_bytes()).is_ok() {
                        self.editor_modified = false;
                        self.toast_manager.push(Toast::new(
                            "Saved",
                            path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
                            Color32::from_rgb(52, 199, 89),
                        ));
                    }
                }
            }
            return;
        }

        // Cmd/Ctrl+N = new file in text editor
        if ctx.input(|i| i.key_pressed(egui::Key::N) && i.modifiers.command) {
            self.editor_content.clear();
            self.editor_file_path = None;
            self.editor_modified = false;
            let win = self.window_mut(WindowKind::TextEditor);
            win.open = true;
            win.minimized = false;
            win.open_anim_start = Some(Instant::now());
            win.id_epoch = win.id_epoch.saturating_add(1);
            self.bring_to_front(WindowKind::TextEditor);
            return;
        }

        // Cmd/Ctrl+T = new file manager tab
        if ctx.input(|i| i.key_pressed(egui::Key::T) && i.modifiers.command) {
            if let Some(kind) = self.active_window() {
                if Self::window_supports_tabs(kind) {
                    self.open_window_tab(kind);
                    return;
                }
            }
            if self.active_window() == Some(WindowKind::FileManager) {
                self.open_file_manager_tab(dirs_home());
                return;
            }
        }

        if let Some(kind) = self.active_window() {
            if Self::window_supports_tabs(kind) && self.window_tab_count(kind) > 1 {
                let next = ctx.input(|i| {
                    i.key_pressed(egui::Key::Tab) && i.modifiers.ctrl && !i.modifiers.shift
                });
                let prev = ctx.input(|i| {
                    i.key_pressed(egui::Key::Tab) && i.modifiers.ctrl && i.modifiers.shift
                });
                if next {
                    let count = self.window_tab_count(kind);
                    match kind {
                        WindowKind::Terminal => {
                            self.terminal_active_tab = (self.terminal_active_tab + 1) % count
                        }
                        WindowKind::Notes => {
                            self.notes_active_tab = (self.notes_active_tab + 1) % count
                        }
                        WindowKind::TextEditor => {
                            self.editor_active_tab = (self.editor_active_tab + 1) % count
                        }
                        _ => {}
                    }
                    self.sync_tabbed_window_globals(kind);
                    return;
                }
                if prev {
                    let count = self.window_tab_count(kind);
                    match kind {
                        WindowKind::Terminal => {
                            self.terminal_active_tab = if self.terminal_active_tab == 0 {
                                count - 1
                            } else {
                                self.terminal_active_tab - 1
                            }
                        }
                        WindowKind::Notes => {
                            self.notes_active_tab = if self.notes_active_tab == 0 {
                                count - 1
                            } else {
                                self.notes_active_tab - 1
                            }
                        }
                        WindowKind::TextEditor => {
                            self.editor_active_tab = if self.editor_active_tab == 0 {
                                count - 1
                            } else {
                                self.editor_active_tab - 1
                            }
                        }
                        _ => {}
                    }
                    self.sync_tabbed_window_globals(kind);
                    return;
                }
            }
        }

        if self.active_window() == Some(WindowKind::FileManager)
            && ctx.input(|i| i.modifiers.command)
        {
            let shortcut = if ctx.input(|i| i.key_pressed(egui::Key::Num1)) {
                Some(1)
            } else if ctx.input(|i| i.key_pressed(egui::Key::Num2)) {
                Some(2)
            } else if ctx.input(|i| i.key_pressed(egui::Key::Num3)) {
                Some(3)
            } else if ctx.input(|i| i.key_pressed(egui::Key::Num4)) {
                Some(4)
            } else {
                None
            };
            if let Some(index) = shortcut.and_then(Self::file_manager_view_mode_from_shortcut) {
                self.set_file_manager_view_mode(index);
                return;
            }
        }

        // Cmd/Ctrl+M = minimize focused window
        if ctx.input(|i| i.key_pressed(egui::Key::M) && i.modifiers.command) {
            if let Some(kind) = self.active_window() {
                let win = self.window_mut(kind);
                win.minimized = true;
                win.id_epoch = win.id_epoch.saturating_add(1);
            }
            return;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::A) && i.modifiers.command)
            && self.active_window().is_none()
            && self.screen_state == AppScreenState::Desktop
        {
            self.desktop_selected_paths = Self::select_all_desktop_entries(&self.desktop_entries);
            return;
        }

        // Ctrl+ArrowLeft / Ctrl+ArrowRight = snap window (halves)
        // Ctrl+Alt+ArrowLeft / Right / Down = snap to thirds
        let snap_left = ctx
            .input(|i| i.key_pressed(egui::Key::ArrowLeft) && i.modifiers.ctrl && !i.modifiers.alt);
        let snap_right = ctx.input(|i| {
            i.key_pressed(egui::Key::ArrowRight) && i.modifiers.ctrl && !i.modifiers.alt
        });
        let snap_left_third = ctx
            .input(|i| i.key_pressed(egui::Key::ArrowLeft) && i.modifiers.ctrl && i.modifiers.alt);
        let snap_center_third = ctx
            .input(|i| i.key_pressed(egui::Key::ArrowDown) && i.modifiers.ctrl && i.modifiers.alt);
        let snap_right_third = ctx
            .input(|i| i.key_pressed(egui::Key::ArrowRight) && i.modifiers.ctrl && i.modifiers.alt);

        let side = if snap_left {
            Some(SnapSide::Left)
        } else if snap_right {
            Some(SnapSide::Right)
        } else if snap_left_third {
            Some(SnapSide::LeftThird)
        } else if snap_center_third {
            Some(SnapSide::CenterThird)
        } else if snap_right_third {
            Some(SnapSide::RightThird)
        } else {
            None
        };

        let Some(side) = side else { return };
        let Some(active) = self.active_window() else {
            return;
        };
        let win = self.window_mut(active);
        win.restore();
        win.maximized = false;
        win.snap = Some(side);
        let snapped = Self::snap_rect(work_rect, side);
        win.default_pos = snapped.min;
        win.default_size = snapped.size();
        win.id_epoch = win.id_epoch.saturating_add(1);
    }

    // ── Background ───────────────────────────────────────────────────────────

    fn paint_wallpaper(painter: &egui::Painter, rect: Rect, wp: &WallpaperPreset, alpha: f32) {
        let a = (alpha * 255.0) as u8;
        for w in wp.bands.windows(2) {
            let (t0, c0) = w[0];
            let (t1, c1) = w[1];
            let y0 = rect.top() + t0 * rect.height();
            let y1 = rect.top() + t1 * rect.height();
            let strip = Rect::from_min_max(Pos2::new(rect.left(), y0), Pos2::new(rect.right(), y1));
            gradient_rect(
                painter,
                strip,
                Color32::from_rgba_unmultiplied(c0[0], c0[1], c0[2], a),
                Color32::from_rgba_unmultiplied(c1[0], c1[1], c1[2], a),
            );
        }
        for &(cx, peak, spread, rgb) in wp.hills {
            paint_hill(
                painter,
                rect,
                cx,
                peak,
                spread,
                Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], a),
            );
        }
    }

    fn render_background(&mut self, ctx: &egui::Context) {
        let rect = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());

        if self.wallpaper_changing && self.wallpaper_transition < 1.0 {
            // Cross-fade: paint old wallpaper fading out, new one fading in
            let t = self.wallpaper_transition;
            let old_wp = &WALLPAPERS[self.wallpaper_prev_idx % WALLPAPERS.len()];
            Self::paint_wallpaper(&painter, rect, old_wp, 1.0 - t);
            let new_wp = &WALLPAPERS[self.wallpaper_idx % WALLPAPERS.len()];
            Self::paint_wallpaper(&painter, rect, new_wp, t);
            self.wallpaper_transition = (self.wallpaper_transition + 0.02).min(1.0);
            if self.wallpaper_transition >= 1.0 {
                self.wallpaper_changing = false;
            }
        } else {
            let wp = &WALLPAPERS[self.wallpaper_idx % WALLPAPERS.len()];
            Self::paint_wallpaper(&painter, rect, wp, 1.0);
        }
    }

    // ── Menu bar ─────────────────────────────────────────────────────────────

    fn render_menu_bar(&mut self, ctx: &egui::Context) -> (bool, bool, bool) {
        let mut toggle_cc = false;
        let mut toggle_spotlight = false;
        let mut toggle_notifications = false;

        let clock_str = Local::now().format("%a %b %-d  %-I:%M %p").to_string();
        let batt_pct = self.sysinfo.battery_pct;
        let batt_charging = self.sysinfo.battery_charging;
        let batt_available = self.sysinfo.battery_available;
        let net_up = self.sysinfo.network_up;

        let menu_fill = if self.app_settings.dark_mode {
            Color32::from_rgba_unmultiplied(15, 15, 18, 200)
        } else {
            Color32::from_rgba_unmultiplied(0, 0, 0, 65)
        };
        egui::TopBottomPanel::top("menu_bar")
            .exact_height(MENU_BAR_HEIGHT)
            .frame(
                egui::Frame::default()
                    .fill(menu_fill)
                    .inner_margin(egui::Margin::symmetric(14, 6)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").size(14.0).color(Color32::WHITE));
                    ui.add_space(4.0);
                    let app_name = self
                        .active_window()
                        .map(|k| k.title())
                        .unwrap_or("AuroraOS");
                    ui.label(
                        RichText::new(app_name)
                            .strong()
                            .size(13.0)
                            .color(Color32::WHITE),
                    );
                    ui.add_space(8.0);

                    let menus = [
                        MenuDropdown::File,
                        MenuDropdown::Edit,
                        MenuDropdown::View,
                        MenuDropdown::Window,
                        MenuDropdown::Help,
                    ];
                    for menu in menus {
                        let is_active = self.active_menu == Some(menu);
                        let bg = if is_active {
                            Color32::from_rgba_unmultiplied(255, 255, 255, 45)
                        } else {
                            Color32::TRANSPARENT
                        };
                        let response = egui::Frame::default()
                            .fill(bg)
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(egui::Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(menu.label())
                                        .size(13.0)
                                        .color(Color32::from_gray(220)),
                                );
                            })
                            .response;
                        if response.clicked() {
                            self.active_menu = if is_active { None } else { Some(menu) };
                        }
                        if self.active_menu.is_some()
                            && response.hovered()
                            && self.active_menu != Some(menu)
                        {
                            self.active_menu = Some(menu);
                        }
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let display_name = self.profile_display_name().to_string();
                        let initials = self.avatar_initials().to_string();
                        let avatar_color = self.avatar_color();
                        let (avatar_rect, avatar_resp) =
                            ui.allocate_exact_size(Vec2::new(58.0, 20.0), Sense::hover());
                        let circle_center =
                            Pos2::new(avatar_rect.left() + 10.0, avatar_rect.center().y);
                        ui.painter().circle_filled(circle_center, 8.0, avatar_color);
                        ui.painter().text(
                            circle_center,
                            Align2::CENTER_CENTER,
                            initials,
                            FontId::proportional(8.0),
                            Color32::WHITE,
                        );
                        ui.painter().text(
                            Pos2::new(circle_center.x + 14.0, avatar_rect.center().y),
                            Align2::LEFT_CENTER,
                            display_name,
                            FontId::proportional(11.0),
                            Color32::from_gray(235),
                        );
                        avatar_resp.on_hover_text("Signed-in user");
                        ui.add_space(6.0);

                        // Spotlight
                        let spot = ui.add(
                            egui::Label::new(RichText::new("O").size(14.0).color(Color32::WHITE))
                                .sense(Sense::click()),
                        );
                        if spot.clicked() {
                            toggle_spotlight = true;
                        }
                        spot.on_hover_text("Spotlight");
                        ui.add_space(6.0);

                        let assistant = ui.add(
                            egui::Label::new(RichText::new("◎").size(14.0).color(Color32::WHITE))
                                .sense(Sense::click()),
                        );
                        if assistant.clicked() {
                            self.show_assistant = !self.show_assistant;
                            if self.show_assistant {
                                self.show_spotlight = false;
                                self.assistant_query.clear();
                                self.assistant_state = AssistantOverlayState::Listening;
                            } else {
                                self.assistant_state = AssistantOverlayState::Idle;
                                self.pending_assistant_query = None;
                            }
                        }
                        assistant.on_hover_text("Aurora Assistant");
                        ui.add_space(6.0);

                        // Clock (click for notifications)
                        let unread = self.notification_center.unread_count();
                        let clock_label = if unread > 0 {
                            format!("{clock_str}  ({unread})")
                        } else {
                            clock_str.clone()
                        };
                        let clock_resp = ui.add(
                            egui::Label::new(
                                RichText::new(&clock_label)
                                    .size(13.0)
                                    .color(Color32::from_gray(235)),
                            )
                            .sense(Sense::click()),
                        );
                        if clock_resp.clicked() {
                            toggle_notifications = true;
                        }
                        clock_resp.on_hover_text("Notification Center");
                        ui.add_space(6.0);

                        // Control Center
                        let cc = ui.add(
                            egui::Label::new(RichText::new("=").size(16.0).color(Color32::WHITE))
                                .sense(Sense::click()),
                        );
                        if cc.clicked() {
                            toggle_cc = true;
                        }
                        cc.on_hover_text("Control Center");
                        ui.add_space(6.0);

                        // Battery (real data)
                        if batt_available {
                            let batt_w = 20.0;
                            let batt_h = 10.0;
                            let (batt_rect, batt_resp) = ui.allocate_exact_size(
                                Vec2::new(batt_w + 3.0, batt_h),
                                Sense::click(),
                            );
                            let body =
                                Rect::from_min_size(batt_rect.min, Vec2::new(batt_w, batt_h));
                            ui.painter().rect_stroke(
                                body,
                                CornerRadius::same(2),
                                Stroke::new(1.0, Color32::from_gray(200)),
                                StrokeKind::Outside,
                            );
                            let fill_ratio = (batt_pct / 100.0).clamp(0.0, 1.0);
                            let fill_color = if batt_charging {
                                Color32::from_rgb(52, 199, 89)
                            } else if batt_pct < 20.0 {
                                Color32::from_rgb(255, 59, 48)
                            } else {
                                Color32::from_rgb(52, 199, 89)
                            };
                            let fill = Rect::from_min_size(
                                Pos2::new(body.left() + 1.5, body.top() + 1.5),
                                Vec2::new((batt_w - 3.0) * fill_ratio, batt_h - 3.0),
                            );
                            ui.painter()
                                .rect_filled(fill, CornerRadius::same(1), fill_color);
                            let nub = Rect::from_min_size(
                                Pos2::new(body.right(), body.center().y - 2.5),
                                Vec2::new(2.0, 5.0),
                            );
                            ui.painter().rect_filled(
                                nub,
                                CornerRadius::same(1),
                                Color32::from_gray(200),
                            );

                            // Charging bolt or percentage on hover
                            let batt_label = if batt_charging {
                                format!("{:.0}% (charging)", batt_pct)
                            } else {
                                format!("{:.0}%", batt_pct)
                            };
                            let (hover_rect, hover_resp) =
                                ui.allocate_exact_size(Vec2::ZERO, Sense::hover());
                            let _ = hover_rect;
                            hover_resp.on_hover_text(&batt_label);
                            if batt_resp.clicked() {
                                self.show_battery_popup = !self.show_battery_popup;
                                self.show_wifi_popup = false;
                                self.show_volume_popup = false;
                                self.show_bluetooth_popup = false;
                            }

                            ui.add_space(4.0);
                        }

                        // Volume (clickable)
                        let vol_resp = ui.add(
                            egui::Label::new(
                                RichText::new(if self.cc_volume > 0.0 { "♪" } else { "✕" })
                                    .size(14.0)
                                    .color(Color32::WHITE),
                            )
                            .sense(Sense::click()),
                        );
                        if vol_resp.clicked() {
                            self.show_volume_popup = !self.show_volume_popup;
                            self.show_wifi_popup = false;
                            self.show_battery_popup = false;
                            self.show_bluetooth_popup = false;
                        }
                        vol_resp.on_hover_text("Volume");
                        ui.add_space(4.0);

                        // Bluetooth (clickable)
                        let bt_color = if self.cc_bluetooth {
                            Color32::WHITE
                        } else {
                            Color32::from_gray(100)
                        };
                        let bt_resp = ui.add(
                            egui::Label::new(RichText::new("B").size(12.0).color(bt_color))
                                .sense(Sense::click()),
                        );
                        if bt_resp.clicked() {
                            self.show_bluetooth_popup = !self.show_bluetooth_popup;
                            self.show_wifi_popup = false;
                            self.show_battery_popup = false;
                            self.show_volume_popup = false;
                        }
                        bt_resp.on_hover_text("Bluetooth");
                        ui.add_space(4.0);

                        // Network status (real, clickable)
                        let net_color = if net_up {
                            Color32::from_gray(230)
                        } else {
                            Color32::from_gray(100)
                        };
                        let net_resp = ui.add(
                            egui::Label::new(RichText::new("W").size(12.0).color(net_color))
                                .sense(Sense::click()),
                        );
                        if net_resp.clicked() {
                            self.show_wifi_popup = !self.show_wifi_popup;
                            self.show_battery_popup = false;
                            self.show_volume_popup = false;
                            self.show_bluetooth_popup = false;
                        }
                        net_resp.on_hover_text("Wi-Fi");
                        ui.add_space(4.0);

                        // Daemon connection
                        let (status_text, status_color) = if self.telemetry.connected {
                            ("Online", Color32::from_rgb(124, 236, 112))
                        } else {
                            ("Offline", Color32::from_rgb(255, 152, 152))
                        };
                        ui.label(RichText::new(status_text).size(12.0).color(status_color));
                        let (dot_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                        ui.painter()
                            .circle_filled(dot_rect.center(), 4.0, status_color);
                    });
                });
            });

        (toggle_cc, toggle_spotlight, toggle_notifications)
    }

    // ── Menu dropdowns ───────────────────────────────────────────────────────

    fn render_menu_dropdown(&mut self, ctx: &egui::Context) {
        let Some(menu) = self.active_menu else { return };
        let menu_idx = match menu {
            MenuDropdown::File => 0,
            MenuDropdown::Edit => 1,
            MenuDropdown::View => 2,
            MenuDropdown::Window => 3,
            MenuDropdown::Help => 4,
        };
        let x = 80.0 + menu_idx as f32 * 55.0;

        let response = egui::Area::new(Id::new("menu_dropdown"))
            .fixed_pos(Pos2::new(x, MENU_BAR_HEIGHT))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(40, 40, 40, 230))
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(4, 4))
                    .show(ui, |ui| {
                        ui.set_min_width(200.0);
                        for item in menu.items() {
                            if *item == "---" {
                                ui.add_space(2.0);
                                let (sep_rect, _) =
                                    ui.allocate_exact_size(Vec2::new(192.0, 1.0), Sense::hover());
                                ui.painter().rect_filled(
                                    sep_rect,
                                    0.0,
                                    Color32::from_white_alpha(30),
                                );
                                ui.add_space(2.0);
                            } else {
                                let resp = ui.add(
                                    egui::Button::new(
                                        RichText::new(*item)
                                            .size(13.0)
                                            .color(Color32::from_gray(220)),
                                    )
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE)
                                    .min_size(Vec2::new(192.0, 26.0))
                                    .corner_radius(CornerRadius::same(4)),
                                );
                                if resp.clicked() {
                                    self.active_menu = None;
                                    let label = item.split("  ").next().unwrap_or(item);
                                    match label {
                                        "Quit" => self.menu_action = Some(MenuAction::Quit),
                                        "Close Window" => {
                                            self.menu_action = Some(MenuAction::CloseWindow)
                                        }
                                        "Minimize" => self.menu_action = Some(MenuAction::Minimize),
                                        "Zoom" => self.menu_action = Some(MenuAction::Maximize),
                                        "Start Screen Saver" => {
                                            self.menu_action = Some(MenuAction::StartScreenSaver)
                                        }
                                        "Tile Left" => {
                                            self.menu_action = Some(MenuAction::TileLeft)
                                        }
                                        "Tile Right" => {
                                            self.menu_action = Some(MenuAction::TileRight)
                                        }
                                        "Tile Left Third" => {
                                            self.menu_action = Some(MenuAction::TileLeftThird)
                                        }
                                        "Tile Center Third" => {
                                            self.menu_action = Some(MenuAction::TileCenterThird)
                                        }
                                        "Tile Right Third" => {
                                            self.menu_action = Some(MenuAction::TileRightThird)
                                        }
                                        "Bring All to Front" => {
                                            self.menu_action = Some(MenuAction::BringAllToFront)
                                        }
                                        "Copy" => self.menu_action = Some(MenuAction::Copy),
                                        "Cut" => self.menu_action = Some(MenuAction::Cut),
                                        "Paste" => self.menu_action = Some(MenuAction::Paste),
                                        "Select All" => {
                                            self.menu_action = Some(MenuAction::SelectAll)
                                        }
                                        "Undo" => self.menu_action = Some(MenuAction::Undo),
                                        "Redo" => self.menu_action = Some(MenuAction::Redo),
                                        "Save" => self.menu_action = Some(MenuAction::Save),
                                        "Enter Full Screen" => {
                                            self.menu_action = Some(MenuAction::ToggleFullScreen)
                                        }
                                        "Show Sidebar" => {
                                            self.menu_action = Some(MenuAction::ToggleSidebar)
                                        }
                                        "Show Path Bar" => {
                                            self.menu_action = Some(MenuAction::TogglePathBar)
                                        }
                                        "Show Status Bar" => {
                                            self.menu_action = Some(MenuAction::ToggleStatusBar)
                                        }
                                        "Show Preview" => {
                                            self.menu_action = Some(MenuAction::TogglePreview)
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    });
            });

        if ctx.input(|i| i.pointer.any_click()) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if !response.response.rect.contains(pos) && pos.y > MENU_BAR_HEIGHT {
                    self.active_menu = None;
                }
            }
        }
    }

    // ── Desktop icons ────────────────────────────────────────────────────────

    fn render_desktop_icons(&mut self, ctx: &egui::Context, work_rect: Rect) {
        self.refresh_desktop_entries_if_needed();
        let entries = self.desktop_entries.clone();
        let use_stacks = self.app_settings.desktop_use_stacks;
        let stack_groups = if use_stacks {
            Some(Self::desktop_stacks(&entries))
        } else {
            None
        };

        if use_stacks && self.desktop_stack_expanded.is_none() {
            let groups = stack_groups.unwrap_or_default();
            let rects = Self::desktop_icon_rects(work_rect, groups.len());
            egui::Area::new(Id::new("desktop_stacks"))
                .fixed_pos(Pos2::new(work_rect.left(), work_rect.top()))
                .show(ctx, |_ui| {
                    for (idx, (label, group_entries)) in groups.iter().enumerate() {
                        let icon_rect = rects.get(idx).copied().unwrap_or(Rect::from_min_size(
                            work_rect.left_top(),
                            Vec2::new(72.0, 76.0),
                        ));
                        let response = egui::Area::new(Id::new(("desktop_stack", idx)))
                            .fixed_pos(icon_rect.min)
                            .show(ctx, |ui| {
                                let frame_resp = egui::Frame::default()
                                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 18))
                                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(35)))
                                    .corner_radius(CornerRadius::same(12))
                                    .inner_margin(egui::Margin::same(8))
                                    .show(ui, |ui| {
                                        ui.set_min_size(icon_rect.size());
                                        ui.vertical_centered(|ui| {
                                            let (icon_r, _) = ui.allocate_exact_size(
                                                Vec2::splat(26.0),
                                                Sense::hover(),
                                            );
                                            let color = Color32::from_rgb(0, 122, 255);
                                            let body = Rect::from_center_size(
                                                icon_r.center(),
                                                Vec2::new(24.0, 18.0),
                                            );
                                            ui.painter().rect_filled(
                                                body,
                                                CornerRadius::same(3),
                                                color,
                                            );
                                            let tab = Rect::from_min_size(
                                                Pos2::new(body.left(), body.top() - 4.0),
                                                Vec2::new(10.0, 5.0),
                                            );
                                            ui.painter().rect_filled(
                                                tab,
                                                CornerRadius::same(2),
                                                color,
                                            );
                                            ui.painter().text(
                                                icon_r.right_top() + Vec2::new(-2.0, 2.0),
                                                Align2::RIGHT_TOP,
                                                group_entries.len().to_string(),
                                                FontId::proportional(10.0),
                                                Color32::WHITE,
                                            );
                                            ui.add_space(6.0);
                                            ui.label(
                                                RichText::new(label)
                                                    .size(11.0)
                                                    .color(Color32::from_gray(240)),
                                            );
                                        });
                                    })
                                    .response;
                                ui.interact(
                                    frame_resp.rect,
                                    Id::new(("desktop_stack_interact", idx)),
                                    Sense::click(),
                                )
                            })
                            .inner;
                        if response.clicked() {
                            self.desktop_stack_expanded = Some(label.clone());
                            self.desktop_selected_paths.clear();
                        }
                    }
                });
            return;
        }

        let entries = if let Some(expanded) = self.desktop_stack_expanded.as_ref() {
            if use_stacks {
                Self::desktop_stacks(&entries)
                    .into_iter()
                    .find(|(label, _)| label == expanded)
                    .map(|(_, grouped)| grouped)
                    .unwrap_or(entries)
            } else {
                entries
            }
        } else {
            entries
        };
        let icon_rects = Self::desktop_icon_rects(work_rect, entries.len());
        let mut open_path: Option<PathBuf> = None;

        if use_stacks && self.desktop_stack_expanded.is_some() {
            let expanded = self.desktop_stack_expanded.clone().unwrap_or_default();
            egui::Area::new(Id::new("desktop_stack_header"))
                .fixed_pos(Pos2::new(work_rect.right() - 180.0, work_rect.top() + 12.0))
                .order(Order::Foreground)
                .show(ctx, |ui| {
                    let resp = ui.add(
                        egui::Button::new(
                            RichText::new(format!("< {expanded}"))
                                .size(11.0)
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgba_unmultiplied(34, 34, 38, 220))
                        .corner_radius(CornerRadius::same(8)),
                    );
                    if resp.clicked() {
                        self.desktop_stack_expanded = None;
                        self.desktop_selected_paths.clear();
                    }
                });
        }

        if self.file_drag_path.is_none() {
            let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
            let primary_pressed = ctx.input(|i| i.pointer.primary_pressed());
            let pointer_down = ctx.input(|i| i.pointer.primary_down());

            if primary_pressed {
                if let Some(pos) = pointer_pos {
                    let hit_icon = icon_rects.iter().any(|rect| rect.contains(pos));
                    if work_rect.contains(pos) && !hit_icon {
                        self.desktop_selection_drag_start = Some(pos);
                        self.desktop_selected_paths.clear();
                    }
                }
            }

            if let Some(start) = self.desktop_selection_drag_start {
                if pointer_down {
                    if let Some(current) = pointer_pos {
                        let selection_rect = Self::desktop_selection_rect(start, current);
                        self.desktop_selected_paths = Self::desktop_paths_in_selection_rect(
                            &entries,
                            &icon_rects,
                            selection_rect,
                        );
                        let painter = ctx.layer_painter(egui::LayerId::new(
                            Order::Foreground,
                            Id::new("desktop_selection_rect"),
                        ));
                        painter.rect_filled(
                            selection_rect,
                            CornerRadius::same(4),
                            Color32::from_rgba_unmultiplied(0, 122, 255, 24),
                        );
                        painter.rect_stroke(
                            selection_rect,
                            CornerRadius::same(4),
                            Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 122, 255, 180)),
                            StrokeKind::Outside,
                        );
                    }
                } else {
                    self.desktop_selection_drag_start = None;
                }
            }
        }

        if let Some(drag_path) = self.file_drag_path.clone() {
            let pointer = ctx.input(|i| i.pointer.hover_pos());
            let copy_drag = ctx.input(|i| Self::drag_uses_copy_modifier(i.modifiers));
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.file_drag_path = None;
                return;
            }
            if let Some(pos) = pointer {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    Order::Tooltip,
                    Id::new("file_drag_preview"),
                ));
                if work_rect.contains(pos) {
                    painter.rect_stroke(
                        work_rect.shrink(6.0),
                        CornerRadius::same(10),
                        Stroke::new(2.0, Color32::from_rgba_unmultiplied(0, 122, 255, 140)),
                        StrokeKind::Outside,
                    );
                }
                let label = drag_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("file");
                let count = self.active_drag_count(&drag_path);
                painter.text(
                    pos + Vec2::new(16.0, -12.0),
                    Align2::LEFT_BOTTOM,
                    format!(
                        "{} {label} to Desktop",
                        if copy_drag { "Copy" } else { "Move" }
                    ),
                    FontId::proportional(12.0),
                    Color32::WHITE,
                );
                if count > 1 {
                    painter.text(
                        pos + Vec2::new(16.0, 6.0),
                        Align2::LEFT_TOP,
                        format!("{count} items"),
                        FontId::proportional(11.0),
                        Color32::from_gray(220),
                    );
                }
            }

            if !ctx.input(|i| i.pointer.any_down()) {
                if let Some(target_dir) = Self::desktop_drop_target(&drag_path, pointer, work_rect)
                {
                    let result = if copy_drag {
                        copy_entry_to_directory(&drag_path, &target_dir)
                    } else {
                        move_entry_to_directory(&drag_path, &target_dir)
                    };
                    match result {
                        Ok(_) => {
                            self.notification_center.notify(
                                "Files",
                                if copy_drag { "Copied" } else { "Moved" },
                                if copy_drag {
                                    "Item copied to Desktop"
                                } else {
                                    "Item moved to Desktop"
                                },
                                Color32::from_rgb(52, 199, 89),
                            );
                            self.fm_entries = read_directory(&self.fm_current_dir);
                            self.desktop_entries = read_directory(&desktop_directory());
                            self.fm_selected_path = None;
                            self.desktop_selected_paths.clear();
                            self.sync_active_tab_from_file_manager();
                        }
                        Err(err) => {
                            self.notification_center.notify(
                                "Files",
                                "Move failed",
                                &err,
                                Color32::from_rgb(255, 59, 48),
                            );
                        }
                    }
                }
                self.file_drag_path = None;
            }
        }

        egui::Area::new(Id::new("desktop_icons"))
            .fixed_pos(Pos2::new(work_rect.left(), work_rect.top()))
            .show(ctx, |_ui| {
                for (idx, entry) in entries.iter().enumerate() {
                    let icon_rect = icon_rects.get(idx).copied().unwrap_or(Rect::from_min_size(
                        work_rect.left_top(),
                        Vec2::new(72.0, 76.0),
                    ));
                    let selected = self
                        .desktop_selected_paths
                        .iter()
                        .any(|selected| selected == &entry.path);
                    let fill = if selected {
                        Color32::from_rgba_unmultiplied(0, 122, 255, 70)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 18)
                    };
                    let stroke = if selected {
                        Stroke::new(1.2, Color32::from_rgb(120, 200, 255))
                    } else {
                        Stroke::new(1.0, Color32::from_white_alpha(35))
                    };
                    let response = egui::Area::new(Id::new(("desktop_entry", idx)))
                        .fixed_pos(icon_rect.min)
                        .show(ctx, |ui| {
                            let frame_resp = egui::Frame::default()
                                .fill(fill)
                                .stroke(stroke)
                                .corner_radius(CornerRadius::same(12))
                                .inner_margin(egui::Margin::same(8))
                                .show(ui, |ui| {
                                    ui.set_min_size(icon_rect.size());
                                    ui.vertical_centered(|ui| {
                                        let (icon_r, _) = ui
                                            .allocate_exact_size(Vec2::splat(26.0), Sense::hover());
                                        let color = if entry.is_dir {
                                            Color32::from_rgb(0, 122, 255)
                                        } else {
                                            Color32::from_gray(210)
                                        };
                                        if entry.is_dir {
                                            let body = Rect::from_center_size(
                                                icon_r.center(),
                                                Vec2::new(24.0, 18.0),
                                            );
                                            ui.painter().rect_filled(
                                                body,
                                                CornerRadius::same(3),
                                                color,
                                            );
                                            let tab = Rect::from_min_size(
                                                Pos2::new(body.left(), body.top() - 4.0),
                                                Vec2::new(10.0, 5.0),
                                            );
                                            ui.painter().rect_filled(
                                                tab,
                                                CornerRadius::same(2),
                                                color,
                                            );
                                        } else {
                                            ui.painter().text(
                                                icon_r.center(),
                                                Align2::CENTER_CENTER,
                                                "F",
                                                FontId::proportional(14.0),
                                                color,
                                            );
                                        }
                                        ui.add_space(6.0);
                                        let truncated = if entry.name.chars().count() > 14 {
                                            format!(
                                                "{}…",
                                                entry.name.chars().take(13).collect::<String>()
                                            )
                                        } else {
                                            entry.name.clone()
                                        };
                                        ui.label(
                                            RichText::new(truncated)
                                                .size(11.0)
                                                .color(Color32::from_gray(240)),
                                        );
                                    });
                                })
                                .response;
                            ui.interact(
                                frame_resp.rect,
                                Id::new(("desktop_entry_interact", idx)),
                                Sense::click_and_drag(),
                            )
                        })
                        .inner;
                    if response.clicked() {
                        let toggle = ctx.input(|i| i.modifiers.command || i.modifiers.ctrl);
                        if toggle {
                            Self::toggle_desktop_selection(
                                &mut self.desktop_selected_paths,
                                &entry.path,
                            );
                        } else {
                            Self::replace_desktop_selection(
                                &mut self.desktop_selected_paths,
                                &entry.path,
                            );
                        }
                    }
                    if response.dragged() {
                        if !self
                            .desktop_selected_paths
                            .iter()
                            .any(|selected| selected == &entry.path)
                        {
                            Self::replace_desktop_selection(
                                &mut self.desktop_selected_paths,
                                &entry.path,
                            );
                        }
                        self.file_drag_path = Some(entry.path.clone());
                    }
                    if response.double_clicked() {
                        open_path = Some(entry.path.clone());
                    }
                    if response.secondary_clicked() {
                        if !self
                            .desktop_selected_paths
                            .iter()
                            .any(|selected| selected == &entry.path)
                        {
                            Self::replace_desktop_selection(
                                &mut self.desktop_selected_paths,
                                &entry.path,
                            );
                        }
                        self.desktop_context_target = Some(entry.path.clone());
                        self.context_menu_pos = ctx.input(|i| i.pointer.interact_pos());
                    }
                }
            });

        if let Some(path) = open_path {
            if path.is_dir() {
                self.fm_current_dir = path.clone();
                self.fm_entries = read_directory(&path);
                self.fm_selected_path = None;
                let win = self.window_mut(WindowKind::FileManager);
                win.restore();
                win.id_epoch = win.id_epoch.saturating_add(1);
                self.bring_to_front(WindowKind::FileManager);
            } else if !self.open_path_in_aurora_if_supported(&path) {
                open_file_with_system(&path);
            }
        }
    }

    // ── Right-click context menu ─────────────────────────────────────────────

    fn check_context_menu(&mut self, ctx: &egui::Context) {
        if self.context_menu_pos.is_none() && ctx.input(|i| i.pointer.secondary_clicked()) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if pos.y > MENU_BAR_HEIGHT && pos.y < ctx.content_rect().bottom() - DOCK_HEIGHT {
                    self.desktop_context_target = None;
                    self.context_menu_pos = Some(pos);
                }
            }
        }
        if ctx.input(|i| i.pointer.primary_clicked()) {
            self.context_menu_pos = None;
            self.desktop_context_target = None;
        }
    }

    fn render_context_menu(&mut self, ctx: &egui::Context) {
        let Some(pos) = self.context_menu_pos else {
            return;
        };
        let item_target = self.desktop_context_target.clone();
        egui::Area::new(Id::new("desktop_context_menu"))
            .fixed_pos(pos)
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(40, 40, 40, 230))
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(4, 4))
                    .show(ui, |ui| {
                        ui.set_min_width(210.0);
                        let items = Self::desktop_context_menu_items(item_target.is_some());
                        for &label in items {
                            if label == "---" {
                                ui.add_space(2.0);
                                let (sr, _) =
                                    ui.allocate_exact_size(Vec2::new(202.0, 1.0), Sense::hover());
                                ui.painter()
                                    .rect_filled(sr, 0.0, Color32::from_white_alpha(30));
                                ui.add_space(2.0);
                            } else {
                                let r = ui.add(
                                    egui::Button::new(
                                        RichText::new(label)
                                            .size(13.0)
                                            .color(Color32::from_gray(220)),
                                    )
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE)
                                    .min_size(Vec2::new(202.0, 26.0))
                                    .corner_radius(CornerRadius::same(4)),
                                );
                                if r.clicked() {
                                    self.context_menu_pos = None;
                                    self.desktop_context_target = None;
                                    match label {
                                        "Open" => {
                                            if let Some(path) = item_target.clone() {
                                                if path.is_dir() {
                                                    self.fm_current_dir = path.clone();
                                                    self.fm_entries = read_directory(&path);
                                                    self.fm_selected_path = None;
                                                    let win =
                                                        self.window_mut(WindowKind::FileManager);
                                                    win.restore();
                                                    win.id_epoch = win.id_epoch.saturating_add(1);
                                                    self.bring_to_front(WindowKind::FileManager);
                                                } else if !self.open_path_in_aurora_if_supported(&path) {
                                                    open_file_with_system(&path);
                                                }
                                            }
                                        }
                                        "Rename" => {
                                            if let Some(path) = item_target.clone() {
                                                self.desktop_rename_buffer = path
                                                    .file_name()
                                                    .and_then(|n| n.to_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                self.desktop_rename_target = Some(path);
                                            }
                                        }
                                        "Copy" => {
                                            if let Some(path) = item_target.clone() {
                                                self.clipboard.copy(&path.to_string_lossy());
                                                self.toast_manager.push(Toast::new(
                                                    "Clipboard",
                                                    "Copied item path",
                                                    Color32::from_rgb(0, 122, 255),
                                                ));
                                            }
                                        }
                                        "Move to Trash" => {
                                            if let Some(path) = item_target.clone() {
                                                if delete_entry(&path).is_ok() {
                                                    self.desktop_entries =
                                                        read_directory(&desktop_directory());
                                                    if self.fm_current_dir == desktop_directory() {
                                                        self.fm_entries =
                                                            read_directory(&self.fm_current_dir);
                                                    }
                                                    self.desktop_selected_paths.clear();
                                                    self.toast_manager.push(Toast::new(
                                                        "Trash",
                                                        "Item moved to Trash",
                                                        Color32::from_rgb(255, 149, 0),
                                                    ));
                                                }
                                            }
                                        }
                                        "New Folder" => {
                                            let desktop = dirs_home().join("Desktop");
                                            let target = if desktop.exists() {
                                                desktop
                                            } else {
                                                dirs_home()
                                            };
                                            let mut name = "New Folder".to_string();
                                            let mut n = 1u32;
                                            while target.join(&name).exists() {
                                                n += 1;
                                                name = format!("New Folder ({n})");
                                            }
                                            if fs::create_dir_all(target.join(&name)).is_ok() {
                                                self.toast_manager.push(Toast::new(
                                                    "Folder Created",
                                                    &name,
                                                    Color32::from_rgb(0, 122, 255),
                                                ));
                                                // Refresh file manager if viewing the same dir
                                                if self.fm_current_dir == target {
                                                    self.fm_entries = read_directory(&target);
                                                }
                                            }
                                        }
                                        "Change Wallpaper" => {
                                            self.wallpaper_idx =
                                                (self.wallpaper_idx + 1) % WALLPAPERS.len();
                                            self.toast_manager.push(Toast::new(
                                                "Wallpaper",
                                                format!(
                                                    "Switched to {}",
                                                    WALLPAPERS
                                                        [self.wallpaper_idx % WALLPAPERS.len()]
                                                    .name
                                                ),
                                                Color32::from_rgb(88, 86, 214),
                                            ));
                                        }
                                        "Use Stacks" => {
                                            self.app_settings.desktop_use_stacks =
                                                !self.app_settings.desktop_use_stacks;
                                            if !self.app_settings.desktop_use_stacks {
                                                self.desktop_stack_expanded = None;
                                            }
                                        }
                                        "Mission Control" => {
                                            self.show_mission_control = !self.show_mission_control;
                                        }
                                        "New File" => {
                                            let desktop = dirs_home().join("Desktop");
                                            let target = if desktop.exists() {
                                                desktop
                                            } else {
                                                dirs_home()
                                            };
                                            let mut name = "Untitled.txt".to_string();
                                            let mut n = 1u32;
                                            while target.join(&name).exists() {
                                                n += 1;
                                                name = format!("Untitled ({n}).txt");
                                            }
                                            if fs::write(target.join(&name), "").is_ok() {
                                                self.toast_manager.push(Toast::new(
                                                    "File Created",
                                                    &name,
                                                    Color32::from_rgb(52, 199, 89),
                                                ));
                                                if self.fm_current_dir == target {
                                                    self.fm_entries = read_directory(&target);
                                                }
                                            }
                                        }
                                        "Get Info" => {
                                            if let Some(path) = item_target.clone() {
                                                self.request_file_info(Some(path));
                                            } else {
                                                self.toast_manager.push(Toast::new(
                                                    "AuroraOS",
                                                    format!(
                                                        "v0.1.0 | {} windows | {:.0} FPS",
                                                        WINDOW_COUNT, self.fps_smoothed
                                                    ),
                                                    Color32::from_rgb(142, 142, 147),
                                                ));
                                            }
                                        }
                                        "Show Desktop" => {
                                            // Minimize all windows
                                            for i in 0..WINDOW_COUNT {
                                                if self.windows[i].open
                                                    && !self.windows[i].minimized
                                                {
                                                    self.windows[i].minimized = true;
                                                    self.windows[i].id_epoch =
                                                        self.windows[i].id_epoch.saturating_add(1);
                                                }
                                            }
                                        }
                                        "Open Terminal Here" => {
                                            let win = self.window_mut(WindowKind::Terminal);
                                            win.restore();
                                            win.id_epoch = win.id_epoch.saturating_add(1);
                                            self.bring_to_front(WindowKind::Terminal);
                                        }
                                        "Keyboard Shortcuts" => {
                                            self.show_shortcuts_overlay = true;
                                        }
                                        "Start Screen Saver" => {
                                            self.start_screensaver();
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    });
            });
    }

    fn render_desktop_rename_dialog(&mut self, ctx: &egui::Context) {
        let Some(target) = self.desktop_rename_target.clone() else {
            return;
        };
        let screen = ctx.content_rect();
        egui::Area::new(Id::new("desktop_rename_dialog"))
            .fixed_pos(screen.center() - Vec2::new(180.0, 40.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(34, 34, 38, 245))
                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(40)))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(320.0);
                        ui.label(
                            RichText::new("Rename Desktop Item")
                                .size(13.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.add_space(6.0);
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.desktop_rename_buffer)
                                .desired_width(280.0)
                                .font(FontId::proportional(12.0)),
                        );
                        if !response.has_focus() {
                            response.request_focus();
                        }
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.desktop_rename_target = None;
                                self.desktop_rename_buffer.clear();
                            }
                            if ui.button("Rename").clicked() {
                                if let Some(new_path) = Self::desktop_rename_destination(
                                    &target,
                                    &self.desktop_rename_buffer,
                                ) {
                                    match rename_entry(&target, &new_path) {
                                        Ok(()) => {
                                            self.desktop_entries =
                                                read_directory(&desktop_directory());
                                            if self.fm_current_dir == desktop_directory() {
                                                self.fm_entries =
                                                    read_directory(&self.fm_current_dir);
                                            }
                                            self.desktop_selected_paths = vec![new_path];
                                            self.desktop_rename_target = None;
                                            self.desktop_rename_buffer.clear();
                                        }
                                        Err(err) => {
                                            self.toast_manager.push(Toast::new(
                                                "Rename Failed",
                                                err,
                                                Color32::from_rgb(255, 59, 48),
                                            ));
                                        }
                                    }
                                }
                            }
                        });
                    });
            });
    }

    // ── Window content renderers ─────────────────────────────────────────────

    fn content_overview(
        ui: &mut egui::Ui,
        si: &RealSystemInfo,
        telemetry: &Telemetry,
        cpu_history: &VecDeque<f32>,
    ) {
        ui.heading(RichText::new("AuroraOS").color(Color32::from_gray(240)));
        ui.add_space(4.0);
        ui.label(
            RichText::new("System Dashboard — Live")
                .size(13.0)
                .color(Color32::from_gray(180)),
        );
        ui.add_space(12.0);

        egui::Grid::new("overview_metrics")
            .num_columns(2)
            .spacing(Vec2::new(10.0, 10.0))
            .show(ui, |ui| {
                let cpu_str = format!("{:.1}%", si.cpu_usage);
                let mem_str = format!(
                    "{:.1} / {:.1} GB ({:.0}%)",
                    si.used_memory_gb, si.total_memory_gb, si.memory_pct
                );
                let disk_str = format!("{:.0} / {:.0} GB", si.disk_used_gb, si.disk_total_gb);
                let proc_str = format!("{} running", si.process_count);

                let metrics: &[(&str, &str, Color32)] = &[
                    ("CPU", &cpu_str, Color32::from_rgb(52, 199, 89)),
                    ("Memory", &mem_str, Color32::from_rgb(0, 122, 255)),
                    ("Disk", &disk_str, Color32::from_rgb(255, 149, 0)),
                    ("Processes", &proc_str, Color32::from_rgb(88, 86, 214)),
                ];
                for (i, (label, value, color)) in metrics.iter().enumerate() {
                    egui::Frame::default()
                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 18))
                        .stroke(Stroke::new(1.0, Color32::from_white_alpha(40)))
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .show(ui, |ui| {
                            ui.set_min_size(Vec2::new(200.0, 50.0));
                            ui.label(
                                RichText::new(*label)
                                    .size(11.0)
                                    .color(Color32::from_gray(160)),
                            );
                            ui.label(RichText::new(*value).size(16.0).strong().color(*color));
                        });
                    if i % 2 == 1 {
                        ui.end_row();
                    }
                }
            });

        // CPU usage history graph
        if cpu_history.len() >= 2 {
            ui.add_space(12.0);
            ui.label(
                RichText::new("CPU History")
                    .size(11.0)
                    .strong()
                    .color(Color32::from_gray(160)),
            );
            ui.add_space(4.0);
            let graph_size = Vec2::new(ui.available_width().min(440.0), 60.0);
            let (graph_rect, _) = ui.allocate_exact_size(graph_size, Sense::hover());
            let painter = ui.painter_at(graph_rect);

            // Background
            painter.rect_filled(
                graph_rect,
                CornerRadius::same(6),
                Color32::from_rgba_unmultiplied(0, 0, 0, 40),
            );

            // Grid lines at 25%, 50%, 75%
            for pct in [25.0f32, 50.0, 75.0] {
                let y = graph_rect.bottom() - (pct / 100.0) * graph_rect.height();
                painter.line_segment(
                    [
                        Pos2::new(graph_rect.left(), y),
                        Pos2::new(graph_rect.right(), y),
                    ],
                    Stroke::new(0.5, Color32::from_white_alpha(20)),
                );
            }

            // Plot CPU values as a filled area + line
            let n = cpu_history.len();
            let dx = graph_rect.width() / (n.max(2) - 1) as f32;
            let green = Color32::from_rgb(52, 199, 89);
            let green_fill = Color32::from_rgba_unmultiplied(52, 199, 89, 40);

            // Build filled polygon: bottom-left → data points → bottom-right
            let mut fill_points = vec![Pos2::new(graph_rect.left(), graph_rect.bottom())];
            let mut line_points = Vec::with_capacity(n);
            for (i, &val) in cpu_history.iter().enumerate() {
                let x = graph_rect.left() + i as f32 * dx;
                let y = graph_rect.bottom() - (val / 100.0).clamp(0.0, 1.0) * graph_rect.height();
                fill_points.push(Pos2::new(x, y));
                line_points.push(Pos2::new(x, y));
            }
            fill_points.push(Pos2::new(graph_rect.right(), graph_rect.bottom()));

            // Fill area under the curve
            if fill_points.len() >= 3 {
                painter.add(Shape::convex_polygon(fill_points, green_fill, Stroke::NONE));
            }
            // Line on top
            if line_points.len() >= 2 {
                painter.add(Shape::line(line_points, Stroke::new(1.5, green)));
            }

            // Current value label
            painter.text(
                Pos2::new(graph_rect.right() - 4.0, graph_rect.top() + 4.0),
                Align2::RIGHT_TOP,
                format!("{:.0}%", si.cpu_usage),
                FontId::proportional(10.0),
                green,
            );
        }

        ui.add_space(12.0);

        // Top processes
        ui.label(
            RichText::new("Top Processes")
                .size(11.0)
                .strong()
                .color(Color32::from_gray(160)),
        );
        ui.add_space(4.0);
        let mut procs: Vec<(&sysinfo::Pid, &sysinfo::Process)> =
            si.sys.processes().iter().collect();
        procs.sort_by(|a, b| {
            b.1.cpu_usage()
                .partial_cmp(&a.1.cpu_usage())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let top_n = procs.iter().take(8);
        egui::ScrollArea::vertical()
            .max_height(120.0)
            .show(ui, |ui| {
                for (_pid, proc_info) in top_n {
                    ui.horizontal(|ui| {
                        let name = proc_info.name().to_string_lossy();
                        let cpu = proc_info.cpu_usage();
                        let mem_mb = proc_info.memory() as f64 / (1024.0 * 1024.0);
                        ui.label(
                            RichText::new(format!("{:<20}", &name[..name.len().min(20)]))
                                .monospace()
                                .size(10.0)
                                .color(Color32::from_gray(200)),
                        );
                        ui.label(
                            RichText::new(format!("{:.1}%", cpu))
                                .monospace()
                                .size(10.0)
                                .color(if cpu > 50.0 {
                                    Color32::from_rgb(255, 100, 100)
                                } else if cpu > 10.0 {
                                    Color32::from_rgb(255, 214, 10)
                                } else {
                                    Color32::from_gray(160)
                                }),
                        );
                        ui.label(
                            RichText::new(format!("{:.0} MB", mem_mb))
                                .monospace()
                                .size(10.0)
                                .color(Color32::from_gray(120)),
                        );
                    });
                }
            });

        ui.add_space(8.0);

        // Network + Battery + Daemon info
        if si.network_up {
            ui.label(
                RichText::new(format!("Network: {} (connected)", si.network_name))
                    .size(12.0)
                    .color(Color32::from_rgb(124, 236, 112)),
            );
        } else {
            ui.label(
                RichText::new("Network: disconnected")
                    .size(12.0)
                    .color(Color32::from_rgb(255, 152, 152)),
            );
        }
        if si.battery_available {
            let batt_str = if si.battery_charging {
                format!("Battery: {:.0}% (charging)", si.battery_pct)
            } else {
                format!("Battery: {:.0}%", si.battery_pct)
            };
            let batt_color = if si.battery_pct < 20.0 {
                Color32::from_rgb(255, 59, 48)
            } else {
                Color32::from_rgb(52, 199, 89)
            };
            ui.label(RichText::new(batt_str).size(12.0).color(batt_color));
        }

        ui.add_space(8.0);
        ui.label(
            RichText::new("Daemon Status")
                .strong()
                .color(Color32::from_gray(220)),
        );
        ui.add_space(4.0);
        if telemetry.status.is_empty() {
            ui.label(RichText::new("No daemon connection.").color(Color32::from_gray(140)));
        } else {
            egui::ScrollArea::vertical()
                .max_height(60.0)
                .id_salt("daemon_scroll")
                .show(ui, |ui| {
                    for line in telemetry.status.lines() {
                        ui.label(
                            RichText::new(line)
                                .monospace()
                                .size(11.0)
                                .color(Color32::from_gray(200)),
                        );
                    }
                });
        }
        if let Some(err) = &telemetry.last_error {
            ui.colored_label(Color32::from_rgb(255, 140, 140), err);
        }
    }

    fn content_terminal_pty(ui: &mut egui::Ui, pty: &mut PtyTerminal, input: &mut String) -> bool {
        let gray = Color32::from_gray(185);
        let mut sent = false;

        // Resize PTY to match available UI area
        let avail = ui.available_size();
        let char_w = 7.2_f32; // approximate monospace char width at 12px
        let char_h = 16.0_f32;
        let cols = ((avail.x / char_w).floor() as u16).max(20);
        let rows = ((avail.y / char_h).floor() as u16).max(4);
        pty.resize(cols, rows);

        // Ctrl+L to clear scrollback
        if ui.input(|i| i.key_pressed(egui::Key::L) && i.modifiers.ctrl) {
            pty.clear();
        }
        // Ctrl+C to send interrupt
        if ui.input(|i| i.key_pressed(egui::Key::C) && i.modifiers.ctrl) {
            let _ = pty.writer.write_all(b"\x03"); // ETX
            let _ = pty.writer.flush();
            input.clear();
        }

        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // Render scrollback
                for line in &pty.scrollback {
                    ui.label(RichText::new(line).monospace().size(12.0).color(gray));
                }

                // Input line
                ui.horizontal(|ui| {
                    let te = egui::TextEdit::singleline(input)
                        .font(FontId::monospace(12.0))
                        .text_color(Color32::from_gray(220))
                        .desired_width(ui.available_width() - 10.0)
                        .frame(false);
                    let resp = ui.add(te);
                    if !resp.has_focus() {
                        resp.request_focus();
                    }

                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    // Tab completion — send Tab character to PTY
                    if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Tab)) {
                        let _ = pty.writer.write_all(input.as_bytes());
                        let _ = pty.writer.write_all(b"\t");
                        let _ = pty.writer.flush();
                        input.clear();
                    }
                    if enter {
                        let cmd = input.trim().to_string();
                        pty.send(&cmd);
                        input.clear();
                        resp.request_focus();
                        sent = true;
                    }
                });
            });
        sent
    }

    fn content_terminal_builtin(
        ui: &mut egui::Ui,
        si: &RealSystemInfo,
        extra_lines: &[(String, Color32)],
        input: &mut String,
    ) -> Option<String> {
        let green = Color32::from_rgb(166, 227, 161);
        let gray = Color32::from_gray(140);
        let cyan = Color32::from_rgb(137, 220, 235);
        let mut submitted_cmd: Option<String> = None;

        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("aurora@localhost ~ % neofetch")
                        .monospace()
                        .size(12.0)
                        .color(green),
                );
                ui.add_space(4.0);

                let os_name = sysinfo::System::name().unwrap_or_else(|| "Unknown".into());
                let os_ver = sysinfo::System::os_version().unwrap_or_else(|| "".into());
                let host = sysinfo::System::host_name().unwrap_or_else(|| "localhost".into());
                let kernel = sysinfo::System::kernel_version().unwrap_or_else(|| "".into());
                let cpu_brand = if !si.sys.cpus().is_empty() {
                    si.sys.cpus()[0].brand().to_string()
                } else {
                    "Unknown".into()
                };
                let cpu_count = si.sys.cpus().len();

                let info_lines = [
                    (format!("aurora@{host}"), cyan),
                    ("─────────────────────".into(), Color32::from_gray(80)),
                    (format!("OS:       AuroraOS (on {os_name} {os_ver})"), gray),
                    (format!("Host:     {host}"), gray),
                    (format!("Kernel:   {kernel}"), gray),
                    (format!("CPU:      {cpu_brand} ({cpu_count} cores)"), gray),
                    (format!("Usage:    {:.1}%", si.cpu_usage), gray),
                    (
                        format!(
                            "Memory:   {:.1} GB / {:.1} GB ({:.0}%)",
                            si.used_memory_gb, si.total_memory_gb, si.memory_pct
                        ),
                        gray,
                    ),
                    (
                        format!(
                            "Disk:     {:.0} GB / {:.0} GB",
                            si.disk_used_gb, si.disk_total_gb
                        ),
                        gray,
                    ),
                    (format!("Procs:    {}", si.process_count), gray),
                    (
                        format!(
                            "Network:  {}",
                            if si.network_up {
                                &si.network_name
                            } else {
                                "disconnected"
                            }
                        ),
                        gray,
                    ),
                ];

                for (line, color) in &info_lines {
                    ui.label(RichText::new(line).monospace().size(12.0).color(*color));
                }

                if si.battery_available {
                    let batt_str = if si.battery_charging {
                        format!("Battery:  {:.0}% (charging)", si.battery_pct)
                    } else {
                        format!("Battery:  {:.0}%", si.battery_pct)
                    };
                    let batt_color = if si.battery_pct < 20.0 {
                        Color32::from_rgb(255, 100, 100)
                    } else {
                        gray
                    };
                    ui.label(
                        RichText::new(batt_str)
                            .monospace()
                            .size(12.0)
                            .color(batt_color),
                    );
                }

                ui.add_space(8.0);
                ui.label(
                    RichText::new("aurora@localhost ~ % aurora services list")
                        .monospace()
                        .size(12.0)
                        .color(green),
                );
                ui.add_space(4.0);
                for (dot, rest) in [
                    ("●", " display-server    active (running)"),
                    ("●", " window-manager    active (running)"),
                    ("●", " network-daemon    active (running)"),
                    ("●", " audio-server      active (running)"),
                    ("●", " file-indexer      active (running)"),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(dot).monospace().size(12.0).color(green));
                        ui.label(RichText::new(rest).monospace().size(12.0).color(gray));
                    });
                }

                for (line, color) in extra_lines {
                    ui.label(RichText::new(line).monospace().size(12.0).color(*color));
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("aurora@localhost ~ % ")
                            .monospace()
                            .size(12.0)
                            .color(green),
                    );
                    let te = egui::TextEdit::singleline(input)
                        .font(FontId::monospace(12.0))
                        .text_color(Color32::from_gray(220))
                        .desired_width(ui.available_width() - 10.0)
                        .frame(false);
                    let resp = ui.add(te);
                    if !resp.has_focus() {
                        resp.request_focus();
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let cmd = input.trim().to_string();
                        if !cmd.is_empty() {
                            submitted_cmd = Some(cmd);
                        }
                        input.clear();
                    }
                });
            });
        submitted_cmd
    }

    fn execute_terminal_command(cmd: &str, si: &RealSystemInfo) -> Vec<(String, Color32)> {
        let green = Color32::from_rgb(166, 227, 161);
        let gray = Color32::from_gray(140);
        let white = Color32::from_gray(205);
        let red = Color32::from_rgb(255, 100, 100);
        let cyan = Color32::from_rgb(137, 220, 235);

        let mut out = vec![(format!("aurora@localhost ~ % {cmd}"), green)];
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        match parts.first().copied().unwrap_or("") {
            "help" => {
                out.push(("Available commands:".into(), cyan));
                for line in [
                    "  help          Show this help",
                    "  whoami        Current user",
                    "  hostname      Machine hostname",
                    "  uname         System info",
                    "  uptime        System uptime",
                    "  ps            Top processes",
                    "  free          Memory usage",
                    "  df            Disk usage",
                    "  date          Current date/time",
                    "  echo [text]   Echo text",
                    "  clear         Clear terminal",
                    "  aurora status  Daemon status",
                    "  open <path>   Open file with system app",
                    "  run <program> Launch an executable",
                    "  <any>         Try as system command",
                ] {
                    out.push((line.into(), gray));
                }
            }
            "whoami" => out.push(("aurora".into(), white)),
            "hostname" => {
                let host = sysinfo::System::host_name().unwrap_or_else(|| "localhost".into());
                out.push((host, white));
            }
            "uname" => {
                let os = sysinfo::System::name().unwrap_or_else(|| "AuroraOS".into());
                let ver = sysinfo::System::os_version().unwrap_or_default();
                let kern = sysinfo::System::kernel_version().unwrap_or_default();
                out.push((format!("{os} {ver} kernel {kern}"), white));
            }
            "uptime" => {
                let boot = sysinfo::System::boot_time();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let up_secs = now.saturating_sub(boot);
                let hours = up_secs / 3600;
                let mins = (up_secs % 3600) / 60;
                out.push((format!("up {hours}h {mins}m"), white));
            }
            "ps" => {
                out.push((
                    format!("{:<6} {:<20} {:>6} {:>8}", "PID", "NAME", "CPU%", "MEM"),
                    cyan,
                ));
                let mut procs: Vec<_> = si.sys.processes().iter().collect();
                procs.sort_by(|a, b| {
                    b.1.cpu_usage()
                        .partial_cmp(&a.1.cpu_usage())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                for (pid, p) in procs.iter().take(10) {
                    let name = p.name().to_string_lossy();
                    let name_short = &name[..name.len().min(20)];
                    let mem_mb = p.memory() as f64 / (1024.0 * 1024.0);
                    out.push((
                        format!(
                            "{:<6} {:<20} {:>5.1}% {:>6.0} MB",
                            pid.as_u32(),
                            name_short,
                            p.cpu_usage(),
                            mem_mb
                        ),
                        gray,
                    ));
                }
            }
            "free" => {
                out.push((format!("Total:  {:.1} GB", si.total_memory_gb), white));
                out.push((
                    format!(
                        "Used:   {:.1} GB ({:.0}%)",
                        si.used_memory_gb, si.memory_pct
                    ),
                    white,
                ));
                out.push((
                    format!("Free:   {:.1} GB", si.total_memory_gb - si.used_memory_gb),
                    white,
                ));
            }
            "df" => {
                out.push((format!("Total:  {:.0} GB", si.disk_total_gb), white));
                out.push((format!("Used:   {:.0} GB", si.disk_used_gb), white));
                out.push((
                    format!("Avail:  {:.0} GB", si.disk_total_gb - si.disk_used_gb),
                    white,
                ));
            }
            "date" => {
                out.push((
                    Local::now().format("%Y-%m-%d %H:%M:%S %Z").to_string(),
                    white,
                ));
            }
            "echo" => {
                let text = parts[1..].join(" ");
                out.push((text, white));
            }
            "clear" => {
                return vec![("__CLEAR__".into(), Color32::TRANSPARENT)];
            }
            "aurora" => {
                if parts.get(1).copied() == Some("status") {
                    out.push(("AuroraOS Desktop v0.1.0".into(), cyan));
                    out.push((
                        format!(
                            "CPU: {:.1}% | Mem: {:.1}/{:.1} GB | Procs: {}",
                            si.cpu_usage, si.used_memory_gb, si.total_memory_gb, si.process_count
                        ),
                        gray,
                    ));
                } else {
                    out.push((
                        format!(
                            "aurora: unknown subcommand '{}'",
                            parts.get(1).unwrap_or(&"")
                        ),
                        red,
                    ));
                }
            }
            "open" | "start" => {
                if let Some(path) = parts.get(1) {
                    let p = std::path::Path::new(path);
                    if p.exists() {
                        if Self::is_supported_audio_path(p) {
                            out.push((
                                format!("__OPEN_MUSIC__{}", p.display()),
                                Color32::TRANSPARENT,
                            ));
                        } else if Self::is_supported_video_path(p) {
                            out.push((
                                format!("__OPEN_VIDEO__{}", p.display()),
                                Color32::TRANSPARENT,
                            ));
                        } else {
                            open_file_with_system(p);
                        }
                        out.push((format!("Opening {path}..."), white));
                    } else if Self::looks_like_browser_target(path) {
                        out.push((
                            format!("__OPEN_BROWSER__{}", path),
                            Color32::TRANSPARENT,
                        ));
                        out.push((format!("Opening {path}..."), white));
                    } else {
                        out.push((format!("open: no such file: {path}"), red));
                    }
                } else {
                    out.push(("Usage: open <path>".into(), gray));
                }
            }
            "run" => {
                if let Some(program) = parts.get(1) {
                    let candidate = std::path::Path::new(program);
                    if candidate.exists() && Self::is_supported_audio_path(candidate) {
                        out.push((
                            format!("__OPEN_MUSIC__{}", candidate.display()),
                            Color32::TRANSPARENT,
                        ));
                        out.push((format!("Opening {}...", candidate.display()), white));
                    } else if candidate.exists() && Self::is_supported_video_path(candidate) {
                        out.push((
                            format!("__OPEN_VIDEO__{}", candidate.display()),
                            Color32::TRANSPARENT,
                        ));
                        out.push((format!("Opening {}...", candidate.display()), white));
                    } else {
                        let args: Vec<&str> = parts[2..].to_vec();
                        match launch_program(program, &args) {
                            Ok(()) => out.push((format!("Launched {program}"), white)),
                            Err(e) => out.push((e, red)),
                        }
                    }
                } else {
                    out.push(("Usage: run <program> [args...]".into(), gray));
                }
            }
            _other => {
                // Try running as a real system command
                match std::process::Command::new("cmd").args(["/C", cmd]).output() {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        for line in stdout.lines() {
                            out.push((line.to_string(), white));
                        }
                        for line in stderr.lines() {
                            if !line.is_empty() {
                                out.push((line.to_string(), red));
                            }
                        }
                        if stdout.is_empty() && stderr.is_empty() && !output.status.success() {
                            out.push((
                                format!(
                                    "Command exited with code {}",
                                    output.status.code().unwrap_or(-1)
                                ),
                                red,
                            ));
                        }
                    }
                    Err(e) => {
                        out.push((format!("Failed to execute: {e}"), red));
                    }
                }
            }
        }
        out.push(("".into(), Color32::TRANSPARENT));
        out
    }

    fn content_filemanager(
        ui: &mut egui::Ui,
        current_dir: &std::path::Path,
        entries: &[FmEntry],
        show_new_dialog: &mut bool,
        new_name: &mut String,
        new_is_dir: &mut bool,
        rename_target: &mut Option<PathBuf>,
        rename_buffer: &mut String,
        selected_path: &mut Option<PathBuf>,
        dragged_path: &mut Option<PathBuf>,
        sidebar_favorites: &mut Vec<PathBuf>,
        file_tags: &mut FileTags,
        tag_labels: &[(TagColor, String)],
        custom_smart_folders: &[CustomSmartFolder],
        selected_tag_filters: &mut Vec<TagColor>,
        tag_filter_match_all: &mut bool,
        tabs: &[FileManagerTab],
        active_tab: usize,
        dragged_tab_index: &mut Option<usize>,
        tab_scroll: &mut usize,
        show_sidebar: bool,
        show_path_bar: bool,
        show_preview_pane: bool,
        show_status_bar: bool,
        toolbar_search: &mut String,
        view_mode: &mut FileManagerViewMode,
        sort_field: &mut FileManagerSortField,
        icon_scale: &mut f32,
        file_info_target: &mut Option<PathBuf>,
    ) -> Option<PathBuf> {
        let mut navigate_to: Option<PathBuf> = None;
        let mut content_drop_result: Option<Result<String, String>> = None;
        let mut pending_tab_move: Option<(usize, usize)> = None;
        let visible_tabs = 6usize;
        let home = dirs_home();
        let trash_path = trash_dir();
        let in_trash = current_dir == trash_path.as_path();
        let accent = Color32::from_rgb(0, 122, 255);
        let btn_bg = Color32::from_rgba_unmultiplied(255, 255, 255, 15);
        let filtered_entries = AuroraDesktopApp::sort_file_manager_entries(
            &AuroraDesktopApp::filter_file_manager_entries(entries, toolbar_search),
            *sort_field,
        );

        ui.horizontal(|ui| {
            let mut pending_switch = None;
            let mut pending_close = None;
            let tab_range =
                AuroraDesktopApp::visible_tab_range(tabs.len(), *tab_scroll, visible_tabs);
            let tab_start = tab_range.start;
            let tab_end = tab_range.end;
            if tabs.len() > visible_tabs {
                let left_enabled = tab_start > 0;
                if ui
                    .add_enabled(left_enabled, egui::Button::new("<"))
                    .clicked()
                {
                    *tab_scroll = tab_start.saturating_sub(1);
                }
            }
            for idx in tab_range {
                let tab = &tabs[idx];
                let label = tab
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Home");
                let fill = if idx == active_tab {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 35)
                } else {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                };
                let tab_frame = egui::Frame::default()
                    .fill(fill)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        if ui.selectable_label(idx == active_tab, label).clicked() {
                            pending_switch = Some(idx);
                        }
                        if tabs.len() > 1 && ui.small_button("x").clicked() {
                            pending_close = Some(idx);
                        }
                    });
                let tab_resp = ui.interact(
                    tab_frame.response.rect,
                    Id::new(("fm_tab_interact", idx)),
                    Sense::click_and_drag(),
                );
                if tab_resp.middle_clicked() && tabs.len() > 1 {
                    pending_close = Some(idx);
                }
                if tab_resp.drag_started() {
                    *dragged_tab_index = Some(idx);
                }
                if let Some(from) = *dragged_tab_index {
                    if from != idx && tab_resp.hovered() && !ui.input(|i| i.pointer.primary_down())
                    {
                        pending_tab_move = Some((from, idx));
                    }
                }
                ui.add_space(4.0);
            }
            if dragged_tab_index.is_some() && !ui.input(|i| i.pointer.primary_down()) {
                *dragged_tab_index = None;
            }
            if tabs.len() > visible_tabs {
                let right_enabled = tab_end < tabs.len();
                if ui
                    .add_enabled(right_enabled, egui::Button::new(">"))
                    .clicked()
                {
                    *tab_scroll = AuroraDesktopApp::clamp_tab_scroll(
                        tab_start.saturating_add(1),
                        tabs.len(),
                        visible_tabs,
                    );
                }
                ui.add_space(4.0);
            }
            if ui.small_button("+").clicked() {
                navigate_to = Some(PathBuf::from("__NEW_TAB__"));
            }
            if let Some(idx) = pending_switch {
                navigate_to = Some(PathBuf::from(format!("__SWITCH_TAB__{idx}")));
            }
            if let Some(idx) = pending_close {
                navigate_to = Some(PathBuf::from(format!("__CLOSE_TAB__{idx}")));
            }
            if let Some((from, to)) = pending_tab_move {
                *dragged_tab_index = None;
                navigate_to = Some(PathBuf::from(format!("__MOVE_TAB__{from}:{to}")));
            }
        });
        ui.add_space(6.0);

        // Toolbar
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !in_trash,
                    egui::Button::new(RichText::new("←").size(11.0).color(Color32::from_gray(220)))
                        .fill(btn_bg)
                        .corner_radius(CornerRadius::same(4)),
                )
                .clicked()
            {
                navigate_to = Some(PathBuf::from("__FM_BACK__"));
            }
            if ui
                .add_enabled(
                    !in_trash,
                    egui::Button::new(RichText::new("→").size(11.0).color(Color32::from_gray(220)))
                        .fill(btn_bg)
                        .corner_radius(CornerRadius::same(4)),
                )
                .clicked()
            {
                navigate_to = Some(PathBuf::from("__FM_FORWARD__"));
            }
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("+ Folder")
                            .size(11.0)
                            .color(Color32::from_gray(220)),
                    )
                    .fill(btn_bg)
                    .corner_radius(CornerRadius::same(4)),
                )
                .clicked()
            {
                *show_new_dialog = true;
                *new_is_dir = true;
                new_name.clear();
            }
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("+ File")
                            .size(11.0)
                            .color(Color32::from_gray(220)),
                    )
                    .fill(btn_bg)
                    .corner_radius(CornerRadius::same(4)),
                )
                .clicked()
            {
                *show_new_dialog = true;
                *new_is_dir = false;
                new_name.clear();
            }
            if in_trash
                && ui
                    .add(
                        egui::Button::new(
                            RichText::new("Empty Trash")
                                .size(11.0)
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(255, 59, 48))
                        .corner_radius(CornerRadius::same(4)),
                    )
                    .clicked()
            {
                match empty_trash() {
                    Ok(()) => navigate_to = Some(PathBuf::from("__NOTIFY_OK__Trash emptied")),
                    Err(e) => navigate_to = Some(PathBuf::from(format!("__NOTIFY_ERR__{e}"))),
                }
            }
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Add to Favorites")
                            .size(11.0)
                            .color(Color32::from_gray(220)),
                    )
                    .fill(btn_bg)
                    .corner_radius(CornerRadius::same(4)),
                )
                .clicked()
                && !sidebar_favorites.iter().any(|path| path == current_dir)
            {
                sidebar_favorites.push(current_dir.to_path_buf());
                navigate_to = Some(PathBuf::from("__NOTIFY_OK__Added to Favorites"));
            }
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Share")
                            .size(11.0)
                            .color(Color32::from_gray(220)),
                    )
                    .fill(btn_bg)
                    .corner_radius(CornerRadius::same(4)),
                )
                .clicked()
            {
                navigate_to = Some(PathBuf::from("__NOTIFY_OK__Share sheet coming next"));
            }
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Sort")
                            .size(11.0)
                            .color(Color32::from_gray(220)),
                    )
                    .fill(btn_bg)
                    .corner_radius(CornerRadius::same(4)),
                )
                .clicked()
            {
                *sort_field = match *sort_field {
                    FileManagerSortField::Name => FileManagerSortField::Kind,
                    FileManagerSortField::Kind => FileManagerSortField::Size,
                    FileManagerSortField::Size => FileManagerSortField::Name,
                };
            }
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Actions")
                            .size(11.0)
                            .color(Color32::from_gray(220)),
                    )
                    .fill(btn_bg)
                    .corner_radius(CornerRadius::same(4)),
                )
                .clicked()
            {
                navigate_to = Some(PathBuf::from(
                    "__NOTIFY_OK__Use context menus for file actions",
                ));
            }
            if *view_mode == FileManagerViewMode::Icon {
                for (label, scale) in [("S", 0.8_f32), ("M", 1.0_f32), ("L", 1.25_f32)] {
                    let fill = if (*icon_scale - scale).abs() < f32::EPSILON {
                        accent
                    } else {
                        btn_bg
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(label).size(10.0).color(Color32::WHITE),
                            )
                            .fill(fill),
                        )
                        .clicked()
                    {
                        *icon_scale = scale;
                    }
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add(
                    egui::TextEdit::singleline(toolbar_search)
                        .hint_text("Search")
                        .desired_width(150.0),
                );
            });
        });

        // New item dialog
        if *show_new_dialog {
            ui.horizontal(|ui| {
                let label = if *new_is_dir {
                    "New folder:"
                } else {
                    "New file:"
                };
                ui.label(
                    RichText::new(label)
                        .size(11.0)
                        .color(Color32::from_gray(180)),
                );
                let te = egui::TextEdit::singleline(new_name)
                    .desired_width(200.0)
                    .font(FontId::proportional(12.0));
                let resp = ui.add(te);
                if !resp.has_focus() && new_name.is_empty() {
                    resp.request_focus();
                }

                if ui
                    .add(
                        egui::Button::new(RichText::new("Create").size(11.0).color(Color32::WHITE))
                            .fill(accent)
                            .corner_radius(CornerRadius::same(4)),
                    )
                    .clicked()
                    && !new_name.is_empty()
                {
                    let target = current_dir.join(new_name.as_str());
                    let result = if *new_is_dir {
                        create_directory(&target)
                    } else {
                        create_file(&target)
                    };
                    match &result {
                        Ok(()) => {
                            let kind = if *new_is_dir { "folder" } else { "file" };
                            navigate_to = Some(PathBuf::from(format!(
                                "__NOTIFY_OK__Created {kind} '{}'",
                                new_name
                            )));
                        }
                        Err(e) => {
                            navigate_to = Some(PathBuf::from(format!("__NOTIFY_ERR__{e}")));
                        }
                    }
                    *show_new_dialog = false;
                }
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Cancel")
                                .size(11.0)
                                .color(Color32::from_gray(180)),
                        )
                        .fill(btn_bg)
                        .corner_radius(CornerRadius::same(4)),
                    )
                    .clicked()
                {
                    *show_new_dialog = false;
                }
            });
        }

        // Rename dialog
        if let Some(ref target_path) = rename_target.clone() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Rename:")
                        .size(11.0)
                        .color(Color32::from_gray(180)),
                );
                let te = egui::TextEdit::singleline(rename_buffer)
                    .desired_width(200.0)
                    .font(FontId::proportional(12.0));
                ui.add(te);
                if ui
                    .add(
                        egui::Button::new(RichText::new("OK").size(11.0).color(Color32::WHITE))
                            .fill(accent)
                            .corner_radius(CornerRadius::same(4)),
                    )
                    .clicked()
                    && !rename_buffer.is_empty()
                {
                    let new_path = target_path
                        .parent()
                        .unwrap_or(current_dir)
                        .join(rename_buffer.as_str());
                    match rename_entry(target_path, &new_path) {
                        Ok(()) => {
                            navigate_to = Some(PathBuf::from(format!(
                                "__NOTIFY_OK__Renamed to '{}'",
                                rename_buffer
                            )));
                        }
                        Err(e) => {
                            navigate_to = Some(PathBuf::from(format!("__NOTIFY_ERR__{e}")));
                        }
                    }
                    *rename_target = None;
                }
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Cancel")
                                .size(11.0)
                                .color(Color32::from_gray(180)),
                        )
                        .fill(btn_bg)
                        .corner_radius(CornerRadius::same(4)),
                    )
                    .clicked()
                {
                    *rename_target = None;
                }
            });
        }

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if show_sidebar {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(8, 8))
                    .show(ui, |ui| {
                        ui.set_min_width(130.0);
                        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                            ui.label(
                                RichText::new("Favorites")
                                    .size(10.0)
                                    .strong()
                                    .color(Color32::from_gray(140)),
                            );
                            ui.add_space(4.0);
                            let default_favorites = AuroraDesktopApp::default_sidebar_favorites();
                            let favorite_snapshot = sidebar_favorites.clone();
                            for path in favorite_snapshot.iter() {
                                let label = if *path == home {
                                    "Home".to_string()
                                } else if *path == trash_path {
                                    "Trash".to_string()
                                } else {
                                    path.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("Folder")
                                        .to_string()
                                };
                                let is_current = current_dir == path.as_path();
                                let color = if is_current {
                                    Color32::from_rgb(0, 122, 255)
                                } else {
                                    Color32::from_gray(220)
                                };
                                let resp = ui.add(
                                    egui::Label::new(
                                        RichText::new(format!("  {label}")).size(12.0).color(color),
                                    )
                                    .sense(Sense::click()),
                                );
                                if resp.clicked() && (path.exists() || *path == trash_path) {
                                    navigate_to = Some(path.clone());
                                }
                                if let Some(dragged) = dragged_path.clone() {
                                    if resp.hovered() {
                                        ui.painter().rect_stroke(
                                            resp.rect.expand(2.0),
                                            CornerRadius::same(4),
                                            Stroke::new(
                                                1.0,
                                                Color32::from_rgba_unmultiplied(0, 122, 255, 160),
                                            ),
                                            StrokeKind::Outside,
                                        );
                                    }
                                    if resp.hovered() && !ui.input(|i| i.pointer.any_down()) {
                                        if let Some(target_path) =
                                            AuroraDesktopApp::favorite_drop_target(
                                                &dragged,
                                                path,
                                                &trash_path,
                                            )
                                        {
                                            let result = if target_path == trash_path {
                                                delete_entry(&dragged)
                                                    .map(|_| "Moved item to Trash".to_string())
                                            } else {
                                                let copy_drag =
                                                    AuroraDesktopApp::drag_uses_copy_modifier(
                                                        ui.input(|i| i.modifiers),
                                                    );
                                                if copy_drag {
                                                    copy_entry_to_directory(&dragged, &target_path)
                                                        .map(|_| {
                                                            format!(
                                                                "Copied item to {}",
                                                                target_path.display()
                                                            )
                                                        })
                                                } else {
                                                    move_entry_to_directory(&dragged, &target_path)
                                                        .map(|_| {
                                                            format!(
                                                                "Moved item to {}",
                                                                target_path.display()
                                                            )
                                                        })
                                                }
                                            };
                                            match result {
                                                Ok(message) => {
                                                    *dragged_path = None;
                                                    *selected_path = None;
                                                    navigate_to = Some(PathBuf::from(format!(
                                                        "__NOTIFY_OK__{message}"
                                                    )));
                                                }
                                                Err(err) => {
                                                    *dragged_path = None;
                                                    navigate_to = Some(PathBuf::from(format!(
                                                        "__NOTIFY_ERR__{err}"
                                                    )));
                                                }
                                            }
                                        }
                                    }
                                }
                                resp.context_menu(|ui| {
                                    let is_builtin =
                                        default_favorites.iter().any(|builtin| builtin == path);
                                    if !is_builtin && ui.button("Remove from Favorites").clicked() {
                                        sidebar_favorites.retain(|favorite| favorite != path);
                                        navigate_to =
                                            Some(PathBuf::from("__NOTIFY_OK__Favorite removed"));
                                        ui.close();
                                    }
                                });
                            }
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("Smart Folders")
                                    .size(10.0)
                                    .strong()
                                    .color(Color32::from_gray(140)),
                            );
                            ui.add_space(4.0);
                            for (label, token) in [
                                ("All Images", "__SMART_FOLDER__images"),
                                ("All Documents", "__SMART_FOLDER__documents"),
                                ("Recent Files", "__SMART_FOLDER__recent"),
                                ("Large Files", "__SMART_FOLDER__large"),
                            ] {
                                let resp = ui.add(
                                    egui::Label::new(
                                        RichText::new(format!("  {label}"))
                                            .size(12.0)
                                            .color(Color32::from_gray(220)),
                                    )
                                    .sense(Sense::click()),
                                );
                                if resp.clicked() {
                                    navigate_to = Some(PathBuf::from(token));
                                }
                            }
                            for folder in custom_smart_folders {
                                let token = format!(
                                    "__SMART_FOLDER__custom_{}",
                                    folder.name.replace(' ', "_")
                                );
                                let resp = ui.add(
                                    egui::Label::new(
                                        RichText::new(format!("  {}", folder.name))
                                            .size(12.0)
                                            .color(Color32::from_gray(220)),
                                    )
                                    .sense(Sense::click()),
                                );
                                if resp.clicked() {
                                    navigate_to = Some(PathBuf::from(token));
                                }
                            }
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("Tags")
                                    .size(10.0)
                                    .strong()
                                    .color(Color32::from_gray(140)),
                            );
                            ui.add_space(4.0);
                            if selected_tag_filters.len() > 1 {
                                ui.horizontal(|ui| {
                                    let any_fill = if !*tag_filter_match_all {
                                        accent
                                    } else {
                                        btn_bg
                                    };
                                    let all_fill = if *tag_filter_match_all {
                                        accent
                                    } else {
                                        btn_bg
                                    };
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("Any")
                                                    .size(10.0)
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(any_fill),
                                        )
                                        .clicked()
                                    {
                                        *tag_filter_match_all = false;
                                        if let Some(token) = AuroraDesktopApp::tag_filter_token(
                                            selected_tag_filters,
                                            false,
                                        ) {
                                            navigate_to = Some(PathBuf::from(format!(
                                                "__SMART_FOLDER__{token}"
                                            )));
                                        }
                                    }
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("All")
                                                    .size(10.0)
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(all_fill),
                                        )
                                        .clicked()
                                    {
                                        *tag_filter_match_all = true;
                                        if let Some(token) = AuroraDesktopApp::tag_filter_token(
                                            selected_tag_filters,
                                            true,
                                        ) {
                                            navigate_to = Some(PathBuf::from(format!(
                                                "__SMART_FOLDER__{token}"
                                            )));
                                        }
                                    }
                                });
                                ui.add_space(4.0);
                            }
                            for color in TagColor::ALL {
                                let label = tag_labels
                                    .iter()
                                    .find(|(stored_color, _)| *stored_color == color)
                                    .map(|(_, label)| label.as_str())
                                    .filter(|label| !label.trim().is_empty())
                                    .unwrap_or(match color {
                                        TagColor::Red => "Red",
                                        TagColor::Orange => "Orange",
                                        TagColor::Yellow => "Yellow",
                                        TagColor::Green => "Green",
                                        TagColor::Blue => "Blue",
                                        TagColor::Purple => "Purple",
                                        TagColor::Gray => "Gray",
                                    });
                                let selected = selected_tag_filters.contains(&color);
                                let color_value = if selected {
                                    accent
                                } else {
                                    Color32::from_gray(220)
                                };
                                let resp = ui.add(
                                    egui::Label::new(
                                        RichText::new(format!("  {label}"))
                                            .size(12.0)
                                            .color(color_value),
                                    )
                                    .sense(Sense::click()),
                                );
                                if resp.clicked() {
                                    let additive =
                                        ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                                    if additive {
                                        if let Some(index) = selected_tag_filters
                                            .iter()
                                            .position(|stored| *stored == color)
                                        {
                                            selected_tag_filters.remove(index);
                                        } else {
                                            selected_tag_filters.push(color);
                                        }
                                    } else {
                                        selected_tag_filters.clear();
                                        selected_tag_filters.push(color);
                                    }
                                    if let Some(token) = AuroraDesktopApp::tag_filter_token(
                                        selected_tag_filters,
                                        *tag_filter_match_all,
                                    ) {
                                        navigate_to =
                                            Some(PathBuf::from(format!("__SMART_FOLDER__{token}")));
                                    }
                                }
                            }
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("Disks")
                                    .size(10.0)
                                    .strong()
                                    .color(Color32::from_gray(140)),
                            );
                            ui.add_space(4.0);
                            let disks = sysinfo::Disks::new_with_refreshed_list();
                            for disk in disks.list() {
                                let mount = disk.mount_point().to_string_lossy().to_string();
                                let label = if mount.len() <= 3 {
                                    format!("  {mount}")
                                } else {
                                    format!("  {}", &mount[..3])
                                };
                                let resp = ui.add(
                                    egui::Label::new(
                                        RichText::new(label)
                                            .size(12.0)
                                            .color(Color32::from_gray(220)),
                                    )
                                    .sense(Sense::click()),
                                );
                                if resp.clicked() {
                                    navigate_to = Some(disk.mount_point().to_path_buf());
                                }
                            }
                        });
                    });

                ui.add_space(8.0);
            }

            // Main content: path bar + file list
            ui.vertical(|ui| {
                if show_path_bar {
                    egui::Frame::default()
                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(egui::Margin::symmetric(8, 5))
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                let segments =
                                    AuroraDesktopApp::file_manager_path_segments(current_dir);
                                for (idx, (label, segment_path)) in segments.iter().enumerate() {
                                    let resp = ui.add(
                                        egui::Button::new(
                                            RichText::new(label)
                                                .size(11.0)
                                                .color(Color32::from_gray(220)),
                                        )
                                        .fill(btn_bg)
                                        .corner_radius(CornerRadius::same(4)),
                                    );
                                    if resp.clicked() {
                                        navigate_to = Some(segment_path.clone());
                                    }
                                    resp.context_menu(|ui| {
                                        if ui.button("Open in New Tab").clicked() {
                                            navigate_to = Some(PathBuf::from(format!(
                                                "__OPEN_TAB__{}",
                                                segment_path.display()
                                            )));
                                            ui.close();
                                        }
                                    });
                                    if let Some(dragged) = dragged_path.clone() {
                                        if resp.hovered() {
                                            ui.painter().rect_stroke(
                                                resp.rect.expand(2.0),
                                                CornerRadius::same(4),
                                                Stroke::new(
                                                    1.0,
                                                    Color32::from_rgba_unmultiplied(
                                                        0, 122, 255, 160,
                                                    ),
                                                ),
                                                StrokeKind::Outside,
                                            );
                                        }
                                        if resp.hovered() && !ui.input(|i| i.pointer.any_down()) {
                                            if let Some(target_path) =
                                                AuroraDesktopApp::directory_row_drop_target(
                                                    &dragged,
                                                    segment_path,
                                                )
                                            {
                                                let copy_drag =
                                                    AuroraDesktopApp::drag_uses_copy_modifier(
                                                        ui.input(|i| i.modifiers),
                                                    );
                                                content_drop_result = Some(if copy_drag {
                                                    copy_entry_to_directory(&dragged, &target_path)
                                                        .map(|_| {
                                                            format!(
                                                                "Copied item to {}",
                                                                target_path.display()
                                                            )
                                                        })
                                                } else {
                                                    move_entry_to_directory(&dragged, &target_path)
                                                        .map(|_| {
                                                            format!(
                                                                "Moved item to {}",
                                                                target_path.display()
                                                            )
                                                        })
                                                });
                                                *dragged_path = None;
                                                *selected_path = None;
                                            }
                                        }
                                    }
                                    if idx + 1 < segments.len() {
                                        ui.label(
                                            RichText::new(">")
                                                .size(10.0)
                                                .color(Color32::from_gray(120)),
                                        );
                                    }
                                }
                            });
                        });
                    ui.add_space(6.0);
                }

                // File/folder list
                if *view_mode == FileManagerViewMode::Icon {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for entry in &filtered_entries {
                                let selected = selected_path.as_ref() == Some(&entry.path);
                                let fill = if selected {
                                    Color32::from_rgba_unmultiplied(0, 122, 255, 45)
                                } else {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 8)
                                };
                                let resp = egui::Frame::default()
                                    .fill(fill)
                                    .corner_radius(CornerRadius::same(8))
                                    .inner_margin(egui::Margin::symmetric(12, 10))
                                    .show(ui, |ui| {
                                        ui.set_min_size(Vec2::new(
                                            96.0 * *icon_scale,
                                            84.0 * *icon_scale,
                                        ));
                                        ui.vertical_centered(|ui| {
                                            ui.label(
                                                RichText::new(if entry.is_dir { "D" } else { "F" })
                                                    .size(24.0 * *icon_scale)
                                                    .color(if entry.is_dir {
                                                        accent
                                                    } else {
                                                        Color32::from_gray(220)
                                                    }),
                                            );
                                            ui.add_space(4.0);
                                            ui.label(
                                                RichText::new(&entry.name)
                                                    .size(11.0)
                                                    .color(Color32::from_gray(230)),
                                            );
                                        });
                                    })
                                    .response
                                    .interact(Sense::click());
                                if resp.clicked() {
                                    *selected_path = Some(entry.path.clone());
                                }
                                if resp.double_clicked() {
                                    navigate_to = Self::file_manager_open_entry_navigation(
                                        entry,
                                        ui.input(|i| i.modifiers.command || i.modifiers.ctrl),
                                    );
                                }
                            }
                        });
                    });
                } else if *view_mode == FileManagerViewMode::Gallery {
                    if !filtered_entries.is_empty() {
                        let current_index = AuroraDesktopApp::gallery_selection_index(
                            &filtered_entries,
                            selected_path.as_ref(),
                        );
                        if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                            *selected_path = Some(
                                filtered_entries[AuroraDesktopApp::gallery_next_index(
                                    filtered_entries.len(),
                                    current_index,
                                    -1,
                                )]
                                .path
                                .clone(),
                            );
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                            *selected_path = Some(
                                filtered_entries[AuroraDesktopApp::gallery_next_index(
                                    filtered_entries.len(),
                                    current_index,
                                    1,
                                )]
                                .path
                                .clone(),
                            );
                        }
                    }
                    let selected_gallery = selected_path
                        .clone()
                        .or_else(|| filtered_entries.first().map(|entry| entry.path.clone()));
                    if let Some(path) = selected_gallery {
                        let preview = build_preview(&path);
                        let info = read_file_info(&path);
                        egui::Frame::default()
                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(egui::Margin::symmetric(12, 10))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(preview.title)
                                        .size(16.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                );
                                ui.label(
                                    RichText::new(preview.subtitle)
                                        .size(10.0)
                                        .color(Color32::from_gray(140)),
                                );
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(format!("{} · {}", info.kind, info.size_label))
                                        .size(10.0)
                                        .color(Color32::from_gray(150)),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(preview.body)
                                        .size(11.0)
                                        .color(Color32::from_gray(220)),
                                );
                            });
                        ui.add_space(8.0);
                    }
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for entry in &filtered_entries {
                                let selected = selected_path.as_ref() == Some(&entry.path);
                                let fill = if selected { accent } else { btn_bg };
                                let resp = egui::Frame::default()
                                    .fill(fill)
                                    .corner_radius(CornerRadius::same(8))
                                    .inner_margin(egui::Margin::symmetric(10, 8))
                                    .show(ui, |ui| {
                                        ui.set_min_size(Vec2::new(90.0, 70.0));
                                        ui.vertical_centered(|ui| {
                                            ui.label(
                                                RichText::new(if entry.is_dir { "D" } else { "F" })
                                                    .size(18.0)
                                                    .color(Color32::WHITE),
                                            );
                                            ui.label(
                                                RichText::new(&entry.name)
                                                    .size(10.0)
                                                    .color(Color32::WHITE),
                                            );
                                        });
                                    })
                                    .response
                                    .interact(Sense::click());
                                if resp.clicked() {
                                    *selected_path = Some(entry.path.clone());
                                }
                                if resp.double_clicked() {
                                    navigate_to = Self::file_manager_open_entry_navigation(
                                        entry,
                                        ui.input(|i| i.modifiers.command || i.modifiers.ctrl),
                                    );
                                }
                            }
                        });
                    });
                } else if *view_mode == FileManagerViewMode::Column {
                    ui.label(
                        RichText::new("Column View")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_gray(150)),
                    );
                    ui.add_space(4.0);
                    ui.columns(2, |columns| {
                        egui::ScrollArea::vertical().show(&mut columns[0], |ui| {
                            for entry in &filtered_entries {
                                let resp = ui.selectable_label(
                                    selected_path.as_ref() == Some(&entry.path),
                                    &entry.name,
                                );
                                if resp.clicked() {
                                    *selected_path = Some(entry.path.clone());
                                }
                                if resp.double_clicked() {
                                    navigate_to = Self::file_manager_open_entry_navigation(
                                        entry,
                                        ui.input(|i| i.modifiers.command || i.modifiers.ctrl),
                                    );
                                }
                            }
                        });
                        egui::ScrollArea::vertical().show(&mut columns[1], |ui| {
                            if let Some(path) = selected_path.as_ref() {
                                if path.is_dir() {
                                    for child in read_directory(path) {
                                        ui.label(
                                            RichText::new(child.name)
                                                .size(11.0)
                                                .color(Color32::from_gray(220)),
                                        );
                                    }
                                } else {
                                    let preview = build_preview(path);
                                    ui.label(
                                        RichText::new(preview.title)
                                            .size(14.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    );
                                    ui.label(
                                        RichText::new(preview.body)
                                            .size(11.0)
                                            .color(Color32::from_gray(220)),
                                    );
                                }
                            } else {
                                ui.label(
                                    RichText::new(
                                        "Select a folder or file to inspect this column.",
                                    )
                                    .size(11.0)
                                    .color(Color32::from_gray(150)),
                                );
                            }
                        });
                    });
                } else {
                    ui.horizontal(|ui| {
                        for (label, field) in [
                            ("Name", FileManagerSortField::Name),
                            ("Kind", FileManagerSortField::Kind),
                            ("Size", FileManagerSortField::Size),
                        ] {
                            let color = if *sort_field == field {
                                accent
                            } else {
                                Color32::from_gray(180)
                            };
                            if ui
                                .button(RichText::new(label).size(10.0).color(color))
                                .clicked()
                            {
                                *sort_field = field;
                            }
                        }
                    });
                    ui.add_space(4.0);
                    let scroll_output = egui::ScrollArea::vertical().show(ui, |ui| {
                        for entry in &filtered_entries {
                            let selected = selected_path.as_ref() == Some(&entry.path);
                            let (icon, color) = if entry.is_dir {
                                ("D", Color32::from_rgb(0, 122, 255))
                            } else {
                                let ext = entry
                                    .path
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("");
                                match ext {
                                    "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" => {
                                        ("<>", Color32::from_rgb(88, 86, 214))
                                    }
                                    "md" | "txt" | "log" => ("T", Color32::from_rgb(142, 142, 147)),
                                    "png" | "jpg" | "jpeg" | "gif" | "svg" => {
                                        ("I", Color32::from_rgb(255, 149, 0))
                                    }
                                    "toml" | "json" | "yaml" | "yml" => {
                                        ("C", Color32::from_rgb(255, 214, 10))
                                    }
                                    _ => ("F", Color32::from_gray(160)),
                                }
                            };

                            let row_resp = ui.horizontal(|ui| {
                                if selected {
                                    let row = ui.max_rect();
                                    ui.painter().rect_filled(
                                        Rect::from_min_max(
                                            Pos2::new(row.left(), row.top()),
                                            Pos2::new(row.right(), row.top() + 24.0),
                                        ),
                                        CornerRadius::same(6),
                                        Color32::from_rgba_unmultiplied(0, 122, 255, 45),
                                    );
                                }
                                // Icon
                                let (ir, _) =
                                    ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                                if entry.is_dir {
                                    // Mini folder
                                    let body =
                                        Rect::from_center_size(ir.center(), Vec2::new(16.0, 12.0));
                                    ui.painter().rect_filled(body, CornerRadius::same(2), color);
                                    let tab = Rect::from_min_size(
                                        Pos2::new(body.left(), body.top() - 3.0),
                                        Vec2::new(7.0, 4.0),
                                    );
                                    ui.painter().rect_filled(tab, CornerRadius::same(1), color);
                                } else {
                                    // File icon
                                    ui.painter().text(
                                        ir.center(),
                                        Align2::CENTER_CENTER,
                                        icon,
                                        FontId::proportional(10.0),
                                        color,
                                    );
                                }

                                if let Some(tag) = file_tags.get(&entry.path).first().copied() {
                                    let tag_color = match tag {
                                        TagColor::Red => Color32::from_rgb(255, 59, 48),
                                        TagColor::Orange => Color32::from_rgb(255, 149, 0),
                                        TagColor::Yellow => Color32::from_rgb(255, 214, 10),
                                        TagColor::Green => Color32::from_rgb(52, 199, 89),
                                        TagColor::Blue => Color32::from_rgb(0, 122, 255),
                                        TagColor::Purple => Color32::from_rgb(175, 82, 222),
                                        TagColor::Gray => Color32::from_gray(142),
                                    };
                                    let (dot_rect, _) =
                                        ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                                    ui.painter()
                                        .circle_filled(dot_rect.center(), 4.0, tag_color);
                                }

                                ui.label(
                                    RichText::new(&entry.name)
                                        .size(12.0)
                                        .color(Color32::from_gray(230)),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if !entry.is_dir {
                                        ui.label(
                                            RichText::new(format_size(entry.size))
                                                .size(10.0)
                                                .color(Color32::from_gray(100)),
                                        );
                                    }
                                });
                            });
                            // Interact with the row rect for double-click and right-click
                            let resp = ui.interact(
                                row_resp.response.rect,
                                Id::new(("fm_entry", &entry.name)),
                                Sense::click_and_drag(),
                            );
                            let open_in_new_tab =
                                ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                            if resp.clicked() {
                                *selected_path = Some(entry.path.clone());
                            }
                            if resp.dragged() {
                                *selected_path = Some(entry.path.clone());
                                *dragged_path = Some(entry.path.clone());
                            }
                            if resp.double_clicked() {
                                navigate_to =
                                    Self::file_manager_open_entry_navigation(entry, open_in_new_tab);
                            }
                            if let Some(dragged) = dragged_path.clone() {
                                if entry.is_dir && resp.hovered() {
                                    ui.painter().rect_stroke(
                                        resp.rect.expand(2.0),
                                        CornerRadius::same(6),
                                        Stroke::new(
                                            1.0,
                                            Color32::from_rgba_unmultiplied(0, 122, 255, 160),
                                        ),
                                        StrokeKind::Outside,
                                    );
                                }
                                if entry.is_dir
                                    && resp.hovered()
                                    && !ui.input(|i| i.pointer.any_down())
                                {
                                    if let Some(target_path) =
                                        AuroraDesktopApp::directory_row_drop_target(
                                            &dragged,
                                            &entry.path,
                                        )
                                    {
                                        let copy_drag = AuroraDesktopApp::drag_uses_copy_modifier(
                                            ui.input(|i| i.modifiers),
                                        );
                                        content_drop_result = Some(if copy_drag {
                                            copy_entry_to_directory(&dragged, &target_path).map(
                                                |_| {
                                                    format!(
                                                        "Copied item to {}",
                                                        target_path.display()
                                                    )
                                                },
                                            )
                                        } else {
                                            move_entry_to_directory(&dragged, &target_path).map(
                                                |_| {
                                                    format!(
                                                        "Moved item to {}",
                                                        target_path.display()
                                                    )
                                                },
                                            )
                                        });
                                        *dragged_path = None;
                                        *selected_path = None;
                                    }
                                }
                            }

                            // Right-click context menu: Rename / Delete
                            resp.context_menu(|ui| {
                                if ui.button("Get Info").clicked() {
                                    *file_info_target = Some(entry.path.clone());
                                    ui.close();
                                }
                                ui.separator();
                                if ui.button("Rename").clicked() {
                                    *rename_target = Some(entry.path.clone());
                                    *rename_buffer = entry.name.clone();
                                    ui.close();
                                }
                                ui.separator();
                                for (label, color) in [
                                    ("Tag Red", TagColor::Red),
                                    ("Tag Orange", TagColor::Orange),
                                    ("Tag Yellow", TagColor::Yellow),
                                    ("Tag Green", TagColor::Green),
                                    ("Tag Blue", TagColor::Blue),
                                    ("Tag Purple", TagColor::Purple),
                                    ("Tag Gray", TagColor::Gray),
                                ] {
                                    if ui.button(label).clicked() {
                                        file_tags.assign(&entry.path, color);
                                        navigate_to =
                                            Some(PathBuf::from("__NOTIFY_OK__Tag updated"));
                                        ui.close();
                                    }
                                }
                                if ui.button("Clear Tags").clicked() {
                                    file_tags.clear(&entry.path);
                                    navigate_to = Some(PathBuf::from("__NOTIFY_OK__Tags cleared"));
                                    ui.close();
                                }
                                if in_trash {
                                    if ui.button("Restore").clicked() {
                                        match restore_trash_entry(&entry.name) {
                                            Ok(path) => {
                                                navigate_to = Some(PathBuf::from(format!(
                                                    "__NOTIFY_OK__Restored '{}'",
                                                    path.file_name()
                                                        .and_then(|n| n.to_str())
                                                        .unwrap_or("item")
                                                )));
                                            }
                                            Err(e) => {
                                                navigate_to = Some(PathBuf::from(format!(
                                                    "__NOTIFY_ERR__{e}"
                                                )));
                                            }
                                        }
                                        ui.close();
                                    }
                                } else if ui.button("Move to Trash").clicked() {
                                    match delete_entry(&entry.path) {
                                        Ok(()) => {
                                            navigate_to = Some(PathBuf::from(format!(
                                                "__NOTIFY_OK__Moved '{}' to Trash",
                                                entry.name
                                            )));
                                        }
                                        Err(e) => {
                                            navigate_to =
                                                Some(PathBuf::from(format!("__NOTIFY_ERR__{e}")));
                                        }
                                    }
                                    ui.close();
                                }
                            });
                        }
                    });
                    let content_rect = scroll_output.inner_rect;
                    if let Some(dragged) = dragged_path.clone() {
                        let hovered = ui.rect_contains_pointer(content_rect);
                        if hovered {
                            ui.painter().rect_stroke(
                                content_rect.shrink(2.0),
                                CornerRadius::same(6),
                                Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 122, 255, 160)),
                                StrokeKind::Outside,
                            );
                        }
                        if hovered && !ui.input(|i| i.pointer.any_down()) {
                            if let Some(target_path) =
                                AuroraDesktopApp::current_folder_drop_target(&dragged, current_dir)
                            {
                                let copy_drag = AuroraDesktopApp::drag_uses_copy_modifier(
                                    ui.input(|i| i.modifiers),
                                );
                                content_drop_result = Some(if copy_drag {
                                    copy_entry_to_directory(&dragged, &target_path).map(|_| {
                                        format!("Copied item to {}", target_path.display())
                                    })
                                } else {
                                    move_entry_to_directory(&dragged, &target_path)
                                        .map(|_| format!("Moved item to {}", target_path.display()))
                                });
                                *dragged_path = None;
                                *selected_path = None;
                            }
                        }
                    }
                }
            });

            if show_preview_pane {
                ui.add_space(8.0);
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(10, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(180.0);
                        ui.label(
                            RichText::new("Preview")
                                .size(10.0)
                                .strong()
                                .color(Color32::from_gray(140)),
                        );
                        ui.add_space(6.0);
                        if let Some(path) = selected_path.as_ref() {
                            let preview = build_preview(path);
                            let info = read_file_info(path);
                            ui.label(
                                RichText::new(&preview.title)
                                    .size(14.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            ui.label(
                                RichText::new(&preview.subtitle)
                                    .size(10.0)
                                    .color(Color32::from_gray(150)),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!("Kind: {}", info.kind))
                                    .size(11.0)
                                    .color(Color32::from_gray(210)),
                            );
                            ui.label(
                                RichText::new(format!("Size: {}", info.size_label))
                                    .size(11.0)
                                    .color(Color32::from_gray(210)),
                            );
                            ui.add_space(8.0);
                            egui::ScrollArea::vertical()
                                .max_height(220.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(preview.body)
                                            .size(11.0)
                                            .color(Color32::from_gray(220)),
                                    );
                                });
                        } else {
                            ui.label(
                                RichText::new("Select an item to preview it here.")
                                    .size(11.0)
                                    .color(Color32::from_gray(150)),
                            );
                        }
                    });
            }

            if show_status_bar {
                ui.add_space(8.0);
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(8, 5))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let available_gb = sysinfo::Disks::new_with_refreshed_list()
                                .list()
                                .iter()
                                .find(|disk| current_dir.starts_with(disk.mount_point()))
                                .map(|disk| {
                                    disk.available_space() as f64 / 1024.0 / 1024.0 / 1024.0
                                })
                                .unwrap_or(0.0);
                            ui.label(
                                RichText::new(format!(
                                    "{} items, {:.1} GB available",
                                    filtered_entries.len(),
                                    available_gb
                                ))
                                .size(10.0)
                                .color(Color32::from_gray(180)),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                for (label, mode) in [
                                    ("Gallery", FileManagerViewMode::Gallery),
                                    ("Column", FileManagerViewMode::Column),
                                    ("List", FileManagerViewMode::List),
                                    ("Icon", FileManagerViewMode::Icon),
                                ] {
                                    let active = *view_mode == mode;
                                    let fill = if active { accent } else { btn_bg };
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(label)
                                                    .size(10.0)
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(fill),
                                        )
                                        .clicked()
                                    {
                                        let token = match mode {
                                            FileManagerViewMode::Icon => "icon",
                                            FileManagerViewMode::List => "list",
                                            FileManagerViewMode::Column => "column",
                                            FileManagerViewMode::Gallery => "gallery",
                                        };
                                        navigate_to =
                                            Some(PathBuf::from(format!("__SET_VIEW__{token}")));
                                    }
                                }
                            });
                        });
                    });
            }
        });

        if let Some(result) = content_drop_result {
            navigate_to = Some(PathBuf::from(match result {
                Ok(message) => format!("__NOTIFY_OK__{message}"),
                Err(err) => format!("__NOTIFY_ERR__{err}"),
            }));
        }

        navigate_to
    }

    fn content_trash(ui: &mut egui::Ui) -> Option<PathBuf> {
        let mut navigate_to = None;
        let entries = load_trash_entries();

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Recently Deleted")
                    .size(11.0)
                    .color(Color32::from_gray(180)),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Empty Trash")
                                .size(11.0)
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(255, 59, 48))
                        .corner_radius(CornerRadius::same(4)),
                    )
                    .clicked()
                {
                    match empty_trash() {
                        Ok(()) => navigate_to = Some(PathBuf::from("__NOTIFY_OK__Trash emptied")),
                        Err(err) => {
                            navigate_to = Some(PathBuf::from(format!("__NOTIFY_ERR__{err}")))
                        }
                    }
                }
            });
        });
        ui.add_space(8.0);

        if entries.is_empty() {
            ui.label(
                RichText::new("Trash is empty")
                    .size(12.0)
                    .color(Color32::from_gray(140)),
            );
            return navigate_to;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for entry in entries {
                let deleted_at = chrono::DateTime::<Local>::from(
                    std::time::UNIX_EPOCH + Duration::from_secs(entry.deleted_at),
                );
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let icon = if entry.is_dir { "D" } else { "F" };
                            let color = if entry.is_dir {
                                Color32::from_rgb(0, 122, 255)
                            } else {
                                Color32::from_gray(210)
                            };
                            ui.label(RichText::new(icon).size(12.0).color(color));
                            ui.vertical(|ui| {
                                let original_name = entry
                                    .original_path
                                    .file_name()
                                    .and_then(|name| name.to_str())
                                    .unwrap_or("item");
                                ui.label(
                                    RichText::new(original_name)
                                        .size(12.0)
                                        .color(Color32::from_gray(235)),
                                );
                                ui.label(
                                    RichText::new(entry.original_path.to_string_lossy())
                                        .size(10.0)
                                        .color(Color32::from_gray(140)),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "Deleted {}",
                                        deleted_at.format("%Y-%m-%d %H:%M")
                                    ))
                                    .size(10.0)
                                    .color(Color32::from_gray(120)),
                                );
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.small_button("Delete").clicked() {
                                    match delete_trash_entry_permanently(&entry.trash_name) {
                                        Ok(()) => {
                                            navigate_to = Some(PathBuf::from(
                                                "__NOTIFY_OK__Item deleted permanently",
                                            ))
                                        }
                                        Err(err) => {
                                            navigate_to =
                                                Some(PathBuf::from(format!("__NOTIFY_ERR__{err}")))
                                        }
                                    }
                                }
                                if ui.small_button("Restore").clicked() {
                                    match restore_trash_entry(&entry.trash_name) {
                                        Ok(path) => {
                                            let name = path
                                                .file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("item");
                                            navigate_to = Some(PathBuf::from(format!(
                                                "__NOTIFY_OK__Restored '{name}'"
                                            )));
                                        }
                                        Err(err) => {
                                            navigate_to =
                                                Some(PathBuf::from(format!("__NOTIFY_ERR__{err}")))
                                        }
                                    }
                                }
                            });
                        });
                    });
                ui.add_space(4.0);
            }
        });

        navigate_to
    }

    fn content_controls(ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Quick Controls")
                .strong()
                .size(16.0)
                .color(Color32::from_gray(240)),
        );
        ui.add_space(8.0);
        egui::Grid::new("controls_grid")
            .num_columns(2)
            .spacing(Vec2::new(10.0, 10.0))
            .show(ui, |ui| {
                for label in ["Wi-Fi", "Bluetooth", "Focus", "Display"] {
                    let _ = ui.add(
                        egui::Button::new(
                            RichText::new(label).strong().color(Color32::from_gray(240)),
                        )
                        .min_size(Vec2::new(130.0, 60.0))
                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 28))
                        .stroke(Stroke::new(1.0, Color32::from_white_alpha(70)))
                        .corner_radius(CornerRadius::same(12)),
                    );
                    if label == "Bluetooth" || label == "Display" {
                        ui.end_row();
                    }
                }
            });
    }

    fn content_messages(&mut self, ui: &mut egui::Ui) {
        self.messages_state.tick();
        // Snapshot conversation list for sidebar
        let conv_data: Vec<(String, usize, usize)> = self
            .messages_state
            .conversations
            .iter()
            .enumerate()
            .map(|(i, c)| (c.contact_name.clone(), c.unread, i))
            .collect();
        let active_idx = self.messages_state.active;

        ui.horizontal(|ui| {
            // Sidebar: conversation list
            egui::Frame::default()
                .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(8, 8))
                .show(ui, |ui| {
                    ui.set_min_width(120.0);
                    ui.label(
                        RichText::new("Messages")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_gray(160)),
                    );
                    ui.add_space(6.0);
                    for (name, unread, idx) in &conv_data {
                        let selected = *idx == active_idx;
                        let bg = if selected {
                            Color32::from_rgba_unmultiplied(0, 122, 255, 80)
                        } else {
                            Color32::TRANSPARENT
                        };
                        let contact_color = self
                            .messages_state
                            .conversations
                            .get(*idx)
                            .map(|conv| conv.contact_color)
                            .unwrap_or(Color32::from_gray(160));
                        let resp = egui::Frame::default()
                            .fill(bg)
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(egui::Margin::symmetric(6, 4))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (dot_rect, _) =
                                        ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                                    ui.painter().circle_filled(
                                        dot_rect.center(),
                                        4.0,
                                        contact_color,
                                    );
                                    ui.label(
                                        RichText::new(name)
                                            .size(12.0)
                                            .strong()
                                            .color(Color32::from_gray(230)),
                                    );
                                    if *unread > 0 {
                                        ui.label(
                                            RichText::new(format!("({})", unread))
                                                .size(10.0)
                                                .color(Color32::from_rgb(0, 122, 255)),
                                        );
                                    }
                                });
                                // Show last message preview
                                if let Some(conv) = self.messages_state.conversations.get(*idx) {
                                    if let Some(last) = conv.messages.last() {
                                        let preview: String = last.text.chars().take(25).collect();
                                        ui.label(
                                            RichText::new(preview)
                                                .size(10.0)
                                                .color(Color32::from_gray(150)),
                                        );
                                    }
                                }
                            })
                            .response;
                        if resp.interact(Sense::click()).clicked() {
                            self.messages_state.switch_conversation(*idx);
                        }
                        ui.add_space(2.0);
                    }
                });

            ui.add_space(8.0);

            // Chat area
            ui.vertical(|ui| {
                // Messages scroll
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if let Some(conv) = self.messages_state.active_conversation() {
                            for msg in &conv.messages {
                                let (align, color, tc) = if msg.is_sent {
                                    (Align::RIGHT, Color32::from_rgb(0, 122, 255), Color32::WHITE)
                                } else {
                                    (
                                        Align::LEFT,
                                        Color32::from_rgba_unmultiplied(255, 255, 255, 40),
                                        Color32::from_gray(230),
                                    )
                                };
                                ui.with_layout(Layout::top_down(align), |ui| {
                                    egui::Frame::default()
                                        .fill(color)
                                        .corner_radius(CornerRadius::same(14))
                                        .inner_margin(egui::Margin::symmetric(12, 6))
                                        .show(ui, |ui| {
                                            ui.set_max_width(220.0);
                                            ui.label(RichText::new(&msg.text).size(12.0).color(tc));
                                            let age = Instant::now()
                                                .saturating_duration_since(msg.timestamp);
                                            let age_label = if age.as_secs() < 60 {
                                                "now".to_string()
                                            } else {
                                                format!("{}m ago", age.as_secs() / 60)
                                            };
                                            ui.label(
                                                RichText::new(age_label)
                                                    .size(9.0)
                                                    .color(Color32::from_gray(180)),
                                            );
                                        });
                                });
                                ui.add_space(4.0);
                            }
                        }
                    });

                // Input field
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("😀").on_hover_text("Emoji & Symbols").clicked() {
                        self.show_emoji_picker = true;
                        self.emoji_query.clear();
                    }
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.messages_state.input_text)
                            .desired_width(ui.available_width() - 96.0)
                            .hint_text("Type a message..."),
                    );
                    if ui.button("Send").clicked()
                        || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        self.messages_state.send_message();
                    }
                });
            });
        });
    }

    fn content_browser(&mut self, ui: &mut egui::Ui) {
        // URL bar with back/forward
        egui::Frame::default()
            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 15))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(10, 5))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let back_color = if self.browser_state.can_go_back() {
                        Color32::from_gray(200)
                    } else {
                        Color32::from_gray(80)
                    };
                    let fwd_color = if self.browser_state.can_go_forward() {
                        Color32::from_gray(200)
                    } else {
                        Color32::from_gray(80)
                    };
                    if ui
                        .add(
                            egui::Label::new(RichText::new("<").size(14.0).color(back_color))
                                .sense(Sense::click()),
                        )
                        .clicked()
                    {
                        self.browser_state.go_back();
                        self.browser_url_input = self.browser_state.url.clone();
                    }
                    if ui
                        .add(
                            egui::Label::new(RichText::new(">").size(14.0).color(fwd_color))
                                .sense(Sense::click()),
                        )
                        .clicked()
                    {
                        self.browser_state.go_forward();
                        self.browser_url_input = self.browser_state.url.clone();
                    }
                    ui.add_space(8.0);
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.browser_url_input)
                            .desired_width(ui.available_width() - 64.0)
                            .font(FontId::proportional(12.0)),
                    );
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.browser_state.navigate(&self.browser_url_input);
                        self.browser_url_input = self.browser_state.url.clone();
                    }
                    if matches!(
                        self.browser_state.page,
                        browser::BrowserPage::MockSite { .. }
                    ) {
                        if ui.small_button("PiP").clicked() {
                            let title = match &self.browser_state.page {
                                browser::BrowserPage::MockSite { title, .. } => title.clone(),
                                _ => "Browser".to_string(),
                            };
                            self.open_pip(
                                PipSource::Browser {
                                    title,
                                    url: self.browser_state.url.clone(),
                                },
                                Self::desktop_work_rect(ui.ctx()),
                            );
                        }
                    }
                });
            });

        ui.add_space(10.0);

        // Render page content
        match &self.browser_state.page {
            browser::BrowserPage::Favorites => {
                ui.label(
                    RichText::new("Favorites")
                        .size(16.0)
                        .strong()
                        .color(Color32::from_gray(220)),
                );
                ui.add_space(8.0);
                let bookmarks: Vec<(String, String, Color32, String)> = self
                    .browser_state
                    .bookmarks
                    .iter()
                    .map(|b| (b.name.clone(), b.url.clone(), b.color, b.abbrev.clone()))
                    .collect();
                egui::Grid::new("browser_favorites")
                    .num_columns(4)
                    .spacing(Vec2::new(16.0, 14.0))
                    .show(ui, |ui| {
                        for (i, (name, url, color, abbrev)) in bookmarks.iter().enumerate() {
                            ui.vertical(|ui| {
                                ui.set_min_width(70.0);
                                let (ir, icon_resp) =
                                    ui.allocate_exact_size(Vec2::splat(44.0), Sense::click());
                                ui.painter().rect_filled(ir, CornerRadius::same(10), *color);
                                ui.painter().text(
                                    ir.center(),
                                    Align2::CENTER_CENTER,
                                    abbrev,
                                    FontId::proportional(16.0),
                                    Color32::WHITE,
                                );
                                let resp = ui.add(
                                    egui::Label::new(
                                        RichText::new(name)
                                            .size(10.0)
                                            .color(Color32::from_gray(180)),
                                    )
                                    .sense(Sense::click()),
                                );
                                if icon_resp.clicked() || resp.clicked() {
                                    self.browser_state.navigate(url);
                                    self.browser_url_input = self.browser_state.url.clone();
                                }
                            });
                            if (i + 1) % 4 == 0 {
                                ui.end_row();
                            }
                        }
                    });
            }
            browser::BrowserPage::MockSite { title, sections } => {
                let title = title.clone();
                let sections: Vec<_> = sections.clone();
                ui.label(
                    RichText::new(&title)
                        .size(18.0)
                        .strong()
                        .color(Color32::from_gray(230)),
                );
                ui.add_space(8.0);
                for section in &sections {
                    match &section.kind {
                        browser::SectionKind::Heading => {
                            ui.label(
                                RichText::new(&section.text)
                                    .size(15.0)
                                    .strong()
                                    .color(Color32::from_gray(220)),
                            );
                            ui.add_space(4.0);
                        }
                        browser::SectionKind::Paragraph => {
                            ui.label(
                                RichText::new(&section.text)
                                    .size(12.0)
                                    .color(Color32::from_gray(170)),
                            );
                            ui.add_space(4.0);
                        }
                        browser::SectionKind::Image { color, height } => {
                            let (r, _) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), *height),
                                Sense::hover(),
                            );
                            ui.painter().rect_filled(r, CornerRadius::same(6), *color);
                            ui.painter().text(
                                r.center(),
                                Align2::CENTER_CENTER,
                                &section.text,
                                FontId::proportional(14.0),
                                Color32::WHITE,
                            );
                            ui.add_space(4.0);
                        }
                        browser::SectionKind::Link { url } => {
                            let url = url.clone();
                            let resp = ui.add(
                                egui::Label::new(
                                    RichText::new(&section.text)
                                        .size(12.0)
                                        .color(Color32::from_rgb(0, 122, 255))
                                        .underline(),
                                )
                                .sense(Sense::click()),
                            );
                            if resp.clicked() && url != "#" {
                                self.browser_state.navigate(&url);
                                self.browser_url_input = self.browser_state.url.clone();
                            }
                            ui.add_space(2.0);
                        }
                        browser::SectionKind::SearchBar => {
                            // Decorative search bar
                            egui::Frame::default()
                                .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 20))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(egui::Margin::symmetric(8, 4))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(if section.text.is_empty() {
                                            "Search..."
                                        } else {
                                            &section.text
                                        })
                                        .size(12.0)
                                        .color(Color32::from_gray(140)),
                                    );
                                });
                            ui.add_space(4.0);
                        }
                        browser::SectionKind::CodeBlock => {
                            egui::Frame::default()
                                .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 60))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(egui::Margin::symmetric(10, 8))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(&section.text)
                                            .size(11.0)
                                            .color(Color32::from_gray(180))
                                            .family(egui::FontFamily::Monospace),
                                    );
                                });
                            ui.add_space(4.0);
                        }
                    }
                }
            }
            browser::BrowserPage::NotFound { url } => {
                let url = url.clone();
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("Page Not Found")
                            .size(20.0)
                            .strong()
                            .color(Color32::from_gray(200)),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Cannot connect to {}", url))
                            .size(12.0)
                            .color(Color32::from_gray(140)),
                    );
                });
            }
        }
    }

    fn content_calculator(
        ui: &mut egui::Ui,
        display: &mut String,
        operand: &mut Option<f64>,
        operator: &mut Option<char>,
        reset_next: &mut bool,
        mode: &mut CalculatorMode,
        history: &mut Vec<String>,
        memory: &mut f64,
        degrees: &mut bool,
        programmer_base: &mut ProgrammerBase,
        programmer_operand: &mut Option<i64>,
        programmer_operator: &mut Option<String>,
    ) {
        fn push_history_entry(history: &mut Vec<String>, entry: String) {
            if !entry.is_empty() {
                history.push(entry);
                if history.len() > 16 {
                    history.remove(0);
                }
            }
        }

        let operator_fill = Color32::from_rgb(255, 149, 0);
        let utility_fill = Color32::from_rgb(165, 165, 165);
        let digit_fill = Color32::from_rgb(80, 80, 80);

        ui.horizontal(|ui| {
            for candidate in [
                CalculatorMode::Basic,
                CalculatorMode::Scientific,
                CalculatorMode::Programmer,
            ] {
                let active = *mode == candidate;
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(match candidate {
                                CalculatorMode::Basic => "Basic",
                                CalculatorMode::Scientific => "Scientific",
                                CalculatorMode::Programmer => "Programmer",
                            })
                            .size(11.0)
                            .color(Color32::WHITE),
                        )
                        .fill(if active {
                            Color32::from_rgb(10, 132, 255)
                        } else {
                            Color32::from_gray(55)
                        }),
                    )
                    .clicked()
                {
                    *mode = candidate;
                    *reset_next = true;
                }
            }
        });
        ui.add_space(6.0);

        egui::Frame::default()
            .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 60))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(match mode {
                                CalculatorMode::Basic => "Basic arithmetic",
                                CalculatorMode::Scientific => {
                                    if *degrees {
                                        "Scientific / Deg"
                                    } else {
                                        "Scientific / Rad"
                                    }
                                }
                                CalculatorMode::Programmer => "Programmer",
                            })
                            .size(10.0)
                            .color(Color32::from_gray(160)),
                        );
                        if let Some(last) = history.last() {
                            ui.label(
                                RichText::new(last)
                                    .size(10.0)
                                    .color(Color32::from_gray(120)),
                            );
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(display.as_str())
                                .size(28.0)
                                .strong()
                                .color(Color32::WHITE)
                                .family(egui::FontFamily::Monospace),
                        );
                    });
                });
            });
        ui.add_space(8.0);

        if *mode == CalculatorMode::Programmer {
            ui.horizontal(|ui| {
                for base in [
                    ProgrammerBase::Dec,
                    ProgrammerBase::Hex,
                    ProgrammerBase::Oct,
                    ProgrammerBase::Bin,
                ] {
                    let active = *programmer_base == base;
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(base.label()).size(10.0).color(Color32::WHITE),
                            )
                            .fill(if active {
                                Color32::from_rgb(52, 120, 246)
                            } else {
                                Color32::from_gray(60)
                            }),
                        )
                        .clicked()
                    {
                        if let Some(value) = parse_programmer_value(display, *programmer_base) {
                            *display = format_programmer_value(value, base);
                        }
                        *programmer_base = base;
                        *reset_next = true;
                    }
                }
            });
            ui.add_space(6.0);
            if let Some(value) = parse_programmer_value(display, *programmer_base) {
                for (label, representation) in programmer_representations(value) {
                    ui.label(
                        RichText::new(format!("{label}: {representation}"))
                            .size(10.0)
                            .family(egui::FontFamily::Monospace)
                            .color(Color32::from_gray(180)),
                    );
                }
                ui.label(
                    RichText::new(format!("ASCII: {}", programmer_ascii(value).unwrap_or('.')))
                        .size(10.0)
                        .color(Color32::from_gray(180)),
                );
                ui.add_space(4.0);
            }
        }

        if *mode == CalculatorMode::Scientific {
            let scientific_rows: &[&[&str]] = &[
                &["MC", "MR", "M+", "M-", "Deg/Rad"],
                &["sin", "cos", "tan", "ln", "log10"],
                &["asin", "acos", "atan", "x2", "x3"],
                &["sqrt", "cbrt", "1/x", "n!", "pi"],
            ];
            for row in scientific_rows {
                ui.horizontal(|ui| {
                    for label in *row {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(*label).size(12.0).color(Color32::BLACK),
                                )
                                .min_size(Vec2::new(54.0, 32.0))
                                .fill(utility_fill),
                            )
                            .clicked()
                        {
                            match *label {
                                "MC" => *memory = 0.0,
                                "MR" => {
                                    *display = format_calc(*memory);
                                    *reset_next = true;
                                }
                                "M+" => {
                                    if let Ok(value) = display.parse::<f64>() {
                                        *memory += value;
                                    }
                                }
                                "M-" => {
                                    if let Ok(value) = display.parse::<f64>() {
                                        *memory -= value;
                                    }
                                }
                                "Deg/Rad" => *degrees = !*degrees,
                                "pi" => {
                                    *display = format_calc(std::f64::consts::PI);
                                    *reset_next = true;
                                }
                                action => {
                                    if let Ok(value) = display.parse::<f64>() {
                                        if let Some(result) =
                                            scientific_eval(action, value, *degrees)
                                        {
                                            let expression = format!(
                                                "{action}({}) = {}",
                                                display,
                                                format_calc(result)
                                            );
                                            *display = format_calc(result);
                                            push_history_entry(history, expression);
                                            *reset_next = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                });
                ui.add_space(4.0);
            }
        }

        let btn_size = Vec2::new(52.0, 42.0);
        let rows: &[&[(&str, Color32)]] = match mode {
            CalculatorMode::Programmer => &[
                &[
                    ("C", utility_fill),
                    ("NOT", utility_fill),
                    ("AND", utility_fill),
                    ("OR", utility_fill),
                    ("XOR", utility_fill),
                ],
                &[
                    ("A", digit_fill),
                    ("B", digit_fill),
                    ("C", digit_fill),
                    ("D", digit_fill),
                    ("<<", operator_fill),
                ],
                &[
                    ("E", digit_fill),
                    ("F", digit_fill),
                    ("7", digit_fill),
                    ("8", digit_fill),
                    (">>", operator_fill),
                ],
                &[
                    ("9", digit_fill),
                    ("4", digit_fill),
                    ("5", digit_fill),
                    ("6", digit_fill),
                    ("=", operator_fill),
                ],
                &[
                    ("1", digit_fill),
                    ("2", digit_fill),
                    ("3", digit_fill),
                    ("0", digit_fill),
                    ("+/-", utility_fill),
                ],
            ],
            _ => &[
                &[
                    ("C", utility_fill),
                    ("+/-", utility_fill),
                    ("%", utility_fill),
                    ("/", operator_fill),
                ],
                &[
                    ("7", digit_fill),
                    ("8", digit_fill),
                    ("9", digit_fill),
                    ("*", operator_fill),
                ],
                &[
                    ("4", digit_fill),
                    ("5", digit_fill),
                    ("6", digit_fill),
                    ("-", operator_fill),
                ],
                &[
                    ("1", digit_fill),
                    ("2", digit_fill),
                    ("3", digit_fill),
                    ("+", operator_fill),
                ],
                &[("0", digit_fill), (".", digit_fill), ("=", operator_fill)],
            ],
        };

        ui.columns(2, |columns| {
            columns[0].vertical(|ui| {
                for row in rows {
                    ui.horizontal(|ui| {
                        for (label, color) in *row {
                            let w = if *label == "0" && *mode != CalculatorMode::Programmer {
                                btn_size.x * 2.0 + 4.0
                            } else {
                                btn_size.x
                            };
                            let text_color = if *color == operator_fill {
                                Color32::WHITE
                            } else if *color == utility_fill {
                                Color32::BLACK
                            } else {
                                Color32::WHITE
                            };
                            let btn = ui.add(
                                egui::Button::new(
                                    RichText::new(*label).size(15.0).strong().color(text_color),
                                )
                                .min_size(Vec2::new(w, btn_size.y))
                                .fill(*color)
                                .corner_radius(CornerRadius::same(18)),
                            );
                            if btn.clicked() {
                                match *mode {
                                    CalculatorMode::Programmer => match *label {
                                        "C" if *color == utility_fill => {
                                            *display = "0".to_string();
                                            *programmer_operand = None;
                                            *programmer_operator = None;
                                            *reset_next = false;
                                        }
                                        "NOT" => {
                                            if let Some(value) =
                                                parse_programmer_value(display, *programmer_base)
                                            {
                                                let result = programmer_not(value);
                                                push_history_entry(
                                                    history,
                                                    format!(
                                                        "NOT {} = {}",
                                                        display,
                                                        format_programmer_value(
                                                            result,
                                                            *programmer_base
                                                        )
                                                    ),
                                                );
                                                *display = format_programmer_value(
                                                    result,
                                                    *programmer_base,
                                                );
                                                *reset_next = true;
                                            }
                                        }
                                        "AND" | "OR" | "XOR" | "<<" | ">>" => {
                                            if let Some(value) =
                                                parse_programmer_value(display, *programmer_base)
                                            {
                                                *programmer_operand = Some(value);
                                                *programmer_operator = Some((*label).to_string());
                                                *reset_next = true;
                                            }
                                        }
                                        "=" => {
                                            if let (Some(left), Some(op), Some(right)) = (
                                                *programmer_operand,
                                                programmer_operator.clone(),
                                                parse_programmer_value(display, *programmer_base),
                                            ) {
                                                if let Some(result) =
                                                    programmer_eval(left, &op, right)
                                                {
                                                    push_history_entry(
                                                        history,
                                                        format!(
                                                            "{} {} {} = {}",
                                                            format_programmer_value(
                                                                left,
                                                                *programmer_base
                                                            ),
                                                            op,
                                                            format_programmer_value(
                                                                right,
                                                                *programmer_base
                                                            ),
                                                            format_programmer_value(
                                                                result,
                                                                *programmer_base
                                                            )
                                                        ),
                                                    );
                                                    *display = format_programmer_value(
                                                        result,
                                                        *programmer_base,
                                                    );
                                                    *programmer_operand = None;
                                                    *programmer_operator = None;
                                                    *reset_next = true;
                                                }
                                            }
                                        }
                                        "+/-" => {
                                            if let Some(value) =
                                                parse_programmer_value(display, *programmer_base)
                                            {
                                                *display = format_programmer_value(
                                                    -value,
                                                    *programmer_base,
                                                );
                                            }
                                        }
                                        digit => {
                                            let allowed = match *programmer_base {
                                                ProgrammerBase::Dec => "0123456789",
                                                ProgrammerBase::Hex => "0123456789ABCDEF",
                                                ProgrammerBase::Oct => "01234567",
                                                ProgrammerBase::Bin => "01",
                                            };
                                            if allowed.contains(digit) {
                                                if *reset_next || *display == "0" {
                                                    *display = digit.to_string();
                                                    *reset_next = false;
                                                } else {
                                                    display.push_str(digit);
                                                }
                                            }
                                        }
                                    },
                                    _ => match *label {
                                        "C" => {
                                            *display = "0".to_string();
                                            *operand = None;
                                            *operator = None;
                                            *reset_next = false;
                                        }
                                        "+/-" => {
                                            if let Ok(val) = display.parse::<f64>() {
                                                *display = format_calc(-val);
                                            }
                                        }
                                        "%" => {
                                            if let Ok(val) = display.parse::<f64>() {
                                                *display = format_calc(val / 100.0);
                                            }
                                        }
                                        "+" | "-" | "*" | "/" => {
                                            if let Ok(val) = display.parse::<f64>() {
                                                if let (Some(prev), Some(op)) =
                                                    (*operand, *operator)
                                                {
                                                    let result = calc_eval(prev, op, val);
                                                    *display = format_calc(result);
                                                    *operand = Some(result);
                                                } else {
                                                    *operand = Some(val);
                                                }
                                            }
                                            *operator = Some(label.chars().next().unwrap());
                                            *reset_next = true;
                                        }
                                        "=" => {
                                            if let (Ok(val), Some(prev), Some(op)) =
                                                (display.parse::<f64>(), *operand, *operator)
                                            {
                                                let result = calc_eval(prev, op, val);
                                                push_history_entry(
                                                    history,
                                                    format!(
                                                        "{} {} {} = {}",
                                                        format_calc(prev),
                                                        op,
                                                        format_calc(val),
                                                        format_calc(result)
                                                    ),
                                                );
                                                *display = format_calc(result);
                                                *operand = None;
                                                *operator = None;
                                                *reset_next = true;
                                            }
                                        }
                                        "." => {
                                            if *reset_next {
                                                *display = "0.".to_string();
                                                *reset_next = false;
                                            } else if !display.contains('.') {
                                                display.push('.');
                                            }
                                        }
                                        digit => {
                                            if *reset_next || *display == "0" {
                                                *display = digit.to_string();
                                                *reset_next = false;
                                            } else {
                                                display.push_str(digit);
                                            }
                                        }
                                    },
                                }
                            }
                        }
                    });
                    ui.add_space(4.0);
                }
            });

            columns[1].vertical(|ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(20)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("History Tape")
                                    .size(11.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            if ui.small_button("Clear").clicked() {
                                history.clear();
                            }
                        });
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical()
                            .max_height(250.0)
                            .show(ui, |ui| {
                                for entry in history.iter().rev() {
                                    if ui.small_button(entry).clicked() {
                                        if let Some(result) = entry.split(" = ").last() {
                                            *display = result.to_string();
                                            *reset_next = true;
                                        }
                                    }
                                }
                            });
                    });
            });
        });
    }

    fn content_notes(ui: &mut egui::Ui, text: &mut String) -> bool {
        let mut open_emoji_picker = false;
        ui.horizontal(|ui| {
            let toolbar_items = ["Bold", "Italic", "List", "---", "Font", "Emoji"];
            for item in toolbar_items {
                if item == "---" {
                    let (sr, _) = ui.allocate_exact_size(Vec2::new(1.0, 18.0), Sense::hover());
                    ui.painter()
                        .rect_filled(sr, 0.0, Color32::from_white_alpha(30));
                } else if item == "Emoji" {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("😀")
                                    .size(11.0)
                                    .color(Color32::from_gray(180)),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                        )
                        .clicked()
                    {
                        open_emoji_picker = true;
                    }
                } else {
                    ui.add(
                        egui::Button::new(
                            RichText::new(item)
                                .size(11.0)
                                .color(Color32::from_gray(180)),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE),
                    );
                }
            }
        });
        ui.add_space(4.0);
        let (sep_rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
        ui.painter()
            .rect_filled(sep_rect, 0.0, Color32::from_white_alpha(20));
        ui.add_space(4.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(text)
                    .font(FontId::proportional(13.0))
                    .text_color(Color32::from_gray(220))
                    .desired_width(ui.available_width())
                    .desired_rows(16)
                    .frame(false),
            );
        });
        open_emoji_picker
    }

    fn content_music(&mut self, ui: &mut egui::Ui) {
        let tracks = [
            (
                "Aurora Ambient",
                "System Sounds",
                Color32::from_rgb(255, 107, 157),
            ),
            ("Neon Waves", "Synthwave FM", Color32::from_rgb(88, 86, 214)),
            (
                "Mountain Breeze",
                "Nature Sounds",
                Color32::from_rgb(52, 199, 89),
            ),
            ("Deep Focus", "Lo-Fi Beats", Color32::from_rgb(255, 149, 0)),
            ("Night Drive", "Electronic", Color32::from_rgb(0, 122, 255)),
        ];
        let real_tracks = Self::music_library_paths(&dirs_home());
        let current_track_path = self.current_music_path(&real_tracks);
        let using_real_tracks = !real_tracks.is_empty();
        let filtered_indices = if using_real_tracks {
            Self::music_queue_indices(&real_tracks, &self.music_library_query, self.music_shuffle)
        } else {
            Vec::new()
        };
        let (name, artist, color) = if let Some(path) = current_track_path.as_deref() {
            (
                Self::music_track_title(path),
                "Local Music Library",
                Self::music_track_color_for_path(path),
            )
        } else {
            let (name, artist, color) = Self::music_track_info(self.music_track_idx);
            (name.to_string(), artist, color)
        };
        let duration_seconds = Self::music_track_duration_seconds(
            self.music_track_idx,
            current_track_path.as_deref(),
        );
        let progress = if duration_seconds > 0.0 {
            (self.music_elapsed_seconds / duration_seconds).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Album art
        let art_size = Vec2::splat(80.0);
        ui.horizontal(|ui| {
            let (art_rect, _) = ui.allocate_exact_size(art_size, Sense::hover());
            gradient_rect(ui.painter(), art_rect, color, Color32::from_rgb(30, 30, 50));
            ui.painter().rect_stroke(
                art_rect,
                CornerRadius::same(8),
                Stroke::new(0.5, Color32::from_white_alpha(30)),
                StrokeKind::Outside,
            );
            // Music note symbol
            ui.painter().text(
                art_rect.center(),
                Align2::CENTER_CENTER,
                "♪",
                FontId::proportional(30.0),
                Color32::from_white_alpha(180),
            );

            ui.vertical(|ui| {
                ui.add_space(12.0);
                ui.label(
                    RichText::new(&name)
                        .size(16.0)
                        .strong()
                        .color(Color32::WHITE),
                );
                ui.label(
                    RichText::new(artist)
                        .size(12.0)
                        .color(Color32::from_gray(150)),
                );
                if let Some(path) = current_track_path.as_deref() {
                    if let Some(metadata) = Self::music_track_metadata_label(path) {
                        ui.label(
                            RichText::new(metadata)
                                .size(10.0)
                                .color(Color32::from_gray(120)),
                        );
                    }
                }
            });
        });

        ui.add_space(12.0);

        // Progress bar
        let (bar_rect, bar_resp) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), 4.0),
            Sense::click_and_drag(),
        );
        ui.painter().rect_filled(
            bar_rect,
            CornerRadius::same(2),
            Color32::from_rgba_unmultiplied(255, 255, 255, 30),
        );
        let filled = Rect::from_min_size(bar_rect.min, Vec2::new(bar_rect.width() * progress, 4.0));
        ui.painter()
            .rect_filled(filled, CornerRadius::same(2), color);
        if bar_resp.clicked() || bar_resp.dragged() {
            if let Some(pointer) = bar_resp.interact_pointer_pos() {
                let fraction = Self::music_seek_fraction(bar_rect, pointer.x);
                self.music_elapsed_seconds = Self::music_seek_seconds(duration_seconds, fraction);
                self.music_last_tick = Instant::now();
                self.persist_music_state();
                self.sync_music_audio(true);
            }
        }
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(Self::format_music_time(self.music_elapsed_seconds))
                    .size(10.0)
                    .color(Color32::from_gray(120)),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(Self::format_music_time(duration_seconds))
                        .size(10.0)
                        .color(Color32::from_gray(120)),
                );
            });
        });

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.selectable_label(self.music_shuffle, "Shuffle").clicked() {
                self.music_shuffle = !self.music_shuffle;
                self.persist_music_state();
            }
            if ui
                .selectable_label(self.music_repeat_all, "Repeat")
                .clicked()
            {
                self.music_repeat_all = !self.music_repeat_all;
                self.persist_music_state();
            }
        });

        ui.add_space(8.0);

        // Controls
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - 212.0) / 2.0);
            // Previous
            if ui
                .add(
                    egui::Button::new(RichText::new("<<").size(16.0).color(Color32::WHITE))
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::splat(36.0)),
                )
                .clicked()
            {
                let next_idx = if using_real_tracks {
                    Self::step_music_track_idx_with_repeat(
                        self.music_track_idx,
                        &filtered_indices,
                        -1,
                        self.music_repeat_all,
                    )
                    .unwrap_or(self.music_track_idx)
                } else {
                    if self.music_repeat_all {
                        self.music_track_idx
                            .checked_sub(1)
                            .unwrap_or(tracks.len() - 1)
                    } else {
                        self.music_track_idx.saturating_sub(1)
                    }
                };
                self.music_track_idx = next_idx;
                self.reset_music_progress();
            }
            // Play/Pause
            let play_label = if self.music_playing { "| |" } else { " > " };
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(play_label)
                            .size(18.0)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(color)
                    .corner_radius(CornerRadius::same(20))
                    .min_size(Vec2::splat(44.0)),
                )
                .clicked()
            {
                self.music_playing = !self.music_playing;
                self.music_last_tick = Instant::now();
                self.persist_music_state();
            }
            // Next
            if ui
                .add(
                    egui::Button::new(RichText::new(">>").size(16.0).color(Color32::WHITE))
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::splat(36.0)),
                )
                .clicked()
            {
                let next_idx = if using_real_tracks {
                    Self::step_music_track_idx_with_repeat(
                        self.music_track_idx,
                        &filtered_indices,
                        1,
                        self.music_repeat_all,
                    )
                    .unwrap_or(self.music_track_idx)
                } else {
                    if self.music_repeat_all {
                        (self.music_track_idx + 1) % tracks.len()
                    } else {
                        (self.music_track_idx + 1).min(tracks.len() - 1)
                    }
                };
                self.music_track_idx = next_idx;
                self.reset_music_progress();
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("PiP").size(11.0).color(Color32::WHITE))
                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 20))
                        .corner_radius(CornerRadius::same(10))
                        .min_size(Vec2::new(42.0, 28.0)),
                )
                .clicked()
            {
                self.open_pip(PipSource::Music, Self::desktop_work_rect(ui.ctx()));
            }
        });

        ui.add_space(12.0);
        ui.label(
            RichText::new(if using_real_tracks {
                "Local Library"
            } else {
                "Up Next"
            })
            .size(12.0)
            .strong()
            .color(Color32::from_gray(160)),
        );
        ui.add_space(4.0);
        if using_real_tracks {
            let search_resp = ui.add(
                egui::TextEdit::singleline(&mut self.music_library_query)
                    .hint_text("Search local library")
                    .desired_width(f32::INFINITY),
            );
            if search_resp.changed() {
                self.persist_music_state();
            }
            ui.add_space(6.0);
        }

        // Track list
        egui::ScrollArea::vertical()
            .max_height(120.0)
            .show(ui, |ui| {
                if using_real_tracks {
                    if filtered_indices.is_empty() {
                        ui.label(
                            RichText::new("No local tracks match this search.")
                                .size(11.0)
                                .color(Color32::from_gray(130)),
                        );
                    }
                    for i in filtered_indices {
                        let path = &real_tracks[i];
                        let is_current = current_track_path.as_deref() == Some(path.as_path());
                        let track_color = Self::music_track_color_for_path(path);
                        let bg = if is_current {
                            Color32::from_rgba_unmultiplied(255, 255, 255, 15)
                        } else {
                            Color32::TRANSPARENT
                        };
                        let resp = egui::Frame::default()
                            .fill(bg)
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(egui::Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (dot_r, _) =
                                        ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                                    ui.painter().circle_filled(dot_r.center(), 4.0, track_color);
                                    if is_current && self.music_playing {
                                        ui.label(
                                            RichText::new("♪").size(11.0).color(track_color),
                                        );
                                    }
                                    ui.vertical(|ui| {
                                        let name_color = if is_current {
                                            Color32::WHITE
                                        } else {
                                            Color32::from_gray(200)
                                        };
                                        ui.label(
                                            RichText::new(Self::music_track_title(path))
                                                .size(12.0)
                                                .color(name_color),
                                        );
                                        if let Some(metadata) =
                                            Self::music_track_metadata_label(path)
                                        {
                                            ui.label(
                                                RichText::new(metadata)
                                                    .size(10.0)
                                                    .color(Color32::from_gray(120)),
                                            );
                                        }
                                        ui.label(
                                            RichText::new(path.to_string_lossy())
                                                .size(9.0)
                                                .color(Color32::from_gray(95)),
                                        );
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.small_button("Play").clicked() {
                                            self.open_audio_path_in_music_player(path.clone());
                                        }
                                    });
                                });
                            })
                            .response;
                        if resp.interact(Sense::click()).clicked() {
                            self.open_audio_path_in_music_player(path.clone());
                        }
                    }
                } else {
                    for (i, (t_name, t_artist, t_color)) in tracks.iter().enumerate() {
                        let is_current = i == self.music_track_idx;
                        let bg = if is_current {
                            Color32::from_rgba_unmultiplied(255, 255, 255, 15)
                        } else {
                            Color32::TRANSPARENT
                        };
                        let resp = egui::Frame::default()
                            .fill(bg)
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(egui::Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (dot_r, _) =
                                        ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                                    ui.painter().circle_filled(dot_r.center(), 4.0, *t_color);
                                    if is_current && self.music_playing {
                                        ui.label(RichText::new("♪").size(11.0).color(*t_color));
                                    }
                                    ui.vertical(|ui| {
                                        let name_color = if is_current {
                                            Color32::WHITE
                                        } else {
                                            Color32::from_gray(200)
                                        };
                                        ui.label(
                                            RichText::new(*t_name).size(12.0).color(name_color),
                                        );
                                        ui.label(
                                            RichText::new(*t_artist)
                                                .size(10.0)
                                                .color(Color32::from_gray(120)),
                                        );
                                    });
                                });
                            })
                            .response;
                        if resp.interact(Sense::click()).clicked() {
                            self.music_track_idx = i;
                            self.music_override_path = None;
                            self.music_playing = true;
                            self.reset_music_progress();
                        }
                    }
                }
            });
    }

    fn content_photos(&mut self, ui: &mut egui::Ui) {
        let photo_paths = Self::photo_library_paths(&dirs_home());
        let using_real_photos = !photo_paths.is_empty();
        ui.horizontal(|ui| {
            for tab in if using_real_photos {
                ["Library", "Pictures", "Recent", "Albums"]
            } else {
                ["All Photos", "Favorites", "Albums", "People"]
            } {
                let active = tab
                    == if using_real_photos {
                        "Library"
                    } else {
                        "All Photos"
                    };
                let bg = if active {
                    Color32::from_rgba_unmultiplied(0, 122, 255, 80)
                } else {
                    Color32::TRANSPARENT
                };
                ui.add(
                    egui::Button::new(RichText::new(tab).size(11.0).color(if active {
                        Color32::WHITE
                    } else {
                        Color32::from_gray(160)
                    }))
                    .fill(bg)
                    .stroke(Stroke::NONE)
                    .corner_radius(CornerRadius::same(6)),
                );
            }
        });
        ui.add_space(8.0);

        // Photo grid — colored rectangles simulating thumbnails
        let thumb_size = Vec2::splat(64.0);
        let colors = [
            Color32::from_rgb(255, 107, 107),
            Color32::from_rgb(78, 205, 196),
            Color32::from_rgb(255, 230, 109),
            Color32::from_rgb(162, 155, 254),
            Color32::from_rgb(255, 159, 243),
            Color32::from_rgb(69, 183, 209),
            Color32::from_rgb(255, 179, 71),
            Color32::from_rgb(119, 221, 119),
            Color32::from_rgb(207, 159, 255),
            Color32::from_rgb(255, 105, 180),
            Color32::from_rgb(100, 149, 237),
            Color32::from_rgb(255, 218, 185),
            Color32::from_rgb(144, 238, 144),
            Color32::from_rgb(255, 160, 122),
            Color32::from_rgb(173, 216, 230),
            Color32::from_rgb(221, 160, 221),
            Color32::from_rgb(245, 222, 179),
            Color32::from_rgb(176, 224, 230),
            Color32::from_rgb(255, 182, 193),
            Color32::from_rgb(152, 251, 152),
            Color32::from_rgb(135, 206, 250),
            Color32::from_rgb(255, 228, 196),
            Color32::from_rgb(230, 230, 250),
            Color32::from_rgb(250, 128, 114),
        ];
        let cols = ((ui.available_width()) / (thumb_size.x + 6.0))
            .floor()
            .max(1.0) as usize;

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("photo_grid")
                .num_columns(cols)
                .spacing(Vec2::splat(6.0))
                .show(ui, |ui| {
                    let total = if using_real_photos {
                        photo_paths.len()
                    } else {
                        colors.len()
                    };
                    for i in 0..total {
                        let color = if using_real_photos {
                            Self::photo_color(i)
                        } else {
                            colors[i]
                        };
                        let (rect, resp) = ui.allocate_exact_size(thumb_size, Sense::click());
                        let mut painted_texture = false;
                        if using_real_photos {
                            if let Some(texture) =
                                self.photo_texture_for_path(ui.ctx(), &photo_paths[i])
                            {
                                ui.painter().image(
                                    texture.id(),
                                    rect,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                                painted_texture = true;
                            }
                        }
                        if !painted_texture {
                            let lighter = Color32::from_rgba_unmultiplied(
                                (color.r() as u16 + 40).min(255) as u8,
                                (color.g() as u16 + 40).min(255) as u8,
                                (color.b() as u16 + 40).min(255) as u8,
                                255,
                            );
                            gradient_rect(ui.painter(), rect, lighter, color);
                        }
                        ui.painter().rect_stroke(
                            rect,
                            CornerRadius::same(4),
                            Stroke::new(0.5, Color32::from_white_alpha(30)),
                            StrokeKind::Outside,
                        );
                        // Landscape/portrait symbol
                        if i % 5 == 0 {
                            let sun_c = Pos2::new(rect.right() - 14.0, rect.top() + 14.0);
                            ui.painter().circle_filled(
                                sun_c,
                                6.0,
                                Color32::from_rgba_unmultiplied(255, 255, 200, 150),
                            );
                        }
                        if resp.hovered() {
                            ui.painter().rect_stroke(
                                rect,
                                CornerRadius::same(4),
                                Stroke::new(2.0, Color32::from_rgb(0, 122, 255)),
                                StrokeKind::Outside,
                            );
                        }
                        if resp.clicked() {
                            self.photo_viewer_idx = Some(i);
                        }
                        if using_real_photos {
                            let label = photo_paths[i]
                                .file_stem()
                                .and_then(|name| name.to_str())
                                .unwrap_or("Photo");
                            ui.painter().text(
                                rect.center_bottom() + Vec2::new(0.0, -8.0),
                                Align2::CENTER_BOTTOM,
                                label,
                                FontId::proportional(9.0),
                                Color32::WHITE,
                            );
                        }
                        if (i + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });
        });

        // Photo lightbox viewer
        if let Some(idx) = self.photo_viewer_idx {
            let color = Self::photo_color(idx);

            // Dim overlay
            let full = ui.max_rect();
            ui.painter()
                .rect_filled(full, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 180));

            // Large photo
            let photo_size = Vec2::new(full.width() * 0.7, full.height() * 0.7);
            let photo_rect = Rect::from_center_size(full.center(), photo_size);
            let mut painted_texture = false;
            if using_real_photos {
                if let Some(texture) = self.photo_texture_for_path(ui.ctx(), &photo_paths[idx]) {
                    ui.painter().image(
                        texture.id(),
                        photo_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    painted_texture = true;
                }
            }
            if !painted_texture {
                let lighter = Color32::from_rgba_unmultiplied(
                    (color.r() as u16 + 40).min(255) as u8,
                    (color.g() as u16 + 40).min(255) as u8,
                    (color.b() as u16 + 40).min(255) as u8,
                    255,
                );
                gradient_rect(ui.painter(), photo_rect, lighter, color);
            }
            ui.painter().rect_stroke(
                photo_rect,
                CornerRadius::same(8),
                Stroke::new(1.0, Color32::from_white_alpha(40)),
                StrokeKind::Outside,
            );
            if using_real_photos && !painted_texture {
                let label = photo_paths[idx]
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Photo");
                ui.painter().text(
                    photo_rect.center(),
                    Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(20.0),
                    Color32::WHITE,
                );
            }

            // Photo number
            ui.painter().text(
                Pos2::new(photo_rect.center().x, photo_rect.bottom() + 16.0),
                Align2::CENTER_TOP,
                if using_real_photos {
                    format!("{} of {}", idx + 1, photo_paths.len())
                } else {
                    format!("Photo {} of {}", idx + 1, colors.len())
                },
                FontId::proportional(12.0),
                Color32::from_gray(180),
            );
            if using_real_photos {
                if let Some(metadata) = Self::photo_metadata_label(&photo_paths[idx]) {
                    ui.painter().text(
                        Pos2::new(photo_rect.center().x, photo_rect.bottom() + 34.0),
                        Align2::CENTER_TOP,
                        metadata,
                        FontId::proportional(11.0),
                        Color32::from_gray(160),
                    );
                }
            }

            // Navigation arrows
            let left_rect = Rect::from_center_size(
                Pos2::new(photo_rect.left() - 24.0, photo_rect.center().y),
                Vec2::splat(32.0),
            );
            let right_rect = Rect::from_center_size(
                Pos2::new(photo_rect.right() + 24.0, photo_rect.center().y),
                Vec2::splat(32.0),
            );

            let left_resp = ui.interact(left_rect, Id::new("photo_prev"), Sense::click());
            ui.painter().text(
                left_rect.center(),
                Align2::CENTER_CENTER,
                "<",
                FontId::proportional(24.0),
                if left_resp.hovered() {
                    Color32::WHITE
                } else {
                    Color32::from_gray(160)
                },
            );
            if left_resp.clicked() && idx > 0 {
                self.photo_viewer_idx = Some(idx - 1);
            }

            let right_resp = ui.interact(right_rect, Id::new("photo_next"), Sense::click());
            ui.painter().text(
                right_rect.center(),
                Align2::CENTER_CENTER,
                ">",
                FontId::proportional(24.0),
                if right_resp.hovered() {
                    Color32::WHITE
                } else {
                    Color32::from_gray(160)
                },
            );
            let total_photos = if using_real_photos {
                photo_paths.len()
            } else {
                colors.len()
            };
            if right_resp.clicked() && idx + 1 < total_photos {
                self.photo_viewer_idx = Some(idx + 1);
            }

            // Close button (X) or click background
            let close_rect = Rect::from_center_size(
                Pos2::new(photo_rect.right() - 12.0, photo_rect.top() - 12.0),
                Vec2::splat(24.0),
            );
            let close_resp = ui.interact(close_rect, Id::new("photo_close"), Sense::click());
            ui.painter().text(
                close_rect.center(),
                Align2::CENTER_CENTER,
                "X",
                FontId::proportional(14.0),
                if close_resp.hovered() {
                    Color32::WHITE
                } else {
                    Color32::from_gray(180)
                },
            );
            if close_resp.clicked() {
                self.photo_viewer_idx = None;
            }

            let pip_rect = Rect::from_center_size(
                Pos2::new(photo_rect.left() + 26.0, photo_rect.top() - 12.0),
                Vec2::new(40.0, 24.0),
            );
            let pip_resp = ui.interact(pip_rect, Id::new("photo_pip"), Sense::click());
            ui.painter().rect_filled(
                pip_rect,
                CornerRadius::same(6),
                if pip_resp.hovered() {
                    Color32::from_rgba_unmultiplied(0, 122, 255, 180)
                } else {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 32)
                },
            );
            ui.painter().text(
                pip_rect.center(),
                Align2::CENTER_CENTER,
                "PiP",
                FontId::proportional(11.0),
                Color32::WHITE,
            );
            if pip_resp.clicked() {
                self.open_pip(PipSource::Photo(idx), Self::desktop_work_rect(ui.ctx()));
            }

            if using_real_photos {
                let open_rect = Rect::from_center_size(
                    Pos2::new(photo_rect.center().x, photo_rect.top() - 12.0),
                    Vec2::new(74.0, 24.0),
                );
                let open_resp =
                    ui.interact(open_rect, Id::new("photo_open_system"), Sense::click());
                ui.painter().rect_filled(
                    open_rect,
                    CornerRadius::same(6),
                    if open_resp.hovered() {
                        Color32::from_rgba_unmultiplied(0, 122, 255, 180)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 32)
                    },
                );
                ui.painter().text(
                    open_rect.center(),
                    Align2::CENTER_CENTER,
                    "Open",
                    FontId::proportional(11.0),
                    Color32::WHITE,
                );
                if open_resp.clicked() {
                    open_file_with_system(&photo_paths[idx]);
                }
            }

            // Escape to close
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.photo_viewer_idx = None;
            }
            // Arrow keys to navigate
            if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) && idx > 0 {
                self.photo_viewer_idx = Some(idx - 1);
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) && idx + 1 < total_photos {
                self.photo_viewer_idx = Some(idx + 1);
            }
        }
    }

    fn content_calendar(&mut self, ui: &mut egui::Ui) {
        let now = Local::now();
        // Apply month offset for navigation
        let base_date = now.date_naive();
        let target = if self.calendar_month_offset >= 0 {
            base_date
                .checked_add_months(chrono::Months::new(self.calendar_month_offset as u32))
                .unwrap_or(base_date)
        } else {
            base_date
                .checked_sub_months(chrono::Months::new((-self.calendar_month_offset) as u32))
                .unwrap_or(base_date)
        };
        let year = target.year();
        let month = target.month();
        let today_day = if self.calendar_month_offset == 0 {
            now.day()
        } else {
            0
        }; // only highlight today in current month

        let month_names = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        let month_name = month_names[(month - 1) as usize];

        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Label::new(RichText::new("<").size(16.0).color(Color32::from_gray(200)))
                        .sense(Sense::click()),
                )
                .clicked()
            {
                self.calendar_month_offset -= 1;
            }
            ui.add_space(8.0);
            let title_resp = ui.add(
                egui::Label::new(
                    RichText::new(format!("{month_name} {year}"))
                        .size(16.0)
                        .strong()
                        .color(Color32::WHITE),
                )
                .sense(Sense::click()),
            );
            if title_resp.clicked() {
                self.calendar_month_offset = 0; // click title to go back to today
            }
            ui.add_space(8.0);
            if ui
                .add(
                    egui::Label::new(RichText::new(">").size(16.0).color(Color32::from_gray(200)))
                        .sense(Sense::click()),
                )
                .clicked()
            {
                self.calendar_month_offset += 1;
            }
        });
        ui.add_space(10.0);

        // Day-of-week headers
        let cell_size = Vec2::new(34.0, 30.0);
        ui.horizontal(|ui| {
            for day in ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"] {
                let (r, _) = ui.allocate_exact_size(cell_size, Sense::hover());
                ui.painter().text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    day,
                    FontId::proportional(11.0),
                    Color32::from_gray(120),
                );
            }
        });

        // Calculate first day of month (0=Sunday using chrono)
        use chrono::NaiveDate;
        let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let first_weekday = first.weekday().num_days_from_sunday() as usize;
        let days_in_month = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .unwrap()
        .signed_duration_since(first)
        .num_days() as u32;

        let mut day = 1u32;
        for _week in 0..6 {
            if day > days_in_month {
                break;
            }
            ui.horizontal(|ui| {
                for col in 0..7u32 {
                    let (r, _resp) = ui.allocate_exact_size(cell_size, Sense::click());
                    if (_week == 0 && (col as usize) < first_weekday) || day > days_in_month {
                        // Empty cell
                    } else {
                        let is_today = day == today_day;
                        if is_today {
                            ui.painter().circle_filled(
                                r.center(),
                                14.0,
                                Color32::from_rgb(255, 59, 48),
                            );
                        }
                        let text_color = if is_today {
                            Color32::WHITE
                        } else {
                            Color32::from_gray(220)
                        };
                        ui.painter().text(
                            r.center(),
                            Align2::CENTER_CENTER,
                            format!("{day}"),
                            FontId::proportional(13.0),
                            text_color,
                        );
                        day += 1;
                    }
                }
            });
        }

        ui.add_space(12.0);
        // Today's events
        ui.label(
            RichText::new("Today")
                .size(12.0)
                .strong()
                .color(Color32::from_gray(160)),
        );
        ui.add_space(4.0);
        let events = [
            ("9:00 AM", "Team Standup", Color32::from_rgb(0, 122, 255)),
            ("2:00 PM", "Code Review", Color32::from_rgb(52, 199, 89)),
            ("5:30 PM", "Gym", Color32::from_rgb(255, 149, 0)),
        ];
        for (time, title, color) in events {
            ui.horizontal(|ui| {
                let (dot_r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                ui.painter().circle_filled(dot_r.center(), 4.0, color);
                ui.label(
                    RichText::new(time)
                        .size(11.0)
                        .color(Color32::from_gray(140)),
                );
                ui.label(
                    RichText::new(title)
                        .size(12.0)
                        .color(Color32::from_gray(220)),
                );
            });
        }
    }

    // ── Text Editor ──────────────────────────────────────────────────────────

    fn content_text_editor(
        ui: &mut egui::Ui,
        file_path: &Option<PathBuf>,
        content: &mut String,
        modified: &mut bool,
    ) -> bool {
        let gray = Color32::from_gray(160);
        let white = Color32::from_gray(230);
        let mut open_emoji_picker = false;

        // Toolbar
        ui.horizontal(|ui| {
            let title = if let Some(ref path) = file_path {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Untitled");
                if *modified {
                    format!("{name} (modified)")
                } else {
                    name.to_string()
                }
            } else {
                "Untitled".to_string()
            };
            ui.label(RichText::new(&title).size(13.0).strong().color(white));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("😀").size(11.0).color(Color32::WHITE))
                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 16))
                            .corner_radius(CornerRadius::same(4))
                            .min_size(Vec2::new(32.0, 22.0)),
                    )
                    .clicked()
                {
                    open_emoji_picker = true;
                }
                // Save button
                if let Some(ref path) = file_path {
                    if *modified {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Save").size(11.0).color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(0, 122, 255))
                                .corner_radius(CornerRadius::same(4))
                                .min_size(Vec2::new(50.0, 22.0)),
                            )
                            .clicked()
                        {
                            if fs::write(path, content.as_bytes()).is_ok() {
                                *modified = false;
                            }
                        }
                    }
                }
                // Line/char count
                let lines = content.lines().count();
                let chars = content.len();
                ui.label(
                    RichText::new(format!("{lines} lines | {chars} chars"))
                        .size(10.0)
                        .color(gray),
                );
            });
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // File path breadcrumb
        if let Some(ref path) = file_path {
            ui.label(
                RichText::new(path.to_string_lossy().to_string())
                    .size(10.0)
                    .color(Color32::from_gray(100)),
            );
            ui.add_space(4.0);
        }

        // Editor area
        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let font_id = FontId::monospace(13.0);
                let resp = ui.add(
                    egui::TextEdit::multiline(content)
                        .font(font_id)
                        .text_color(Color32::from_gray(220))
                        .desired_width(f32::INFINITY)
                        .desired_rows(30)
                        .code_editor(),
                );
                if resp.changed() {
                    *modified = true;
                }
            });
        open_emoji_picker
    }

    fn open_file_in_editor(&mut self, path: PathBuf) {
        match fs::read_to_string(&path) {
            Ok(content) => {
                self.editor_content = content;
                self.editor_file_path = Some(path.clone());
                self.editor_modified = false;
                self.sync_active_tab_from_globals(WindowKind::TextEditor);
                let win = self.window_mut(WindowKind::TextEditor);
                win.open = true;
                win.minimized = false;
                win.open_anim_start = Some(Instant::now());
                win.closing = false;
                win.close_anim_start = None;
                win.id_epoch = win.id_epoch.saturating_add(1);
                self.bring_to_front(WindowKind::TextEditor);
                self.toast_manager.push(Toast::new(
                    "File Opened",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
                    Color32::from_rgb(52, 199, 89),
                ));
            }
            Err(e) => {
                self.toast_manager.push(Toast::new(
                    "Error",
                    format!("Cannot open file: {e}"),
                    Color32::from_rgb(255, 59, 48),
                ));
            }
        }
    }

    // ── Settings ─────────────────────────────────────────────────────────────

    fn accent_color(&self) -> Color32 {
        Color32::from_rgb(
            self.app_settings.accent_r,
            self.app_settings.accent_g,
            self.app_settings.accent_b,
        )
    }

    fn content_settings(&mut self, ui: &mut egui::Ui) {
        let white = Color32::from_gray(230);
        let gray = Color32::from_gray(140);
        let accent = self.accent_color();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.label(RichText::new("General").size(14.0).strong().color(white));
                ui.add_space(6.0);

                // Wallpaper selection
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Wallpaper").size(12.0).color(gray));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let name = WALLPAPERS
                            .get(self.wallpaper_idx)
                            .map(|w| w.name)
                            .unwrap_or("Unknown");
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!("◀ {} ▶", name))
                                        .size(11.0)
                                        .color(Color32::WHITE),
                                )
                                .fill(accent)
                                .corner_radius(CornerRadius::same(4)),
                            )
                            .clicked()
                        {
                            self.wallpaper_prev_idx = self.wallpaper_idx;
                            self.wallpaper_idx = (self.wallpaper_idx + 1) % WALLPAPERS.len();
                            self.wallpaper_transition = 0.0;
                            self.wallpaper_changing = true;
                        }
                    });
                });
                ui.add_space(8.0);

                // Volume
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Volume").size(12.0).color(gray));
                    ui.add(egui::Slider::new(&mut self.cc_volume, 0.0..=1.0).show_value(true));
                });
                ui.add_space(4.0);

                // Brightness
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Brightness").size(12.0).color(gray));
                    ui.add(egui::Slider::new(&mut self.cc_brightness, 0.0..=1.0).show_value(true));
                });
                ui.add_space(12.0);

                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new("Network").size(14.0).strong().color(white));
                ui.add_space(6.0);

                Self::toggle_row(ui, "Wi-Fi", &mut self.cc_wifi, gray);
                Self::toggle_row(ui, "Bluetooth", &mut self.cc_bluetooth, gray);
                Self::toggle_row(ui, "AirDrop", &mut self.cc_airdrop, gray);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Diagnostics").size(12.0).color(gray));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Assist Me...")
                                        .size(11.0)
                                        .color(Color32::WHITE),
                                )
                                .fill(accent)
                                .corner_radius(CornerRadius::same(4)),
                            )
                            .clicked()
                        {
                            let win = self.window_mut(WindowKind::NetworkDiagnostics);
                            win.restore();
                            win.id_epoch = win.id_epoch.saturating_add(1);
                            self.bring_to_front(WindowKind::NetworkDiagnostics);
                        }
                    });
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new("Appearance").size(14.0).strong().color(white));
                ui.add_space(6.0);

                Self::toggle_row(ui, "Dark Mode", &mut self.app_settings.dark_mode, gray);
                ui.add_space(4.0);

                // Accent color picker
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Accent Color").size(12.0).color(gray));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let presets: &[(u8, u8, u8, &str)] = &[
                            (0, 122, 255, "Blue"),
                            (255, 59, 48, "Red"),
                            (52, 199, 89, "Green"),
                            (255, 149, 0, "Orange"),
                            (175, 82, 222, "Purple"),
                            (255, 45, 85, "Pink"),
                        ];
                        for &(r, g, b, _name) in presets.iter().rev() {
                            let color = Color32::from_rgb(r, g, b);
                            let is_selected = self.app_settings.accent_r == r
                                && self.app_settings.accent_g == g
                                && self.app_settings.accent_b == b;
                            let size = if is_selected { 16.0 } else { 12.0 };
                            let (dot_r, _) =
                                ui.allocate_exact_size(Vec2::splat(size), Sense::click());
                            ui.painter()
                                .circle_filled(dot_r.center(), size / 2.0, color);
                            if is_selected {
                                ui.painter().circle_stroke(
                                    dot_r.center(),
                                    size / 2.0 + 1.5,
                                    Stroke::new(1.5, Color32::WHITE),
                                );
                            }
                            if ui
                                .interact(dot_r, Id::new(("accent", r, g, b)), Sense::click())
                                .clicked()
                            {
                                self.app_settings.accent_r = r;
                                self.app_settings.accent_g = g;
                                self.app_settings.accent_b = b;
                            }
                        }
                    });
                });
                ui.add_space(4.0);

                // Custom wallpaper
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Custom Wallpaper").size(12.0).color(gray));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !self.app_settings.custom_wallpaper.is_empty() {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Clear")
                                            .size(10.0)
                                            .color(Color32::from_gray(180)),
                                    )
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                                )
                                .clicked()
                            {
                                self.app_settings.custom_wallpaper.clear();
                            }
                        }
                        let path_display = if self.app_settings.custom_wallpaper.is_empty() {
                            "None (built-in)"
                        } else {
                            &self.app_settings.custom_wallpaper
                        };
                        ui.label(
                            RichText::new(path_display)
                                .size(10.0)
                                .color(Color32::from_gray(120)),
                        );
                    });
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Show FPS Overlay").size(12.0).color(gray));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let label = if self.fps_smoothed > 0.0 { "On" } else { "Off" };
                        ui.label(RichText::new(label).size(11.0).color(accent));
                    });
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new("Dock").size(14.0).strong().color(white));
                ui.add_space(6.0);

                Self::toggle_row(
                    ui,
                    "Automatically Hide Dock",
                    &mut self.app_settings.dock_auto_hide,
                    gray,
                );
                Self::toggle_row(
                    ui,
                    "Show Running Indicators",
                    &mut self.app_settings.dock_show_running_indicators,
                    gray,
                );
                Self::toggle_row(
                    ui,
                    "Magnification",
                    &mut self.app_settings.dock_magnification,
                    gray,
                );

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Icon Size").size(12.0).color(gray));
                    ui.add(
                        egui::Slider::new(&mut self.app_settings.dock_icon_size, 32.0..=96.0)
                            .show_value(true),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Position").size(12.0).color(gray));
                    for position in [
                        DockPosition::Bottom,
                        DockPosition::Left,
                        DockPosition::Right,
                    ] {
                        let selected = Self::dock_position(&self.app_settings) == position;
                        let fill = if selected {
                            accent
                        } else {
                            Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(position.as_str())
                                        .size(11.0)
                                        .color(Color32::WHITE),
                                )
                                .fill(fill),
                            )
                            .clicked()
                        {
                            self.app_settings.dock_position = position.as_str().to_string();
                        }
                    }
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Users & Groups")
                        .size(14.0)
                        .strong()
                        .color(white),
                );
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    let (avatar_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
                    ui.painter()
                        .circle_filled(avatar_rect.center(), 15.0, self.avatar_color());
                    ui.painter().text(
                        avatar_rect.center(),
                        Align2::CENTER_CENTER,
                        self.avatar_initials(),
                        FontId::proportional(11.0),
                        Color32::WHITE,
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(self.profile_display_name())
                            .size(12.0)
                            .color(Color32::WHITE),
                    );
                });
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Display Name").size(12.0).color(gray));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.profile_name_buffer)
                            .desired_width(180.0)
                            .font(FontId::proportional(12.0)),
                    );
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Save").size(11.0).color(Color32::WHITE),
                            )
                            .fill(accent)
                            .corner_radius(CornerRadius::same(4)),
                        )
                        .clicked()
                    {
                        let next_name = self.profile_name_buffer.trim();
                        if !next_name.is_empty() {
                            self.user_profile = UserProfile::from_display_name(
                                next_name,
                                (
                                    self.user_profile.avatar_r,
                                    self.user_profile.avatar_g,
                                    self.user_profile.avatar_b,
                                ),
                            );
                            self.app_settings.user_name = next_name.to_string();
                            let _ = self.user_profile.save();
                            let _ = self.app_settings.save();
                        }
                    }
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Tags & Smart Folders")
                        .size(14.0)
                        .strong()
                        .color(white),
                );
                ui.add_space(6.0);

                for color in TagColor::ALL {
                    let mut label = self.tag_label(color);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{} Label", color.as_str()))
                                .size(12.0)
                                .color(gray),
                        );
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut label)
                                    .desired_width(180.0)
                                    .font(FontId::proportional(12.0)),
                            )
                            .changed()
                        {
                            self.set_tag_label(color, &label);
                        }
                    });
                }

                ui.add_space(8.0);
                ui.label(
                    RichText::new("Create Custom Smart Folder")
                        .size(12.0)
                        .color(gray),
                );
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_custom_folder_name)
                        .hint_text("Name")
                        .desired_width(200.0),
                );
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_custom_folder_extension)
                        .hint_text("Extension (optional)")
                        .desired_width(200.0),
                );
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_custom_folder_min_size_mb)
                        .hint_text("Min size MB (optional)")
                        .desired_width(200.0),
                );
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_custom_folder_tag)
                        .hint_text("Tag color (optional)")
                        .desired_width(200.0),
                );
                ui.add_space(4.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Add Smart Folder")
                                .size(11.0)
                                .color(Color32::WHITE),
                        )
                        .fill(accent)
                        .corner_radius(CornerRadius::same(4)),
                    )
                    .clicked()
                {
                    let name = self.settings_custom_folder_name.trim();
                    if !name.is_empty() {
                        let mut folders = Self::load_custom_smart_folders(&self.app_settings);
                        folders.push(CustomSmartFolder {
                            name: name.to_string(),
                            extension: (!self.settings_custom_folder_extension.trim().is_empty())
                                .then(|| self.settings_custom_folder_extension.trim().to_string()),
                            min_size_mb: self
                                .settings_custom_folder_min_size_mb
                                .trim()
                                .parse::<u64>()
                                .ok(),
                            tag: self
                                .settings_custom_folder_tag
                                .trim()
                                .to_ascii_lowercase()
                                .is_empty()
                                .then_some(String::new())
                                .or_else(|| {
                                    Some(
                                        self.settings_custom_folder_tag.trim().to_ascii_lowercase(),
                                    )
                                })
                                .filter(|value| !value.is_empty()),
                        });
                        self.save_custom_smart_folders(&folders);
                        self.settings_custom_folder_name.clear();
                        self.settings_custom_folder_extension.clear();
                        self.settings_custom_folder_min_size_mb.clear();
                        self.settings_custom_folder_tag.clear();
                    }
                }
                let mut remove_index = None;
                for (idx, folder) in Self::load_custom_smart_folders(&self.app_settings)
                    .iter()
                    .enumerate()
                {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(folder.name.clone())
                                .size(11.0)
                                .color(Color32::WHITE),
                        );
                        let summary = format!(
                            "{}{}{}",
                            folder
                                .extension
                                .as_deref()
                                .map(|ext| format!(".{ext}"))
                                .unwrap_or_default(),
                            folder
                                .min_size_mb
                                .map(|size| format!(" {}MB+", size))
                                .unwrap_or_default(),
                            folder
                                .tag
                                .as_deref()
                                .map(|tag| format!(" tag:{tag}"))
                                .unwrap_or_default(),
                        );
                        ui.label(
                            RichText::new(summary)
                                .size(10.0)
                                .color(Color32::from_gray(110)),
                        );
                        if ui.small_button("Remove").clicked() {
                            remove_index = Some(idx);
                        }
                    });
                }
                if let Some(index) = remove_index {
                    let mut folders = Self::load_custom_smart_folders(&self.app_settings);
                    folders.remove(index);
                    self.save_custom_smart_folders(&folders);
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new("Security").size(14.0).strong().color(white));
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Signed In As").size(12.0).color(gray));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(self.profile_display_name())
                                .size(11.0)
                                .color(Color32::WHITE),
                        );
                    });
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Change Password").size(12.0).color(gray));
                });
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_current_password)
                        .hint_text("Current password")
                        .password(true)
                        .desired_width(220.0),
                );
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_new_password)
                        .hint_text("New password")
                        .password(true)
                        .desired_width(220.0),
                );
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_confirm_password)
                        .hint_text("Confirm new password")
                        .password(true)
                        .desired_width(220.0),
                );
                ui.add_space(6.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Update Password")
                                .size(11.0)
                                .color(Color32::WHITE),
                        )
                        .fill(accent)
                        .corner_radius(CornerRadius::same(4)),
                    )
                    .clicked()
                {
                    match Self::try_change_password(
                        &mut self.app_settings,
                        &self.settings_current_password,
                        &self.settings_new_password,
                        &self.settings_confirm_password,
                    ) {
                        Ok(()) => {
                            self.settings_current_password.clear();
                            self.settings_new_password.clear();
                            self.settings_confirm_password.clear();
                            self.settings_password_message = Some("Password updated".to_string());
                            let _ = self.app_settings.save();
                        }
                        Err(message) => {
                            self.settings_password_message = Some(message.to_string());
                        }
                    }
                }
                if let Some(message) = &self.settings_password_message {
                    ui.add_space(4.0);
                    let color = if message == "Password updated" {
                        Color32::from_rgb(52, 199, 89)
                    } else {
                        Color32::from_rgb(255, 99, 99)
                    };
                    ui.label(RichText::new(message).size(11.0).color(color));
                }
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Auto-Lock (minutes)").size(12.0).color(gray));
                    ui.add(
                        egui::Slider::new(&mut self.app_settings.idle_lock_minutes, 1..=60)
                            .show_value(true),
                    );
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Lock Screen").size(12.0).color(gray));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Lock Now").size(11.0).color(Color32::WHITE),
                                )
                                .fill(Color32::from_gray(60))
                                .corner_radius(CornerRadius::same(4)),
                            )
                            .clicked()
                        {
                            self.lock_screen("Locked");
                        }
                    });
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new("About").size(14.0).strong().color(white));
                ui.add_space(6.0);
                ui.label(
                    RichText::new("AuroraOS Desktop v0.2.0")
                        .size(12.0)
                        .color(gray),
                );
                ui.label(
                    RichText::new("Built with Rust + egui")
                        .size(11.0)
                        .color(Color32::from_gray(100)),
                );
                ui.label(
                    RichText::new(format!(
                        "User: {} (@{})",
                        self.user_profile.display_name, self.user_profile.username
                    ))
                    .size(11.0)
                    .color(Color32::from_gray(100)),
                );
                ui.label(
                    RichText::new(if self.user_profile.is_admin {
                        "Role: Administrator"
                    } else {
                        "Role: Standard User"
                    })
                    .size(11.0)
                    .color(Color32::from_gray(100)),
                );
                ui.label(
                    RichText::new(Self::profile_created_label(&self.user_profile))
                        .size(11.0)
                        .color(Color32::from_gray(100)),
                );
                ui.label(
                    RichText::new(format!("Process count: {}", self.sysinfo.process_count))
                        .size(11.0)
                        .color(Color32::from_gray(100)),
                );
            });
    }

    fn toggle_row(ui: &mut egui::Ui, label: &str, value: &mut bool, color: Color32) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).size(12.0).color(color));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let text = if *value { "On" } else { "Off" };
                let btn_color = if *value {
                    Color32::from_rgb(52, 199, 89)
                } else {
                    Color32::from_gray(80)
                };
                if ui
                    .add(
                        egui::Button::new(RichText::new(text).size(11.0).color(Color32::WHITE))
                            .fill(btn_color)
                            .corner_radius(CornerRadius::same(10))
                            .min_size(Vec2::new(44.0, 22.0)),
                    )
                    .clicked()
                {
                    *value = !*value;
                }
            });
        });
        ui.add_space(4.0);
    }

    // ── Process Manager ─────────────────────────────────────────────────────

    fn content_process_manager(&mut self, ui: &mut egui::Ui) {
        let white = Color32::from_gray(230);
        let gray = Color32::from_gray(140);

        // Lazy init
        if self.proc_manager.is_none() {
            self.proc_manager = Some(ProcessManager::new());
        }

        ui.horizontal(|ui| {
            ui.label(RichText::new("🔍").size(13.0));
            ui.add(
                egui::TextEdit::singleline(&mut self.proc_search)
                    .hint_text("Search processes...")
                    .desired_width(200.0)
                    .text_color(white),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Refresh").size(11.0).color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(0, 122, 255))
                        .corner_radius(CornerRadius::same(4)),
                    )
                    .clicked()
                {
                    if let Some(ref mut pm) = self.proc_manager {
                        pm.refresh();
                    }
                }
                let sort_label = if self.proc_sort_by_cpu {
                    "Sort: CPU"
                } else {
                    "Sort: Memory"
                };
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(sort_label).size(11.0).color(Color32::WHITE),
                        )
                        .fill(Color32::from_gray(60))
                        .corner_radius(CornerRadius::same(4)),
                    )
                    .clicked()
                {
                    self.proc_sort_by_cpu = !self.proc_sort_by_cpu;
                }
            });
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // Header row
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(60.0, 16.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.label(RichText::new("PID").size(10.0).strong().color(gray));
                },
            );
            ui.allocate_ui_with_layout(
                Vec2::new(200.0, 16.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.label(RichText::new("Name").size(10.0).strong().color(gray));
                },
            );
            ui.allocate_ui_with_layout(
                Vec2::new(70.0, 16.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.label(RichText::new("CPU %").size(10.0).strong().color(gray));
                },
            );
            ui.allocate_ui_with_layout(
                Vec2::new(80.0, 16.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.label(RichText::new("Memory").size(10.0).strong().color(gray));
                },
            );
        });
        ui.add_space(2.0);

        if let Some(ref pm) = self.proc_manager {
            let procs = if self.proc_search.is_empty() {
                if self.proc_sort_by_cpu {
                    pm.list_sorted_by_cpu()
                } else {
                    pm.list_sorted_by_memory()
                }
            } else {
                let mut p = pm.search(&self.proc_search);
                if self.proc_sort_by_cpu {
                    p.sort_by(|a, b| {
                        b.cpu_usage
                            .partial_cmp(&a.cpu_usage)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                } else {
                    p.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
                }
                p
            };

            let total = procs.len();
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for proc in procs.iter().take(200) {
                        ui.horizontal(|ui| {
                            ui.allocate_ui_with_layout(
                                Vec2::new(60.0, 16.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(format!("{}", proc.pid))
                                            .size(10.0)
                                            .color(gray),
                                    );
                                },
                            );
                            ui.allocate_ui_with_layout(
                                Vec2::new(200.0, 16.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.label(RichText::new(&proc.name).size(10.0).color(white));
                                },
                            );
                            ui.allocate_ui_with_layout(
                                Vec2::new(70.0, 16.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    let cpu_color = if proc.cpu_usage > 50.0 {
                                        Color32::from_rgb(255, 59, 48)
                                    } else if proc.cpu_usage > 10.0 {
                                        Color32::from_rgb(255, 149, 0)
                                    } else {
                                        Color32::from_gray(180)
                                    };
                                    ui.label(
                                        RichText::new(format!("{:.1}", proc.cpu_usage))
                                            .size(10.0)
                                            .color(cpu_color),
                                    );
                                },
                            );
                            ui.allocate_ui_with_layout(
                                Vec2::new(80.0, 16.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(ProcessManager::format_memory(
                                            proc.memory_bytes,
                                        ))
                                        .size(10.0)
                                        .color(Color32::from_gray(180)),
                                    );
                                },
                            );
                        });
                    }
                });

            ui.add_space(4.0);
            ui.separator();
            ui.label(
                RichText::new(format!("{total} processes"))
                    .size(10.0)
                    .color(gray),
            );
        }
    }

    // ── Unsaved changes confirmation ────────────────────────────────────────

    fn render_confirm_close(&mut self, ctx: &egui::Context) {
        if let Some(kind) = self.confirm_close_window {
            let mut open = true;
            egui::Window::new("Unsaved Changes")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .open(&mut open)
                .frame(
                    egui::Frame::NONE
                        .fill(Color32::from_rgba_unmultiplied(40, 40, 50, 240))
                        .stroke(Stroke::new(1.0, Color32::from_gray(80)))
                        .corner_radius(CornerRadius::same(12))
                        .inner_margin(egui::Margin::same(20))
                        .shadow(egui::epaint::Shadow {
                            offset: [0, 4],
                            blur: 20,
                            spread: 0,
                            color: Color32::from_black_alpha(100),
                        }),
                )
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new("You have unsaved changes.")
                            .size(14.0)
                            .color(Color32::from_gray(230)),
                    );
                    ui.label(
                        RichText::new("Do you want to save before closing?")
                            .size(12.0)
                            .color(Color32::from_gray(160)),
                    );
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        // Save & close
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Save").size(12.0).color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(0, 122, 255))
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(70.0, 28.0)),
                            )
                            .clicked()
                        {
                            if let Some(ref path) = self.editor_file_path {
                                let _ = fs::write(path, self.editor_content.as_bytes());
                            }
                            self.editor_modified = false;
                            let w = self.window_mut(kind);
                            w.start_close();
                            self.confirm_close_window = None;
                        }
                        ui.add_space(8.0);
                        // Discard & close
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Don't Save").size(12.0).color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(255, 59, 48))
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(90.0, 28.0)),
                            )
                            .clicked()
                        {
                            self.editor_modified = false;
                            let w = self.window_mut(kind);
                            w.start_close();
                            self.confirm_close_window = None;
                        }
                        ui.add_space(8.0);
                        // Cancel
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Cancel")
                                        .size(12.0)
                                        .color(Color32::from_gray(200)),
                                )
                                .fill(Color32::from_gray(60))
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(70.0, 28.0)),
                            )
                            .clicked()
                        {
                            self.confirm_close_window = None;
                        }
                    });
                });
            if !open {
                self.confirm_close_window = None;
            }
        }
    }

    // ── Toast notifications ──────────────────────────────────────────────────

    fn render_toasts(&mut self, ctx: &egui::Context) {
        self.toast_manager.tick();
        let screen = ctx.viewport_rect();
        let toast_w = 300.0;
        let toast_h = 60.0;

        // Collect visible toasts data to avoid borrow conflict
        let visible: Vec<(String, String, Color32, f32)> = self
            .toast_manager
            .visible()
            .map(|t| (t.title.clone(), t.body.clone(), t.color, t.slide_factor()))
            .collect();
        let overflow = self.toast_manager.overflow_count();

        for (i, (title, body, color, slide)) in visible.iter().enumerate() {
            let x = screen.right() - toast_w - 16.0 + (1.0 - slide) * (toast_w + 20.0);
            let y = screen.top() + MENU_BAR_HEIGHT + 12.0 + i as f32 * (toast_h + 8.0);

            egui::Area::new(Id::new(("toast", i)))
                .fixed_pos(Pos2::new(x, y))
                .order(Order::Foreground)
                .interactable(false)
                .show(ctx, |ui| {
                    egui::Frame::default()
                        .fill(Color32::from_rgba_unmultiplied(30, 30, 35, 220))
                        .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .show(ui, |ui| {
                            ui.set_min_width(toast_w - 24.0);
                            ui.horizontal(|ui| {
                                let (dot, _) =
                                    ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                                ui.painter().circle_filled(dot.center(), 5.0, *color);
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(title)
                                            .size(12.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    );
                                    ui.label(
                                        RichText::new(body)
                                            .size(11.0)
                                            .color(Color32::from_gray(160)),
                                    );
                                });
                            });
                        });
                });
        }

        // Overflow indicator
        if overflow > 0 {
            let y = screen.top() + MENU_BAR_HEIGHT + 12.0 + visible.len() as f32 * (toast_h + 8.0);
            egui::Area::new(Id::new("toast_overflow"))
                .fixed_pos(Pos2::new(screen.right() - toast_w - 16.0, y))
                .order(Order::Foreground)
                .interactable(false)
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new(format!("+{} more", overflow))
                            .size(11.0)
                            .color(Color32::from_gray(120)),
                    );
                });
        }
    }

    // ── Window rendering ─────────────────────────────────────────────────────

    fn traffic_light(ui: &mut egui::Ui, color: Color32, label: &'static str) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(13.0), Sense::click());
        ui.painter().circle_filled(rect.center(), 5.5, color);
        if response.hovered() {
            ui.painter().circle_stroke(
                rect.center(),
                5.5,
                Stroke::new(0.5, Color32::from_black_alpha(40)),
            );
        }
        response.on_hover_text(label)
    }

    fn render_windows(&mut self, ctx: &egui::Context, work_rect: Rect) {
        // Snapshot data for borrow-checker
        let cpu = self.sysinfo.cpu_usage;
        let mem_used = self.sysinfo.used_memory_gb;
        let mem_total = self.sysinfo.total_memory_gb;
        let mem_pct = self.sysinfo.memory_pct;
        let disk_used = self.sysinfo.disk_used_gb;
        let disk_total = self.sysinfo.disk_total_gb;
        let proc_count = self.sysinfo.process_count;
        let net_up = self.sysinfo.network_up;
        let net_name = self.sysinfo.network_name.clone();
        let batt_pct = self.sysinfo.battery_pct;
        let batt_charging = self.sysinfo.battery_charging;
        let batt_available = self.sysinfo.battery_available;

        let si_snap = RealSystemInfo {
            sys: System::new(),
            networks: Networks::new(),
            cpu_usage: cpu,
            total_memory_gb: mem_total,
            used_memory_gb: mem_used,
            memory_pct: mem_pct,
            battery_pct: batt_pct,
            battery_charging: batt_charging,
            battery_available: batt_available,
            network_up: net_up,
            network_name: net_name,
            disk_total_gb: disk_total,
            disk_used_gb: disk_used,
            process_count: proc_count,
            last_refresh: None,
        };

        let telemetry = Telemetry {
            connected: self.telemetry.connected,
            status: self.telemetry.status.clone(),
            health: self.telemetry.health.clone(),
            uptime: self.telemetry.uptime.clone(),
            boot: self.telemetry.boot.clone(),
            last_error: self.telemetry.last_error.clone(),
            last_poll: self.telemetry.last_poll,
        };

        let mut bring_front = None;
        let mut fm_navigate: Option<PathBuf> = None;
        let mut term_cmd: Option<String> = None;
        let fm_dir = self.fm_current_dir.clone();
        let fm_entries = self.fm_entries.clone();
        let term_lines = self.terminal_output.clone();
        let cpu_hist = self.cpu_history.clone();

        for kind in self.z_order.clone() {
            let win_ref = self.window_ref(kind);
            let snap = win_ref.snapshot();
            if !snap.open || snap.minimized {
                continue;
            }
            // Only show windows on the current desktop
            if win_ref.desktop != self.current_desktop {
                continue;
            }

            // Animation alpha/scale
            let anim_alpha = win_ref.anim_alpha();
            let _anim_scale = win_ref.anim_scale();

            let mut close_clicked = false;
            let mut minimize_clicked = false;
            let mut maximize_clicked = false;

            let is_terminal = kind == WindowKind::Terminal;
            let alpha_byte = (anim_alpha * 255.0) as u8;
            let dark = self.app_settings.dark_mode;
            let (win_fill, win_stroke, title_color) = if is_terminal {
                (
                    Color32::from_rgba_unmultiplied(30, 30, 46, alpha_byte),
                    Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(255, 255, 255, (30.0 * anim_alpha) as u8),
                    ),
                    Color32::from_rgba_unmultiplied(180, 180, 180, alpha_byte),
                )
            } else if dark {
                (
                    Color32::from_rgba_unmultiplied(30, 30, 34, (230.0 * anim_alpha) as u8),
                    Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(60, 60, 65, (200.0 * anim_alpha) as u8),
                    ),
                    Color32::from_rgba_unmultiplied(220, 220, 225, alpha_byte),
                )
            } else {
                (
                    Color32::from_rgba_unmultiplied(255, 255, 255, (38.0 * anim_alpha) as u8),
                    Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(255, 255, 255, (86.0 * anim_alpha) as u8),
                    ),
                    Color32::from_rgba_unmultiplied(245, 245, 245, alpha_byte),
                )
            };

            // Frame with NO inner_margin — padding is handled inside the content closure.
            // This ensures default_size/fixed_size == response.rect.size() (no mismatch).
            let win_frame = egui::Frame::NONE
                .fill(win_fill)
                .stroke(win_stroke)
                .corner_radius(CornerRadius::same(12))
                .shadow({
                    let is_focused = self.focused == Some(kind);
                    let (blur, alpha) = if is_focused { (24, 90.0) } else { (12, 40.0) };
                    egui::epaint::Shadow {
                        offset: [0, if is_focused { 6 } else { 3 }],
                        blur,
                        spread: 0,
                        color: Color32::from_black_alpha((alpha * anim_alpha) as u8),
                    }
                });

            let full_viewport = ctx.viewport_rect();

            // IMPORTANT: resizable must be TRUE when using fixed_size, otherwise
            // egui enters auto-size mode and shrinks to content, ignoring fixed_size.
            // fixed_size(x) sets min=max=x so the user can't actually resize.
            let is_fixed = snap.maximized || snap.snap.is_some();

            let mut window = egui::Window::new(kind.title())
                .id(Id::new(("window", kind.title(), snap.id_epoch)))
                .title_bar(false)
                .resizable(true)
                .collapsible(false)
                .default_pos(snap.default_pos)
                .default_size(snap.default_size)
                .frame(win_frame)
                .constrain_to(full_viewport);

            if snap.maximized {
                window = window
                    .constrain(false)
                    .pivot(Align2::LEFT_TOP)
                    .fixed_pos(work_rect.min)
                    .fixed_size(work_rect.size());
            } else if let Some(side) = snap.snap {
                let r = Self::snap_rect(work_rect, side);
                window = window
                    .constrain(false)
                    .pivot(Align2::LEFT_TOP)
                    .fixed_pos(r.min)
                    .fixed_size(r.size());
            }

            let response = window.show(ctx, |ui| {
                // When maximized/snapped, force the UI to claim the full target size.
                // This overrides egui's auto-sizing which would shrink to content.
                if snap.maximized {
                    ui.set_min_size(work_rect.size());
                } else if let Some(side) = snap.snap {
                    ui.set_min_size(Self::snap_rect(work_rect, side).size());
                }

                // Inner padding — keeps visual spacing without polluting the Frame's layout math
                egui::Frame::NONE
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if Self::traffic_light(ui, Color32::from_rgb(255, 95, 87), "Close")
                                .clicked()
                            {
                                close_clicked = true;
                            }
                            if Self::traffic_light(ui, Color32::from_rgb(255, 189, 47), "Minimize")
                                .clicked()
                            {
                                minimize_clicked = true;
                            }
                            if Self::traffic_light(ui, Color32::from_rgb(40, 200, 64), "Maximize")
                                .clicked()
                            {
                                maximize_clicked = true;
                            }
                            ui.add_space(8.0);
                            // Double-click title text to toggle maximize
                            // Dynamic title for text editor (show filename)
                            let display_title = match kind {
                                WindowKind::TextEditor => {
                                    if let Some(ref path) = self.editor_file_path {
                                        let name = path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("Untitled");
                                        if self.editor_modified {
                                            format!("{name} — TextEdit (edited)")
                                        } else {
                                            format!("{name} — TextEdit")
                                        }
                                    } else {
                                        "Untitled — TextEdit".to_string()
                                    }
                                }
                                WindowKind::Notes => self
                                    .notes_tabs
                                    .get(self.notes_active_tab)
                                    .map(|tab| format!("{} — Notes", tab.title))
                                    .unwrap_or_else(|| "Notes".to_string()),
                                WindowKind::Terminal => self
                                    .terminal_tabs
                                    .get(self.terminal_active_tab)
                                    .map(|tab| format!("{} — Terminal", tab.title))
                                    .unwrap_or_else(|| "Terminal".to_string()),
                                _ => kind.title().to_string(),
                            };
                            let title_resp = ui.add(
                                egui::Label::new(
                                    RichText::new(&display_title).size(13.0).color(title_color),
                                )
                                .sense(Sense::click()),
                            );
                            if title_resp.double_clicked() {
                                maximize_clicked = true;
                            }
                        });
                        ui.add_space(6.0);
                        if Self::window_supports_tabs(kind) {
                            self.render_window_tabs(ui, kind);
                        } else {
                            ui.separator();
                            ui.add_space(8.0);
                        }

                        match kind {
                            WindowKind::Overview => {
                                Self::content_overview(ui, &si_snap, &telemetry, &cpu_hist)
                            }
                            WindowKind::Terminal => {
                                let use_pty_tab =
                                    self.terminal_active_tab == 0 && self.pty_terminal.is_some();
                                if use_pty_tab {
                                    let mut pty_lines = Vec::new();
                                    if let Some(pty) = self.pty_terminal.as_mut() {
                                        Self::content_terminal_pty(
                                            ui,
                                            pty,
                                            &mut self.terminal_input,
                                        );
                                        pty_lines = pty
                                            .scrollback
                                            .iter()
                                            .cloned()
                                            .map(|line| (line, Color32::from_gray(185)))
                                            .collect();
                                    }
                                    if let Some(tab) =
                                        self.terminal_tabs.get_mut(self.terminal_active_tab)
                                    {
                                        tab.input = self.terminal_input.clone();
                                        tab.output = pty_lines;
                                    }
                                } else if let Some(cmd) = Self::content_terminal_builtin(
                                    ui,
                                    &si_snap,
                                    &term_lines,
                                    &mut self.terminal_input,
                                ) {
                                    term_cmd = Some(cmd);
                                }
                                self.sync_active_tab_from_globals(WindowKind::Terminal);
                            }
                            WindowKind::FileManager => {
                                if let Some(nav) = Self::content_filemanager(
                                    ui,
                                    &fm_dir,
                                    &fm_entries,
                                    &mut self.fm_show_new_dialog,
                                    &mut self.fm_new_name,
                                    &mut self.fm_new_is_dir,
                                    &mut self.fm_rename_target,
                                    &mut self.fm_rename_buffer,
                                    &mut self.fm_selected_path,
                                    &mut self.file_drag_path,
                                    &mut self.sidebar_favorites,
                                    &mut self.file_tags,
                                    &Self::parse_tag_labels(&self.app_settings),
                                    &Self::load_custom_smart_folders(&self.app_settings),
                                    &mut self.selected_tag_filters,
                                    &mut self.tag_filter_match_all,
                                    &self.fm_tabs,
                                    self.fm_active_tab,
                                    &mut self.fm_drag_tab_index,
                                    &mut self.fm_tab_scroll,
                                    self.show_file_sidebar,
                                    self.show_file_path_bar,
                                    self.show_file_preview_pane,
                                    self.show_file_status_bar,
                                    &mut self.fm_toolbar_search,
                                    &mut self.fm_view_mode,
                                    &mut self.fm_sort_field,
                                    &mut self.fm_icon_scale,
                                    &mut self.file_info_target,
                                ) {
                                    fm_navigate = Some(nav);
                                }
                            }
                            WindowKind::Controls => Self::content_controls(ui),
                            WindowKind::Messages => self.content_messages(ui),
                            WindowKind::Browser => self.content_browser(ui),
                            WindowKind::Calculator => Self::content_calculator(
                                ui,
                                &mut self.calc_display,
                                &mut self.calc_operand,
                                &mut self.calc_operator,
                                &mut self.calc_reset_next,
                                &mut self.calc_mode,
                                &mut self.calc_history,
                                &mut self.calc_memory,
                                &mut self.calc_degrees,
                                &mut self.calc_programmer_base,
                                &mut self.calc_programmer_operand,
                                &mut self.calc_programmer_operator,
                            ),
                            WindowKind::Notes => {
                                if Self::content_notes(ui, &mut self.notes_text) {
                                    self.show_emoji_picker = true;
                                    self.emoji_query.clear();
                                }
                                self.sync_active_tab_from_globals(WindowKind::Notes);
                            }
                            WindowKind::MusicPlayer => self.content_music(ui),
                            WindowKind::Photos => self.content_photos(ui),
                            WindowKind::Calendar => self.content_calendar(ui),
                            WindowKind::TextEditor => {
                                if Self::content_text_editor(
                                    ui,
                                    &self.editor_file_path,
                                    &mut self.editor_content,
                                    &mut self.editor_modified,
                                ) {
                                    self.show_emoji_picker = true;
                                    self.emoji_query.clear();
                                }
                                self.sync_active_tab_from_globals(WindowKind::TextEditor);
                            }
                            WindowKind::Settings => self.content_settings(ui),
                            WindowKind::ProcessManager => self.content_process_manager(ui),
                            WindowKind::Trash => {
                                if let Some(nav) = Self::content_trash(ui) {
                                    fm_navigate = Some(nav);
                                }
                            }
                            WindowKind::NetworkDiagnostics => {
                                self.network_diagnostics.render(ui, &self.clipboard)
                            }
                            WindowKind::DiskUtility => self.disk_utility.render(ui),
                            WindowKind::Dictionary => self.dictionary_app.render(ui),
                            WindowKind::Console => {
                                if self.proc_manager.is_none() {
                                    self.proc_manager = Some(ProcessManager::new());
                                }
                                let process_snapshot = self
                                    .proc_manager
                                    .as_ref()
                                    .map(|pm| pm.list_sorted_by_cpu())
                                    .unwrap_or_default();
                                let telemetry_snapshot = ConsoleTelemetrySnapshot {
                                    connected: self.telemetry.connected,
                                    status: self.telemetry.status.clone(),
                                    health: self.telemetry.health.clone(),
                                    uptime: self.telemetry.uptime.clone(),
                                    last_error: self.telemetry.last_error.clone(),
                                    network_name: if self.sysinfo.network_up {
                                        self.sysinfo.network_name.clone()
                                    } else {
                                        "disconnected".to_string()
                                    },
                                    process_count: self.sysinfo.process_count,
                                };
                                let toast_snapshot =
                                    self.toast_manager.visible().collect::<Vec<_>>();
                                self.console_app.render(
                                    ui,
                                    &telemetry_snapshot,
                                    self.notification_center.all(),
                                    &toast_snapshot,
                                    &process_snapshot,
                                    &dirs_home().join(".aurora_logs"),
                                );
                            }
                            WindowKind::FontBook => self.font_book.render(ui),
                            WindowKind::ColorPicker => {
                                if self.color_picker.render(ui, &self.clipboard) {
                                    self.app_settings.color_picker_saved_colors =
                                        self.color_picker.serialized_favorites();
                                    let _ = self.app_settings.save();
                                }
                            }
                        }
                    }); // end inner padding Frame

                // Claim any remaining space so the Resize widget doesn't
                // auto-shrink back to content height after user resizes.
                let remaining = ui.available_size();
                if remaining.y > 1.0 {
                    ui.allocate_space(remaining);
                }
            });

            let mut should_bring = false;
            if let Some(inner) = response {
                should_bring = inner.response.clicked() || inner.response.dragged();
                let win_rect = inner.response.rect;

                // Only track position/size when freely positioned (not maximized/snapped)
                // and only when user is interacting (drag/resize)
                if !is_fixed && (inner.response.dragged() || inner.response.changed()) {
                    let win = self.window_mut(kind);
                    win.default_pos = win_rect.min;
                    win.default_size = win_rect.size();
                }

                // Edge resize from all sides (N, S, E, W, corners)
                if !is_fixed {
                    let edge = 5.0; // hit zone size
                    let min_w = 200.0;
                    let min_h = 150.0;
                    let pointer = ctx.input(|i| i.pointer.hover_pos());
                    let dragging = ctx.input(|i| i.pointer.is_decidedly_dragging());
                    let delta = ctx.input(|i| i.pointer.delta());

                    if let Some(pos) = pointer {
                        let near_left = (pos.x - win_rect.left()).abs() < edge
                            && pos.y > win_rect.top()
                            && pos.y < win_rect.bottom();
                        let near_right = (pos.x - win_rect.right()).abs() < edge
                            && pos.y > win_rect.top()
                            && pos.y < win_rect.bottom();
                        let near_top = (pos.y - win_rect.top()).abs() < edge
                            && pos.x > win_rect.left()
                            && pos.x < win_rect.right();
                        let near_bottom = (pos.y - win_rect.bottom()).abs() < edge
                            && pos.x > win_rect.left()
                            && pos.x < win_rect.right();

                        // Corner detection
                        let near_tl = (pos.x - win_rect.left()).abs() < edge * 2.0
                            && (pos.y - win_rect.top()).abs() < edge * 2.0;
                        let near_tr = (pos.x - win_rect.right()).abs() < edge * 2.0
                            && (pos.y - win_rect.top()).abs() < edge * 2.0;
                        let near_bl = (pos.x - win_rect.left()).abs() < edge * 2.0
                            && (pos.y - win_rect.bottom()).abs() < edge * 2.0;

                        let on_edge = near_left
                            || near_right
                            || near_top
                            || near_bottom
                            || near_tl
                            || near_tr
                            || near_bl;

                        if on_edge {
                            // Set cursor style
                            let cursor = if near_tl || near_tr || near_bl {
                                egui::CursorIcon::Crosshair
                            } else if near_left || near_right {
                                egui::CursorIcon::ResizeHorizontal
                            } else if near_top || near_bottom {
                                egui::CursorIcon::ResizeVertical
                            } else {
                                egui::CursorIcon::Default
                            };
                            ctx.set_cursor_icon(cursor);

                            if dragging && (delta.x != 0.0 || delta.y != 0.0) {
                                let win = self.window_mut(kind);
                                let mut pos = win.default_pos;
                                let mut size = win.default_size;

                                if near_tl {
                                    let new_w = (size.x - delta.x).max(min_w);
                                    let new_h = (size.y - delta.y).max(min_h);
                                    pos.x += size.x - new_w;
                                    pos.y += size.y - new_h;
                                    size.x = new_w;
                                    size.y = new_h;
                                } else if near_tr {
                                    let new_w = (size.x + delta.x).max(min_w);
                                    let new_h = (size.y - delta.y).max(min_h);
                                    pos.y += size.y - new_h;
                                    size.x = new_w;
                                    size.y = new_h;
                                } else if near_bl {
                                    let new_w = (size.x - delta.x).max(min_w);
                                    let new_h = (size.y + delta.y).max(min_h);
                                    pos.x += size.x - new_w;
                                    size.x = new_w;
                                    size.y = new_h;
                                } else if near_left {
                                    let new_w = (size.x - delta.x).max(min_w);
                                    pos.x += size.x - new_w;
                                    size.x = new_w;
                                } else if near_right {
                                    size.x = (size.x + delta.x).max(min_w);
                                } else if near_top {
                                    let new_h = (size.y - delta.y).max(min_h);
                                    pos.y += size.y - new_h;
                                    size.y = new_h;
                                } else if near_bottom {
                                    size.y = (size.y + delta.y).max(min_h);
                                }

                                win.default_pos = pos;
                                win.default_size = size;
                                win.id_epoch = win.id_epoch.saturating_add(1);
                                should_bring = true;
                            }
                        }
                    }
                }
            }

            if close_clicked {
                // Unsaved changes guard for TextEditor
                if kind == WindowKind::TextEditor && self.editor_modified {
                    self.confirm_close_window = Some(kind);
                } else {
                    let win = self.window_mut(kind);
                    win.closing = true;
                    win.close_anim_start = Some(Instant::now());
                }
            }
            let win = self.window_mut(kind);
            if minimize_clicked {
                win.minimizing = true;
                win.minimize_anim_start = Some(Instant::now());
            }
            if maximize_clicked {
                if win.maximized {
                    // Un-maximize: restore previous rect
                    win.maximized = false;
                    if let Some(r) = win.restore_rect.take() {
                        win.default_pos = r.min;
                        win.default_size = r.size();
                    }
                } else {
                    // Maximize: save current rect for restore
                    win.restore_rect = Some(Rect::from_min_size(win.default_pos, win.default_size));
                    win.maximized = true;
                    win.snap = None;
                }
                win.id_epoch = win.id_epoch.saturating_add(1);
            }
            if should_bring {
                bring_front = Some(kind);
            }
        }

        if let Some(kind) = bring_front {
            self.bring_to_front(kind);
        }

        // Handle file manager navigation
        if let Some(path) = fm_navigate {
            let path_str = path.to_string_lossy();
            if let Some(msg) = path_str.strip_prefix("__NOTIFY_OK__") {
                self.notification_center
                    .notify("Files", msg, "", Color32::from_rgb(52, 199, 89));
                self.fm_entries = read_directory(&self.fm_current_dir);
                self.desktop_entries = read_directory(&desktop_directory());
                self.sync_active_tab_from_file_manager();
            } else if let Some(msg) = path_str.strip_prefix("__NOTIFY_ERR__") {
                self.notification_center.notify(
                    "Files",
                    "Error",
                    msg,
                    Color32::from_rgb(255, 59, 48),
                );
            } else if path_str == "__NEW_TAB__" {
                self.open_file_manager_tab(dirs_home());
            } else if path_str == "__FM_BACK__" {
                if let Some(previous) = self.fm_back_history.pop() {
                    self.fm_forward_history.push(self.fm_current_dir.clone());
                    self.navigate_file_manager_to(previous, false);
                }
            } else if path_str == "__FM_FORWARD__" {
                if let Some(next) = self.fm_forward_history.pop() {
                    self.fm_back_history.push(self.fm_current_dir.clone());
                    self.navigate_file_manager_to(next, false);
                }
            } else if let Some(dir_path) = path_str.strip_prefix("__OPEN_TAB__") {
                self.open_file_manager_tab(PathBuf::from(dir_path));
            } else if let Some(view) = path_str.strip_prefix("__SET_VIEW__") {
                let mode = match view {
                    "icon" => Some(FileManagerViewMode::Icon),
                    "list" => Some(FileManagerViewMode::List),
                    "column" => Some(FileManagerViewMode::Column),
                    "gallery" => Some(FileManagerViewMode::Gallery),
                    _ => None,
                };
                if let Some(mode) = mode {
                    self.set_file_manager_view_mode(mode);
                }
            } else if let Some(kind) = path_str.strip_prefix("__SMART_FOLDER__") {
                if self.fm_current_dir != PathBuf::from(self.smart_folder_title(kind)) {
                    self.fm_back_history.push(self.fm_current_dir.clone());
                    self.fm_forward_history.clear();
                }
                let root = dirs_home();
                self.fm_current_dir = PathBuf::from(self.smart_folder_title(kind));
                self.fm_entries = self.smart_folder_entries_for_token(kind, &root, &self.file_tags);
                self.fm_selected_path = None;
                self.sync_active_tab_from_file_manager();
            } else if let Some(idx_str) = path_str.strip_prefix("__SWITCH_TAB__") {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    self.switch_file_manager_tab(idx);
                }
            } else if let Some(idx_str) = path_str.strip_prefix("__CLOSE_TAB__") {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    self.close_file_manager_tab(idx);
                }
            } else if let Some(move_args) = path_str.strip_prefix("__MOVE_TAB__") {
                if let Some((from, to)) = move_args.split_once(':') {
                    if let (Ok(from), Ok(to)) = (from.parse::<usize>(), to.parse::<usize>()) {
                        self.move_file_manager_tab(from, to);
                    }
                }
            } else if path_str == "__REFRESH__" {
                self.fm_entries = read_directory(&self.fm_current_dir);
                self.sync_active_tab_from_file_manager();
            } else if let Some(file_path) = path_str.strip_prefix("__OPEN_EDITOR__") {
                self.open_file_in_editor(PathBuf::from(file_path));
            } else if let Some(file_path) = path_str.strip_prefix("__OPEN_MUSIC__") {
                self.open_audio_path_in_music_player(PathBuf::from(file_path));
            } else if let Some(file_path) = path_str.strip_prefix("__OPEN_VIDEO__") {
                self.open_video_path_in_quick_look(PathBuf::from(file_path));
            } else {
                self.navigate_file_manager_to(path.clone(), true);
            }
        }

        // Handle terminal command
        if let Some(cmd) = term_cmd {
            let output = Self::execute_terminal_command(&cmd, &si_snap);
            if output.len() == 1 && output[0].0 == "__CLEAR__" {
                self.terminal_output.clear();
            } else {
                for (line, color) in output {
                    if let Some(file_path) = line.strip_prefix("__OPEN_MUSIC__") {
                        self.open_audio_path_in_music_player(PathBuf::from(file_path));
                    } else if let Some(file_path) = line.strip_prefix("__OPEN_VIDEO__") {
                        self.open_video_path_in_quick_look(PathBuf::from(file_path));
                    } else if let Some(url) = line.strip_prefix("__OPEN_BROWSER__") {
                        self.open_url_in_browser(url);
                    } else {
                        self.terminal_output.push((line, color));
                    }
                }
            }
            self.sync_active_tab_from_globals(WindowKind::Terminal);
        }
    }

    // ── Dock ─────────────────────────────────────────────────────────────────

    fn render_dock(&mut self, ctx: &egui::Context) {
        let mut open_window: Option<WindowKind> = None;
        let mut dock_drop_icon: Option<DockIcon> = None;
        let drag_path = self.file_drag_path.clone();
        let screen = ctx.content_rect();
        let dock_position = Self::dock_position(&self.app_settings);
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let dock_hovered = Self::dock_hovered_for_position(dock_position, screen, pointer);
        let hover_elapsed = self.update_dock_hover_state(dock_hovered, Instant::now());
        let hidden_factor = Self::dock_hidden_offset(
            self.app_settings.dock_auto_hide,
            dock_hovered,
            hover_elapsed,
        );

        if let Some((_, t)) = self.dock_bounce {
            if t.elapsed() > Duration::from_millis(800) {
                self.dock_bounce = None;
            }
        }

        egui::Area::new(Id::new("dock"))
            .fixed_pos(Self::dock_panel_rect(screen, dock_position, hidden_factor).min)
            .order(Order::Foreground)
            .show(ctx, |ui| {
                ui.set_min_size(Self::dock_panel_rect(screen, dock_position, hidden_factor).size());
                let icons = DockIcon::all();
                let n_real = icons.iter().filter(|i| !i.is_separator()).count();
                let base_size = self.app_settings.dock_icon_size.clamp(32.0, 96.0);
                let total_base_w = n_real as f32 * (base_size + 3.0) + 12.0;
                let total_base_h = n_real as f32 * (base_size + 3.0) + 12.0;
                let dock_bounds = ui.available_rect_before_wrap();
                let dock_start_x = (dock_bounds.width() - total_base_w) / 2.0;
                let dock_start_y = ((dock_bounds.height() - total_base_h) / 2.0).max(8.0);

                let mut sizes: Vec<f32> = Vec::with_capacity(icons.len());
                let mut cum_primary = if dock_position == DockPosition::Bottom {
                    dock_start_x
                } else {
                    dock_start_y
                };
                for icon in icons {
                    if icon.is_separator() {
                        sizes.push(2.0);
                        cum_primary += 10.0;
                        continue;
                    }
                    let center_primary = cum_primary + base_size / 2.0;
                    let size = match pointer {
                        Some(pos) if dock_hovered && self.app_settings.dock_magnification => {
                            let dist = if dock_position == DockPosition::Bottom {
                                (pos.x - center_primary).abs()
                            } else {
                                (pos.y - center_primary).abs()
                            };
                            if dist < DOCK_EFFECT_DIST {
                                let ratio = 1.0 - dist / DOCK_EFFECT_DIST;
                                base_size * (1.0 + (DOCK_ICON_MAX_SCALE - 1.0) * ratio.powf(1.5))
                            } else {
                                base_size
                            }
                        }
                        _ => base_size,
                    };
                    sizes.push(size);
                    cum_primary += base_size + 3.0;
                }

                let frame_inner = egui::Frame::default()
                    .fill(if self.app_settings.dark_mode {
                        Color32::from_rgba_unmultiplied(20, 20, 22, 180)
                    } else {
                        Color32::from_rgba_unmultiplied(40, 40, 40, 90)
                    })
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(76)))
                    .corner_radius(CornerRadius::same(16))
                    .inner_margin(egui::Margin::symmetric(8, 4));
                let outer_layout = match dock_position {
                    DockPosition::Bottom => Layout::bottom_up(Align::Center),
                    DockPosition::Left => Layout::left_to_right(Align::Center),
                    DockPosition::Right => Layout::right_to_left(Align::Center),
                };
                ui.with_layout(outer_layout, |ui| {
                    ui.add_space(8.0);
                    frame_inner.show(ui, |ui| {
                        let icon_layout = match dock_position {
                            DockPosition::Bottom => Layout::left_to_right(Align::Max),
                            DockPosition::Left | DockPosition::Right => {
                                Layout::top_down(Align::Center)
                            }
                        };
                        ui.with_layout(icon_layout, |ui| {
                            ui.spacing_mut().item_spacing = if dock_position == DockPosition::Bottom
                            {
                                Vec2::new(3.0, 0.0)
                            } else {
                                Vec2::new(0.0, 3.0)
                            };
                            for (i, icon) in icons.iter().enumerate() {
                                if icon.is_separator() {
                                    let sep_size = if dock_position == DockPosition::Bottom {
                                        Vec2::new(1.0, 40.0)
                                    } else {
                                        Vec2::new(40.0, 1.0)
                                    };
                                    let (sr, _) = ui.allocate_exact_size(sep_size, Sense::hover());
                                    ui.painter().rect_filled(
                                        sr,
                                        0.0,
                                        Color32::from_white_alpha(50),
                                    );
                                    continue;
                                }
                                let mut size = sizes[i];
                                let mut bounce_offset = Vec2::ZERO;
                                if let Some((bi, bt)) = self.dock_bounce {
                                    if *icon == bi {
                                        let e = bt.elapsed().as_secs_f32();
                                        // macOS-style bounce: two bounces with decreasing amplitude
                                        let bounce = if e < 0.15 {
                                            // Rise up
                                            (e / 0.15).powf(0.5) * 18.0
                                        } else if e < 0.35 {
                                            // Fall and first bounce
                                            let t = (e - 0.15) / 0.20;
                                            18.0 * (1.0 - t).max(0.0)
                                                * (std::f32::consts::PI * t).sin().abs()
                                        } else if e < 0.55 {
                                            // Second smaller bounce
                                            let t = (e - 0.35) / 0.20;
                                            8.0 * (1.0 - t).max(0.0)
                                                * (std::f32::consts::PI * t).sin().abs()
                                        } else if e < 0.70 {
                                            // Tiny third bounce
                                            let t = (e - 0.55) / 0.15;
                                            3.0 * (1.0 - t).max(0.0)
                                                * (std::f32::consts::PI * t).sin().abs()
                                        } else {
                                            0.0
                                        };
                                        bounce_offset = match dock_position {
                                            DockPosition::Bottom => Vec2::new(0.0, -bounce),
                                            DockPosition::Left => Vec2::new(bounce, 0.0),
                                            DockPosition::Right => Vec2::new(-bounce, 0.0),
                                        };
                                        size += bounce * 0.15; // slight scale with bounce
                                    }
                                }
                                let (icon_rect, response) =
                                    ui.allocate_exact_size(Vec2::splat(size), Sense::click());
                                let icon_rect = icon_rect.translate(bounce_offset);
                                let hovered_drop = drag_path.is_some() && response.hovered();
                                if hovered_drop {
                                    let drop_color = if Self::dock_icon_accepts_file_drop(*icon) {
                                        if *icon == DockIcon::Trash {
                                            Color32::from_rgba_unmultiplied(255, 59, 48, 190)
                                        } else {
                                            Color32::from_rgba_unmultiplied(0, 122, 255, 190)
                                        }
                                    } else {
                                        Color32::from_rgba_unmultiplied(255, 59, 48, 160)
                                    };
                                    ui.painter().rect_stroke(
                                        icon_rect.expand(5.0),
                                        CornerRadius::same(12),
                                        Stroke::new(2.0, drop_color),
                                        StrokeKind::Outside,
                                    );
                                    if Self::dock_icon_accepts_file_drop(*icon) {
                                        dock_drop_icon = Some(*icon);
                                    }
                                }
                                paint_dock_icon(ui.painter(), icon_rect, *icon);
                                if let Some(wk) = icon.window_kind() {
                                    if self.app_settings.dock_show_running_indicators
                                        && self.window_ref(wk).open
                                        && !self.window_ref(wk).minimized
                                    {
                                        let indicator_center = match dock_position {
                                            DockPosition::Bottom => Pos2::new(
                                                icon_rect.center().x,
                                                icon_rect.bottom() + 4.0,
                                            ),
                                            DockPosition::Left => Pos2::new(
                                                icon_rect.right() + 4.0,
                                                icon_rect.center().y,
                                            ),
                                            DockPosition::Right => Pos2::new(
                                                icon_rect.left() - 4.0,
                                                icon_rect.center().y,
                                            ),
                                        };
                                        ui.painter().circle_filled(
                                            indicator_center,
                                            2.5,
                                            Color32::from_white_alpha(200),
                                        );
                                    }
                                }
                                let clicked = response.clicked();
                                response.on_hover_text(icon.label());
                                if clicked {
                                    if *icon == DockIcon::Launchpad {
                                        self.show_launchpad = !self.show_launchpad;
                                        self.launchpad_query.clear();
                                        self.launchpad_page = 0;
                                        self.dock_bounce = Some((*icon, Instant::now()));
                                    } else if *icon == DockIcon::Store {
                                        // Toggle downloads stack
                                        self.show_downloads_stack = !self.show_downloads_stack;
                                        if self.show_downloads_stack
                                            && self.recent_downloads.is_empty()
                                        {
                                            // Scan Downloads folder
                                            let dl_dir = dirs_home().join("Downloads");
                                            if dl_dir.exists() {
                                                if let Ok(entries) = fs::read_dir(&dl_dir) {
                                                    let mut files: Vec<(
                                                        String,
                                                        std::time::SystemTime,
                                                    )> = entries
                                                        .filter_map(|e| e.ok())
                                                        .filter(|e| {
                                                            e.file_type()
                                                                .map(|t| t.is_file())
                                                                .unwrap_or(false)
                                                        })
                                                        .filter_map(|e| {
                                                            let name = e
                                                                .file_name()
                                                                .to_string_lossy()
                                                                .to_string();
                                                            let modified = e
                                                                .metadata()
                                                                .ok()?
                                                                .modified()
                                                                .ok()?;
                                                            Some((name, modified))
                                                        })
                                                        .collect();
                                                    files.sort_by(|a, b| b.1.cmp(&a.1)); // newest first
                                                    self.recent_downloads = files
                                                        .into_iter()
                                                        .take(10)
                                                        .map(|(n, _)| n)
                                                        .collect();
                                                }
                                            }
                                        }
                                        self.dock_bounce = Some((*icon, Instant::now()));
                                    } else if let Some(wk) = icon.window_kind() {
                                        open_window = Some(wk);
                                        self.dock_bounce = Some((*icon, Instant::now()));
                                    }
                                }
                            }
                        });
                    });
                });
            });

        if let Some(kind) = open_window {
            let win = self.window_mut(kind);
            win.restore();
            win.id_epoch = win.id_epoch.saturating_add(1);
            self.bring_to_front(kind);
        }

        if let Some(drag_path) = drag_path {
            if !ctx.input(|i| i.pointer.any_down()) {
                if let Some(icon) = dock_drop_icon {
                    match icon {
                        DockIcon::Trash => match delete_entry(&drag_path) {
                            Ok(()) => {
                                self.notification_center.notify(
                                    "Files",
                                    "Moved",
                                    "Item moved to Trash",
                                    Color32::from_rgb(52, 199, 89),
                                );
                            }
                            Err(err) => {
                                self.notification_center.notify(
                                    "Files",
                                    "Move failed",
                                    &err,
                                    Color32::from_rgb(255, 59, 48),
                                );
                            }
                        },
                        _ => {
                            let _ = open_file_with_system(&drag_path);
                            self.notification_center.notify(
                                "Files",
                                "Opened",
                                &format!("Opened with {}", icon.label()),
                                Color32::from_rgb(52, 199, 89),
                            );
                            if let Some(wk) = icon.window_kind() {
                                let win = self.window_mut(wk);
                                win.restore();
                                win.id_epoch = win.id_epoch.saturating_add(1);
                                self.bring_to_front(wk);
                            }
                        }
                    }
                    self.fm_entries = read_directory(&self.fm_current_dir);
                    self.desktop_entries = read_directory(&desktop_directory());
                    self.fm_selected_path = None;
                    self.desktop_selected_paths.clear();
                    self.file_drag_path = None;
                }
            }
        }
    }

    // ── Downloads Stack ────────────────────────────────────────────────────

    fn render_downloads_stack(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        let popup_w = 260.0;
        let dock_position = Self::dock_position(&self.app_settings);
        let (pos, pivot) = match dock_position {
            DockPosition::Bottom => (
                Pos2::new(
                    screen.right() - popup_w - 80.0,
                    screen.bottom() - DOCK_HEIGHT - 8.0,
                ),
                Align2::LEFT_BOTTOM,
            ),
            DockPosition::Left => (
                Pos2::new(DOCK_HEIGHT + 16.0, screen.bottom() - 24.0),
                Align2::LEFT_BOTTOM,
            ),
            DockPosition::Right => (
                Pos2::new(screen.right() - DOCK_HEIGHT - 16.0, screen.bottom() - 24.0),
                Align2::RIGHT_BOTTOM,
            ),
        };

        let dl_dir = dirs_home().join("Downloads");
        let files = self.recent_downloads.clone();

        egui::Area::new(Id::new("downloads_stack"))
            .fixed_pos(pos)
            .pivot(pivot)
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(35, 35, 40, 230))
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(popup_w - 24.0);
                        ui.label(
                            RichText::new("Downloads")
                                .size(13.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.add_space(6.0);

                        if files.is_empty() {
                            ui.label(
                                RichText::new("No recent files")
                                    .size(12.0)
                                    .color(Color32::from_gray(120)),
                            );
                        } else {
                            egui::ScrollArea::vertical()
                                .max_height(240.0)
                                .show(ui, |ui| {
                                    for name in &files {
                                        let ext = name.rsplit('.').next().unwrap_or("");
                                        let ext_color = match ext {
                                            "pdf" => Color32::from_rgb(255, 59, 48),
                                            "zip" | "rar" | "7z" => Color32::from_rgb(88, 86, 214),
                                            "exe" | "msi" => Color32::from_rgb(142, 142, 147),
                                            "png" | "jpg" | "jpeg" | "gif" => {
                                                Color32::from_rgb(0, 122, 255)
                                            }
                                            "mp3" | "wav" | "flac" => {
                                                Color32::from_rgb(255, 55, 95)
                                            }
                                            "mp4" | "mkv" | "avi" => Color32::from_rgb(255, 149, 0),
                                            "doc" | "docx" => Color32::from_rgb(0, 112, 192),
                                            "xls" | "xlsx" => Color32::from_rgb(52, 199, 89),
                                            _ => Color32::from_gray(100),
                                        };
                                        let resp = egui::Frame::default()
                                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                            .corner_radius(CornerRadius::same(6))
                                            .inner_margin(egui::Margin::symmetric(8, 4))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    let (dot_r, _) = ui.allocate_exact_size(
                                                        Vec2::splat(8.0),
                                                        Sense::hover(),
                                                    );
                                                    ui.painter().circle_filled(
                                                        dot_r.center(),
                                                        4.0,
                                                        ext_color,
                                                    );
                                                    let truncated = if name.len() > 30 {
                                                        format!("{}...", &name[..28])
                                                    } else {
                                                        name.clone()
                                                    };
                                                    ui.label(
                                                        RichText::new(truncated)
                                                            .size(12.0)
                                                            .color(Color32::from_gray(220)),
                                                    );
                                                });
                                            })
                                            .response;
                                        if resp.interact(Sense::click()).clicked() {
                                            let path = dl_dir.join(name);
                                            if !self.open_path_in_aurora_if_supported(&path) {
                                                open_file_with_system(&path);
                                            }
                                            self.show_downloads_stack = false;
                                        }
                                        ui.add_space(1.0);
                                    }
                                });
                        }

                        ui.add_space(6.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Open Downloads Folder")
                                        .size(11.0)
                                        .color(Color32::from_rgb(0, 122, 255)),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            open_file_with_system(&dl_dir);
                            self.show_downloads_stack = false;
                        }
                    });
            });

        // Click outside to dismiss
        if ctx.input(|i| i.pointer.primary_clicked()) {
            // Simple dismiss — the click on an item above already handles that case
            // We rely on the Area not consuming the click to dismiss
        }
    }

    // ── Control Center ───────────────────────────────────────────────────────

    fn render_control_center(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        egui::Area::new(Id::new("control_center"))
            .fixed_pos(Pos2::new(screen.right() - 340.0, MENU_BAR_HEIGHT + 8.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(30, 30, 30, 200))
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(egui::Margin::symmetric(14, 14))
                    .show(ui, |ui| {
                        ui.set_min_width(300.0);
                        let accent = self.accent_color();
                        egui::Grid::new("cc_toggles")
                            .num_columns(2)
                            .spacing(Vec2::new(8.0, 8.0))
                            .show(ui, |ui| {
                                // Wi-Fi with real network name
                                let wifi_label = if self.cc_wifi && self.sysinfo.network_up {
                                    format!("Wi-Fi\n{}", self.sysinfo.network_name)
                                } else {
                                    "Wi-Fi".to_string()
                                };
                                let wifi_fill = if self.cc_wifi {
                                    accent
                                } else {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 25)
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(wifi_label)
                                                .size(11.0)
                                                .color(Color32::WHITE),
                                        )
                                        .min_size(Vec2::new(140.0, 55.0))
                                        .fill(wifi_fill)
                                        .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                                        .corner_radius(CornerRadius::same(10)),
                                    )
                                    .clicked()
                                {
                                    self.cc_wifi = !self.cc_wifi;
                                }

                                let bt_fill = if self.cc_bluetooth {
                                    accent
                                } else {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 25)
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Bluetooth")
                                                .size(12.0)
                                                .color(Color32::WHITE),
                                        )
                                        .min_size(Vec2::new(140.0, 55.0))
                                        .fill(bt_fill)
                                        .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                                        .corner_radius(CornerRadius::same(10)),
                                    )
                                    .clicked()
                                {
                                    self.cc_bluetooth = !self.cc_bluetooth;
                                }
                                ui.end_row();

                                let ad_fill = if self.cc_airdrop {
                                    accent
                                } else {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 25)
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("AirDrop")
                                                .size(12.0)
                                                .color(Color32::WHITE),
                                        )
                                        .min_size(Vec2::new(140.0, 55.0))
                                        .fill(ad_fill)
                                        .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                                        .corner_radius(CornerRadius::same(10)),
                                    )
                                    .clicked()
                                {
                                    self.cc_airdrop = !self.cc_airdrop;
                                }

                                let focus_fill = if self.cc_focus {
                                    accent
                                } else {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 25)
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Focus").size(12.0).color(Color32::WHITE),
                                        )
                                        .min_size(Vec2::new(140.0, 55.0))
                                        .fill(focus_fill)
                                        .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                                        .corner_radius(CornerRadius::same(10)),
                                    )
                                    .clicked()
                                {
                                    self.cc_focus = !self.cc_focus;
                                }
                                ui.end_row();
                            });

                        // Real system stats
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let batt_str = if self.sysinfo.battery_available {
                                let icon = if self.sysinfo.battery_charging {
                                    "⚡"
                                } else {
                                    "🔋"
                                };
                                format!("{icon} {:.0}%", self.sysinfo.battery_pct)
                            } else {
                                "AC Power".to_string()
                            };
                            ui.label(
                                RichText::new(batt_str)
                                    .size(11.0)
                                    .color(Color32::from_gray(180)),
                            );
                            ui.label(
                                RichText::new(format!("CPU {:.0}%", self.sysinfo.cpu_usage))
                                    .size(11.0)
                                    .color(Color32::from_gray(140)),
                            );
                            ui.label(
                                RichText::new(format!("RAM {:.0}%", self.sysinfo.memory_pct))
                                    .size(11.0)
                                    .color(Color32::from_gray(140)),
                            );
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("*")
                                    .size(14.0)
                                    .color(Color32::from_rgb(255, 214, 10)),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.cc_brightness, 0.0..=1.0)
                                    .show_value(false)
                                    .text("Display"),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(")").size(14.0).color(Color32::WHITE));
                            ui.add(
                                egui::Slider::new(&mut self.cc_volume, 0.0..=1.0)
                                    .show_value(false)
                                    .text("Sound"),
                            );
                        });
                        ui.add_space(10.0);
                        egui::Frame::default()
                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 18))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (ar, _) =
                                        ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                                    gradient_rect(
                                        ui.painter(),
                                        ar,
                                        Color32::from_rgb(255, 107, 157),
                                        Color32::from_rgb(87, 75, 144),
                                    );
                                    ui.painter().rect_stroke(
                                        ar,
                                        CornerRadius::same(4),
                                        Stroke::new(0.5, Color32::from_white_alpha(30)),
                                        StrokeKind::Outside,
                                    );
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new("Aurora Ambient")
                                                .size(12.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        );
                                        ui.label(
                                            RichText::new("System Sounds")
                                                .size(11.0)
                                                .color(Color32::from_gray(160)),
                                        );
                                    });
                                });
                            });
                    });
            });
    }

    // ── Menu bar popup: Wi-Fi ────────────────────────────────────────────────

    fn render_wifi_popup(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        egui::Area::new(Id::new("wifi_popup"))
            .fixed_pos(Pos2::new(screen.right() - 260.0, MENU_BAR_HEIGHT + 4.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(30, 30, 30, 220))
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(220.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Wi-Fi")
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let label = if self.cc_wifi { "On" } else { "Off" };
                                let color = if self.cc_wifi {
                                    Color32::from_rgb(52, 199, 89)
                                } else {
                                    Color32::from_gray(120)
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(label).size(11.0).color(Color32::WHITE),
                                        )
                                        .fill(color)
                                        .corner_radius(CornerRadius::same(8))
                                        .min_size(Vec2::new(40.0, 20.0)),
                                    )
                                    .clicked()
                                {
                                    self.cc_wifi = !self.cc_wifi;
                                }
                            });
                        });
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);
                        if self.cc_wifi {
                            let networks = [
                                ("AuroraOS-5G", true, 4),
                                ("Neighbors_WiFi", false, 3),
                                ("CoffeeShop_Free", false, 2),
                                ("IoT_Network", false, 1),
                            ];
                            for (name, connected, strength) in networks {
                                ui.horizontal(|ui| {
                                    let bars = "▂▄▆█";
                                    let signal: String = bars.chars().take(strength).collect();
                                    ui.label(
                                        RichText::new(name)
                                            .size(12.0)
                                            .color(Color32::from_gray(220)),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(&signal)
                                                .size(10.0)
                                                .color(Color32::from_gray(160)),
                                        );
                                        if connected {
                                            ui.label(
                                                RichText::new("✓")
                                                    .size(12.0)
                                                    .color(Color32::from_rgb(0, 122, 255)),
                                            );
                                        }
                                    });
                                });
                                ui.add_space(2.0);
                            }
                        } else {
                            ui.label(
                                RichText::new("Wi-Fi is turned off")
                                    .size(12.0)
                                    .color(Color32::from_gray(120)),
                            );
                        }
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Network Preferences...")
                                .size(12.0)
                                .color(Color32::from_rgb(0, 122, 255)),
                        );
                    });
            });
    }

    fn render_battery_popup(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        egui::Area::new(Id::new("battery_popup"))
            .fixed_pos(Pos2::new(screen.right() - 280.0, MENU_BAR_HEIGHT + 4.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(30, 30, 30, 220))
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(240.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Battery")
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let label = if self.app_settings.low_power_mode {
                                    "Low Power On"
                                } else {
                                    "Low Power Off"
                                };
                                let fill = if self.app_settings.low_power_mode {
                                    Color32::from_rgb(255, 149, 0)
                                } else {
                                    Color32::from_gray(90)
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(label).size(11.0).color(Color32::WHITE),
                                        )
                                        .fill(fill)
                                        .corner_radius(CornerRadius::same(8))
                                        .min_size(Vec2::new(98.0, 20.0)),
                                    )
                                    .clicked()
                                {
                                    self.app_settings.low_power_mode =
                                        !self.app_settings.low_power_mode;
                                    let _ = self.app_settings.save();
                                }
                            });
                        });
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);

                        let source = if self.sysinfo.battery_charging {
                            "Power Source: Power Adapter"
                        } else {
                            "Power Source: Battery"
                        };
                        ui.label(
                            RichText::new(format!("{:.0}% charged", self.sysinfo.battery_pct))
                                .size(20.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.label(
                            RichText::new(source)
                                .size(11.0)
                                .color(Color32::from_gray(160)),
                        );
                        ui.label(
                            RichText::new(Self::battery_time_remaining_label(
                                self.sysinfo.battery_pct,
                                self.sysinfo.battery_charging,
                                self.app_settings.low_power_mode,
                            ))
                            .size(11.0)
                            .color(Color32::from_gray(180)),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Apps Using Significant Energy")
                                .size(11.0)
                                .color(Color32::from_gray(150)),
                        );
                        for app in if self.app_settings.low_power_mode {
                            vec!["Browser - Medium", "Photos - Low"]
                        } else {
                            vec!["Browser - High", "Music - Medium", "Embedded Apps - High"]
                        } {
                            ui.label(RichText::new(app).size(11.0).color(Color32::from_gray(210)));
                        }
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);
                        if ui.button("Battery Preferences...").clicked() {
                            let win = self.window_mut(WindowKind::Settings);
                            win.restore();
                            win.id_epoch = win.id_epoch.saturating_add(1);
                            self.bring_to_front(WindowKind::Settings);
                            self.show_battery_popup = false;
                        }
                    });
            });
    }

    // ── Menu bar popup: Volume ───────────────────────────────────────────────

    fn render_volume_popup(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        egui::Area::new(Id::new("volume_popup"))
            .fixed_pos(Pos2::new(screen.right() - 200.0, MENU_BAR_HEIGHT + 4.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(30, 30, 30, 220))
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(180.0);
                        ui.label(
                            RichText::new("Sound")
                                .size(13.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("✕").size(11.0).color(Color32::from_gray(140)));
                            ui.add(
                                egui::Slider::new(&mut self.cc_volume, 0.0..=1.0).show_value(false),
                            );
                            ui.label(RichText::new("♪").size(11.0).color(Color32::from_gray(140)));
                        });
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Output Device")
                                .size(11.0)
                                .color(Color32::from_gray(140)),
                        );
                        ui.add_space(2.0);
                        let devices = ["Built-in Speakers", "AuroraOS Audio"];
                        for (i, dev) in devices.iter().enumerate() {
                            ui.horizontal(|ui| {
                                if i == 0 {
                                    ui.label(
                                        RichText::new("✓")
                                            .size(12.0)
                                            .color(Color32::from_rgb(0, 122, 255)),
                                    );
                                } else {
                                    ui.add_space(16.0);
                                }
                                ui.label(
                                    RichText::new(*dev)
                                        .size(12.0)
                                        .color(Color32::from_gray(220)),
                                );
                            });
                        }
                    });
            });
    }

    // ── Menu bar popup: Bluetooth ────────────────────────────────────────────

    fn render_bluetooth_popup(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        egui::Area::new(Id::new("bluetooth_popup"))
            .fixed_pos(Pos2::new(screen.right() - 230.0, MENU_BAR_HEIGHT + 4.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(30, 30, 30, 220))
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(200.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Bluetooth")
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let label = if self.cc_bluetooth { "On" } else { "Off" };
                                let color = if self.cc_bluetooth {
                                    Color32::from_rgb(0, 122, 255)
                                } else {
                                    Color32::from_gray(120)
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(label).size(11.0).color(Color32::WHITE),
                                        )
                                        .fill(color)
                                        .corner_radius(CornerRadius::same(8))
                                        .min_size(Vec2::new(40.0, 20.0)),
                                    )
                                    .clicked()
                                {
                                    self.cc_bluetooth = !self.cc_bluetooth;
                                }
                            });
                        });
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);
                        if self.cc_bluetooth {
                            let devices = [
                                ("AirPods Pro", "Connected"),
                                ("Magic Mouse", "Not Connected"),
                                ("Magic Keyboard", "Not Connected"),
                            ];
                            for (name, status) in devices {
                                ui.horizontal(|ui| {
                                    let connected = status == "Connected";
                                    let color = if connected {
                                        Color32::from_gray(220)
                                    } else {
                                        Color32::from_gray(140)
                                    };
                                    ui.label(RichText::new(name).size(12.0).color(color));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(status)
                                                .size(10.0)
                                                .color(Color32::from_gray(100)),
                                        );
                                    });
                                });
                                ui.add_space(2.0);
                            }
                        } else {
                            ui.label(
                                RichText::new("Bluetooth is turned off")
                                    .size(12.0)
                                    .color(Color32::from_gray(120)),
                            );
                        }
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Bluetooth Preferences...")
                                .size(12.0)
                                .color(Color32::from_rgb(0, 122, 255)),
                        );
                    });
            });
    }

    // ── Notification Center ──────────────────────────────────────────────────

    fn render_notification_center(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        let overlay_painter = ctx.layer_painter(egui::LayerId::new(
            Order::Foreground,
            Id::new("notification_center_bg"),
        ));
        overlay_painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 70));
        // Mark all as read when opening
        self.notification_center.mark_all_read();

        let slide_t = (Instant::now()
            .duration_since(
                self.notification_panel_opened_at
                    .unwrap_or_else(Instant::now),
            )
            .as_secs_f32()
            .min(0.3)
            / 0.3)
            .clamp(0.0, 1.0);
        let panel_x = screen.right() - 16.0 - (350.0 * slide_t);
        let today = Local::now();
        let weather_summary = format!("{}°C · Clear", ((self.cc_brightness * 10.0) + 18.0) as i32);
        let groups = self
            .notification_center
            .grouped_by_app()
            .into_iter()
            .map(|(app, items)| {
                (
                    app,
                    items
                        .into_iter()
                        .map(|item| {
                            (
                                item.title.clone(),
                                item.body.clone(),
                                item.time_ago(),
                                item.color,
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        egui::Area::new(Id::new("notification_center"))
            .fixed_pos(Pos2::new(panel_x, MENU_BAR_HEIGHT + 8.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(30, 30, 30, 210))
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(egui::Margin::symmetric(14, 14))
                    .show(ui, |ui| {
                        ui.set_min_width(350.0);
                        ui.label(
                            RichText::new(today.format("%A").to_string())
                                .size(12.0)
                                .color(Color32::from_gray(160)),
                        );
                        ui.label(
                            RichText::new(today.format("%B %d").to_string())
                                .size(24.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.label(
                            RichText::new(&weather_summary)
                                .size(11.0)
                                .color(Color32::from_gray(170)),
                        );
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Notifications")
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Clear All")
                                                .size(11.0)
                                                .color(Color32::from_gray(160)),
                                        )
                                        .fill(Color32::TRANSPARENT)
                                        .stroke(Stroke::NONE),
                                    )
                                    .clicked()
                                {
                                    self.notification_center.clear();
                                }
                            });
                        });
                        ui.add_space(10.0);

                        if self.notification_center.is_empty() {
                            ui.label(
                                RichText::new("No notifications")
                                    .size(13.0)
                                    .color(Color32::from_gray(100)),
                            );
                        } else {
                            egui::ScrollArea::vertical()
                                .max_height(360.0)
                                .show(ui, |ui| {
                                    let mut clear_group: Option<String> = None;
                                    let mut toggle_group: Option<String> = None;
                                    for (app, items) in &groups {
                                        let collapsed =
                                            self.collapsed_notification_apps.contains(app);
                                        egui::Frame::default()
                                            .fill(Color32::from_rgba_unmultiplied(
                                                255, 255, 255, 18,
                                            ))
                                            .corner_radius(CornerRadius::same(10))
                                            .inner_margin(egui::Margin::symmetric(10, 8))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    let color = items
                                                        .first()
                                                        .map(|item| item.3)
                                                        .unwrap_or(Color32::from_gray(140));
                                                    let (dr, _) = ui.allocate_exact_size(
                                                        Vec2::splat(10.0),
                                                        Sense::hover(),
                                                    );
                                                    ui.painter().circle_filled(
                                                        dr.center(),
                                                        5.0,
                                                        color,
                                                    );
                                                    if ui
                                                        .button(
                                                            RichText::new(format!(
                                                                "{} ({})",
                                                                app,
                                                                items.len()
                                                            ))
                                                            .size(11.0)
                                                            .color(Color32::WHITE),
                                                        )
                                                        .clicked()
                                                    {
                                                        toggle_group = Some(app.clone());
                                                    }
                                                    ui.with_layout(
                                                        Layout::right_to_left(Align::Center),
                                                        |ui| {
                                                            if ui.small_button("Clear").clicked() {
                                                                clear_group = Some(app.clone());
                                                            }
                                                        },
                                                    );
                                                });
                                                if !collapsed {
                                                    ui.add_space(4.0);
                                                    for notif in items {
                                                        ui.label(
                                                            RichText::new(&notif.0)
                                                                .size(13.0)
                                                                .strong()
                                                                .color(Color32::WHITE),
                                                        );
                                                        ui.horizontal(|ui| {
                                                            ui.label(
                                                                RichText::new(&notif.1)
                                                                    .size(12.0)
                                                                    .color(Color32::from_gray(180)),
                                                            );
                                                            ui.with_layout(
                                                                Layout::right_to_left(
                                                                    Align::Center,
                                                                ),
                                                                |ui| {
                                                                    ui.label(
                                                                        RichText::new(&notif.2)
                                                                            .size(10.0)
                                                                            .color(
                                                                                Color32::from_gray(
                                                                                    100,
                                                                                ),
                                                                            ),
                                                                    );
                                                                },
                                                            );
                                                        });
                                                        ui.add_space(4.0);
                                                    }
                                                }
                                            });
                                        ui.add_space(6.0);
                                    }
                                    if let Some(app) = toggle_group {
                                        if !self.collapsed_notification_apps.insert(app.clone()) {
                                            self.collapsed_notification_apps.remove(&app);
                                        }
                                    }
                                    if let Some(app) = clear_group {
                                        self.notification_center.clear_app(&app);
                                        self.collapsed_notification_apps.remove(&app);
                                    }
                                });
                        }

                        ui.add_space(10.0);
                        ui.label(
                            RichText::new("Widgets")
                                .size(16.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.add_space(6.0);
                        egui::Frame::default()
                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 14))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("Weather")
                                        .size(12.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                );
                                ui.label(
                                    RichText::new(weather_summary)
                                        .size(11.0)
                                        .color(Color32::from_gray(180)),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new("System")
                                        .size(12.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{} unread notifications",
                                        self.notification_center.unread_count()
                                    ))
                                    .size(11.0)
                                    .color(Color32::from_gray(180)),
                                );
                            });
                        ui.add_space(8.0);
                        let _ = ui.button(
                            RichText::new("Edit Widgets")
                                .size(11.0)
                                .color(Color32::from_gray(210)),
                        );
                    });
            });
    }

    // ── Spotlight ────────────────────────────────────────────────────────────

    fn render_spotlight(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        let overlay_painter = ctx.layer_painter(egui::LayerId::new(
            Order::Foreground,
            Id::new("spotlight_bg"),
        ));
        overlay_painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 80));

        // Search real files
        let file_results = self.file_index.search(&self.spotlight_query, 8);

        egui::Area::new(Id::new("spotlight"))
            .fixed_pos(Pos2::new(
                screen.center().x - 280.0,
                screen.top() + screen.height() * 0.22,
            ))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(40, 40, 40, 220))
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(50)))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(540.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("O").size(18.0).color(Color32::from_gray(160)));
                            let te = egui::TextEdit::singleline(&mut self.spotlight_query)
                                .hint_text("Spotlight Search")
                                .font(FontId::proportional(20.0))
                                .text_color(Color32::WHITE)
                                .desired_width(480.0)
                                .frame(false);
                            let response = ui.add(te);
                            if !response.has_focus() {
                                response.request_focus();
                            }
                        });

                        if !self.spotlight_query.is_empty() {
                            ui.add_space(6.0);
                            let (sep_rect, _) =
                                ui.allocate_exact_size(Vec2::new(530.0, 1.0), Sense::hover());
                            ui.painter()
                                .rect_filled(sep_rect, 0.0, Color32::from_white_alpha(30));
                            ui.add_space(6.0);

                            // App results
                            let query = self.spotlight_query.to_lowercase();
                            let inline_result =
                                Self::spotlight_inline_result(&self.spotlight_query);
                            let apps: &[(&str, WindowKind)] = &[
                                ("System Overview", WindowKind::Overview),
                                ("Terminal", WindowKind::Terminal),
                                ("Files", WindowKind::FileManager),
                                ("Messages", WindowKind::Messages),
                                ("Browser", WindowKind::Browser),
                                ("Quick Controls", WindowKind::Controls),
                                ("Calculator", WindowKind::Calculator),
                                ("Notes", WindowKind::Notes),
                                ("Music", WindowKind::MusicPlayer),
                                ("Photos", WindowKind::Photos),
                                ("Calendar", WindowKind::Calendar),
                                ("Network Diagnostics", WindowKind::NetworkDiagnostics),
                                ("Disk Utility", WindowKind::DiskUtility),
                                ("Font Book", WindowKind::FontBook),
                                ("Color Picker", WindowKind::ColorPicker),
                                ("Dictionary", WindowKind::Dictionary),
                                ("Console", WindowKind::Console),
                            ];
                            let app_hits = apps
                                .iter()
                                .filter(|(name, _)| name.to_lowercase().contains(&query))
                                .count();
                            let contact_results =
                                self.spotlight_contact_hits(&self.spotlight_query);
                            let message_results =
                                self.spotlight_message_hits(&self.spotlight_query);
                            let calendar_results =
                                self.spotlight_calendar_hits(&self.spotlight_query);
                            let reminder_results =
                                Self::spotlight_reminder_hits(&self.spotlight_query);
                            let preference_results =
                                Self::spotlight_system_preference_hits(&self.spotlight_query);
                            let folder_results = file_results
                                .iter()
                                .filter(|path| std::path::Path::new(path).is_dir())
                                .cloned()
                                .collect::<Vec<_>>();
                            let document_results = file_results
                                .iter()
                                .filter(|path| std::path::Path::new(path).is_file())
                                .cloned()
                                .collect::<Vec<_>>();
                            if let Some(label) = Self::spotlight_top_hit_label(
                                &self.spotlight_query,
                                app_hits,
                                file_results.len(),
                                inline_result.is_some(),
                            ) {
                                ui.label(
                                    RichText::new(label)
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::from_gray(120)),
                                );
                                ui.add_space(2.0);
                                if let Some(result) = &inline_result {
                                    egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 14))
                                        .corner_radius(CornerRadius::same(8))
                                        .inner_margin(egui::Margin::symmetric(8, 6))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(&result.title)
                                                    .size(13.0)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            );
                                            ui.label(
                                                RichText::new(&result.body)
                                                    .size(12.0)
                                                    .color(Color32::from_gray(210)),
                                            );
                                        });
                                } else if let Some((name, wk)) = apps
                                    .iter()
                                    .find(|(name, _)| name.to_lowercase().contains(&query))
                                {
                                    let resp = egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 14))
                                        .corner_radius(CornerRadius::same(8))
                                        .inner_margin(egui::Margin::symmetric(8, 6))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(*name)
                                                    .size(13.0)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            );
                                            ui.label(
                                                RichText::new("Application")
                                                    .size(11.0)
                                                    .color(Color32::from_gray(120)),
                                            );
                                        })
                                        .response;
                                    if resp.interact(Sense::click()).clicked() {
                                        self.spotlight_open_window = Some(*wk);
                                    }
                                } else if let Some(path) = file_results.first() {
                                    let resp = egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 14))
                                        .corner_radius(CornerRadius::same(8))
                                        .inner_margin(egui::Margin::symmetric(8, 6))
                                        .show(ui, |ui| {
                                            let file_name = std::path::Path::new(path)
                                                .file_name()
                                                .and_then(|name| name.to_str())
                                                .unwrap_or(path);
                                            ui.label(
                                                RichText::new(file_name)
                                                    .size(13.0)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            );
                                            ui.label(
                                                RichText::new(path)
                                                    .size(10.0)
                                                    .color(Color32::from_gray(120)),
                                            );
                                        })
                                        .response;
                                    if resp.interact(Sense::click()).clicked() {
                                        self.spotlight_open_file = Some(PathBuf::from(path));
                                    }
                                } else if let Some(contact) = contact_results.first() {
                                    let resp = egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 14))
                                        .corner_radius(CornerRadius::same(8))
                                        .inner_margin(egui::Margin::symmetric(8, 6))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(&contact.name)
                                                    .size(13.0)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            );
                                            ui.label(
                                                RichText::new(&contact.email)
                                                    .size(11.0)
                                                    .color(Color32::from_gray(120)),
                                            );
                                        })
                                        .response;
                                    if resp.interact(Sense::click()).clicked() {
                                        self.spotlight_open_window = Some(WindowKind::Messages);
                                    }
                                } else if let Some(event) = calendar_results.first() {
                                    let resp = egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 14))
                                        .corner_radius(CornerRadius::same(8))
                                        .inner_margin(egui::Margin::symmetric(8, 6))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(&event.title)
                                                    .size(13.0)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} · {}",
                                                    event.time, event.details
                                                ))
                                                .size(11.0)
                                                .color(Color32::from_gray(120)),
                                            );
                                        })
                                        .response;
                                    if resp.interact(Sense::click()).clicked() {
                                        self.spotlight_open_window = Some(WindowKind::Calendar);
                                    }
                                }
                                ui.add_space(6.0);
                            }

                            if let Some(result) = &inline_result {
                                ui.label(
                                    RichText::new(match result.kind {
                                        SpotlightInlineKind::Calculation => "Calculation",
                                        SpotlightInlineKind::Conversion => "Conversion",
                                        SpotlightInlineKind::Definition => "Definition",
                                    })
                                    .size(11.0)
                                    .strong()
                                    .color(Color32::from_gray(120)),
                                );
                                ui.add_space(2.0);
                                egui::Frame::default()
                                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(egui::Margin::symmetric(8, 4))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(&result.body)
                                                .size(13.0)
                                                .color(Color32::WHITE),
                                        );
                                    });
                                ui.add_space(4.0);
                            }
                            let mut has_app = false;
                            for (name, wk) in apps {
                                if name.to_lowercase().contains(&query) {
                                    if !has_app {
                                        ui.label(
                                            RichText::new("Applications")
                                                .size(11.0)
                                                .strong()
                                                .color(Color32::from_gray(120)),
                                        );
                                        ui.add_space(2.0);
                                        has_app = true;
                                    }
                                    let resp = egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 12))
                                        .corner_radius(CornerRadius::same(6))
                                        .inner_margin(egui::Margin::symmetric(8, 4))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.set_min_width(510.0);
                                                ui.label(
                                                    RichText::new(*name)
                                                        .size(13.0)
                                                        .color(Color32::WHITE),
                                                );
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new("Application")
                                                                .size(11.0)
                                                                .color(Color32::from_gray(100)),
                                                        );
                                                    },
                                                );
                                            });
                                        })
                                        .response;
                                    if resp.interact(Sense::click()).clicked() {
                                        self.spotlight_open_window = Some(*wk);
                                    }
                                    ui.add_space(2.0);
                                }
                            }

                            if !contact_results.is_empty() {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Contacts")
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::from_gray(120)),
                                );
                                ui.add_space(2.0);
                                for contact in &contact_results {
                                    let resp = egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                        .corner_radius(CornerRadius::same(6))
                                        .inner_margin(egui::Margin::symmetric(8, 4))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.set_min_width(510.0);
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        RichText::new(&contact.name)
                                                            .size(13.0)
                                                            .color(Color32::WHITE),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{} · {}",
                                                            contact.email, contact.phone
                                                        ))
                                                        .size(10.0)
                                                        .color(Color32::from_gray(100)),
                                                    );
                                                });
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new("Contact")
                                                                .size(11.0)
                                                                .color(Color32::from_gray(100)),
                                                        );
                                                    },
                                                );
                                            });
                                        })
                                        .response;
                                    if resp.interact(Sense::click()).clicked() {
                                        self.spotlight_open_window = Some(WindowKind::Messages);
                                    }
                                    ui.add_space(1.0);
                                }
                            }

                            if !message_results.is_empty() {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Messages")
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::from_gray(120)),
                                );
                                ui.add_space(2.0);
                                for message in &message_results {
                                    let resp = egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                        .corner_radius(CornerRadius::same(6))
                                        .inner_margin(egui::Margin::symmetric(8, 4))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.set_min_width(510.0);
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        RichText::new(&message.contact_name)
                                                            .size(13.0)
                                                            .color(Color32::WHITE),
                                                    );
                                                    ui.label(
                                                        RichText::new(&message.snippet)
                                                            .size(10.0)
                                                            .color(Color32::from_gray(100)),
                                                    );
                                                });
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new("Conversation")
                                                                .size(11.0)
                                                                .color(Color32::from_gray(100)),
                                                        );
                                                    },
                                                );
                                            });
                                        })
                                        .response;
                                    if resp.interact(Sense::click()).clicked() {
                                        self.spotlight_open_window = Some(WindowKind::Messages);
                                    }
                                    ui.add_space(1.0);
                                }
                            }

                            if !calendar_results.is_empty() {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Calendar Events")
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::from_gray(120)),
                                );
                                ui.add_space(2.0);
                                for event in &calendar_results {
                                    let resp = egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                        .corner_radius(CornerRadius::same(6))
                                        .inner_margin(egui::Margin::symmetric(8, 4))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.set_min_width(510.0);
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        RichText::new(&event.title)
                                                            .size(13.0)
                                                            .color(Color32::WHITE),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{} · {}",
                                                            event.time, event.details
                                                        ))
                                                        .size(10.0)
                                                        .color(Color32::from_gray(100)),
                                                    );
                                                });
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new("Event")
                                                                .size(11.0)
                                                                .color(Color32::from_gray(100)),
                                                        );
                                                    },
                                                );
                                            });
                                        })
                                        .response;
                                    if resp.interact(Sense::click()).clicked() {
                                        self.spotlight_open_window = Some(WindowKind::Calendar);
                                    }
                                    ui.add_space(1.0);
                                }
                            }

                            if !reminder_results.is_empty() {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Reminders")
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::from_gray(120)),
                                );
                                ui.add_space(2.0);
                                for reminder in &reminder_results {
                                    let resp = egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                        .corner_radius(CornerRadius::same(6))
                                        .inner_margin(egui::Margin::symmetric(8, 4))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.set_min_width(510.0);
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        RichText::new(&reminder.title)
                                                            .size(13.0)
                                                            .color(Color32::WHITE),
                                                    );
                                                    ui.label(
                                                        RichText::new(&reminder.details)
                                                            .size(10.0)
                                                            .color(Color32::from_gray(100)),
                                                    );
                                                });
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new("Reminder")
                                                                .size(11.0)
                                                                .color(Color32::from_gray(100)),
                                                        );
                                                    },
                                                );
                                            });
                                        })
                                        .response;
                                    if resp.interact(Sense::click()).clicked() {
                                        self.spotlight_open_window = Some(WindowKind::Notes);
                                    }
                                    ui.add_space(1.0);
                                }
                            }

                            if !preference_results.is_empty() {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("System Preferences")
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::from_gray(120)),
                                );
                                ui.add_space(2.0);
                                for preference in &preference_results {
                                    let resp = egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                        .corner_radius(CornerRadius::same(6))
                                        .inner_margin(egui::Margin::symmetric(8, 4))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.set_min_width(510.0);
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        RichText::new(&preference.title)
                                                            .size(13.0)
                                                            .color(Color32::WHITE),
                                                    );
                                                    ui.label(
                                                        RichText::new(&preference.subtitle)
                                                            .size(10.0)
                                                            .color(Color32::from_gray(100)),
                                                    );
                                                });
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new("Settings")
                                                                .size(11.0)
                                                                .color(Color32::from_gray(100)),
                                                        );
                                                    },
                                                );
                                            });
                                        })
                                        .response;
                                    if resp.interact(Sense::click()).clicked() {
                                        self.spotlight_open_window = Some(WindowKind::Settings);
                                    }
                                    ui.add_space(1.0);
                                }
                            }

                            // Installed apps from AppCatalog (external apps discovered on system)
                            let installed: Vec<(String, String, std::path::PathBuf)> = self
                                .app_catalog
                                .search(&self.spotlight_query)
                                .iter()
                                .filter(|a| !a.path.to_string_lossy().starts_with("aurora://")) // skip builtins (already shown above)
                                .take(6)
                                .map(|a| (a.name.clone(), a.category.clone(), a.path.clone()))
                                .collect();
                            if !installed.is_empty() {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Installed Apps")
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::from_gray(120)),
                                );
                                ui.add_space(2.0);
                                for (name, category, path) in &installed {
                                    let resp = egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                        .corner_radius(CornerRadius::same(6))
                                        .inner_margin(egui::Margin::symmetric(8, 4))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.set_min_width(510.0);
                                                ui.label(
                                                    RichText::new(name)
                                                        .size(13.0)
                                                        .color(Color32::WHITE),
                                                );
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new(category)
                                                                .size(11.0)
                                                                .color(Color32::from_gray(100)),
                                                        );
                                                    },
                                                );
                                            });
                                        })
                                        .response;
                                    if resp.interact(Sense::click()).clicked() {
                                        let _ = open_file_with_system(path);
                                        self.show_spotlight = false;
                                        self.spotlight_query.clear();
                                    }
                                    ui.add_space(1.0);
                                }
                            }

                            // File results (real)
                            if !document_results.is_empty() {
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("Documents")
                                            .size(11.0)
                                            .strong()
                                            .color(Color32::from_gray(120)),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui
                                            .link(RichText::new("Show All in Finder").size(10.0))
                                            .clicked()
                                        {
                                            self.spotlight_open_window =
                                                Some(WindowKind::FileManager);
                                        }
                                    });
                                });
                                ui.add_space(2.0);

                                egui::ScrollArea::vertical()
                                    .max_height(200.0)
                                    .show(ui, |ui| {
                                        for path in &document_results {
                                            let resp = egui::Frame::default()
                                                .fill(Color32::from_rgba_unmultiplied(
                                                    255, 255, 255, 8,
                                                ))
                                                .corner_radius(CornerRadius::same(6))
                                                .inner_margin(egui::Margin::symmetric(8, 3))
                                                .show(ui, |ui| {
                                                    let p = std::path::Path::new(path);
                                                    let fname = p
                                                        .file_name()
                                                        .and_then(|n| n.to_str())
                                                        .unwrap_or(path);
                                                    ui.label(
                                                        RichText::new(fname)
                                                            .size(13.0)
                                                            .color(Color32::WHITE),
                                                    );
                                                    ui.label(
                                                        RichText::new(path)
                                                            .size(10.0)
                                                            .color(Color32::from_gray(100)),
                                                    );
                                                })
                                                .response;
                                            if resp.interact(Sense::click()).clicked() {
                                                self.spotlight_open_file =
                                                    Some(PathBuf::from(path));
                                            }
                                            ui.add_space(1.0);
                                        }
                                    });
                            }

                            if !folder_results.is_empty() {
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("Folders")
                                            .size(11.0)
                                            .strong()
                                            .color(Color32::from_gray(120)),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui
                                            .link(RichText::new("Show All in Finder").size(10.0))
                                            .clicked()
                                        {
                                            self.spotlight_open_window =
                                                Some(WindowKind::FileManager);
                                        }
                                    });
                                });
                                ui.add_space(2.0);
                                for path in &folder_results {
                                    let resp = egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                        .corner_radius(CornerRadius::same(6))
                                        .inner_margin(egui::Margin::symmetric(8, 3))
                                        .show(ui, |ui| {
                                            let p = std::path::Path::new(path);
                                            let fname = p
                                                .file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or(path);
                                            ui.label(
                                                RichText::new(fname)
                                                    .size(13.0)
                                                    .color(Color32::WHITE),
                                            );
                                            ui.label(
                                                RichText::new(path)
                                                    .size(10.0)
                                                    .color(Color32::from_gray(100)),
                                            );
                                        })
                                        .response;
                                    if resp.interact(Sense::click()).clicked() {
                                        self.spotlight_open_file = Some(PathBuf::from(path));
                                    }
                                    ui.add_space(1.0);
                                }
                            }

                            // System commands section — suggest running as terminal command
                            if !query.is_empty() && query.len() >= 2 {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Actions")
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::from_gray(120)),
                                );
                                ui.add_space(2.0);
                                let resp = egui::Frame::default()
                                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(egui::Margin::symmetric(8, 4))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.set_min_width(510.0);
                                            ui.label(
                                                RichText::new(format!(
                                                    "Run \"{}\"",
                                                    self.spotlight_query
                                                ))
                                                .size(13.0)
                                                .color(Color32::WHITE),
                                            );
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    ui.label(
                                                        RichText::new("System Command")
                                                            .size(11.0)
                                                            .color(Color32::from_gray(100)),
                                                    );
                                                },
                                            );
                                        });
                                    })
                                    .response;
                                if resp.interact(Sense::click()).clicked() {
                                    // Send to PTY or execute
                                    if let Some(ref mut pty) = self.pty_terminal {
                                        pty.send(&self.spotlight_query);
                                        // Open terminal window
                                        let win = self.window_mut(WindowKind::Terminal);
                                        win.restore();
                                        win.id_epoch = win.id_epoch.saturating_add(1);
                                        self.bring_to_front(WindowKind::Terminal);
                                    } else {
                                        // Try launching directly
                                        let parts: Vec<&str> =
                                            self.spotlight_query.split_whitespace().collect();
                                        if let Some(program) = parts.first() {
                                            let args: Vec<&str> = parts[1..].to_vec();
                                            match launch_program(program, &args) {
                                                Ok(()) => self.toast_manager.push(Toast::new(
                                                    "Launched",
                                                    *program,
                                                    Color32::from_rgb(52, 199, 89),
                                                )),
                                                Err(e) => self.toast_manager.push(Toast::new(
                                                    "Error",
                                                    e,
                                                    Color32::from_rgb(255, 59, 48),
                                                )),
                                            }
                                        }
                                    }
                                    self.show_spotlight = false;
                                    self.spotlight_query.clear();
                                }
                            }

                            ui.add_space(8.0);
                            egui::Frame::default()
                                .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(egui::Margin::symmetric(10, 8))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new("Preview")
                                            .size(11.0)
                                            .strong()
                                            .color(Color32::from_gray(120)),
                                    );
                                    if let Some(result) = &inline_result {
                                        ui.label(
                                            RichText::new(&result.title)
                                                .size(13.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        );
                                        ui.label(
                                            RichText::new(&result.subtitle)
                                                .size(10.0)
                                                .color(Color32::from_gray(140)),
                                        );
                                        ui.label(
                                            RichText::new(&result.body)
                                                .size(12.0)
                                                .color(Color32::from_gray(210)),
                                        );
                                    } else if let Some((name, _)) = apps
                                        .iter()
                                        .find(|(name, _)| name.to_lowercase().contains(&query))
                                    {
                                        ui.label(
                                            RichText::new(*name)
                                                .size(13.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        );
                                        ui.label(
                                            RichText::new("Built-in application")
                                                .size(10.0)
                                                .color(Color32::from_gray(140)),
                                        );
                                        ui.label(
                                            RichText::new("Open directly from Spotlight.")
                                                .size(11.0)
                                                .color(Color32::from_gray(210)),
                                        );
                                    } else if let Some(contact) = contact_results.first() {
                                        ui.label(
                                            RichText::new(&contact.name)
                                                .size(13.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        );
                                        ui.label(
                                            RichText::new("Contact")
                                                .size(10.0)
                                                .color(Color32::from_gray(140)),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "{} · {}",
                                                contact.email, contact.phone
                                            ))
                                            .size(11.0)
                                            .color(Color32::from_gray(210)),
                                        );
                                    } else if let Some(event) = calendar_results.first() {
                                        ui.label(
                                            RichText::new(&event.title)
                                                .size(13.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        );
                                        ui.label(
                                            RichText::new(event.time.clone())
                                                .size(10.0)
                                                .color(Color32::from_gray(140)),
                                        );
                                        ui.label(
                                            RichText::new(&event.details)
                                                .size(11.0)
                                                .color(Color32::from_gray(210)),
                                        );
                                    } else if let Some(reminder) = reminder_results.first() {
                                        ui.label(
                                            RichText::new(&reminder.title)
                                                .size(13.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        );
                                        ui.label(
                                            RichText::new("Reminder")
                                                .size(10.0)
                                                .color(Color32::from_gray(140)),
                                        );
                                        ui.label(
                                            RichText::new(&reminder.details)
                                                .size(11.0)
                                                .color(Color32::from_gray(210)),
                                        );
                                    } else if let Some(preference) = preference_results.first() {
                                        ui.label(
                                            RichText::new(&preference.title)
                                                .size(13.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        );
                                        ui.label(
                                            RichText::new("Settings pane")
                                                .size(10.0)
                                                .color(Color32::from_gray(140)),
                                        );
                                        ui.label(
                                            RichText::new(&preference.subtitle)
                                                .size(11.0)
                                                .color(Color32::from_gray(210)),
                                        );
                                    } else if let Some(message) = message_results.first() {
                                        ui.label(
                                            RichText::new(&message.contact_name)
                                                .size(13.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        );
                                        ui.label(
                                            RichText::new("Latest matching message")
                                                .size(10.0)
                                                .color(Color32::from_gray(140)),
                                        );
                                        ui.label(
                                            RichText::new(&message.snippet)
                                                .size(11.0)
                                                .color(Color32::from_gray(210)),
                                        );
                                    } else if let Some(path) = file_results.first() {
                                        let preview = build_preview(&PathBuf::from(path));
                                        ui.label(
                                            RichText::new(preview.title)
                                                .size(13.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        );
                                        ui.label(
                                            RichText::new(preview.subtitle)
                                                .size(10.0)
                                                .color(Color32::from_gray(140)),
                                        );
                                        ui.label(
                                            RichText::new(preview.body)
                                                .size(11.0)
                                                .color(Color32::from_gray(210)),
                                        );
                                    } else {
                                        ui.label(
                                            RichText::new("Select a result to preview it here.")
                                                .size(11.0)
                                                .color(Color32::from_gray(120)),
                                        );
                                    }
                                });

                            if !has_app
                                && installed.is_empty()
                                && file_results.is_empty()
                                && inline_result.is_none()
                                && contact_results.is_empty()
                                && message_results.is_empty()
                                && calendar_results.is_empty()
                                && reminder_results.is_empty()
                                && preference_results.is_empty()
                                && !query.is_empty()
                            {
                                ui.label(
                                    RichText::new("No results found")
                                        .size(13.0)
                                        .color(Color32::from_gray(120)),
                                );
                            }
                        }
                    });
            });
    }

    // ── FPS ──────────────────────────────────────────────────────────────────

    fn update_fps(&mut self, ctx: &egui::Context) {
        let dt = ctx.input(|i| i.stable_dt).max(0.0001);
        let fps = 1.0 / dt;
        self.fps_smoothed = if self.fps_smoothed <= 0.0 {
            fps
        } else {
            self.fps_smoothed * 0.9 + fps * 0.1
        };
    }

    fn render_fps_overlay(&self, ctx: &egui::Context) {
        egui::Area::new(Id::new("fps_overlay"))
            .fixed_pos(Pos2::new(
                ctx.content_rect().right() - 100.0,
                MENU_BAR_HEIGHT + 8.0,
            ))
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(6, 12, 30, 150))
                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(50)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(format!("FPS {:.0}", self.fps_smoothed))
                                .size(11.0)
                                .color(Color32::WHITE),
                        );
                    });
            });
    }

    // ── Login Screen ─────────────────────────────────────────────────────────

    fn render_login_screen(&mut self, ctx: &egui::Context) {
        let screen = ctx.viewport_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(Order::Foreground, Id::new("login_bg")));

        // Blurred dark overlay
        painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 160));

        // Shake animation offset
        let shake_x = if let Some(t) = self.login_shake {
            let elapsed = t.elapsed().as_secs_f32();
            if elapsed > 0.4 {
                self.login_shake = None;
                0.0
            } else {
                (elapsed * 40.0).sin() * (1.0 - elapsed * 2.5).max(0.0) * 12.0
            }
        } else {
            0.0
        };

        egui::Area::new(Id::new("login_screen"))
            .fixed_pos(Pos2::new(screen.center().x - 160.0 + shake_x, screen.center().y - 140.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let setup_name_focus = Self::consume_auth_focus(
                        &mut self.auth_focus_pending,
                        self.screen_state == AppScreenState::Setup,
                    );
                    let login_password_focus = Self::consume_auth_focus(
                        &mut self.auth_focus_pending,
                        matches!(self.screen_state, AppScreenState::Login | AppScreenState::Locked),
                    );
                    // User avatar circle
                    let (avatar_r, _) = ui.allocate_exact_size(Vec2::splat(80.0), Sense::hover());
                    ui.painter().circle_filled(avatar_r.center(), 40.0, Color32::from_rgba_unmultiplied(255, 255, 255, 30));
                    ui.painter().circle_stroke(avatar_r.center(), 40.0, Stroke::new(1.5, Color32::from_white_alpha(80)));
                    ui.painter().text(
                        avatar_r.center(),
                        Align2::CENTER_CENTER,
                        self.avatar_initials(),
                        FontId::proportional(32.0),
                        Color32::WHITE,
                    );

                    ui.add_space(12.0);
                    let title = match self.screen_state {
                        AppScreenState::Setup => match self.setup_step {
                            0 => "Welcome to AuroraOS",
                            1 => "Create Your Profile",
                            2 => "Secure Your Account",
                            _ => "Choose Your Appearance",
                        },
                        AppScreenState::Login => self.profile_display_name(),
                        AppScreenState::Locked => self.profile_display_name(),
                        AppScreenState::Desktop => "",
                    };
                    ui.label(RichText::new(title).size(18.0).strong().color(Color32::WHITE));
                    ui.add_space(16.0);

                    match self.screen_state {
                        AppScreenState::Setup => {
                            ui.label(
                                RichText::new(format!("Step {} of {}", self.setup_step + 1, Self::setup_step_count()))
                                    .size(11.0)
                                    .color(Color32::from_gray(150)),
                            );
                            ui.add_space(10.0);
                            match self.setup_step {
                                0 => {
                                    ui.label(RichText::new("Set up your account, password, and desktop appearance before entering AuroraOS.").size(12.0).color(Color32::from_gray(190)));
                                }
                                1 => {
                                    Self::render_auth_field(ui, &mut self.setup_user_name, "Full Name", false, setup_name_focus);
                                }
                                2 => {
                                    Self::render_auth_field(ui, &mut self.setup_password, "Create Password", true, false);
                                    ui.add_space(8.0);
                                    Self::render_auth_field(ui, &mut self.setup_password_confirm, "Confirm Password", true, false);
                                }
                                _ => {
                                    ui.checkbox(&mut self.app_settings.dark_mode, "Use dark appearance");
                                    ui.add_space(8.0);
                                    ui.label(RichText::new("Accent Color").size(11.0).color(Color32::from_gray(180)));
                                    ui.horizontal_wrapped(|ui| {
                                        for (r, g, b) in [
                                            (0, 122, 255),
                                            (52, 199, 89),
                                            (255, 149, 0),
                                            (175, 82, 222),
                                            (255, 59, 48),
                                        ] {
                                            let color = Color32::from_rgb(r, g, b);
                                            let (dot_rect, dot_resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
                                            ui.painter().circle_filled(dot_rect.center(), 8.0, color);
                                            if self.app_settings.accent_r == r && self.app_settings.accent_g == g && self.app_settings.accent_b == b {
                                                ui.painter().circle_stroke(dot_rect.center(), 9.5, Stroke::new(1.5, Color32::WHITE));
                                            }
                                            if dot_resp.clicked() {
                                                self.app_settings.accent_r = r;
                                                self.app_settings.accent_g = g;
                                                self.app_settings.accent_b = b;
                                            }
                                        }
                                    });
                                }
                            }
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                if self.setup_step > 0 && ui.add(
                                    egui::Button::new(RichText::new("Back").size(12.0).color(Color32::from_gray(230)))
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 20))
                                        .corner_radius(CornerRadius::same(14))
                                        .min_size(Vec2::new(110.0, 32.0)),
                                ).clicked() {
                                    self.setup_step = self.setup_step.saturating_sub(1);
                                    self.auth_error = None;
                                }
                                let button_label = if self.setup_step + 1 == Self::setup_step_count() {
                                    "Create Profile"
                                } else {
                                    "Continue"
                                };
                                if ui.add(
                                    egui::Button::new(RichText::new(button_label).size(12.0).color(Color32::WHITE))
                                        .fill(Color32::from_rgb(0, 122, 255))
                                        .corner_radius(CornerRadius::same(14))
                                        .min_size(Vec2::new(126.0, 32.0)),
                                ).clicked() {
                                    self.submit_auth();
                                }
                            });
                            ui.add_space(10.0);
                            let hint = match self.setup_step {
                                0 => "First boot setup is required before entering the desktop.",
                                1 => "Your display name is shown in the menu bar and lock screen.",
                                2 => "A password is currently required to unlock AuroraOS.",
                                _ => "Choose the appearance you want to start with.",
                            };
                            ui.label(RichText::new(hint).size(11.0).color(Color32::from_gray(120)));
                        }
                        AppScreenState::Login | AppScreenState::Locked => {
                            Self::render_auth_field(ui, &mut self.auth_password, "Enter Password", true, login_password_focus);
                            ui.add_space(10.0);
                            let button_label = if self.screen_state == AppScreenState::Locked { "Unlock" } else { "Login" };
                            if ui.add(
                                egui::Button::new(RichText::new(button_label).size(12.0).color(Color32::WHITE))
                                    .fill(Color32::from_rgb(0, 122, 255))
                                    .corner_radius(CornerRadius::same(14))
                                    .min_size(Vec2::new(240.0, 32.0)),
                            ).clicked() {
                                self.submit_auth();
                            }
                            ui.add_space(10.0);
                            let prompt = if self.screen_state == AppScreenState::Locked {
                                "Press Enter to unlock"
                            } else {
                                "Press Enter to log in"
                            };
                            ui.label(RichText::new(prompt).size(11.0).color(Color32::from_gray(120)));
                        }
                        AppScreenState::Desktop => {}
                    }

                    if let Some(error) = &self.auth_error {
                        ui.add_space(8.0);
                        ui.label(RichText::new(error).size(11.0).color(Color32::from_rgb(255, 99, 99)));
                    }

                    // Clock
                    ui.add_space(30.0);
                    let time_str = Local::now().format("%-I:%M %p").to_string();
                    ui.label(RichText::new(time_str).size(48.0).color(Color32::WHITE));
                    let date_str = Local::now().format("%A, %B %-d").to_string();
                    ui.label(RichText::new(date_str).size(16.0).color(Color32::from_gray(180)));
                });
            });

        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.submit_auth();
        }
    }

    fn render_auth_field(
        ui: &mut egui::Ui,
        value: &mut String,
        hint: &str,
        password: bool,
        should_focus: bool,
    ) {
        egui::Frame::default()
            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 25))
            .stroke(Stroke::new(1.0, Color32::from_white_alpha(60)))
            .corner_radius(CornerRadius::same(20))
            .inner_margin(egui::Margin::symmetric(14, 6))
            .show(ui, |ui| {
                let te = egui::TextEdit::singleline(value)
                    .password(password)
                    .hint_text(hint)
                    .font(FontId::proportional(14.0))
                    .text_color(Color32::WHITE)
                    .desired_width(240.0)
                    .frame(false);
                let resp = ui.add(te);
                if !resp.has_focus() && should_focus {
                    resp.request_focus();
                }
            });
    }

    // ── Mission Control ──────────────────────────────────────────────────────

    fn render_mission_control(&mut self, ctx: &egui::Context) {
        let screen = ctx.viewport_rect();
        let target = if self.show_mission_control {
            1.0f32
        } else {
            0.0
        };
        let speed = 6.0 * ctx.input(|i| i.stable_dt);
        self.mission_control_anim += (target - self.mission_control_anim) * speed.min(1.0);

        if self.mission_control_anim < 0.01 && !self.show_mission_control {
            self.mission_control_anim = 0.0;
            return;
        }

        let t = self.mission_control_anim;
        let painter = ctx.layer_painter(egui::LayerId::new(Order::Foreground, Id::new("mc_bg")));
        painter.rect_filled(
            screen,
            0.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, (120.0 * t) as u8),
        );

        // Desktop strip at top
        let strip_y = 12.0 * t;
        let desk_w = 120.0;
        let desk_h = 70.0;
        let strip_total = self.desktop_count as f32 * (desk_w + 12.0) + desk_w + 12.0; // +1 for "+" button
        let strip_x = (screen.width() - strip_total) / 2.0;

        egui::Area::new(Id::new("mc_desktops"))
            .fixed_pos(Pos2::new(strip_x, strip_y))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for desk_i in 0..self.desktop_count {
                        let is_current = desk_i == self.current_desktop;
                        let border_color = if is_current {
                            Color32::from_rgba_unmultiplied(0, 122, 255, (200.0 * t) as u8)
                        } else {
                            Color32::from_rgba_unmultiplied(255, 255, 255, (60.0 * t) as u8)
                        };
                        let resp = egui::Frame::default()
                            .fill(Color32::from_rgba_unmultiplied(
                                40,
                                40,
                                50,
                                (180.0 * t) as u8,
                            ))
                            .stroke(Stroke::new(
                                if is_current { 2.0 } else { 1.0 },
                                border_color,
                            ))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(egui::Margin::symmetric(4, 4))
                            .show(ui, |ui| {
                                ui.set_min_size(Vec2::new(desk_w - 8.0, desk_h - 8.0));
                                ui.label(
                                    RichText::new(format!("Desktop {}", desk_i + 1))
                                        .size(10.0)
                                        .color(Color32::from_rgba_unmultiplied(
                                            200,
                                            200,
                                            200,
                                            (255.0 * t) as u8,
                                        )),
                                );
                            })
                            .response;
                        if resp.interact(Sense::click()).clicked() {
                            self.current_desktop = desk_i;
                        }
                    }
                    // "+" button to add desktop
                    if self.desktop_count < 6 {
                        let resp = egui::Frame::default()
                            .fill(Color32::from_rgba_unmultiplied(
                                255,
                                255,
                                255,
                                (20.0 * t) as u8,
                            ))
                            .stroke(Stroke::new(
                                1.0,
                                Color32::from_rgba_unmultiplied(255, 255, 255, (40.0 * t) as u8),
                            ))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(egui::Margin::symmetric(4, 4))
                            .show(ui, |ui| {
                                ui.set_min_size(Vec2::new(40.0, desk_h - 8.0));
                                ui.centered_and_justified(|ui| {
                                    ui.label(RichText::new("+").size(24.0).color(
                                        Color32::from_rgba_unmultiplied(
                                            200,
                                            200,
                                            200,
                                            (200.0 * t) as u8,
                                        ),
                                    ));
                                });
                            })
                            .response;
                        if resp.interact(Sense::click()).clicked() {
                            self.desktop_count += 1;
                        }
                    }
                });
            });

        // Show windows on the current desktop as thumbnails
        let open_windows: Vec<WindowKind> = self
            .z_order
            .iter()
            .copied()
            .filter(|k| {
                let w = self.window_ref(*k);
                w.open && !w.minimized && w.desktop == self.current_desktop
            })
            .collect();

        if open_windows.is_empty() {
            return;
        }
        let count = open_windows.len();
        let cols = ((count as f32).sqrt().ceil() as usize).max(1);
        let rows = (count + cols - 1) / cols;
        let thumb_w = (screen.width() * 0.7 / cols as f32).min(300.0);
        let thumb_h = (screen.height() * 0.5 / rows as f32).min(200.0);
        let total_w = cols as f32 * (thumb_w + 16.0) - 16.0;
        let start_x = (screen.width() - total_w) / 2.0;
        let start_y = 110.0; // Below desktop strip

        let mut clicked_window: Option<WindowKind> = None;

        for (i, kind) in open_windows.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let x = start_x + col as f32 * (thumb_w + 16.0);
            let y = start_y + row as f32 * (thumb_h + 40.0);

            let thumb_pos = Pos2::new(x, y).lerp(Pos2::new(x, y + 20.0), 1.0 - t);

            egui::Area::new(Id::new(("mc_thumb", i)))
                .fixed_pos(thumb_pos)
                .order(Order::Foreground)
                .show(ctx, |ui| {
                    let alpha = (255.0 * t) as u8;
                    let resp = egui::Frame::default()
                        .fill(Color32::from_rgba_unmultiplied(50, 50, 60, alpha))
                        .stroke(Stroke::new(
                            1.0,
                            Color32::from_rgba_unmultiplied(255, 255, 255, (60.0 * t) as u8),
                        ))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.set_min_size(Vec2::new(thumb_w - 16.0, thumb_h - 12.0));
                            // Traffic lights (tiny)
                            ui.horizontal(|ui| {
                                let s = 6.0;
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(s), Sense::hover());
                                ui.painter().circle_filled(
                                    r.center(),
                                    3.0,
                                    Color32::from_rgb(255, 95, 87),
                                );
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(s), Sense::hover());
                                ui.painter().circle_filled(
                                    r.center(),
                                    3.0,
                                    Color32::from_rgb(255, 189, 47),
                                );
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(s), Sense::hover());
                                ui.painter().circle_filled(
                                    r.center(),
                                    3.0,
                                    Color32::from_rgb(40, 200, 64),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(kind.title()).size(10.0).color(
                                        Color32::from_rgba_unmultiplied(200, 200, 200, alpha),
                                    ),
                                );
                            });
                            // Placeholder content area
                            let (content_r, _) = ui.allocate_exact_size(
                                Vec2::new(thumb_w - 32.0, thumb_h - 44.0),
                                Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                content_r,
                                CornerRadius::same(4),
                                Color32::from_rgba_unmultiplied(30, 30, 40, alpha),
                            );
                        })
                        .response;
                    let interact_resp = resp.interact(Sense::click_and_drag());
                    if interact_resp.clicked() {
                        clicked_window = Some(*kind);
                    }
                    // Start dragging
                    if interact_resp.dragged() {
                        self.mc_dragging_window = Some(*kind);
                    }
                    // Right-click to move window to another desktop
                    interact_resp.context_menu(|ui| {
                        for d in 0..self.desktop_count {
                            if d != self.current_desktop {
                                if ui.button(format!("Move to Desktop {}", d + 1)).clicked() {
                                    self.window_mut(*kind).desktop = d;
                                    ui.close();
                                }
                            }
                        }
                    });

                    // Window title below thumbnail
                    ui.label(RichText::new(kind.title()).size(11.0).color(
                        Color32::from_rgba_unmultiplied(220, 220, 220, (255.0 * t) as u8),
                    ));
                });
        }

        // Drag-and-drop windows to desktop strips
        if let Some(dragging_kind) = self.mc_dragging_window {
            // Draw drag indicator near cursor
            if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                let drag_painter =
                    ctx.layer_painter(egui::LayerId::new(Order::Tooltip, Id::new("mc_drag")));
                drag_painter.text(
                    pos + Vec2::new(12.0, -12.0),
                    Align2::LEFT_BOTTOM,
                    dragging_kind.title(),
                    FontId::proportional(12.0),
                    Color32::from_rgba_unmultiplied(255, 255, 255, 200),
                );
            }
            // Check if drag released
            if !ctx.input(|i| i.pointer.any_down()) {
                if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                    // Check if over a desktop strip
                    if pos.y < desk_h + 20.0 {
                        for desk_i in 0..self.desktop_count {
                            let dx = strip_x + desk_i as f32 * (desk_w + 12.0);
                            if pos.x >= dx && pos.x <= dx + desk_w && desk_i != self.current_desktop
                            {
                                self.window_mut(dragging_kind).desktop = desk_i;
                                self.toast_manager.push(Toast::new(
                                    "Moved",
                                    format!("{} → Desktop {}", dragging_kind.title(), desk_i + 1),
                                    Color32::from_rgb(0, 122, 255),
                                ));
                                break;
                            }
                        }
                    }
                }
                self.mc_dragging_window = None;
            }
        }

        // Click on window thumbnail to focus it and exit MC
        if let Some(kind) = clicked_window {
            self.bring_to_front(kind);
            self.show_mission_control = false;
        }

        // Click on background to close MC
        if ctx.input(|i| i.pointer.primary_clicked()) {
            if clicked_window.is_none() && self.mc_dragging_window.is_none() {
                self.show_mission_control = false;
            }
        }
    }

    // ── App Switcher (Ctrl+Tab) ─────────────────────────────────────────────

    fn handle_app_switcher(&mut self, ctx: &egui::Context) {
        if let Some(kind) = self.active_window() {
            if Self::window_supports_tabs(kind) && self.window_tab_count(kind) > 1 {
                return;
            }
        }
        let tab_pressed = ctx.input(|i| i.key_pressed(egui::Key::Tab) && i.modifiers.ctrl);
        let ctrl_held = ctx.input(|i| i.modifiers.ctrl);

        if tab_pressed {
            let open_windows: Vec<WindowKind> = self
                .z_order
                .iter()
                .copied()
                .filter(|k| {
                    let w = self.window_ref(*k);
                    w.open && !w.minimized && w.desktop == self.current_desktop
                })
                .collect();
            if open_windows.is_empty() {
                return;
            }

            if !self.show_app_switcher {
                self.show_app_switcher = true;
                self.app_switcher_idx = 0;
            }
            // Shift+Tab = go backwards
            let shift = ctx.input(|i| i.modifiers.shift);
            let count = open_windows.len();
            if shift {
                self.app_switcher_idx = if self.app_switcher_idx == 0 {
                    count - 1
                } else {
                    self.app_switcher_idx - 1
                };
            } else {
                self.app_switcher_idx = (self.app_switcher_idx + 1) % count;
            }
        }

        if self.show_app_switcher && !ctrl_held {
            // Ctrl released — select the window
            let open_windows: Vec<WindowKind> = self
                .z_order
                .iter()
                .copied()
                .filter(|k| {
                    let w = self.window_ref(*k);
                    w.open && !w.minimized && w.desktop == self.current_desktop
                })
                .collect();
            if let Some(&kind) = open_windows.get(self.app_switcher_idx) {
                self.bring_to_front(kind);
            }
            self.show_app_switcher = false;
            return;
        }

        if !self.show_app_switcher {
            return;
        }

        let open_windows: Vec<WindowKind> = self
            .z_order
            .iter()
            .copied()
            .filter(|k| {
                let w = self.window_ref(*k);
                w.open && !w.minimized && w.desktop == self.current_desktop
            })
            .collect();
        if open_windows.is_empty() {
            self.show_app_switcher = false;
            return;
        }

        let screen = ctx.viewport_rect();
        let count = open_windows.len();
        let icon_size = 72.0;
        let padding = 12.0;
        let total_w = count as f32 * (icon_size + padding) - padding + 32.0;
        let bar_h = icon_size + 40.0;

        egui::Area::new(Id::new("app_switcher"))
            .fixed_pos(Pos2::new(
                (screen.width() - total_w) / 2.0,
                screen.center().y - bar_h / 2.0,
            ))
            .order(Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(40, 40, 50, 220))
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                    .corner_radius(CornerRadius::same(16))
                    .inner_margin(egui::Margin::symmetric(16, 12))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (i, kind) in open_windows.iter().enumerate() {
                                let selected = i == self.app_switcher_idx % count;
                                ui.vertical(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(
                                        Vec2::splat(icon_size),
                                        Sense::hover(),
                                    );
                                    if selected {
                                        ui.painter().rect_filled(
                                            rect.expand(4.0),
                                            CornerRadius::same(12),
                                            Color32::from_rgba_unmultiplied(0, 122, 255, 40),
                                        );
                                        ui.painter().rect_stroke(
                                            rect.expand(4.0),
                                            CornerRadius::same(12),
                                            Stroke::new(
                                                2.0,
                                                Color32::from_rgba_unmultiplied(0, 122, 255, 160),
                                            ),
                                            StrokeKind::Outside,
                                        );
                                    }
                                    // App icon background
                                    let icon_color = match kind {
                                        WindowKind::Terminal => Color32::from_rgb(30, 30, 46),
                                        WindowKind::Browser => Color32::from_rgb(0, 180, 216),
                                        WindowKind::Messages => Color32::from_rgb(76, 217, 100),
                                        WindowKind::FileManager => Color32::from_rgb(0, 122, 255),
                                        WindowKind::Calculator => Color32::from_rgb(80, 80, 80),
                                        WindowKind::Notes => Color32::from_rgb(255, 214, 10),
                                        WindowKind::MusicPlayer => Color32::from_rgb(255, 55, 95),
                                        WindowKind::Photos => Color32::from_rgb(255, 107, 107),
                                        WindowKind::Calendar => Color32::from_rgb(255, 59, 48),
                                        WindowKind::Settings => Color32::from_rgb(142, 142, 147),
                                        WindowKind::NetworkDiagnostics => {
                                            Color32::from_rgb(0, 122, 255)
                                        }
                                        WindowKind::DiskUtility => Color32::from_rgb(255, 159, 10),
                                        WindowKind::Dictionary => Color32::from_rgb(255, 214, 10),
                                        WindowKind::Console => Color32::from_rgb(88, 86, 214),
                                        WindowKind::FontBook => Color32::from_rgb(191, 90, 242),
                                        WindowKind::ColorPicker => {
                                            self.color_picker.selected_color()
                                        }
                                        _ => Color32::from_rgb(88, 86, 214),
                                    };
                                    ui.painter().rect_filled(
                                        rect.shrink(4.0),
                                        CornerRadius::same(14),
                                        icon_color,
                                    );
                                    icons::paint_app_icon(
                                        &ui.painter(),
                                        rect.shrink(8.0),
                                        kind.title(),
                                        "System",
                                    );
                                    // Title below
                                    let title_color = if selected {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_gray(160)
                                    };
                                    ui.label(
                                        RichText::new(kind.title()).size(10.0).color(title_color),
                                    );
                                });
                            }
                        });
                    });
            });
    }

    // ── Keyboard shortcuts overlay ───────────────────────────────────────────

    fn render_shortcuts_overlay(&mut self, ctx: &egui::Context) {
        let screen = ctx.viewport_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            Order::Foreground,
            Id::new("shortcuts_bg"),
        ));
        painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 160));

        let shortcuts: &[(&str, &str)] = &[
            ("Ctrl+Space", "Spotlight Search"),
            ("F3 / Ctrl+Shift+Up", "Mission Control"),
            ("F4", "Launchpad"),
            ("Ctrl+Tab", "App Switcher"),
            ("Ctrl+Left/Right", "Snap Window Half"),
            ("Ctrl+Alt+Left/Right/Down", "Snap Window Third"),
            ("Ctrl+W", "Close Window"),
            ("Ctrl+M", "Minimize Window"),
            ("Ctrl+Q", "Quit"),
            ("Ctrl+Shift+E", "Embed External App"),
            ("Escape", "Dismiss Overlay"),
            ("Ctrl+C / X / V", "Copy / Cut / Paste"),
            ("Ctrl+Z / Y", "Undo / Redo"),
            ("Ctrl+S", "Save"),
            ("Ctrl+/", "This Shortcuts Panel"),
        ];

        egui::Area::new(Id::new("shortcuts_overlay"))
            .fixed_pos(Pos2::new(
                screen.center().x - 240.0,
                screen.top() + screen.height() * 0.12,
            ))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(40, 40, 45, 240))
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(50)))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(egui::Margin::symmetric(24, 20))
                    .show(ui, |ui| {
                        ui.set_min_width(440.0);
                        ui.label(
                            RichText::new("Keyboard Shortcuts")
                                .size(18.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.add_space(12.0);

                        for (key, desc) in shortcuts {
                            ui.horizontal(|ui| {
                                egui::Frame::default()
                                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 15))
                                    .corner_radius(CornerRadius::same(4))
                                    .inner_margin(egui::Margin::symmetric(8, 3))
                                    .show(ui, |ui| {
                                        ui.set_min_width(180.0);
                                        ui.label(
                                            RichText::new(*key)
                                                .size(12.0)
                                                .color(Color32::from_gray(220))
                                                .family(egui::FontFamily::Monospace),
                                        );
                                    });
                                ui.label(
                                    RichText::new(*desc)
                                        .size(12.0)
                                        .color(Color32::from_gray(170)),
                                );
                            });
                            ui.add_space(2.0);
                        }

                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Press Escape or Ctrl+/ to close")
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                        );
                    });
            });

        if ctx.input(|i| {
            i.key_pressed(egui::Key::Escape)
                || (i.key_pressed(egui::Key::Slash) && i.modifiers.ctrl)
        }) {
            self.show_shortcuts_overlay = false;
        }
    }

    // ── Edge-snap preview ────────────────────────────────────────────────────

    fn detect_edge_snap(&mut self, ctx: &egui::Context, work_rect: Rect) {
        // Only detect when actively dragging a window
        let dragging = ctx.input(|i| i.pointer.is_decidedly_dragging());
        if !dragging {
            if let Some(side) = self.drag_snap_preview.take() {
                // User released while preview showing = apply snap
                if let Some(kind) = self.active_window() {
                    let win = self.window_mut(kind);
                    win.restore();
                    win.maximized = false;
                    win.snap = Some(side);
                    let snapped = Self::snap_rect(work_rect, side);
                    win.default_pos = snapped.min;
                    win.default_size = snapped.size();
                    win.id_epoch = win.id_epoch.saturating_add(1);
                }
            }
            if self.drag_snap_maximize {
                self.drag_snap_maximize = false;
                if let Some(kind) = self.active_window() {
                    let win = self.window_mut(kind);
                    win.restore_rect = Some(Rect::from_min_size(win.default_pos, win.default_size));
                    win.maximized = true;
                    win.snap = None;
                    win.id_epoch = win.id_epoch.saturating_add(1);
                }
            }
            return;
        }

        if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
            let edge_threshold = 8.0;
            self.drag_snap_preview = None;
            self.drag_snap_maximize = false;

            if pos.y <= work_rect.top() + edge_threshold {
                self.drag_snap_maximize = true;
            } else if pos.x <= work_rect.left() + edge_threshold {
                // Alt held = left third, otherwise left half
                let alt = ctx.input(|i| i.modifiers.alt);
                self.drag_snap_preview = Some(if alt {
                    SnapSide::LeftThird
                } else {
                    SnapSide::Left
                });
            } else if pos.x >= work_rect.right() - edge_threshold {
                let alt = ctx.input(|i| i.modifiers.alt);
                self.drag_snap_preview = Some(if alt {
                    SnapSide::RightThird
                } else {
                    SnapSide::Right
                });
            } else if pos.y >= work_rect.bottom() - edge_threshold {
                // Bottom edge + alt = center third
                let alt = ctx.input(|i| i.modifiers.alt);
                if alt {
                    self.drag_snap_preview = Some(SnapSide::CenterThird);
                }
            }
        }
    }

    fn render_edge_snap_preview(&self, ctx: &egui::Context, work_rect: Rect) {
        let preview_rect = if self.drag_snap_maximize {
            Some(work_rect)
        } else {
            self.drag_snap_preview
                .map(|side| Self::snap_rect(work_rect, side))
        };

        if let Some(rect) = preview_rect {
            let painter =
                ctx.layer_painter(egui::LayerId::new(Order::Tooltip, Id::new("snap_preview")));
            painter.rect_filled(
                rect.shrink(4.0),
                CornerRadius::same(12),
                Color32::from_rgba_unmultiplied(0, 122, 255, 40),
            );
            painter.rect_stroke(
                rect.shrink(4.0),
                CornerRadius::same(12),
                Stroke::new(2.0, Color32::from_rgba_unmultiplied(0, 122, 255, 120)),
                StrokeKind::Outside,
            );
        }
    }

    // ── Persistent state (save/load) ─────────────────────────────────────────

    fn state_file_path() -> PathBuf {
        dirs_home().join(".aurora_desktop_state.json")
    }

    fn save_state(&mut self) {
        let mut window_rects = Vec::new();
        for i in 0..WINDOW_COUNT {
            let w = &self.windows[i];
            window_rects.push(format!(
                "[{},{},{},{}]",
                w.default_pos.x, w.default_pos.y, w.default_size.x, w.default_size.y
            ));
        }
        let json = format!(
            "{{\n  \"wallpaper\": {},\n  \"notes\": {:?},\n  \"volume\": {},\n  \"brightness\": {},\n  \"windows\": [{}]\n}}",
            self.wallpaper_idx,
            self.notes_text,
            self.cc_volume,
            self.cc_brightness,
            window_rects.join(",")
        );
        let _ = fs::write(Self::state_file_path(), json);
        // Save open windows and z-order
        let open_indices: Vec<String> = (0..WINDOW_COUNT)
            .filter(|i| self.windows[*i].open)
            .map(|i| i.to_string())
            .collect();
        let z_indices: Vec<String> = self
            .z_order
            .iter()
            .map(|k| (*k as usize).to_string())
            .collect();

        // Also persist AppSettings
        self.app_settings.wallpaper_idx = self.wallpaper_idx;
        self.app_settings.volume = self.cc_volume;
        self.app_settings.brightness = self.cc_brightness;
        self.app_settings.wifi_enabled = self.cc_wifi;
        self.app_settings.bluetooth_enabled = self.cc_bluetooth;
        self.app_settings.airdrop_enabled = self.cc_airdrop;
        self.app_settings.show_file_path_bar = self.show_file_path_bar;
        self.app_settings.show_file_status_bar = self.show_file_status_bar;
        self.app_settings.open_windows = open_indices.join(",");
        self.app_settings.z_order = z_indices.join(",");
        self.app_settings.color_picker_saved_colors = self.color_picker.serialized_favorites();
        self.persist_sidebar_favorites();
        self.persist_recent_emojis();
        self.persist_music_state();
        let _ = self.app_settings.save();
        let _ = self.file_tags.save();
    }

    fn load_state(&mut self) {
        let path = Self::state_file_path();
        if let Ok(content) = fs::read_to_string(&path) {
            // Simple JSON parsing without serde
            if let Some(wp) = content.find("\"wallpaper\":").and_then(|i| {
                content[i + 12..]
                    .trim()
                    .split(|c: char| !c.is_ascii_digit())
                    .next()?
                    .parse::<usize>()
                    .ok()
            }) {
                self.wallpaper_idx = wp % WALLPAPERS.len();
            }
            if let Some(vol) = content.find("\"volume\":").and_then(|i| {
                content[i + 9..]
                    .trim()
                    .split(|c: char| !c.is_ascii_digit() && c != '.')
                    .next()?
                    .parse::<f32>()
                    .ok()
            }) {
                self.cc_volume = vol.clamp(0.0, 1.0);
            }
            if let Some(br) = content.find("\"brightness\":").and_then(|i| {
                content[i + 13..]
                    .trim()
                    .split(|c: char| !c.is_ascii_digit() && c != '.')
                    .next()?
                    .parse::<f32>()
                    .ok()
            }) {
                self.cc_brightness = br.clamp(0.0, 1.0);
            }
            // Load notes between the first and last quote after "notes":
            if let Some(start) = content
                .find("\"notes\":")
                .and_then(|i| content[i + 8..].find('"').map(|j| i + 8 + j + 1))
            {
                if let Some(end) = content[start..].find("\",\n").map(|j| start + j) {
                    let notes = content[start..end]
                        .replace("\\n", "\n")
                        .replace("\\\"", "\"")
                        .replace("\\\\", "\\");
                    self.notes_text = notes;
                }
            }
        }

        // Restore open windows and z-order from AppSettings
        if !self.app_settings.open_windows.is_empty() {
            // Close all windows first
            for i in 0..WINDOW_COUNT {
                self.windows[i].open = false;
            }
            // Open saved windows
            for idx_str in self.app_settings.open_windows.split(',') {
                if let Ok(idx) = idx_str.trim().parse::<usize>() {
                    if idx < WINDOW_COUNT {
                        self.windows[idx].open = true;
                    }
                }
            }
        }
        if !self.app_settings.z_order.is_empty() {
            let mut new_z = Vec::new();
            for idx_str in self.app_settings.z_order.split(',') {
                if let Ok(idx) = idx_str.trim().parse::<usize>() {
                    if let Some(kind) = WindowKind::from_index(idx) {
                        new_z.push(kind);
                    }
                }
            }
            if new_z.len() == WINDOW_COUNT {
                self.z_order = new_z;
            }
        }
    }

    // ── Launchpad (Application Grid) ──────────────────────────────────────

    fn render_pip_overlay(&mut self, ctx: &egui::Context, work_rect: Rect) {
        let Some(mut pip) = self.pip_state.clone() else {
            return;
        };

        pip.pos = Self::pip_clamp_position(work_rect, pip.size, pip.pos);
        let area = egui::Area::new(Id::new("pip_overlay"))
            .order(Order::Foreground)
            .fixed_pos(pip.pos);
        let mut close_pip = false;
        let mut return_to_source = false;

        area.show(ctx, |ui| {
            ui.set_min_size(pip.size);
            let pip_rect = Rect::from_min_size(Pos2::ZERO, pip.size);
            let drag_resp = ui.interact(pip_rect, Id::new("pip_drag"), Sense::click_and_drag());
            if drag_resp.double_clicked() {
                pip.size = Self::pip_toggle_size(pip.size);
                pip.pos = Self::pip_clamp_position(work_rect, pip.size, pip.pos);
                pip.last_interaction = Instant::now();
            }
            if drag_resp.dragged() {
                pip.pos += ui.input(|i| i.pointer.delta());
                pip.pos = Self::pip_clamp_position(work_rect, pip.size, pip.pos);
                self.pip_dragging = true;
                pip.last_interaction = Instant::now();
            }

            let alpha =
                if drag_resp.hovered() || pip.last_interaction.elapsed() < Duration::from_secs(2) {
                    240
                } else {
                    208
                };
            let fill = Color32::from_rgba_unmultiplied(18, 20, 28, alpha);
            egui::Frame::new()
                .fill(fill)
                .corner_radius(CornerRadius::same(14))
                .stroke(Stroke::new(
                    1.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 40),
                ))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_min_size(pip.size);
                    let controls_visible = drag_resp.hovered()
                        || pip.last_interaction.elapsed() < Duration::from_secs(2);

                    ui.horizontal(|ui| {
                        let title = match &pip.source {
                            PipSource::Music => "Now Playing",
                            PipSource::Photo(_) => "Photos",
                            PipSource::Browser { .. } => "Browser",
                        };
                        ui.label(
                            RichText::new(title)
                                .size(12.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if controls_visible && ui.small_button("Back").clicked() {
                                return_to_source = true;
                            }
                            if controls_visible && ui.small_button("X").clicked() {
                                close_pip = true;
                            }
                        });
                    });
                    ui.add_space(8.0);

                    match &pip.source {
                        PipSource::Music => {
                            let real_tracks = Self::music_library_paths(&dirs_home());
                            let current_track_path = self.current_music_path(&real_tracks);
                            let (name, artist, color) = if let Some(path) = current_track_path.as_deref() {
                                (
                                    Self::music_track_title(path),
                                    "Local Music Library",
                                    Self::music_track_color_for_path(path),
                                )
                            } else {
                                let (name, artist, color) =
                                    Self::music_track_info(self.music_track_idx);
                                (name.to_string(), artist, color)
                            };
                            let cover = Rect::from_min_size(
                                ui.cursor().min,
                                Vec2::new(pip.size.x - 20.0, pip.size.y - 58.0),
                            );
                            gradient_rect(
                                ui.painter(),
                                cover,
                                color,
                                Color32::from_rgb(30, 30, 50),
                            );
                            ui.painter().text(
                                cover.center_top() + Vec2::new(0.0, 18.0),
                                Align2::CENTER_TOP,
                                "♪",
                                FontId::proportional(28.0),
                                Color32::from_white_alpha(180),
                            );
                            ui.painter().text(
                                cover.center(),
                                Align2::CENTER_CENTER,
                                if self.music_playing { "Pause" } else { "Play" },
                                FontId::proportional(16.0),
                                Color32::WHITE,
                            );
                            ui.allocate_space(cover.size());
                            ui.label(
                                RichText::new(&name)
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            ui.label(
                                RichText::new(artist)
                                    .size(11.0)
                                    .color(Color32::from_gray(180)),
                            );
                            if let Some(path) = current_track_path.as_deref() {
                                if let Some(metadata) = Self::music_track_metadata_label(path) {
                                    ui.label(
                                        RichText::new(metadata)
                                            .size(10.0)
                                            .color(Color32::from_gray(185)),
                                    );
                                }
                            }
                            if controls_visible
                                && ui
                                    .button(if self.music_playing { "Pause" } else { "Play" })
                                    .clicked()
                            {
                                self.music_playing = !self.music_playing;
                                self.music_last_tick = Instant::now();
                                self.persist_music_state();
                            }
                        }
                        PipSource::Photo(idx) => {
                            let photo_paths = Self::photo_library_paths(&dirs_home());
                            let photo_path = photo_paths.get(*idx);
                            let photo = Rect::from_min_size(
                                ui.cursor().min,
                                Vec2::new(pip.size.x - 20.0, pip.size.y - 58.0),
                            );
                            let mut painted_texture = false;
                            if let Some(path) = photo_path {
                                if let Some(texture) = self.photo_texture_for_path(ui.ctx(), path) {
                                    ui.painter().image(
                                        texture.id(),
                                        photo,
                                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                        Color32::WHITE,
                                    );
                                    painted_texture = true;
                                }
                            }
                            if !painted_texture {
                                let color = Self::photo_color(*idx);
                                let lighter = Color32::from_rgba_unmultiplied(
                                    (color.r() as u16 + 40).min(255) as u8,
                                    (color.g() as u16 + 40).min(255) as u8,
                                    (color.b() as u16 + 40).min(255) as u8,
                                    255,
                                );
                                gradient_rect(ui.painter(), photo, lighter, color);
                                ui.painter().text(
                                    photo.center(),
                                    Align2::CENTER_CENTER,
                                    format!("Photo {}", idx + 1),
                                    FontId::proportional(18.0),
                                    Color32::WHITE,
                                );
                            }
                            ui.allocate_space(photo.size());
                            if let Some(path) = photo_path {
                                let label = path
                                    .file_name()
                                    .and_then(|name| name.to_str())
                                    .unwrap_or("Photo");
                                ui.label(
                                    RichText::new(label)
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                );
                                if let Some(metadata) = Self::photo_metadata_label(path) {
                                    ui.label(
                                        RichText::new(metadata)
                                            .size(10.0)
                                            .color(Color32::from_gray(185)),
                                    );
                                }
                            } else {
                                ui.label(
                                    RichText::new("Floating preview")
                                        .size(11.0)
                                        .color(Color32::from_gray(185)),
                                );
                            }
                        }
                        PipSource::Browser { title, url } => {
                            let card = Rect::from_min_size(
                                ui.cursor().min,
                                Vec2::new(pip.size.x - 20.0, pip.size.y - 58.0),
                            );
                            gradient_rect(
                                ui.painter(),
                                card,
                                Color32::from_rgb(0, 122, 255),
                                Color32::from_rgb(88, 86, 214),
                            );
                            ui.painter().text(
                                card.center_top() + Vec2::new(0.0, 18.0),
                                Align2::CENTER_TOP,
                                "Browser PiP",
                                FontId::proportional(16.0),
                                Color32::WHITE,
                            );
                            ui.painter().text(
                                card.center(),
                                Align2::CENTER_CENTER,
                                title,
                                FontId::proportional(14.0),
                                Color32::WHITE,
                            );
                            ui.allocate_space(card.size());
                            ui.label(RichText::new(url).size(10.0).color(Color32::from_gray(190)));
                            if controls_visible && ui.button("Open in Browser").clicked() {
                                return_to_source = true;
                            }
                        }
                    }
                });

            let resize_rect = Rect::from_min_size(
                Pos2::new(pip.size.x - 18.0, pip.size.y - 18.0),
                Vec2::splat(14.0),
            );
            let resize_resp =
                ui.interact(resize_rect, Id::new("pip_resize"), Sense::click_and_drag());
            ui.painter().text(
                resize_rect.center(),
                Align2::CENTER_CENTER,
                "◢",
                FontId::proportional(11.0),
                Color32::from_gray(200),
            );
            if resize_resp.dragged() {
                pip.size = Self::pip_resized_size(pip.size, ui.input(|i| i.pointer.delta()));
                pip.pos = Self::pip_clamp_position(work_rect, pip.size, pip.pos);
                self.pip_resizing = true;
                pip.last_interaction = Instant::now();
            }
        });

        if self.pip_dragging && !ctx.input(|i| i.pointer.primary_down()) {
            pip.pos = Self::pip_snapped_position(work_rect, pip.size, pip.pos);
            self.pip_dragging = false;
        }
        if self.pip_resizing && !ctx.input(|i| i.pointer.primary_down()) {
            pip.size = Self::pip_resized_size(pip.size, Vec2::ZERO);
            pip.pos = Self::pip_clamp_position(work_rect, pip.size, pip.pos);
            self.pip_resizing = false;
        }

        if return_to_source {
            match &pip.source {
                PipSource::Music => {
                    let win = self.window_mut(WindowKind::MusicPlayer);
                    win.open = true;
                    win.minimized = false;
                    self.bring_to_front(WindowKind::MusicPlayer);
                }
                PipSource::Photo(idx) => {
                    self.photo_viewer_idx = Some(*idx);
                    let win = self.window_mut(WindowKind::Photos);
                    win.open = true;
                    win.minimized = false;
                    self.bring_to_front(WindowKind::Photos);
                }
                PipSource::Browser { .. } => {
                    let win = self.window_mut(WindowKind::Browser);
                    win.open = true;
                    win.minimized = false;
                    self.bring_to_front(WindowKind::Browser);
                }
            }
        }

        self.pip_last_pos = Some(pip.pos);
        self.pip_last_size = Some(pip.size);
        self.pip_state = if close_pip { None } else { Some(pip) };
    }

    fn render_launchpad(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();

        // Dimmed backdrop
        let painter = ctx.layer_painter(egui::LayerId::new(
            Order::Foreground,
            Id::new("launchpad_bg"),
        ));
        painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 160));

        // Clone filtered results to avoid borrow conflict with the closure
        let filtered: Vec<app_launcher::AppEntry> = self
            .app_catalog
            .search(&self.launchpad_query)
            .into_iter()
            .cloned()
            .collect();

        // Layout constants
        let cols = 7_usize;
        let rows_per_page = 5_usize;
        let items_per_page = cols * rows_per_page;
        let total_pages = (filtered.len() + items_per_page - 1).max(1) / items_per_page.max(1);
        let page = self.launchpad_page.min(total_pages.saturating_sub(1));

        let page_start = page * items_per_page;
        let page_end = (page_start + items_per_page).min(filtered.len());
        let page_items = &filtered[page_start..page_end.min(filtered.len())];

        let icon_size = 64.0_f32;
        let cell_w = 110.0_f32;
        let cell_h = 100.0_f32;
        let grid_w = cols as f32 * cell_w;
        let grid_h = rows_per_page as f32 * cell_h;
        let grid_x = (screen.width() - grid_w) * 0.5;
        let search_y = screen.top() + 60.0;
        let grid_y = search_y + 70.0;

        let mut open_app: Option<String> = None;
        let mut launch_external: Option<PathBuf> = None;

        egui::Area::new(Id::new("launchpad"))
            .fixed_pos(Pos2::new(0.0, 0.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                ui.set_min_size(screen.size());

                // Search bar
                let search_x = (screen.width() - 300.0) * 0.5;
                ui.put(
                    Rect::from_min_size(Pos2::new(search_x, search_y), Vec2::new(300.0, 36.0)),
                    |ui: &mut egui::Ui| {
                        egui::Frame::new()
                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 25))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::symmetric(12, 6))
                            .show(ui, |ui| {
                                let te = egui::TextEdit::singleline(&mut self.launchpad_query)
                                    .hint_text("Search apps...")
                                    .font(FontId::proportional(16.0))
                                    .text_color(Color32::WHITE)
                                    .desired_width(260.0)
                                    .frame(false);
                                let resp = ui.add(te);
                                if !resp.has_focus() && self.launchpad_query.is_empty() {
                                    resp.request_focus();
                                }
                                // Reset page on query change
                                if resp.changed() {
                                    self.launchpad_page = 0;
                                }
                            })
                            .response
                    },
                );

                // App grid
                for (i, app) in page_items.iter().enumerate() {
                    let col = i % cols;
                    let row = i / cols;
                    let cx = grid_x + col as f32 * cell_w + cell_w * 0.5;
                    let cy = grid_y + row as f32 * cell_h;

                    let icon_rect = Rect::from_center_size(
                        Pos2::new(cx, cy + icon_size * 0.5),
                        Vec2::splat(icon_size),
                    );

                    let resp = ui.interact(
                        icon_rect.expand(6.0),
                        Id::new(("lp_app", i, page)),
                        Sense::click(),
                    );

                    // Icon background (colored rounded rect)
                    let is_builtin = app.path.to_string_lossy().starts_with("aurora://");
                    let bg = if is_builtin {
                        category_color(&app.category)
                    } else {
                        category_color(&app.category)
                    };

                    let hover_scale = if resp.hovered() { 1.08 } else { 1.0 };
                    let scaled_rect =
                        Rect::from_center_size(icon_rect.center(), icon_rect.size() * hover_scale);

                    // Shadow
                    painter.rect_filled(
                        scaled_rect.translate(Vec2::new(0.0, 3.0)),
                        CornerRadius::same(14),
                        Color32::from_rgba_unmultiplied(0, 0, 0, 40),
                    );
                    // Background
                    painter.rect_filled(scaled_rect, CornerRadius::same(14), bg);

                    // Icon drawn with painter primitives
                    icons::paint_app_icon(&painter, scaled_rect, &app.name, &app.category);

                    // App name below icon
                    let name_pos = Pos2::new(cx, cy + icon_size + 10.0);
                    let truncated = if app.name.len() > 12 {
                        format!("{}...", &app.name[..10])
                    } else {
                        app.name.clone()
                    };
                    painter.text(
                        name_pos,
                        Align2::CENTER_TOP,
                        &truncated,
                        FontId::proportional(11.0),
                        Color32::WHITE,
                    );

                    if resp.clicked() {
                        if is_builtin {
                            open_app = Some(app.name.clone());
                        } else {
                            launch_external = Some(app.path.clone());
                        }
                    }

                    if resp.hovered() && !is_builtin {
                        resp.on_hover_text(format!("{}\n{}", app.name, app.path.display()));
                    }
                }

                // Page dots
                if total_pages > 1 {
                    let dots_y = grid_y + grid_h + 20.0;
                    let dots_w = total_pages as f32 * 18.0;
                    let dots_x = (screen.width() - dots_w) * 0.5;
                    for p in 0..total_pages {
                        let dot_center = Pos2::new(dots_x + p as f32 * 18.0 + 9.0, dots_y);
                        let dot_rect = Rect::from_center_size(dot_center, Vec2::splat(12.0));
                        let dot_resp =
                            ui.interact(dot_rect, Id::new(("lp_page", p)), Sense::click());
                        let alpha = if p == page { 255 } else { 100 };
                        painter.circle_filled(
                            dot_center,
                            if p == page { 4.0 } else { 3.0 },
                            Color32::from_white_alpha(alpha),
                        );
                        if dot_resp.clicked() {
                            self.launchpad_page = p;
                        }
                    }

                    // Arrow key navigation
                    if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) && page > 0 {
                        self.launchpad_page = page - 1;
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) && page + 1 < total_pages
                    {
                        self.launchpad_page = page + 1;
                    }
                }

                // Click on empty space dismisses launchpad
                let bg_resp = ui.interact(screen, Id::new("launchpad_dismiss"), Sense::click());
                if bg_resp.clicked() && open_app.is_none() && launch_external.is_none() {
                    self.show_launchpad = false;
                }
            });

        // Handle app launch
        if let Some(name) = open_app {
            self.open_builtin_app(&name);
            self.show_launchpad = false;
        }
        if let Some(path) = launch_external {
            self.launch_external_app(&path);
            self.show_launchpad = false;
        }
    }

    fn open_builtin_app(&mut self, name: &str) {
        let kind = match name {
            "System Overview" => Some(WindowKind::Overview),
            "Terminal" => Some(WindowKind::Terminal),
            "Files" => Some(WindowKind::FileManager),
            "Browser" => Some(WindowKind::Browser),
            "Calculator" => Some(WindowKind::Calculator),
            "Notes" => Some(WindowKind::Notes),
            "Music" => Some(WindowKind::MusicPlayer),
            "Photos" => Some(WindowKind::Photos),
            "Calendar" => Some(WindowKind::Calendar),
            "TextEdit" => Some(WindowKind::TextEditor),
            "Settings" => Some(WindowKind::Settings),
            "Activity Monitor" => Some(WindowKind::ProcessManager),
            "Network Diagnostics" => Some(WindowKind::NetworkDiagnostics),
            "Disk Utility" => Some(WindowKind::DiskUtility),
            "Font Book" => Some(WindowKind::FontBook),
            "Color Picker" => Some(WindowKind::ColorPicker),
            "Dictionary" => Some(WindowKind::Dictionary),
            "Console" => Some(WindowKind::Console),
            "Messages" => Some(WindowKind::Messages),
            "Quick Controls" => Some(WindowKind::Controls),
            _ => None,
        };
        if let Some(wk) = kind {
            if wk == WindowKind::Dictionary {
                self.dictionary_app.open_word("aurora");
            }
            let win = self.window_mut(wk);
            win.restore();
            win.id_epoch = win.id_epoch.saturating_add(1);
            self.bring_to_front(wk);
        }
    }

    fn launch_external_app(&mut self, path: &std::path::Path) {
        let name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("App")
            .to_string();
        let path_str = path.to_string_lossy().to_string();

        // For .lnk shortcuts, we need to resolve the target or use cmd /C start
        // For .exe files, we can launch directly
        let (program, args): (String, Vec<String>) = if path_str.ends_with(".lnk") {
            // Windows .lnk shortcuts: use cmd /C start to resolve the shortcut
            (
                "cmd".to_string(),
                vec![
                    "/C".to_string(),
                    "start".to_string(),
                    String::new(),
                    path_str.clone(),
                ],
            )
        } else {
            (path_str.clone(), Vec::new())
        };

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        match EmbeddedApp::launch(&name, &program, &args_refs) {
            Ok(app) => {
                self.notification_center.notify(
                    "Launchpad",
                    &format!("Embedding {}", name),
                    "Finding window...",
                    Color32::from_rgb(52, 199, 89),
                );
                self.embedded_apps.push(app);
            }
            Err(e) => {
                // Fallback: open normally in Windows if embedding fails
                open_file_with_system(path);
                self.toast_manager.push(Toast::new(
                    "Launched externally",
                    format!("{} (embed failed: {})", name, e),
                    Color32::from_rgb(255, 214, 10),
                ));
            }
        }
    }

    // ── Embedded Windows Apps ───────────────────────────────────────────────

    fn update_embedded_apps(&mut self, ctx: &egui::Context) {
        // Remove dead apps
        self.embedded_apps.retain_mut(|app| app.is_alive());

        #[cfg(windows)]
        {
            let parent = self.own_hwnd;
            for app in &mut self.embedded_apps {
                // Try to find HWND if not yet found
                if app.hwnd.is_none() {
                    app.try_find_hwnd();
                }
                // Reparent once found
                if app.hwnd.is_some() && !app.is_reparented() {
                    if let Some(p) = parent {
                        app.reparent(p);
                    }
                }
            }
        }

        // Request repaint while apps are being searched
        if self
            .embedded_apps
            .iter()
            .any(|a| a.hwnd.is_none() && !a.gave_up())
        {
            ctx.request_repaint();
        }
    }

    fn render_embed_launcher(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        let panel_w = 420.0_f32;
        let panel_h = 380.0_f32;
        let panel_pos = Pos2::new(
            (screen.width() - panel_w) * 0.5,
            (screen.height() - panel_h) * 0.4,
        );

        // Dimmed backdrop
        let painter =
            ctx.layer_painter(egui::LayerId::new(Order::Foreground, Id::new("embed_dim")));
        painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 120));

        egui::Area::new(Id::new("embed_launcher"))
            .fixed_pos(panel_pos)
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(40, 40, 48, 240))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::same(20))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(80, 80, 100)))
                    .show(ui, |ui| {
                        ui.set_width(panel_w - 40.0);

                        ui.label(
                            RichText::new("Launch Embedded App")
                                .size(18.0)
                                .color(Color32::WHITE)
                                .strong(),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(
                                "Enter a program path or name (e.g. notepad, calc, mspaint)",
                            )
                            .size(12.0)
                            .color(Color32::from_rgb(160, 160, 180)),
                        );
                        ui.add_space(12.0);

                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.embed_launch_input)
                                .desired_width(panel_w - 60.0)
                                .hint_text("Program name or path...")
                                .font(FontId::monospace(14.0)),
                        );

                        if !resp.has_focus() && self.embed_launch_input.is_empty() {
                            resp.request_focus();
                        }

                        ui.add_space(12.0);

                        ui.horizontal(|ui| {
                            let launch = ui.button(RichText::new("Launch").size(14.0));
                            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));

                            if (launch.clicked() || enter)
                                && !self.embed_launch_input.trim().is_empty()
                            {
                                let program = self.embed_launch_input.trim().to_string();
                                let label = program
                                    .split(['/', '\\'])
                                    .last()
                                    .unwrap_or(&program)
                                    .to_string();
                                match EmbeddedApp::launch(&label, &program, &[]) {
                                    Ok(app) => {
                                        self.notification_center.notify(
                                            "Embedded Apps",
                                            &format!("Launching {}", label),
                                            "Finding window...",
                                            Color32::from_rgb(52, 199, 89),
                                        );
                                        self.embedded_apps.push(app);
                                        self.embed_launch_input.clear();
                                        self.show_embed_launcher = false;
                                    }
                                    Err(e) => {
                                        self.notification_center.notify(
                                            "Embedded Apps",
                                            "Launch Failed",
                                            &e,
                                            Color32::from_rgb(255, 59, 48),
                                        );
                                    }
                                }
                            }

                            if ui.button(RichText::new("Cancel").size(14.0)).clicked() {
                                self.show_embed_launcher = false;
                                self.embed_launch_input.clear();
                            }
                        });

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // Quick launch presets
                        ui.label(
                            RichText::new("Quick Launch")
                                .size(13.0)
                                .color(Color32::from_rgb(180, 180, 200)),
                        );
                        ui.add_space(6.0);

                        let presets = [
                            ("Notepad", "notepad"),
                            ("Calculator", "calc"),
                            ("Paint", "mspaint"),
                            ("WordPad", "wordpad"),
                            ("Snipping Tool", "snippingtool"),
                        ];

                        let mut launch_preset: Option<(&str, &str)> = None;
                        for (label, cmd) in presets {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(format!("  {}  ", label)).size(12.0),
                                    )
                                    .corner_radius(CornerRadius::same(6)),
                                )
                                .clicked()
                            {
                                launch_preset = Some((label, cmd));
                            }
                        }

                        if let Some((label, cmd)) = launch_preset {
                            match EmbeddedApp::launch(label, cmd, &[]) {
                                Ok(app) => {
                                    self.notification_center.notify(
                                        "Embedded Apps",
                                        &format!("Launching {}", label),
                                        "Finding window...",
                                        Color32::from_rgb(52, 199, 89),
                                    );
                                    self.embedded_apps.push(app);
                                    self.show_embed_launcher = false;
                                }
                                Err(e) => {
                                    self.notification_center.notify(
                                        "Embedded Apps",
                                        "Launch Failed",
                                        &e,
                                        Color32::from_rgb(255, 59, 48),
                                    );
                                }
                            }
                        }

                        // Show running embedded apps
                        if !self.embedded_apps.is_empty() {
                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("Running Embedded Apps")
                                    .size(13.0)
                                    .color(Color32::from_rgb(180, 180, 200)),
                            );
                            ui.add_space(4.0);

                            let mut detach_idx: Option<usize> = None;
                            for (i, app) in self.embedded_apps.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    let status = if app.is_reparented() {
                                        RichText::new("●")
                                            .size(10.0)
                                            .color(Color32::from_rgb(52, 199, 89))
                                    } else if app.gave_up() {
                                        RichText::new("●")
                                            .size(10.0)
                                            .color(Color32::from_rgb(255, 59, 48))
                                    } else {
                                        RichText::new("●")
                                            .size(10.0)
                                            .color(Color32::from_rgb(255, 214, 10))
                                    };
                                    ui.label(status);
                                    ui.label(
                                        RichText::new(&app.label).size(12.0).color(Color32::WHITE),
                                    );
                                    if ui.small_button("Detach").clicked() {
                                        detach_idx = Some(i);
                                    }
                                });
                            }

                            if let Some(idx) = detach_idx {
                                if idx < self.embedded_apps.len() {
                                    self.embedded_apps[idx].detach();
                                    self.embedded_apps.remove(idx);
                                }
                            }
                        }
                    });
            });
    }

    /// Render embedded app areas (each app gets a floating egui window)
    fn render_embedded_app_windows(&mut self, _ctx: &egui::Context, work_rect: Rect) {
        let app_count = self.embedded_apps.len();
        if app_count == 0 {
            return;
        }

        for (i, app) in self.embedded_apps.iter_mut().enumerate() {
            if !app.is_reparented() {
                continue;
            }

            // Position each embedded app in a tiled layout within the work area
            let cols = (app_count as f32).sqrt().ceil() as usize;
            let rows = (app_count + cols - 1) / cols;
            let col = i % cols;
            let row = i / cols;
            let tile_w = work_rect.width() / cols as f32;
            let tile_h = work_rect.height() / rows as f32;
            let x = work_rect.left() + col as f32 * tile_w;
            let y = work_rect.top() + row as f32 * tile_h;

            // Leave some margin
            let margin = 4.0;
            app.position(
                (x + margin) as i32,
                (y + margin) as i32,
                (tile_w - margin * 2.0) as i32,
                (tile_h - margin * 2.0) as i32,
            );
        }
    }
}

// ── eframe::App ──────────────────────────────────────────────────────────────

impl eframe::App for AuroraDesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_fps(ctx);
        self.maybe_poll();
        self.handle_battery_alerts();
        self.maybe_process_assistant_query();
        self.maybe_tick_music_playback();
        self.sync_music_audio(false);
        if self.music_playing {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        // Refresh real system data periodically
        if self.sysinfo.should_refresh() {
            self.sysinfo.refresh();
            // Push CPU sample to history ring buffer
            if self.cpu_history.len() >= 120 {
                self.cpu_history.pop_front();
            }
            self.cpu_history.push_back(self.sysinfo.cpu_usage);
        }

        // Poll PTY output
        if let Some(ref mut pty) = self.pty_terminal {
            pty.poll_output();
        }

        // Cache our own HWND for embedded app reparenting
        #[cfg(windows)]
        if self.own_hwnd.is_none() {
            self.own_hwnd = embedded_app::find_own_hwnd();
        }

        if self.app_phase == AppPhase::Booting {
            let elapsed = self.boot_started_at.elapsed();
            if Self::should_finish_boot(elapsed) {
                self.app_phase = AppPhase::Ready;
            } else {
                self.render_boot_splash(ctx);
                ctx.request_repaint_after(Duration::from_millis(16));
                return;
            }
        }

        self.render_background(ctx);

        self.note_input_activity(ctx);
        self.handle_session_shortcuts(ctx);
        self.handle_idle_lock();

        if self.screensaver_active && self.screen_state == AppScreenState::Desktop {
            render_screensaver_overlay(
                ctx,
                self.screensaver_kind,
                self.screensaver_started_at
                    .map(|started| started.elapsed())
                    .unwrap_or_default(),
                &Self::screensaver_photo_candidates(&dirs_home()),
            );
            ctx.request_repaint_after(Duration::from_millis(16));
            return;
        }

        // Login/setup/lock screen blocks everything else
        if self.screen_state != AppScreenState::Desktop {
            self.render_login_screen(ctx);
            ctx.request_repaint_after(Duration::from_millis(16));
            return;
        }

        let (toggle_cc, toggle_spotlight, toggle_notifications) = self.render_menu_bar(ctx);
        if toggle_cc {
            self.show_control_center = !self.show_control_center;
            self.show_notifications = false;
            self.notification_panel_opened_at = None;
        }
        if toggle_spotlight {
            self.show_spotlight = !self.show_spotlight;
            self.spotlight_query.clear();
            if self.show_spotlight {
                self.show_assistant = false;
            }
        }
        if toggle_notifications {
            self.show_notifications = !self.show_notifications;
            self.show_control_center = false;
            self.notification_panel_opened_at = self.show_notifications.then(Instant::now);
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Space) && i.modifiers.command) {
            self.show_spotlight = !self.show_spotlight;
            self.spotlight_query.clear();
            if self.show_spotlight {
                self.show_assistant = false;
            }
        }
        if ctx
            .input(|i| i.key_pressed(egui::Key::Space) && i.modifiers.ctrl && !i.modifiers.command)
        {
            self.show_assistant = !self.show_assistant;
            if self.show_assistant {
                self.show_spotlight = false;
                self.assistant_query.clear();
                self.assistant_state = AssistantOverlayState::Listening;
            } else {
                self.assistant_state = AssistantOverlayState::Idle;
                self.pending_assistant_query = None;
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.show_shortcuts_overlay {
                self.show_shortcuts_overlay = false;
            } else if self.show_launchpad {
                self.show_launchpad = false;
            } else if self.show_mission_control {
                self.show_mission_control = false;
            } else {
                self.show_assistant = false;
                self.assistant_state = AssistantOverlayState::Idle;
                self.pending_assistant_query = None;
                self.show_spotlight = false;
                self.show_control_center = false;
                self.show_notifications = false;
                self.notification_panel_opened_at = None;
                self.show_downloads_stack = false;
                self.show_wifi_popup = false;
                self.show_battery_popup = false;
                self.show_volume_popup = false;
                self.show_bluetooth_popup = false;
                self.active_menu = None;
                self.context_menu_pos = None;
            }
        }
        // F4 = Launchpad
        if ctx.input(|i| i.key_pressed(egui::Key::F4)) {
            self.show_launchpad = !self.show_launchpad;
            self.launchpad_query.clear();
            self.launchpad_page = 0;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::C) && i.modifiers.ctrl && i.modifiers.shift) {
            let win = self.window_mut(WindowKind::ColorPicker);
            win.restore();
            win.id_epoch = win.id_epoch.saturating_add(1);
            self.bring_to_front(WindowKind::ColorPicker);
        }
        // F3 or Ctrl+Up = Mission Control
        if ctx.input(|i| {
            i.key_pressed(egui::Key::F3)
                || (i.key_pressed(egui::Key::ArrowUp) && i.modifiers.ctrl && i.modifiers.shift)
        }) {
            self.show_mission_control = !self.show_mission_control;
        }

        let work_rect = Self::desktop_work_rect(ctx);
        if self.windows[WindowKind::ColorPicker as usize].open && self.color_picker.keep_on_top() {
            self.bring_to_front(WindowKind::ColorPicker);
        }
        if self.color_picker.eyedropper_active() && ctx.input(|i| i.pointer.primary_clicked()) {
            if let Some(position) = ctx.input(|i| i.pointer.interact_pos()) {
                let sample = sample_color_from_position(position, work_rect);
                self.color_picker.apply_sample(sample);
            }
        }
        self.handle_shortcuts(ctx, work_rect);
        self.handle_emoji_picker_shortcuts(ctx);
        self.handle_quick_look_shortcuts(ctx);
        self.detect_edge_snap(ctx, work_rect);
        self.render_desktop_icons(ctx, work_rect);
        self.check_context_menu(ctx);
        self.render_windows(ctx, work_rect);
        self.render_edge_snap_preview(ctx, work_rect);
        self.render_dock(ctx);
        if self.show_downloads_stack {
            self.render_downloads_stack(ctx);
        }

        if self.show_control_center {
            self.render_control_center(ctx);
        }
        if self.show_notifications {
            self.render_notification_center(ctx);
        }
        if self.show_wifi_popup {
            self.render_wifi_popup(ctx);
        }
        if self.show_battery_popup {
            self.render_battery_popup(ctx);
        }
        if self.show_volume_popup {
            self.render_volume_popup(ctx);
        }
        if self.show_bluetooth_popup {
            self.render_bluetooth_popup(ctx);
        }
        if self.show_spotlight {
            self.render_spotlight(ctx);
        }
        if self.active_menu.is_some() {
            self.render_menu_dropdown(ctx);
        }
        if self.context_menu_pos.is_some() {
            self.render_context_menu(ctx);
        }
        if self.desktop_rename_target.is_some() {
            self.render_desktop_rename_dialog(ctx);
        }
        if self.quick_look_open {
            self.render_quick_look(ctx);
        }
        if self.file_info_target.is_some() {
            self.render_file_info_panel(ctx);
        }
        if self.show_emoji_picker {
            self.render_emoji_picker(ctx);
        }
        if self.pip_state.is_some() {
            self.render_pip_overlay(ctx, work_rect);
        }
        if self.show_assistant {
            self.render_assistant_overlay(ctx);
        }

        // Launchpad overlay (full-screen app grid)
        if self.show_launchpad {
            self.render_launchpad(ctx);
        }

        // Mission Control overlay (renders on top of everything)
        if self.show_mission_control || self.mission_control_anim > 0.01 {
            self.render_mission_control(ctx);
        }

        // App Switcher (Ctrl+Tab)
        self.handle_app_switcher(ctx);

        // Keyboard shortcuts overlay (Ctrl+/)
        if ctx.input(|i| i.key_pressed(egui::Key::Slash) && i.modifiers.ctrl)
            && !self.show_shortcuts_overlay
        {
            self.show_shortcuts_overlay = true;
        }
        if self.show_shortcuts_overlay {
            self.render_shortcuts_overlay(ctx);
        }

        if !self.show_control_center && !self.show_notifications {
            self.render_fps_overlay(ctx);
        }

        let battery_alert_level = Self::battery_alert_level(
            self.sysinfo.battery_available,
            self.sysinfo.battery_pct,
            self.sysinfo.battery_charging,
        );
        if battery_alert_level >= 2 {
            let screen = ctx.content_rect();
            let color = if battery_alert_level >= 3 {
                Color32::from_rgba_unmultiplied(255, 59, 48, 220)
            } else {
                Color32::from_rgba_unmultiplied(255, 149, 0, 210)
            };
            egui::Area::new(Id::new("battery_alert_banner"))
                .fixed_pos(Pos2::new(screen.center().x - 170.0, MENU_BAR_HEIGHT + 18.0))
                .order(Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::default()
                        .fill(color)
                        .corner_radius(CornerRadius::same(12))
                        .inner_margin(egui::Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(if battery_alert_level >= 3 {
                                    "Reserve Power: save your work now"
                                } else {
                                    "Critical Battery: save your work soon"
                                })
                                .size(12.0)
                                .strong()
                                .color(Color32::WHITE),
                            );
                        });
                });
        }

        // Unsaved changes confirmation dialog
        self.render_confirm_close(ctx);

        // Toast notifications
        if !self.toast_manager.is_empty() {
            self.render_toasts(ctx);
        }

        // Embedded Windows apps
        self.update_embedded_apps(ctx);
        self.render_embedded_app_windows(ctx, work_rect);
        if self.show_embed_launcher {
            self.render_embed_launcher(ctx);
        }

        // Ctrl+Shift+E = Embed launcher
        if ctx.input(|i| i.key_pressed(egui::Key::E) && i.modifiers.ctrl && i.modifiers.shift) {
            self.show_embed_launcher = !self.show_embed_launcher;
        }

        // Auto-save
        if self.editor_modified {
            self.auto_save.mark_dirty();
        }
        if self.auto_save.should_save() {
            let _ = self.auto_save.save_recovery("editor", &self.editor_content);
            let _ = self.auto_save.save_recovery("notes", &self.notes_text);
        }

        // Handle menu actions
        if let Some(action) = self.menu_action.take() {
            match action {
                MenuAction::Quit => {
                    self.save_state();
                    self.should_quit = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                MenuAction::CloseWindow => {
                    if let Some(kind) = self.active_window() {
                        let win = self.window_mut(kind);
                        win.open = false;
                        win.id_epoch = win.id_epoch.saturating_add(1);
                    }
                }
                MenuAction::Minimize => {
                    if let Some(kind) = self.active_window() {
                        let win = self.window_mut(kind);
                        win.minimized = true;
                        win.id_epoch = win.id_epoch.saturating_add(1);
                    }
                }
                MenuAction::Maximize => {
                    if let Some(kind) = self.active_window() {
                        let win = self.window_mut(kind);
                        if win.maximized {
                            win.maximized = false;
                            if let Some(r) = win.restore_rect.take() {
                                win.default_pos = r.min;
                                win.default_size = r.size();
                            }
                        } else {
                            win.restore_rect =
                                Some(Rect::from_min_size(win.default_pos, win.default_size));
                            win.maximized = true;
                            win.snap = None;
                        }
                        win.id_epoch = win.id_epoch.saturating_add(1);
                    }
                }
                MenuAction::TileLeft => {
                    if let Some(kind) = self.active_window() {
                        let win = self.window_mut(kind);
                        win.restore();
                        win.maximized = false;
                        win.snap = Some(SnapSide::Left);
                        let snapped = Self::snap_rect(work_rect, SnapSide::Left);
                        win.default_pos = snapped.min;
                        win.default_size = snapped.size();
                        win.id_epoch = win.id_epoch.saturating_add(1);
                    }
                }
                MenuAction::TileRight => {
                    if let Some(kind) = self.active_window() {
                        let win = self.window_mut(kind);
                        win.restore();
                        win.maximized = false;
                        win.snap = Some(SnapSide::Right);
                        let snapped = Self::snap_rect(work_rect, SnapSide::Right);
                        win.default_pos = snapped.min;
                        win.default_size = snapped.size();
                        win.id_epoch = win.id_epoch.saturating_add(1);
                    }
                }
                MenuAction::TileLeftThird
                | MenuAction::TileCenterThird
                | MenuAction::TileRightThird => {
                    let side = match action {
                        MenuAction::TileLeftThird => SnapSide::LeftThird,
                        MenuAction::TileCenterThird => SnapSide::CenterThird,
                        _ => SnapSide::RightThird,
                    };
                    if let Some(kind) = self.active_window() {
                        let win = self.window_mut(kind);
                        win.restore();
                        win.maximized = false;
                        win.snap = Some(side);
                        let snapped = Self::snap_rect(work_rect, side);
                        win.default_pos = snapped.min;
                        win.default_size = snapped.size();
                        win.id_epoch = win.id_epoch.saturating_add(1);
                    }
                }
                MenuAction::BringAllToFront => {
                    for i in 0..WINDOW_COUNT {
                        let win = &mut self.windows[i];
                        if win.open {
                            win.minimized = false;
                            win.id_epoch = win.id_epoch.saturating_add(1);
                        }
                    }
                }
                MenuAction::Copy => {
                    // Copy selected text from editor/notes to internal clipboard
                    let active = self.active_window();
                    let text = match active {
                        Some(WindowKind::TextEditor) => self.editor_content.clone(),
                        Some(WindowKind::Notes) => self.notes_text.clone(),
                        _ => String::new(),
                    };
                    if !text.is_empty() {
                        self.clipboard.copy(&text);
                    }
                }
                MenuAction::Cut => {
                    let active = self.active_window();
                    match active {
                        Some(WindowKind::TextEditor) => {
                            self.clipboard.copy(&self.editor_content);
                            self.editor_content.clear();
                            self.editor_modified = true;
                            self.sync_active_tab_from_globals(WindowKind::TextEditor);
                        }
                        Some(WindowKind::Notes) => {
                            self.clipboard.copy(&self.notes_text);
                            self.notes_text.clear();
                            self.sync_active_tab_from_globals(WindowKind::Notes);
                        }
                        _ => {}
                    }
                }
                MenuAction::Paste => {
                    let pasted = self.clipboard.paste();
                    if !pasted.is_empty() {
                        let active = self.active_window();
                        match active {
                            Some(WindowKind::TextEditor) => {
                                self.editor_content.push_str(&pasted);
                                self.editor_modified = true;
                                self.sync_active_tab_from_globals(WindowKind::TextEditor);
                            }
                            Some(WindowKind::Notes) => {
                                self.notes_text.push_str(&pasted);
                                self.sync_active_tab_from_globals(WindowKind::Notes);
                            }
                            Some(WindowKind::Messages) => {
                                self.messages_state.input_text.push_str(&pasted);
                            }
                            _ => {}
                        }
                    }
                }
                MenuAction::SelectAll | MenuAction::Undo | MenuAction::Redo => {
                    // These are handled natively by egui's TextEdit widget
                }
                MenuAction::Save => {
                    if let Some(ref path) = self.editor_file_path {
                        if let Ok(()) = std::fs::write(path, self.editor_content.as_bytes()) {
                            self.editor_modified = false;
                            self.notification_center.notify(
                                "TextEdit",
                                "File saved",
                                &path.to_string_lossy(),
                                Color32::from_rgb(52, 199, 89),
                            );
                        }
                    }
                }
                MenuAction::StartScreenSaver => {
                    self.start_screensaver();
                }
                MenuAction::ToggleFullScreen => {
                    let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
                }
                MenuAction::ToggleSidebar => {
                    self.toggle_file_sidebar();
                }
                MenuAction::TogglePathBar => {
                    self.toggle_file_path_bar();
                }
                MenuAction::ToggleStatusBar => {
                    self.toggle_file_status_bar();
                }
                MenuAction::TogglePreview => {
                    self.toggle_file_preview_pane();
                }
            }
        }

        // Process close/minimize animations — finalize when done
        for i in 0..WINDOW_COUNT {
            if self.windows[i].closing && self.windows[i].is_close_anim_done() {
                self.windows[i].open = false;
                self.windows[i].minimized = false;
                self.windows[i].closing = false;
                self.windows[i].close_anim_start = None;
                self.windows[i].id_epoch = self.windows[i].id_epoch.saturating_add(1);
            }
            if self.windows[i].minimizing && self.windows[i].is_minimize_anim_done() {
                self.windows[i].minimized = true;
                self.windows[i].minimizing = false;
                self.windows[i].minimize_anim_start = None;
                self.windows[i].id_epoch = self.windows[i].id_epoch.saturating_add(1);
            }
        }

        // Handle spotlight window open
        if let Some(kind) = self.spotlight_open_window.take() {
            if kind == WindowKind::Dictionary {
                if let Some(word) = self
                    .spotlight_query
                    .trim()
                    .strip_prefix("define ")
                    .map(str::trim)
                    .filter(|word| !word.is_empty())
                {
                    self.dictionary_app.open_word(word);
                }
            }
            let win = self.window_mut(kind);
            win.restore();
            win.id_epoch = win.id_epoch.saturating_add(1);
            self.bring_to_front(kind);
            self.show_spotlight = false;
            self.spotlight_query.clear();
        }

        // Handle spotlight file open
        if let Some(path) = self.spotlight_open_file.take() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let is_text = matches!(
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
                    | "gitignore"
                    | "lock"
                    | "sql"
                    | "lua"
                    | "rb"
            );
            if is_text {
                self.open_file_in_editor(path);
            } else if !self.open_path_in_aurora_if_supported(&path) {
                open_file_with_system(&path);
                self.toast_manager.push(Toast::new(
                    "File Opened",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
                    Color32::from_rgb(52, 199, 89),
                ));
            }
            self.show_spotlight = false;
            self.spotlight_query.clear();
        }

        // Drag & drop files from host OS
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in &i.raw.dropped_files {
                    if let Some(ref path) = file.path {
                        let pb = path.clone();
                        if pb.is_dir() {
                            // Navigate file manager to dropped directory
                            self.fm_current_dir = pb.clone();
                            self.fm_entries = read_directory(&pb);
                            let win = &mut self.windows[WindowKind::FileManager as usize];
                            win.open = true;
                            win.minimized = false;
                            self.bring_to_front(WindowKind::FileManager);
                            self.toast_manager.push(Toast::new(
                                "Folder Opened",
                                pb.file_name().and_then(|n| n.to_str()).unwrap_or("folder"),
                                Color32::from_rgb(0, 122, 255),
                            ));
                        } else if pb
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| terminal::is_text_extension(e))
                            .unwrap_or(false)
                        {
                            // Open text file in editor
                            if let Ok(content) = fs::read_to_string(&pb) {
                                self.editor_content = content;
                                self.editor_file_path = Some(pb.clone());
                                self.editor_modified = false;
                                let win = &mut self.windows[WindowKind::TextEditor as usize];
                                win.open = true;
                                win.minimized = false;
                                win.id_epoch = win.id_epoch.saturating_add(1);
                                self.bring_to_front(WindowKind::TextEditor);
                                self.toast_manager.push(Toast::new(
                                    "File Opened",
                                    pb.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
                                    Color32::from_rgb(52, 199, 89),
                                ));
                            }
                        } else {
                            // Image files: offer as wallpaper
                            let ext = pb.extension().and_then(|e| e.to_str()).unwrap_or("");
                            if matches!(ext, "png" | "jpg" | "jpeg" | "bmp") {
                                self.app_settings.custom_wallpaper =
                                    pb.to_string_lossy().to_string();
                                self.notification_center.notify(
                                    "System",
                                    "Wallpaper set",
                                    &pb.to_string_lossy(),
                                    Color32::from_rgb(52, 199, 89),
                                );
                            } else if self.open_path_in_aurora_if_supported(&pb) {
                            } else {
                                open_file_with_system(&pb);
                            }
                            self.toast_manager.push(Toast::new(
                                "File Opened",
                                pb.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
                                Color32::from_rgb(52, 199, 89),
                            ));
                        }
                    }
                }
            }
        });

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

// ── main ─────────────────────────────────────────────────────────────────────

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("AuroraOS Desktop")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1080.0, 700.0])
            .with_fullscreen(true),
        ..Default::default()
    };

    eframe::run_native(
        "AuroraOS Desktop",
        options,
        Box::new(|cc| {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::TRANSPARENT;
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::new(AuroraDesktopApp::new()))
        }),
    )
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

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

    // ── Wallpaper presets ────────────────────────────────────────────────

    #[test]
    fn wallpaper_presets_exist() {
        assert!(WALLPAPERS.len() >= 2);
    }

    #[test]
    fn wallpaper_bands_are_sorted() {
        for wp in WALLPAPERS {
            for pair in wp.bands.windows(2) {
                assert!(
                    pair[0].0 <= pair[1].0,
                    "wallpaper bands must be sorted by position"
                );
            }
        }
    }

    // ── execute_terminal_command (built-in) ──────────────────────────────

    #[test]
    fn terminal_cmd_help() {
        let si = RealSystemInfo::new();
        let out = AuroraDesktopApp::execute_terminal_command("help", &si);
        assert!(out.len() > 2, "help should produce multiple lines");
        // First line is the echoed command
        assert!(out[0].0.contains("help"));
    }

    #[test]
    fn terminal_cmd_whoami() {
        let si = RealSystemInfo::new();
        let out = AuroraDesktopApp::execute_terminal_command("whoami", &si);
        assert!(out.iter().any(|(line, _)| line == "aurora"));
    }

    #[test]
    fn terminal_cmd_date() {
        let si = RealSystemInfo::new();
        let out = AuroraDesktopApp::execute_terminal_command("date", &si);
        // Should contain year
        assert!(out.iter().any(|(line, _)| line.contains("20")));
    }

    #[test]
    fn terminal_cmd_echo() {
        let si = RealSystemInfo::new();
        let out = AuroraDesktopApp::execute_terminal_command("echo hello world", &si);
        assert!(out.iter().any(|(line, _)| line == "hello world"));
    }

    #[test]
    fn terminal_cmd_clear_returns_sentinel() {
        let si = RealSystemInfo::new();
        let out = AuroraDesktopApp::execute_terminal_command("clear", &si);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, "__CLEAR__");
    }

    #[test]
    fn terminal_cmd_unknown_tries_real_command() {
        let si = RealSystemInfo::new();
        // "echo" via cmd /C should work on Windows
        let out = AuroraDesktopApp::execute_terminal_command("echo test123", &si);
        // Our built-in "echo" handles this, so check it works
        assert!(out.iter().any(|(line, _)| line.contains("test123")));
    }

    #[test]
    fn terminal_cmd_aurora_status() {
        let si = RealSystemInfo::new();
        let out = AuroraDesktopApp::execute_terminal_command("aurora status", &si);
        assert!(out.iter().any(|(line, _)| line.contains("AuroraOS")));
    }

    #[test]
    fn terminal_cmd_aurora_unknown_subcommand() {
        let si = RealSystemInfo::new();
        let out = AuroraDesktopApp::execute_terminal_command("aurora foobar", &si);
        assert!(out
            .iter()
            .any(|(line, _)| line.contains("unknown subcommand")));
    }

    #[test]
    fn terminal_cmd_open_audio_returns_music_token() {
        let si = RealSystemInfo::new();
        let root = std::env::temp_dir().join(format!(
            "aurora_terminal_open_audio_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("track.mp3");
        std::fs::write(&path, b"audio").unwrap();

        let out =
            AuroraDesktopApp::execute_terminal_command(&format!("open {}", path.display()), &si);

        assert!(out
            .iter()
            .any(|(line, _)| line == &format!("__OPEN_MUSIC__{}", path.display())));
        assert!(out
            .iter()
            .any(|(line, _)| line.contains(&format!("Opening {}", path.display()))));

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&root);
    }

    #[test]
    fn terminal_cmd_run_audio_returns_music_token() {
        let si = RealSystemInfo::new();
        let root = std::env::temp_dir().join(format!(
            "aurora_terminal_run_audio_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("track.flac");
        std::fs::write(&path, b"audio").unwrap();

        let out =
            AuroraDesktopApp::execute_terminal_command(&format!("run {}", path.display()), &si);

        assert!(out
            .iter()
            .any(|(line, _)| line == &format!("__OPEN_MUSIC__{}", path.display())));
        assert!(out
            .iter()
            .any(|(line, _)| line.contains(&format!("Opening {}", path.display()))));

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&root);
    }

    #[test]
    fn terminal_cmd_open_video_returns_video_token() {
        let si = RealSystemInfo::new();
        let root = std::env::temp_dir().join(format!(
            "aurora_terminal_open_video_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("clip.mp4");
        std::fs::write(&path, b"video").unwrap();

        let out =
            AuroraDesktopApp::execute_terminal_command(&format!("open {}", path.display()), &si);

        assert!(out
            .iter()
            .any(|(line, _)| line == &format!("__OPEN_VIDEO__{}", path.display())));
        assert!(out
            .iter()
            .any(|(line, _)| line.contains(&format!("Opening {}", path.display()))));

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&root);
    }

    #[test]
    fn terminal_cmd_run_video_returns_video_token() {
        let si = RealSystemInfo::new();
        let root = std::env::temp_dir().join(format!(
            "aurora_terminal_run_video_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("clip.mkv");
        std::fs::write(&path, b"video").unwrap();

        let out =
            AuroraDesktopApp::execute_terminal_command(&format!("run {}", path.display()), &si);

        assert!(out
            .iter()
            .any(|(line, _)| line == &format!("__OPEN_VIDEO__{}", path.display())));
        assert!(out
            .iter()
            .any(|(line, _)| line.contains(&format!("Opening {}", path.display()))));

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&root);
    }

    #[test]
    fn terminal_cmd_open_url_returns_browser_token() {
        let si = RealSystemInfo::new();
        let out = AuroraDesktopApp::execute_terminal_command("open github.com", &si);
        assert!(out
            .iter()
            .any(|(line, _)| line == "__OPEN_BROWSER__github.com"));
        assert!(out
            .iter()
            .any(|(line, _)| line.contains("Opening github.com")));
    }

    #[test]
    fn initial_screen_state_requires_setup_without_profile() {
        let settings = AppSettings::default();
        assert_eq!(
            AuroraDesktopApp::initial_screen_state(&settings),
            AppScreenState::Setup
        );
    }

    #[test]
    fn initial_screen_state_uses_login_with_profile() {
        let mut settings = AppSettings::default();
        settings.user_name = "Aurora User".to_string();
        settings.set_password("secret");
        assert_eq!(
            AuroraDesktopApp::initial_screen_state(&settings),
            AppScreenState::Login
        );
    }

    #[test]
    fn create_profile_rejects_mismatched_passwords() {
        let mut settings = AppSettings::default();
        let result =
            AuroraDesktopApp::try_create_profile(&mut settings, "Aurora User", "secret", "other");
        assert_eq!(result, Err("Passwords do not match."));
        assert!(!settings.has_user_profile());
    }

    #[test]
    fn create_profile_sets_user_and_hashes_password() {
        let mut settings = AppSettings::default();
        let result = AuroraDesktopApp::try_create_profile(
            &mut settings,
            " Aurora User ",
            "secret",
            "secret",
        );
        assert_eq!(result, Ok(()));
        assert_eq!(settings.user_name, "Aurora User");
        assert!(settings.verify_password("secret"));
    }

    #[test]
    fn validate_setup_step_checks_required_fields() {
        assert_eq!(AuroraDesktopApp::validate_setup_step(0, "", "", ""), Ok(()));
        assert_eq!(
            AuroraDesktopApp::validate_setup_step(1, "   ", "", ""),
            Err("Enter a user name.")
        );
        assert_eq!(
            AuroraDesktopApp::validate_setup_step(2, "Aurora", "", ""),
            Err("Enter a password.")
        );
        assert_eq!(
            AuroraDesktopApp::validate_setup_step(2, "Aurora", "secret", "other"),
            Err("Passwords do not match.")
        );
        assert_eq!(
            AuroraDesktopApp::validate_setup_step(2, "Aurora", "secret", "secret"),
            Ok(())
        );
    }

    #[test]
    fn try_unlock_requires_matching_password() {
        let mut settings = AppSettings::default();
        settings.user_name = "Aurora User".to_string();
        settings.set_password("secret");
        assert_eq!(
            AuroraDesktopApp::try_unlock(&settings, "wrong"),
            Err("Incorrect password.")
        );
        assert_eq!(AuroraDesktopApp::try_unlock(&settings, "secret"), Ok(()));
    }

    #[test]
    fn try_change_password_requires_current_and_matching_new_password() {
        let mut settings = AppSettings::default();
        settings.user_name = "Aurora User".to_string();
        settings.set_password("secret");
        assert_eq!(
            AuroraDesktopApp::try_change_password(&mut settings, "", "new-secret", "new-secret"),
            Err("Enter your current password.")
        );
        assert_eq!(
            AuroraDesktopApp::try_change_password(
                &mut settings,
                "wrong",
                "new-secret",
                "new-secret"
            ),
            Err("Current password is incorrect.")
        );
        assert_eq!(
            AuroraDesktopApp::try_change_password(&mut settings, "secret", "new-secret", "other"),
            Err("New passwords do not match.")
        );
        assert_eq!(
            AuroraDesktopApp::try_change_password(
                &mut settings,
                "secret",
                "new-secret",
                "new-secret"
            ),
            Ok(())
        );
        assert!(settings.verify_password("new-secret"));
    }

    #[test]
    fn profile_created_label_reflects_profile_timestamp() {
        let mut profile = UserProfile::from_display_name("Aurora User", (0, 122, 255));
        profile.created_at = 42;
        assert_eq!(
            AuroraDesktopApp::profile_created_label(&profile),
            "Created 42"
        );
    }

    #[test]
    fn auto_lock_only_triggers_on_desktop_after_timeout() {
        assert!(AuroraDesktopApp::should_auto_lock(
            Duration::from_secs(5 * 60),
            5,
            AppScreenState::Desktop,
        ));
        assert!(!AuroraDesktopApp::should_auto_lock(
            Duration::from_secs(4 * 60 + 59),
            5,
            AppScreenState::Desktop,
        ));
        assert!(!AuroraDesktopApp::should_auto_lock(
            Duration::from_secs(10 * 60),
            5,
            AppScreenState::Locked,
        ));
    }

    #[test]
    fn screensaver_starts_before_lock_timeout() {
        assert!(AuroraDesktopApp::should_start_screensaver(
            Duration::from_secs(4 * 60 + 30),
            5,
            AppScreenState::Desktop,
        ));
        assert!(!AuroraDesktopApp::should_start_screensaver(
            Duration::from_secs(60),
            5,
            AppScreenState::Desktop,
        ));
    }

    #[test]
    fn start_screensaver_marks_active_and_cycles_kind() {
        let mut app = AuroraDesktopApp::new();
        app.screen_state = AppScreenState::Desktop;
        let first = app.screensaver_kind;
        app.start_screensaver();
        assert!(app.screensaver_active);
        assert!(app.screensaver_started_at.is_some());
        assert_eq!(app.screensaver_kind, first.next());
    }

    #[test]
    fn boot_progress_clamps_between_zero_and_one() {
        assert_eq!(
            AuroraDesktopApp::boot_progress(Duration::from_millis(0)),
            0.0
        );
        assert!((AuroraDesktopApp::boot_progress(Duration::from_secs(1)) - 0.5).abs() < 0.001);
        assert_eq!(AuroraDesktopApp::boot_progress(Duration::from_secs(3)), 1.0);
    }

    #[test]
    fn boot_finishes_after_two_seconds() {
        assert!(!AuroraDesktopApp::should_finish_boot(
            Duration::from_millis(1999)
        ));
        assert!(AuroraDesktopApp::should_finish_boot(Duration::from_secs(2)));
    }

    #[test]
    fn desktop_icon_positions_stay_within_work_rect() {
        let work_rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));
        let positions = AuroraDesktopApp::desktop_icon_positions(work_rect, 8);
        assert_eq!(positions.len(), 8);
        for pos in positions {
            assert!(pos.x >= work_rect.left());
            assert!(pos.x <= work_rect.right());
            assert!(pos.y >= work_rect.top());
            assert!(pos.y <= work_rect.bottom());
        }
    }

    #[test]
    fn desktop_icon_positions_progress_down_then_left() {
        let work_rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(400.0, 260.0));
        let positions = AuroraDesktopApp::desktop_icon_positions(work_rect, 4);
        assert!(positions[1].y > positions[0].y || positions[1].x < positions[0].x);
    }

    #[test]
    fn load_sidebar_favorites_uses_defaults_when_empty() {
        let settings = AppSettings::default();
        let favorites = AuroraDesktopApp::load_sidebar_favorites(&settings);
        assert!(!favorites.is_empty());
    }

    #[test]
    fn load_sidebar_favorites_parses_persisted_entries() {
        let mut settings = AppSettings::default();
        settings.favorite_paths = "C:/One|C:/Two".to_string();
        let favorites = AuroraDesktopApp::load_sidebar_favorites(&settings);
        assert_eq!(
            favorites,
            vec![PathBuf::from("C:/One"), PathBuf::from("C:/Two")]
        );
    }

    #[test]
    fn parse_tag_labels_and_set_tag_label_roundtrip() {
        let mut app = AuroraDesktopApp::new();
        app.set_tag_label(TagColor::Blue, "Work");
        app.set_tag_label(TagColor::Red, "Urgent");
        let labels = AuroraDesktopApp::parse_tag_labels(&app.app_settings);
        assert!(labels.contains(&(TagColor::Blue, "Work".to_string())));
        assert_eq!(app.tag_label(TagColor::Red), "Urgent");
    }

    #[test]
    fn custom_smart_folders_roundtrip_helpers_preserve_rules() {
        let mut app = AuroraDesktopApp::new();
        let folders = vec![CustomSmartFolder {
            name: "Big Logs".to_string(),
            extension: Some("log".to_string()),
            min_size_mb: Some(5),
            tag: Some("red".to_string()),
        }];
        app.save_custom_smart_folders(&folders);
        let loaded = AuroraDesktopApp::load_custom_smart_folders(&app.app_settings);
        assert_eq!(loaded, folders);
    }

    #[test]
    fn smart_folder_title_maps_large_and_tag_tokens() {
        let app = AuroraDesktopApp::new();
        assert_eq!(app.smart_folder_title("large"), "Large Files");
        assert_eq!(app.smart_folder_title("tag_blue"), "Tagged Blue");
    }

    #[test]
    fn tag_filter_token_serializes_any_and_all_modes() {
        assert_eq!(
            AuroraDesktopApp::tag_filter_token(&[TagColor::Red, TagColor::Blue], false),
            Some("tags_any_red,blue".to_string())
        );
        assert_eq!(
            AuroraDesktopApp::tag_filter_token(&[TagColor::Red, TagColor::Blue], true),
            Some("tags_all_red,blue".to_string())
        );
    }

    #[test]
    fn drag_uses_copy_modifier_detects_ctrl() {
        let mut modifiers = egui::Modifiers::NONE;
        assert!(!AuroraDesktopApp::drag_uses_copy_modifier(modifiers));
        modifiers.ctrl = true;
        assert!(AuroraDesktopApp::drag_uses_copy_modifier(modifiers));
    }

    #[test]
    fn active_drag_count_uses_multi_selection_when_dragging_selected_desktop_item() {
        let mut app = AuroraDesktopApp::new();
        let a = PathBuf::from("C:/Desktop/a.txt");
        let b = PathBuf::from("C:/Desktop/b.txt");
        app.desktop_selected_paths = vec![a.clone(), b];
        assert_eq!(app.active_drag_count(&a), 2);
        assert_eq!(app.active_drag_count(&PathBuf::from("C:/Other/c.txt")), 1);
    }

    #[test]
    fn dock_icon_accepts_file_drop_rejects_separator_only() {
        assert!(AuroraDesktopApp::dock_icon_accepts_file_drop(
            DockIcon::Files
        ));
        assert!(AuroraDesktopApp::dock_icon_accepts_file_drop(
            DockIcon::Trash
        ));
        assert!(!AuroraDesktopApp::dock_icon_accepts_file_drop(
            DockIcon::Separator
        ));
    }

    #[test]
    fn dock_position_uses_settings_value() {
        let mut settings = AppSettings::default();
        settings.dock_position = "left".to_string();
        assert_eq!(
            AuroraDesktopApp::dock_position(&settings),
            DockPosition::Left
        );
        settings.dock_position = "nope".to_string();
        assert_eq!(
            AuroraDesktopApp::dock_position(&settings),
            DockPosition::Bottom
        );
    }

    #[test]
    fn dock_hovered_for_position_checks_correct_edge() {
        let screen = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1000.0, 800.0));
        assert!(AuroraDesktopApp::dock_hovered_for_position(
            DockPosition::Bottom,
            screen,
            Some(Pos2::new(500.0, 790.0))
        ));
        assert!(AuroraDesktopApp::dock_hovered_for_position(
            DockPosition::Left,
            screen,
            Some(Pos2::new(10.0, 300.0))
        ));
        assert!(AuroraDesktopApp::dock_hovered_for_position(
            DockPosition::Right,
            screen,
            Some(Pos2::new(990.0, 300.0))
        ));
        assert!(!AuroraDesktopApp::dock_hovered_for_position(
            DockPosition::Bottom,
            screen,
            Some(Pos2::new(500.0, 100.0))
        ));
    }

    #[test]
    fn dock_hidden_offset_respects_auto_hide_and_hover() {
        assert_eq!(
            AuroraDesktopApp::dock_hidden_offset(false, false, Duration::from_secs(1)),
            0.0
        );
        assert_eq!(
            AuroraDesktopApp::dock_hidden_offset(true, false, Duration::from_secs(1)),
            1.0
        );
        assert!(AuroraDesktopApp::dock_hidden_offset(true, true, Duration::from_millis(100)) < 1.0);
        assert_eq!(
            AuroraDesktopApp::dock_hidden_offset(true, true, Duration::from_millis(500)),
            0.0
        );
    }

    #[test]
    fn update_dock_hover_state_tracks_enter_and_exit_durations() {
        let mut app = AuroraDesktopApp::new();
        let start = Instant::now();
        assert_eq!(app.update_dock_hover_state(true, start), Duration::ZERO);
        assert!(
            app.update_dock_hover_state(true, start + Duration::from_millis(120))
                >= Duration::from_millis(120)
        );
        assert_eq!(
            app.update_dock_hover_state(false, start + Duration::from_millis(130)),
            Duration::ZERO
        );
        assert!(
            app.update_dock_hover_state(false, start + Duration::from_millis(700))
                >= Duration::from_millis(500)
        );
    }

    #[test]
    fn spotlight_calc_result_supports_basic_patterns() {
        assert_eq!(
            AuroraDesktopApp::spotlight_calc_result("2 + 2").map(|result| result.body),
            Some("4".to_string())
        );
        assert_eq!(
            AuroraDesktopApp::spotlight_calc_result("15% of 200").map(|result| result.body),
            Some("30".to_string())
        );
        assert_eq!(
            AuroraDesktopApp::spotlight_calc_result("sqrt(144)").map(|result| result.body),
            Some("12".to_string())
        );
    }

    #[test]
    fn spotlight_conversion_and_definition_results_work() {
        assert_eq!(
            AuroraDesktopApp::spotlight_conversion_result("10 km in miles")
                .map(|result| result.body),
            Some("6.21371".to_string())
        );
        let definition = AuroraDesktopApp::spotlight_definition_result("define aurora").unwrap();
        assert_eq!(definition.kind, SpotlightInlineKind::Definition);
        assert!(definition.body.contains("light display"));
    }

    #[test]
    fn spotlight_top_hit_label_prefers_inline_and_non_empty_queries() {
        assert_eq!(
            AuroraDesktopApp::spotlight_top_hit_label("2 + 2", 0, 0, true),
            Some("Top Hit")
        );
        assert_eq!(
            AuroraDesktopApp::spotlight_top_hit_label("files", 1, 0, false),
            Some("Top Hit")
        );
        assert_eq!(
            AuroraDesktopApp::spotlight_top_hit_label("", 0, 0, false),
            None
        );
    }

    #[test]
    fn spotlight_category_helpers_surface_contacts_messages_and_preferences() {
        let app = AuroraDesktopApp::new();

        let contacts = app.spotlight_contact_hits("alice");
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].email, "alice@aurora.local");

        let messages = app.spotlight_message_hits("screenshot");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].contact_name, "Alice");

        let prefs = AuroraDesktopApp::spotlight_system_preference_hits("privacy");
        assert_eq!(prefs.len(), 1);
        assert_eq!(prefs[0].title, "Security & Privacy");
    }

    #[test]
    fn spotlight_category_helpers_surface_calendar_and_reminders() {
        let app = AuroraDesktopApp::new();

        let events = app.spotlight_calendar_hits("standup");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].time, "9:00 AM");

        let reminders = AuroraDesktopApp::spotlight_reminder_hits("alice");
        assert_eq!(reminders.len(), 1);
        assert_eq!(reminders[0].title, "Send prototype build");
    }

    #[test]
    fn smart_folder_entries_for_token_supports_large_files_and_tags() {
        let root = unique_temp_dir("aurora_smart_folder_tokens");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let large = root.join("movie.bin");
        let tagged = root.join("notes.txt");
        std::fs::write(&large, vec![0_u8; 101 * 1024 * 1024]).unwrap();
        std::fs::write(&tagged, "hello").unwrap();

        let mut tags = FileTags::default();
        tags.assign(&tagged, TagColor::Blue);

        let app = AuroraDesktopApp::new();
        let large_entries = app.smart_folder_entries_for_token("large", &root, &tags);
        assert_eq!(large_entries.len(), 1);
        assert_eq!(large_entries[0].name, "movie.bin");

        let tagged_entries = app.smart_folder_entries_for_token("tag_blue", &root, &tags);
        assert_eq!(tagged_entries.len(), 1);
        assert_eq!(tagged_entries[0].name, "notes.txt");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn smart_folder_entries_for_token_supports_multi_tag_and_custom_rules() {
        let root = unique_temp_dir("aurora_smart_folder_custom_tokens");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let keep = root.join("keep.log");
        let skip = root.join("skip.log");
        std::fs::write(&keep, vec![0_u8; 2 * 1024 * 1024]).unwrap();
        std::fs::write(&skip, vec![0_u8; 2 * 1024 * 1024]).unwrap();

        let mut app = AuroraDesktopApp::new();
        app.save_custom_smart_folders(&[CustomSmartFolder {
            name: "Big Logs".to_string(),
            extension: Some("log".to_string()),
            min_size_mb: Some(1),
            tag: Some("blue".to_string()),
        }]);

        let mut tags = FileTags::default();
        tags.assign(&keep, TagColor::Blue);
        tags.assign(&keep, TagColor::Red);
        tags.assign(&skip, TagColor::Red);

        let any_entries = app.smart_folder_entries_for_token("tags_any_red,blue", &root, &tags);
        assert_eq!(any_entries.len(), 2);
        let all_entries = app.smart_folder_entries_for_token("tags_all_red,blue", &root, &tags);
        assert_eq!(all_entries.len(), 1);
        assert_eq!(all_entries[0].name, "keep.log");

        let custom_entries = app.smart_folder_entries_for_token("custom_Big_Logs", &root, &tags);
        assert_eq!(custom_entries.len(), 1);
        assert_eq!(custom_entries[0].name, "keep.log");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn desktop_drop_target_returns_desktop_for_pointer_inside_work_rect() {
        let work_rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(200.0, 200.0));
        let source = dirs_home().join("Documents").join("note.txt");
        let target =
            AuroraDesktopApp::desktop_drop_target(&source, Some(Pos2::new(40.0, 40.0)), work_rect);
        assert_eq!(target, Some(desktop_directory()));
    }

    #[test]
    fn desktop_drop_target_skips_files_already_on_desktop() {
        let work_rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(200.0, 200.0));
        let source = desktop_directory().join("note.txt");
        let target =
            AuroraDesktopApp::desktop_drop_target(&source, Some(Pos2::new(40.0, 40.0)), work_rect);
        assert_eq!(target, None);
    }

    #[test]
    fn favorite_drop_target_accepts_folder_and_trash_destinations() {
        let source = dirs_home().join("Documents").join("note.txt");
        let folder = dirs_home().join("Downloads");
        let trash = trash_dir();
        assert_eq!(
            AuroraDesktopApp::favorite_drop_target(&source, &folder, &trash),
            Some(folder)
        );
        assert_eq!(
            AuroraDesktopApp::favorite_drop_target(&source, &trash, &trash),
            Some(trash)
        );
    }

    #[test]
    fn favorite_drop_target_skips_current_parent_folder() {
        let folder = dirs_home().join("Documents");
        let source = folder.join("note.txt");
        let trash = trash_dir();
        let target = AuroraDesktopApp::favorite_drop_target(&source, &folder, &trash);
        assert_eq!(target, None);
    }

    #[test]
    fn current_folder_drop_target_uses_current_directory_when_movable() {
        let source = dirs_home().join("Desktop").join("note.txt");
        let current = dirs_home().join("Documents");
        let target = AuroraDesktopApp::current_folder_drop_target(&source, &current);
        assert_eq!(target, Some(current));
    }

    #[test]
    fn current_folder_drop_target_skips_same_parent() {
        let current = dirs_home().join("Documents");
        let source = current.join("note.txt");
        let target = AuroraDesktopApp::current_folder_drop_target(&source, &current);
        assert_eq!(target, None);
    }

    #[test]
    fn directory_row_drop_target_uses_directory_when_movable() {
        let source = dirs_home().join("Desktop").join("note.txt");
        let target_dir = dirs_home().join("Pictures");
        let target = AuroraDesktopApp::directory_row_drop_target(&source, &target_dir);
        assert_eq!(target, Some(target_dir));
    }

    #[test]
    fn directory_row_drop_target_skips_same_parent() {
        let target_dir = dirs_home().join("Pictures");
        let source = target_dir.join("note.txt");
        let target = AuroraDesktopApp::directory_row_drop_target(&source, &target_dir);
        assert_eq!(target, None);
    }

    #[test]
    fn desktop_context_menu_items_switch_between_background_and_item_actions() {
        assert_eq!(
            AuroraDesktopApp::desktop_context_menu_items(true),
            &["Open", "Rename", "Copy", "Move to Trash", "---", "Get Info"]
        );
        assert!(AuroraDesktopApp::desktop_context_menu_items(false).contains(&"New Folder"));
        assert!(AuroraDesktopApp::desktop_context_menu_items(false).contains(&"Start Screen Saver"));
    }

    #[test]
    fn insert_text_into_active_input_routes_to_supported_windows() {
        let mut app = AuroraDesktopApp::new();

        app.windows[WindowKind::Notes as usize].open = true;
        app.focused = Some(WindowKind::Notes);
        assert!(app.insert_text_into_active_input("😀"));
        assert!(app.notes_text.ends_with("😀"));

        app.windows[WindowKind::TextEditor as usize].open = true;
        app.focused = Some(WindowKind::TextEditor);
        assert!(app.insert_text_into_active_input("🚀"));
        assert!(app.editor_content.ends_with("🚀"));
        assert!(app.editor_modified);

        app.windows[WindowKind::Messages as usize].open = true;
        app.focused = Some(WindowKind::Messages);
        assert!(app.insert_text_into_active_input("👍"));
        assert!(app.messages_state.input_text.ends_with("👍"));
    }

    #[test]
    fn screensaver_photo_candidates_prefer_picture_formats() {
        let root = std::env::temp_dir().join(format!(
            "aurora_screensaver_photos_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let pictures = root.join("Pictures");
        std::fs::create_dir_all(&pictures).unwrap();
        std::fs::write(pictures.join("one.png"), "a").unwrap();
        std::fs::write(pictures.join("two.jpg"), "b").unwrap();
        std::fs::write(pictures.join("notes.txt"), "c").unwrap();

        let photos = AuroraDesktopApp::screensaver_photo_candidates(&root);
        assert_eq!(
            photos,
            vec![pictures.join("one.png"), pictures.join("two.jpg")]
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn photo_library_paths_filters_supported_images() {
        let root = std::env::temp_dir().join(format!(
            "aurora_photo_library_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let pictures = root.join("Pictures");
        std::fs::create_dir_all(&pictures).unwrap();
        std::fs::write(pictures.join("one.png"), "a").unwrap();
        std::fs::write(pictures.join("two.webp"), "b").unwrap();
        std::fs::write(pictures.join("notes.txt"), "c").unwrap();

        let photos = AuroraDesktopApp::photo_library_paths(&root);
        assert_eq!(
            photos,
            vec![pictures.join("one.png"), pictures.join("two.webp")]
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn music_library_paths_filters_supported_audio() {
        let root = std::env::temp_dir().join(format!(
            "aurora_music_library_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let music = root.join("Music");
        std::fs::create_dir_all(&music).unwrap();
        std::fs::write(music.join("one.mp3"), "a").unwrap();
        std::fs::write(music.join("two.flac"), "b").unwrap();
        std::fs::write(music.join("cover.jpg"), "c").unwrap();

        let tracks = AuroraDesktopApp::music_library_paths(&root);
        assert_eq!(tracks, vec![music.join("one.mp3"), music.join("two.flac")]);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn decode_photo_color_image_reads_png_dimensions() {
        let root = std::env::temp_dir().join(format!(
            "aurora_photo_decode_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("sample.png");

        let image = image::RgbaImage::from_fn(2, 3, |x, y| {
            image::Rgba([(x * 20) as u8, (y * 30) as u8, 120, 255])
        });
        image.save(&path).unwrap();

        let decoded = AuroraDesktopApp::decode_photo_color_image(&path).unwrap();
        assert_eq!(decoded.size, [2, 3]);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn photo_metadata_label_reports_dimensions_and_size() {
        let root = std::env::temp_dir().join(format!(
            "aurora_photo_metadata_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("meta.png");

        let image = image::RgbaImage::from_fn(4, 5, |x, y| {
            image::Rgba([(x * 10) as u8, (y * 20) as u8, 180, 255])
        });
        image.save(&path).unwrap();

        let label = AuroraDesktopApp::photo_metadata_label(&path).unwrap();
        assert!(label.contains("4x5"));
        assert!(label.ends_with('B'));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn music_track_helpers_report_title_metadata_and_stable_color() {
        let root = std::env::temp_dir().join(format!(
            "aurora_music_metadata_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("midnight_drive.flac");
        std::fs::write(&path, b"audio-bytes").unwrap();

        assert_eq!(
            AuroraDesktopApp::music_track_title(&path),
            "midnight_drive".to_string()
        );
        let metadata = AuroraDesktopApp::music_track_metadata_label(&path).unwrap();
        assert!(metadata.contains("FLAC"));
        assert!(metadata.ends_with('B'));
        assert_eq!(
            AuroraDesktopApp::music_track_color_for_path(&path),
            AuroraDesktopApp::music_track_color_for_path(&path)
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn assistant_track_match_with_paths_prefers_real_library_matches() {
        let tracks = vec![
            PathBuf::from("C:/Users/test/Music/Night Drive.mp3"),
            PathBuf::from("C:/Users/test/Music/Deep Focus.flac"),
        ];
        assert_eq!(
            AuroraDesktopApp::assistant_track_match_with_paths("focus", &tracks),
            Some(1)
        );
        assert_eq!(
            AuroraDesktopApp::assistant_track_match_with_paths("missing", &tracks),
            None
        );
    }

    #[test]
    fn current_music_track_title_uses_real_library_when_available() {
        let tracks = vec![
            PathBuf::from("C:/Users/test/Music/First Song.mp3"),
            PathBuf::from("C:/Users/test/Music/Second Song.flac"),
        ];
        assert_eq!(
            AuroraDesktopApp::current_music_track_title(3, &tracks),
            "Second Song".to_string()
        );
        assert_eq!(
            AuroraDesktopApp::current_music_track_title(1, &[]),
            "Neon Waves".to_string()
        );
    }

    #[test]
    fn filtered_music_track_indices_match_title_and_extension() {
        let tracks = vec![
            PathBuf::from("C:/Users/test/Music/First Song.mp3"),
            PathBuf::from("C:/Users/test/Music/Rain Loop.flac"),
            PathBuf::from("C:/Users/test/Music/Notes.wav"),
        ];
        assert_eq!(
            AuroraDesktopApp::filtered_music_track_indices(&tracks, "rain"),
            vec![1]
        );
        assert_eq!(
            AuroraDesktopApp::filtered_music_track_indices(&tracks, "wav"),
            vec![2]
        );
        assert_eq!(
            AuroraDesktopApp::filtered_music_track_indices(&tracks, ""),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn music_filtered_queue_position_finds_current_track() {
        let queue = vec![4, 1, 3];
        assert_eq!(
            AuroraDesktopApp::music_filtered_queue_position(&queue, 1),
            Some(1)
        );
        assert_eq!(
            AuroraDesktopApp::music_filtered_queue_position(&queue, 2),
            None
        );
    }

    #[test]
    fn step_music_track_idx_wraps_within_filtered_queue() {
        let queue = vec![4, 1, 3];
        assert_eq!(AuroraDesktopApp::step_music_track_idx(1, &queue, 1), 3);
        assert_eq!(AuroraDesktopApp::step_music_track_idx(1, &queue, -1), 4);
        assert_eq!(AuroraDesktopApp::step_music_track_idx(9, &queue, 1), 1);
    }

    #[test]
    fn music_queue_indices_can_shuffle_stably() {
        let tracks = vec![
            PathBuf::from("C:/Users/test/Music/C Song.mp3"),
            PathBuf::from("C:/Users/test/Music/A Song.mp3"),
            PathBuf::from("C:/Users/test/Music/B Song.mp3"),
        ];
        let queue_a = AuroraDesktopApp::music_queue_indices(&tracks, "", true);
        let queue_b = AuroraDesktopApp::music_queue_indices(&tracks, "", true);
        assert_eq!(queue_a, queue_b);
        assert_eq!(queue_a.len(), 3);
    }

    #[test]
    fn step_music_track_idx_with_repeat_stops_at_edges_when_disabled() {
        let queue = vec![4, 1, 3];
        assert_eq!(
            AuroraDesktopApp::step_music_track_idx_with_repeat(4, &queue, -1, false),
            None
        );
        assert_eq!(
            AuroraDesktopApp::step_music_track_idx_with_repeat(3, &queue, 1, false),
            None
        );
        assert_eq!(
            AuroraDesktopApp::step_music_track_idx_with_repeat(1, &queue, 1, false),
            Some(3)
        );
    }

    #[test]
    fn format_music_time_uses_minute_second_clock() {
        assert_eq!(AuroraDesktopApp::format_music_time(0.0), "0:00");
        assert_eq!(AuroraDesktopApp::format_music_time(61.2), "1:01");
    }

    #[test]
    fn advance_music_elapsed_reports_track_end() {
        assert_eq!(
            AuroraDesktopApp::advance_music_elapsed(10.0, 5.0, 20.0),
            (15.0, false)
        );
        assert_eq!(
            AuroraDesktopApp::advance_music_elapsed(18.0, 5.0, 20.0),
            (20.0, true)
        );
    }

    #[test]
    fn music_track_duration_seconds_uses_mock_and_real_inputs() {
        let root = std::env::temp_dir().join(format!(
            "aurora_music_duration_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("ambient.flac");
        std::fs::write(&path, vec![0_u8; 96_000]).unwrap();

        assert_eq!(
            AuroraDesktopApp::music_track_duration_seconds(0, None),
            208.0
        );
        let real_duration = AuroraDesktopApp::music_track_duration_seconds(0, Some(&path));
        assert!(real_duration >= 90.0);
        assert!(real_duration <= 420.0);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn music_seek_fraction_clamps_to_bar_bounds() {
        let rect = Rect::from_min_size(Pos2::new(10.0, 0.0), Vec2::new(200.0, 4.0));
        assert_eq!(AuroraDesktopApp::music_seek_fraction(rect, -20.0), 0.0);
        assert_eq!(AuroraDesktopApp::music_seek_fraction(rect, 110.0), 0.5);
        assert_eq!(AuroraDesktopApp::music_seek_fraction(rect, 400.0), 1.0);
    }

    #[test]
    fn music_seek_seconds_scales_fraction_to_duration() {
        assert_eq!(AuroraDesktopApp::music_seek_seconds(200.0, 0.25), 50.0);
        assert_eq!(AuroraDesktopApp::music_seek_seconds(200.0, -1.0), 0.0);
        assert_eq!(AuroraDesktopApp::music_seek_seconds(200.0, 2.0), 200.0);
    }

    #[test]
    fn desired_music_audio_action_matches_target_and_play_state() {
        let active = std::path::Path::new("C:/Music/current.mp3");
        let other = std::path::Path::new("C:/Music/other.mp3");
        assert_eq!(
            AuroraDesktopApp::desired_music_audio_action(Some(active), Some(active), true, false),
            MusicAudioAction::Resume
        );
        assert_eq!(
            AuroraDesktopApp::desired_music_audio_action(Some(active), Some(other), true, false),
            MusicAudioAction::PlayFromOffset
        );
        assert_eq!(
            AuroraDesktopApp::desired_music_audio_action(Some(active), Some(active), false, false),
            MusicAudioAction::Pause
        );
        assert_eq!(
            AuroraDesktopApp::desired_music_audio_action(Some(active), None, true, false),
            MusicAudioAction::Stop
        );
        assert_eq!(
            AuroraDesktopApp::desired_music_audio_action(Some(active), Some(active), true, true),
            MusicAudioAction::PlayFromOffset
        );
    }

    #[test]
    fn desktop_rename_destination_uses_parent_and_trimmed_name() {
        let target = PathBuf::from("C:/Users/test/Desktop/old.txt");
        let renamed = AuroraDesktopApp::desktop_rename_destination(&target, "  new.txt  ");
        assert_eq!(
            renamed,
            Some(PathBuf::from("C:/Users/test/Desktop/new.txt"))
        );
    }

    #[test]
    fn desktop_rename_destination_rejects_empty_name() {
        let target = PathBuf::from("C:/Users/test/Desktop/old.txt");
        assert_eq!(
            AuroraDesktopApp::desktop_rename_destination(&target, "   "),
            None
        );
    }

    #[test]
    fn toggle_desktop_selection_adds_and_removes_path() {
        let path = PathBuf::from("C:/Users/test/Desktop/file.txt");
        let mut selected = Vec::new();
        AuroraDesktopApp::toggle_desktop_selection(&mut selected, &path);
        assert_eq!(selected, vec![path.clone()]);
        AuroraDesktopApp::toggle_desktop_selection(&mut selected, &path);
        assert!(selected.is_empty());
    }

    #[test]
    fn replace_desktop_selection_keeps_only_target_path() {
        let mut selected = vec![PathBuf::from("C:/one"), PathBuf::from("C:/two")];
        let path = PathBuf::from("C:/Users/test/Desktop/file.txt");
        AuroraDesktopApp::replace_desktop_selection(&mut selected, &path);
        assert_eq!(selected, vec![path]);
    }

    #[test]
    fn select_all_desktop_entries_returns_every_path() {
        let entries = vec![
            FmEntry {
                name: "a".into(),
                is_dir: false,
                size: 0,
                path: PathBuf::from("C:/a"),
            },
            FmEntry {
                name: "b".into(),
                is_dir: true,
                size: 0,
                path: PathBuf::from("C:/b"),
            },
        ];
        let selected = AuroraDesktopApp::select_all_desktop_entries(&entries);
        assert_eq!(selected, vec![PathBuf::from("C:/a"), PathBuf::from("C:/b")]);
    }

    #[test]
    fn desktop_icon_rects_match_entry_count() {
        let work_rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(300.0, 300.0));
        let rects = AuroraDesktopApp::desktop_icon_rects(work_rect, 3);
        assert_eq!(rects.len(), 3);
    }

    #[test]
    fn desktop_paths_in_selection_rect_returns_intersecting_entries() {
        let entries = vec![
            FmEntry {
                name: "a".into(),
                is_dir: false,
                size: 0,
                path: PathBuf::from("C:/a"),
            },
            FmEntry {
                name: "b".into(),
                is_dir: false,
                size: 0,
                path: PathBuf::from("C:/b"),
            },
        ];
        let rects = vec![
            Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(72.0, 76.0)),
            Rect::from_min_size(Pos2::new(100.0, 0.0), Vec2::new(72.0, 76.0)),
        ];
        let selection =
            AuroraDesktopApp::desktop_selection_rect(Pos2::new(10.0, 10.0), Pos2::new(80.0, 80.0));
        let selected =
            AuroraDesktopApp::desktop_paths_in_selection_rect(&entries, &rects, selection);
        assert_eq!(selected, vec![PathBuf::from("C:/a")]);
    }

    #[test]
    fn desktop_stack_name_groups_common_file_types() {
        let image = FmEntry {
            name: "photo.png".into(),
            is_dir: false,
            size: 0,
            path: PathBuf::from("C:/photo.png"),
        };
        let doc = FmEntry {
            name: "notes.md".into(),
            is_dir: false,
            size: 0,
            path: PathBuf::from("C:/notes.md"),
        };
        let dir = FmEntry {
            name: "Projects".into(),
            is_dir: true,
            size: 0,
            path: PathBuf::from("C:/Projects"),
        };
        assert_eq!(AuroraDesktopApp::desktop_stack_name(&image), "Images");
        assert_eq!(AuroraDesktopApp::desktop_stack_name(&doc), "Documents");
        assert_eq!(AuroraDesktopApp::desktop_stack_name(&dir), "Folders");
    }

    #[test]
    fn desktop_stacks_groups_entries_by_stack_name() {
        let entries = vec![
            FmEntry {
                name: "a.png".into(),
                is_dir: false,
                size: 0,
                path: PathBuf::from("C:/a.png"),
            },
            FmEntry {
                name: "b.jpg".into(),
                is_dir: false,
                size: 0,
                path: PathBuf::from("C:/b.jpg"),
            },
            FmEntry {
                name: "notes.md".into(),
                is_dir: false,
                size: 0,
                path: PathBuf::from("C:/notes.md"),
            },
        ];
        let groups = AuroraDesktopApp::desktop_stacks(&entries);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "Documents");
        assert_eq!(groups[1].0, "Images");
        assert_eq!(groups[1].1.len(), 2);
    }

    #[test]
    fn smoke_auth_flow_setup_to_unlock() {
        let mut settings = AppSettings::default();
        assert_eq!(
            AuroraDesktopApp::initial_screen_state(&settings),
            AppScreenState::Setup
        );

        AuroraDesktopApp::try_create_profile(&mut settings, "Aurora User", "secret", "secret")
            .unwrap();
        assert_eq!(
            AuroraDesktopApp::initial_screen_state(&settings),
            AppScreenState::Login
        );
        assert!(AuroraDesktopApp::try_unlock(&settings, "secret").is_ok());
    }

    #[test]
    fn smoke_setup_wizard_advances_to_profile_creation() {
        let mut app = AuroraDesktopApp::new();
        app.screen_state = AppScreenState::Setup;
        app.setup_user_name = "Aurora User".to_string();
        app.setup_password = "secret".to_string();
        app.setup_password_confirm = "secret".to_string();

        assert_eq!(app.try_advance_setup(), Ok(false));
        assert_eq!(app.setup_step, 1);
        assert_eq!(app.try_advance_setup(), Ok(false));
        assert_eq!(app.setup_step, 2);
        assert_eq!(app.try_advance_setup(), Ok(false));
        assert_eq!(app.setup_step, 3);
        assert_eq!(app.try_advance_setup(), Ok(true));
        assert!(app.app_settings.has_user_profile());
    }

    #[test]
    fn smoke_desktop_move_flow_moves_file_between_user_visible_locations() {
        let root = unique_temp_dir("aurora_smoke_move_flow");
        let _ = std::fs::remove_dir_all(&root);
        let desktop = root.join("Desktop");
        let documents = root.join("Documents");
        std::fs::create_dir_all(&desktop).unwrap();
        std::fs::create_dir_all(&documents).unwrap();

        let source = documents.join("note.txt");
        std::fs::write(&source, "hello").unwrap();

        let desktop_target =
            AuroraDesktopApp::current_folder_drop_target(&source, &desktop).unwrap();
        let moved = move_entry_to_directory(&source, &desktop_target).unwrap();
        assert_eq!(moved, desktop.join("note.txt"));
        assert!(moved.exists());

        let documents_target =
            AuroraDesktopApp::current_folder_drop_target(&moved, &documents).unwrap();
        let moved_back = move_entry_to_directory(&moved, &documents_target).unwrap();
        assert_eq!(moved_back, documents.join("note.txt"));
        assert!(moved_back.exists());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn consume_auth_focus_only_returns_true_once() {
        let mut pending = true;
        assert!(AuroraDesktopApp::consume_auth_focus(&mut pending, true));
        assert!(!pending);
        assert!(!AuroraDesktopApp::consume_auth_focus(&mut pending, true));
    }

    #[test]
    fn consume_auth_focus_respects_disabled_state() {
        let mut pending = true;
        assert!(!AuroraDesktopApp::consume_auth_focus(&mut pending, false));
        assert!(pending);
    }

    #[test]
    fn toggle_file_preview_pane_flips_state() {
        let mut app = AuroraDesktopApp::new();
        assert!(app.show_file_preview_pane);
        app.toggle_file_preview_pane();
        assert!(!app.show_file_preview_pane);
        app.toggle_file_preview_pane();
        assert!(app.show_file_preview_pane);
    }

    #[test]
    fn toggle_file_sidebar_flips_state() {
        let mut app = AuroraDesktopApp::new();
        assert!(app.show_file_sidebar);
        app.toggle_file_sidebar();
        assert!(!app.show_file_sidebar);
        app.toggle_file_sidebar();
        assert!(app.show_file_sidebar);
    }

    #[test]
    fn toggle_file_bars_flip_state() {
        let mut app = AuroraDesktopApp::new();
        assert!(app.show_file_path_bar);
        assert!(app.show_file_status_bar);
        app.toggle_file_path_bar();
        app.toggle_file_status_bar();
        assert!(!app.show_file_path_bar);
        assert!(!app.show_file_status_bar);
    }

    #[test]
    fn file_manager_path_segments_build_clickable_breadcrumbs() {
        let path = PathBuf::from("C:/Users/test/Documents/Projects");
        let segments = AuroraDesktopApp::file_manager_path_segments(&path);
        assert_eq!(
            segments.last().map(|(label, _)| label.as_str()),
            Some("Projects")
        );
        assert_eq!(segments.last().map(|(_, path)| path.clone()), Some(path));
    }

    #[test]
    fn filter_file_manager_entries_matches_name_and_extension() {
        let entries = vec![
            FmEntry {
                name: "notes.txt".to_string(),
                is_dir: false,
                size: 10,
                path: PathBuf::from("C:/notes.txt"),
            },
            FmEntry {
                name: "photo.png".to_string(),
                is_dir: false,
                size: 20,
                path: PathBuf::from("C:/photo.png"),
            },
            FmEntry {
                name: "Projects".to_string(),
                is_dir: true,
                size: 0,
                path: PathBuf::from("C:/Projects"),
            },
        ];

        assert_eq!(
            AuroraDesktopApp::filter_file_manager_entries(&entries, "png").len(),
            1
        );
        assert_eq!(
            AuroraDesktopApp::filter_file_manager_entries(&entries, "proj").len(),
            1
        );
        assert_eq!(
            AuroraDesktopApp::filter_file_manager_entries(&entries, "").len(),
            3
        );
    }

    #[test]
    fn file_manager_view_mode_shortcuts_map_expected_modes() {
        assert_eq!(
            AuroraDesktopApp::file_manager_view_mode_from_shortcut(1),
            Some(FileManagerViewMode::Icon)
        );
        assert_eq!(
            AuroraDesktopApp::file_manager_view_mode_from_shortcut(2),
            Some(FileManagerViewMode::List)
        );
        assert_eq!(
            AuroraDesktopApp::file_manager_view_mode_from_shortcut(3),
            Some(FileManagerViewMode::Column)
        );
        assert_eq!(
            AuroraDesktopApp::file_manager_view_mode_from_shortcut(4),
            Some(FileManagerViewMode::Gallery)
        );
        assert_eq!(
            AuroraDesktopApp::file_manager_view_mode_from_shortcut(9),
            None
        );
    }

    #[test]
    fn file_manager_view_mode_persists_per_directory() {
        let mut app = AuroraDesktopApp::new();
        let docs = PathBuf::from("C:/Users/test/Documents");
        let pics = PathBuf::from("C:/Users/test/Pictures");

        app.fm_current_dir = docs.clone();
        app.set_file_manager_view_mode(FileManagerViewMode::Gallery);
        app.fm_current_dir = pics.clone();
        app.set_file_manager_view_mode(FileManagerViewMode::Icon);

        app.fm_current_dir = docs;
        app.sync_file_manager_view_mode_for_current_dir();
        assert_eq!(app.fm_view_mode, FileManagerViewMode::Gallery);

        app.fm_current_dir = pics;
        app.sync_file_manager_view_mode_for_current_dir();
        assert_eq!(app.fm_view_mode, FileManagerViewMode::Icon);
    }

    #[test]
    fn sort_file_manager_entries_keeps_directories_first_and_sorts_by_field() {
        let entries = vec![
            FmEntry {
                name: "zeta.txt".to_string(),
                is_dir: false,
                size: 40,
                path: PathBuf::from("C:/zeta.txt"),
            },
            FmEntry {
                name: "alpha".to_string(),
                is_dir: true,
                size: 0,
                path: PathBuf::from("C:/alpha"),
            },
            FmEntry {
                name: "beta.log".to_string(),
                is_dir: false,
                size: 10,
                path: PathBuf::from("C:/beta.log"),
            },
        ];

        let by_name =
            AuroraDesktopApp::sort_file_manager_entries(&entries, FileManagerSortField::Name);
        assert_eq!(by_name[0].name, "alpha");
        assert_eq!(by_name[1].name, "beta.log");

        let by_size =
            AuroraDesktopApp::sort_file_manager_entries(&entries, FileManagerSortField::Size);
        assert_eq!(by_size[0].name, "alpha");
        assert_eq!(by_size[1].name, "beta.log");
        assert_eq!(by_size[2].name, "zeta.txt");
    }

    #[test]
    fn gallery_next_index_clamps_within_bounds() {
        assert_eq!(AuroraDesktopApp::gallery_next_index(0, 0, 1), 0);
        assert_eq!(AuroraDesktopApp::gallery_next_index(3, 0, -1), 0);
        assert_eq!(AuroraDesktopApp::gallery_next_index(3, 0, 1), 1);
        assert_eq!(AuroraDesktopApp::gallery_next_index(3, 2, 1), 2);
    }

    #[test]
    fn request_file_info_tracks_target_path() {
        let mut app = AuroraDesktopApp::new();
        let path = PathBuf::from("C:/Users/test/Documents/note.txt");
        app.request_file_info(Some(path.clone()));
        assert_eq!(app.file_info_target, Some(path));
        app.request_file_info(None);
        assert_eq!(app.file_info_target, None);
    }

    #[test]
    fn smoke_file_manager_preview_flow_builds_preview_and_info_for_selected_entry() {
        let root = unique_temp_dir("aurora_smoke_preview_flow");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("note.txt");
        std::fs::write(&path, "hello preview").unwrap();

        let mut app = AuroraDesktopApp::new();
        app.fm_selected_path = Some(path.clone());
        app.request_file_info(app.fm_selected_path.clone());

        let preview = build_preview(app.fm_selected_path.as_ref().unwrap());
        let info = read_file_info(app.file_info_target.as_ref().unwrap());

        assert_eq!(preview.kind, PreviewKind::Text);
        assert!(preview.body.contains("hello preview"));
        assert_eq!(info.name, "note.txt");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn close_file_manager_tab_keeps_one_tab_minimum() {
        let mut app = AuroraDesktopApp::new();
        assert_eq!(app.fm_tabs.len(), 1);
        app.close_file_manager_tab(0);
        assert_eq!(app.fm_tabs.len(), 1);
    }

    #[test]
    fn open_file_manager_tab_adds_and_activates_tab() {
        let mut app = AuroraDesktopApp::new();
        let path = desktop_directory();
        app.open_file_manager_tab(path.clone());
        assert_eq!(app.fm_tabs.len(), 2);
        assert_eq!(app.fm_active_tab, 1);
        assert_eq!(app.fm_current_dir, path);
    }

    #[test]
    fn switch_file_manager_tab_restores_tab_state() {
        let mut app = AuroraDesktopApp::new();
        let original = app.fm_current_dir.clone();
        let second = desktop_directory();
        app.open_file_manager_tab(second);
        app.switch_file_manager_tab(0);
        assert_eq!(app.fm_current_dir, original);
        assert_eq!(app.fm_active_tab, 0);
    }

    #[test]
    fn move_file_manager_tab_reorders_tabs_and_active_index() {
        let mut app = AuroraDesktopApp::new();
        let desktop = desktop_directory();
        let documents = dirs_home().join("Documents");
        app.open_file_manager_tab(desktop.clone());
        app.open_file_manager_tab(documents.clone());
        app.move_file_manager_tab(2, 0);
        assert_eq!(app.fm_tabs[0].path, documents);
        assert_eq!(app.fm_active_tab, 0);
        assert_eq!(app.fm_current_dir, app.fm_tabs[0].path);
    }

    #[test]
    fn file_manager_directory_navigation_supports_new_tab_target() {
        let path = PathBuf::from("C:/Users/test/Documents");
        assert_eq!(
            AuroraDesktopApp::file_manager_directory_navigation(&path, false),
            path
        );
        assert_eq!(
            AuroraDesktopApp::file_manager_directory_navigation(
                &PathBuf::from("C:/Users/test/Documents"),
                true
            ),
            PathBuf::from("__OPEN_TAB__C:/Users/test/Documents")
        );
    }

    #[test]
    fn visible_tab_range_limits_overflow_window() {
        let range = AuroraDesktopApp::visible_tab_range(10, 3, 6);
        assert_eq!(range.start, 3);
        assert_eq!(range.end, 9);
        let clamped = AuroraDesktopApp::visible_tab_range(4, 9, 6);
        assert_eq!(clamped.start, 0);
        assert_eq!(clamped.end, 4);
    }

    #[test]
    fn open_file_manager_tab_advances_tab_scroll_for_overflow() {
        let mut app = AuroraDesktopApp::new();
        for idx in 0..7 {
            app.open_file_manager_tab(dirs_home().join(format!("Overflow-{idx}")));
        }
        assert!(app.fm_tab_scroll > 0);
        assert_eq!(app.fm_active_tab, app.fm_tabs.len() - 1);
    }

    #[test]
    fn battery_alert_level_respects_thresholds_and_charging_state() {
        assert_eq!(AuroraDesktopApp::battery_alert_level(false, 10.0, false), 0);
        assert_eq!(AuroraDesktopApp::battery_alert_level(true, 50.0, true), 0);
        assert_eq!(AuroraDesktopApp::battery_alert_level(true, 19.0, false), 1);
        assert_eq!(AuroraDesktopApp::battery_alert_level(true, 9.0, false), 2);
        assert_eq!(AuroraDesktopApp::battery_alert_level(true, 4.0, false), 3);
    }

    #[test]
    fn battery_time_remaining_label_changes_for_mode_and_charging() {
        let normal = AuroraDesktopApp::battery_time_remaining_label(50.0, false, false);
        let saver = AuroraDesktopApp::battery_time_remaining_label(50.0, false, true);
        let charging = AuroraDesktopApp::battery_time_remaining_label(50.0, true, false);
        assert!(normal.contains("remaining"));
        assert!(saver.contains("remaining"));
        assert!(charging.contains("until full"));
        assert_ne!(normal, saver);
    }

    #[test]
    fn tabbed_window_support_matches_expected_kinds() {
        assert!(AuroraDesktopApp::window_supports_tabs(WindowKind::Terminal));
        assert!(AuroraDesktopApp::window_supports_tabs(WindowKind::Notes));
        assert!(AuroraDesktopApp::window_supports_tabs(
            WindowKind::TextEditor
        ));
        assert!(!AuroraDesktopApp::window_supports_tabs(
            WindowKind::FileManager
        ));
        assert!(!AuroraDesktopApp::window_supports_tabs(
            WindowKind::Settings
        ));
    }

    #[test]
    fn open_and_cycle_note_tabs_updates_active_content() {
        let mut app = AuroraDesktopApp::new();
        app.notes_text = "Primary note".to_string();
        app.sync_active_tab_from_globals(WindowKind::Notes);

        app.open_window_tab(WindowKind::Notes);
        assert_eq!(app.window_tab_count(WindowKind::Notes), 2);
        assert_eq!(app.notes_active_tab, 1);
        assert_eq!(app.notes_text, "");

        app.notes_text = "Secondary note".to_string();
        app.sync_active_tab_from_globals(WindowKind::Notes);
        app.cycle_window_tab(WindowKind::Notes, -1);
        assert_eq!(app.notes_active_tab, 0);
        assert_eq!(app.notes_text, "Primary note");

        app.cycle_window_tab(WindowKind::Notes, 1);
        assert_eq!(app.notes_active_tab, 1);
        assert_eq!(app.notes_text, "Secondary note");
    }

    #[test]
    fn close_window_tab_keeps_one_editor_tab_minimum() {
        let mut app = AuroraDesktopApp::new();
        app.close_window_tab(WindowKind::TextEditor);
        assert_eq!(app.window_tab_count(WindowKind::TextEditor), 1);

        app.open_window_tab(WindowKind::TextEditor);
        assert_eq!(app.window_tab_count(WindowKind::TextEditor), 2);
        app.close_window_tab(WindowKind::TextEditor);

        assert_eq!(app.window_tab_count(WindowKind::TextEditor), 1);
        assert_eq!(app.editor_active_tab, 0);
    }

    #[test]
    fn syncing_editor_globals_updates_active_tab_title_and_content() {
        let mut app = AuroraDesktopApp::new();
        let path = PathBuf::from("C:/Users/test/Documents/report.txt");
        app.editor_file_path = Some(path.clone());
        app.editor_content = "draft".to_string();
        app.editor_modified = true;

        app.sync_active_tab_from_globals(WindowKind::TextEditor);

        let tab = &app.editor_tabs[app.editor_active_tab];
        assert_eq!(tab.file_path, Some(path));
        assert_eq!(tab.content, "draft");
        assert!(tab.modified);
        assert_eq!(tab.title, "report.txt");
    }

    #[test]
    fn pip_resize_preserves_sixteen_nine_aspect_ratio() {
        let resized = AuroraDesktopApp::pip_resized_size(
            AuroraDesktopApp::pip_small_size(),
            Vec2::new(80.0, 10.0),
        );
        assert!((resized.x / resized.y - (16.0 / 9.0)).abs() < 0.01);
        assert!(resized.x > AuroraDesktopApp::pip_small_size().x);
    }

    #[test]
    fn pip_snapped_position_chooses_nearest_corner() {
        let work_rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(1200.0, 800.0));
        let size = AuroraDesktopApp::pip_small_size();
        let snapped =
            AuroraDesktopApp::pip_snapped_position(work_rect, size, Pos2::new(920.0, 610.0));
        assert!(snapped.x > 800.0);
        assert!(snapped.y > 500.0);
    }

    #[test]
    fn pip_toggle_size_switches_between_presets() {
        assert_eq!(
            AuroraDesktopApp::pip_toggle_size(AuroraDesktopApp::pip_small_size()),
            AuroraDesktopApp::pip_medium_size()
        );
        assert_eq!(
            AuroraDesktopApp::pip_toggle_size(AuroraDesktopApp::pip_medium_size()),
            AuroraDesktopApp::pip_small_size()
        );
    }

    #[test]
    fn assistant_query_can_open_apps_and_seed_history() {
        let mut app = AuroraDesktopApp::new();
        app.run_assistant_query("Open Files");
        assert!(app.window_ref(WindowKind::FileManager).open);
        assert_eq!(app.assistant_history.len(), 2);
        assert!(app.assistant_history[1].text.contains("Opening"));
    }

    #[test]
    fn assistant_query_can_search_and_toggle_wifi() {
        let mut app = AuroraDesktopApp::new();
        app.run_assistant_query("Search for screenshots");
        assert!(app.show_spotlight);
        assert_eq!(app.spotlight_query, "screenshots");

        app.run_assistant_query("Turn off Wi-Fi");
        assert!(!app.cc_wifi);
        assert!(!app.app_settings.wifi_enabled);
    }

    #[test]
    fn assistant_query_creates_reminder_in_notes() {
        let mut app = AuroraDesktopApp::new();
        app.run_assistant_query("Remind me to ship the build");
        assert!(app.notes_text.contains("ship the build"));
        assert!(app
            .assistant_history
            .last()
            .unwrap()
            .text
            .contains("Reminder saved"));
    }

    #[test]
    fn queued_assistant_query_enters_thinking_state_before_response() {
        let mut app = AuroraDesktopApp::new();
        app.queue_assistant_query("Open Files");
        assert_eq!(app.assistant_state, AssistantOverlayState::Thinking);
        assert_eq!(app.assistant_history.len(), 1);
        assert!(app.pending_assistant_query.is_some());
    }

    #[test]
    fn pending_assistant_query_completes_and_appends_response() {
        let mut app = AuroraDesktopApp::new();
        app.queue_assistant_query("Open Files");
        if let Some((_, ready_at)) = app.pending_assistant_query.as_mut() {
            *ready_at = Instant::now() - Duration::from_millis(1);
        }
        app.maybe_process_assistant_query();
        assert_eq!(app.assistant_state, AssistantOverlayState::Idle);
        assert!(app.pending_assistant_query.is_none());
        assert_eq!(app.assistant_history.len(), 2);
        assert!(app.assistant_history[1].text.contains("Opening"));
    }

    #[test]
    fn open_pip_reuses_last_geometry_when_available() {
        let mut app = AuroraDesktopApp::new();
        let work_rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(1200.0, 800.0));
        app.pip_last_pos = Some(Pos2::new(100.0, 140.0));
        app.pip_last_size = Some(Vec2::new(360.0, 202.5));
        app.open_pip(PipSource::Music, work_rect);
        let pip = app.pip_state.unwrap();
        assert_eq!(pip.pos, Pos2::new(100.0, 140.0));
        assert_eq!(pip.size, Vec2::new(360.0, 202.5));
    }
}
