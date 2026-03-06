use std::time::{Duration, Instant};
use eframe::egui::Color32;

pub struct Toast {
    pub title: String,
    pub body: String,
    pub color: Color32,
    pub created: Instant,
    pub duration: Duration,
}

impl Toast {
    pub fn new(title: impl Into<String>, body: impl Into<String>, color: Color32) -> Self {
        Self { title: title.into(), body: body.into(), color, created: Instant::now(), duration: Duration::from_secs(4) }
    }

    pub fn progress(&self) -> f32 {
        self.created.elapsed().as_secs_f32() / self.duration.as_secs_f32()
    }

    pub fn is_expired(&self) -> bool {
        self.created.elapsed() > self.duration
    }

    /// Compute slide-in/out factor (0.0 = off-screen, 1.0 = fully visible)
    #[allow(dead_code)]
    pub fn slide_factor(&self) -> f32 {
        let p = self.progress();
        if p < 0.1 {
            p / 0.1
        } else if p > 0.85 {
            (1.0 - p) / 0.15
        } else {
            1.0
        }
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
    fn slide_factor_starts_at_zero() {
        let mut t = Toast::new("T", "B", Color32::WHITE);
        // At t=0, progress=0, slide = 0/0.1 = 0
        t.created = Instant::now();
        assert!(t.slide_factor() <= 0.1);
    }

    #[test]
    fn slide_factor_mid_is_one() {
        let mut t = Toast::new("T", "B", Color32::WHITE);
        // At progress=0.5, slide should be 1.0
        t.created = Instant::now() - Duration::from_secs(2); // 2/4 = 0.5
        assert!((t.slide_factor() - 1.0).abs() < 0.01);
    }

    #[test]
    fn slide_factor_end_approaches_zero() {
        let mut t = Toast::new("T", "B", Color32::WHITE);
        // At progress=0.99, slide should be near 0
        t.created = Instant::now() - Duration::from_millis(3960); // 3.96/4 = 0.99
        assert!(t.slide_factor() < 0.2);
    }

    #[test]
    fn progress_increases_over_time() {
        let t = Toast::new("T", "B", Color32::WHITE);
        let p1 = t.progress();
        std::thread::sleep(Duration::from_millis(50));
        let p2 = t.progress();
        assert!(p2 > p1);
    }
}
