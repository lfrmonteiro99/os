use sysinfo::{Pid, ProcessesToUpdate, System};

pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
}

pub struct ProcessManager {
    sys: System,
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        Self { sys }
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
    }

    pub fn list(&self) -> Vec<ProcessInfo> {
        self.sys
            .processes()
            .iter()
            .map(|(pid, proc_)| ProcessInfo {
                pid: pid.as_u32(),
                name: proc_.name().to_string_lossy().to_string(),
                cpu_usage: proc_.cpu_usage(),
                memory_bytes: proc_.memory(),
            })
            .collect()
    }

    pub fn list_sorted_by_cpu(&self) -> Vec<ProcessInfo> {
        let mut procs = self.list();
        procs.sort_by(|a, b| {
            b.cpu_usage
                .partial_cmp(&a.cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        procs
    }

    pub fn list_sorted_by_memory(&self) -> Vec<ProcessInfo> {
        let mut procs = self.list();
        procs.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
        procs
    }

    #[allow(dead_code)]
    pub fn kill(&self, pid: u32) -> bool {
        if let Some(process) = self.sys.process(Pid::from_u32(pid)) {
            process.kill()
        } else {
            false
        }
    }

    pub fn search(&self, query: &str) -> Vec<ProcessInfo> {
        if query.is_empty() {
            return self.list();
        }
        let q = query.to_lowercase();
        self.list()
            .into_iter()
            .filter(|p| p.name.to_lowercase().contains(&q))
            .collect()
    }

    #[allow(dead_code)]
    pub fn total_count(&self) -> usize {
        self.sys.processes().len()
    }

    pub fn format_memory(bytes: u64) -> String {
        if bytes < 1024 {
            return format!("{bytes} B");
        }
        if bytes < 1024 * 1024 {
            return format!("{:.1} KB", bytes as f64 / 1024.0);
        }
        if bytes < 1024 * 1024 * 1024 {
            return format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0));
        }
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create() {
        let pm = ProcessManager::new();
        assert!(pm.total_count() > 0, "system should have running processes");
    }

    #[test]
    fn list_returns_processes() {
        let pm = ProcessManager::new();
        let procs = pm.list();
        assert!(!procs.is_empty());
    }

    #[test]
    fn processes_have_names() {
        let pm = ProcessManager::new();
        let procs = pm.list();
        let named = procs.iter().filter(|p| !p.name.is_empty()).count();
        assert!(named > 0, "some processes should have names");
    }

    #[test]
    fn processes_have_valid_pids() {
        let pm = ProcessManager::new();
        let procs = pm.list();
        // PID 0 is valid on Windows (System Idle Process)
        let nonzero = procs.iter().filter(|p| p.pid > 0).count();
        assert!(nonzero > 5, "most processes should have PID > 0");
    }

    #[test]
    fn sorted_by_memory_is_descending() {
        let pm = ProcessManager::new();
        let procs = pm.list_sorted_by_memory();
        if procs.len() >= 2 {
            assert!(procs[0].memory_bytes >= procs[1].memory_bytes);
        }
    }

    #[test]
    fn sorted_by_cpu_is_descending() {
        let pm = ProcessManager::new();
        let procs = pm.list_sorted_by_cpu();
        if procs.len() >= 2 {
            assert!(procs[0].cpu_usage >= procs[1].cpu_usage);
        }
    }

    #[test]
    fn search_filters_by_name() {
        let pm = ProcessManager::new();
        // Search for something unlikely to exist
        let results = pm.search("zzzzz_nonexistent_process_zzzzz");
        assert!(results.is_empty());
    }

    #[test]
    fn search_empty_returns_all() {
        let pm = ProcessManager::new();
        let all = pm.list();
        let searched = pm.search("");
        assert_eq!(all.len(), searched.len());
    }

    #[test]
    fn kill_nonexistent_returns_false() {
        let pm = ProcessManager::new();
        assert!(!pm.kill(999_999_999));
    }

    #[test]
    fn format_memory_bytes() {
        assert_eq!(ProcessManager::format_memory(500), "500 B");
    }

    #[test]
    fn format_memory_kb() {
        assert_eq!(ProcessManager::format_memory(2048), "2.0 KB");
    }

    #[test]
    fn format_memory_mb() {
        assert_eq!(ProcessManager::format_memory(10 * 1024 * 1024), "10.0 MB");
    }

    #[test]
    fn format_memory_gb() {
        assert_eq!(
            ProcessManager::format_memory(2 * 1024 * 1024 * 1024),
            "2.0 GB"
        );
    }

    #[test]
    fn refresh_does_not_panic() {
        let mut pm = ProcessManager::new();
        pm.refresh();
        assert!(pm.total_count() > 0);
    }
}
