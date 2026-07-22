use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Url;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use super::status::registry_slug;
use super::types::{
    CloudLoginRequest, CloudLoginResponse, CloudSession, RemoteSkillRecord, SkillDetail,
    SkillHubPublicSkill, SkillHubPublicSkillsPage, SkillHubSearchRequest, StoredCloudSession,
    SubmitSkillReviewRequest,
};
use super::{public_session, CloudSessionStore};

#[async_trait]
pub trait RemoteSkillHubClient: Send + Sync {
    async fn login(&self, request: CloudLoginRequest) -> Result<StoredCloudSession>;
    async fn refresh(&self, session: &StoredCloudSession) -> Result<StoredCloudSession>;
    async fn list_private_skills(
        &self,
        session: &StoredCloudSession,
    ) -> Result<Vec<RemoteSkillRecord>>;
    async fn search_public_skills(
        &self,
        request: SkillHubSearchRequest,
    ) -> Result<SkillHubPublicSkillsPage>;
    async fn get_public_skill_detail(
        &self,
        skill_id: &str,
        session: Option<&StoredCloudSession>,
    ) -> Result<SkillDetail>;
    async fn submit_review(
        &self,
        session: &StoredCloudSession,
        request: SubmitSkillReviewRequest,
        zip_base64: String,
    ) -> Result<RemoteSkillRecord>;
    async fn download_skill(
        &self,
        session: Option<&StoredCloudSession>,
        skill_id: &str,
    ) -> Result<Vec<u8>>;
}

#[derive(Clone)]
pub struct AgentWorkSkillHubClient {
    base_url: Url,
    http: reqwest::Client,
}

impl AgentWorkSkillHubClient {
    pub fn new(base_url: String) -> Result<Self> {
        let base_url =
            Url::parse(base_url.trim_end_matches('/')).context("parse AgentWork server URL")?;
        Ok(Self {
            base_url,
            http: reqwest::Client::new(),
        })
    }

    pub fn endpoint(&self, path: &str) -> Url {
        self.base_url
            .join(path.trim_start_matches('/'))
            .expect("valid endpoint path")
    }

    async fn json_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T> {
        let status = response.status();
        let body = response.text().await.context("read AgentWork response")?;
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(anyhow!("cloud authentication failed"));
        }
        if !status.is_success() {
            return Err(anyhow!("AgentWork request failed with {}", status));
        }
        let payload: Value = serde_json::from_str(&body).context("parse AgentWork response")?;
        let data = payload.get("data").cloned().unwrap_or(payload);
        serde_json::from_value(data).context("decode AgentWork response data")
    }

    async fn authed(
        &self,
        session: &StoredCloudSession,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response> {
        request
            .bearer_auth(&session.access_token)
            .send()
            .await
            .context("AgentWork request")
    }

    fn public_search_request(&self, request: SkillHubSearchRequest) -> Result<reqwest::Request> {
        let path = if request.query.is_some() {
            "/api/skillhub/public/search"
        } else {
            "/api/skillhub/public"
        };
        let mut builder = self.http.get(self.endpoint(path));
        if let Some(query) = &request.query {
            builder = builder.query(&[("q", query)]);
        }
        if let Some(limit) = request.limit {
            builder = builder.query(&[("limit", limit)]);
        }
        if let Some(page) = request.page {
            builder = builder.query(&[("page", page)]);
        }
        if let Some(sort) = &request.sort {
            builder = builder.query(&[("sort", sort)]);
        }
        if let Some(category) = &request.category {
            builder = builder.query(&[("category", category)]);
        }
        builder
            .build()
            .context("build AgentWork public search request")
    }
}

fn remote_record_from_submission(value: Value) -> Result<RemoteSkillRecord> {
    remote_record_from_value(value, None)
}

fn remote_record_from_private_list(
    value: Value,
    session: &StoredCloudSession,
) -> Result<RemoteSkillRecord> {
    let fallback_slug = value
        .get("name")
        .and_then(Value::as_str)
        .map(|name| registry_slug(&session.user.username, name));
    remote_record_from_value(value, fallback_slug)
}

fn remote_record_from_value(
    value: Value,
    fallback_registry_slug: Option<String>,
) -> Result<RemoteSkillRecord> {
    Ok(RemoteSkillRecord {
        id: value
            .get("id")
            .or_else(|| value.get("skillId"))
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("review response missing skill id"))?
            .to_string(),
        registry_slug: value
            .get("registrySlug")
            .or_else(|| value.get("registry_slug"))
            .or_else(|| value.get("slug"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or(fallback_registry_slug)
            .ok_or_else(|| anyhow!("review response missing registry slug"))?,
        visibility: value
            .get("visibility")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("review response missing visibility"))?
            .to_string(),
        approval_status: value
            .get("approvalStatus")
            .or_else(|| value.get("approval_status"))
            .and_then(Value::as_str)
            .map(str::to_string),
        upstream_slug: value
            .get("upstreamSlug")
            .or_else(|| value.get("upstream_slug"))
            .and_then(Value::as_str)
            .map(str::to_string),
        name: value
            .get("name")
            .or_else(|| value.get("skillName"))
            .or_else(|| value.get("skill_name"))
            .and_then(Value::as_str)
            .map(str::to_string),
        owner_user_id: value
            .get("ownerUserId")
            .or_else(|| value.get("owner_user_id"))
            .or_else(|| value.get("userId"))
            .or_else(|| value.get("user_id"))
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

#[async_trait]
impl RemoteSkillHubClient for AgentWorkSkillHubClient {
    async fn login(&self, request: CloudLoginRequest) -> Result<StoredCloudSession> {
        let response = self
            .http
            .post(self.endpoint("/api/auth/login"))
            .json(&json!({
                "username": request.username,
                "password": request.password,
            }))
            .send()
            .await
            .context("AgentWork login request")?;
        let data: CloudLoginResponse = self.json_response(response).await?;
        Ok(StoredCloudSession {
            server_url: request.server_url,
            user: data.user,
            access_token: data.access_token,
            refresh_token: data.refresh_token,
        })
    }

    async fn refresh(&self, session: &StoredCloudSession) -> Result<StoredCloudSession> {
        let refresh_token = session
            .refresh_token
            .as_deref()
            .ok_or_else(|| anyhow!("cloud refresh token unavailable"))?;
        let response = self
            .http
            .post(self.endpoint("/api/auth/refresh"))
            .json(&json!({ "refreshToken": refresh_token }))
            .send()
            .await
            .context("AgentWork refresh request")?;
        let data: CloudLoginResponse = self.json_response(response).await?;
        Ok(StoredCloudSession {
            server_url: session.server_url.clone(),
            user: data.user,
            access_token: data.access_token,
            refresh_token: data.refresh_token,
        })
    }

    async fn list_private_skills(
        &self,
        session: &StoredCloudSession,
    ) -> Result<Vec<RemoteSkillRecord>> {
        let response = self
            .authed(
                session,
                self.http.get(self.endpoint("/api/skillhub/private")),
            )
            .await?;
        let values: Vec<Value> = self.json_response(response).await?;
        values
            .into_iter()
            .map(|value| remote_record_from_private_list(value, session))
            .collect()
    }

    async fn search_public_skills(
        &self,
        request: SkillHubSearchRequest,
    ) -> Result<SkillHubPublicSkillsPage> {
        let response = self
            .http
            .execute(self.public_search_request(request)?)
            .await
            .context("AgentWork public search request")?;
        let data: Value = self.json_response(response).await?;
        if data.get("items").is_some() {
            let page: SkillHubPublicSkillsPage =
                serde_json::from_value(data).context("decode public search page")?;
            Ok(page)
        } else {
            let items: Vec<SkillHubPublicSkill> =
                serde_json::from_value(data).context("decode public search skills array")?;
            let total = items.len() as u64;
            let page_size = items.len();
            Ok(SkillHubPublicSkillsPage {
                items,
                total,
                page_size,
            })
        }
    }

    async fn get_public_skill_detail(
        &self,
        skill_id: &str,
        session: Option<&StoredCloudSession>,
    ) -> Result<SkillDetail> {
        let request = self
            .http
            .get(self.endpoint(&format!("/api/skillhub/public/{}", skill_id)));
        let response = match session {
            Some(session) => self.authed(session, request).await?,
            None => request
                .send()
                .await
                .context("AgentWork public detail request")?,
        };
        self.json_response(response).await
    }

    async fn submit_review(
        &self,
        session: &StoredCloudSession,
        mut request: SubmitSkillReviewRequest,
        zip_base64: String,
    ) -> Result<RemoteSkillRecord> {
        request.package_base64 = zip_base64;
        let response = self
            .authed(
                session,
                self.http
                    .post(self.endpoint("/api/skillhub/review-submissions"))
                    .json(&request),
            )
            .await?;
        let data: Value = self.json_response(response).await?;
        remote_record_from_submission(data)
    }

    async fn download_skill(
        &self,
        session: Option<&StoredCloudSession>,
        skill_id: &str,
    ) -> Result<Vec<u8>> {
        let request = self
            .http
            .get(self.endpoint(&format!("/api/skillhub/{skill_id}/download")));
        let response = match session {
            Some(session) => self.authed(session, request).await?,
            None => request.send().await.context("AgentWork download request")?,
        };
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(anyhow!("cloud authentication failed"));
        }
        if !response.status().is_success() {
            return Err(anyhow!(
                "AgentWork request failed with {}",
                response.status()
            ));
        }
        Ok(response
            .bytes()
            .await
            .context("read skill download")?
            .to_vec())
    }
}

pub async fn cloud_login(request: CloudLoginRequest) -> Result<CloudSession> {
    let client = AgentWorkSkillHubClient::new(request.server_url.clone())?;
    let stored = client.login(request).await?;
    CloudSessionStore::default()?.save(&stored)?;
    Ok(public_session(stored))
}

pub fn cloud_logout() -> Result<()> {
    CloudSessionStore::default()?.clear()
}

pub async fn cloud_refresh_session() -> Result<Option<CloudSession>> {
    let store = CloudSessionStore::default()?;
    let Some(current) = store.load()? else {
        return Ok(None);
    };
    let client = AgentWorkSkillHubClient::new(current.server_url.clone())?;
    let refreshed = client.refresh(&current).await?;
    store.save(&refreshed)?;
    Ok(Some(public_session(refreshed)))
}

pub async fn search_public_skills(
    request: SkillHubSearchRequest,
) -> Result<SkillHubPublicSkillsPage> {
    let session = CloudSessionStore::default()?.load()?;
    let server_url = crate::skillhub::types::resolve_skillhub_server_url(
        session.as_ref().map(|stored| stored.server_url.as_str()),
    );
    AgentWorkSkillHubClient::new(server_url)?
        .search_public_skills(request)
        .await
}

pub async fn get_public_skill_detail(skill_id: String) -> Result<SkillDetail> {
    let session = CloudSessionStore::default()?.load()?;
    let server_url = crate::skillhub::types::resolve_skillhub_server_url(
        session.as_ref().map(|stored| stored.server_url.as_str()),
    );
    AgentWorkSkillHubClient::new(server_url)?
        .get_public_skill_detail(&skill_id, session.as_ref())
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_agentwork_paths_from_base_url() {
        let client =
            AgentWorkSkillHubClient::new("http://127.0.0.1:3000/".into()).expect("valid base URL");
        assert_eq!(
            client.endpoint("/api/skillhub/public").as_str(),
            "http://127.0.0.1:3000/api/skillhub/public"
        );
        assert_eq!(
            client.endpoint("api/skillhub/private").as_str(),
            "http://127.0.0.1:3000/api/skillhub/private"
        );
    }

    #[test]
    fn rejects_an_invalid_server_url_without_panicking() {
        assert!(AgentWorkSkillHubClient::new("not a URL".into()).is_err());
    }

    #[test]
    fn public_search_request_uses_agentwork_q_query_parameter() {
        let client =
            AgentWorkSkillHubClient::new("http://127.0.0.1:3000/".into()).expect("valid base URL");
        let request = client
            .public_search_request(SkillHubSearchRequest {
                query: Some("abc".into()),
                limit: Some(10),
                page: None,
                sort: None,
                category: None,
            })
            .expect("build public search request");

        assert_eq!(
            request.url().as_str(),
            "http://127.0.0.1:3000/api/skillhub/public/search?q=abc&limit=10"
        );
    }

    #[test]
    fn public_search_item_accepts_agentwork_payload_without_views() {
        let skill: SkillHubPublicSkill = serde_json::from_value(json!({
            "id": "skill-123",
            "name": "query-writing",
            "description": "Write SQL queries",
            "slug": "zzq02/query-writing",
            "authorName": "zzq02",
            "downloadCount": 0,
            "starCount": 0
        }))
        .expect("decode AgentWork public skill");

        assert_eq!(skill.registry_slug, "zzq02/query-writing");
        assert_eq!(skill.author_username.as_deref(), Some("zzq02"));
        assert_eq!(skill.downloads, 0);
        assert_eq!(skill.stars, 0);
        assert_eq!(skill.views, 0);
        assert_eq!(skill.updated_at, None);

        // Test with string updatedAt
        let skill_str_update: SkillHubPublicSkill = serde_json::from_value(json!({
            "id": "skill-123",
            "name": "query-writing",
            "slug": "zzq02/query-writing",
            "downloadCount": 0,
            "starCount": 0,
            "updatedAt": "2025-11-12T19:58:47.092Z"
        }))
        .expect("decode with string updatedAt");
        assert_eq!(
            skill_str_update.updated_at.as_deref(),
            Some("2025-11-12T19:58:47.092Z")
        );

        // Test with integer timestamp updatedAt
        let skill_int_update: SkillHubPublicSkill = serde_json::from_value(json!({
            "id": "skill-123",
            "name": "query-writing",
            "slug": "zzq02/query-writing",
            "downloadCount": 0,
            "starCount": 0,
            "updatedAt": 1762910327092i64
        }))
        .expect("decode with integer updatedAt");
        // 1762910327092 ms == 2025-11-12T01:18:47.092Z (UTC)
        assert_eq!(
            skill_int_update.updated_at.as_deref(),
            Some("2025-11-12T01:18:47.092Z")
        );
    }

    #[test]
    fn public_search_request_accepts_frontend_query_field() {
        let request: SkillHubSearchRequest =
            serde_json::from_value(json!({ "query": "sql", "limit": 5 }))
                .expect("decode frontend search request");

        assert_eq!(request.query.as_deref(), Some("sql"));
        assert_eq!(request.limit, Some(5));
    }

    #[test]
    fn review_request_uses_agentwork_wire_names() {
        let value = serde_json::to_value(SubmitSkillReviewRequest {
            file_name: "skill.zip".into(),
            package_base64: "eA==".into(),
            local_skill_name: Some("local-demo".into()),
            local_skill_id: None,
            name: Some("Demo".into()),
            description: None,
            version: None,
            author: None,
            tags: None,
        })
        .expect("serialize request");
        assert_eq!(value["fileName"], "skill.zip");
        assert_eq!(value["packageBase64"], "eA==");
        assert!(value.get("zip_base64").is_none());
        assert!(value.get("localSkillName").is_none());
    }

    #[test]
    fn review_response_maps_agentwork_required_fields() {
        let record = remote_record_from_submission(json!({
            "skillId": "skill-123",
            "registrySlug": "demo-skill",
            "visibility": "private",
            "approvalStatus": "pending",
        }))
        .expect("map review response");

        assert_eq!(record.id, "skill-123");
        assert_eq!(record.registry_slug, "demo-skill");
        assert_eq!(record.visibility, "private");
        assert_eq!(record.approval_status.as_deref(), Some("pending"));
    }

    #[test]
    fn review_response_rejects_missing_registry_slug() {
        let error = remote_record_from_submission(json!({
            "skillId": "skill-123",
            "visibility": "private",
        }))
        .expect_err("registry slug is required");

        assert!(error.to_string().contains("registry slug"));
    }

    #[test]
    fn review_response_rejects_missing_visibility() {
        let error = remote_record_from_submission(json!({
            "skillId": "skill-123",
            "registrySlug": "demo-skill",
        }))
        .expect_err("visibility is required");

        assert!(error.to_string().contains("visibility"));
    }

    #[test]
    fn private_list_records_without_slug_derive_registry_slug_from_session_user() {
        let session = StoredCloudSession {
            server_url: "http://127.0.0.1:3000".into(),
            user: crate::skillhub::CloudUser {
                id: "user-123".into(),
                username: "alice".into(),
                display_name: None,
            },
            access_token: "access".into(),
            refresh_token: Some("refresh".into()),
        };

        let record = remote_record_from_private_list(
            json!({
                "id": "skill-123",
                "name": "CodeView Skill",
                "visibility": "private",
            }),
            &session,
        )
        .expect("fallback slug");

        assert_eq!(record.registry_slug, "alice/codeview-skill");
    }
}
