use std::path::PathBuf;

use anyhow::{Context, Result};

use super::types::{CloudSession, StoredCloudSession};

#[derive(Debug, Clone)]
pub struct CloudSessionStore {
    path: PathBuf,
}

impl CloudSessionStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default() -> Result<Self> {
        Ok(Self::new(
            crate::paths::credentials_dir()?.join("cloud-session.json"),
        ))
    }

    pub fn load(&self) -> Result<Option<StoredCloudSession>> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("read cloud session {}", self.path.display()));
            }
        };
        Ok(Some(
            serde_json::from_str(&text).context("parse cloud session")?,
        ))
    }

    pub fn save(&self, session: &StoredCloudSession) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        let text = serde_json::to_vec_pretty(session).context("serialize cloud session")?;
        crate::platform::write_secure_file(&self.path, &text)
            .with_context(|| format!("write cloud session {}", self.path.display()))?;
        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("remove cloud session {}", self.path.display()));
            }
        }
        Ok(())
    }
}

pub fn public_session(session: StoredCloudSession) -> CloudSession {
    CloudSession {
        server_url: session.server_url,
        user: session.user,
        authenticated: true,
    }
}

pub fn get_cloud_session() -> Result<Option<CloudSession>> {
    let store = CloudSessionStore::default()?;
    Ok(store.load()?.map(public_session))
}

pub fn save_cloud_session(session: &StoredCloudSession) -> Result<CloudSession> {
    let store = CloudSessionStore::default()?;
    store.save(session)?;
    Ok(public_session(session.clone()))
}

pub fn clear_cloud_session() -> Result<()> {
    CloudSessionStore::default()?.clear()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skillhub::types::{CloudLoginRequest, CloudUser};
    use tempfile::tempdir;

    #[test]
    fn persists_and_clears_cloud_session() {
        let dir = tempdir().expect("tempdir");
        let store = CloudSessionStore::new(dir.path().join("cloud-session.json"));
        let saved = StoredCloudSession {
            server_url: "http://127.0.0.1:3000".into(),
            user: CloudUser {
                id: "u1".into(),
                username: "alice".into(),
                display_name: Some("Alice".into()),
            },
            access_token: "access".into(),
            refresh_token: Some("refresh".into()),
        };

        store.save(&saved).expect("save");
        let loaded = store.load().expect("load").expect("some session");
        assert_eq!(loaded.user.username, "alice");
        assert_eq!(loaded.access_token, "access");

        store.clear().expect("clear");
        assert!(store.load().expect("load after clear").is_none());
    }

    #[test]
    fn missing_cloud_session_can_be_loaded_and_cleared() {
        let dir = tempdir().expect("tempdir");
        let store = CloudSessionStore::new(dir.path().join("cloud-session.json"));

        assert!(store.load().expect("load missing session").is_none());
        store.clear().expect("clear missing session");
    }

    #[test]
    fn projects_stored_session_without_tokens() {
        let session = StoredCloudSession {
            server_url: "https://cloud.example".into(),
            user: CloudUser {
                id: "u1".into(),
                username: "alice".into(),
                display_name: None,
            },
            access_token: "secret".into(),
            refresh_token: Some("refresh-secret".into()),
        };

        let public = public_session(session);
        assert_eq!(public.server_url, "https://cloud.example");
        assert_eq!(public.user.username, "alice");
        assert!(public.authenticated);
    }

    #[test]
    fn login_types_use_camel_case_json_fields() {
        let request = CloudLoginRequest {
            server_url: "https://cloud.example".into(),
            username: "alice".into(),
            password: "secret".into(),
        };
        let json = serde_json::to_value(request).expect("serialize login request");
        assert_eq!(json["serverUrl"], "https://cloud.example");
        assert_eq!(json["username"], "alice");
    }

    #[cfg(unix)]
    #[test]
    fn saves_cloud_session_with_private_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("cloud-session.json");
        let store = CloudSessionStore::new(path.clone());
        let saved = StoredCloudSession {
            server_url: "https://cloud.example".into(),
            user: CloudUser {
                id: "u1".into(),
                username: "alice".into(),
                display_name: None,
            },
            access_token: "secret".into(),
            refresh_token: None,
        };

        store.save(&saved).expect("save");

        assert_eq!(
            std::fs::metadata(path)
                .expect("metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
