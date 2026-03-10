use eframe::egui::Color32;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub struct Toast {
    pub title: String,
    pub body: String,
    pub color: Color32,
    pub created: Instant,
    pub duration: Duration,
}

impl Toast {
    pub fn new(title: impl Into<String>, body: impl Into<String>, color: Color32) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            color,
            created: Instant::now(),
            duration: Duration::from_secs(4),
        }
    }

    pub fn progress(&self) -> f32 {
        self.created.elapsed().as_secs_f32() / self.duration.as_secs_f32()
    }

    pub fn is_expired(&self) -> bool {
        self.created.elapsed() > self.duration
    }

    /// Compute slide-in/out factor with ease-out cubic (0.0 = off-screen, 1.0 = fully visible)
    pub fn slide_factor(&self) -> f32 {
        let p = self.progress();
        if p < 0.1 {
            ease_out_cubic(p / 0.1)
        } else if p > 0.85 {
            ease_out_cubic((1.0 - p) / 0.15)
        } else {
            1.0
        }
    }
}

/// Ease-out cubic: decelerating curve, feels natural
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Manages a queue of toasts with a max visible count.
pub struct ToastManager {
    toasts: VecDeque<Toast>,
    max_visible: usize,
}

impl ToastManager {
    pub fn new(max_visible: usize) -> Self {
        Self {
            toasts: VecDeque::new(),
            max_visible,
        }
    }

    pub fn push(&mut self, toast: Toast) {
        self.toasts.push_back(toast);
    }

    /// Remove expired toasts.
    pub fn tick(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }

    /// Get the currently visible toasts (up to max_visible).
    pub fn visible(&self) -> impl Iterator<Item = &Toast> {
        self.toasts
            .iter()
            .filter(|t| !t.is_expired())
            .take(self.max_visible)
    }

    /// How many non-expired toasts are hidden beyond max_visible.
    pub fn overflow_count(&self) -> usize {
        let active = self.toasts.iter().filter(|t| !t.is_expired()).count();
        active.saturating_sub(self.max_visible)
    }

    pub fn is_empty(&self) -> bool {
        self.toasts.iter().all(|t| t.is_expired())
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.toasts.iter().filter(|t| !t.is_expired()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_expired_when_fresh() {
        let t = Toast::new("Test", "body", Color32::WHITE);
        assert!(!t.is_expired());
        assert!(t.progress() < 0.5);
    }

    #[test]
    fn expired_after_duration() {
        let mut t = Toast::new("Test", "body", Color32::WHITE);
        t.created = Instant::now() - Duration::from_secs(5);
        assert!(t.is_expired());
        assert!(t.progress() > 1.0);
    }

    #[test]
    fn stores_fields() {
        let t = Toast::new("Title", "Body text", Color32::from_rgb(255, 0, 0));
        assert_eq!(t.title, "Title");
        assert_eq!(t.body, "Body text");
        assert_eq!(t.color, Color32::from_rgb(255, 0, 0));
        assert_eq!(t.duration, Duration::from_secs(4));
    }

    #[test]
    fn slide_factor_starts_low() {
        let t = Toast::new("T", "B", Color32::WHITE);
        assert!(t.slide_factor() <= 0.3);
    }

    #[test]
    fn slide_factor_mid_is_one() {
        let mut t = Toast::new("T", "B", Color32::WHITE);
        t.created = Instant::now() - Duration::from_secs(2);
        assert!((t.slide_factor() - 1.0).abs() < 0.01);
    }

    #[test]
    fn slide_factor_end_approaches_zero() {
        let mut t = Toast::new("T", "B", Color32::WHITE);
        t.created = Instant::now() - Duration::from_millis(3960);
        assert!(t.slide_factor() < 0.3);
    }

    #[test]
    fn progress_increases_over_time() {
        let t = Toast::new("T", "B", Color32::WHITE);
        let p1 = t.progress();
        std::thread::sleep(Duration::from_millis(50));
        let p2 = t.progress();
        assert!(p2 > p1);
    }

    // ── ease_out_cubic ──────────────────────────────────────────────────

    #[test]
    fn ease_out_cubic_zero() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
    }

    #[test]
    fn ease_out_cubic_one() {
        assert!((ease_out_cubic(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn ease_out_cubic_monotonic() {
        let mut prev = 0.0;
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            let v = ease_out_cubic(t);
            assert!(
                v >= prev,
                "ease_out_cubic should be monotonic: {} < {}",
                v,
                prev
            );
            prev = v;
        }
    }

    #[test]
    fn ease_out_cubic_clamps() {
        assert_eq!(ease_out_cubic(-1.0), 0.0);
        assert!((ease_out_cubic(2.0) - 1.0).abs() < 0.001);
    }

    // ── ToastManager ────────────────────────────────────────────────────

    #[test]
    fn manager_visible_respects_max() {
        let mut m = ToastManager::new(2);
        m.push(Toast::new("A", "a", Color32::WHITE));
        m.push(Toast::new("B", "b", Color32::WHITE));
        m.push(Toast::new("C", "c", Color32::WHITE));
        assert_eq!(m.visible().count(), 2);
    }

    #[test]
    fn manager_overflow_count() {
        let mut m = ToastManager::new(2);
        m.push(Toast::new("A", "a", Color32::WHITE));
        m.push(Toast::new("B", "b", Color32::WHITE));
        m.push(Toast::new("C", "c", Color32::WHITE));
        assert_eq!(m.overflow_count(), 1);
    }

    #[test]
    fn manager_tick_removes_expired() {
        let mut m = ToastManager::new(5);
        let mut t = Toast::new("Old", "old", Color32::WHITE);
        t.created = Instant::now() - Duration::from_secs(10);
        m.push(t);
        m.push(Toast::new("New", "new", Color32::WHITE));
        m.tick();
        assert_eq!(m.visible().count(), 1);
    }

    #[test]
    fn manager_is_empty_when_all_expired() {
        let mut m = ToastManager::new(3);
        let mut t = Toast::new("Old", "old", Color32::WHITE);
        t.created = Instant::now() - Duration::from_secs(10);
        m.push(t);
        assert!(m.is_empty());
    }

    #[test]
    fn manager_not_empty_with_active() {
        let mut m = ToastManager::new(3);
        m.push(Toast::new("New", "new", Color32::WHITE));
        assert!(!m.is_empty());
    }
}
