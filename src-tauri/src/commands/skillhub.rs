use crate::commands::CmdError;

/// Thin Tauri adapters for Hope cloud session + SkillHub core services.
#[tauri::command]
pub async fn cloud_get_session() -> Result<Option<ha_core::skillhub::CloudSession>, CmdError> {
    ha_core::skillhub::get_cloud_session().map_err(CmdError::from)
}

#[tauri::command]
pub async fn cloud_login(
    request: ha_core::skillhub::CloudLoginRequest,
) -> Result<ha_core::skillhub::CloudSession, CmdError> {
    ha_core::skillhub::cloud_login(request)
        .await
        .map_err(CmdError::from)
}

#[tauri::command]
pub async fn cloud_logout() -> Result<(), CmdError> {
    ha_core::skillhub::cloud_logout().map_err(CmdError::from)
}

#[tauri::command]
pub async fn cloud_refresh_session() -> Result<Option<ha_core::skillhub::CloudSession>, CmdError> {
    ha_core::skillhub::cloud_refresh_session()
        .await
        .map_err(CmdError::from)
}

#[tauri::command]
pub async fn my_skills_list() -> Result<Vec<ha_core::skillhub::MySkillCloudEntry>, CmdError> {
    ha_core::skillhub::list_my_skills_with_cloud()
        .await
        .map_err(CmdError::from)
}

/// Explicit cloud sync. Not used by the default "refresh local status" button.
#[tauri::command]
pub async fn my_skills_refresh_cloud() -> Result<Vec<ha_core::skillhub::MySkillCloudEntry>, CmdError>
{
    ha_core::skillhub::refresh_my_skills_cloud()
        .await
        .map_err(CmdError::from)
}

#[tauri::command]
pub async fn my_skills_submit_review(
    local_skill_name: String,
) -> Result<ha_core::skillhub::MySkillCloudEntry, CmdError> {
    ha_core::skillhub::submit_local_skill_for_review(local_skill_name)
        .await
        .map_err(CmdError::from)
}

#[tauri::command]
pub async fn skillhub_search_public(
    request: ha_core::skillhub::SkillHubSearchRequest,
) -> Result<ha_core::skillhub::SkillHubPublicSkillsPage, CmdError> {
    ha_core::skillhub::search_public_skills(request)
        .await
        .map_err(CmdError::from)
}

#[tauri::command]
pub async fn skillhub_get_public_detail(
    skill_id: String,
) -> Result<ha_core::skillhub::SkillDetail, CmdError> {
    ha_core::skillhub::get_public_skill_detail(skill_id)
        .await
        .map_err(CmdError::from)
}

#[tauri::command]
pub async fn skillhub_download_skill(
    skill_id: String,
    overwrite: Option<bool>,
) -> Result<ha_core::skillhub::MySkillCloudEntry, CmdError> {
    ha_core::skillhub::install_skillhub_download(skill_id, overwrite.unwrap_or(false))
        .await
        .map_err(CmdError::from)
}
