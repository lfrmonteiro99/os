use eframe::egui::{Pos2, Rect, Vec2};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SnapSide {
    Left,
    Right,
    LeftThird,
    CenterThird,
    RightThird,
}

#[derive(Clone, Copy)]
pub struct WindowSnapshot {
    pub open: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub snap: Option<SnapSide>,
    pub default_pos: Pos2,
    pub default_size: Vec2,
    pub id_epoch: u64,
}

pub struct ManagedWindow {
    pub open: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub snap: Option<SnapSide>,
    pub default_pos: Pos2,
    pub default_size: Vec2,
    pub restore_rect: Option<Rect>,
    pub id_epoch: u64,
    pub open_anim_start: Option<Instant>,
    pub closing: bool,
    pub close_anim_start: Option<Instant>,
    pub minimizing: bool,
    pub minimize_anim_start: Option<Instant>,
    /// Which virtual desktop this window belongs to (0-based)
    pub desktop: usize,
}

impl ManagedWindow {
    pub fn new(default_pos: Pos2, default_size: Vec2) -> Self {
        Self {
            open: true,
            minimized: false,
            maximized: false,
            snap: None,
            default_pos,
            default_size,
            restore_rect: None,
            id_epoch: 0,
            open_anim_start: None,
            closing: false,
            close_anim_start: None,
            minimizing: false,
            minimize_anim_start: None,
            desktop: 0,
        }
    }

    pub fn snapshot(&self) -> WindowSnapshot {
        WindowSnapshot {
            open: self.open,
            minimized: self.minimized,
            maximized: self.maximized,
            snap: self.snap,
            default_pos: self.default_pos,
            default_size: self.default_size,
            id_epoch: self.id_epoch,
        }
    }

    pub fn restore(&mut self) {
        self.open = true;
        self.minimized = false;
        self.minimizing = false;
        self.minimize_anim_start = None;
        self.open_anim_start = Some(Instant::now());
        self.closing = false;
        self.close_anim_start = None;
    }

    pub fn start_close(&mut self) {
        self.closing = true;
        self.close_anim_start = Some(Instant::now());
    }

    #[allow(dead_code)]
    pub fn start_minimize(&mut self) {
        self.minimizing = true;
        self.minimize_anim_start = Some(Instant::now());
    }

    #[allow(dead_code)]
    pub fn finalize_close(&mut self) {
        self.open = false;
        self.minimized = false;
        self.closing = false;
        self.close_anim_start = None;
        self.id_epoch = self.id_epoch.saturating_add(1);
    }

    #[allow(dead_code)]
    pub fn finalize_minimize(&mut self) {
        self.minimized = true;
        self.minimizing = false;
        self.minimize_anim_start = None;
        self.id_epoch = self.id_epoch.saturating_add(1);
    }

    pub fn anim_alpha(&self) -> f32 {
        const OPEN_DUR: f32 = 0.2;
        const CLOSE_DUR: f32 = 0.15;
        const MIN_DUR: f32 = 0.25;
        if let Some(t) = self.minimize_anim_start {
            let e = t.elapsed().as_secs_f32() / MIN_DUR;
            return (1.0 - e).clamp(0.0, 1.0);
        }
        if let Some(t) = self.close_anim_start {
            let e = t.elapsed().as_secs_f32() / CLOSE_DUR;
            return (1.0 - e).clamp(0.0, 1.0);
        }
        if let Some(t) = self.open_anim_start {
            let e = t.elapsed().as_secs_f32() / OPEN_DUR;
            return e.clamp(0.0, 1.0);
        }
        1.0
    }

    pub fn anim_scale(&self) -> f32 {
        const OPEN_DUR: f32 = 0.2;
        const CLOSE_DUR: f32 = 0.15;
        const MIN_DUR: f32 = 0.25;
        if let Some(t) = self.minimize_anim_start {
            let e = t.elapsed().as_secs_f32() / MIN_DUR;
            let p = (1.0 - e).clamp(0.0, 1.0);
            return 0.3 + 0.7 * p;
        }
        if let Some(t) = self.close_anim_start {
            let e = t.elapsed().as_secs_f32() / CLOSE_DUR;
            let p = (1.0 - e).clamp(0.0, 1.0);
            return 0.95 + 0.05 * p;
        }
        if let Some(t) = self.open_anim_start {
            let e = t.elapsed().as_secs_f32() / OPEN_DUR;
            let p = e.clamp(0.0, 1.0);
            return 0.92 + 0.08 * p;
        }
        1.0
    }

    pub fn is_close_anim_done(&self) -> bool {
        self.close_anim_start
            .map_or(false, |t| t.elapsed().as_secs_f32() > 0.15)
    }

    pub fn is_minimize_anim_done(&self) -> bool {
        self.minimize_anim_start
            .map_or(false, |t| t.elapsed().as_secs_f32() > 0.25)
    }
}

/// Compute snap rect for a given side within a work area
#[allow(dead_code)]
pub fn snap_rect(work_rect: Rect, side: SnapSide) -> Rect {
    let w = work_rect.width();
    let h = work_rect.height();
    let top = work_rect.top();
    let left = work_rect.left();
    match side {
        SnapSide::Left => Rect::from_min_size(work_rect.left_top(), Vec2::new(w * 0.5, h)),
        SnapSide::Right => {
            Rect::from_min_size(Pos2::new(left + w * 0.5, top), Vec2::new(w * 0.5, h))
        }
        SnapSide::LeftThird => Rect::from_min_size(work_rect.left_top(), Vec2::new(w / 3.0, h)),
        SnapSide::CenterThird => {
            Rect::from_min_size(Pos2::new(left + w / 3.0, top), Vec2::new(w / 3.0, h))
        }
        SnapSide::RightThird => {
            Rect::from_min_size(Pos2::new(left + w * 2.0 / 3.0, top), Vec2::new(w / 3.0, h))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ── ManagedWindow ────────────────────────────────────────────────

    #[test]
    fn new_defaults() {
        let w = ManagedWindow::new(Pos2::new(10.0, 20.0), Vec2::new(300.0, 200.0));
        assert!(w.open);
        assert!(!w.minimized);
        assert!(!w.maximized);
        assert!(w.snap.is_none());
        assert!(!w.closing);
        assert!(!w.minimizing);
        assert_eq!(w.id_epoch, 0);
    }

    #[test]
    fn snapshot_reflects_state() {
        let mut w = ManagedWindow::new(Pos2::new(50.0, 60.0), Vec2::new(400.0, 300.0));
        w.maximized = true;
        w.id_epoch = 5;
        let snap = w.snapshot();
        assert!(snap.maximized);
        assert_eq!(snap.default_pos, Pos2::new(50.0, 60.0));
        assert_eq!(snap.default_size, Vec2::new(400.0, 300.0));
        assert_eq!(snap.id_epoch, 5);
    }

    #[test]
    fn restore_clears_all_states() {
        let mut w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        w.minimized = true;
        w.closing = true;
        w.minimizing = true;
        w.close_anim_start = Some(Instant::now());
        w.minimize_anim_start = Some(Instant::now());
        w.restore();
        assert!(w.open);
        assert!(!w.minimized);
        assert!(!w.closing);
        assert!(!w.minimizing);
        assert!(w.close_anim_start.is_none());
        assert!(w.minimize_anim_start.is_none());
        assert!(w.open_anim_start.is_some());
    }

    #[test]
    fn start_close_sets_state() {
        let mut w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        w.start_close();
        assert!(w.closing);
        assert!(w.close_anim_start.is_some());
    }

    #[test]
    fn start_minimize_sets_state() {
        let mut w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        w.start_minimize();
        assert!(w.minimizing);
        assert!(w.minimize_anim_start.is_some());
    }

    #[test]
    fn finalize_close_resets_and_closes() {
        let mut w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        let epoch_before = w.id_epoch;
        w.start_close();
        w.finalize_close();
        assert!(!w.open);
        assert!(!w.closing);
        assert!(w.close_anim_start.is_none());
        assert_eq!(w.id_epoch, epoch_before + 1);
    }

    #[test]
    fn finalize_minimize_resets_and_minimizes() {
        let mut w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        let epoch_before = w.id_epoch;
        w.start_minimize();
        w.finalize_minimize();
        assert!(w.minimized);
        assert!(!w.minimizing);
        assert!(w.minimize_anim_start.is_none());
        assert_eq!(w.id_epoch, epoch_before + 1);
    }

    // ── Animation ────────────────────────────────────────────────────

    #[test]
    fn anim_alpha_default_is_1() {
        let w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        assert_eq!(w.anim_alpha(), 1.0);
    }

    #[test]
    fn anim_scale_default_is_1() {
        let w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        assert_eq!(w.anim_scale(), 1.0);
    }

    #[test]
    fn close_anim_starts_near_1() {
        let mut w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        w.close_anim_start = Some(Instant::now());
        assert!(w.anim_alpha() > 0.9);
    }

    #[test]
    fn close_anim_done_after_duration() {
        let mut w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        w.close_anim_start = Some(Instant::now() - Duration::from_millis(200));
        assert!(w.is_close_anim_done());
    }

    #[test]
    fn close_anim_not_done_immediately() {
        let mut w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        w.close_anim_start = Some(Instant::now());
        assert!(!w.is_close_anim_done());
    }

    #[test]
    fn minimize_anim_done_after_duration() {
        let mut w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        w.minimize_anim_start = Some(Instant::now() - Duration::from_millis(300));
        assert!(w.is_minimize_anim_done());
    }

    #[test]
    fn minimize_anim_scale_shrinks() {
        let mut w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        w.minimize_anim_start = Some(Instant::now() - Duration::from_millis(200));
        assert!(w.anim_scale() < 0.8, "minimize should scale down");
    }

    #[test]
    fn minimize_priority_over_close() {
        // If both are set, minimize takes priority
        let mut w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        w.close_anim_start = Some(Instant::now());
        w.minimize_anim_start = Some(Instant::now() - Duration::from_millis(200));
        // minimize is 80% done → alpha should be near 0.2
        assert!(w.anim_alpha() < 0.5, "minimize should take priority");
    }

    // ── snap_rect ────────────────────────────────────────────────────

    #[test]
    fn snap_left_is_left_half() {
        let work = Rect::from_min_size(Pos2::new(0.0, 34.0), Vec2::new(1440.0, 800.0));
        let r = snap_rect(work, SnapSide::Left);
        assert_eq!(r.left(), 0.0);
        assert_eq!(r.width(), 720.0);
        assert_eq!(r.height(), 800.0);
        assert_eq!(r.top(), 34.0);
    }

    #[test]
    fn snap_right_is_right_half() {
        let work = Rect::from_min_size(Pos2::new(0.0, 34.0), Vec2::new(1440.0, 800.0));
        let r = snap_rect(work, SnapSide::Right);
        assert_eq!(r.left(), 720.0);
        assert_eq!(r.width(), 720.0);
        assert_eq!(r.top(), 34.0);
    }

    #[test]
    fn desktop_defaults_to_zero() {
        let w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        assert_eq!(w.desktop, 0);
    }

    #[test]
    fn desktop_can_be_changed() {
        let mut w = ManagedWindow::new(Pos2::ZERO, Vec2::new(100.0, 100.0));
        w.desktop = 2;
        assert_eq!(w.desktop, 2);
    }

    #[test]
    fn snap_rect_with_offset_origin() {
        let work = Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(800.0, 600.0));
        let l = snap_rect(work, SnapSide::Left);
        let r = snap_rect(work, SnapSide::Right);
        assert_eq!(l.left(), 100.0);
        assert_eq!(l.width(), 400.0);
        assert_eq!(r.left(), 500.0);
        assert_eq!(r.width(), 400.0);
    }

    // ── Snap thirds ─────────────────────────────────────────────────────

    #[test]
    fn snap_left_third() {
        let work = Rect::from_min_size(Pos2::new(0.0, 34.0), Vec2::new(1440.0, 800.0));
        let r = snap_rect(work, SnapSide::LeftThird);
        assert_eq!(r.left(), 0.0);
        assert!((r.width() - 480.0).abs() < 0.1);
        assert_eq!(r.height(), 800.0);
    }

    #[test]
    fn snap_center_third() {
        let work = Rect::from_min_size(Pos2::new(0.0, 34.0), Vec2::new(1440.0, 800.0));
        let r = snap_rect(work, SnapSide::CenterThird);
        assert!((r.left() - 480.0).abs() < 0.1);
        assert!((r.width() - 480.0).abs() < 0.1);
        assert_eq!(r.top(), 34.0);
    }

    #[test]
    fn snap_right_third() {
        let work = Rect::from_min_size(Pos2::new(0.0, 34.0), Vec2::new(1440.0, 800.0));
        let r = snap_rect(work, SnapSide::RightThird);
        assert!((r.left() - 960.0).abs() < 0.1);
        assert!((r.width() - 480.0).abs() < 0.1);
    }

    #[test]
    fn thirds_cover_full_width() {
        let work = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(900.0, 600.0));
        let l = snap_rect(work, SnapSide::LeftThird);
        let c = snap_rect(work, SnapSide::CenterThird);
        let r = snap_rect(work, SnapSide::RightThird);
        let total = l.width() + c.width() + r.width();
        assert!(
            (total - 900.0).abs() < 0.1,
            "Thirds should cover full width, got {}",
            total
        );
    }

    #[test]
    fn thirds_are_contiguous() {
        let work = Rect::from_min_size(Pos2::new(50.0, 30.0), Vec2::new(1200.0, 700.0));
        let l = snap_rect(work, SnapSide::LeftThird);
        let c = snap_rect(work, SnapSide::CenterThird);
        let r = snap_rect(work, SnapSide::RightThird);
        assert!((l.right() - c.left()).abs() < 0.1);
        assert!((c.right() - r.left()).abs() < 0.1);
    }
}
