use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::AppError;

/// `GET /api/cloud/session`
pub async fn cloud_get_session() -> Result<Json<Option<ha_core::skillhub::CloudSession>>, AppError>
{
    Ok(Json(ha_core::skillhub::get_cloud_session()?))
}

/// `POST /api/cloud/login`
pub async fn cloud_login(
    Json(request): Json<ha_core::skillhub::CloudLoginRequest>,
) -> Result<Json<ha_core::skillhub::CloudSession>, AppError> {
    Ok(Json(ha_core::skillhub::cloud_login(request).await?))
}

/// `POST /api/cloud/logout`
pub async fn cloud_logout() -> Result<Json<Value>, AppError> {
    ha_core::skillhub::cloud_logout()?;
    Ok(Json(json!({ "ok": true })))
}

/// `POST /api/cloud/session/refresh`
pub async fn cloud_refresh_session(
) -> Result<Json<Option<ha_core::skillhub::CloudSession>>, AppError> {
    Ok(Json(ha_core::skillhub::cloud_refresh_session().await?))
}

/// `GET /api/my-skills`
pub async fn my_skills_list() -> Result<Json<Vec<ha_core::skillhub::MySkillCloudEntry>>, AppError> {
    Ok(Json(ha_core::skillhub::list_my_skills_with_cloud().await?))
}

/// `POST /api/my-skills/refresh-cloud`
///
/// Explicit cloud sync for the current account. The default My Skills "refresh
/// status" button uses local list reload and must not call this path.
pub async fn my_skills_refresh_cloud(
) -> Result<Json<Vec<ha_core::skillhub::MySkillCloudEntry>>, AppError> {
    Ok(Json(ha_core::skillhub::refresh_my_skills_cloud().await?))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitReviewBody {
    pub local_skill_name: String,
}

/// `POST /api/my-skills/submit-review`
pub async fn my_skills_submit_review(
    Json(body): Json<SubmitReviewBody>,
) -> Result<Json<ha_core::skillhub::MySkillCloudEntry>, AppError> {
    Ok(Json(
        ha_core::skillhub::submit_local_skill_for_review(body.local_skill_name).await?,
    ))
}

/// `POST /api/skillhub/public/search`
pub async fn skillhub_search_public(
    Json(request): Json<ha_core::skillhub::SkillHubSearchRequest>,
) -> Result<Json<ha_core::skillhub::SkillHubPublicSkillsPage>, AppError> {
    Ok(Json(
        ha_core::skillhub::search_public_skills(request).await?,
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDetailBody {
    pub skill_id: String,
}

/// `POST /api/skillhub/public/detail`
pub async fn skillhub_get_public_detail(
    Json(body): Json<GetDetailBody>,
) -> Result<Json<ha_core::skillhub::SkillDetail>, AppError> {
    Ok(Json(
        ha_core::skillhub::get_public_skill_detail(body.skill_id).await?,
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSkillBody {
    pub skill_id: String,
    #[serde(default)]
    pub overwrite: bool,
}

/// `POST /api/skillhub/download`
pub async fn skillhub_download_skill(
    Json(body): Json<DownloadSkillBody>,
) -> Result<Json<ha_core::skillhub::MySkillCloudEntry>, AppError> {
    Ok(Json(
        ha_core::skillhub::install_skillhub_download(body.skill_id, body.overwrite).await?,
    ))
}
