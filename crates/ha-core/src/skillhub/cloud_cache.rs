use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::types::SkillCloudMetadata;

/// On-disk cache of cloud review/publish metadata for local skills.
///
/// Keyed by `{username}/{local_skill_name}` so account switches do not
/// accidentally reuse another user's remote ids as the current account state.
#[derive(Debug, Clone)]
pub struct CloudMetadataCache {
    path: PathBuf,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudMetadataCacheFile {
    #[serde(default)]
    entries: HashMap<String, SkillCloudMetadata>,
}

impl CloudMetadataCache {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default() -> Result<Self> {
        Ok(Self::new(
            crate::paths::credentials_dir()?.join("skill-cloud-metadata.json"),
        ))
    }

    pub fn cache_key(username: &str, local_skill_name: &str) -> String {
        let user = username.trim();
        if user.is_empty() {
            format!("anonymous/{local_skill_name}")
        } else {
            format!("{user}/{local_skill_name}")
        }
    }

    pub fn load_all(&self) -> Result<HashMap<String, SkillCloudMetadata>> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(HashMap::new());
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("read cloud metadata cache {}", self.path.display()));
            }
        };
        let file: CloudMetadataCacheFile =
            serde_json::from_str(&text).context("parse cloud metadata cache")?;
        Ok(file.entries)
    }

    pub fn get(
        &self,
        username: &str,
        local_skill_name: &str,
    ) -> Result<Option<SkillCloudMetadata>> {
        let key = Self::cache_key(username, local_skill_name);
        Ok(self.load_all()?.get(&key).cloned())
    }

    pub fn upsert(
        &self,
        username: &str,
        local_skill_name: &str,
        metadata: SkillCloudMetadata,
    ) -> Result<()> {
        let key = Self::cache_key(username, local_skill_name);
        let mut entries = self.load_all()?;
        entries.insert(key, metadata);
        self.write_all(&entries)
    }

    pub fn upsert_many(
        &self,
        username: &str,
        updates: impl IntoIterator<Item = (String, SkillCloudMetadata)>,
    ) -> Result<()> {
        let mut entries = self.load_all()?;
        for (local_skill_name, metadata) in updates {
            entries.insert(Self::cache_key(username, &local_skill_name), metadata);
        }
        self.write_all(&entries)
    }

    fn write_all(&self, entries: &HashMap<String, SkillCloudMetadata>) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        let file = CloudMetadataCacheFile {
            entries: entries.clone(),
        };
        let text = serde_json::to_vec_pretty(&file).context("serialize cloud metadata cache")?;
        crate::platform::write_secure_file(&self.path, &text)
            .with_context(|| format!("write cloud metadata cache {}", self.path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skillhub::types::SkillPublishState;
    use tempfile::tempdir;

    fn sample_meta(slug: &str, state: SkillPublishState) -> SkillCloudMetadata {
        SkillCloudMetadata {
            registry_slug: slug.into(),
            remote_skill_id: Some("remote-1".into()),
            upstream_slug: None,
            publish_state: state,
            publish_state_reason: None,
            last_synced_at: Some(1),
            last_error: None,
        }
    }

    #[test]
    fn persists_metadata_per_user_skill() {
        let dir = tempdir().expect("tempdir");
        let cache = CloudMetadataCache::new(dir.path().join("cache.json"));
        cache
            .upsert(
                "alice",
                "code-review",
                sample_meta("alice/code-review", SkillPublishState::PendingReview),
            )
            .expect("upsert");

        let loaded = cache
            .get("alice", "code-review")
            .expect("get")
            .expect("some");
        assert_eq!(loaded.registry_slug, "alice/code-review");
        assert_eq!(loaded.publish_state, SkillPublishState::PendingReview);

        assert!(cache
            .get("bob", "code-review")
            .expect("get other user")
            .is_none());
    }
}
