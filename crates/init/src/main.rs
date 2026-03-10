use std::collections::{HashMap, VecDeque};
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

use control_plane::ControlPlane;
use ipc::{
    decode_command, decode_response, encode_command, encode_response, CommandFrame, ResponseFrame,
};
use shell::run_shell_with_auth;
use svc_manager::ServiceManager;

#[derive(Debug, Default)]
struct AppConfig {
    profile: Option<String>,
    autosave: Option<String>,
    script: Option<String>,
    auth_token: Option<String>,
    audit_log: Option<String>,
    audit_format: Option<String>,
    audit_max_bytes: Option<u64>,
    max_commands: Option<usize>,
    ip_rate_limit: Option<usize>,
    ip_rate_window_sec: Option<u64>,
    idle_timeout_sec: Option<u64>,
    no_interactive: bool,
    daemon: bool,
    listen: Option<String>,
    connect: Option<String>,
    show_help: bool,
}

fn parse_args_from(args: impl Iterator<Item = String>) -> AppConfig {
    let mut config = AppConfig::default();
    let mut args = args;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--profile" => {
                if let Some(path) = args.next() {
                    config.profile = Some(path);
                }
            }
            "--autosave" => {
                if let Some(path) = args.next() {
                    config.autosave = Some(path);
                }
            }
            "--script" => {
                if let Some(path) = args.next() {
                    config.script = Some(path);
                }
            }
            "--auth-token" => {
                if let Some(token) = args.next() {
                    config.auth_token = Some(token);
                }
            }
            "--audit-log" => {
                if let Some(path) = args.next() {
                    config.audit_log = Some(path);
                }
            }
            "--audit-format" => {
                if let Some(value) = args.next() {
                    config.audit_format = Some(value);
                }
            }
            "--audit-max-bytes" => {
                if let Some(value) = args.next() {
                    config.audit_max_bytes = value.parse::<u64>().ok();
                }
            }
            "--max-commands" => {
                if let Some(value) = args.next() {
                    config.max_commands = value.parse::<usize>().ok();
                }
            }
            "--ip-rate-limit" => {
                if let Some(value) = args.next() {
                    config.ip_rate_limit = value.parse::<usize>().ok();
                }
            }
            "--ip-rate-window-sec" => {
                if let Some(value) = args.next() {
                    config.ip_rate_window_sec = value.parse::<u64>().ok();
                }
            }
            "--idle-timeout-sec" => {
                if let Some(value) = args.next() {
                    config.idle_timeout_sec = value.parse::<u64>().ok();
                }
            }
            "--no-interactive" => {
                config.no_interactive = true;
            }
            "--daemon" => {
                config.daemon = true;
            }
            "--listen" => {
                if let Some(addr) = args.next() {
                    config.listen = Some(addr);
                }
            }
            "--connect" => {
                if let Some(addr) = args.next() {
                    config.connect = Some(addr);
                }
            }
            "--help" | "-h" => {
                config.show_help = true;
            }
            _ => {}
        }
    }

    config
}

fn parse_args() -> AppConfig {
    parse_args_from(env::args().skip(1))
}

fn print_help() {
    println!("AuroraOS prototype options:");
    println!("  --profile <path>   Load services snapshot at startup");
    println!("  --autosave <path>  Save services snapshot on shell exit");
    println!("  --script <path>    Run shell commands from file before interactive mode");
    println!("  --auth-token <t>   Attach/require token for command authorization");
    println!("  --audit-log <path> Append daemon command audit lines to file");
    println!("  --audit-format <f> Audit format: text (default) or json");
    println!("  --audit-max-bytes <n> Rotate audit log when size exceeds n bytes");
    println!("  --max-commands <n> Per-connection command cap in daemon mode");
    println!("  --ip-rate-limit <n> Max commands per IP in the window");
    println!("  --ip-rate-window-sec <s> Window size for IP rate limiting");
    println!("  --idle-timeout-sec <s> Close idle client connections after timeout");
    println!("  --no-interactive   Skip interactive shell after script execution");
    println!("  --daemon           Run as control-plane TCP daemon");
    println!("  --listen <addr>    TCP bind address for daemon mode (default 127.0.0.1:7878)");
    println!("  --connect <addr>   Connect shell to remote daemon");
}

fn load_service_manager(profile: Option<&String>) -> ServiceManager {
    if let Some(profile_path) = profile {
        match fs::read_to_string(profile_path) {
            Ok(snapshot) => {
                let mut manager = ServiceManager::default();
                match manager.import_snapshot(&snapshot) {
                    Ok(loaded) => {
                        println!("loaded {loaded} services from profile '{profile_path}'");
                        return manager;
                    }
                    Err(error) => {
                        eprintln!("profile load failed: {error}");
                    }
                }
            }
            Err(error) => {
                eprintln!("failed to read profile '{profile_path}': {error}");
            }
        }
        eprintln!("falling back to seeded services");
    }

    let mut manager = ServiceManager::with_seeded_services();
    manager.start_all();
    manager
}

fn run_script_with_transport(
    script_path: &str,
    auth_token: Option<&String>,
    transport: &mut impl FnMut(CommandFrame) -> ResponseFrame,
) -> bool {
    match fs::read_to_string(script_path) {
        Ok(content) => {
            let mut frame_id = 1u64;
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let response = transport(CommandFrame::with_auth(
                    frame_id,
                    auth_token.cloned(),
                    line.to_string(),
                ));
                frame_id = frame_id.saturating_add(1);
                if !response.payload.is_empty() {
                    println!("{}", response.payload);
                }
                if response.exit {
                    return false;
                }
            }
        }
        Err(error) => {
            eprintln!("failed to read script '{script_path}': {error}");
        }
    }
    true
}

fn run_remote_shell(
    addr: &str,
    script: Option<&String>,
    auth_token: Option<&String>,
    no_interactive: bool,
) -> Result<(), String> {
    let stream = TcpStream::connect(addr).map_err(|error| format!("connect failed: {error}"))?;
    let reader_stream = stream
        .try_clone()
        .map_err(|error| format!("stream clone failed: {error}"))?;
    let mut reader = BufReader::new(reader_stream);
    let mut writer = stream;

    let mut transport = |frame: CommandFrame| -> ResponseFrame {
        let mut encoded = encode_command(&frame);
        encoded.push('\n');

        if let Err(error) = writer.write_all(encoded.as_bytes()) {
            return ResponseFrame::new(frame.id, true, format!("transport write error: {error}"));
        }
        if let Err(error) = writer.flush() {
            return ResponseFrame::new(frame.id, true, format!("transport flush error: {error}"));
        }

        let mut line = String::new();
        if let Err(error) = reader.read_line(&mut line) {
            return ResponseFrame::new(frame.id, true, format!("transport read error: {error}"));
        }
        if line.trim().is_empty() {
            return ResponseFrame::new(frame.id, true, "transport read error: empty response");
        }

        match decode_response(line.trim()) {
            Ok(response) => response,
            Err(error) => ResponseFrame::new(frame.id, true, format!("decode error: {error}")),
        }
    };

    let mut should_enter_shell = true;
    if let Some(script_path) = script {
        should_enter_shell = run_script_with_transport(script_path, auth_token, &mut transport);
    }

    if should_enter_shell && !no_interactive {
        run_shell_with_auth(auth_token.cloned(), transport)
            .map_err(|error| format!("shell error: {error}"))?;
    }
    Ok(())
}

struct IpRateLimiter {
    max_requests: usize,
    window: Duration,
    history: HashMap<String, VecDeque<Instant>>,
}

impl IpRateLimiter {
    fn new(max_requests: usize, window_sec: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(window_sec.max(1)),
            history: HashMap::new(),
        }
    }

    fn allow(&mut self, ip: &str, now: Instant) -> bool {
        let entry = self.history.entry(ip.to_string()).or_default();
        while let Some(first) = entry.front() {
            if now.duration_since(*first) > self.window {
                let _ = entry.pop_front();
            } else {
                break;
            }
        }

        if entry.len() >= self.max_requests {
            return false;
        }

        entry.push_back(now);
        true
    }
}

#[derive(Clone, Copy)]
enum AuditFormat {
    Text,
    Json,
}

struct AuditSink {
    path: String,
    format: AuditFormat,
    max_bytes: Option<u64>,
}

fn json_escape(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

impl AuditSink {
    fn from_config(path: &str, format: Option<&str>, max_bytes: Option<u64>) -> Self {
        let format = match format {
            Some("json") => AuditFormat::Json,
            _ => AuditFormat::Text,
        };
        Self {
            path: path.to_string(),
            format,
            max_bytes,
        }
    }

    fn append(&mut self, peer: &str, status: &str, message: &str) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let single_line_message = message.replace('\n', " | ");
        let line = match self.format {
            AuditFormat::Text => {
                format!("{timestamp} peer={peer} status={status} msg={single_line_message}\n")
            }
            AuditFormat::Json => format!(
                "{{\"ts\":{timestamp},\"peer\":\"{}\",\"status\":\"{}\",\"msg\":\"{}\"}}\n",
                json_escape(peer),
                json_escape(status),
                json_escape(&single_line_message)
            ),
        };

        let path = Path::new(&self.path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fs::create_dir_all(parent);
            }
        }

        if let Some(max_bytes) = self.max_bytes {
            if let Ok(metadata) = fs::metadata(path) {
                let next_size = metadata.len().saturating_add(line.len() as u64);
                if next_size > max_bytes {
                    let rotated = format!("{}.1", self.path);
                    let rotated_path = Path::new(&rotated);
                    if rotated_path.exists() {
                        let _ = fs::remove_file(rotated_path);
                    }
                    let _ = fs::rename(path, rotated_path);
                }
            }
        }

        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(line.as_bytes());
        }
    }
}

fn handle_connection(
    stream: TcpStream,
    control_plane: Arc<Mutex<ControlPlane>>,
    audit_sink: Option<Arc<Mutex<AuditSink>>>,
    max_commands: Option<usize>,
    ip_limiter: Option<Arc<Mutex<IpRateLimiter>>>,
    idle_timeout_sec: Option<u64>,
    shutdown_flag: Arc<AtomicBool>,
) -> Result<(), String> {
    let peer_addr = stream
        .peer_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let peer_ip = stream
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let reader_stream = stream
        .try_clone()
        .map_err(|error| format!("stream clone failed: {error}"))?;
    if let Some(timeout_sec) = idle_timeout_sec {
        let _ = reader_stream.set_read_timeout(Some(Duration::from_secs(timeout_sec.max(1))));
    }
    let mut reader = BufReader::new(reader_stream);
    let mut writer = stream;
    let mut command_count: usize = 0;

    loop {
        let mut line = String::new();
        let bytes = match reader.read_line(&mut line) {
            Ok(bytes) => bytes,
            Err(error) => {
                if error.kind() == ErrorKind::TimedOut || error.kind() == ErrorKind::WouldBlock {
                    let response = ResponseFrame::new(0, true, "connection closed: idle timeout");
                    let mut encoded = encode_response(&response);
                    encoded.push('\n');
                    let _ = writer.write_all(encoded.as_bytes());
                    let _ = writer.flush();
                    append_audit(
                        audit_sink.as_ref(),
                        &peer_addr,
                        "idle_timeout",
                        "connection closed after idle timeout",
                    );
                    return Ok(());
                }
                return Err(format!("read failed: {error}"));
            }
        };

        if bytes == 0 {
            return Ok(());
        }

        if let Some(limit) = max_commands {
            if command_count >= limit {
                let response = ResponseFrame::new(0, true, "rate limit: max commands reached");
                let mut encoded = encode_response(&response);
                encoded.push('\n');
                writer
                    .write_all(encoded.as_bytes())
                    .map_err(|error| format!("write failed: {error}"))?;
                writer
                    .flush()
                    .map_err(|error| format!("flush failed: {error}"))?;
                append_audit(
                    audit_sink.as_ref(),
                    &peer_addr,
                    "rate_limit",
                    "connection closed after max commands",
                );
                return Ok(());
            }
        }

        if let Some(limiter) = &ip_limiter {
            let allowed = limiter
                .lock()
                .map_err(|_| "ip limiter lock poisoned".to_string())?
                .allow(&peer_ip, Instant::now());
            if !allowed {
                let response = ResponseFrame::new(0, true, "rate limit: ip window exceeded");
                let mut encoded = encode_response(&response);
                encoded.push('\n');
                writer
                    .write_all(encoded.as_bytes())
                    .map_err(|error| format!("write failed: {error}"))?;
                writer
                    .flush()
                    .map_err(|error| format!("flush failed: {error}"))?;
                append_audit(
                    audit_sink.as_ref(),
                    &peer_addr,
                    "rate_limit_ip",
                    "connection closed after ip window exceeded",
                );
                return Ok(());
            }
        }

        let frame = decode_command(line.trim())
            .map_err(|error| format!("decode request failed: {error}"))?;
        command_count = command_count.saturating_add(1);
        let response = control_plane
            .lock()
            .map_err(|_| "control plane lock poisoned".to_string())?
            .handle_frame(frame);
        let audit_status = if response.payload.starts_with("unauthorized:") {
            "unauthorized"
        } else {
            "ok"
        };
        append_audit(
            audit_sink.as_ref(),
            &peer_addr,
            audit_status,
            &response.payload,
        );
        let mut encoded = encode_response(&response);
        encoded.push('\n');
        writer
            .write_all(encoded.as_bytes())
            .map_err(|error| format!("write failed: {error}"))?;
        writer
            .flush()
            .map_err(|error| format!("flush failed: {error}"))?;

        if response.exit {
            if response.shutdown {
                shutdown_flag.store(true, Ordering::SeqCst);
            }
            return Ok(());
        }
    }
}

fn append_audit(sink: Option<&Arc<Mutex<AuditSink>>>, peer: &str, status: &str, message: &str) {
    let Some(sink) = sink else {
        return;
    };
    if let Ok(mut sink) = sink.lock() {
        sink.append(peer, status, message);
    }
}

fn run_daemon(
    addr: &str,
    control_plane: ControlPlane,
    script: Option<&String>,
    auth_token: Option<&String>,
    audit_log: Option<&String>,
    audit_format: Option<&String>,
    audit_max_bytes: Option<u64>,
    max_commands: Option<usize>,
    ip_rate_limit: Option<usize>,
    ip_rate_window_sec: Option<u64>,
    idle_timeout_sec: Option<u64>,
) -> Result<(), String> {
    let control_plane = Arc::new(Mutex::new(control_plane));
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let audit_sink = audit_log.map(|path| {
        Arc::new(Mutex::new(AuditSink::from_config(
            path,
            audit_format.map(|s| s.as_str()),
            audit_max_bytes,
        )))
    });
    let ip_limiter = match (ip_rate_limit, ip_rate_window_sec) {
        (Some(limit), Some(window_sec)) => {
            Some(Arc::new(Mutex::new(IpRateLimiter::new(limit, window_sec))))
        }
        _ => None,
    };

    if let Some(script_path) = script {
        let mut local_transport = |frame: CommandFrame| {
            control_plane
                .lock()
                .expect("control plane lock for script")
                .handle_frame(frame)
        };
        let _ = run_script_with_transport(script_path, auth_token, &mut local_transport);
    }

    let listener = TcpListener::bind(addr).map_err(|error| format!("bind failed: {error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("nonblocking failed: {error}"))?;
    println!("daemon listening on {addr}");
    let mut workers = Vec::new();

    while !shutdown_flag.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let control_plane = Arc::clone(&control_plane);
                let shutdown_flag = Arc::clone(&shutdown_flag);
                let audit_sink = audit_sink.as_ref().map(Arc::clone);
                let ip_limiter = ip_limiter.as_ref().map(Arc::clone);
                let handle = thread::spawn(move || {
                    if let Err(error) = handle_connection(
                        stream,
                        control_plane,
                        audit_sink,
                        max_commands,
                        ip_limiter,
                        idle_timeout_sec,
                        shutdown_flag,
                    ) {
                        eprintln!("connection error: {error}");
                    }
                });
                workers.push(handle);
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => {
                eprintln!("accept error: {error}");
            }
        }
    }

    for worker in workers {
        let _ = worker.join();
    }
    println!("daemon shutdown complete");

    Ok(())
}

fn main() {
    let config = parse_args();
    if config.show_help {
        print_help();
        return;
    }

    if let Some(connect_addr) = &config.connect {
        if config.profile.is_some()
            || config.autosave.is_some()
            || config.daemon
            || config.listen.is_some()
            || config.audit_log.is_some()
            || config.audit_format.is_some()
            || config.audit_max_bytes.is_some()
            || config.max_commands.is_some()
            || config.ip_rate_limit.is_some()
            || config.ip_rate_window_sec.is_some()
            || config.idle_timeout_sec.is_some()
        {
            eprintln!(
                "warning: --connect ignores local boot/profile/autosave/daemon/listen/audit/rate-limit options"
            );
        }
        if let Err(error) = run_remote_shell(
            connect_addr,
            config.script.as_ref(),
            config.auth_token.as_ref(),
            config.no_interactive,
        ) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    let boot_start = Instant::now();
    println!("booting AuroraOS prototype...");

    let service_manager = load_service_manager(config.profile.as_ref());
    let boot_duration = boot_start.elapsed();
    println!("core services started in {}ms", boot_duration.as_millis());

    let control_plane =
        ControlPlane::new(service_manager, boot_duration, config.auth_token.clone());
    if config.daemon {
        let listen_addr = config.listen.as_deref().unwrap_or("127.0.0.1:7878");
        if let Err(error) = run_daemon(
            listen_addr,
            control_plane,
            config.script.as_ref(),
            config.auth_token.as_ref(),
            config.audit_log.as_ref(),
            config.audit_format.as_ref(),
            config.audit_max_bytes,
            config.max_commands,
            config.ip_rate_limit,
            config.ip_rate_window_sec,
            config.idle_timeout_sec,
        ) {
            eprintln!("daemon error: {error}");
            std::process::exit(1);
        }
        return;
    }

    let mut control_plane = control_plane;
    let mut local_transport = |frame: CommandFrame| control_plane.handle_frame(frame);
    let mut should_enter_shell = true;
    if let Some(script_path) = &config.script {
        should_enter_shell = run_script_with_transport(
            script_path,
            config.auth_token.as_ref(),
            &mut local_transport,
        );
    }

    if should_enter_shell && !config.no_interactive {
        if let Err(err) = run_shell_with_auth(config.auth_token.clone(), local_transport) {
            eprintln!("shell error: {err}");
            std::process::exit(1);
        }
    }

    if let Some(path) = &config.autosave {
        if let Err(error) = fs::write(path, control_plane.service_manager().export_snapshot()) {
            eprintln!("autosave failed for '{path}': {error}");
        } else {
            println!("autosaved service snapshot to '{path}'");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{parse_args_from, AuditSink};

    #[test]
    fn parses_profile_and_autosave() {
        let config = parse_args_from(
            vec![
                "--profile".to_string(),
                "profiles/default.services".to_string(),
                "--autosave".to_string(),
                "profiles/last.services".to_string(),
                "--script".to_string(),
                "profiles/bootstrap.commands".to_string(),
                "--auth-token".to_string(),
                "topsecret".to_string(),
                "--daemon".to_string(),
                "--listen".to_string(),
                "127.0.0.1:9000".to_string(),
                "--audit-log".to_string(),
                "logs/audit.log".to_string(),
                "--audit-format".to_string(),
                "json".to_string(),
                "--audit-max-bytes".to_string(),
                "4096".to_string(),
                "--max-commands".to_string(),
                "15".to_string(),
                "--ip-rate-limit".to_string(),
                "100".to_string(),
                "--ip-rate-window-sec".to_string(),
                "60".to_string(),
                "--idle-timeout-sec".to_string(),
                "30".to_string(),
                "--connect".to_string(),
                "127.0.0.1:9001".to_string(),
                "--no-interactive".to_string(),
            ]
            .into_iter(),
        );

        assert_eq!(config.profile.as_deref(), Some("profiles/default.services"));
        assert_eq!(config.autosave.as_deref(), Some("profiles/last.services"));
        assert_eq!(
            config.script.as_deref(),
            Some("profiles/bootstrap.commands")
        );
        assert_eq!(config.auth_token.as_deref(), Some("topsecret"));
        assert_eq!(config.audit_log.as_deref(), Some("logs/audit.log"));
        assert_eq!(config.audit_format.as_deref(), Some("json"));
        assert_eq!(config.audit_max_bytes, Some(4096));
        assert_eq!(config.max_commands, Some(15));
        assert_eq!(config.ip_rate_limit, Some(100));
        assert_eq!(config.ip_rate_window_sec, Some(60));
        assert_eq!(config.idle_timeout_sec, Some(30));
        assert!(config.daemon);
        assert_eq!(config.listen.as_deref(), Some("127.0.0.1:9000"));
        assert_eq!(config.connect.as_deref(), Some("127.0.0.1:9001"));
        assert!(config.no_interactive);
        assert!(!config.show_help);
    }

    #[test]
    fn parses_help_flag() {
        let config = parse_args_from(vec!["--help".to_string()].into_iter());
        assert!(config.show_help);
    }

    #[test]
    fn writes_json_audit_lines() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("audit-json-{unique}.log"));
        let path_str = path.to_string_lossy().to_string();

        let mut sink = AuditSink::from_config(&path_str, Some("json"), None);
        sink.append("127.0.0.1:1234", "ok", "hello");

        let content = fs::read_to_string(&path).expect("read audit file");
        assert!(content.contains("\"status\":\"ok\""));
        assert!(content.contains("\"peer\":\"127.0.0.1:1234\""));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rotates_audit_file_at_limit() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("audit-rotate-{unique}.log"));
        let rotated = std::env::temp_dir().join(format!("audit-rotate-{unique}.log.1"));
        let path_str = path.to_string_lossy().to_string();

        let mut sink = AuditSink::from_config(&path_str, Some("text"), Some(60));
        sink.append("p", "ok", "first line");
        sink.append("p", "ok", "second line should rotate");

        assert!(rotated.exists());
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(rotated);
    }
}
