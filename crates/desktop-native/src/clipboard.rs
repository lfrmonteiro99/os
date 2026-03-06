use std::sync::{Arc, Mutex};

/// Simple in-app clipboard (system clipboard requires platform-specific crates)
#[derive(Clone)]
pub struct AppClipboard {
    content: Arc<Mutex<String>>,
}

impl AppClipboard {
    pub fn new() -> Self {
        Self { content: Arc::new(Mutex::new(String::new())) }
    }

    pub fn copy(&self, text: &str) {
        if let Ok(mut c) = self.content.lock() {
            *c = text.to_string();
        }
    }

    pub fn paste(&self) -> String {
        self.content.lock().map(|c| c.clone()).unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.content.lock().map(|c| c.is_empty()).unwrap_or(true)
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        if let Ok(mut c) = self.content.lock() {
            c.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_clipboard_is_empty() {
        let cb = AppClipboard::new();
        assert!(cb.is_empty());
        assert_eq!(cb.paste(), "");
    }

    #[test]
    fn copy_then_paste() {
        let cb = AppClipboard::new();
        cb.copy("hello world");
        assert_eq!(cb.paste(), "hello world");
        assert!(!cb.is_empty());
    }

    #[test]
    fn copy_overwrites() {
        let cb = AppClipboard::new();
        cb.copy("first");
        cb.copy("second");
        assert_eq!(cb.paste(), "second");
    }

    #[test]
    fn clear_empties() {
        let cb = AppClipboard::new();
        cb.copy("data");
        cb.clear();
        assert!(cb.is_empty());
    }

    #[test]
    fn paste_multiple_times() {
        let cb = AppClipboard::new();
        cb.copy("text");
        assert_eq!(cb.paste(), "text");
        assert_eq!(cb.paste(), "text");
    }

    #[test]
    fn clone_shares_state() {
        let cb = AppClipboard::new();
        let cb2 = cb.clone();
        cb.copy("shared");
        assert_eq!(cb2.paste(), "shared");
    }

    #[test]
    fn copy_empty_string() {
        let cb = AppClipboard::new();
        cb.copy("data");
        cb.copy("");
        assert!(cb.is_empty());
    }

    #[test]
    fn multiline_content() {
        let cb = AppClipboard::new();
        cb.copy("line1\nline2\nline3");
        assert_eq!(cb.paste(), "line1\nline2\nline3");
    }
}
