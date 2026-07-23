//! Backend-owned locale resolution and small-message localization.
//!
//! `AppConfig.language` is the product/UI language preference. It is separate
//! from `UserConfig.language`, which only tells the model how the user prefers
//! assistant replies. Backend-generated system messages must use this module
//! rather than reading profile language.

use crate::config::AppConfig;

pub const DEFAULT_LOCALE: &str = "en";

/// Locale order shared by backend translation tables.
pub const SUPPORTED_LOCALES: [&str; 3] = [
    "zh", "zh-TW", "en",
];

/// Normalize a raw locale string to one of the backend-supported locale codes.
///
/// Empty strings, `"auto"`, and unsupported locales return `None`; use
/// [`locale_from_preference`] when a configured non-auto preference should
/// fail open to English.
pub fn normalize_locale(raw: &str) -> Option<&'static str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        return None;
    }

    let lower = trimmed.replace('_', "-").to_ascii_lowercase();
    if lower == "zh"
        || lower.starts_with("zh-cn")
        || lower.starts_with("zh-sg")
        || lower.starts_with("zh-hans")
    {
        return Some("zh");
    }
    if lower == "zh-tw"
        || lower.starts_with("zh-tw-")
        || lower.starts_with("zh-hk")
        || lower.starts_with("zh-mo")
        || lower.starts_with("zh-hant")
    {
        return Some("zh-TW");
    }

    SUPPORTED_LOCALES.iter().copied().find(|&locale| {
        if matches!(locale, "zh" | "zh-TW") {
            return false;
        }
        let l = locale.to_ascii_lowercase();
        lower == l || lower.starts_with(&(l + "-"))
    })
}

/// Resolve a persisted preference field.
///
/// `None` means "no explicit preference; continue to the next fallback".
/// Unsupported explicit preferences become English so backend messages never
/// claim one language while falling through to another source.
pub fn locale_from_preference(raw: &str) -> Option<&'static str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        None
    } else {
        Some(normalize_locale(trimmed).unwrap_or(DEFAULT_LOCALE))
    }
}

fn system_locale() -> &'static str {
    normalize_locale(&crate::agent_loader::detect_system_locale()).unwrap_or(DEFAULT_LOCALE)
}

/// Resolve an output locale with one optional subsystem override, then the
/// global UI language, then the host system locale.
pub fn effective_locale(subsystem_language: Option<&str>, app_language: &str) -> &'static str {
    subsystem_language
        .and_then(locale_from_preference)
        .or_else(|| locale_from_preference(app_language))
        .unwrap_or_else(system_locale)
}

/// Resolve the backend-visible UI locale from `AppConfig.language`.
pub fn effective_ui_locale(config: &AppConfig) -> &'static str {
    locale_from_preference(&config.language).unwrap_or_else(system_locale)
}

/// Resolve the current process-wide UI locale from cached app config.
pub fn current_ui_locale() -> &'static str {
    let config = crate::config::cached_config();
    effective_ui_locale(&config)
}

//// Pick a localized string from a row ordered like [`SUPPORTED_LOCALES`].
pub fn pick_locale(locale: &str, row: [&'static str; 3]) -> &'static str {
    let locale = normalize_locale(locale).unwrap_or(DEFAULT_LOCALE);
    let idx = SUPPORTED_LOCALES
        .iter()
        .position(|&candidate| candidate == locale)
        .unwrap_or(2);
    row[idx]
}

/// Human-readable language name with native-script hint.
pub fn language_name(locale: &str) -> &'static str {
    match normalize_locale(locale).unwrap_or(DEFAULT_LOCALE) {
        "zh" => "Simplified Chinese (简体中文)",
        "zh-TW" => "Traditional Chinese (繁體中文)",
        _ => "English",
    }
}

/// Small backend-owned messages that may be rendered outside the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendMessage {
    StartupBackOnline,
    ChannelSessionEvicted,
}

impl BackendMessage {
    /// Stable frontend/i18next key for UI surfaces that prefer late rendering.
    pub fn key(self) -> &'static str {
        match self {
            Self::StartupBackOnline => "backend.startup.backOnline",
            Self::ChannelSessionEvicted => "backend.channel.sessionEvicted",
        }
    }
}

/// Render a backend-owned message in the requested locale.
pub fn localized_backend_message(message: BackendMessage, locale: &str) -> &'static str {
    let locale = normalize_locale(locale).unwrap_or(DEFAULT_LOCALE);
    match message {
        BackendMessage::StartupBackOnline => match locale {
            "zh" => "📡 TPA CoWork 已恢复在线。如果你正在等回复，请重新发送上一条消息。",
            "zh-TW" => "📡 TPA CoWork 已恢復連線。如果你正在等回覆，請重新傳送上一則訊息。",
            _ => "📡 TPA CoWork is back online. If you were waiting on a reply, send your last message again.",
        },
        BackendMessage::ChannelSessionEvicted => match locale {
            "zh" => "📢 这个聊天已被另一个入口接管。你已离开之前的会话；发送新消息即可开始新会话。",
            "zh-TW" => "📢 這個聊天已被另一個入口接管。你已離開先前的會話；傳送新訊息即可開始新會話。",
            _ => "📢 This chat has been taken over by another endpoint. You've left the previous session; send a new message to start a fresh one.",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_locale_handles_supported_aliases() {
        assert_eq!(normalize_locale("zh-CN"), Some("zh"));
        assert_eq!(normalize_locale("zh_Hant"), Some("zh-TW"));
        assert_eq!(normalize_locale("ZH"), Some("zh"));
        assert_eq!(normalize_locale("auto"), None);
        assert_eq!(normalize_locale("de"), None);
    }

    #[test]
    fn configured_unsupported_locale_fails_open_to_english() {
        assert_eq!(locale_from_preference("de"), Some("en"));
        assert_eq!(effective_locale(Some("de"), "zh"), "en");
        assert_eq!(effective_locale(Some("auto"), "zh-TW"), "zh-TW");
    }

    #[test]
    fn every_backend_message_has_supported_locale_text() {
        for message in [
            BackendMessage::StartupBackOnline,
            BackendMessage::ChannelSessionEvicted,
        ] {
            for locale in SUPPORTED_LOCALES {
                let text = localized_backend_message(message, locale);
                assert!(!text.trim().is_empty(), "{message:?} {locale}");
            }
        }
    }
}
