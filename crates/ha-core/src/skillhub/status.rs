use super::types::{RemoteSkillAvailability, RemoteSkillRecord, SkillPublishState};

pub fn derive_publish_state(
    record: Option<&RemoteSkillRecord>,
    availability: RemoteSkillAvailability,
) -> SkillPublishState {
    if !matches!(availability, RemoteSkillAvailability::Reachable) {
        return SkillPublishState::Unknown;
    }
    let Some(record) = record else {
        return SkillPublishState::NotPublished;
    };
    if record.visibility == "shared" && record.approval_status.as_deref() == Some("approved") {
        SkillPublishState::Published
    } else if record.approval_status.as_deref() == Some("pending") {
        SkillPublishState::PendingReview
    } else {
        SkillPublishState::NotPublished
    }
}

pub fn skill_name_slug(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    let mut skipped_non_ascii = false;
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
        if !ch.is_ascii() {
            skipped_non_ascii = true;
        }
    }
    let out = out.trim_matches('-');
    if !skipped_non_ascii {
        return out.to_string();
    }

    let hash = blake3::hash(name.as_bytes()).to_hex().to_string();
    if out.is_empty() {
        format!("skill-{}", &hash[..8])
    } else {
        format!("{}-{}", out, &hash[..8])
    }
}

pub fn registry_slug(username: &str, local_skill_name: &str) -> String {
    let skill_slug = skill_name_slug(local_skill_name);
    if username.trim().is_empty() {
        skill_slug
    } else {
        format!("{username}/{skill_slug}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skillhub::types::{RemoteSkillAvailability, RemoteSkillRecord};

    fn record(visibility: &str, approval_status: Option<&str>) -> RemoteSkillRecord {
        RemoteSkillRecord {
            id: "skill-1".into(),
            registry_slug: "alice/code-review".into(),
            visibility: visibility.into(),
            approval_status: approval_status.map(str::to_string),
            upstream_slug: None,
            name: Some("code-review".into()),
            owner_user_id: Some("user-1".into()),
        }
    }

    #[test]
    fn maps_agentwork_states_to_desktop_states() {
        assert_eq!(
            derive_publish_state(
                Some(&record("shared", Some("approved"))),
                RemoteSkillAvailability::Reachable
            ),
            SkillPublishState::Published
        );
        assert_eq!(
            derive_publish_state(
                Some(&record("private", Some("pending"))),
                RemoteSkillAvailability::Reachable
            ),
            SkillPublishState::PendingReview
        );
        assert_eq!(
            derive_publish_state(
                Some(&record("private", Some("rejected"))),
                RemoteSkillAvailability::Reachable
            ),
            SkillPublishState::NotPublished
        );
        assert_eq!(
            derive_publish_state(None, RemoteSkillAvailability::Reachable),
            SkillPublishState::NotPublished
        );
        assert_eq!(
            derive_publish_state(None, RemoteSkillAvailability::Offline),
            SkillPublishState::Unknown
        );
    }

    #[test]
    fn registry_slug_uses_username_and_normalized_skill_name() {
        assert_eq!(skill_name_slug("Code Review!"), "code-review");
        assert_eq!(registry_slug("alice", "Code Review!"), "alice/code-review");
    }

    #[test]
    fn non_ascii_skill_names_get_stable_ascii_slugs() {
        let slug = skill_name_slug("审计");

        assert!(!slug.is_empty());
        assert_eq!(slug, skill_name_slug("审计"));
        assert!(slug
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'));
    }

    #[test]
    fn non_ascii_characters_prevent_ascii_slug_collisions() {
        assert_ne!(skill_name_slug("Code 审核"), skill_name_slug("Code"));
    }
}
