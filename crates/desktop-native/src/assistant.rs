use crate::types::WindowKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssistantIntent {
    OpenApp(WindowKind),
    AskTime,
    SetReminder(String),
    Search(String),
    ToggleDarkMode(bool),
    ToggleWifi(bool),
    ToggleBluetooth(bool),
    BatteryStatus,
    Weather,
    PlaySong(Option<String>),
    Fallback,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssistantMessage {
    pub is_user: bool,
    pub text: String,
}

pub fn suggestion_chips() -> &'static [&'static str] {
    &[
        "Open Files",
        "What time is it?",
        "Search for screenshots",
        "Turn on dark mode",
        "How much battery?",
        "Play Aurora Ambient",
    ]
}

pub fn parse_query(query: &str) -> AssistantIntent {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return AssistantIntent::Fallback;
    }

    let lower = trimmed.to_lowercase();

    if let Some(rest) = lower.strip_prefix("open ") {
        return parse_open_app(rest)
            .map(AssistantIntent::OpenApp)
            .unwrap_or(AssistantIntent::Fallback);
    }

    if lower.contains("what time") || lower.contains("current time") || lower == "time" {
        return AssistantIntent::AskTime;
    }

    if let Some(reminder) = lower
        .strip_prefix("set a reminder for ")
        .or_else(|| lower.strip_prefix("remind me to "))
    {
        let original = trimmed[trimmed.len() - reminder.len()..].trim().to_string();
        if !original.is_empty() {
            return AssistantIntent::SetReminder(original);
        }
    }

    if let Some(rest) = lower.strip_prefix("search for ") {
        let original = trimmed[trimmed.len() - rest.len()..].trim().to_string();
        if !original.is_empty() {
            return AssistantIntent::Search(original);
        }
    }

    if lower.contains("turn on dark mode") || lower.contains("enable dark mode") {
        return AssistantIntent::ToggleDarkMode(true);
    }
    if lower.contains("turn off dark mode") || lower.contains("disable dark mode") {
        return AssistantIntent::ToggleDarkMode(false);
    }
    if lower.contains("turn on wi-fi")
        || lower.contains("turn on wifi")
        || lower.contains("enable wi-fi")
        || lower.contains("enable wifi")
    {
        return AssistantIntent::ToggleWifi(true);
    }
    if lower.contains("turn off wi-fi")
        || lower.contains("turn off wifi")
        || lower.contains("disable wi-fi")
        || lower.contains("disable wifi")
    {
        return AssistantIntent::ToggleWifi(false);
    }
    if lower.contains("turn on bluetooth") || lower.contains("enable bluetooth") {
        return AssistantIntent::ToggleBluetooth(true);
    }
    if lower.contains("turn off bluetooth") || lower.contains("disable bluetooth") {
        return AssistantIntent::ToggleBluetooth(false);
    }
    if lower.contains("how much battery")
        || lower.contains("battery percentage")
        || lower == "battery"
    {
        return AssistantIntent::BatteryStatus;
    }
    if lower.contains("weather") {
        return AssistantIntent::Weather;
    }
    if let Some(rest) = lower.strip_prefix("play ") {
        let original = trimmed[trimmed.len() - rest.len()..].trim().to_string();
        return AssistantIntent::PlaySong(if original.is_empty() {
            None
        } else {
            Some(original)
        });
    }

    AssistantIntent::Fallback
}

fn parse_open_app(name: &str) -> Option<WindowKind> {
    let cleaned = name.trim();
    match cleaned {
        "files" | "finder" => Some(WindowKind::FileManager),
        "terminal" => Some(WindowKind::Terminal),
        "browser" | "safari" => Some(WindowKind::Browser),
        "messages" => Some(WindowKind::Messages),
        "notes" => Some(WindowKind::Notes),
        "music" => Some(WindowKind::MusicPlayer),
        "photos" => Some(WindowKind::Photos),
        "calendar" => Some(WindowKind::Calendar),
        "textedit" | "text edit" | "editor" => Some(WindowKind::TextEditor),
        "settings" | "system settings" => Some(WindowKind::Settings),
        "activity monitor" => Some(WindowKind::ProcessManager),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_open_app_matches_supported_targets() {
        assert_eq!(
            parse_query("Open Files"),
            AssistantIntent::OpenApp(WindowKind::FileManager)
        );
        assert_eq!(
            parse_query("open text edit"),
            AssistantIntent::OpenApp(WindowKind::TextEditor)
        );
    }

    #[test]
    fn parse_toggle_queries_detect_requested_state() {
        assert_eq!(
            parse_query("Turn on dark mode"),
            AssistantIntent::ToggleDarkMode(true)
        );
        assert_eq!(
            parse_query("turn off wifi"),
            AssistantIntent::ToggleWifi(false)
        );
        assert_eq!(
            parse_query("enable bluetooth"),
            AssistantIntent::ToggleBluetooth(true)
        );
    }

    #[test]
    fn parse_search_and_reminder_preserve_payloads() {
        assert_eq!(
            parse_query("Search for meeting notes"),
            AssistantIntent::Search("meeting notes".to_string())
        );
        assert_eq!(
            parse_query("Remind me to submit the build"),
            AssistantIntent::SetReminder("submit the build".to_string())
        );
    }

    #[test]
    fn parse_time_weather_battery_and_play_queries() {
        assert_eq!(parse_query("What time is it?"), AssistantIntent::AskTime);
        assert_eq!(parse_query("What's the weather?"), AssistantIntent::Weather);
        assert_eq!(
            parse_query("How much battery?"),
            AssistantIntent::BatteryStatus
        );
        assert_eq!(
            parse_query("Play Aurora Ambient"),
            AssistantIntent::PlaySong(Some("Aurora Ambient".to_string()))
        );
    }

    #[test]
    fn suggestion_chip_list_is_not_empty() {
        assert!(suggestion_chips().len() >= 4);
    }
}
