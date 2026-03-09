//! Embedded Windows application support.
//!
//! Launches an external process, finds its HWND, reparents it as a child
//! of the AuroraOS egui window, and repositions it each frame so it appears
//! as a native panel inside the desktop.

#[cfg(windows)]
mod platform {
    use std::process::{Child, Command};
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowLongW, GetWindowThreadProcessId, IsWindowVisible,
        MoveWindow, SetParent, SetWindowLongW, ShowWindow,
        GWL_EXSTYLE, GWL_STYLE, SW_SHOW,
        WS_CAPTION, WS_CHILD, WS_POPUP, WS_THICKFRAME,
        WS_EX_APPWINDOW, WS_EX_WINDOWEDGE,
    };

    /// State for a single embedded application
    pub struct EmbeddedApp {
        pub label: String,
        pub process: Child,
        pub hwnd: Option<HWND>,
        pub parent_hwnd: Option<HWND>,
        launched_at: Instant,
        pub(crate) hwnd_search_attempts: u32,
        last_rect: Option<(i32, i32, i32, i32)>,
        reparented: bool,
    }

    impl EmbeddedApp {
        pub fn launch(label: &str, program: &str, args: &[&str]) -> Result<Self, String> {
            let process = Command::new(program)
                .args(args)
                .spawn()
                .map_err(|e| format!("Failed to launch '{}': {}", program, e))?;

            Ok(Self {
                label: label.to_string(),
                process,
                hwnd: None,
                parent_hwnd: None,
                launched_at: Instant::now(),
                hwnd_search_attempts: 0,
                last_rect: None,
                reparented: false,
            })
        }

        pub fn is_alive(&mut self) -> bool {
            self.process.try_wait().ok().flatten().is_none()
        }

        pub fn try_find_hwnd(&mut self) -> bool {
            if self.hwnd.is_some() {
                return true;
            }
            if self.launched_at.elapsed() < Duration::from_millis(300) {
                return false;
            }
            self.hwnd_search_attempts += 1;
            if self.hwnd_search_attempts > 50 {
                return false;
            }
            let pid = self.process.id();
            if let Some(h) = find_hwnd_by_pid(pid) {
                self.hwnd = Some(h);
                true
            } else {
                false
            }
        }

        pub fn reparent(&mut self, parent: HWND) {
            if self.reparented { return; }
            let Some(child) = self.hwnd else { return };
            self.parent_hwnd = Some(parent);

            unsafe {
                SetParent(child, parent);

                let style = GetWindowLongW(child, GWL_STYLE) as u32;
                let new_style = (style & !(WS_CAPTION | WS_THICKFRAME | WS_POPUP)) | WS_CHILD;
                SetWindowLongW(child, GWL_STYLE, new_style as i32);

                let ex_style = GetWindowLongW(child, GWL_EXSTYLE) as u32;
                let new_ex = ex_style & !(WS_EX_APPWINDOW | WS_EX_WINDOWEDGE);
                SetWindowLongW(child, GWL_EXSTYLE, new_ex as i32);

                ShowWindow(child, SW_SHOW);
            }
            self.reparented = true;
        }

        pub fn position(&mut self, x: i32, y: i32, w: i32, h: i32) {
            let Some(child) = self.hwnd else { return };
            if w <= 0 || h <= 0 { return; }
            let rect = (x, y, w, h);
            if self.last_rect == Some(rect) { return; }
            unsafe { MoveWindow(child, x, y, w, h, 1); }
            self.last_rect = Some(rect);
        }

        pub fn detach(&mut self) {
            if let Some(child) = self.hwnd.take() {
                unsafe {
                    SetParent(child, std::ptr::null_mut());
                    let style = GetWindowLongW(child, GWL_STYLE) as u32;
                    let new_style = (style & !WS_CHILD) | WS_CAPTION | WS_THICKFRAME;
                    SetWindowLongW(child, GWL_STYLE, new_style as i32);
                    ShowWindow(child, SW_SHOW);
                }
            }
            self.reparented = false;
            self.last_rect = None;
        }

        pub fn kill(&mut self) {
            self.detach();
            let _ = self.process.kill();
        }

        pub fn is_reparented(&self) -> bool { self.reparented }

        pub fn gave_up(&self) -> bool {
            self.hwnd.is_none() && self.hwnd_search_attempts > 50
        }
    }

    impl Drop for EmbeddedApp {
        fn drop(&mut self) { self.kill(); }
    }

    fn find_hwnd_by_pid(target_pid: u32) -> Option<HWND> {
        struct Ctx { target_pid: u32, result: Option<HWND> }

        unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let ctx = &mut *(lparam as *mut Ctx);
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == ctx.target_pid && IsWindowVisible(hwnd) != 0 {
                ctx.result = Some(hwnd);
                return 0;
            }
            1
        }

        let mut ctx = Ctx { target_pid, result: None };
        unsafe { EnumWindows(Some(cb), &mut ctx as *mut Ctx as LPARAM); }
        ctx.result
    }

    /// List visible top-level windows with their titles and PIDs.
    #[allow(dead_code)]
    pub fn list_visible_windows() -> Vec<(HWND, u32, String)> {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowTextLengthW, GetWindowTextW};

        struct ListCtx { windows: Vec<(HWND, u32, String)> }

        unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let ctx = &mut *(lparam as *mut ListCtx);
            if IsWindowVisible(hwnd) == 0 { return 1; }
            let len = GetWindowTextLengthW(hwnd);
            if len <= 0 { return 1; }
            let mut buf = vec![0u16; (len + 1) as usize];
            let actual = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            if actual > 0 {
                let title = String::from_utf16_lossy(&buf[..actual as usize]);
                if !title.is_empty() && !title.contains("AuroraOS") {
                    let mut pid: u32 = 0;
                    GetWindowThreadProcessId(hwnd, &mut pid);
                    ctx.windows.push((hwnd, pid, title));
                }
            }
            1
        }

        let mut ctx = ListCtx { windows: Vec::new() };
        unsafe { EnumWindows(Some(cb), &mut ctx as *mut ListCtx as LPARAM); }
        ctx.windows
    }

    /// Find our own HWND by PID + title match.
    pub fn find_own_hwnd() -> Option<HWND> {
        let our_pid = std::process::id();
        // list_visible_windows excludes "AuroraOS", so we do a direct enumeration
        struct Ctx { pid: u32, result: Option<HWND> }

        unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowTextLengthW, GetWindowTextW};
            let ctx = &mut *(lparam as *mut Ctx);
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid != ctx.pid || IsWindowVisible(hwnd) == 0 { return 1; }
            let len = GetWindowTextLengthW(hwnd);
            if len <= 0 { return 1; }
            let mut buf = vec![0u16; (len + 1) as usize];
            let actual = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            if actual > 0 {
                let title = String::from_utf16_lossy(&buf[..actual as usize]);
                if title.contains("AuroraOS") {
                    ctx.result = Some(hwnd);
                    return 0;
                }
            }
            1
        }

        let mut ctx = Ctx { pid: our_pid, result: None };
        unsafe { EnumWindows(Some(cb), &mut ctx as *mut Ctx as LPARAM); }
        ctx.result
    }

    // NOTE: Window capture (PrintWindow/BitBlt) for GPU-accelerated apps
    // is available via Win32_Storage_Xps feature if needed in the future.
    // The SetParent reparenting approach works for most standard Win32 apps.
}

#[cfg(windows)]
pub use platform::*;

// Stub for non-Windows platforms
#[cfg(not(windows))]
pub struct EmbeddedApp {
    pub label: String,
}

#[cfg(not(windows))]
impl EmbeddedApp {
    pub fn launch(_label: &str, _program: &str, _args: &[&str]) -> Result<Self, String> {
        Err("Embedded apps are only supported on Windows".to_string())
    }
    pub fn is_alive(&mut self) -> bool { false }
    pub fn try_find_hwnd(&mut self) -> bool { false }
    #[allow(dead_code)]
    pub fn reparent(&mut self, _parent: isize) {}
    pub fn position(&mut self, _x: i32, _y: i32, _w: i32, _h: i32) {}
    pub fn detach(&mut self) {}
    pub fn kill(&mut self) {}
    pub fn is_reparented(&self) -> bool { false }
    pub fn gave_up(&self) -> bool { true }
}

#[cfg(not(windows))]
pub fn list_visible_windows() -> Vec<(isize, u32, String)> { Vec::new() }

#[cfg(not(windows))]
pub fn find_own_hwnd() -> Option<isize> { None }

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_nonexistent_fails() {
        let result = EmbeddedApp::launch("test", "__nonexistent_binary__", &[]);
        assert!(result.is_err());
    }

    #[cfg(windows)]
    #[test]
    fn launch_notepad_is_alive() {
        // Windows 11 notepad is a UWP app — its HWND may belong to a different PID.
        // We verify the process launches and stays alive; HWND matching is best-effort.
        let mut app = EmbeddedApp::launch("Notepad", "notepad.exe", &[]).unwrap();
        assert!(app.is_alive());
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert!(app.is_alive(), "Notepad should still be running");
        app.kill();
    }

    #[cfg(windows)]
    #[test]
    fn list_visible_windows_returns_some() {
        let windows = list_visible_windows();
        assert!(!windows.is_empty(), "Should find at least one visible window");
    }

    #[cfg(windows)]
    #[test]
    fn find_own_hwnd_works() {
        // May or may not find our HWND in a test context (no GUI), just check no panic
        let _ = find_own_hwnd();
    }

    #[cfg(windows)]
    #[test]
    fn gave_up_after_max_attempts() {
        let mut app = EmbeddedApp::launch("test", "cmd.exe", &["/C", "timeout", "/t", "10"]).unwrap();
        app.hwnd_search_attempts = 51;
        assert!(app.gave_up());
        app.kill();
    }

    #[test]
    fn detach_without_hwnd_is_safe() {
        if let Ok(mut app) = EmbeddedApp::launch("test", "cmd.exe", &["/C", "echo", "hi"]) {
            app.detach();
            app.kill();
        }
    }
}
