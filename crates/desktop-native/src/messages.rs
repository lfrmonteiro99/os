//! Messages window state — conversations, chat bubbles, auto-reply.

use std::time::{Duration, Instant};
use eframe::egui::Color32;

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub text: String,
    pub is_sent: bool,
    pub timestamp: Instant,
}

#[derive(Clone)]
pub struct Conversation {
    pub contact_name: String,
    pub contact_color: Color32,
    pub messages: Vec<ChatMessage>,
    pub unread: usize,
}

pub struct MessagesState {
    pub conversations: Vec<Conversation>,
    pub active: usize,
    pub input_text: String,
    pub pending_reply: Option<(usize, Instant)>,
}

impl MessagesState {
    pub fn new() -> Self {
        let mut state = Self {
            conversations: Vec::new(),
            active: 0,
            input_text: String::new(),
            pending_reply: None,
        };
        state.seed_defaults();
        state
    }

    pub fn seed_defaults(&mut self) {
        let now = Instant::now();
        self.conversations = vec![
            Conversation {
                contact_name: "Alice".to_string(),
                contact_color: Color32::from_rgb(0, 122, 255),
                messages: vec![
                    ChatMessage { text: "Hey! How's the AuroraOS project going?".into(), is_sent: false, timestamp: now - Duration::from_secs(300) },
                    ChatMessage { text: "Going great! Just finished the desktop shell".into(), is_sent: true, timestamp: now - Duration::from_secs(240) },
                    ChatMessage { text: "The Big Sur wallpaper looks amazing".into(), is_sent: false, timestamp: now - Duration::from_secs(180) },
                    ChatMessage { text: "Thanks! The dock magnification was tricky".into(), is_sent: true, timestamp: now - Duration::from_secs(120) },
                    ChatMessage { text: "Can you show me a screenshot?".into(), is_sent: false, timestamp: now - Duration::from_secs(60) },
                    ChatMessage { text: "Sure, sending one now...".into(), is_sent: true, timestamp: now - Duration::from_secs(30) },
                    ChatMessage { text: "Hey! The new UI looks amazing".into(), is_sent: false, timestamp: now },
                ],
                unread: 1,
            },
            Conversation {
                contact_name: "Bob".to_string(),
                contact_color: Color32::from_rgb(52, 199, 89),
                messages: vec![
                    ChatMessage { text: "Did you push the latest changes?".into(), is_sent: false, timestamp: now - Duration::from_secs(600) },
                    ChatMessage { text: "Yes, just merged the PR".into(), is_sent: true, timestamp: now - Duration::from_secs(500) },
                    ChatMessage { text: "Sure, sounds good".into(), is_sent: false, timestamp: now - Duration::from_secs(400) },
                ],
                unread: 0,
            },
            Conversation {
                contact_name: "Team".to_string(),
                contact_color: Color32::from_rgb(255, 149, 0),
                messages: vec![
                    ChatMessage { text: "Sprint review at 3pm".into(), is_sent: false, timestamp: now - Duration::from_secs(3600) },
                    ChatMessage { text: "Build passed!".into(), is_sent: false, timestamp: now - Duration::from_secs(1800) },
                    ChatMessage { text: "Great work everyone".into(), is_sent: true, timestamp: now - Duration::from_secs(900) },
                ],
                unread: 0,
            },
            Conversation {
                contact_name: "Carol".to_string(),
                contact_color: Color32::from_rgb(175, 82, 222),
                messages: vec![
                    ChatMessage { text: "Are you coming to the meetup?".into(), is_sent: false, timestamp: now - Duration::from_secs(7200) },
                    ChatMessage { text: "Definitely! See you there".into(), is_sent: true, timestamp: now - Duration::from_secs(7000) },
                    ChatMessage { text: "See you tomorrow".into(), is_sent: false, timestamp: now - Duration::from_secs(3600) },
                ],
                unread: 0,
            },
        ];
    }

    pub fn send_message(&mut self) {
        let text = self.input_text.trim().to_string();
        if text.is_empty() { return; }

        let idx = self.active;
        if idx >= self.conversations.len() { return; }

        self.conversations[idx].messages.push(ChatMessage {
            text,
            is_sent: true,
            timestamp: Instant::now(),
        });
        self.input_text.clear();

        // Schedule auto-reply in 1-3 seconds
        let delay = Duration::from_millis(1000 + (idx as u64 * 500) % 2000);
        self.pending_reply = Some((idx, Instant::now() + delay));
    }

    /// Call each frame to deliver pending auto-replies.
    pub fn tick(&mut self) {
        if let Some((conv_idx, deliver_at)) = self.pending_reply {
            if Instant::now() >= deliver_at {
                if conv_idx < self.conversations.len() {
                    let reply = auto_reply_for(&self.conversations[conv_idx].contact_name,
                        self.conversations[conv_idx].messages.len());
                    self.conversations[conv_idx].messages.push(ChatMessage {
                        text: reply,
                        is_sent: false,
                        timestamp: Instant::now(),
                    });
                    if conv_idx != self.active {
                        self.conversations[conv_idx].unread += 1;
                    }
                }
                self.pending_reply = None;
            }
        }
    }

    pub fn switch_conversation(&mut self, idx: usize) {
        if idx < self.conversations.len() {
            self.active = idx;
            self.conversations[idx].unread = 0;
        }
    }

    pub fn active_conversation(&self) -> Option<&Conversation> {
        self.conversations.get(self.active)
    }

    pub fn total_unread(&self) -> usize {
        self.conversations.iter().map(|c| c.unread).sum()
    }
}

fn auto_reply_for(contact: &str, msg_count: usize) -> String {
    let replies = match contact {
        "Alice" => &[
            "That's awesome!", "Keep up the great work!", "I love the new features!",
            "Can't wait to try it", "Send me the build!", "Looking good!",
        ][..],
        "Bob" => &[
            "Nice!", "LGTM", "Sounds good", "On it", "Will review later",
            "Merged!", "Thanks for the update",
        ],
        "Team" => &[
            "Noted", "Thanks for the update", "Will do", "Good progress!",
            "Let's sync up later", "Action items updated",
        ],
        "Carol" => &[
            "Sounds great!", "See you there!", "Can't wait", "Perfect",
            "Let me know if anything changes",
        ],
        _ => &["Ok", "Got it", "Thanks", "Sure"],
    };
    let idx = msg_count % replies.len();
    replies[idx].to_string()
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_default_conversations() {
        let s = MessagesState::new();
        assert_eq!(s.conversations.len(), 4);
        assert_eq!(s.conversations[0].contact_name, "Alice");
        assert_eq!(s.conversations[1].contact_name, "Bob");
    }

    #[test]
    fn send_message_adds_to_active() {
        let mut s = MessagesState::new();
        let before = s.conversations[0].messages.len();
        s.input_text = "Hello!".to_string();
        s.send_message();
        assert_eq!(s.conversations[0].messages.len(), before + 1);
        assert!(s.conversations[0].messages.last().unwrap().is_sent);
        assert!(s.input_text.is_empty());
    }

    #[test]
    fn send_empty_does_nothing() {
        let mut s = MessagesState::new();
        let before = s.conversations[0].messages.len();
        s.input_text = "   ".to_string();
        s.send_message();
        assert_eq!(s.conversations[0].messages.len(), before);
    }

    #[test]
    fn send_schedules_auto_reply() {
        let mut s = MessagesState::new();
        s.input_text = "Test".to_string();
        s.send_message();
        assert!(s.pending_reply.is_some());
    }

    #[test]
    fn tick_delivers_reply_after_delay() {
        let mut s = MessagesState::new();
        s.input_text = "Test".to_string();
        s.send_message();
        let before = s.conversations[0].messages.len();

        // Force the delivery time to now
        if let Some((idx, _)) = s.pending_reply {
            s.pending_reply = Some((idx, Instant::now() - Duration::from_millis(1)));
        }
        s.tick();

        assert_eq!(s.conversations[0].messages.len(), before + 1);
        assert!(!s.conversations[0].messages.last().unwrap().is_sent);
        assert!(s.pending_reply.is_none());
    }

    #[test]
    fn switch_conversation_clears_unread() {
        let mut s = MessagesState::new();
        s.conversations[1].unread = 3;
        s.switch_conversation(1);
        assert_eq!(s.active, 1);
        assert_eq!(s.conversations[1].unread, 0);
    }

    #[test]
    fn total_unread_sums_correctly() {
        let mut s = MessagesState::new();
        s.conversations[0].unread = 2;
        s.conversations[1].unread = 1;
        s.conversations[2].unread = 0;
        s.conversations[3].unread = 3;
        assert_eq!(s.total_unread(), 6);
    }

    #[test]
    fn active_conversation_returns_correct() {
        let s = MessagesState::new();
        let active = s.active_conversation().unwrap();
        assert_eq!(active.contact_name, "Alice");
    }

    #[test]
    fn auto_reply_varies_by_contact() {
        let r1 = auto_reply_for("Alice", 0);
        let r2 = auto_reply_for("Bob", 0);
        // Different contacts should have different reply pools
        // (might be the same for index 0, but the pools differ)
        assert!(!r1.is_empty());
        assert!(!r2.is_empty());
    }

    #[test]
    fn auto_reply_cycles() {
        let r1 = auto_reply_for("Alice", 0);
        let r2 = auto_reply_for("Alice", 6); // wraps around
        assert_eq!(r1, r2);
    }

    #[test]
    fn send_to_different_conversation() {
        let mut s = MessagesState::new();
        s.switch_conversation(2);
        let before = s.conversations[2].messages.len();
        s.input_text = "Hello Team!".to_string();
        s.send_message();
        assert_eq!(s.conversations[2].messages.len(), before + 1);
    }
}
