use eframe::egui::Color32;
use std::time::Instant;

/// A single notification entry
pub struct AppNotification {
    pub app: String,
    pub title: String,
    pub body: String,
    pub color: Color32,
    pub created: Instant,
    pub read: bool,
}

impl AppNotification {
    pub fn new(
        app: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
        color: Color32,
    ) -> Self {
        Self {
            app: app.into(),
            title: title.into(),
            body: body.into(),
            color,
            created: Instant::now(),
            read: false,
        }
    }

    /// Human-friendly relative time string
    pub fn time_ago(&self) -> String {
        let secs = self.created.elapsed().as_secs();
        if secs < 60 {
            return "Just now".to_string();
        }
        if secs < 3600 {
            return format!("{}m ago", secs / 60);
        }
        if secs < 86400 {
            return format!("{}h ago", secs / 3600);
        }
        if secs < 172800 {
            return "Yesterday".to_string();
        }
        format!("{}d ago", secs / 86400)
    }
}

/// Notification center that holds a queue of notifications
pub struct NotificationCenter {
    notifications: Vec<AppNotification>,
    max_notifications: usize,
}

impl NotificationCenter {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
            max_notifications: 50,
        }
    }

    /// Seed with some demo notifications for first launch
    pub fn seed_defaults(&mut self) {
        if !self.notifications.is_empty() {
            return;
        }
        self.push(AppNotification::new(
            "Mail",
            "New message",
            "Build report: all tests passed",
            Color32::from_rgb(88, 86, 214),
        ));
        self.push(AppNotification::new(
            "Calendar",
            "Standup in 15 min",
            "Daily sync — Conference Room B",
            Color32::from_rgb(255, 59, 48),
        ));
        self.push(AppNotification::new(
            "System",
            "Update available",
            "AuroraOS 0.2.0 is ready to install",
            Color32::from_rgb(52, 199, 89),
        ));
    }

    /// Add a notification
    pub fn push(&mut self, notif: AppNotification) {
        self.notifications.insert(0, notif); // newest first
        if self.notifications.len() > self.max_notifications {
            self.notifications.pop();
        }
    }

    /// Add a simple notification
    pub fn notify(&mut self, app: &str, title: &str, body: &str, color: Color32) {
        self.push(AppNotification::new(app, title, body, color));
    }

    /// Get all notifications (newest first)
    pub fn all(&self) -> &[AppNotification] {
        &self.notifications
    }

    /// Count of unread notifications
    pub fn unread_count(&self) -> usize {
        self.notifications.iter().filter(|n| !n.read).count()
    }

    /// Mark all as read
    pub fn mark_all_read(&mut self) {
        for n in &mut self.notifications {
            n.read = true;
        }
    }

    /// Clear all notifications
    pub fn clear(&mut self) {
        self.notifications.clear();
    }

    pub fn grouped_by_app(&self) -> Vec<(String, Vec<&AppNotification>)> {
        let mut groups: Vec<(String, Vec<&AppNotification>)> = Vec::new();
        for notification in &self.notifications {
            if let Some((_, items)) = groups.iter_mut().find(|(app, _)| app == &notification.app) {
                items.push(notification);
            } else {
                groups.push((notification.app.clone(), vec![notification]));
            }
        }
        groups
    }

    pub fn clear_app(&mut self, app: &str) {
        self.notifications
            .retain(|notification| notification.app != app);
    }

    /// Total count
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.notifications.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notifications.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_center_is_empty() {
        let nc = NotificationCenter::new();
        assert!(nc.is_empty());
        assert_eq!(nc.len(), 0);
        assert_eq!(nc.unread_count(), 0);
    }

    #[test]
    fn push_adds_notification() {
        let mut nc = NotificationCenter::new();
        nc.notify("App", "Title", "Body", Color32::WHITE);
        assert_eq!(nc.len(), 1);
        assert_eq!(nc.all()[0].title, "Title");
    }

    #[test]
    fn newest_first() {
        let mut nc = NotificationCenter::new();
        nc.notify("A", "First", "", Color32::WHITE);
        nc.notify("A", "Second", "", Color32::WHITE);
        assert_eq!(nc.all()[0].title, "Second");
        assert_eq!(nc.all()[1].title, "First");
    }

    #[test]
    fn unread_count_tracks() {
        let mut nc = NotificationCenter::new();
        nc.notify("A", "T1", "", Color32::WHITE);
        nc.notify("A", "T2", "", Color32::WHITE);
        assert_eq!(nc.unread_count(), 2);
    }

    #[test]
    fn mark_all_read() {
        let mut nc = NotificationCenter::new();
        nc.notify("A", "T1", "", Color32::WHITE);
        nc.notify("A", "T2", "", Color32::WHITE);
        nc.mark_all_read();
        assert_eq!(nc.unread_count(), 0);
    }

    #[test]
    fn clear_empties() {
        let mut nc = NotificationCenter::new();
        nc.notify("A", "T", "", Color32::WHITE);
        nc.clear();
        assert!(nc.is_empty());
    }

    #[test]
    fn respects_max_limit() {
        let mut nc = NotificationCenter::new();
        // Push more than max
        for i in 0..60 {
            nc.notify("A", &format!("N{i}"), "", Color32::WHITE);
        }
        assert_eq!(nc.len(), 50);
    }

    #[test]
    fn seed_defaults_adds_items() {
        let mut nc = NotificationCenter::new();
        nc.seed_defaults();
        assert!(nc.len() >= 3);
    }

    #[test]
    fn seed_defaults_idempotent() {
        let mut nc = NotificationCenter::new();
        nc.seed_defaults();
        let count = nc.len();
        nc.seed_defaults();
        assert_eq!(nc.len(), count);
    }

    #[test]
    fn time_ago_just_now() {
        let n = AppNotification::new("A", "T", "B", Color32::WHITE);
        assert_eq!(n.time_ago(), "Just now");
    }

    #[test]
    fn time_ago_minutes() {
        let mut n = AppNotification::new("A", "T", "B", Color32::WHITE);
        n.created = Instant::now() - std::time::Duration::from_secs(300);
        assert_eq!(n.time_ago(), "5m ago");
    }

    #[test]
    fn time_ago_hours() {
        let mut n = AppNotification::new("A", "T", "B", Color32::WHITE);
        n.created = Instant::now() - std::time::Duration::from_secs(7200);
        assert_eq!(n.time_ago(), "2h ago");
    }

    #[test]
    fn notification_stores_fields() {
        let n = AppNotification::new("Mail", "Subject", "Content", Color32::RED);
        assert_eq!(n.app, "Mail");
        assert_eq!(n.title, "Subject");
        assert_eq!(n.body, "Content");
        assert_eq!(n.color, Color32::RED);
        assert!(!n.read);
    }

    #[test]
    fn grouped_by_app_keeps_newest_order_within_groups() {
        let mut nc = NotificationCenter::new();
        nc.notify("Mail", "First", "", Color32::WHITE);
        nc.notify("System", "Second", "", Color32::WHITE);
        nc.notify("Mail", "Third", "", Color32::WHITE);
        let groups = nc.grouped_by_app();
        assert_eq!(groups[0].0, "Mail");
        assert_eq!(groups[0].1.len(), 2);
        assert_eq!(groups[1].0, "System");
    }

    #[test]
    fn clear_app_removes_only_matching_group() {
        let mut nc = NotificationCenter::new();
        nc.notify("Mail", "First", "", Color32::WHITE);
        nc.notify("System", "Second", "", Color32::WHITE);
        nc.clear_app("Mail");
        assert_eq!(nc.len(), 1);
        assert_eq!(nc.all()[0].app, "System");
    }

    #[test]
    fn time_ago_yesterday() {
        let mut n = AppNotification::new("A", "T", "B", Color32::WHITE);
        n.created = Instant::now() - std::time::Duration::from_secs(90_000);
        assert_eq!(n.time_ago(), "Yesterday");
    }
}
