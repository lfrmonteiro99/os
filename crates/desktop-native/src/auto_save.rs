use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Tracks auto-save timing and crash recovery state
pub struct AutoSave {
    interval: Duration,
    last_save: Option<Instant>,
    dirty: bool,
    recovery_dir: PathBuf,
}

impl AutoSave {
    pub fn new(interval_secs: u64, recovery_dir: PathBuf) -> Self {
        Self {
            interval: Duration::from_secs(interval_secs),
            last_save: None,
            dirty: false,
            recovery_dir,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
        self.last_save = Some(Instant::now());
    }

    #[allow(dead_code)]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn should_save(&self) -> bool {
        if !self.dirty { return false; }
        match self.last_save {
            Some(t) => t.elapsed() >= self.interval,
            None => true,
        }
    }

    pub fn recovery_file(&self, name: &str) -> PathBuf {
        self.recovery_dir.join(format!(".aurora_recovery_{}", name))
    }

    pub fn save_recovery(&mut self, name: &str, content: &str) -> Result<(), String> {
        let path = self.recovery_file(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
        std::fs::write(&path, content).map_err(|e| format!("write: {e}"))?;
        self.mark_clean();
        Ok(())
    }

    pub fn load_recovery(&self, name: &str) -> Option<String> {
        let path = self.recovery_file(name);
        std::fs::read_to_string(&path).ok()
    }

    #[allow(dead_code)]
    pub fn clear_recovery(&self, name: &str) {
        let path = self.recovery_file(name);
        let _ = std::fs::remove_file(&path);
    }

    #[allow(dead_code)]
    pub fn has_recovery(&self, name: &str) -> bool {
        self.recovery_file(name).exists()
    }

    #[allow(dead_code)]
    pub fn set_interval(&mut self, secs: u64) {
        self.interval = Duration::from_secs(secs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn test_dir() -> PathBuf {
        env::temp_dir().join("aurora_test_autosave")
    }

    fn cleanup(dir: &PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn new_is_not_dirty() {
        let a = AutoSave::new(30, test_dir());
        assert!(!a.is_dirty());
        assert!(!a.should_save());
    }

    #[test]
    fn mark_dirty_triggers_save() {
        let mut a = AutoSave::new(30, test_dir());
        a.mark_dirty();
        assert!(a.is_dirty());
        assert!(a.should_save()); // no last_save, so should save immediately
    }

    #[test]
    fn mark_clean_resets() {
        let mut a = AutoSave::new(30, test_dir());
        a.mark_dirty();
        a.mark_clean();
        assert!(!a.is_dirty());
        assert!(!a.should_save());
    }

    #[test]
    fn should_save_respects_interval() {
        let mut a = AutoSave::new(9999, test_dir());
        a.mark_dirty();
        a.mark_clean(); // sets last_save to now
        a.mark_dirty();
        // interval is 9999s, last_save is just now → should NOT save
        assert!(!a.should_save());
    }

    #[test]
    fn should_save_after_interval_expired() {
        let mut a = AutoSave::new(0, test_dir()); // 0 second interval = always save
        a.mark_dirty();
        a.mark_clean();
        a.mark_dirty();
        assert!(a.should_save());
    }

    #[test]
    fn recovery_file_path() {
        let a = AutoSave::new(30, PathBuf::from("/tmp/test"));
        let path = a.recovery_file("notes");
        assert!(path.to_string_lossy().contains("aurora_recovery_notes"));
    }

    #[test]
    fn save_and_load_recovery() {
        let dir = test_dir();
        cleanup(&dir);
        let mut a = AutoSave::new(30, dir.clone());
        a.mark_dirty();
        a.save_recovery("test", "hello recovery").unwrap();
        assert!(!a.is_dirty()); // save_recovery calls mark_clean
        assert!(a.has_recovery("test"));
        assert_eq!(a.load_recovery("test"), Some("hello recovery".to_string()));
        a.clear_recovery("test");
        assert!(!a.has_recovery("test"));
        cleanup(&dir);
    }

    #[test]
    fn load_nonexistent_returns_none() {
        let a = AutoSave::new(30, test_dir());
        assert_eq!(a.load_recovery("nonexistent_xyz"), None);
    }

    #[test]
    fn set_interval_changes_timing() {
        let mut a = AutoSave::new(30, test_dir());
        a.set_interval(60);
        a.mark_dirty();
        a.mark_clean();
        a.mark_dirty();
        // 60s interval, just saved → should not trigger
        assert!(!a.should_save());
    }

    #[test]
    fn has_recovery_false_when_missing() {
        let a = AutoSave::new(30, test_dir());
        assert!(!a.has_recovery("definitely_missing_xyz"));
    }
}
