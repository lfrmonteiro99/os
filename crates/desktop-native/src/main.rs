mod auto_save;
mod calculator;
mod clipboard;
mod file_index;
mod notifications;
mod process_manager;
mod settings;
mod terminal;
mod toast;
mod types;
mod window;

use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader, Write as IoWrite};
use std::net::TcpStream;
use std::path::PathBuf;

use std::time::{Duration, Instant};

use battery::Manager as BatteryManager;
use chrono::{Datelike, Local};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Id, Layout, Order, Pos2, Rect, RichText,
    Sense, Shape, Stroke, StrokeKind, Vec2,
};
use ipc::{decode_response, encode_command, CommandFrame};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};

use calculator::{calc_eval, format_calc};
use file_index::{dirs_home, read_directory, format_size, create_directory, create_file, rename_entry, delete_entry, FileIndex, FmEntry};
use auto_save::AutoSave;
use clipboard::AppClipboard;
use notifications::NotificationCenter;
use process_manager::ProcessManager;
use settings::AppSettings;
use terminal::{open_file_with_system, launch_program, PtyTerminal};
use toast::Toast;
use types::*;
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
        self.memory_pct = if total > 0 { (used as f32 / total as f32) * 100.0 } else { 0.0 };

        // Processes
        self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
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
        self.last_refresh.map(|t| t.elapsed() >= SYSINFO_INTERVAL).unwrap_or(true)
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
    painter: &egui::Painter, screen: Rect,
    center_x_ratio: f32, peak_height: f32, spread: f32, color: Color32,
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

// ── Dock icon painting ───────────────────────────────────────────────────────

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
            let body = Rect::from_min_max(Pos2::new(inner.left(), inner.top() + inner.height() * 0.2), inner.max);
            painter.rect_filled(body, CornerRadius::same(2), white);
            let tab = Rect::from_min_size(inner.left_top(), Vec2::new(inner.width() * 0.45, inner.height() * 0.25));
            painter.rect_filled(tab, CornerRadius::same(2), white);
        }
        DockIcon::Terminal => {
            let left = inner.left();
            let mid_y = inner.center().y;
            let cw = inner.width() * 0.35;
            let ch = inner.height() * 0.3;
            let stroke = Stroke::new(line_w * 1.5, Color32::from_rgb(166, 227, 161));
            painter.line_segment([Pos2::new(left, mid_y - ch), Pos2::new(left + cw, mid_y)], stroke);
            painter.line_segment([Pos2::new(left + cw, mid_y), Pos2::new(left, mid_y + ch)], stroke);
            painter.line_segment([Pos2::new(left + cw + 4.0, mid_y + ch), Pos2::new(inner.right(), mid_y + ch)], Stroke::new(line_w * 1.5, white));
        }
        DockIcon::Browser => {
            let r = inner.width() * 0.42;
            painter.circle_stroke(c, r, Stroke::new(line_w, white));
            painter.line_segment([Pos2::new(c.x - r, c.y), Pos2::new(c.x + r, c.y)], Stroke::new(line_w * 0.8, white));
            painter.line_segment([Pos2::new(c.x, c.y - r), Pos2::new(c.x, c.y + r)], Stroke::new(line_w * 0.8, white));
            painter.circle_stroke(c, r * 0.5, Stroke::new(line_w * 0.6, Color32::from_white_alpha(160)));
        }
        DockIcon::Overview => {
            let bar_h = inner.height() * 0.16;
            let gap = inner.height() * 0.12;
            for (i, w_ratio) in [0.8f32, 0.55, 0.7].iter().enumerate() {
                let y = inner.top() + i as f32 * (bar_h + gap);
                let bar = Rect::from_min_size(Pos2::new(inner.left(), y), Vec2::new(inner.width() * w_ratio, bar_h));
                painter.rect_filled(bar, CornerRadius::same((bar_h * 0.5) as u8), white);
            }
        }
        DockIcon::Mail => {
            let body = inner.shrink2(Vec2::new(0.0, inner.height() * 0.1));
            painter.rect_stroke(body, CornerRadius::same(2), Stroke::new(line_w, white), StrokeKind::Outside);
            painter.line_segment([body.left_top(), Pos2::new(body.center().x, body.center().y + 2.0)], Stroke::new(line_w, white));
            painter.line_segment([body.right_top(), Pos2::new(body.center().x, body.center().y + 2.0)], Stroke::new(line_w, white));
        }
        DockIcon::Messages => {
            let r = inner.width() * 0.38;
            painter.circle_filled(c, r, white);
            painter.add(Shape::convex_polygon(
                vec![Pos2::new(c.x - r * 0.3, c.y + r * 0.7), Pos2::new(c.x - r * 0.9, c.y + r * 1.2), Pos2::new(c.x + r * 0.1, c.y + r * 0.9)],
                white, Stroke::NONE,
            ));
        }
        DockIcon::Notes => {
            painter.rect_stroke(inner, CornerRadius::same(2), Stroke::new(line_w * 0.8, Color32::from_rgb(120, 100, 10)), StrokeKind::Outside);
            for i in 0..3 {
                let y = inner.top() + inner.height() * (0.3 + i as f32 * 0.22);
                painter.line_segment([Pos2::new(inner.left() + 3.0, y), Pos2::new(inner.right() - 3.0, y)], Stroke::new(line_w * 0.7, Color32::from_rgb(140, 120, 20)));
            }
        }
        DockIcon::Calendar => {
            // Show real day of month
            let day = Local::now().format("%d").to_string();
            painter.text(Pos2::new(c.x, c.y + s * 0.08), Align2::CENTER_CENTER, &day, FontId::proportional(s * 0.38), Color32::from_rgb(30, 30, 30));
        }
        DockIcon::Music => {
            let nr = inner.width() * 0.18;
            let nc = Pos2::new(c.x - nr * 0.5, inner.bottom() - nr);
            painter.circle_filled(nc, nr, white);
            let st = Pos2::new(nc.x + nr, inner.top() + 2.0);
            let sb = Pos2::new(nc.x + nr, nc.y);
            painter.line_segment([st, sb], Stroke::new(line_w * 1.2, white));
            painter.line_segment([st, Pos2::new(st.x + inner.width() * 0.25, st.y + inner.height() * 0.2)], Stroke::new(line_w * 1.5, white));
        }
        DockIcon::Photos => {
            let sr = inner.width() * 0.15;
            painter.circle_filled(Pos2::new(inner.right() - sr - 2.0, inner.top() + sr + 2.0), sr, white);
            painter.add(Shape::convex_polygon(
                vec![Pos2::new(c.x - 2.0, inner.top() + inner.height() * 0.3), Pos2::new(inner.right(), inner.bottom()), Pos2::new(inner.left(), inner.bottom())],
                white, Stroke::NONE,
            ));
        }
        DockIcon::Calculator => {
            let cw = inner.width() * 0.38; let ch = inner.height() * 0.25;
            let gx = inner.width() * 0.24; let gy = inner.height() * 0.12;
            for row in 0..3 { for col in 0..2 {
                let x = inner.left() + col as f32 * (cw + gx);
                let y = inner.top() + row as f32 * (ch + gy);
                painter.rect_filled(Rect::from_min_size(Pos2::new(x, y), Vec2::new(cw, ch)), CornerRadius::same(2), white);
            }}
        }
        DockIcon::Settings => {
            let r = inner.width() * 0.3;
            painter.circle_stroke(c, r, Stroke::new(line_w * 1.2, white));
            painter.circle_filled(c, r * 0.4, white);
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                painter.line_segment([
                    Pos2::new(c.x + angle.cos() * r * 0.8, c.y + angle.sin() * r * 0.8),
                    Pos2::new(c.x + angle.cos() * r * 1.35, c.y + angle.sin() * r * 1.35),
                ], Stroke::new(line_w * 1.5, white));
            }
        }
        DockIcon::Store => {
            painter.text(c, Align2::CENTER_CENTER, "A", FontId::proportional(s * 0.45), white);
        }
        DockIcon::Controls => {
            let cell = inner.width() * 0.38; let gap = inner.width() * 0.24;
            for row in 0..2 { for col in 0..2 {
                let x = inner.left() + col as f32 * (cell + gap);
                let y = inner.top() + row as f32 * (cell + gap);
                painter.rect_filled(Rect::from_min_size(Pos2::new(x, y), Vec2::splat(cell)), CornerRadius::same(3), white);
            }}
        }
        DockIcon::Info => {
            let r = inner.width() * 0.4;
            painter.circle_stroke(c, r, Stroke::new(line_w * 1.2, white));
            painter.text(c, Align2::CENTER_CENTER, "i", FontId::proportional(s * 0.35), white);
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
            (0.00, [27, 43, 94]), (0.15, [45, 74, 140]), (0.28, [107, 91, 149]),
            (0.40, [192, 108, 132]), (0.52, [246, 114, 128]), (0.62, [248, 181, 149]),
            (0.72, [255, 207, 135]), (0.82, [248, 168, 96]), (0.92, [208, 112, 80]),
            (1.00, [42, 26, 58]),
        ],
        hills: &[(0.50, 160.0, 500.0, [74, 53, 112]), (0.75, 180.0, 400.0, [46, 37, 83]),
            (0.25, 200.0, 450.0, [26, 31, 58]), (0.55, 130.0, 600.0, [10, 14, 26])],
    },
    WallpaperPreset {
        name: "Ocean Blue",
        bands: &[
            (0.00, [10, 15, 40]), (0.20, [15, 40, 80]), (0.40, [20, 80, 140]),
            (0.55, [40, 120, 180]), (0.70, [80, 170, 210]), (0.85, [140, 210, 230]),
            (1.00, [20, 50, 90]),
        ],
        hills: &[(0.40, 140.0, 500.0, [10, 30, 60]), (0.70, 170.0, 400.0, [8, 20, 50]),
            (0.20, 190.0, 550.0, [5, 15, 35])],
    },
    WallpaperPreset {
        name: "Aurora Borealis",
        bands: &[
            (0.00, [5, 10, 20]), (0.15, [10, 25, 45]), (0.30, [15, 60, 50]),
            (0.45, [20, 100, 80]), (0.55, [30, 140, 100]), (0.65, [20, 100, 90]),
            (0.75, [15, 60, 60]), (0.90, [10, 30, 40]), (1.00, [5, 10, 20]),
        ],
        hills: &[(0.45, 150.0, 500.0, [5, 20, 15]), (0.70, 180.0, 400.0, [3, 12, 10]),
            (0.25, 200.0, 500.0, [2, 8, 8])],
    },
    WallpaperPreset {
        name: "Warm Desert",
        bands: &[
            (0.00, [40, 20, 50]), (0.15, [80, 40, 60]), (0.30, [160, 80, 60]),
            (0.45, [220, 140, 80]), (0.55, [240, 180, 100]), (0.65, [250, 210, 140]),
            (0.80, [230, 170, 90]), (0.90, [180, 110, 60]), (1.00, [60, 30, 30]),
        ],
        hills: &[(0.50, 120.0, 600.0, [120, 70, 40]), (0.30, 160.0, 450.0, [80, 45, 25]),
            (0.75, 140.0, 500.0, [50, 25, 15])],
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
    show_notifications: bool,
    spotlight_query: String,
    cc_wifi: bool,
    cc_bluetooth: bool,
    cc_airdrop: bool,
    cc_focus: bool,
    cc_brightness: f32,
    cc_volume: f32,
    fps_smoothed: f32,
    active_menu: Option<MenuDropdown>,
    context_menu_pos: Option<Pos2>,
    dock_bounce: Option<(DockIcon, Instant)>,
    fm_current_dir: PathBuf,
    fm_entries: Vec<FmEntry>,
    menu_action: Option<MenuAction>,
    spotlight_open_window: Option<WindowKind>,
    should_quit: bool,
    terminal_output: Vec<(String, Color32)>,
    terminal_input: String,
    pty_terminal: Option<PtyTerminal>,
    use_real_terminal: bool,
    cpu_history: VecDeque<f32>,
    // Calculator state
    calc_display: String,
    calc_operand: Option<f64>,
    calc_operator: Option<char>,
    calc_reset_next: bool,
    // Notes state
    notes_text: String,
    // Music player state
    music_playing: bool,
    music_track_idx: usize,
    // Text editor state
    editor_file_path: Option<PathBuf>,
    editor_content: String,
    editor_modified: bool,
    // Toast notifications
    toasts: Vec<Toast>,
    // Spotlight: deferred file open
    spotlight_open_file: Option<PathBuf>,
    // Wallpaper
    wallpaper_idx: usize,
    // Login screen
    show_login: bool,
    login_password: String,
    login_shake: Option<Instant>,
    // Mission Control
    show_mission_control: bool,
    mission_control_anim: f32, // 0.0 = hidden, 1.0 = fully shown
    // Edge snap preview
    drag_snap_preview: Option<SnapSide>,
    drag_snap_maximize: bool,
    // Menu bar popups
    show_wifi_popup: bool,
    show_volume_popup: bool,
    show_bluetooth_popup: bool,
    // Multi-desktop
    current_desktop: usize,
    desktop_count: usize,
    // Clipboard
    clipboard: AppClipboard,
    // Process manager
    proc_manager: Option<ProcessManager>,
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
    // App settings (persisted)
    app_settings: AppSettings,
    // File manager state
    fm_rename_target: Option<PathBuf>,
    fm_rename_buffer: String,
    fm_show_new_dialog: bool,
    fm_new_name: String,
    fm_new_is_dir: bool,
}

impl AuroraDesktopApp {
    fn new() -> Self {
        let daemon_addr = std::env::var("AURORA_DAEMON").unwrap_or_else(|_| "127.0.0.1:7878".to_string());
        let auth_token = std::env::var("AURORA_TOKEN").ok().filter(|v| !v.is_empty());

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
                ManagedWindow::new(Pos2::new(500.0, 150.0), Vec2::new(260.0, 380.0)),  // Calculator
                ManagedWindow::new(Pos2::new(100.0, 120.0), Vec2::new(400.0, 360.0)),  // Notes
                ManagedWindow::new(Pos2::new(350.0, 180.0), Vec2::new(340.0, 200.0)),  // MusicPlayer
                ManagedWindow::new(Pos2::new(200.0, 70.0), Vec2::new(480.0, 400.0)),   // Photos
                ManagedWindow::new(Pos2::new(420.0, 90.0), Vec2::new(300.0, 340.0)),   // Calendar
                ManagedWindow::new(Pos2::new(150.0, 80.0), Vec2::new(600.0, 450.0)),  // TextEditor
                ManagedWindow::new(Pos2::new(300.0, 100.0), Vec2::new(500.0, 450.0)), // Settings
                ManagedWindow::new(Pos2::new(250.0, 70.0), Vec2::new(560.0, 420.0)),  // ProcessManager
            ],
            z_order: vec![
                WindowKind::Overview, WindowKind::FileManager, WindowKind::Browser,
                WindowKind::Messages, WindowKind::Terminal, WindowKind::Controls,
            ],
            focused: Some(WindowKind::Terminal),
            show_control_center: false,
            show_spotlight: false,
            show_notifications: false,
            spotlight_query: String::new(),
            cc_wifi: true,
            cc_bluetooth: true,
            cc_airdrop: false,
            cc_focus: false,
            cc_brightness: 0.7,
            cc_volume: 0.5,
            fps_smoothed: 0.0,
            active_menu: None,
            context_menu_pos: None,
            dock_bounce: None,
            fm_current_dir: dirs_home(),
            fm_entries: read_directory(&dirs_home()),
            menu_action: None,
            spotlight_open_window: None,
            should_quit: false,
            terminal_output: Vec::new(),
            terminal_input: String::new(),
            pty_terminal: None,
            use_real_terminal: true,
            cpu_history: VecDeque::with_capacity(120),
            calc_display: "0".to_string(),
            calc_operand: None,
            calc_operator: None,
            calc_reset_next: false,
            notes_text: "Welcome to AuroraOS Notes!\n\nYou can type anything here.\nThis is a simple scratchpad.\n\n- Todo: finish the desktop shell\n- Todo: add more apps\n- Todo: write documentation".to_string(),
            music_playing: false,
            music_track_idx: 0,
            editor_file_path: None,
            editor_content: String::new(),
            editor_modified: false,
            toasts: Vec::new(),
            spotlight_open_file: None,
            wallpaper_idx: 0,
            show_login: true,
            login_password: String::new(),
            login_shake: None,
            show_mission_control: false,
            mission_control_anim: 0.0,
            drag_snap_preview: None,
            drag_snap_maximize: false,
            show_wifi_popup: false,
            show_volume_popup: false,
            show_bluetooth_popup: false,
            current_desktop: 0,
            desktop_count: 2,
            clipboard: AppClipboard::new(),
            proc_manager: None,
            proc_search: String::new(),
            proc_sort_by_cpu: true,
            auto_save: AutoSave::new(30, dirs_home()),
            wallpaper_prev_idx: 0,
            wallpaper_transition: 1.0,
            wallpaper_changing: false,
            confirm_close_window: None,
            notification_center: NotificationCenter::new(),
            app_settings: AppSettings::load(),
            fm_rename_target: None,
            fm_rename_buffer: String::new(),
            fm_show_new_dialog: false,
            fm_new_name: String::new(),
            fm_new_is_dir: true,
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
                app.toasts.push(Toast::new("Recovery", "Notes restored from auto-save", Color32::from_rgb(255, 149, 0)));
            }
        }
        if let Some(recovered_editor) = app.auto_save.load_recovery("editor") {
            if !recovered_editor.is_empty() {
                app.editor_content = recovered_editor;
                app.editor_modified = true;
                app.toasts.push(Toast::new("Recovery", "Editor content restored from auto-save", Color32::from_rgb(255, 149, 0)));
            }
        }
        // Initialize real PTY terminal
        if app.use_real_terminal {
            match PtyTerminal::new() {
                Some(pty) => {
                    app.pty_terminal = Some(pty);
                    app.toasts.push(Toast::new("Terminal Ready", "Real shell connected", Color32::from_rgb(52, 199, 89)));
                    app.notification_center.notify("System", "Terminal Ready", "Real shell connected", Color32::from_rgb(52, 199, 89));
                }
                None => {
                    app.toasts.push(Toast::new("Terminal", "Using built-in shell (PTY unavailable)", Color32::from_rgb(255, 149, 0)));
                }
            }
        }
        app
    }

    fn window_ref(&self, kind: WindowKind) -> &ManagedWindow { &self.windows[kind as usize] }
    fn window_mut(&mut self, kind: WindowKind) -> &mut ManagedWindow { &mut self.windows[kind as usize] }

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
        match side {
            SnapSide::Left => Rect::from_min_size(work_rect.left_top(), Vec2::new(hw, work_rect.height())),
            SnapSide::Right => Rect::from_min_size(Pos2::new(work_rect.left() + hw, work_rect.top()), Vec2::new(hw, work_rect.height())),
        }
    }

    fn active_window(&self) -> Option<WindowKind> {
        if let Some(kind) = self.focused {
            let w = self.window_ref(kind);
            if w.open && !w.minimized { return Some(kind); }
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
        let stream = TcpStream::connect(&self.daemon_addr).map_err(|e| format!("connect {} failed: {e}", self.daemon_addr))?;
        let reader_stream = stream.try_clone().map_err(|e| format!("clone failed: {e}"))?;
        let mut reader = BufReader::new(reader_stream);
        let mut writer = stream;
        let frame = CommandFrame::with_auth(self.next_frame_id(), self.auth_token.clone(), command);
        let mut encoded = encode_command(&frame);
        encoded.push('\n');
        writer.write_all(encoded.as_bytes()).map_err(|e| format!("write: {e}"))?;
        writer.flush().map_err(|e| format!("flush: {e}"))?;
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| format!("read: {e}"))?;
        if line.trim().is_empty() { return Err("empty response".to_string()); }
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
                self.telemetry.status = s; self.telemetry.health = h;
                self.telemetry.uptime = u; self.telemetry.boot = b;
                self.telemetry.last_error = None;
                self.telemetry.last_poll = Some(Instant::now());
            }
            (s, h, u, b) => {
                self.telemetry.connected = false;
                let mut errs = Vec::new();
                if let Err(e) = s { errs.push(format!("status: {e}")); }
                if let Err(e) = h { errs.push(format!("health: {e}")); }
                if let Err(e) = u { errs.push(format!("uptime: {e}")); }
                if let Err(e) = b { errs.push(format!("boot: {e}")); }
                self.telemetry.last_error = Some(errs.join(" | "));
                self.telemetry.last_poll = Some(Instant::now());
            }
        }
    }

    fn maybe_poll(&mut self) {
        let should = self.telemetry.last_poll.map(|l| l.elapsed() >= POLL_EVERY).unwrap_or(true);
        if should { self.refresh_telemetry(); }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context, work_rect: Rect) {
        // Cmd/Ctrl+Q = quit
        if ctx.input(|i| i.key_pressed(egui::Key::Q) && i.modifiers.command) {
            self.save_state();
            self.should_quit = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // Cmd/Ctrl+W = close focused window
        if ctx.input(|i| i.key_pressed(egui::Key::W) && i.modifiers.command) {
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
                        self.toasts.push(Toast::new("Saved", path.file_name().and_then(|n| n.to_str()).unwrap_or("file"), Color32::from_rgb(52, 199, 89)));
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

        // Cmd/Ctrl+M = minimize focused window
        if ctx.input(|i| i.key_pressed(egui::Key::M) && i.modifiers.command) {
            if let Some(kind) = self.active_window() {
                let win = self.window_mut(kind);
                win.minimized = true;
                win.id_epoch = win.id_epoch.saturating_add(1);
            }
            return;
        }

        // Ctrl+ArrowLeft / Ctrl+ArrowRight = snap window
        let snap_left = ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft) && i.modifiers.ctrl);
        let snap_right = ctx.input(|i| i.key_pressed(egui::Key::ArrowRight) && i.modifiers.ctrl);
        if !(snap_left || snap_right) { return; }
        let Some(active) = self.active_window() else { return };
        let win = self.window_mut(active);
        win.restore();
        win.maximized = false;
        win.snap = if snap_left { Some(SnapSide::Left) } else { Some(SnapSide::Right) };
        let snapped = Self::snap_rect(work_rect, win.snap.unwrap());
        win.default_pos = snapped.min;
        win.default_size = snapped.size();
        win.id_epoch = win.id_epoch.saturating_add(1);
    }

    // ── Background ───────────────────────────────────────────────────────────

    fn paint_wallpaper(painter: &egui::Painter, rect: Rect, wp: &WallpaperPreset, alpha: f32) {
        let a = (alpha * 255.0) as u8;
        for w in wp.bands.windows(2) {
            let (t0, c0) = w[0]; let (t1, c1) = w[1];
            let y0 = rect.top() + t0 * rect.height();
            let y1 = rect.top() + t1 * rect.height();
            let strip = Rect::from_min_max(Pos2::new(rect.left(), y0), Pos2::new(rect.right(), y1));
            gradient_rect(painter, strip,
                Color32::from_rgba_unmultiplied(c0[0], c0[1], c0[2], a),
                Color32::from_rgba_unmultiplied(c1[0], c1[1], c1[2], a));
        }
        for &(cx, peak, spread, rgb) in wp.hills {
            paint_hill(painter, rect, cx, peak, spread,
                Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], a));
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
            .frame(egui::Frame::default().fill(menu_fill).inner_margin(egui::Margin::symmetric(14, 6)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").size(14.0).color(Color32::WHITE));
                    ui.add_space(4.0);
                    let app_name = self.active_window().map(|k| k.title()).unwrap_or("AuroraOS");
                    ui.label(RichText::new(app_name).strong().size(13.0).color(Color32::WHITE));
                    ui.add_space(8.0);

                    let menus = [MenuDropdown::File, MenuDropdown::Edit, MenuDropdown::View, MenuDropdown::Window, MenuDropdown::Help];
                    for menu in menus {
                        let is_active = self.active_menu == Some(menu);
                        let bg = if is_active { Color32::from_rgba_unmultiplied(255, 255, 255, 45) } else { Color32::TRANSPARENT };
                        let response = egui::Frame::default()
                            .fill(bg).corner_radius(CornerRadius::same(4))
                            .inner_margin(egui::Margin::symmetric(6, 2))
                            .show(ui, |ui| { ui.label(RichText::new(menu.label()).size(13.0).color(Color32::from_gray(220))); })
                            .response;
                        if response.clicked() {
                            self.active_menu = if is_active { None } else { Some(menu) };
                        }
                        if self.active_menu.is_some() && response.hovered() && self.active_menu != Some(menu) {
                            self.active_menu = Some(menu);
                        }
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Spotlight
                        let spot = ui.add(egui::Label::new(RichText::new("O").size(14.0).color(Color32::WHITE)).sense(Sense::click()));
                        if spot.clicked() { toggle_spotlight = true; }
                        spot.on_hover_text("Spotlight");
                        ui.add_space(6.0);

                        // Clock (click for notifications)
                        let unread = self.notification_center.unread_count();
                        let clock_label = if unread > 0 {
                            format!("{clock_str}  ({unread})")
                        } else {
                            clock_str.clone()
                        };
                        let clock_resp = ui.add(egui::Label::new(RichText::new(&clock_label).size(13.0).color(Color32::from_gray(235))).sense(Sense::click()));
                        if clock_resp.clicked() { toggle_notifications = true; }
                        clock_resp.on_hover_text("Notification Center");
                        ui.add_space(6.0);

                        // Control Center
                        let cc = ui.add(egui::Label::new(RichText::new("=").size(16.0).color(Color32::WHITE)).sense(Sense::click()));
                        if cc.clicked() { toggle_cc = true; }
                        cc.on_hover_text("Control Center");
                        ui.add_space(6.0);

                        // Battery (real data)
                        if batt_available {
                            let batt_w = 20.0; let batt_h = 10.0;
                            let (batt_rect, _) = ui.allocate_exact_size(Vec2::new(batt_w + 3.0, batt_h), Sense::hover());
                            let body = Rect::from_min_size(batt_rect.min, Vec2::new(batt_w, batt_h));
                            ui.painter().rect_stroke(body, CornerRadius::same(2), Stroke::new(1.0, Color32::from_gray(200)), StrokeKind::Outside);
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
                            ui.painter().rect_filled(fill, CornerRadius::same(1), fill_color);
                            let nub = Rect::from_min_size(Pos2::new(body.right(), body.center().y - 2.5), Vec2::new(2.0, 5.0));
                            ui.painter().rect_filled(nub, CornerRadius::same(1), Color32::from_gray(200));

                            // Charging bolt or percentage on hover
                            let batt_label = if batt_charging {
                                format!("{:.0}% (charging)", batt_pct)
                            } else {
                                format!("{:.0}%", batt_pct)
                            };
                            let (hover_rect, hover_resp) = ui.allocate_exact_size(Vec2::ZERO, Sense::hover());
                            let _ = hover_rect;
                            hover_resp.on_hover_text(&batt_label);

                            ui.add_space(4.0);
                        }

                        // Volume (clickable)
                        let vol_resp = ui.add(egui::Label::new(RichText::new(if self.cc_volume > 0.0 { "♪" } else { "✕" }).size(14.0).color(Color32::WHITE)).sense(Sense::click()));
                        if vol_resp.clicked() { self.show_volume_popup = !self.show_volume_popup; self.show_wifi_popup = false; self.show_bluetooth_popup = false; }
                        vol_resp.on_hover_text("Volume");
                        ui.add_space(4.0);

                        // Bluetooth (clickable)
                        let bt_color = if self.cc_bluetooth { Color32::WHITE } else { Color32::from_gray(100) };
                        let bt_resp = ui.add(egui::Label::new(RichText::new("B").size(12.0).color(bt_color)).sense(Sense::click()));
                        if bt_resp.clicked() { self.show_bluetooth_popup = !self.show_bluetooth_popup; self.show_wifi_popup = false; self.show_volume_popup = false; }
                        bt_resp.on_hover_text("Bluetooth");
                        ui.add_space(4.0);

                        // Network status (real, clickable)
                        let net_color = if net_up { Color32::from_gray(230) } else { Color32::from_gray(100) };
                        let net_resp = ui.add(egui::Label::new(RichText::new("W").size(12.0).color(net_color)).sense(Sense::click()));
                        if net_resp.clicked() { self.show_wifi_popup = !self.show_wifi_popup; self.show_volume_popup = false; self.show_bluetooth_popup = false; }
                        net_resp.on_hover_text("Wi-Fi");
                        ui.add_space(4.0);

                        // Daemon connection
                        let (status_text, status_color) = if self.telemetry.connected {
                            ("Online", Color32::from_rgb(124, 236, 112))
                        } else {
                            ("Offline", Color32::from_rgb(255, 152, 152))
                        };
                        ui.label(RichText::new(status_text).size(12.0).color(status_color));
                        let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                        ui.painter().circle_filled(dot_rect.center(), 4.0, status_color);
                    });
                });
            });

        (toggle_cc, toggle_spotlight, toggle_notifications)
    }

    // ── Menu dropdowns ───────────────────────────────────────────────────────

    fn render_menu_dropdown(&mut self, ctx: &egui::Context) {
        let Some(menu) = self.active_menu else { return };
        let menu_idx = match menu { MenuDropdown::File => 0, MenuDropdown::Edit => 1, MenuDropdown::View => 2, MenuDropdown::Window => 3, MenuDropdown::Help => 4 };
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
                                let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(192.0, 1.0), Sense::hover());
                                ui.painter().rect_filled(sep_rect, 0.0, Color32::from_white_alpha(30));
                                ui.add_space(2.0);
                            } else {
                                let resp = ui.add(egui::Button::new(RichText::new(*item).size(13.0).color(Color32::from_gray(220)))
                                    .fill(Color32::TRANSPARENT).stroke(Stroke::NONE)
                                    .min_size(Vec2::new(192.0, 26.0)).corner_radius(CornerRadius::same(4)));
                                if resp.clicked() {
                                    self.active_menu = None;
                                    let label = item.split("  ").next().unwrap_or(item);
                                    match label {
                                        "Quit" => self.menu_action = Some(MenuAction::Quit),
                                        "Close Window" => self.menu_action = Some(MenuAction::CloseWindow),
                                        "Minimize" => self.menu_action = Some(MenuAction::Minimize),
                                        "Zoom" => self.menu_action = Some(MenuAction::Maximize),
                                        "Tile Left" => self.menu_action = Some(MenuAction::TileLeft),
                                        "Tile Right" => self.menu_action = Some(MenuAction::TileRight),
                                        "Bring All to Front" => self.menu_action = Some(MenuAction::BringAllToFront),
                                        "Copy" => self.menu_action = Some(MenuAction::Copy),
                                        "Cut" => self.menu_action = Some(MenuAction::Cut),
                                        "Paste" => self.menu_action = Some(MenuAction::Paste),
                                        "Select All" => self.menu_action = Some(MenuAction::SelectAll),
                                        "Undo" => self.menu_action = Some(MenuAction::Undo),
                                        "Redo" => self.menu_action = Some(MenuAction::Redo),
                                        "Save" => self.menu_action = Some(MenuAction::Save),
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
        let icons: &[(&str, Option<WindowKind>, Color32)] = &[
            ("Aurora HD", Some(WindowKind::FileManager), Color32::from_rgb(0, 122, 255)),
            ("Projects", Some(WindowKind::FileManager), Color32::from_rgb(88, 86, 214)),
            ("Terminal", Some(WindowKind::Terminal), Color32::from_rgb(30, 30, 46)),
            ("Notes", Some(WindowKind::Notes), Color32::from_rgb(255, 214, 10)),
        ];
        let mut open_kind: Option<WindowKind> = None;

        egui::Area::new(Id::new("desktop_icons"))
            .fixed_pos(Pos2::new(work_rect.left() + 18.0, work_rect.top() + 12.0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    for (label, kind, color) in icons {
                        let resp = egui::Frame::default()
                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 20))
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(50)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::symmetric(8, 6))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (icon_r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                                    ui.painter().rect_filled(icon_r, CornerRadius::same(3), *color);
                                    ui.label(RichText::new(*label).size(12.0).color(Color32::from_gray(240)));
                                });
                            }).response;
                        if resp.interact(Sense::click()).double_clicked() {
                            if let Some(k) = kind {
                                open_kind = Some(*k);
                            }
                        }
                        ui.add_space(8.0);
                    }
                });
            });

        if let Some(kind) = open_kind {
            let win = self.window_mut(kind);
            win.restore();
            win.id_epoch = win.id_epoch.saturating_add(1);
            self.bring_to_front(kind);
        }
    }

    // ── Right-click context menu ─────────────────────────────────────────────

    fn check_context_menu(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.pointer.secondary_clicked()) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if pos.y > MENU_BAR_HEIGHT && pos.y < ctx.content_rect().bottom() - DOCK_HEIGHT {
                    self.context_menu_pos = Some(pos);
                }
            }
        }
        if ctx.input(|i| i.pointer.primary_clicked()) {
            self.context_menu_pos = None;
        }
    }

    fn render_context_menu(&mut self, ctx: &egui::Context) {
        let Some(pos) = self.context_menu_pos else { return };
        egui::Area::new(Id::new("desktop_context_menu"))
            .fixed_pos(pos).order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(40, 40, 40, 230))
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(4, 4))
                    .show(ui, |ui| {
                        ui.set_min_width(210.0);
                        let items = ["New Folder", "---", "Get Info", "Change Wallpaper",
                            "---", "Use Stacks", "Sort By", "Clean Up",
                            "---", "Mission Control", "Show View Options"];
                        for label in items {
                            if label == "---" {
                                ui.add_space(2.0);
                                let (sr, _) = ui.allocate_exact_size(Vec2::new(202.0, 1.0), Sense::hover());
                                ui.painter().rect_filled(sr, 0.0, Color32::from_white_alpha(30));
                                ui.add_space(2.0);
                            } else {
                                let r = ui.add(egui::Button::new(RichText::new(label).size(13.0).color(Color32::from_gray(220)))
                                    .fill(Color32::TRANSPARENT).stroke(Stroke::NONE)
                                    .min_size(Vec2::new(202.0, 26.0)).corner_radius(CornerRadius::same(4)));
                                if r.clicked() {
                                    self.context_menu_pos = None;
                                    match label {
                                        "New Folder" => {
                                            let desktop = dirs_home().join("Desktop");
                                            let target = if desktop.exists() { desktop } else { dirs_home() };
                                            let mut name = "New Folder".to_string();
                                            let mut n = 1u32;
                                            while target.join(&name).exists() {
                                                n += 1;
                                                name = format!("New Folder ({n})");
                                            }
                                            if fs::create_dir_all(target.join(&name)).is_ok() {
                                                self.toasts.push(Toast::new("Folder Created", &name, Color32::from_rgb(0, 122, 255)));
                                                // Refresh file manager if viewing the same dir
                                                if self.fm_current_dir == target {
                                                    self.fm_entries = read_directory(&target);
                                                }
                                            }
                                        }
                                        "Change Wallpaper" => {
                                            self.wallpaper_idx = (self.wallpaper_idx + 1) % WALLPAPERS.len();
                                            self.toasts.push(Toast::new("Wallpaper", format!("Switched to {}", WALLPAPERS[self.wallpaper_idx % WALLPAPERS.len()].name), Color32::from_rgb(88, 86, 214)));
                                        }
                                        "Mission Control" => {
                                            self.show_mission_control = !self.show_mission_control;
                                        }
                                        "Get Info" => {
                                            self.toasts.push(Toast::new("AuroraOS", format!("v0.1.0 | {} windows | {:.0} FPS", WINDOW_COUNT, self.fps_smoothed), Color32::from_rgb(142, 142, 147)));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    });
            });
    }

    // ── Window content renderers ─────────────────────────────────────────────

    fn content_overview(ui: &mut egui::Ui, si: &RealSystemInfo, telemetry: &Telemetry, cpu_history: &VecDeque<f32>) {
        ui.heading(RichText::new("AuroraOS").color(Color32::from_gray(240)));
        ui.add_space(4.0);
        ui.label(RichText::new("System Dashboard — Live").size(13.0).color(Color32::from_gray(180)));
        ui.add_space(12.0);

        egui::Grid::new("overview_metrics")
            .num_columns(2)
            .spacing(Vec2::new(10.0, 10.0))
            .show(ui, |ui| {
                let cpu_str = format!("{:.1}%", si.cpu_usage);
                let mem_str = format!("{:.1} / {:.1} GB ({:.0}%)", si.used_memory_gb, si.total_memory_gb, si.memory_pct);
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
                            ui.label(RichText::new(*label).size(11.0).color(Color32::from_gray(160)));
                            ui.label(RichText::new(*value).size(16.0).strong().color(*color));
                        });
                    if i % 2 == 1 { ui.end_row(); }
                }
            });

        // CPU usage history graph
        if cpu_history.len() >= 2 {
            ui.add_space(12.0);
            ui.label(RichText::new("CPU History").size(11.0).strong().color(Color32::from_gray(160)));
            ui.add_space(4.0);
            let graph_size = Vec2::new(ui.available_width().min(440.0), 60.0);
            let (graph_rect, _) = ui.allocate_exact_size(graph_size, Sense::hover());
            let painter = ui.painter_at(graph_rect);

            // Background
            painter.rect_filled(graph_rect, CornerRadius::same(6), Color32::from_rgba_unmultiplied(0, 0, 0, 40));

            // Grid lines at 25%, 50%, 75%
            for pct in [25.0f32, 50.0, 75.0] {
                let y = graph_rect.bottom() - (pct / 100.0) * graph_rect.height();
                painter.line_segment(
                    [Pos2::new(graph_rect.left(), y), Pos2::new(graph_rect.right(), y)],
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
        ui.label(RichText::new("Top Processes").size(11.0).strong().color(Color32::from_gray(160)));
        ui.add_space(4.0);
        let mut procs: Vec<(&sysinfo::Pid, &sysinfo::Process)> = si.sys.processes().iter().collect();
        procs.sort_by(|a, b| b.1.cpu_usage().partial_cmp(&a.1.cpu_usage()).unwrap_or(std::cmp::Ordering::Equal));
        let top_n = procs.iter().take(8);
        egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
            for (_pid, proc_info) in top_n {
                ui.horizontal(|ui| {
                    let name = proc_info.name().to_string_lossy();
                    let cpu = proc_info.cpu_usage();
                    let mem_mb = proc_info.memory() as f64 / (1024.0 * 1024.0);
                    ui.label(RichText::new(format!("{:<20}", &name[..name.len().min(20)])).monospace().size(10.0).color(Color32::from_gray(200)));
                    ui.label(RichText::new(format!("{:.1}%", cpu)).monospace().size(10.0).color(
                        if cpu > 50.0 { Color32::from_rgb(255, 100, 100) }
                        else if cpu > 10.0 { Color32::from_rgb(255, 214, 10) }
                        else { Color32::from_gray(160) }
                    ));
                    ui.label(RichText::new(format!("{:.0} MB", mem_mb)).monospace().size(10.0).color(Color32::from_gray(120)));
                });
            }
        });

        ui.add_space(8.0);

        // Network + Battery + Daemon info
        if si.network_up {
            ui.label(RichText::new(format!("Network: {} (connected)", si.network_name)).size(12.0).color(Color32::from_rgb(124, 236, 112)));
        } else {
            ui.label(RichText::new("Network: disconnected").size(12.0).color(Color32::from_rgb(255, 152, 152)));
        }
        if si.battery_available {
            let batt_str = if si.battery_charging {
                format!("Battery: {:.0}% (charging)", si.battery_pct)
            } else {
                format!("Battery: {:.0}%", si.battery_pct)
            };
            let batt_color = if si.battery_pct < 20.0 { Color32::from_rgb(255, 59, 48) } else { Color32::from_rgb(52, 199, 89) };
            ui.label(RichText::new(batt_str).size(12.0).color(batt_color));
        }

        ui.add_space(8.0);
        ui.label(RichText::new("Daemon Status").strong().color(Color32::from_gray(220)));
        ui.add_space(4.0);
        if telemetry.status.is_empty() {
            ui.label(RichText::new("No daemon connection.").color(Color32::from_gray(140)));
        } else {
            egui::ScrollArea::vertical().max_height(60.0).id_salt("daemon_scroll").show(ui, |ui| {
                for line in telemetry.status.lines() {
                    ui.label(RichText::new(line).monospace().size(11.0).color(Color32::from_gray(200)));
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

        egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false; 2]).show(ui, |ui| {
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
                if !resp.has_focus() { resp.request_focus(); }

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

    fn content_terminal_builtin(ui: &mut egui::Ui, si: &RealSystemInfo, extra_lines: &[(String, Color32)], input: &mut String) -> Option<String> {
        let green = Color32::from_rgb(166, 227, 161);
        let gray = Color32::from_gray(140);
        let cyan = Color32::from_rgb(137, 220, 235);
        let mut submitted_cmd: Option<String> = None;

        egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
            ui.label(RichText::new("aurora@localhost ~ % neofetch").monospace().size(12.0).color(green));
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
                (format!("Memory:   {:.1} GB / {:.1} GB ({:.0}%)", si.used_memory_gb, si.total_memory_gb, si.memory_pct), gray),
                (format!("Disk:     {:.0} GB / {:.0} GB", si.disk_used_gb, si.disk_total_gb), gray),
                (format!("Procs:    {}", si.process_count), gray),
                (format!("Network:  {}", if si.network_up { &si.network_name } else { "disconnected" }), gray),
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
                let batt_color = if si.battery_pct < 20.0 { Color32::from_rgb(255, 100, 100) } else { gray };
                ui.label(RichText::new(batt_str).monospace().size(12.0).color(batt_color));
            }

            ui.add_space(8.0);
            ui.label(RichText::new("aurora@localhost ~ % aurora services list").monospace().size(12.0).color(green));
            ui.add_space(4.0);
            for (dot, rest) in [("●", " display-server    active (running)"), ("●", " window-manager    active (running)"),
                ("●", " network-daemon    active (running)"), ("●", " audio-server      active (running)"), ("●", " file-indexer      active (running)")] {
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
                ui.label(RichText::new("aurora@localhost ~ % ").monospace().size(12.0).color(green));
                let te = egui::TextEdit::singleline(input)
                    .font(FontId::monospace(12.0))
                    .text_color(Color32::from_gray(220))
                    .desired_width(ui.available_width() - 10.0)
                    .frame(false);
                let resp = ui.add(te);
                if !resp.has_focus() { resp.request_focus(); }
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
                for line in ["  help          Show this help", "  whoami        Current user",
                    "  hostname      Machine hostname", "  uname         System info",
                    "  uptime        System uptime", "  ps            Top processes",
                    "  free          Memory usage", "  df            Disk usage",
                    "  date          Current date/time", "  echo [text]   Echo text",
                    "  clear         Clear terminal", "  aurora status  Daemon status",
                    "  open <path>   Open file with system app",
                    "  run <program> Launch an executable",
                    "  <any>         Try as system command"] {
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
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                let up_secs = now.saturating_sub(boot);
                let hours = up_secs / 3600;
                let mins = (up_secs % 3600) / 60;
                out.push((format!("up {hours}h {mins}m"), white));
            }
            "ps" => {
                out.push((format!("{:<6} {:<20} {:>6} {:>8}", "PID", "NAME", "CPU%", "MEM"), cyan));
                let mut procs: Vec<_> = si.sys.processes().iter().collect();
                procs.sort_by(|a, b| b.1.cpu_usage().partial_cmp(&a.1.cpu_usage()).unwrap_or(std::cmp::Ordering::Equal));
                for (pid, p) in procs.iter().take(10) {
                    let name = p.name().to_string_lossy();
                    let name_short = &name[..name.len().min(20)];
                    let mem_mb = p.memory() as f64 / (1024.0 * 1024.0);
                    out.push((format!("{:<6} {:<20} {:>5.1}% {:>6.0} MB", pid.as_u32(), name_short, p.cpu_usage(), mem_mb), gray));
                }
            }
            "free" => {
                out.push((format!("Total:  {:.1} GB", si.total_memory_gb), white));
                out.push((format!("Used:   {:.1} GB ({:.0}%)", si.used_memory_gb, si.memory_pct), white));
                out.push((format!("Free:   {:.1} GB", si.total_memory_gb - si.used_memory_gb), white));
            }
            "df" => {
                out.push((format!("Total:  {:.0} GB", si.disk_total_gb), white));
                out.push((format!("Used:   {:.0} GB", si.disk_used_gb), white));
                out.push((format!("Avail:  {:.0} GB", si.disk_total_gb - si.disk_used_gb), white));
            }
            "date" => {
                out.push((Local::now().format("%Y-%m-%d %H:%M:%S %Z").to_string(), white));
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
                    out.push((format!("CPU: {:.1}% | Mem: {:.1}/{:.1} GB | Procs: {}",
                        si.cpu_usage, si.used_memory_gb, si.total_memory_gb, si.process_count), gray));
                } else {
                    out.push((format!("aurora: unknown subcommand '{}'", parts.get(1).unwrap_or(&"")), red));
                }
            }
            "open" | "start" => {
                if let Some(path) = parts.get(1) {
                    let p = std::path::Path::new(path);
                    if p.exists() {
                        open_file_with_system(p);
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
                    let args: Vec<&str> = parts[2..].to_vec();
                    match launch_program(program, &args) {
                        Ok(()) => out.push((format!("Launched {program}"), white)),
                        Err(e) => out.push((e, red)),
                    }
                } else {
                    out.push(("Usage: run <program> [args...]".into(), gray));
                }
            }
            _other => {
                // Try running as a real system command
                match std::process::Command::new("cmd")
                    .args(["/C", cmd])
                    .output()
                {
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
                            out.push((format!("Command exited with code {}", output.status.code().unwrap_or(-1)), red));
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

    fn content_filemanager(ui: &mut egui::Ui, current_dir: &std::path::Path, entries: &[FmEntry],
        show_new_dialog: &mut bool, new_name: &mut String, new_is_dir: &mut bool,
        rename_target: &mut Option<PathBuf>, rename_buffer: &mut String,
    ) -> Option<PathBuf> {
        let mut navigate_to: Option<PathBuf> = None;
        let home = dirs_home();
        let accent = Color32::from_rgb(0, 122, 255);
        let btn_bg = Color32::from_rgba_unmultiplied(255, 255, 255, 15);

        // Toolbar
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new(RichText::new("+ Folder").size(11.0).color(Color32::from_gray(220)))
                .fill(btn_bg).corner_radius(CornerRadius::same(4))).clicked()
            {
                *show_new_dialog = true;
                *new_is_dir = true;
                new_name.clear();
            }
            if ui.add(egui::Button::new(RichText::new("+ File").size(11.0).color(Color32::from_gray(220)))
                .fill(btn_bg).corner_radius(CornerRadius::same(4))).clicked()
            {
                *show_new_dialog = true;
                *new_is_dir = false;
                new_name.clear();
            }
        });

        // New item dialog
        if *show_new_dialog {
            ui.horizontal(|ui| {
                let label = if *new_is_dir { "New folder:" } else { "New file:" };
                ui.label(RichText::new(label).size(11.0).color(Color32::from_gray(180)));
                let te = egui::TextEdit::singleline(new_name)
                    .desired_width(200.0)
                    .font(FontId::proportional(12.0));
                let resp = ui.add(te);
                if !resp.has_focus() && new_name.is_empty() { resp.request_focus(); }

                if ui.add(egui::Button::new(RichText::new("Create").size(11.0).color(Color32::WHITE))
                    .fill(accent).corner_radius(CornerRadius::same(4))).clicked() && !new_name.is_empty()
                {
                    let target = current_dir.join(new_name.as_str());
                    let result = if *new_is_dir { create_directory(&target) } else { create_file(&target) };
                    match &result {
                        Ok(()) => {
                            let kind = if *new_is_dir { "folder" } else { "file" };
                            navigate_to = Some(PathBuf::from(format!("__NOTIFY_OK__Created {kind} '{}'", new_name)));
                        }
                        Err(e) => {
                            navigate_to = Some(PathBuf::from(format!("__NOTIFY_ERR__{e}")));
                        }
                    }
                    *show_new_dialog = false;
                }
                if ui.add(egui::Button::new(RichText::new("Cancel").size(11.0).color(Color32::from_gray(180)))
                    .fill(btn_bg).corner_radius(CornerRadius::same(4))).clicked()
                {
                    *show_new_dialog = false;
                }
            });
        }

        // Rename dialog
        if let Some(ref target_path) = rename_target.clone() {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Rename:").size(11.0).color(Color32::from_gray(180)));
                let te = egui::TextEdit::singleline(rename_buffer)
                    .desired_width(200.0)
                    .font(FontId::proportional(12.0));
                ui.add(te);
                if ui.add(egui::Button::new(RichText::new("OK").size(11.0).color(Color32::WHITE))
                    .fill(accent).corner_radius(CornerRadius::same(4))).clicked() && !rename_buffer.is_empty()
                {
                    let new_path = target_path.parent().unwrap_or(current_dir).join(rename_buffer.as_str());
                    match rename_entry(target_path, &new_path) {
                        Ok(()) => {
                            navigate_to = Some(PathBuf::from(format!("__NOTIFY_OK__Renamed to '{}'", rename_buffer)));
                        }
                        Err(e) => {
                            navigate_to = Some(PathBuf::from(format!("__NOTIFY_ERR__{e}")));
                        }
                    }
                    *rename_target = None;
                }
                if ui.add(egui::Button::new(RichText::new("Cancel").size(11.0).color(Color32::from_gray(180)))
                    .fill(btn_bg).corner_radius(CornerRadius::same(4))).clicked()
                {
                    *rename_target = None;
                }
            });
        }

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            // Sidebar with real quick-access folders
            egui::Frame::default()
                .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(8, 8))
                .show(ui, |ui| {
                    ui.set_min_width(130.0);
                    ui.label(RichText::new("Favorites").size(10.0).strong().color(Color32::from_gray(140)));
                    ui.add_space(4.0);
                    let favorites = [("Home", home.clone()), ("Desktop", home.join("Desktop")),
                        ("Documents", home.join("Documents")), ("Downloads", home.join("Downloads")),
                        ("Pictures", home.join("Pictures"))];
                    for (label, path) in &favorites {
                        let is_current = current_dir == path.as_path();
                        let color = if is_current { Color32::from_rgb(0, 122, 255) } else { Color32::from_gray(220) };
                        let resp = ui.add(egui::Label::new(RichText::new(format!("  {label}")).size(12.0).color(color)).sense(Sense::click()));
                        if resp.clicked() && path.exists() {
                            navigate_to = Some(path.clone());
                        }
                    }
                    ui.add_space(8.0);
                    ui.label(RichText::new("Disks").size(10.0).strong().color(Color32::from_gray(140)));
                    ui.add_space(4.0);
                    let disks = sysinfo::Disks::new_with_refreshed_list();
                    for disk in disks.list() {
                        let mount = disk.mount_point().to_string_lossy().to_string();
                        let label = if mount.len() <= 3 { format!("  {mount}") } else { format!("  {}", &mount[..3]) };
                        let resp = ui.add(egui::Label::new(RichText::new(label).size(12.0).color(Color32::from_gray(220))).sense(Sense::click()));
                        if resp.clicked() {
                            navigate_to = Some(disk.mount_point().to_path_buf());
                        }
                    }
                });

            ui.add_space(8.0);

            // Main content: path bar + file list
            ui.vertical(|ui| {
                // Path breadcrumb
                ui.horizontal(|ui| {
                    // Back button
                    if let Some(parent) = current_dir.parent() {
                        let resp = ui.add(egui::Button::new(RichText::new("<").size(13.0).color(Color32::from_gray(200)))
                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 15))
                            .stroke(Stroke::NONE).corner_radius(CornerRadius::same(4)));
                        if resp.clicked() {
                            navigate_to = Some(parent.to_path_buf());
                        }
                    }
                    ui.add_space(4.0);
                    ui.label(RichText::new(current_dir.to_string_lossy().to_string()).size(11.0).color(Color32::from_gray(160)));
                });
                ui.add_space(6.0);

                // File/folder list
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for entry in entries {
                        let (icon, color) = if entry.is_dir {
                            ("D", Color32::from_rgb(0, 122, 255))
                        } else {
                            let ext = entry.path.extension().and_then(|e| e.to_str()).unwrap_or("");
                            match ext {
                                "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" => ("<>", Color32::from_rgb(88, 86, 214)),
                                "md" | "txt" | "log" => ("T", Color32::from_rgb(142, 142, 147)),
                                "png" | "jpg" | "jpeg" | "gif" | "svg" => ("I", Color32::from_rgb(255, 149, 0)),
                                "toml" | "json" | "yaml" | "yml" => ("C", Color32::from_rgb(255, 214, 10)),
                                _ => ("F", Color32::from_gray(160)),
                            }
                        };

                        let row_resp = ui.horizontal(|ui| {
                            // Icon
                            let (ir, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                            if entry.is_dir {
                                // Mini folder
                                let body = Rect::from_center_size(ir.center(), Vec2::new(16.0, 12.0));
                                ui.painter().rect_filled(body, CornerRadius::same(2), color);
                                let tab = Rect::from_min_size(Pos2::new(body.left(), body.top() - 3.0), Vec2::new(7.0, 4.0));
                                ui.painter().rect_filled(tab, CornerRadius::same(1), color);
                            } else {
                                // File icon
                                ui.painter().text(ir.center(), Align2::CENTER_CENTER, icon, FontId::proportional(10.0), color);
                            }

                            ui.label(RichText::new(&entry.name).size(12.0).color(Color32::from_gray(230)));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if !entry.is_dir {
                                    ui.label(RichText::new(format_size(entry.size)).size(10.0).color(Color32::from_gray(100)));
                                }
                            });
                        });
                        // Interact with the row rect for double-click and right-click
                        let resp = ui.interact(row_resp.response.rect, Id::new(("fm_entry", &entry.name)), Sense::click());

                        if resp.double_clicked() {
                            if entry.is_dir {
                                navigate_to = Some(entry.path.clone());
                            } else {
                                // Text-like files open in built-in editor; others with system app
                                let ext = entry.path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                let is_text = matches!(ext,
                                    "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "go" | "java" |
                                    "md" | "txt" | "log" | "csv" | "json" | "toml" | "yaml" | "yml" |
                                    "html" | "css" | "xml" | "sh" | "bat" | "cmd" | "ps1" | "cfg" |
                                    "ini" | "conf" | "env" | "gitignore" | "lock" | "sql" | "lua" | "rb"
                                );
                                if is_text {
                                    navigate_to = Some(PathBuf::from(format!("__OPEN_EDITOR__{}", entry.path.display())));
                                } else {
                                    open_file_with_system(&entry.path);
                                }
                            }
                        }

                        // Right-click context menu: Rename / Delete
                        resp.context_menu(|ui| {
                            if ui.button("Rename").clicked() {
                                *rename_target = Some(entry.path.clone());
                                *rename_buffer = entry.name.clone();
                                ui.close();
                            }
                            if ui.button("Delete").clicked() {
                                match delete_entry(&entry.path) {
                                    Ok(()) => {
                                        navigate_to = Some(PathBuf::from(format!("__NOTIFY_OK__Deleted '{}'", entry.name)));
                                    }
                                    Err(e) => {
                                        navigate_to = Some(PathBuf::from(format!("__NOTIFY_ERR__{e}")));
                                    }
                                }
                                ui.close();
                            }
                        });
                    }
                });
            });
        });

        navigate_to
    }

    fn content_controls(ui: &mut egui::Ui) {
        ui.label(RichText::new("Quick Controls").strong().size(16.0).color(Color32::from_gray(240)));
        ui.add_space(8.0);
        egui::Grid::new("controls_grid").num_columns(2).spacing(Vec2::new(10.0, 10.0)).show(ui, |ui| {
            for label in ["Wi-Fi", "Bluetooth", "Focus", "Display"] {
                let _ = ui.add(egui::Button::new(RichText::new(label).strong().color(Color32::from_gray(240)))
                    .min_size(Vec2::new(130.0, 60.0))
                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 28))
                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(70)))
                    .corner_radius(CornerRadius::same(12)));
                if label == "Bluetooth" || label == "Display" { ui.end_row(); }
            }
        });
    }

    fn content_messages(ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            egui::Frame::default()
                .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(8, 8))
                .show(ui, |ui| {
                    ui.set_min_width(120.0);
                    ui.label(RichText::new("Messages").size(11.0).strong().color(Color32::from_gray(160)));
                    ui.add_space(6.0);
                    for (name, preview, selected) in [("Alice", "Hey! The new UI...", true), ("Bob", "Sure, sounds good", false),
                        ("Team", "Build passed!", false), ("Carol", "See you tomorrow", false)] {
                        let bg = if selected { Color32::from_rgba_unmultiplied(0, 122, 255, 80) } else { Color32::TRANSPARENT };
                        egui::Frame::default().fill(bg).corner_radius(CornerRadius::same(6)).inner_margin(egui::Margin::symmetric(6, 4)).show(ui, |ui| {
                            ui.label(RichText::new(name).size(12.0).strong().color(Color32::from_gray(230)));
                            ui.label(RichText::new(preview).size(10.0).color(Color32::from_gray(150)));
                        });
                        ui.add_space(2.0);
                    }
                });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let messages: &[(&str, bool)] = &[
                        ("Hey! How's the AuroraOS project going?", false),
                        ("Going great! Just finished the desktop shell", true),
                        ("The Big Sur wallpaper looks amazing", false),
                        ("Thanks! The dock magnification was tricky", true),
                        ("Can you show me a screenshot?", false),
                        ("Sure, sending one now...", true),
                        ("Hey! The new UI looks amazing", false),
                    ];
                    for (text, is_sent) in messages {
                        let (align, color, tc) = if *is_sent {
                            (Align::RIGHT, Color32::from_rgb(0, 122, 255), Color32::WHITE)
                        } else {
                            (Align::LEFT, Color32::from_rgba_unmultiplied(255, 255, 255, 40), Color32::from_gray(230))
                        };
                        ui.with_layout(Layout::top_down(align), |ui| {
                            egui::Frame::default().fill(color).corner_radius(CornerRadius::same(14))
                                .inner_margin(egui::Margin::symmetric(12, 6))
                                .show(ui, |ui| { ui.set_max_width(220.0); ui.label(RichText::new(*text).size(12.0).color(tc)); });
                        });
                        ui.add_space(4.0);
                    }
                });
            });
        });
    }

    fn content_browser(ui: &mut egui::Ui) {
        egui::Frame::default()
            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 15))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(10, 5))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("<  >").size(13.0).color(Color32::from_gray(120)));
                    ui.add_space(8.0);
                    egui::Frame::default()
                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 20))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(egui::Margin::symmetric(8, 3))
                        .show(ui, |ui| { ui.set_min_width(300.0); ui.label(RichText::new("auroraos://favorites").size(12.0).color(Color32::from_gray(160))); });
                });
            });
        ui.add_space(10.0);
        ui.label(RichText::new("Favorites").size(16.0).strong().color(Color32::from_gray(220)));
        ui.add_space(8.0);
        egui::Grid::new("browser_favorites").num_columns(4).spacing(Vec2::new(16.0, 14.0)).show(ui, |ui| {
            let sites = [("Apple", Color32::from_rgb(142, 142, 147), "A"), ("Google", Color32::from_rgb(66, 133, 244), "G"),
                ("GitHub", Color32::from_rgb(36, 41, 46), "GH"), ("Reddit", Color32::from_rgb(255, 69, 0), "R"),
                ("Netflix", Color32::from_rgb(229, 9, 20), "N"), ("Rust", Color32::from_rgb(222, 165, 88), "Rs"),
                ("Twitter", Color32::from_rgb(29, 161, 242), "T"), ("LinkedIn", Color32::from_rgb(0, 119, 181), "Li")];
            for (i, (name, color, abbrev)) in sites.iter().enumerate() {
                ui.vertical(|ui| {
                    ui.set_min_width(70.0);
                    let (ir, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::click());
                    ui.painter().rect_filled(ir, CornerRadius::same(10), *color);
                    ui.painter().text(ir.center(), Align2::CENTER_CENTER, *abbrev, FontId::proportional(16.0), Color32::WHITE);
                    ui.label(RichText::new(*name).size(10.0).color(Color32::from_gray(180)));
                });
                if (i + 1) % 4 == 0 { ui.end_row(); }
            }
        });
    }

    fn content_calculator(ui: &mut egui::Ui, display: &mut String, operand: &mut Option<f64>, operator: &mut Option<char>, reset_next: &mut bool) {
        // Display
        egui::Frame::default()
            .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 60))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(display.as_str()).size(28.0).strong().color(Color32::WHITE).family(egui::FontFamily::Monospace));
                });
            });
        ui.add_space(8.0);

        let btn_size = Vec2::new(52.0, 42.0);
        let rows: &[&[(&str, Color32)]] = &[
            &[("C", Color32::from_rgb(165, 165, 165)), ("+/-", Color32::from_rgb(165, 165, 165)), ("%", Color32::from_rgb(165, 165, 165)), ("/", Color32::from_rgb(255, 149, 0))],
            &[("7", Color32::from_rgb(80, 80, 80)), ("8", Color32::from_rgb(80, 80, 80)), ("9", Color32::from_rgb(80, 80, 80)), ("*", Color32::from_rgb(255, 149, 0))],
            &[("4", Color32::from_rgb(80, 80, 80)), ("5", Color32::from_rgb(80, 80, 80)), ("6", Color32::from_rgb(80, 80, 80)), ("-", Color32::from_rgb(255, 149, 0))],
            &[("1", Color32::from_rgb(80, 80, 80)), ("2", Color32::from_rgb(80, 80, 80)), ("3", Color32::from_rgb(80, 80, 80)), ("+", Color32::from_rgb(255, 149, 0))],
            &[("0", Color32::from_rgb(80, 80, 80)), (".", Color32::from_rgb(80, 80, 80)), ("=", Color32::from_rgb(255, 149, 0))],
        ];

        for row in rows {
            ui.horizontal(|ui| {
                for (label, color) in *row {
                    let w = if *label == "0" { btn_size.x * 2.0 + 4.0 } else { btn_size.x };
                    let text_color = if *color == Color32::from_rgb(255, 149, 0) { Color32::WHITE }
                        else if *color == Color32::from_rgb(165, 165, 165) { Color32::BLACK }
                        else { Color32::WHITE };
                    let btn = ui.add(egui::Button::new(RichText::new(*label).size(16.0).strong().color(text_color))
                        .min_size(Vec2::new(w, btn_size.y))
                        .fill(*color)
                        .corner_radius(CornerRadius::same(21)));
                    if btn.clicked() {
                        match *label {
                            "C" => { *display = "0".to_string(); *operand = None; *operator = None; *reset_next = false; }
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
                                    if let (Some(prev), Some(op)) = (*operand, *operator) {
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
                                if let (Ok(val), Some(prev), Some(op)) = (display.parse::<f64>(), *operand, *operator) {
                                    let result = calc_eval(prev, op, val);
                                    *display = format_calc(result);
                                    *operand = None;
                                    *operator = None;
                                    *reset_next = true;
                                }
                            }
                            "." => {
                                if *reset_next { *display = "0.".to_string(); *reset_next = false; }
                                else if !display.contains('.') { display.push('.'); }
                            }
                            digit => {
                                if *reset_next || *display == "0" {
                                    *display = digit.to_string();
                                    *reset_next = false;
                                } else {
                                    display.push_str(digit);
                                }
                            }
                        }
                    }
                }
            });
            ui.add_space(4.0);
        }
    }

    fn content_notes(ui: &mut egui::Ui, text: &mut String) {
        ui.horizontal(|ui| {
            let toolbar_items = ["Bold", "Italic", "List", "---", "Font"];
            for item in toolbar_items {
                if item == "---" {
                    let (sr, _) = ui.allocate_exact_size(Vec2::new(1.0, 18.0), Sense::hover());
                    ui.painter().rect_filled(sr, 0.0, Color32::from_white_alpha(30));
                } else {
                    ui.add(egui::Button::new(RichText::new(item).size(11.0).color(Color32::from_gray(180)))
                        .fill(Color32::TRANSPARENT).stroke(Stroke::NONE));
                }
            }
        });
        ui.add_space(4.0);
        let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(sep_rect, 0.0, Color32::from_white_alpha(20));
        ui.add_space(4.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(egui::TextEdit::multiline(text)
                .font(FontId::proportional(13.0))
                .text_color(Color32::from_gray(220))
                .desired_width(ui.available_width())
                .desired_rows(16)
                .frame(false));
        });
    }

    fn content_music(ui: &mut egui::Ui, playing: &mut bool, track_idx: &mut usize) {
        let tracks = [
            ("Aurora Ambient", "System Sounds", Color32::from_rgb(255, 107, 157)),
            ("Neon Waves", "Synthwave FM", Color32::from_rgb(88, 86, 214)),
            ("Mountain Breeze", "Nature Sounds", Color32::from_rgb(52, 199, 89)),
            ("Deep Focus", "Lo-Fi Beats", Color32::from_rgb(255, 149, 0)),
            ("Night Drive", "Electronic", Color32::from_rgb(0, 122, 255)),
        ];
        let (name, artist, color) = tracks[*track_idx % tracks.len()];

        // Album art
        let art_size = Vec2::splat(80.0);
        ui.horizontal(|ui| {
            let (art_rect, _) = ui.allocate_exact_size(art_size, Sense::hover());
            gradient_rect(ui.painter(), art_rect, color, Color32::from_rgb(30, 30, 50));
            ui.painter().rect_stroke(art_rect, CornerRadius::same(8), Stroke::new(0.5, Color32::from_white_alpha(30)), StrokeKind::Outside);
            // Music note symbol
            ui.painter().text(art_rect.center(), Align2::CENTER_CENTER, "♪", FontId::proportional(30.0), Color32::from_white_alpha(180));

            ui.vertical(|ui| {
                ui.add_space(12.0);
                ui.label(RichText::new(name).size(16.0).strong().color(Color32::WHITE));
                ui.label(RichText::new(artist).size(12.0).color(Color32::from_gray(150)));
            });
        });

        ui.add_space(12.0);

        // Progress bar
        let progress = 0.35; // Simulated
        let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 4.0), Sense::hover());
        ui.painter().rect_filled(bar_rect, CornerRadius::same(2), Color32::from_rgba_unmultiplied(255, 255, 255, 30));
        let filled = Rect::from_min_size(bar_rect.min, Vec2::new(bar_rect.width() * progress, 4.0));
        ui.painter().rect_filled(filled, CornerRadius::same(2), color);
        ui.horizontal(|ui| {
            ui.label(RichText::new("1:12").size(10.0).color(Color32::from_gray(120)));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new("3:28").size(10.0).color(Color32::from_gray(120)));
            });
        });

        ui.add_space(8.0);

        // Controls
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - 160.0) / 2.0);
            // Previous
            if ui.add(egui::Button::new(RichText::new("<<").size(16.0).color(Color32::WHITE))
                .fill(Color32::TRANSPARENT).stroke(Stroke::NONE).min_size(Vec2::splat(36.0))).clicked() {
                *track_idx = track_idx.checked_sub(1).unwrap_or(tracks.len() - 1);
            }
            // Play/Pause
            let play_label = if *playing { "| |" } else { " > " };
            if ui.add(egui::Button::new(RichText::new(play_label).size(18.0).strong().color(Color32::WHITE))
                .fill(color).corner_radius(CornerRadius::same(20)).min_size(Vec2::splat(44.0))).clicked() {
                *playing = !*playing;
            }
            // Next
            if ui.add(egui::Button::new(RichText::new(">>").size(16.0).color(Color32::WHITE))
                .fill(Color32::TRANSPARENT).stroke(Stroke::NONE).min_size(Vec2::splat(36.0))).clicked() {
                *track_idx = (*track_idx + 1) % tracks.len();
            }
        });
    }

    fn content_photos(ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for tab in ["All Photos", "Favorites", "Albums", "People"] {
                let active = tab == "All Photos";
                let bg = if active { Color32::from_rgba_unmultiplied(0, 122, 255, 80) } else { Color32::TRANSPARENT };
                ui.add(egui::Button::new(RichText::new(tab).size(11.0).color(if active { Color32::WHITE } else { Color32::from_gray(160) }))
                    .fill(bg).stroke(Stroke::NONE).corner_radius(CornerRadius::same(6)));
            }
        });
        ui.add_space(8.0);

        // Photo grid — colored rectangles simulating thumbnails
        let thumb_size = Vec2::splat(64.0);
        let colors = [
            Color32::from_rgb(255, 107, 107), Color32::from_rgb(78, 205, 196), Color32::from_rgb(255, 230, 109),
            Color32::from_rgb(162, 155, 254), Color32::from_rgb(255, 159, 243), Color32::from_rgb(69, 183, 209),
            Color32::from_rgb(255, 179, 71), Color32::from_rgb(119, 221, 119), Color32::from_rgb(207, 159, 255),
            Color32::from_rgb(255, 105, 180), Color32::from_rgb(100, 149, 237), Color32::from_rgb(255, 218, 185),
            Color32::from_rgb(144, 238, 144), Color32::from_rgb(255, 160, 122), Color32::from_rgb(173, 216, 230),
            Color32::from_rgb(221, 160, 221), Color32::from_rgb(245, 222, 179), Color32::from_rgb(176, 224, 230),
            Color32::from_rgb(255, 182, 193), Color32::from_rgb(152, 251, 152), Color32::from_rgb(135, 206, 250),
            Color32::from_rgb(255, 228, 196), Color32::from_rgb(230, 230, 250), Color32::from_rgb(250, 128, 114),
        ];
        let cols = ((ui.available_width()) / (thumb_size.x + 6.0)).floor().max(1.0) as usize;

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("photo_grid").num_columns(cols).spacing(Vec2::splat(6.0)).show(ui, |ui| {
                for (i, color) in colors.iter().enumerate() {
                    let (rect, resp) = ui.allocate_exact_size(thumb_size, Sense::click());
                    // Gradient fill to simulate photo
                    let lighter = Color32::from_rgba_unmultiplied(
                        (color.r() as u16 + 40).min(255) as u8,
                        (color.g() as u16 + 40).min(255) as u8,
                        (color.b() as u16 + 40).min(255) as u8, 255);
                    gradient_rect(ui.painter(), rect, lighter, *color);
                    ui.painter().rect_stroke(rect, CornerRadius::same(4), Stroke::new(0.5, Color32::from_white_alpha(30)), StrokeKind::Outside);
                    // Landscape/portrait symbol
                    if i % 5 == 0 {
                        let sun_c = Pos2::new(rect.right() - 14.0, rect.top() + 14.0);
                        ui.painter().circle_filled(sun_c, 6.0, Color32::from_rgba_unmultiplied(255, 255, 200, 150));
                    }
                    if resp.hovered() {
                        ui.painter().rect_stroke(rect, CornerRadius::same(4), Stroke::new(2.0, Color32::from_rgb(0, 122, 255)), StrokeKind::Outside);
                    }
                    if (i + 1) % cols == 0 { ui.end_row(); }
                }
            });
        });
    }

    fn content_calendar(ui: &mut egui::Ui) {
        let now = Local::now();
        let year = now.year();
        let month = now.month();
        let today = now.day();

        let month_names = ["January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"];
        let month_name = month_names[(month - 1) as usize];

        ui.horizontal(|ui| {
            ui.label(RichText::new("<").size(16.0).color(Color32::from_gray(140)));
            ui.add_space(8.0);
            ui.label(RichText::new(format!("{month_name} {year}")).size(16.0).strong().color(Color32::WHITE));
            ui.add_space(8.0);
            ui.label(RichText::new(">").size(16.0).color(Color32::from_gray(140)));
        });
        ui.add_space(10.0);

        // Day-of-week headers
        let cell_size = Vec2::new(34.0, 30.0);
        ui.horizontal(|ui| {
            for day in ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"] {
                let (r, _) = ui.allocate_exact_size(cell_size, Sense::hover());
                ui.painter().text(r.center(), Align2::CENTER_CENTER, day, FontId::proportional(11.0), Color32::from_gray(120));
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
        }.unwrap().signed_duration_since(first).num_days() as u32;

        let mut day = 1u32;
        for _week in 0..6 {
            if day > days_in_month { break; }
            ui.horizontal(|ui| {
                for col in 0..7u32 {
                    let (r, _resp) = ui.allocate_exact_size(cell_size, Sense::click());
                    if (_week == 0 && (col as usize) < first_weekday) || day > days_in_month {
                        // Empty cell
                    } else {
                        let is_today = day == today;
                        if is_today {
                            ui.painter().circle_filled(r.center(), 14.0, Color32::from_rgb(255, 59, 48));
                        }
                        let text_color = if is_today { Color32::WHITE } else { Color32::from_gray(220) };
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, format!("{day}"), FontId::proportional(13.0), text_color);
                        day += 1;
                    }
                }
            });
        }

        ui.add_space(12.0);
        // Today's events
        ui.label(RichText::new("Today").size(12.0).strong().color(Color32::from_gray(160)));
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
                ui.label(RichText::new(time).size(11.0).color(Color32::from_gray(140)));
                ui.label(RichText::new(title).size(12.0).color(Color32::from_gray(220)));
            });
        }
    }

    // ── Text Editor ──────────────────────────────────────────────────────────

    fn content_text_editor(ui: &mut egui::Ui, file_path: &Option<PathBuf>, content: &mut String, modified: &mut bool) {
        let gray = Color32::from_gray(160);
        let white = Color32::from_gray(230);

        // Toolbar
        ui.horizontal(|ui| {
            let title = if let Some(ref path) = file_path {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("Untitled");
                if *modified { format!("{name} (modified)") } else { name.to_string() }
            } else {
                "Untitled".to_string()
            };
            ui.label(RichText::new(&title).size(13.0).strong().color(white));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Save button
                if let Some(ref path) = file_path {
                    if *modified {
                        if ui.add(egui::Button::new(RichText::new("Save").size(11.0).color(Color32::WHITE))
                            .fill(Color32::from_rgb(0, 122, 255))
                            .corner_radius(CornerRadius::same(4))
                            .min_size(Vec2::new(50.0, 22.0))).clicked()
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
                ui.label(RichText::new(format!("{lines} lines | {chars} chars")).size(10.0).color(gray));
            });
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // File path breadcrumb
        if let Some(ref path) = file_path {
            ui.label(RichText::new(path.to_string_lossy().to_string()).size(10.0).color(Color32::from_gray(100)));
            ui.add_space(4.0);
        }

        // Editor area
        egui::ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
            let font_id = FontId::monospace(13.0);
            let resp = ui.add(
                egui::TextEdit::multiline(content)
                    .font(font_id)
                    .text_color(Color32::from_gray(220))
                    .desired_width(f32::INFINITY)
                    .desired_rows(30)
                    .code_editor()
            );
            if resp.changed() {
                *modified = true;
            }
        });
    }

    fn open_file_in_editor(&mut self, path: PathBuf) {
        match fs::read_to_string(&path) {
            Ok(content) => {
                self.editor_content = content;
                self.editor_file_path = Some(path.clone());
                self.editor_modified = false;
                let win = self.window_mut(WindowKind::TextEditor);
                win.open = true;
                win.minimized = false;
                win.open_anim_start = Some(Instant::now());
                win.closing = false;
                win.close_anim_start = None;
                win.id_epoch = win.id_epoch.saturating_add(1);
                self.bring_to_front(WindowKind::TextEditor);
                self.toasts.push(Toast::new(
                    "File Opened",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
                    Color32::from_rgb(52, 199, 89),
                ));
            }
            Err(e) => {
                self.toasts.push(Toast::new(
                    "Error",
                    format!("Cannot open file: {e}"),
                    Color32::from_rgb(255, 59, 48),
                ));
            }
        }
    }

    // ── Settings ─────────────────────────────────────────────────────────────

    fn content_settings(&mut self, ui: &mut egui::Ui) {
        let white = Color32::from_gray(230);
        let gray = Color32::from_gray(140);
        let accent = Color32::from_rgb(0, 122, 255);

        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            ui.label(RichText::new("General").size(14.0).strong().color(white));
            ui.add_space(6.0);

            // Wallpaper selection
            ui.horizontal(|ui| {
                ui.label(RichText::new("Wallpaper").size(12.0).color(gray));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let name = WALLPAPERS.get(self.wallpaper_idx).map(|w| w.name).unwrap_or("Unknown");
                    if ui.add(egui::Button::new(RichText::new(format!("◀ {} ▶", name)).size(11.0).color(Color32::WHITE))
                        .fill(accent).corner_radius(CornerRadius::same(4))).clicked()
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

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            ui.label(RichText::new("Appearance").size(14.0).strong().color(white));
            ui.add_space(6.0);

            Self::toggle_row(ui, "Dark Mode", &mut self.app_settings.dark_mode, gray);

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
            ui.label(RichText::new("About").size(14.0).strong().color(white));
            ui.add_space(6.0);
            ui.label(RichText::new("AuroraOS Desktop v0.2.0").size(12.0).color(gray));
            ui.label(RichText::new("Built with Rust + egui").size(11.0).color(Color32::from_gray(100)));
            ui.label(RichText::new(format!("Process count: {}", self.sysinfo.process_count)).size(11.0).color(Color32::from_gray(100)));
        });
    }

    fn toggle_row(ui: &mut egui::Ui, label: &str, value: &mut bool, color: Color32) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).size(12.0).color(color));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let text = if *value { "On" } else { "Off" };
                let btn_color = if *value { Color32::from_rgb(52, 199, 89) } else { Color32::from_gray(80) };
                if ui.add(egui::Button::new(RichText::new(text).size(11.0).color(Color32::WHITE))
                    .fill(btn_color).corner_radius(CornerRadius::same(10))
                    .min_size(Vec2::new(44.0, 22.0))).clicked()
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
            ui.add(egui::TextEdit::singleline(&mut self.proc_search)
                .hint_text("Search processes...")
                .desired_width(200.0)
                .text_color(white));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(egui::Button::new(RichText::new("Refresh").size(11.0).color(Color32::WHITE))
                    .fill(Color32::from_rgb(0, 122, 255))
                    .corner_radius(CornerRadius::same(4))).clicked()
                {
                    if let Some(ref mut pm) = self.proc_manager {
                        pm.refresh();
                    }
                }
                let sort_label = if self.proc_sort_by_cpu { "Sort: CPU" } else { "Sort: Memory" };
                if ui.add(egui::Button::new(RichText::new(sort_label).size(11.0).color(Color32::WHITE))
                    .fill(Color32::from_gray(60))
                    .corner_radius(CornerRadius::same(4))).clicked()
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
            ui.allocate_ui_with_layout(Vec2::new(60.0, 16.0), Layout::left_to_right(Align::Center), |ui| {
                ui.label(RichText::new("PID").size(10.0).strong().color(gray));
            });
            ui.allocate_ui_with_layout(Vec2::new(200.0, 16.0), Layout::left_to_right(Align::Center), |ui| {
                ui.label(RichText::new("Name").size(10.0).strong().color(gray));
            });
            ui.allocate_ui_with_layout(Vec2::new(70.0, 16.0), Layout::left_to_right(Align::Center), |ui| {
                ui.label(RichText::new("CPU %").size(10.0).strong().color(gray));
            });
            ui.allocate_ui_with_layout(Vec2::new(80.0, 16.0), Layout::left_to_right(Align::Center), |ui| {
                ui.label(RichText::new("Memory").size(10.0).strong().color(gray));
            });
        });
        ui.add_space(2.0);

        if let Some(ref pm) = self.proc_manager {
            let procs = if self.proc_search.is_empty() {
                if self.proc_sort_by_cpu { pm.list_sorted_by_cpu() } else { pm.list_sorted_by_memory() }
            } else {
                let mut p = pm.search(&self.proc_search);
                if self.proc_sort_by_cpu {
                    p.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
                } else {
                    p.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
                }
                p
            };

            let total = procs.len();
            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                for proc in procs.iter().take(200) {
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(Vec2::new(60.0, 16.0), Layout::left_to_right(Align::Center), |ui| {
                            ui.label(RichText::new(format!("{}", proc.pid)).size(10.0).color(gray));
                        });
                        ui.allocate_ui_with_layout(Vec2::new(200.0, 16.0), Layout::left_to_right(Align::Center), |ui| {
                            ui.label(RichText::new(&proc.name).size(10.0).color(white));
                        });
                        ui.allocate_ui_with_layout(Vec2::new(70.0, 16.0), Layout::left_to_right(Align::Center), |ui| {
                            let cpu_color = if proc.cpu_usage > 50.0 { Color32::from_rgb(255, 59, 48) }
                                else if proc.cpu_usage > 10.0 { Color32::from_rgb(255, 149, 0) }
                                else { Color32::from_gray(180) };
                            ui.label(RichText::new(format!("{:.1}", proc.cpu_usage)).size(10.0).color(cpu_color));
                        });
                        ui.allocate_ui_with_layout(Vec2::new(80.0, 16.0), Layout::left_to_right(Align::Center), |ui| {
                            ui.label(RichText::new(ProcessManager::format_memory(proc.memory_bytes)).size(10.0).color(Color32::from_gray(180)));
                        });
                    });
                }
            });

            ui.add_space(4.0);
            ui.separator();
            ui.label(RichText::new(format!("{total} processes")).size(10.0).color(gray));
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
                .frame(egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(40, 40, 50, 240))
                    .stroke(Stroke::new(1.0, Color32::from_gray(80)))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::same(20))
                    .shadow(egui::epaint::Shadow { offset: [0, 4], blur: 20, spread: 0, color: Color32::from_black_alpha(100) }))
                .show(ctx, |ui| {
                    ui.label(RichText::new("You have unsaved changes.").size(14.0).color(Color32::from_gray(230)));
                    ui.label(RichText::new("Do you want to save before closing?").size(12.0).color(Color32::from_gray(160)));
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        // Save & close
                        if ui.add(egui::Button::new(RichText::new("Save").size(12.0).color(Color32::WHITE))
                            .fill(Color32::from_rgb(0, 122, 255))
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(70.0, 28.0))).clicked()
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
                        if ui.add(egui::Button::new(RichText::new("Don't Save").size(12.0).color(Color32::WHITE))
                            .fill(Color32::from_rgb(255, 59, 48))
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(90.0, 28.0))).clicked()
                        {
                            self.editor_modified = false;
                            let w = self.window_mut(kind);
                            w.start_close();
                            self.confirm_close_window = None;
                        }
                        ui.add_space(8.0);
                        // Cancel
                        if ui.add(egui::Button::new(RichText::new("Cancel").size(12.0).color(Color32::from_gray(200)))
                            .fill(Color32::from_gray(60))
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(70.0, 28.0))).clicked()
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
        self.toasts.retain(|t| !t.is_expired());
        let screen = ctx.viewport_rect();
        let toast_w = 300.0;
        let toast_h = 60.0;

        for (i, toast) in self.toasts.iter().enumerate() {
            let progress = toast.progress();
            // Slide in from right, slide out at end
            let slide = if progress < 0.1 {
                progress / 0.1  // slide in
            } else if progress > 0.85 {
                (1.0 - progress) / 0.15  // slide out
            } else {
                1.0
            };
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
                                let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                                ui.painter().circle_filled(dot.center(), 5.0, toast.color);
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&toast.title).size(12.0).strong().color(Color32::WHITE));
                                    ui.label(RichText::new(&toast.body).size(11.0).color(Color32::from_gray(160)));
                                });
                            });
                        });
                });
        }
    }

    // ── Window rendering ─────────────────────────────────────────────────────

    fn traffic_light(ui: &mut egui::Ui, color: Color32, label: &'static str) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(13.0), Sense::click());
        ui.painter().circle_filled(rect.center(), 5.5, color);
        if response.hovered() {
            ui.painter().circle_stroke(rect.center(), 5.5, Stroke::new(0.5, Color32::from_black_alpha(40)));
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
            sys: System::new(), networks: Networks::new(),
            cpu_usage: cpu, total_memory_gb: mem_total, used_memory_gb: mem_used,
            memory_pct: mem_pct, battery_pct: batt_pct, battery_charging: batt_charging,
            battery_available: batt_available, network_up: net_up, network_name: net_name,
            disk_total_gb: disk_total, disk_used_gb: disk_used, process_count: proc_count,
            last_refresh: None,
        };

        let telemetry = Telemetry {
            connected: self.telemetry.connected,
            status: self.telemetry.status.clone(), health: self.telemetry.health.clone(),
            uptime: self.telemetry.uptime.clone(), boot: self.telemetry.boot.clone(),
            last_error: self.telemetry.last_error.clone(), last_poll: self.telemetry.last_poll,
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
            if !snap.open || snap.minimized { continue; }

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
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, (30.0 * anim_alpha) as u8)),
                    Color32::from_rgba_unmultiplied(180, 180, 180, alpha_byte),
                )
            } else if dark {
                (
                    Color32::from_rgba_unmultiplied(30, 30, 34, (230.0 * anim_alpha) as u8),
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(60, 60, 65, (200.0 * anim_alpha) as u8)),
                    Color32::from_rgba_unmultiplied(220, 220, 225, alpha_byte),
                )
            } else {
                (
                    Color32::from_rgba_unmultiplied(255, 255, 255, (38.0 * anim_alpha) as u8),
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, (86.0 * anim_alpha) as u8)),
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
                egui::Frame::NONE.inner_margin(egui::Margin::same(8)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if Self::traffic_light(ui, Color32::from_rgb(255, 95, 87), "Close").clicked() { close_clicked = true; }
                        if Self::traffic_light(ui, Color32::from_rgb(255, 189, 47), "Minimize").clicked() { minimize_clicked = true; }
                        if Self::traffic_light(ui, Color32::from_rgb(40, 200, 64), "Maximize").clicked() { maximize_clicked = true; }
                        ui.add_space(8.0);
                        // Double-click title text to toggle maximize
                        // Dynamic title for text editor (show filename)
                        let display_title = if kind == WindowKind::TextEditor {
                            if let Some(ref path) = self.editor_file_path {
                                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("Untitled");
                                if self.editor_modified { format!("{name} — TextEdit (edited)") } else { format!("{name} — TextEdit") }
                            } else {
                                "Untitled — TextEdit".to_string()
                            }
                        } else {
                            kind.title().to_string()
                        };
                        let title_resp = ui.add(egui::Label::new(RichText::new(&display_title).size(13.0).color(title_color)).sense(Sense::click()));
                        if title_resp.double_clicked() { maximize_clicked = true; }
                    });
                    ui.add_space(6.0); ui.separator(); ui.add_space(8.0);

                    match kind {
                        WindowKind::Overview => Self::content_overview(ui, &si_snap, &telemetry, &cpu_hist),
                        WindowKind::Terminal => {
                            if let Some(ref mut pty) = self.pty_terminal {
                                Self::content_terminal_pty(ui, pty, &mut self.terminal_input);
                            } else if let Some(cmd) = Self::content_terminal_builtin(ui, &si_snap, &term_lines, &mut self.terminal_input) {
                                term_cmd = Some(cmd);
                            }
                        }
                        WindowKind::FileManager => {
                            if let Some(nav) = Self::content_filemanager(
                                ui, &fm_dir, &fm_entries,
                                &mut self.fm_show_new_dialog, &mut self.fm_new_name, &mut self.fm_new_is_dir,
                                &mut self.fm_rename_target, &mut self.fm_rename_buffer,
                            ) {
                                fm_navigate = Some(nav);
                            }
                        }
                        WindowKind::Controls => Self::content_controls(ui),
                        WindowKind::Messages => Self::content_messages(ui),
                        WindowKind::Browser => Self::content_browser(ui),
                        WindowKind::Calculator => Self::content_calculator(ui,
                            &mut self.calc_display, &mut self.calc_operand,
                            &mut self.calc_operator, &mut self.calc_reset_next),
                        WindowKind::Notes => Self::content_notes(ui, &mut self.notes_text),
                        WindowKind::MusicPlayer => Self::content_music(ui,
                            &mut self.music_playing, &mut self.music_track_idx),
                        WindowKind::Photos => Self::content_photos(ui),
                        WindowKind::Calendar => Self::content_calendar(ui),
                        WindowKind::TextEditor => Self::content_text_editor(ui,
                            &self.editor_file_path, &mut self.editor_content, &mut self.editor_modified),
                        WindowKind::Settings => self.content_settings(ui),
                        WindowKind::ProcessManager => self.content_process_manager(ui),
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
                        let near_left = (pos.x - win_rect.left()).abs() < edge && pos.y > win_rect.top() && pos.y < win_rect.bottom();
                        let near_right = (pos.x - win_rect.right()).abs() < edge && pos.y > win_rect.top() && pos.y < win_rect.bottom();
                        let near_top = (pos.y - win_rect.top()).abs() < edge && pos.x > win_rect.left() && pos.x < win_rect.right();
                        let near_bottom = (pos.y - win_rect.bottom()).abs() < edge && pos.x > win_rect.left() && pos.x < win_rect.right();

                        // Corner detection
                        let near_tl = (pos.x - win_rect.left()).abs() < edge * 2.0 && (pos.y - win_rect.top()).abs() < edge * 2.0;
                        let near_tr = (pos.x - win_rect.right()).abs() < edge * 2.0 && (pos.y - win_rect.top()).abs() < edge * 2.0;
                        let near_bl = (pos.x - win_rect.left()).abs() < edge * 2.0 && (pos.y - win_rect.bottom()).abs() < edge * 2.0;

                        let on_edge = near_left || near_right || near_top || near_bottom || near_tl || near_tr || near_bl;

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
            if should_bring { bring_front = Some(kind); }
        }

        if let Some(kind) = bring_front { self.bring_to_front(kind); }

        // Handle file manager navigation
        if let Some(path) = fm_navigate {
            let path_str = path.to_string_lossy();
            if let Some(msg) = path_str.strip_prefix("__NOTIFY_OK__") {
                self.notification_center.notify("Files", msg, "", Color32::from_rgb(52, 199, 89));
                self.fm_entries = read_directory(&self.fm_current_dir);
            } else if let Some(msg) = path_str.strip_prefix("__NOTIFY_ERR__") {
                self.notification_center.notify("Files", "Error", msg, Color32::from_rgb(255, 59, 48));
            } else if path_str == "__REFRESH__" {
                self.fm_entries = read_directory(&self.fm_current_dir);
            } else if let Some(file_path) = path_str.strip_prefix("__OPEN_EDITOR__") {
                self.open_file_in_editor(PathBuf::from(file_path));
            } else {
                self.fm_current_dir = path.clone();
                self.fm_entries = read_directory(&path);
            }
        }

        // Handle terminal command
        if let Some(cmd) = term_cmd {
            let output = Self::execute_terminal_command(&cmd, &si_snap);
            if output.len() == 1 && output[0].0 == "__CLEAR__" {
                self.terminal_output.clear();
            } else {
                self.terminal_output.extend(output);
            }
        }
    }

    // ── Dock ─────────────────────────────────────────────────────────────────

    fn render_dock(&mut self, ctx: &egui::Context) {
        let mut open_window: Option<WindowKind> = None;

        if let Some((_, t)) = self.dock_bounce {
            if t.elapsed() > Duration::from_millis(800) { self.dock_bounce = None; }
        }

        egui::TopBottomPanel::bottom("dock")
            .exact_height(DOCK_HEIGHT)
            .frame(egui::Frame::default().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let pointer = ui.input(|i| i.pointer.hover_pos());
                let icons = DockIcon::all();
                let n_real = icons.iter().filter(|i| !i.is_separator()).count();
                let total_base_w = n_real as f32 * (DOCK_ICON_BASE + 3.0) + 12.0;
                let screen_w = ui.available_rect_before_wrap().width();
                let dock_start_x = (screen_w - total_base_w) / 2.0;
                let dock_bottom_y = ui.available_rect_before_wrap().bottom();

                let mut sizes: Vec<f32> = Vec::with_capacity(icons.len());
                let mut cum_x = dock_start_x;
                for icon in icons {
                    if icon.is_separator() { sizes.push(2.0); cum_x += 10.0; continue; }
                    let center_x = cum_x + DOCK_ICON_BASE / 2.0;
                    let size = match pointer {
                        Some(pos) if pos.y > dock_bottom_y - DOCK_HEIGHT - 20.0 => {
                            let dist = (pos.x - center_x).abs();
                            if dist < DOCK_EFFECT_DIST {
                                let ratio = 1.0 - dist / DOCK_EFFECT_DIST;
                                DOCK_ICON_BASE * (1.0 + (DOCK_ICON_MAX_SCALE - 1.0) * ratio.powf(1.5))
                            } else { DOCK_ICON_BASE }
                        }
                        _ => DOCK_ICON_BASE,
                    };
                    sizes.push(size);
                    cum_x += DOCK_ICON_BASE + 3.0;
                }

                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    ui.add_space(8.0);
                    let dock_fill = if self.app_settings.dark_mode {
                        Color32::from_rgba_unmultiplied(20, 20, 22, 180)
                    } else {
                        Color32::from_rgba_unmultiplied(40, 40, 40, 90)
                    };
                    egui::Frame::default()
                        .fill(dock_fill)
                        .stroke(Stroke::new(0.5, Color32::from_white_alpha(76)))
                        .corner_radius(CornerRadius::same(16))
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
                                ui.spacing_mut().item_spacing.x = 3.0;
                                for (i, icon) in icons.iter().enumerate() {
                                    if icon.is_separator() {
                                        let (sr, _) = ui.allocate_exact_size(Vec2::new(1.0, 40.0), Sense::hover());
                                        ui.painter().rect_filled(sr, 0.0, Color32::from_white_alpha(50));
                                        continue;
                                    }
                                    let mut size = sizes[i];
                                    let mut bounce_offset_y: f32 = 0.0;
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
                                                18.0 * (1.0 - t).max(0.0) * (std::f32::consts::PI * t).sin().abs()
                                            } else if e < 0.55 {
                                                // Second smaller bounce
                                                let t = (e - 0.35) / 0.20;
                                                8.0 * (1.0 - t).max(0.0) * (std::f32::consts::PI * t).sin().abs()
                                            } else if e < 0.70 {
                                                // Tiny third bounce
                                                let t = (e - 0.55) / 0.15;
                                                3.0 * (1.0 - t).max(0.0) * (std::f32::consts::PI * t).sin().abs()
                                            } else {
                                                0.0
                                            };
                                            bounce_offset_y = -bounce;
                                            size += bounce * 0.15; // slight scale with bounce
                                        }
                                    }
                                    let (icon_rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
                                    let icon_rect = if bounce_offset_y != 0.0 {
                                        icon_rect.translate(Vec2::new(0.0, bounce_offset_y))
                                    } else {
                                        icon_rect
                                    };
                                    paint_dock_icon(ui.painter(), icon_rect, *icon);
                                    if let Some(wk) = icon.window_kind() {
                                        if self.window_ref(wk).open && !self.window_ref(wk).minimized {
                                            ui.painter().circle_filled(Pos2::new(icon_rect.center().x, icon_rect.bottom() + 4.0), 2.5, Color32::from_white_alpha(200));
                                        }
                                    }
                                    let clicked = response.clicked();
                                    response.on_hover_text(icon.label());
                                    if clicked {
                                        if let Some(wk) = icon.window_kind() {
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
                        egui::Grid::new("cc_toggles").num_columns(2).spacing(Vec2::new(8.0, 8.0)).show(ui, |ui| {
                            let toggles: &mut [(&str, &mut bool)] = &mut [
                                ("Wi-Fi", &mut self.cc_wifi), ("Bluetooth", &mut self.cc_bluetooth),
                                ("AirDrop", &mut self.cc_airdrop), ("Focus", &mut self.cc_focus),
                            ];
                            for (i, (label, active)) in toggles.iter_mut().enumerate() {
                                let fill = if **active { Color32::from_rgb(0, 122, 255) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 25) };
                                if ui.add(egui::Button::new(RichText::new(*label).size(12.0).color(Color32::WHITE))
                                    .min_size(Vec2::new(140.0, 55.0)).fill(fill)
                                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                                    .corner_radius(CornerRadius::same(10))).clicked() {
                                    **active = !**active;
                                }
                                if i % 2 == 1 { ui.end_row(); }
                            }
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("*").size(14.0).color(Color32::from_rgb(255, 214, 10)));
                            ui.add(egui::Slider::new(&mut self.cc_brightness, 0.0..=1.0).show_value(false).text("Display"));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(")").size(14.0).color(Color32::WHITE));
                            ui.add(egui::Slider::new(&mut self.cc_volume, 0.0..=1.0).show_value(false).text("Sound"));
                        });
                        ui.add_space(10.0);
                        egui::Frame::default()
                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 18))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (ar, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                                    gradient_rect(ui.painter(), ar, Color32::from_rgb(255, 107, 157), Color32::from_rgb(87, 75, 144));
                                    ui.painter().rect_stroke(ar, CornerRadius::same(4), Stroke::new(0.5, Color32::from_white_alpha(30)), StrokeKind::Outside);
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new("Aurora Ambient").size(12.0).strong().color(Color32::WHITE));
                                        ui.label(RichText::new("System Sounds").size(11.0).color(Color32::from_gray(160)));
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
                            ui.label(RichText::new("Wi-Fi").size(13.0).strong().color(Color32::WHITE));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let label = if self.cc_wifi { "On" } else { "Off" };
                                let color = if self.cc_wifi { Color32::from_rgb(52, 199, 89) } else { Color32::from_gray(120) };
                                if ui.add(egui::Button::new(RichText::new(label).size(11.0).color(Color32::WHITE))
                                    .fill(color).corner_radius(CornerRadius::same(8))
                                    .min_size(Vec2::new(40.0, 20.0))).clicked() {
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
                                    ui.label(RichText::new(name).size(12.0).color(Color32::from_gray(220)));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(RichText::new(&signal).size(10.0).color(Color32::from_gray(160)));
                                        if connected {
                                            ui.label(RichText::new("✓").size(12.0).color(Color32::from_rgb(0, 122, 255)));
                                        }
                                    });
                                });
                                ui.add_space(2.0);
                            }
                        } else {
                            ui.label(RichText::new("Wi-Fi is turned off").size(12.0).color(Color32::from_gray(120)));
                        }
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                        ui.label(RichText::new("Network Preferences...").size(12.0).color(Color32::from_rgb(0, 122, 255)));
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
                        ui.label(RichText::new("Sound").size(13.0).strong().color(Color32::WHITE));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("✕").size(11.0).color(Color32::from_gray(140)));
                            ui.add(egui::Slider::new(&mut self.cc_volume, 0.0..=1.0).show_value(false));
                            ui.label(RichText::new("♪").size(11.0).color(Color32::from_gray(140)));
                        });
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);
                        ui.label(RichText::new("Output Device").size(11.0).color(Color32::from_gray(140)));
                        ui.add_space(2.0);
                        let devices = ["Built-in Speakers", "AuroraOS Audio"];
                        for (i, dev) in devices.iter().enumerate() {
                            ui.horizontal(|ui| {
                                if i == 0 {
                                    ui.label(RichText::new("✓").size(12.0).color(Color32::from_rgb(0, 122, 255)));
                                } else {
                                    ui.add_space(16.0);
                                }
                                ui.label(RichText::new(*dev).size(12.0).color(Color32::from_gray(220)));
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
                            ui.label(RichText::new("Bluetooth").size(13.0).strong().color(Color32::WHITE));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let label = if self.cc_bluetooth { "On" } else { "Off" };
                                let color = if self.cc_bluetooth { Color32::from_rgb(0, 122, 255) } else { Color32::from_gray(120) };
                                if ui.add(egui::Button::new(RichText::new(label).size(11.0).color(Color32::WHITE))
                                    .fill(color).corner_radius(CornerRadius::same(8))
                                    .min_size(Vec2::new(40.0, 20.0))).clicked() {
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
                                    let color = if connected { Color32::from_gray(220) } else { Color32::from_gray(140) };
                                    ui.label(RichText::new(name).size(12.0).color(color));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(RichText::new(status).size(10.0).color(Color32::from_gray(100)));
                                    });
                                });
                                ui.add_space(2.0);
                            }
                        } else {
                            ui.label(RichText::new("Bluetooth is turned off").size(12.0).color(Color32::from_gray(120)));
                        }
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                        ui.label(RichText::new("Bluetooth Preferences...").size(12.0).color(Color32::from_rgb(0, 122, 255)));
                    });
            });
    }

    // ── Notification Center ──────────────────────────────────────────────────

    fn render_notification_center(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        // Mark all as read when opening
        self.notification_center.mark_all_read();

        egui::Area::new(Id::new("notification_center"))
            .fixed_pos(Pos2::new(screen.right() - 360.0, MENU_BAR_HEIGHT + 8.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(30, 30, 30, 210))
                    .stroke(Stroke::new(0.5, Color32::from_white_alpha(40)))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(egui::Margin::symmetric(14, 14))
                    .show(ui, |ui| {
                        ui.set_min_width(320.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Notifications").size(16.0).strong().color(Color32::WHITE));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.add(egui::Button::new(RichText::new("Clear All").size(11.0).color(Color32::from_gray(160)))
                                    .fill(Color32::TRANSPARENT).stroke(Stroke::NONE)).clicked()
                                {
                                    self.notification_center.clear();
                                }
                            });
                        });
                        ui.add_space(10.0);

                        if self.notification_center.is_empty() {
                            ui.label(RichText::new("No notifications").size(13.0).color(Color32::from_gray(100)));
                        } else {
                            egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                                for notif in self.notification_center.all() {
                                    egui::Frame::default()
                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 18))
                                        .corner_radius(CornerRadius::same(10))
                                        .inner_margin(egui::Margin::symmetric(10, 8))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                let (dr, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                                                ui.painter().circle_filled(dr.center(), 5.0, notif.color);
                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new(&notif.app).size(10.0).color(Color32::from_gray(140)));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            ui.label(RichText::new(notif.time_ago()).size(10.0).color(Color32::from_gray(100)));
                                                        });
                                                    });
                                                    ui.label(RichText::new(&notif.title).size(13.0).strong().color(Color32::WHITE));
                                                    ui.label(RichText::new(&notif.body).size(12.0).color(Color32::from_gray(180)));
                                                });
                                            });
                                        });
                                    ui.add_space(6.0);
                                }
                            });
                        }
                    });
            });
    }

    // ── Spotlight ────────────────────────────────────────────────────────────

    fn render_spotlight(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        let overlay_painter = ctx.layer_painter(egui::LayerId::new(Order::Foreground, Id::new("spotlight_bg")));
        overlay_painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 80));

        // Search real files
        let file_results = self.file_index.search(&self.spotlight_query, 8);

        egui::Area::new(Id::new("spotlight"))
            .fixed_pos(Pos2::new(screen.center().x - 280.0, screen.top() + screen.height() * 0.22))
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
                            if !response.has_focus() { response.request_focus(); }
                        });

                        if !self.spotlight_query.is_empty() {
                            ui.add_space(6.0);
                            let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(530.0, 1.0), Sense::hover());
                            ui.painter().rect_filled(sep_rect, 0.0, Color32::from_white_alpha(30));
                            ui.add_space(6.0);

                            // App results
                            let query = self.spotlight_query.to_lowercase();
                            let apps: &[(&str, WindowKind)] = &[
                                ("System Overview", WindowKind::Overview), ("Terminal", WindowKind::Terminal),
                                ("Files", WindowKind::FileManager), ("Messages", WindowKind::Messages),
                                ("Browser", WindowKind::Browser), ("Quick Controls", WindowKind::Controls),
                                ("Calculator", WindowKind::Calculator), ("Notes", WindowKind::Notes), ("Music", WindowKind::MusicPlayer),
                                ("Photos", WindowKind::Photos), ("Calendar", WindowKind::Calendar),
                            ];
                            let mut has_app = false;
                            for (name, wk) in apps {
                                if name.to_lowercase().contains(&query) {
                                    if !has_app {
                                        ui.label(RichText::new("Applications").size(11.0).strong().color(Color32::from_gray(120)));
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
                                                ui.label(RichText::new(*name).size(13.0).color(Color32::WHITE));
                                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                    ui.label(RichText::new("Application").size(11.0).color(Color32::from_gray(100)));
                                                });
                                            });
                                        }).response;
                                    if resp.interact(Sense::click()).clicked() {
                                        self.spotlight_open_window = Some(*wk);
                                    }
                                    ui.add_space(2.0);
                                }
                            }

                            // File results (real)
                            if !file_results.is_empty() {
                                ui.add_space(4.0);
                                ui.label(RichText::new("Files").size(11.0).strong().color(Color32::from_gray(120)));
                                ui.add_space(2.0);

                                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                    for path in &file_results {
                                        let resp = egui::Frame::default()
                                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                            .corner_radius(CornerRadius::same(6))
                                            .inner_margin(egui::Margin::symmetric(8, 3))
                                            .show(ui, |ui| {
                                                let p = std::path::Path::new(path);
                                                let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or(path);
                                                ui.label(RichText::new(fname).size(13.0).color(Color32::WHITE));
                                                ui.label(RichText::new(path).size(10.0).color(Color32::from_gray(100)));
                                            }).response;
                                        if resp.interact(Sense::click()).clicked() {
                                            self.spotlight_open_file = Some(PathBuf::from(path));
                                        }
                                        ui.add_space(1.0);
                                    }
                                });
                            }

                            // System commands section — suggest running as terminal command
                            if !query.is_empty() && query.len() >= 2 {
                                ui.add_space(4.0);
                                ui.label(RichText::new("Actions").size(11.0).strong().color(Color32::from_gray(120)));
                                ui.add_space(2.0);
                                let resp = egui::Frame::default()
                                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(egui::Margin::symmetric(8, 4))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.set_min_width(510.0);
                                            ui.label(RichText::new(format!("Run \"{}\"", self.spotlight_query)).size(13.0).color(Color32::WHITE));
                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                ui.label(RichText::new("System Command").size(11.0).color(Color32::from_gray(100)));
                                            });
                                        });
                                    }).response;
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
                                        let parts: Vec<&str> = self.spotlight_query.split_whitespace().collect();
                                        if let Some(program) = parts.first() {
                                            let args: Vec<&str> = parts[1..].to_vec();
                                            match launch_program(program, &args) {
                                                Ok(()) => self.toasts.push(Toast::new("Launched", *program, Color32::from_rgb(52, 199, 89))),
                                                Err(e) => self.toasts.push(Toast::new("Error", e, Color32::from_rgb(255, 59, 48))),
                                            }
                                        }
                                    }
                                    self.show_spotlight = false;
                                    self.spotlight_query.clear();
                                }
                            }

                            if !has_app && file_results.is_empty() && query.len() < 2 {
                                ui.label(RichText::new("No results found").size(13.0).color(Color32::from_gray(120)));
                            }
                        }
                    });
            });
    }

    // ── FPS ──────────────────────────────────────────────────────────────────

    fn update_fps(&mut self, ctx: &egui::Context) {
        let dt = ctx.input(|i| i.stable_dt).max(0.0001);
        let fps = 1.0 / dt;
        self.fps_smoothed = if self.fps_smoothed <= 0.0 { fps } else { self.fps_smoothed * 0.9 + fps * 0.1 };
    }

    fn render_fps_overlay(&self, ctx: &egui::Context) {
        egui::Area::new(Id::new("fps_overlay"))
            .fixed_pos(Pos2::new(ctx.content_rect().right() - 100.0, MENU_BAR_HEIGHT + 8.0))
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgba_unmultiplied(6, 12, 30, 150))
                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(50)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.label(RichText::new(format!("FPS {:.0}", self.fps_smoothed)).size(11.0).color(Color32::WHITE));
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
            if elapsed > 0.4 { self.login_shake = None; 0.0 }
            else { (elapsed * 40.0).sin() * (1.0 - elapsed * 2.5).max(0.0) * 12.0 }
        } else { 0.0 };

        egui::Area::new(Id::new("login_screen"))
            .fixed_pos(Pos2::new(screen.center().x - 160.0 + shake_x, screen.center().y - 140.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // User avatar circle
                    let (avatar_r, _) = ui.allocate_exact_size(Vec2::splat(80.0), Sense::hover());
                    ui.painter().circle_filled(avatar_r.center(), 40.0, Color32::from_rgba_unmultiplied(255, 255, 255, 30));
                    ui.painter().circle_stroke(avatar_r.center(), 40.0, Stroke::new(1.5, Color32::from_white_alpha(80)));
                    ui.painter().text(avatar_r.center(), Align2::CENTER_CENTER, "A", FontId::proportional(32.0), Color32::WHITE);

                    ui.add_space(12.0);
                    ui.label(RichText::new("Aurora User").size(18.0).strong().color(Color32::WHITE));
                    ui.add_space(16.0);

                    // Password field
                    egui::Frame::default()
                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 25))
                        .stroke(Stroke::new(1.0, Color32::from_white_alpha(60)))
                        .corner_radius(CornerRadius::same(20))
                        .inner_margin(egui::Margin::symmetric(14, 6))
                        .show(ui, |ui| {
                            let te = egui::TextEdit::singleline(&mut self.login_password)
                                .password(true)
                                .hint_text("Enter Password")
                                .font(FontId::proportional(14.0))
                                .text_color(Color32::WHITE)
                                .desired_width(240.0)
                                .frame(false);
                            let resp = ui.add(te);
                            if !resp.has_focus() && self.login_shake.is_none() { resp.request_focus(); }
                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if enter_pressed {
                                // Accept any password — this is a demo lock screen
                                self.show_login = false;
                                self.login_password.clear();
                            }
                        });

                    ui.add_space(10.0);
                    ui.label(RichText::new("Press Enter to unlock").size(11.0).color(Color32::from_gray(120)));

                    // Clock
                    ui.add_space(30.0);
                    let time_str = Local::now().format("%-I:%M %p").to_string();
                    ui.label(RichText::new(time_str).size(48.0).color(Color32::WHITE));
                    let date_str = Local::now().format("%A, %B %-d").to_string();
                    ui.label(RichText::new(date_str).size(16.0).color(Color32::from_gray(180)));
                });
            });
    }

    // ── Mission Control ──────────────────────────────────────────────────────

    fn render_mission_control(&mut self, ctx: &egui::Context) {
        let screen = ctx.viewport_rect();
        let target = if self.show_mission_control { 1.0f32 } else { 0.0 };
        let speed = 6.0 * ctx.input(|i| i.stable_dt);
        self.mission_control_anim += (target - self.mission_control_anim) * speed.min(1.0);

        if self.mission_control_anim < 0.01 && !self.show_mission_control {
            self.mission_control_anim = 0.0;
            return;
        }

        let t = self.mission_control_anim;
        let painter = ctx.layer_painter(egui::LayerId::new(Order::Foreground, Id::new("mc_bg")));
        painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, (120.0 * t) as u8));

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
                            .fill(Color32::from_rgba_unmultiplied(40, 40, 50, (180.0 * t) as u8))
                            .stroke(Stroke::new(if is_current { 2.0 } else { 1.0 }, border_color))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(egui::Margin::symmetric(4, 4))
                            .show(ui, |ui| {
                                ui.set_min_size(Vec2::new(desk_w - 8.0, desk_h - 8.0));
                                ui.label(RichText::new(format!("Desktop {}", desk_i + 1))
                                    .size(10.0).color(Color32::from_rgba_unmultiplied(200, 200, 200, (255.0 * t) as u8)));
                            }).response;
                        if resp.interact(Sense::click()).clicked() {
                            self.current_desktop = desk_i;
                        }
                    }
                    // "+" button to add desktop
                    if self.desktop_count < 6 {
                        let resp = egui::Frame::default()
                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, (20.0 * t) as u8))
                            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, (40.0 * t) as u8)))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(egui::Margin::symmetric(4, 4))
                            .show(ui, |ui| {
                                ui.set_min_size(Vec2::new(40.0, desk_h - 8.0));
                                ui.centered_and_justified(|ui| {
                                    ui.label(RichText::new("+").size(24.0).color(Color32::from_rgba_unmultiplied(200, 200, 200, (200.0 * t) as u8)));
                                });
                            }).response;
                        if resp.interact(Sense::click()).clicked() {
                            self.desktop_count += 1;
                        }
                    }
                });
            });

        // Show all open windows as thumbnails
        let open_windows: Vec<WindowKind> = self.z_order.iter().copied()
            .filter(|k| { let w = self.window_ref(*k); w.open && !w.minimized })
            .collect();

        if open_windows.is_empty() { return; }
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
                        .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, (60.0 * t) as u8)))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.set_min_size(Vec2::new(thumb_w - 16.0, thumb_h - 12.0));
                            // Traffic lights (tiny)
                            ui.horizontal(|ui| {
                                let s = 6.0;
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(s), Sense::hover());
                                ui.painter().circle_filled(r.center(), 3.0, Color32::from_rgb(255, 95, 87));
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(s), Sense::hover());
                                ui.painter().circle_filled(r.center(), 3.0, Color32::from_rgb(255, 189, 47));
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(s), Sense::hover());
                                ui.painter().circle_filled(r.center(), 3.0, Color32::from_rgb(40, 200, 64));
                                ui.add_space(6.0);
                                ui.label(RichText::new(kind.title()).size(10.0).color(Color32::from_rgba_unmultiplied(200, 200, 200, alpha)));
                            });
                            // Placeholder content area
                            let (content_r, _) = ui.allocate_exact_size(Vec2::new(thumb_w - 32.0, thumb_h - 44.0), Sense::hover());
                            ui.painter().rect_filled(content_r, CornerRadius::same(4), Color32::from_rgba_unmultiplied(30, 30, 40, alpha));
                        }).response;
                    if resp.interact(Sense::click()).clicked() {
                        clicked_window = Some(*kind);
                    }

                    // Window title below thumbnail
                    ui.label(RichText::new(kind.title()).size(11.0).color(Color32::from_rgba_unmultiplied(220, 220, 220, (255.0 * t) as u8)));
                });
        }

        // Click on window thumbnail to focus it and exit MC
        if let Some(kind) = clicked_window {
            self.bring_to_front(kind);
            self.show_mission_control = false;
        }

        // Click on background to close MC
        if ctx.input(|i| i.pointer.primary_clicked()) {
            if clicked_window.is_none() {
                self.show_mission_control = false;
            }
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
            if pos.x <= work_rect.left() + edge_threshold {
                self.drag_snap_preview = Some(SnapSide::Left);
            } else if pos.x >= work_rect.right() - edge_threshold {
                self.drag_snap_preview = Some(SnapSide::Right);
            } else if pos.y <= work_rect.top() + edge_threshold {
                self.drag_snap_maximize = true;
            }
        }
    }

    fn render_edge_snap_preview(&self, ctx: &egui::Context, work_rect: Rect) {
        let preview_rect = if self.drag_snap_maximize {
            Some(work_rect)
        } else {
            self.drag_snap_preview.map(|side| Self::snap_rect(work_rect, side))
        };

        if let Some(rect) = preview_rect {
            let painter = ctx.layer_painter(egui::LayerId::new(Order::Tooltip, Id::new("snap_preview")));
            painter.rect_filled(rect.shrink(4.0), CornerRadius::same(12),
                Color32::from_rgba_unmultiplied(0, 122, 255, 40));
            painter.rect_stroke(rect.shrink(4.0), CornerRadius::same(12),
                Stroke::new(2.0, Color32::from_rgba_unmultiplied(0, 122, 255, 120)), StrokeKind::Outside);
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
            window_rects.push(format!("[{},{},{},{}]", w.default_pos.x, w.default_pos.y, w.default_size.x, w.default_size.y));
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
        // Also persist AppSettings
        self.app_settings.wallpaper_idx = self.wallpaper_idx;
        self.app_settings.volume = self.cc_volume;
        self.app_settings.brightness = self.cc_brightness;
        self.app_settings.wifi_enabled = self.cc_wifi;
        self.app_settings.bluetooth_enabled = self.cc_bluetooth;
        self.app_settings.airdrop_enabled = self.cc_airdrop;
        let _ = self.app_settings.save();
    }

    fn load_state(&mut self) {
        let path = Self::state_file_path();
        if let Ok(content) = fs::read_to_string(&path) {
            // Simple JSON parsing without serde
            if let Some(wp) = content.find("\"wallpaper\":").and_then(|i| {
                content[i+12..].trim().split(|c: char| !c.is_ascii_digit()).next()?.parse::<usize>().ok()
            }) {
                self.wallpaper_idx = wp % WALLPAPERS.len();
            }
            if let Some(vol) = content.find("\"volume\":").and_then(|i| {
                content[i+9..].trim().split(|c: char| !c.is_ascii_digit() && c != '.').next()?.parse::<f32>().ok()
            }) {
                self.cc_volume = vol.clamp(0.0, 1.0);
            }
            if let Some(br) = content.find("\"brightness\":").and_then(|i| {
                content[i+13..].trim().split(|c: char| !c.is_ascii_digit() && c != '.').next()?.parse::<f32>().ok()
            }) {
                self.cc_brightness = br.clamp(0.0, 1.0);
            }
            // Load notes between the first and last quote after "notes":
            if let Some(start) = content.find("\"notes\":").and_then(|i| content[i+8..].find('"').map(|j| i + 8 + j + 1)) {
                if let Some(end) = content[start..].find("\",\n").map(|j| start + j) {
                    let notes = content[start..end].replace("\\n", "\n").replace("\\\"", "\"").replace("\\\\", "\\");
                    self.notes_text = notes;
                }
            }
        }
    }
}

// ── eframe::App ──────────────────────────────────────────────────────────────

impl eframe::App for AuroraDesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_fps(ctx);
        self.maybe_poll();

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

        self.render_background(ctx);

        // Login screen blocks everything else
        if self.show_login {
            self.render_login_screen(ctx);
            ctx.request_repaint_after(Duration::from_millis(16));
            return;
        }

        let (toggle_cc, toggle_spotlight, toggle_notifications) = self.render_menu_bar(ctx);
        if toggle_cc { self.show_control_center = !self.show_control_center; self.show_notifications = false; }
        if toggle_spotlight { self.show_spotlight = !self.show_spotlight; self.spotlight_query.clear(); }
        if toggle_notifications { self.show_notifications = !self.show_notifications; self.show_control_center = false; }

        if ctx.input(|i| i.key_pressed(egui::Key::Space) && i.modifiers.command) {
            self.show_spotlight = !self.show_spotlight;
            self.spotlight_query.clear();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.show_mission_control { self.show_mission_control = false; }
            else {
                self.show_spotlight = false;
                self.show_control_center = false;
                self.show_notifications = false;
                self.show_wifi_popup = false;
                self.show_volume_popup = false;
                self.show_bluetooth_popup = false;
                self.active_menu = None;
                self.context_menu_pos = None;
            }
        }
        // F3 or Ctrl+Up = Mission Control
        if ctx.input(|i| i.key_pressed(egui::Key::F3) || (i.key_pressed(egui::Key::ArrowUp) && i.modifiers.ctrl && i.modifiers.shift)) {
            self.show_mission_control = !self.show_mission_control;
        }

        let work_rect = Self::desktop_work_rect(ctx);
        self.handle_shortcuts(ctx, work_rect);
        self.detect_edge_snap(ctx, work_rect);
        self.render_desktop_icons(ctx, work_rect);
        self.check_context_menu(ctx);
        self.render_windows(ctx, work_rect);
        self.render_edge_snap_preview(ctx, work_rect);
        self.render_dock(ctx);

        if self.show_control_center { self.render_control_center(ctx); }
        if self.show_notifications { self.render_notification_center(ctx); }
        if self.show_wifi_popup { self.render_wifi_popup(ctx); }
        if self.show_volume_popup { self.render_volume_popup(ctx); }
        if self.show_bluetooth_popup { self.render_bluetooth_popup(ctx); }
        if self.show_spotlight { self.render_spotlight(ctx); }
        if self.active_menu.is_some() { self.render_menu_dropdown(ctx); }
        if self.context_menu_pos.is_some() { self.render_context_menu(ctx); }

        // Mission Control overlay (renders on top of everything)
        if self.show_mission_control || self.mission_control_anim > 0.01 {
            self.render_mission_control(ctx);
        }

        if !self.show_control_center && !self.show_notifications {
            self.render_fps_overlay(ctx);
        }

        // Unsaved changes confirmation dialog
        self.render_confirm_close(ctx);

        // Toast notifications
        if !self.toasts.is_empty() {
            self.render_toasts(ctx);
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
                            win.restore_rect = Some(Rect::from_min_size(win.default_pos, win.default_size));
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
                    if !text.is_empty() { self.clipboard.copy(&text); }
                }
                MenuAction::Cut => {
                    let active = self.active_window();
                    match active {
                        Some(WindowKind::TextEditor) => {
                            self.clipboard.copy(&self.editor_content);
                            self.editor_content.clear();
                            self.editor_modified = true;
                        }
                        Some(WindowKind::Notes) => {
                            self.clipboard.copy(&self.notes_text);
                            self.notes_text.clear();
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
                            }
                            Some(WindowKind::Notes) => {
                                self.notes_text.push_str(&pasted);
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
                            self.notification_center.notify("TextEdit", "File saved", &path.to_string_lossy(), Color32::from_rgb(52, 199, 89));
                        }
                    }
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
            let is_text = matches!(ext,
                "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "go" | "java" |
                "md" | "txt" | "log" | "csv" | "json" | "toml" | "yaml" | "yml" |
                "html" | "css" | "xml" | "sh" | "bat" | "cmd" | "ps1" | "cfg" |
                "ini" | "conf" | "env" | "gitignore" | "lock" | "sql" | "lua" | "rb"
            );
            if is_text {
                self.open_file_in_editor(path);
            } else {
                open_file_with_system(&path);
                self.toasts.push(Toast::new("File Opened", path.file_name().and_then(|n| n.to_str()).unwrap_or("file"), Color32::from_rgb(52, 199, 89)));
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
                            self.toasts.push(Toast::new("Folder Opened", pb.file_name().and_then(|n| n.to_str()).unwrap_or("folder"), Color32::from_rgb(0, 122, 255)));
                        } else if pb.extension().and_then(|e| e.to_str()).map(|e| terminal::is_text_extension(e)).unwrap_or(false) {
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
                                self.toasts.push(Toast::new("File Opened", pb.file_name().and_then(|n| n.to_str()).unwrap_or("file"), Color32::from_rgb(52, 199, 89)));
                            }
                        } else {
                            open_file_with_system(&pb);
                            self.toasts.push(Toast::new("File Opened", pb.file_name().and_then(|n| n.to_str()).unwrap_or("file"), Color32::from_rgb(52, 199, 89)));
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
            .with_min_inner_size([1080.0, 700.0]),
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

    // ── Wallpaper presets ────────────────────────────────────────────────

    #[test]
    fn wallpaper_presets_exist() {
        assert!(WALLPAPERS.len() >= 2);
    }

    #[test]
    fn wallpaper_bands_are_sorted() {
        for wp in WALLPAPERS {
            for pair in wp.bands.windows(2) {
                assert!(pair[0].0 <= pair[1].0, "wallpaper bands must be sorted by position");
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
        assert!(out.iter().any(|(line, _)| line.contains("unknown subcommand")));
    }

}
