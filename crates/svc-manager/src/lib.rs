#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Inactive,
    Running,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    pub name: String,
    pub state: ServiceState,
}

impl Service {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            state: ServiceState::Inactive,
        }
    }

    pub fn with_state(name: impl Into<String>, state: ServiceState) -> Self {
        Self {
            name: name.into(),
            state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    AlreadyExists(String),
    NotFound(String),
    AlreadyRunning(String),
    AlreadyStopped(String),
    InvalidSnapshot(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::AlreadyExists(name) => write!(f, "service '{name}' already exists"),
            ServiceError::NotFound(name) => write!(f, "service '{name}' not found"),
            ServiceError::AlreadyRunning(name) => write!(f, "service '{name}' is already running"),
            ServiceError::AlreadyStopped(name) => write!(f, "service '{name}' is already stopped"),
            ServiceError::InvalidSnapshot(reason) => write!(f, "invalid snapshot: {reason}"),
        }
    }
}

#[derive(Debug, Default)]
pub struct ServiceManager {
    services: Vec<Service>,
    events: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceStats {
    pub total: usize,
    pub running: usize,
    pub inactive: usize,
}

impl ServiceManager {
    pub fn with_seeded_services() -> Self {
        let mut manager = Self::default();
        manager.register("logging").expect("seed logging");
        manager.register("session").expect("seed session");
        manager
            .register("notification")
            .expect("seed notification");
        manager
    }

    pub fn register(&mut self, name: impl Into<String>) -> Result<(), ServiceError> {
        let name = name.into();
        if self.services.iter().any(|service| service.name == name) {
            return Err(ServiceError::AlreadyExists(name));
        }

        self.services.push(Service::new(name.clone()));
        self.events.push(format!("registered service '{name}'"));
        Ok(())
    }

    pub fn start_all(&mut self) {
        let mut started = Vec::new();
        for service in &mut self.services {
            service.state = ServiceState::Running;
            started.push(service.name.clone());
        }

        for name in started {
            self.events.push(format!("started service '{name}'"));
        }
    }

    pub fn status_report(&self) -> String {
        if self.services.is_empty() {
            return "no services registered".to_string();
        }

        self.services
            .iter()
            .map(|service| {
                let state = match service.state {
                    ServiceState::Inactive => "inactive",
                    ServiceState::Running => "running",
                };
                format!("{}: {state}", service.name)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn list(&self) -> Vec<Service> {
        self.services.clone()
    }

    pub fn start(&mut self, name: &str) -> Result<(), ServiceError> {
        let service = self
            .services
            .iter_mut()
            .find(|service| service.name == name)
            .ok_or_else(|| ServiceError::NotFound(name.to_string()))?;

        if service.state == ServiceState::Running {
            return Err(ServiceError::AlreadyRunning(name.to_string()));
        }

        service.state = ServiceState::Running;
        self.events.push(format!("started service '{name}'"));
        Ok(())
    }

    pub fn stop(&mut self, name: &str) -> Result<(), ServiceError> {
        let service = self
            .services
            .iter_mut()
            .find(|service| service.name == name)
            .ok_or_else(|| ServiceError::NotFound(name.to_string()))?;

        if service.state == ServiceState::Inactive {
            return Err(ServiceError::AlreadyStopped(name.to_string()));
        }

        service.state = ServiceState::Inactive;
        self.events.push(format!("stopped service '{name}'"));
        Ok(())
    }

    pub fn recent_events(&self, limit: usize) -> Vec<String> {
        let len = self.events.len();
        let start = len.saturating_sub(limit);
        self.events[start..].to_vec()
    }

    pub fn clear_events(&mut self) -> usize {
        let removed = self.events.len();
        self.events.clear();
        removed
    }

    pub fn export_snapshot(&self) -> String {
        self.services
            .iter()
            .map(|service| {
                let state = match service.state {
                    ServiceState::Inactive => "inactive",
                    ServiceState::Running => "running",
                };
                format!("{},{}", service.name, state)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn import_snapshot(&mut self, snapshot: &str) -> Result<usize, ServiceError> {
        let mut loaded = Vec::new();
        for (line_number, line) in snapshot.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            let Some((name, state_raw)) = line.split_once(',') else {
                return Err(ServiceError::InvalidSnapshot(format!(
                    "line {} has invalid format",
                    line_number + 1
                )));
            };

            let name = name.trim();
            if name.is_empty() {
                return Err(ServiceError::InvalidSnapshot(format!(
                    "line {} has empty service name",
                    line_number + 1
                )));
            }

            let state = match state_raw.trim() {
                "inactive" => ServiceState::Inactive,
                "running" => ServiceState::Running,
                _ => {
                    return Err(ServiceError::InvalidSnapshot(format!(
                        "line {} has invalid state",
                        line_number + 1
                    )))
                }
            };

            if loaded.iter().any(|service: &Service| service.name == name) {
                return Err(ServiceError::InvalidSnapshot(format!(
                    "line {} duplicates service '{}'",
                    line_number + 1,
                    name
                )));
            }

            loaded.push(Service::with_state(name.to_string(), state));
        }

        self.services = loaded;
        self.events.push("imported service snapshot".to_string());
        Ok(self.services.len())
    }

    pub fn stats(&self) -> ServiceStats {
        let running = self
            .services
            .iter()
            .filter(|service| service.state == ServiceState::Running)
            .count();
        let total = self.services.len();
        ServiceStats {
            total,
            running,
            inactive: total.saturating_sub(running),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ServiceError, ServiceManager};

    #[test]
    fn starts_seeded_services() {
        let mut manager = ServiceManager::with_seeded_services();
        manager.start_all();
        let report = manager.status_report();

        assert!(report.contains("logging: running"));
        assert!(report.contains("session: running"));
        assert!(report.contains("notification: running"));
    }

    #[test]
    fn defaults_to_inactive_state() {
        let mut manager = ServiceManager::default();
        manager.register("demo").expect("register demo");

        assert!(manager.status_report().contains("demo: inactive"));

        manager.start_all();

        assert!(manager.status_report().contains("demo: running"));
    }

    #[test]
    fn blocks_duplicate_service_registration() {
        let mut manager = ServiceManager::default();
        manager.register("demo").expect("initial register");
        let error = manager.register("demo").expect_err("duplicate should fail");

        assert_eq!(error, ServiceError::AlreadyExists("demo".to_string()));
    }

    #[test]
    fn supports_start_stop_lifecycle() {
        let mut manager = ServiceManager::default();
        manager.register("demo").expect("register demo");

        manager.start("demo").expect("start demo");
        assert!(manager.status_report().contains("demo: running"));

        manager.stop("demo").expect("stop demo");
        assert!(manager.status_report().contains("demo: inactive"));
    }

    #[test]
    fn captures_recent_events() {
        let mut manager = ServiceManager::default();
        manager.register("alpha").expect("register alpha");
        manager.start("alpha").expect("start alpha");
        manager.stop("alpha").expect("stop alpha");

        let events = manager.recent_events(2);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], "started service 'alpha'");
        assert_eq!(events[1], "stopped service 'alpha'");
    }

    #[test]
    fn computes_service_stats() {
        let mut manager = ServiceManager::default();
        manager.register("alpha").expect("register alpha");
        manager.register("beta").expect("register beta");
        manager.start("alpha").expect("start alpha");

        let stats = manager.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.running, 1);
        assert_eq!(stats.inactive, 1);
    }

    #[test]
    fn roundtrips_snapshot() {
        let mut manager = ServiceManager::default();
        manager.register("alpha").expect("register alpha");
        manager.register("beta").expect("register beta");
        manager.start("beta").expect("start beta");

        let snapshot = manager.export_snapshot();
        let mut restored = ServiceManager::default();
        restored
            .import_snapshot(&snapshot)
            .expect("import snapshot should succeed");

        assert_eq!(restored.status_report(), manager.status_report());
    }

    #[test]
    fn clears_event_journal() {
        let mut manager = ServiceManager::default();
        manager.register("alpha").expect("register alpha");
        manager.start("alpha").expect("start alpha");
        assert_eq!(manager.recent_events(10).len(), 2);

        let removed = manager.clear_events();
        assert_eq!(removed, 2);
        assert!(manager.recent_events(10).is_empty());
    }
}
use std::fmt;
