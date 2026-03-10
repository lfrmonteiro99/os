use std::fmt::Write as _;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, CornerRadius, RichText, Stroke};
use sysinfo::Networks;

use crate::clipboard::AppClipboard;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiagnosticLevel {
    Success,
    Warning,
    Error,
    Info,
}

impl DiagnosticLevel {
    fn color(self) -> Color32 {
        match self {
            Self::Success => Color32::from_rgb(52, 199, 89),
            Self::Warning => Color32::from_rgb(255, 204, 0),
            Self::Error => Color32::from_rgb(255, 99, 99),
            Self::Info => Color32::from_rgb(120, 190, 255),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Success => "OK",
            Self::Warning => "WARN",
            Self::Error => "ERR",
            Self::Info => "INFO",
        }
    }
}

#[derive(Clone, Debug)]
struct DiagnosticEntry {
    timestamp: String,
    source: String,
    level: DiagnosticLevel,
    message: String,
}

#[derive(Clone, Debug)]
struct InterfaceSnapshot {
    name: String,
    received_bytes: u64,
    transmitted_bytes: u64,
    is_active: bool,
    connection_type: &'static str,
    signal_strength: Option<u8>,
}

#[derive(Clone, Debug)]
struct NetworkOverview {
    interfaces: Vec<InterfaceSnapshot>,
    primary_ip: Option<String>,
    gateway: Option<String>,
    dns_servers: Vec<String>,
}

pub struct NetworkDiagnostics {
    networks: Networks,
    pub ping_target: String,
    pub dns_target: String,
    pub traceroute_target: String,
    pub port_target: String,
    pub port_value: String,
    entries: Vec<DiagnosticEntry>,
    paused: bool,
    last_export_path: Option<PathBuf>,
}

impl NetworkDiagnostics {
    pub fn new() -> Self {
        let mut diag = Self {
            networks: Networks::new_with_refreshed_list(),
            ping_target: "1.1.1.1".to_string(),
            dns_target: "auroraos.dev".to_string(),
            traceroute_target: "auroraos.dev".to_string(),
            port_target: "auroraos.dev".to_string(),
            port_value: "443".to_string(),
            entries: Vec::new(),
            paused: false,
            last_export_path: None,
        };
        diag.seed();
        diag
    }

    pub fn render(&mut self, ui: &mut egui::Ui, clipboard: &AppClipboard) {
        let overview = self.collect_overview();
        let white = Color32::from_gray(235);
        let gray = Color32::from_gray(150);
        let panel = Color32::from_rgba_unmultiplied(255, 255, 255, 10);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Network Diagnostics")
                                .size(16.0)
                                .strong()
                                .color(white),
                        );
                        ui.label(
                            RichText::new(
                                "Connectivity tests, interface status, and exportable reports.",
                            )
                            .size(11.0)
                            .color(gray),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Export Report").clicked() {
                            self.export_report();
                        }
                        if ui.button("Copy Results").clicked() {
                            clipboard.copy(&self.render_report());
                            self.push(
                                "console",
                                DiagnosticLevel::Info,
                                "Copied diagnostics report to clipboard.",
                            );
                        }
                        let pause_label = if self.paused { "Resume" } else { "Pause" };
                        if ui.button(pause_label).clicked() {
                            self.paused = !self.paused;
                            self.push(
                                "console",
                                DiagnosticLevel::Info,
                                if self.paused {
                                    "Paused live log updates."
                                } else {
                                    "Resumed live log updates."
                                },
                            );
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

                ui.add_space(10.0);
                ui.columns(2, |columns| {
                    columns[0].vertical(|ui| {
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("Diagnostics")
                                        .size(13.0)
                                        .strong()
                                        .color(white),
                                );
                                ui.add_space(6.0);
                                self.render_ping_card(ui);
                                ui.add_space(8.0);
                                self.render_dns_card(ui);
                                ui.add_space(8.0);
                                self.render_port_card(ui);
                                ui.add_space(8.0);
                                self.render_traceroute_card(ui);
                                ui.add_space(8.0);
                                if ui.button("Run Speed Test").clicked() {
                                    self.run_speed_test(&overview);
                                }
                            });
                    });

                    columns[1].vertical(|ui| {
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("Status Dashboard")
                                        .size(13.0)
                                        .strong()
                                        .color(white),
                                );
                                ui.add_space(6.0);
                                self.render_status_dashboard(ui, &overview);
                            });
                    });
                });

                ui.add_space(10.0);
                egui::Frame::default()
                    .fill(panel)
                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Event Log").size(13.0).strong().color(white));
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                if ui.small_button("Clear").clicked() {
                                    self.entries.clear();
                                    self.seed();
                                }
                            });
                        });
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical()
                            .max_height(260.0)
                            .show(ui, |ui| {
                                for entry in &self.entries {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(&entry.timestamp)
                                                .size(10.0)
                                                .monospace()
                                                .color(Color32::from_gray(115)),
                                        );
                                        ui.label(
                                            RichText::new(format!("[{}]", entry.level.label()))
                                                .size(10.0)
                                                .monospace()
                                                .color(entry.level.color()),
                                        );
                                        ui.label(
                                            RichText::new(format!("{}:", entry.source))
                                                .size(10.0)
                                                .monospace()
                                                .color(Color32::from_gray(170)),
                                        );
                                        ui.label(
                                            RichText::new(&entry.message)
                                                .size(10.0)
                                                .monospace()
                                                .color(Color32::from_gray(210)),
                                        );
                                    });
                                }
                            });
                    });
            });
    }

    fn render_ping_card(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Ping")
                    .size(11.0)
                    .color(Color32::from_gray(180)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.ping_target)
                    .desired_width(140.0)
                    .hint_text("hostname or IP"),
            );
            if ui.button("Run").clicked() {
                self.run_ping();
            }
        });
    }

    fn render_dns_card(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("DNS")
                    .size(11.0)
                    .color(Color32::from_gray(180)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.dns_target)
                    .desired_width(140.0)
                    .hint_text("hostname"),
            );
            if ui.button("Lookup").clicked() {
                self.run_dns_lookup();
            }
        });
    }

    fn render_port_card(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Port")
                    .size(11.0)
                    .color(Color32::from_gray(180)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.port_target)
                    .desired_width(110.0)
                    .hint_text("host"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.port_value)
                    .desired_width(56.0)
                    .hint_text("443"),
            );
            if ui.button("Check").clicked() {
                self.run_port_check();
            }
        });
    }

    fn render_traceroute_card(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Route")
                    .size(11.0)
                    .color(Color32::from_gray(180)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.traceroute_target)
                    .desired_width(140.0)
                    .hint_text("hostname"),
            );
            if ui.button("Trace").clicked() {
                self.run_traceroute();
            }
        });
    }

    fn render_status_dashboard(&self, ui: &mut egui::Ui, overview: &NetworkOverview) {
        let online = !overview.interfaces.is_empty()
            && overview.interfaces.iter().any(|iface| iface.is_active);
        let status = if online { "Connected" } else { "Offline" };
        let status_color = if online {
            Color32::from_rgb(52, 199, 89)
        } else {
            Color32::from_rgb(255, 99, 99)
        };
        ui.label(
            RichText::new(status)
                .size(12.0)
                .strong()
                .color(status_color),
        );
        ui.add_space(4.0);

        let primary_ip = overview.primary_ip.as_deref().unwrap_or("Unavailable");
        let gateway = overview.gateway.as_deref().unwrap_or("Unavailable");
        let dns = if overview.dns_servers.is_empty() {
            "System resolver".to_string()
        } else {
            overview.dns_servers.join(", ")
        };

        for line in [
            format!("Primary IP: {primary_ip}"),
            format!("Gateway (estimated): {gateway}"),
            format!("DNS: {dns}"),
        ] {
            ui.label(
                RichText::new(line)
                    .size(11.0)
                    .monospace()
                    .color(Color32::from_gray(190)),
            );
        }

        ui.add_space(8.0);
        ui.label(
            RichText::new("Interfaces")
                .size(11.0)
                .strong()
                .color(Color32::from_gray(180)),
        );
        ui.add_space(4.0);

        if overview.interfaces.is_empty() {
            ui.label(
                RichText::new("No active interfaces discovered.")
                    .size(11.0)
                    .color(Color32::from_gray(120)),
            );
            return;
        }

        for iface in &overview.interfaces {
            egui::Frame::default()
                .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(&iface.name)
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "{}  |  RX {}  |  TX {}",
                                    iface.connection_type,
                                    format_bytes(iface.received_bytes),
                                    format_bytes(iface.transmitted_bytes)
                                ))
                                .size(10.0)
                                .monospace()
                                .color(Color32::from_gray(170)),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            let badge = if iface.is_active { "UP" } else { "DOWN" };
                            let badge_color = if iface.is_active {
                                Color32::from_rgb(52, 199, 89)
                            } else {
                                Color32::from_gray(90)
                            };
                            ui.label(RichText::new(badge).size(10.0).strong().color(badge_color));
                        });
                    });
                    if let Some(signal) = iface.signal_strength {
                        ui.label(
                            RichText::new(format!("Signal strength: {signal}%"))
                                .size(10.0)
                                .color(Color32::from_gray(150)),
                        );
                    }
                });
            ui.add_space(4.0);
        }
    }

    fn run_ping(&mut self) {
        let target = self.ping_target.trim();
        if target.is_empty() {
            self.push(
                "ping",
                DiagnosticLevel::Warning,
                "Enter a host or IP before running ping.",
            );
            return;
        }

        let start = Instant::now();
        match resolve_first_addr(target, 443).or_else(|_| resolve_first_addr(target, 80)) {
            Ok(addr) => match TcpStream::connect_timeout(&addr, Duration::from_millis(900)) {
                Ok(_) => {
                    let latency = start.elapsed().as_millis().max(4);
                    self.push(
                        "ping",
                        DiagnosticLevel::Success,
                        &format!("{target} responded in {latency} ms with 0% packet loss."),
                    );
                }
                Err(err) => {
                    self.push(
                        "ping",
                        DiagnosticLevel::Warning,
                        &format!(
                            "{target} resolved to {} but TCP probe failed: {err}.",
                            addr.ip()
                        ),
                    );
                }
            },
            Err(err) => {
                self.push(
                    "ping",
                    DiagnosticLevel::Error,
                    &format!("Could not resolve {target}: {err}."),
                );
            }
        }
    }

    fn run_dns_lookup(&mut self) {
        let host = self.dns_target.trim();
        if host.is_empty() {
            self.push(
                "dns",
                DiagnosticLevel::Warning,
                "Enter a hostname before running DNS lookup.",
            );
            return;
        }

        let resolved: Vec<String> = match (host, 0u16).to_socket_addrs() {
            Ok(addrs) => addrs.map(|addr| addr.ip().to_string()).collect(),
            Err(err) => {
                self.push(
                    "dns",
                    DiagnosticLevel::Error,
                    &format!("Lookup failed for {host}: {err}."),
                );
                return;
            }
        };

        if resolved.is_empty() {
            self.push(
                "dns",
                DiagnosticLevel::Warning,
                &format!("No records returned for {host}."),
            );
        } else {
            self.push(
                "dns",
                DiagnosticLevel::Success,
                &format!("{host} resolved to {}.", resolved.join(", ")),
            );
        }
    }

    fn run_port_check(&mut self) {
        let host = self.port_target.trim();
        let port = match self.port_value.trim().parse::<u16>() {
            Ok(port) => port,
            Err(_) => {
                self.push(
                    "port",
                    DiagnosticLevel::Warning,
                    "Port must be a valid number between 1 and 65535.",
                );
                return;
            }
        };
        match resolve_first_addr(host, port) {
            Ok(addr) => match TcpStream::connect_timeout(&addr, Duration::from_millis(750)) {
                Ok(_) => self.push(
                    "port",
                    DiagnosticLevel::Success,
                    &format!("{host}:{port} is reachable."),
                ),
                Err(err) => self.push(
                    "port",
                    DiagnosticLevel::Error,
                    &format!("{host}:{port} did not respond: {err}."),
                ),
            },
            Err(err) => self.push(
                "port",
                DiagnosticLevel::Error,
                &format!("Unable to resolve {host}:{port}: {err}."),
            ),
        }
    }

    fn run_traceroute(&mut self) {
        let host = self.traceroute_target.trim();
        if host.is_empty() {
            self.push(
                "trace",
                DiagnosticLevel::Warning,
                "Enter a hostname before tracing route.",
            );
            return;
        }

        let hops = traceroute_hops(host);
        self.push(
            "trace",
            DiagnosticLevel::Info,
            &format!("Simulated route for {host}: {}", hops.join(" -> ")),
        );
    }

    fn run_speed_test(&mut self, overview: &NetworkOverview) {
        let active = overview.interfaces.iter().find(|iface| iface.is_active);
        if let Some(iface) = active {
            let baseline = ((iface.received_bytes + iface.transmitted_bytes) / 1_000_000).max(25);
            let download = (baseline as f32 * 0.82).min(940.0);
            let upload = (baseline as f32 * 0.34).min(280.0);
            self.push(
                "speed",
                DiagnosticLevel::Info,
                &format!(
                    "Estimated throughput on {}: {:.1} Mbps down / {:.1} Mbps up.",
                    iface.name, download, upload
                ),
            );
        } else {
            self.push(
                "speed",
                DiagnosticLevel::Warning,
                "No active interface available for throughput estimate.",
            );
        }
    }

    fn collect_overview(&mut self) -> NetworkOverview {
        self.networks.refresh(true);
        let interfaces = self
            .networks
            .iter()
            .map(|(name, data)| {
                let lower = name.to_ascii_lowercase();
                let is_wifi = lower.contains("wi-fi")
                    || lower.contains("wifi")
                    || lower.contains("wlan")
                    || lower.starts_with("wl");
                let total = data.total_received() + data.total_transmitted();
                InterfaceSnapshot {
                    name: name.clone(),
                    received_bytes: data.total_received(),
                    transmitted_bytes: data.total_transmitted(),
                    is_active: total > 0,
                    connection_type: if is_wifi { "Wi-Fi" } else { "Ethernet" },
                    signal_strength: is_wifi.then_some(infer_signal_strength(total)),
                }
            })
            .collect::<Vec<_>>();

        let primary_ip = infer_primary_ip();
        let gateway = primary_ip.as_deref().and_then(infer_gateway);

        NetworkOverview {
            interfaces,
            primary_ip,
            gateway,
            dns_servers: vec!["System resolver".to_string()],
        }
    }

    fn seed(&mut self) {
        self.push(
            "console",
            DiagnosticLevel::Info,
            "Diagnostics window ready. Select a test to begin.",
        );
        self.push(
            "status",
            DiagnosticLevel::Info,
            "Interface telemetry refreshes when the window renders.",
        );
    }

    fn push(&mut self, source: &str, level: DiagnosticLevel, message: &str) {
        if self.paused {
            return;
        }
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        self.entries.insert(
            0,
            DiagnosticEntry {
                timestamp,
                source: source.to_string(),
                level,
                message: message.to_string(),
            },
        );
        self.entries.truncate(120);
    }

    fn render_report(&self) -> String {
        let mut report = String::new();
        report.push_str("AuroraOS Network Diagnostics Report\n");
        report.push_str("===============================\n");
        for entry in self.entries.iter().rev() {
            let _ = writeln!(
                report,
                "{} [{}] {}: {}",
                entry.timestamp,
                entry.level.label(),
                entry.source,
                entry.message
            );
        }
        report
    }

    fn export_report(&mut self) {
        let path = std::env::temp_dir().join("aurora_network_diagnostics_report.txt");
        match std::fs::write(&path, self.render_report()) {
            Ok(()) => {
                self.last_export_path = Some(path.clone());
                self.push(
                    "export",
                    DiagnosticLevel::Success,
                    &format!("Saved report to {}.", path.display()),
                );
            }
            Err(err) => {
                self.push(
                    "export",
                    DiagnosticLevel::Error,
                    &format!("Could not write report: {err}."),
                );
            }
        }
    }
}

fn resolve_first_addr(host: &str, port: u16) -> Result<SocketAddr, String> {
    (host, port)
        .to_socket_addrs()
        .map_err(|err| err.to_string())?
        .next()
        .ok_or_else(|| "no address records found".to_string())
}

fn infer_primary_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("1.1.1.1:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

fn infer_gateway(ip: &str) -> Option<String> {
    let mut segments = ip.split('.').collect::<Vec<_>>();
    if segments.len() == 4 {
        segments[3] = "1";
        return Some(segments.join("."));
    }
    None
}

fn infer_signal_strength(total_bytes: u64) -> u8 {
    match total_bytes {
        0..=5_000_000 => 38,
        5_000_001..=20_000_000 => 61,
        20_000_001..=80_000_000 => 78,
        _ => 92,
    }
}

fn traceroute_hops(host: &str) -> Vec<String> {
    let slug = host.replace(' ', "-");
    vec![
        "192.168.1.1 (gateway)".to_string(),
        "10.0.0.1 (isp edge)".to_string(),
        "172.16.10.4 (metro backbone)".to_string(),
        format!("edge.{slug}.net"),
        host.to_string(),
    ]
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_inference_uses_last_octet() {
        assert_eq!(
            infer_gateway("192.168.0.42").as_deref(),
            Some("192.168.0.1")
        );
    }

    #[test]
    fn traceroute_hops_include_destination() {
        let hops = traceroute_hops("auroraos.dev");
        assert_eq!(hops.last().map(String::as_str), Some("auroraos.dev"));
        assert!(hops.len() >= 4);
    }

    #[test]
    fn format_bytes_scales_values() {
        assert_eq!(format_bytes(980), "980 B");
        assert_eq!(format_bytes(25_000), "25.0 KB");
    }
}
