use std::fs;
use std::time::{Duration, Instant};

use ipc::{CommandFrame, ResponseFrame};
use svc_manager::{ServiceError, ServiceManager};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Help,
    Status,
    List,
    Register(String),
    Start(String),
    Stop(String),
    Boot,
    Uptime,
    Health,
    Events(Option<usize>),
    History(Option<usize>),
    ClearEvents,
    Shutdown,
    Save(String),
    Load(String),
    Exit,
    Unknown(String),
}

fn parse_command(raw: &str) -> Command {
    let trimmed = raw.trim();
    let mut parts = trimmed.split_whitespace();
    let Some(name) = parts.next() else {
        return Command::Unknown(String::new());
    };

    match name {
        "help" => Command::Help,
        "status" => Command::Status,
        "list" => Command::List,
        "boot" => Command::Boot,
        "uptime" => Command::Uptime,
        "health" => Command::Health,
        "register" => parts
            .next()
            .map(|value| Command::Register(value.to_string()))
            .unwrap_or_else(|| Command::Unknown(trimmed.to_string())),
        "start" => parts
            .next()
            .map(|value| Command::Start(value.to_string()))
            .unwrap_or_else(|| Command::Unknown(trimmed.to_string())),
        "stop" => parts
            .next()
            .map(|value| Command::Stop(value.to_string()))
            .unwrap_or_else(|| Command::Unknown(trimmed.to_string())),
        "events" => {
            let parsed_limit = parts.next().and_then(|value| value.parse::<usize>().ok());
            if parts.next().is_some() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Events(parsed_limit)
            }
        }
        "history" => {
            let parsed_limit = parts.next().and_then(|value| value.parse::<usize>().ok());
            if parts.next().is_some() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::History(parsed_limit)
            }
        }
        "clear-events" => Command::ClearEvents,
        "shutdown" => Command::Shutdown,
        "save" => parts
            .next()
            .map(|value| Command::Save(value.to_string()))
            .unwrap_or_else(|| Command::Unknown(trimmed.to_string())),
        "load" => parts
            .next()
            .map(|value| Command::Load(value.to_string()))
            .unwrap_or_else(|| Command::Unknown(trimmed.to_string())),
        "exit" | "quit" => Command::Exit,
        _ => Command::Unknown(trimmed.to_string()),
    }
}

fn map_service_error(error: ServiceError) -> String {
    match error {
        ServiceError::AlreadyExists(name) => format!("service '{name}' already exists"),
        ServiceError::NotFound(name) => format!("service '{name}' not found"),
        ServiceError::AlreadyRunning(name) => format!("service '{name}' is already running"),
        ServiceError::AlreadyStopped(name) => format!("service '{name}' is already stopped"),
        ServiceError::InvalidSnapshot(reason) => format!("invalid snapshot: {reason}"),
    }
}

fn format_boot_report(boot_duration: Duration, uptime: Duration) -> String {
    format!(
        "boot_duration_ms: {}\nuptime_ms: {}",
        boot_duration.as_millis(),
        uptime.as_millis()
    )
}

pub struct ControlPlane {
    service_manager: ServiceManager,
    boot_duration: Duration,
    shell_start: Instant,
    command_history: Vec<String>,
    required_auth_token: Option<String>,
}

impl ControlPlane {
    pub fn new(
        service_manager: ServiceManager,
        boot_duration: Duration,
        required_auth_token: Option<String>,
    ) -> Self {
        Self {
            service_manager,
            boot_duration,
            shell_start: Instant::now(),
            command_history: Vec::new(),
            required_auth_token,
        }
    }

    pub fn service_manager(&self) -> &ServiceManager {
        &self.service_manager
    }

    pub fn handle_frame(&mut self, frame: CommandFrame) -> ResponseFrame {
        if let Some(required) = &self.required_auth_token {
            if frame.auth_token.as_deref() != Some(required.as_str()) {
                return ResponseFrame::new(frame.id, false, "unauthorized: invalid auth token");
            }
        }

        let trimmed = frame.payload.trim();
        if !trimmed.is_empty() {
            self.command_history.push(trimmed.to_string());
        }
        let command = parse_command(trimmed);
        let response = match command {
            Command::Help => ResponseFrame::new(
                frame.id,
                false,
                "commands: help, status, list, register <name>, start <name>, stop <name>, boot, uptime, health, events [limit], history [limit], clear-events, shutdown, save <path>, load <path>, exit",
            ),
            Command::Status | Command::List => {
                ResponseFrame::new(frame.id, false, self.service_manager.status_report())
            }
            Command::Register(name) => match self.service_manager.register(name.clone()) {
                Ok(()) => ResponseFrame::new(frame.id, false, format!("registered service '{name}'")),
                Err(error) => ResponseFrame::new(frame.id, false, map_service_error(error)),
            },
            Command::Start(name) => match self.service_manager.start(&name) {
                Ok(()) => ResponseFrame::new(frame.id, false, format!("started service '{name}'")),
                Err(error) => ResponseFrame::new(frame.id, false, map_service_error(error)),
            },
            Command::Stop(name) => match self.service_manager.stop(&name) {
                Ok(()) => ResponseFrame::new(frame.id, false, format!("stopped service '{name}'")),
                Err(error) => ResponseFrame::new(frame.id, false, map_service_error(error)),
            },
            Command::Boot => ResponseFrame::new(
                frame.id,
                false,
                format_boot_report(self.boot_duration, self.shell_start.elapsed()),
            ),
            Command::Uptime => ResponseFrame::new(
                frame.id,
                false,
                format!("uptime_ms: {}", self.shell_start.elapsed().as_millis()),
            ),
            Command::Health => {
                let stats = self.service_manager.stats();
                ResponseFrame::new(
                    frame.id,
                    false,
                    format!(
                        "services_total: {}\nservices_running: {}\nservices_inactive: {}",
                        stats.total, stats.running, stats.inactive
                    ),
                )
            }
            Command::Events(limit) => {
                let events = self.service_manager.recent_events(limit.unwrap_or(20));
                if events.is_empty() {
                    ResponseFrame::new(frame.id, false, "no events recorded")
                } else {
                    ResponseFrame::new(frame.id, false, events.join("\n"))
                }
            }
            Command::History(limit) => {
                let limit = limit.unwrap_or(20);
                let len = self.command_history.len();
                let start = len.saturating_sub(limit);
                let recent = &self.command_history[start..];
                if recent.is_empty() {
                    ResponseFrame::new(frame.id, false, "no command history")
                } else {
                    ResponseFrame::new(frame.id, false, recent.join("\n"))
                }
            }
            Command::ClearEvents => {
                let removed = self.service_manager.clear_events();
                ResponseFrame::new(frame.id, false, format!("cleared {removed} events"))
            }
            Command::Shutdown => {
                ResponseFrame::with_shutdown(frame.id, true, true, "daemon shutdown requested")
            }
            Command::Save(path) => match fs::write(&path, self.service_manager.export_snapshot()) {
                Ok(()) => ResponseFrame::new(
                    frame.id,
                    false,
                    format!("saved service snapshot to '{path}'"),
                ),
                Err(error) => {
                    ResponseFrame::new(frame.id, false, format!("failed to save snapshot: {error}"))
                }
            },
            Command::Load(path) => match fs::read_to_string(&path) {
                Ok(content) => match self.service_manager.import_snapshot(&content) {
                    Ok(count) => {
                        ResponseFrame::new(frame.id, false, format!("loaded {count} services from '{path}'"))
                    }
                    Err(error) => ResponseFrame::new(frame.id, false, map_service_error(error)),
                },
                Err(error) => {
                    ResponseFrame::new(frame.id, false, format!("failed to read snapshot: {error}"))
                }
            },
            Command::Exit => ResponseFrame::new(frame.id, true, "shutting down shell"),
            Command::Unknown(command) => {
                if command.is_empty() {
                    ResponseFrame::new(frame.id, false, "")
                } else {
                    ResponseFrame::new(frame.id, false, format!("unknown command: {command}"))
                }
            }
        };

        response
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ipc::CommandFrame;
    use svc_manager::ServiceManager;

    use super::ControlPlane;

    #[test]
    fn handles_lifecycle_commands() {
        let mut manager = ServiceManager::default();
        manager.register("alpha").expect("register alpha");
        let mut control = ControlPlane::new(manager, Duration::from_millis(5), None);

        let start = control.handle_frame(CommandFrame::new(1, "start alpha"));
        assert_eq!(start.payload, "started service 'alpha'");

        let status = control.handle_frame(CommandFrame::new(2, "status"));
        assert!(status.payload.contains("alpha: running"));
    }

    #[test]
    fn marks_exit_commands() {
        let control = ServiceManager::with_seeded_services();
        let mut control = ControlPlane::new(control, Duration::from_millis(1), None);
        let response = control.handle_frame(CommandFrame::new(8, "exit"));

        assert!(response.exit);
        assert!(!response.shutdown);
        assert_eq!(response.id, 8);
    }

    #[test]
    fn returns_recent_command_history() {
        let manager = ServiceManager::with_seeded_services();
        let mut control = ControlPlane::new(manager, Duration::from_millis(1), None);

        let _ = control.handle_frame(CommandFrame::new(1, "status"));
        let _ = control.handle_frame(CommandFrame::new(2, "health"));
        let response = control.handle_frame(CommandFrame::new(3, "history 2"));

        assert_eq!(response.payload, "health\nhistory 2");
    }

    #[test]
    fn rejects_unauthorized_commands() {
        let manager = ServiceManager::with_seeded_services();
        let mut control = ControlPlane::new(
            manager,
            Duration::from_millis(1),
            Some("secret".to_string()),
        );

        let denied = control.handle_frame(CommandFrame::new(4, "status"));
        assert_eq!(denied.payload, "unauthorized: invalid auth token");

        let allowed =
            control.handle_frame(CommandFrame::with_auth(5, Some("secret".to_string()), "status"));
        assert!(allowed.payload.contains("logging:"));
    }

    #[test]
    fn shutdown_command_sets_shutdown_flag() {
        let manager = ServiceManager::with_seeded_services();
        let mut control = ControlPlane::new(manager, Duration::from_millis(1), None);
        let response = control.handle_frame(CommandFrame::new(9, "shutdown"));

        assert!(response.exit);
        assert!(response.shutdown);
        assert_eq!(response.payload, "daemon shutdown requested");
    }

    #[test]
    fn help_returns_available_commands() {
        let manager = ServiceManager::default();
        let mut control = ControlPlane::new(manager, Duration::from_millis(1), None);
        let response = control.handle_frame(CommandFrame::new(1, "help"));
        assert!(response.payload.contains("help"));
        assert!(response.payload.contains("status"));
        assert!(response.payload.contains("list"));
        assert!(!response.exit);
    }

    #[test]
    fn list_shows_registered_services() {
        let mut manager = ServiceManager::default();
        manager.register("svc-a").unwrap();
        manager.register("svc-b").unwrap();
        let mut control = ControlPlane::new(manager, Duration::from_millis(1), None);
        let response = control.handle_frame(CommandFrame::new(1, "list"));
        assert!(response.payload.contains("svc-a"));
        assert!(response.payload.contains("svc-b"));
    }

    #[test]
    fn unknown_command_returns_error() {
        let manager = ServiceManager::default();
        let mut control = ControlPlane::new(manager, Duration::from_millis(1), None);
        let response = control.handle_frame(CommandFrame::new(1, "nonexistent"));
        assert!(response.payload.contains("unknown"));
    }

    #[test]
    fn register_missing_name() {
        let manager = ServiceManager::default();
        let mut control = ControlPlane::new(manager, Duration::from_millis(1), None);
        let response = control.handle_frame(CommandFrame::new(1, "register"));
        assert!(response.payload.contains("unknown"));
    }

    #[test]
    fn start_nonexistent_service() {
        let manager = ServiceManager::default();
        let mut control = ControlPlane::new(manager, Duration::from_millis(1), None);
        let response = control.handle_frame(CommandFrame::new(1, "start ghost"));
        assert!(response.payload.contains("error") || response.payload.contains("not found") || response.payload.contains("Error"));
    }

    #[test]
    fn stop_nonexistent_service() {
        let manager = ServiceManager::default();
        let mut control = ControlPlane::new(manager, Duration::from_millis(1), None);
        let response = control.handle_frame(CommandFrame::new(1, "stop ghost"));
        assert!(response.payload.contains("error") || response.payload.contains("not found") || response.payload.contains("Error"));
    }

    #[test]
    fn uptime_returns_duration() {
        let manager = ServiceManager::default();
        let mut control = ControlPlane::new(manager, Duration::from_millis(1), None);
        let response = control.handle_frame(CommandFrame::new(1, "uptime"));
        // Should contain some time indicator
        assert!(!response.payload.is_empty());
    }

    #[test]
    fn health_returns_info() {
        let manager = ServiceManager::with_seeded_services();
        let mut control = ControlPlane::new(manager, Duration::from_millis(1), None);
        let response = control.handle_frame(CommandFrame::new(1, "health"));
        assert!(!response.payload.is_empty());
    }

    #[test]
    fn events_with_no_events() {
        let manager = ServiceManager::default();
        let mut control = ControlPlane::new(manager, Duration::from_millis(1), None);
        let response = control.handle_frame(CommandFrame::new(1, "events"));
        // Should not panic, may be empty or show "no events"
        assert!(!response.exit);
    }

    #[test]
    fn clear_events_does_not_panic() {
        let manager = ServiceManager::default();
        let mut control = ControlPlane::new(manager, Duration::from_millis(1), None);
        let _ = control.handle_frame(CommandFrame::new(1, "status"));
        let response = control.handle_frame(CommandFrame::new(2, "clear-events"));
        assert!(!response.exit);
    }

    #[test]
    fn response_ids_match_request() {
        let manager = ServiceManager::default();
        let mut control = ControlPlane::new(manager, Duration::from_millis(1), None);
        for id in [1u64, 42, 999] {
            let response = control.handle_frame(CommandFrame::new(id, "help"));
            assert_eq!(response.id, id);
        }
    }
}
