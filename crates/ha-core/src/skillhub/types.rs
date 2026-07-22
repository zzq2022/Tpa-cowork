use serde::{Deserialize, Serialize};

/// Default AgentWork / SkillHub base URL for TPA CoWork local/dev deployments.
///
/// Resolution order for public catalog calls:
/// 1. saved cloud session `server_url`
/// 2. `TPA_SKILLHUB_URL` env (preferred)
/// 3. legacy `HOPE_SKILLHUB_URL` env
/// 4. this constant
pub const DEFAULT_SKILLHUB_SERVER_URL: &str = "http://127.0.0.1:3000";

/// Resolve the SkillHub server URL with session → env → default fallback.
pub fn resolve_skillhub_server_url(session_url: Option<&str>) -> String {
    if let Some(url) = session_url.map(str::trim).filter(|u| !u.is_empty()) {
        return url.to_string();
    }
    if let Ok(url) = std::env::var("TPA_SKILLHUB_URL") {
        let url = url.trim();
        if !url.is_empty() {
            return url.to_string();
        }
    }
    if let Ok(url) = std::env::var("HOPE_SKILLHUB_URL") {
        let url = url.trim();
        if !url.is_empty() {
            return url.to_string();
        }
    }
    DEFAULT_SKILLHUB_SERVER_URL.to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillPublishState {
    Published,
    PendingReview,
    NotPublished,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteSkillAvailability {
    Reachable,
    Offline,
    AuthFailed,
    Unconfigured,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudUser {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSession {
    pub server_url: String,
    pub user: CloudUser,
    pub authenticated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudLoginRequest {
    pub server_url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudLoginResponse {
    pub user: CloudUser,
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredCloudSession {
    pub server_url: String,
    pub user: CloudUser,
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSkillRecord {
    pub id: String,
    pub registry_slug: String,
    pub visibility: String,
    pub approval_status: Option<String>,
    pub upstream_slug: Option<String>,
    /// Optional skill display/name from private list payloads.
    #[serde(default)]
    pub name: Option<String>,
    /// Optional owner user id for userId + skill matching.
    #[serde(default)]
    pub owner_user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillCloudMetadata {
    pub registry_slug: String,
    pub remote_skill_id: Option<String>,
    pub upstream_slug: Option<String>,
    pub publish_state: SkillPublishState,
    pub publish_state_reason: Option<String>,
    pub last_synced_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MySkillCloudEntry {
    pub local: crate::skills::SkillSummary,
    pub cloud: SkillCloudMetadata,
}

pub fn deserialize_optional_timestamp_or_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct OptionalTimestampOrStringVisitor;

    impl<'de> Visitor<'de> for OptionalTimestampOrStringVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string, an integer timestamp, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(self)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_string()))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.timestamp_to_rfc3339(value)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.timestamp_to_rfc3339(value as i64)
        }
    }

    impl OptionalTimestampOrStringVisitor {
        fn timestamp_to_rfc3339<E>(&self, value: i64) -> Result<Option<String>, E>
        where
            E: de::Error,
        {
            use chrono::{TimeZone, Utc};
            let dt = if value > 9_999_999_999 {
                let secs = value / 1000;
                let nsecs = (value % 1000) * 1_000_000;
                Utc.timestamp_opt(secs, nsecs as u32)
            } else {
                Utc.timestamp_opt(value, 0)
            };
            match dt {
                chrono::LocalResult::Single(t) => {
                    Ok(Some(t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)))
                }
                _ => Err(de::Error::custom(format!(
                    "invalid timestamp value: {}",
                    value
                ))),
            }
        }
    }

    deserializer.deserialize_option(OptionalTimestampOrStringVisitor)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillHubPublicSkill {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(alias = "slug")]
    pub registry_slug: String,
    #[serde(alias = "authorName")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_username: Option<String>,
    #[serde(alias = "downloadCount")]
    pub downloads: u64,
    #[serde(alias = "starCount")]
    pub stars: u64,
    #[serde(default, alias = "viewCount")]
    pub views: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(alias = "updatedAt")]
    #[serde(default, deserialize_with = "deserialize_optional_timestamp_or_string")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillHubPublicSkillsPage {
    pub items: Vec<SkillHubPublicSkill>,
    pub total: u64,
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillHubSearchRequest {
    #[serde(rename = "q", alias = "query")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDetail {
    pub id: String,
    pub name: String,
    pub description: String,
    pub visibility: String,
    pub owner_user_id: Option<String>,
    pub skill_md: Option<String>,
    pub skill_md_available: bool,
    pub slug: Option<String>,
    pub approval_status: Option<String>,
    pub author_name: Option<String>,
    pub category: Option<String>,
    pub star_count: u64,
    pub download_count: u64,
    pub view_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitSkillReviewRequest {
    pub file_name: String,
    pub package_base64: String,
    #[serde(default, skip_serializing)]
    pub local_skill_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_skill_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_skillhub_url_is_local_agentwork() {
        assert_eq!(DEFAULT_SKILLHUB_SERVER_URL, "http://127.0.0.1:3000");
    }

    #[test]
    fn resolve_prefers_session_url() {
        assert_eq!(
            resolve_skillhub_server_url(Some("https://cloud.example")),
            "https://cloud.example"
        );
    }

    #[test]
    fn resolve_falls_back_to_default_without_session() {
        // Clear env overrides for the assertion; ignore if unset.
        std::env::remove_var("TPA_SKILLHUB_URL");
        std::env::remove_var("HOPE_SKILLHUB_URL");
        assert_eq!(
            resolve_skillhub_server_url(None),
            DEFAULT_SKILLHUB_SERVER_URL
        );
        assert_eq!(
            resolve_skillhub_server_url(Some("")),
            DEFAULT_SKILLHUB_SERVER_URL
        );
        assert_eq!(
            resolve_skillhub_server_url(Some("   ")),
            DEFAULT_SKILLHUB_SERVER_URL
        );
    }
}
