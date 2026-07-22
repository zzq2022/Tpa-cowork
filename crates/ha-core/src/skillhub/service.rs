use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, ErrorKind, Read};
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use zip::write::SimpleFileOptions;

use super::agentwork_client::{AgentWorkSkillHubClient, RemoteSkillHubClient};
use super::cloud_cache::CloudMetadataCache;
use super::session::CloudSessionStore;
use super::status::{derive_publish_state, registry_slug};
use super::types::{
    MySkillCloudEntry, RemoteSkillAvailability, RemoteSkillRecord, SkillCloudMetadata,
    SkillPublishState, StoredCloudSession, SubmitSkillReviewRequest,
};

const EXCLUDED_PACKAGE_COMPONENTS: &[&str] = &[".git", "target", "node_modules", "__pycache__"];
const SENSITIVE_PACKAGE_COMPONENTS: &[&str] = &[
    ".ssh",
    ".aws",
    ".azure",
    ".config",
    ".npmrc",
    ".pypirc",
    ".netrc",
    "credentials",
    "id_rsa",
    "id_ed25519",
];

/// Package a local skill without following symlinks or including generated and sensitive files.
pub fn package_local_skill_for_review(skill_dir: &Path) -> Result<Vec<u8>> {
    if fs::symlink_metadata(skill_dir)
        .with_context(|| format!("inspect skill directory {}", skill_dir.display()))?
        .file_type()
        .is_symlink()
    {
        bail!(
            "skill directory must not be a symlink: {}",
            skill_dir.display()
        );
    }
    let root = skill_dir
        .canonicalize()
        .with_context(|| format!("resolve skill directory {}", skill_dir.display()))?;
    if !root.is_dir() {
        bail!("skill directory is not a directory: {}", root.display());
    }
    let skill_md = root.join("SKILL.md");
    let skill_md_metadata = fs::symlink_metadata(&skill_md).with_context(|| {
        format!(
            "skill directory must contain root SKILL.md: {}",
            root.display()
        )
    })?;
    if skill_md_metadata.file_type().is_symlink() || !skill_md_metadata.is_file() {
        bail!("skill directory must contain a regular root SKILL.md");
    }
    let skill_md_target = skill_md.canonicalize().with_context(|| {
        format!(
            "skill directory must contain root SKILL.md: {}",
            root.display()
        )
    })?;
    if !skill_md_target.starts_with(&root) {
        bail!("skill directory must contain a regular root SKILL.md");
    }

    let mut files = Vec::new();
    collect_package_files(&root, &root, &mut files)?;
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = zip::ZipWriter::new(&mut cursor);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for file in files {
        let relative = file
            .strip_prefix(&root)
            .expect("collected file belongs to package root");
        let entry_name = path_to_zip_entry(relative)?;
        writer
            .start_file(entry_name, options)
            .context("add skill package entry")?;
        let mut input = fs::File::open(&file)
            .with_context(|| format!("open skill package file {}", file.display()))?;
        std::io::copy(&mut input, &mut writer)
            .with_context(|| format!("package skill file {}", file.display()))?;
    }
    writer.finish().context("finish skill package")?;
    Ok(cursor.into_inner())
}

/// Submit an installed local skill to the logged-in SkillHub account for review.
pub async fn submit_local_skill_for_review(local_skill_name: String) -> Result<MySkillCloudEntry> {
    let store = CloudSessionStore::default()?;
    let session = store
        .load()?
        .ok_or_else(|| anyhow!("cloud login required"))?;
    let detail = crate::skills::commands::get_skill_detail(&local_skill_name)
        .ok_or_else(|| anyhow!("local skill not found: {local_skill_name}"))?;
    let package = package_local_skill_for_review(Path::new(&detail.base_dir))?;
    let package_base64 = base64::engine::general_purpose::STANDARD.encode(package);
    let request = SubmitSkillReviewRequest {
        file_name: format!("{}.zip", detail.name),
        package_base64: package_base64.clone(),
        local_skill_name: Some(local_skill_name.clone()),
        local_skill_id: detail
            .skill_key
            .clone()
            .or_else(|| Some(local_skill_name.clone())),
        name: Some(detail.name),
        description: Some(detail.description),
        version: detail.display.version,
        author: detail.display.author,
        tags: (!detail.display.tags.is_empty()).then_some(detail.display.tags),
    };

    let client = AgentWorkSkillHubClient::new(session.server_url.clone())?;
    let remote =
        submit_review_with_refresh_retry(&client, &store, &session, request, package_base64)
            .await?;

    let now = unix_now_secs();
    let metadata = SkillCloudMetadata {
        registry_slug: remote.registry_slug.clone(),
        remote_skill_id: Some(remote.id.clone()),
        upstream_slug: remote.upstream_slug.clone(),
        publish_state: derive_publish_state(Some(&remote), RemoteSkillAvailability::Reachable),
        publish_state_reason: None,
        last_synced_at: Some(now),
        last_error: None,
    };
    let cache_store = CloudMetadataCache::default()?;
    cache_store.upsert(&session.user.username, &local_skill_name, metadata)?;

    let local_skills = crate::skills::commands::list_skills();
    let cache = cache_store.load_all().unwrap_or_default();
    let entry = overlay_local_skills_with_cache(&session.user.username, local_skills, &cache)
        .into_iter()
        .find(|entry| entry.local.name == local_skill_name)
        .ok_or_else(|| {
            anyhow!("submitted local skill not found in skill list: {local_skill_name}")
        })?;
    Ok(entry)
}

/// Download a public SkillHub package, install it locally, then return its local Skill row.
pub async fn install_skillhub_download(
    skill_id: String,
    overwrite: bool,
) -> Result<MySkillCloudEntry> {
    let session = CloudSessionStore::default()?.load()?;
    let server_url = crate::skillhub::types::resolve_skillhub_server_url(
        session.as_ref().map(|current| current.server_url.as_str()),
    );
    let package = AgentWorkSkillHubClient::new(server_url)?
        .download_skill(session.as_ref(), &skill_id)
        .await?;
    let local_skill_name = install_downloaded_zip_as_local_skill(&package, &skill_id, overwrite)?;
    crate::skills::bump_skill_version();

    if let Some(session) = session.as_ref() {
        let metadata = SkillCloudMetadata {
            registry_slug: registry_slug(&session.user.username, &local_skill_name),
            remote_skill_id: Some(skill_id.clone()),
            upstream_slug: None,
            publish_state: SkillPublishState::Unknown,
            publish_state_reason: None,
            last_synced_at: Some(unix_now_secs()),
            last_error: None,
        };
        let _ = CloudMetadataCache::default()?.upsert(
            &session.user.username,
            &local_skill_name,
            metadata,
        );
    }

    list_my_skills_with_cloud()
        .await?
        .into_iter()
        .find(|entry| entry.local.name == local_skill_name)
        .ok_or_else(|| anyhow!("installed local skill not found in skill list: {local_skill_name}"))
}

/// Install a downloaded archive into Hope's managed skills directory.
pub fn install_downloaded_zip_as_local_skill(
    zip_bytes: &[u8],
    remote_skill_id: &str,
    overwrite: bool,
) -> Result<String> {
    let root = crate::paths::skills_dir()?;
    install_downloaded_zip_as_local_skill_in_root(zip_bytes, remote_skill_id, &root, overwrite)
}

fn collect_package_files(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(current)
        .with_context(|| format!("read skill directory {}", current.display()))?
    {
        let entry = entry.with_context(|| format!("read entry in {}", current.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("inspect skill entry {}", entry.path().display()))?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .expect("walked path belongs to package root");
        if is_excluded_package_path(relative) {
            continue;
        }
        if file_type.is_dir() {
            collect_package_files(root, &path, files)?;
        } else if file_type.is_file() {
            let canonical = path
                .canonicalize()
                .with_context(|| format!("resolve skill package file {}", path.display()))?;
            if canonical.starts_with(root) {
                files.push(path);
            }
        }
    }
    Ok(())
}

fn is_excluded_package_path(relative: &Path) -> bool {
    relative.components().any(|component| match component {
        Component::Normal(name) => {
            let name = name.to_string_lossy().to_ascii_lowercase();
            EXCLUDED_PACKAGE_COMPONENTS.contains(&name.as_str())
                || SENSITIVE_PACKAGE_COMPONENTS.contains(&name.as_str())
                || name.starts_with(".env")
                || name.starts_with("credentials.")
                || name.ends_with(".pem")
                || name.ends_with(".key")
        }
        _ => true,
    })
}

fn path_to_zip_entry(path: &Path) -> Result<String> {
    let parts = path
        .components()
        .map(|component| match component {
            Component::Normal(part) => Ok(part.to_string_lossy().into_owned()),
            _ => bail!("invalid skill package path: {}", path.display()),
        })
        .collect::<Result<Vec<_>>>()?;
    if parts.is_empty() {
        bail!("invalid empty skill package path");
    }
    Ok(parts.join("/"))
}
fn install_downloaded_zip_as_local_skill_in_root(
    zip_bytes: &[u8],
    remote_skill_id: &str,
    root: &Path,
    overwrite: bool,
) -> Result<String> {
    fs::create_dir_all(root).with_context(|| format!("create skills root {}", root.display()))?;
    let mut archive =
        zip::ZipArchive::new(Cursor::new(zip_bytes)).context("open downloaded skill ZIP")?;

    // First pass: locate the SKILL.md file and determine its prefix directory (if any)
    let mut skill_md_prefix: Option<String> = None;
    for index in 0..archive.len() {
        if let Ok(entry) = archive.by_index(index) {
            let name = entry.name();
            if name == "SKILL.md" {
                skill_md_prefix = Some("".to_string());
                break;
            } else if name.ends_with("/SKILL.md") {
                let prefix = &name[..name.len() - "SKILL.md".len()];
                if skill_md_prefix.is_none()
                    || prefix.len() < skill_md_prefix.as_ref().unwrap().len()
                {
                    skill_md_prefix = Some(prefix.to_string());
                }
            }
        }
    }

    let prefix =
        skill_md_prefix.ok_or_else(|| anyhow!("downloaded skill ZIP must contain SKILL.md"))?;

    let mut files = Vec::new();
    let mut root_skill_md = None;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .context("read downloaded ZIP entry")?;
        let name = entry.name().to_string();
        if !name.starts_with(&prefix) {
            continue;
        }
        let relative_name = name[prefix.len()..].to_string();
        if relative_name.is_empty() {
            continue;
        }
        if !is_safe_zip_entry_name(&relative_name) {
            bail!("unsafe ZIP entry: {relative_name}");
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("unsafe ZIP entry symlink: {relative_name}");
        }
        if entry.is_dir() {
            continue;
        }
        let mut content = Vec::new();
        entry
            .read_to_end(&mut content)
            .with_context(|| format!("read downloaded ZIP entry {name}"))?;
        if relative_name == "SKILL.md" {
            if root_skill_md.replace(content.clone()).is_some() {
                bail!("downloaded skill ZIP contains duplicate SKILL.md");
            }
        }
        files.push((relative_name, content));
    }

    let root_skill_md =
        root_skill_md.ok_or_else(|| anyhow!("downloaded skill ZIP must contain SKILL.md"))?;
    let local_skill_name = local_skill_name_from_skill_md(&root_skill_md, remote_skill_id);
    let target = root.join(normalize_skill_dir_name(&local_skill_name));
    let staging = tempfile::Builder::new()
        .prefix(".skillhub-download-")
        .tempdir_in(root)
        .with_context(|| format!("create skill install staging in {}", root.display()))?;
    for (name, content) in files {
        let destination = staging.path().join(&name);
        let parent = destination.parent().expect("ZIP entry has a parent");
        fs::create_dir_all(parent)
            .with_context(|| format!("create downloaded skill directory {}", parent.display()))?;
        fs::write(&destination, content)
            .with_context(|| format!("write downloaded skill file {}", destination.display()))?;
    }
    install_staged_skill(staging.path(), &target, overwrite)?;
    Ok(local_skill_name)
}

fn install_staged_skill(staging: &Path, target: &Path, overwrite: bool) -> Result<()> {
    let existing_metadata = match fs::symlink_metadata(target) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error).with_context(|| format!("inspect {}", target.display()));
        }
    };

    let Some(metadata) = existing_metadata else {
        fs::rename(staging, target)
            .with_context(|| format!("install downloaded skill at {}", target.display()))?;
        return Ok(());
    };

    if !overwrite {
        bail!("local skill already exists: {}", target.display());
    }

    if metadata.file_type().is_symlink() {
        bail!(
            "existing local skill path must not be a symlink: {}",
            target.display()
        );
    }
    if !metadata.is_dir() {
        bail!(
            "existing local skill path is not a directory: {}",
            target.display()
        );
    }

    let root = target
        .parent()
        .ok_or_else(|| anyhow!("skill install target has no parent: {}", target.display()))?;
    let backup = tempfile::Builder::new()
        .prefix(".skillhub-overwrite-backup-")
        .tempdir_in(root)
        .with_context(|| format!("create skill overwrite backup in {}", root.display()))?;
    let backup_path = backup.keep();
    fs::remove_dir(&backup_path)
        .with_context(|| format!("prepare skill overwrite backup {}", backup_path.display()))?;
    fs::rename(target, &backup_path).with_context(|| {
        format!(
            "move existing local skill {} to {}",
            target.display(),
            backup_path.display()
        )
    })?;

    if let Err(error) = fs::rename(staging, target) {
        let restore_result = fs::rename(&backup_path, target);
        if let Err(restore_error) = restore_result {
            return Err(error).with_context(|| {
                format!(
                    "install downloaded skill at {} failed, and restore from {} also failed: {}",
                    target.display(),
                    backup_path.display(),
                    restore_error
                )
            });
        }
        return Err(error)
            .with_context(|| format!("install downloaded skill at {}", target.display()));
    }

    fs::remove_dir_all(&backup_path)
        .with_context(|| format!("remove overwritten skill backup {}", backup_path.display()))?;
    Ok(())
}

fn is_safe_zip_entry_name(name: &str) -> bool {
    if name.is_empty() || name.contains('\\') || name.contains(':') {
        return false;
    }
    let path = Path::new(name);
    !path.is_absolute()
        && path.components().all(|component| match component {
            Component::Normal(part) => is_safe_zip_entry_component(&part.to_string_lossy()),
            _ => false,
        })
}

fn is_safe_zip_entry_component(component: &str) -> bool {
    if component.ends_with([' ', '.']) {
        return false;
    }
    let base_name = component.split('.').next().unwrap_or_default();
    let base_name = base_name.to_ascii_uppercase();
    !matches!(
        base_name.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

fn local_skill_name_from_skill_md(content: &[u8], remote_skill_id: &str) -> String {
    let content = std::str::from_utf8(content).unwrap_or_default();
    let frontmatter_name = content
        .trim_start()
        .strip_prefix("---")
        .and_then(|rest| rest.split_once("\n---"))
        .and_then(|(frontmatter, _)| {
            frontmatter.lines().find_map(|line| {
                let value = line.trim().strip_prefix("name:")?.trim();
                (!value.is_empty())
                    .then(|| value.trim_matches(|c| c == '\'' || c == '\"').to_string())
            })
        });
    if let Some(name) = frontmatter_name {
        return name;
    }
    if let Some(name) = crate::skills::parse_skill_name_fallback(content) {
        return name;
    }
    normalize_skill_dir_name(remote_skill_id)
}

fn normalize_skill_dir_name(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
            previous_separator = false;
        } else if (character == '-' || character == '_') && !normalized.is_empty() {
            normalized.push(character);
            previous_separator = true;
        } else if !normalized.is_empty() && !previous_separator {
            normalized.push('-');
            previous_separator = true;
        }
    }
    let normalized = normalized.trim_matches(|character| character == '-' || character == '_');
    if normalized.is_empty() {
        "skill".to_string()
    } else {
        normalized.chars().take(96).collect()
    }
}

pub fn overlay_local_skills(
    username: &str,
    local_skills: Vec<crate::skills::SkillSummary>,
    remote_records: &[RemoteSkillRecord],
    availability: RemoteSkillAvailability,
) -> Vec<MySkillCloudEntry> {
    overlay_local_skills_for_user(
        username,
        None,
        local_skills,
        remote_records,
        availability,
        None,
    )
}

/// Overlay remote private records onto local skills using the full match order:
/// cached remoteSkillId → registrySlug → current username+slug → userId+skill name/slug.
pub fn overlay_local_skills_for_user(
    username: &str,
    owner_user_id: Option<&str>,
    local_skills: Vec<crate::skills::SkillSummary>,
    remote_records: &[RemoteSkillRecord],
    availability: RemoteSkillAvailability,
    previous_cache: Option<&HashMap<String, SkillCloudMetadata>>,
) -> Vec<MySkillCloudEntry> {
    let now = unix_now_secs();

    local_skills
        .into_iter()
        .filter(|skill| skill.source != "bundled")
        .map(|local| {
            let expected_slug = registry_slug(username, &local.name);
            let previous = previous_cache.and_then(|cache| {
                cache
                    .get(&CloudMetadataCache::cache_key(username, &local.name))
                    .cloned()
            });
            let remote = match_remote_record(
                &local.name,
                &expected_slug,
                owner_user_id,
                previous.as_ref(),
                remote_records,
            );

            let publish_state = derive_publish_state(remote, availability);
            let (last_synced_at, last_error) = match availability {
                RemoteSkillAvailability::Reachable => (Some(now), None),
                _ => (
                    previous.as_ref().and_then(|meta| meta.last_synced_at),
                    previous.as_ref().and_then(|meta| meta.last_error.clone()),
                ),
            };

            // On successful sync, unmatched locals become not-published (AgentWork "本地").
            // On unreachable cloud, preserve previous cache state when present.
            let cloud = if matches!(availability, RemoteSkillAvailability::Reachable) {
                SkillCloudMetadata {
                    registry_slug: expected_slug,
                    remote_skill_id: remote.map(|record| record.id.clone()),
                    upstream_slug: remote.and_then(|record| record.upstream_slug.clone()),
                    publish_state,
                    publish_state_reason: None,
                    last_synced_at,
                    last_error,
                }
            } else if let Some(mut previous) = previous {
                previous.publish_state = if previous.publish_state == SkillPublishState::Unknown {
                    publish_state
                } else {
                    previous.publish_state
                };
                previous.last_error = last_error.or(previous.last_error);
                previous
            } else {
                SkillCloudMetadata {
                    registry_slug: expected_slug,
                    remote_skill_id: None,
                    upstream_slug: None,
                    publish_state,
                    publish_state_reason: None,
                    last_synced_at,
                    last_error,
                }
            };

            MySkillCloudEntry { local, cloud }
        })
        .collect()
}

fn match_remote_record<'a>(
    local_skill_name: &str,
    expected_registry_slug: &str,
    owner_user_id: Option<&str>,
    previous: Option<&SkillCloudMetadata>,
    remote_records: &'a [RemoteSkillRecord],
) -> Option<&'a RemoteSkillRecord> {
    if let Some(remote_id) = previous
        .and_then(|meta| meta.remote_skill_id.as_deref())
        .filter(|id| !id.is_empty())
    {
        if let Some(record) = remote_records.iter().find(|record| record.id == remote_id) {
            return Some(record);
        }
    }

    if let Some(slug) = previous
        .map(|meta| meta.registry_slug.as_str())
        .filter(|slug| !slug.is_empty())
    {
        if let Some(record) = remote_records
            .iter()
            .find(|record| record.registry_slug == slug)
        {
            return Some(record);
        }
    }

    if let Some(record) = remote_records
        .iter()
        .find(|record| record.registry_slug == expected_registry_slug)
    {
        return Some(record);
    }

    let local_slug = super::status::skill_name_slug(local_skill_name);
    remote_records.iter().find(|record| {
        let owner_ok = match (owner_user_id, record.owner_user_id.as_deref()) {
            (Some(expected), Some(actual)) => expected == actual,
            // Private list is already scoped to the current session; accept name match
            // when owner id is absent from either side.
            _ => true,
        };
        if !owner_ok {
            return false;
        }
        if let Some(name) = record.name.as_deref() {
            if name.eq_ignore_ascii_case(local_skill_name)
                || super::status::skill_name_slug(name) == local_slug
            {
                return true;
            }
        }
        record
            .registry_slug
            .rsplit_once('/')
            .map(|(_, skill)| skill == local_slug || skill.eq_ignore_ascii_case(local_skill_name))
            .unwrap_or(false)
    })
}

/// Build local My Skill rows and attach any cached cloud review metadata.
///
/// Default page load / local refresh path: never calls the remote private list.
pub fn overlay_local_skills_with_cache(
    username: &str,
    local_skills: Vec<crate::skills::SkillSummary>,
    cache: &HashMap<String, SkillCloudMetadata>,
) -> Vec<MySkillCloudEntry> {
    local_skills
        .into_iter()
        .filter(|skill| skill.source != "bundled")
        .map(|local| {
            let registry_slug = registry_slug(username, &local.name);
            let cache_key = CloudMetadataCache::cache_key(username, &local.name);
            let cloud = cache.get(&cache_key).cloned().unwrap_or_else(|| {
                SkillCloudMetadata {
                    registry_slug: registry_slug.clone(),
                    remote_skill_id: None,
                    upstream_slug: None,
                    // Without an explicit cloud sync/submit result we do not claim
                    // "not published"; unknown keeps remote absence from becoming UI truth.
                    publish_state: SkillPublishState::Unknown,
                    publish_state_reason: None,
                    last_synced_at: None,
                    last_error: None,
                }
            });
            let cloud = SkillCloudMetadata {
                registry_slug: if cloud.registry_slug.is_empty() {
                    registry_slug
                } else {
                    cloud.registry_slug
                },
                ..cloud
            };
            MySkillCloudEntry { local, cloud }
        })
        .collect()
}

fn unix_now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

/// Local-first My Skills list. Uses local discovery as the only row source and
/// attaches cached cloud review badges. Does not call remote private skill APIs.
pub async fn list_my_skills_with_cloud() -> Result<Vec<MySkillCloudEntry>> {
    let local_skills = crate::skills::commands::list_skills();
    let username = CloudSessionStore::default()?
        .load()?
        .map(|session| session.user.username)
        .unwrap_or_default();
    let cache = CloudMetadataCache::default()?
        .load_all()
        .unwrap_or_default();
    Ok(overlay_local_skills_with_cache(
        &username,
        local_skills,
        &cache,
    ))
}

/// Cloud sync for My Skills. Pulls the current account's private remote records
/// and writes matched results into the local cloud metadata cache.
///
/// Logged-in My Skills page open / refresh should call this so desktop state
/// tracks server `userId + skill` publish status.
pub async fn refresh_my_skills_cloud() -> Result<Vec<MySkillCloudEntry>> {
    let local_skills = crate::skills::commands::list_skills();
    let store = CloudSessionStore::default()?;
    let cache_store = CloudMetadataCache::default()?;
    let previous_cache = cache_store.load_all().unwrap_or_default();
    let Some(session) = store.load()? else {
        return Ok(overlay_local_skills_with_cache(
            "",
            local_skills,
            &previous_cache,
        ));
    };

    let remote_result = match AgentWorkSkillHubClient::new(session.server_url.clone()) {
        Ok(client) => list_private_skills_with_refresh_retry(&client, &store, &session).await,
        Err(error) => Err(error),
    };

    let remote_records = match remote_result {
        Ok(records) => records,
        Err(error) => {
            let availability = availability_for_error(&error);
            let last_error = error.to_string();
            let mut entries = overlay_local_skills_for_user(
                &session.user.username,
                Some(session.user.id.as_str()),
                local_skills,
                &[],
                availability,
                Some(&previous_cache),
            );
            for entry in &mut entries {
                entry.cloud.last_error = Some(last_error.clone());
            }
            return Ok(entries);
        }
    };

    let entries = overlay_local_skills_for_user(
        &session.user.username,
        Some(session.user.id.as_str()),
        local_skills,
        &remote_records,
        RemoteSkillAvailability::Reachable,
        Some(&previous_cache),
    );
    let updates = entries
        .iter()
        .map(|entry| (entry.local.name.clone(), entry.cloud.clone()))
        .collect::<Vec<_>>();
    cache_store.upsert_many(&session.user.username, updates)?;
    Ok(entries)
}

fn availability_for_error(error: &anyhow::Error) -> RemoteSkillAvailability {
    if is_auth_failed_error(error) {
        RemoteSkillAvailability::AuthFailed
    } else {
        RemoteSkillAvailability::Offline
    }
}

fn is_auth_failed_error(error: &anyhow::Error) -> bool {
    error.to_string().contains("cloud authentication failed")
}

async fn refresh_stored_session<C: RemoteSkillHubClient>(
    client: &C,
    store: &CloudSessionStore,
    session: &StoredCloudSession,
) -> Result<StoredCloudSession> {
    let refreshed = client.refresh(session).await?;
    store.save(&refreshed)?;
    Ok(refreshed)
}

async fn list_private_skills_with_refresh_retry<C: RemoteSkillHubClient>(
    client: &C,
    store: &CloudSessionStore,
    session: &StoredCloudSession,
) -> Result<Vec<RemoteSkillRecord>> {
    match client.list_private_skills(session).await {
        Ok(records) => Ok(records),
        Err(error) if is_auth_failed_error(&error) => {
            let refreshed = refresh_stored_session(client, store, session).await?;
            client.list_private_skills(&refreshed).await
        }
        Err(error) => Err(error),
    }
}

async fn submit_review_with_refresh_retry<C: RemoteSkillHubClient>(
    client: &C,
    store: &CloudSessionStore,
    session: &StoredCloudSession,
    request: SubmitSkillReviewRequest,
    package_base64: String,
) -> Result<RemoteSkillRecord> {
    match client
        .submit_review(session, request.clone(), package_base64.clone())
        .await
    {
        Ok(record) => Ok(record),
        Err(error) if is_auth_failed_error(&error) => {
            let refreshed = refresh_stored_session(client, store, session).await?;
            client
                .submit_review(&refreshed, request, package_base64)
                .await
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skillhub::{RemoteSkillAvailability, RemoteSkillRecord, SkillPublishState};
    use crate::skills::{SkillDisplay, SkillStatus, SkillSummary};
    use async_trait::async_trait;
    use std::collections::VecDeque;
    use std::io::{Cursor, Write};
    use std::sync::Mutex;
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    fn local(name: &str) -> SkillSummary {
        SkillSummary {
            name: name.into(),
            description: String::new(),
            source: "user".into(),
            base_dir: format!("/tmp/{name}"),
            enabled: true,
            requires_env: vec![],
            skill_key: None,
            user_invocable: Some(true),
            disable_model_invocation: Some(false),
            has_install: false,
            any_bins: vec![],
            always: false,
            allowed_tools: vec![],
            context_mode: None,
            agent: None,
            effort: None,
            status: SkillStatus::Active,
            authored_by: None,
            display: SkillDisplay::default(),
        }
    }

    fn remote(slug: &str, visibility: &str, approval_status: Option<&str>) -> RemoteSkillRecord {
        let name = slug
            .rsplit_once('/')
            .map(|(_, skill)| skill.to_string())
            .unwrap_or_else(|| slug.to_string());
        RemoteSkillRecord {
            id: format!("remote-{slug}"),
            registry_slug: slug.into(),
            visibility: visibility.into(),
            approval_status: approval_status.map(str::to_string),
            upstream_slug: None,
            name: Some(name),
            owner_user_id: Some("user-1".into()),
        }
    }

    fn stored_session(access_token: &str) -> StoredCloudSession {
        StoredCloudSession {
            server_url: "http://127.0.0.1:3000".into(),
            user: crate::skillhub::CloudUser {
                id: "user-1".into(),
                username: "alice".into(),
                display_name: None,
            },
            access_token: access_token.into(),
            refresh_token: Some("refresh".into()),
        }
    }

    struct RetryClient {
        list_results: Mutex<VecDeque<Result<Vec<RemoteSkillRecord>>>>,
        submit_results: Mutex<VecDeque<Result<RemoteSkillRecord>>>,
    }

    impl RetryClient {
        fn new(
            list_results: Vec<Result<Vec<RemoteSkillRecord>>>,
            submit_results: Vec<Result<RemoteSkillRecord>>,
        ) -> Self {
            Self {
                list_results: Mutex::new(list_results.into()),
                submit_results: Mutex::new(submit_results.into()),
            }
        }
    }

    #[async_trait]
    impl RemoteSkillHubClient for RetryClient {
        async fn login(
            &self,
            _request: crate::skillhub::CloudLoginRequest,
        ) -> Result<StoredCloudSession> {
            Ok(stored_session("new-access"))
        }

        async fn refresh(&self, _session: &StoredCloudSession) -> Result<StoredCloudSession> {
            Ok(stored_session("new-access"))
        }

        async fn list_private_skills(
            &self,
            session: &StoredCloudSession,
        ) -> Result<Vec<RemoteSkillRecord>> {
            assert_ne!(session.access_token, "");
            self.list_results
                .lock()
                .expect("lock list results")
                .pop_front()
                .expect("list result")
        }

        async fn search_public_skills(
            &self,
            _request: crate::skillhub::SkillHubSearchRequest,
        ) -> Result<crate::skillhub::SkillHubPublicSkillsPage> {
            Ok(crate::skillhub::SkillHubPublicSkillsPage {
                items: Vec::new(),
                total: 0,
                page_size: 10,
            })
        }

        async fn get_public_skill_detail(
            &self,
            _skill_id: &str,
            _session: Option<&StoredCloudSession>,
        ) -> Result<crate::skillhub::SkillDetail> {
            Err(anyhow::anyhow!(
                "mock get_public_skill_detail not implemented"
            ))
        }

        async fn submit_review(
            &self,
            session: &StoredCloudSession,
            _request: SubmitSkillReviewRequest,
            _zip_base64: String,
        ) -> Result<RemoteSkillRecord> {
            assert_ne!(session.access_token, "");
            self.submit_results
                .lock()
                .expect("lock submit results")
                .pop_front()
                .expect("submit result")
        }

        async fn download_skill(
            &self,
            _session: Option<&StoredCloudSession>,
            _skill_id: &str,
        ) -> Result<Vec<u8>> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn overlays_remote_status_without_adding_remote_only_rows() {
        let locals = vec![local("code-review")];
        let remotes = vec![
            remote("alice/code-review", "private", Some("pending")),
            remote("alice/remote-only", "shared", Some("approved")),
        ];

        let rows = overlay_local_skills(
            "alice",
            locals,
            &remotes,
            RemoteSkillAvailability::Reachable,
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].local.name, "code-review");
        assert_eq!(rows[0].cloud.registry_slug, "alice/code-review");
        assert_eq!(
            rows[0].cloud.publish_state,
            SkillPublishState::PendingReview
        );
    }

    #[test]
    fn maps_reachable_remote_records_to_publish_states() {
        let locals = vec![local("published"), local("pending"), local("private")];
        let remotes = vec![
            remote("alice/published", "shared", Some("approved")),
            remote("alice/pending", "private", Some("pending")),
            remote("alice/private", "private", Some("rejected")),
        ];

        let states: Vec<_> = overlay_local_skills(
            "alice",
            locals,
            &remotes,
            RemoteSkillAvailability::Reachable,
        )
        .into_iter()
        .map(|entry| entry.cloud.publish_state)
        .collect();

        assert_eq!(
            states,
            vec![
                SkillPublishState::Published,
                SkillPublishState::PendingReview,
                SkillPublishState::NotPublished,
            ]
        );
    }

    #[test]
    fn matches_remote_by_cached_id_and_user_skill_name() {
        let locals = vec![local("Code Review")];
        let mut remotes = vec![remote("alice/other", "private", Some("pending"))];
        remotes[0].id = "remote-keep".into();
        remotes[0].name = Some("Code Review".into());
        remotes[0].registry_slug = "alice/code-review-renamed".into();

        let mut previous = HashMap::new();
        previous.insert(
            CloudMetadataCache::cache_key("alice", "Code Review"),
            SkillCloudMetadata {
                registry_slug: "alice/code-review".into(),
                remote_skill_id: Some("remote-keep".into()),
                upstream_slug: None,
                publish_state: SkillPublishState::NotPublished,
                publish_state_reason: None,
                last_synced_at: Some(1),
                last_error: None,
            },
        );

        let rows = overlay_local_skills_for_user(
            "alice",
            Some("user-1"),
            locals,
            &remotes,
            RemoteSkillAvailability::Reachable,
            Some(&previous),
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].cloud.remote_skill_id.as_deref(),
            Some("remote-keep")
        );
        assert_eq!(
            rows[0].cloud.publish_state,
            SkillPublishState::PendingReview
        );
    }

    #[test]
    fn reachable_sync_marks_unmatched_local_as_not_published() {
        let rows = overlay_local_skills_for_user(
            "alice",
            Some("user-1"),
            vec![local("local-only")],
            &[],
            RemoteSkillAvailability::Reachable,
            None,
        );
        assert_eq!(rows[0].cloud.publish_state, SkillPublishState::NotPublished);
        assert!(rows[0].cloud.last_synced_at.is_some());
    }

    #[test]
    fn keeps_local_skills_when_cloud_is_unconfigured() {
        let rows = overlay_local_skills(
            "",
            vec![local("local-only")],
            &[],
            RemoteSkillAvailability::Unconfigured,
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].local.name, "local-only");
        assert_eq!(rows[0].cloud.registry_slug, "local-only");
        assert_eq!(rows[0].cloud.publish_state, SkillPublishState::Unknown);
    }

    #[test]
    fn excludes_bundled_local_skills_from_my_skill_overlay() {
        let mut bundled = local("bundled-skill");
        bundled.source = "bundled".into();

        let rows = overlay_local_skills(
            "alice",
            vec![local("user-skill"), bundled],
            &[],
            RemoteSkillAvailability::Reachable,
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].local.name, "user-skill");
    }

    #[test]
    fn local_list_uses_cache_without_remote_only_rows() {
        let mut cache = HashMap::new();
        cache.insert(
            CloudMetadataCache::cache_key("alice", "code-review"),
            SkillCloudMetadata {
                registry_slug: "alice/code-review".into(),
                remote_skill_id: Some("r1".into()),
                upstream_slug: None,
                publish_state: SkillPublishState::PendingReview,
                publish_state_reason: None,
                last_synced_at: Some(10),
                last_error: None,
            },
        );
        // Remote-only cache must not invent a local row.
        cache.insert(
            CloudMetadataCache::cache_key("alice", "remote-only"),
            SkillCloudMetadata {
                registry_slug: "alice/remote-only".into(),
                remote_skill_id: Some("r2".into()),
                upstream_slug: None,
                publish_state: SkillPublishState::Published,
                publish_state_reason: None,
                last_synced_at: Some(10),
                last_error: None,
            },
        );

        let rows = overlay_local_skills_with_cache(
            "alice",
            vec![local("code-review"), local("unsubmitted")],
            &cache,
        );

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].local.name, "code-review");
        assert_eq!(
            rows[0].cloud.publish_state,
            SkillPublishState::PendingReview
        );
        assert_eq!(rows[1].local.name, "unsubmitted");
        assert_eq!(rows[1].cloud.publish_state, SkillPublishState::Unknown);
        assert!(rows[1].cloud.remote_skill_id.is_none());
    }

    #[test]
    fn account_scoped_cache_keys_do_not_leak_across_users() {
        let mut cache = HashMap::new();
        cache.insert(
            CloudMetadataCache::cache_key("alice", "demo"),
            SkillCloudMetadata {
                registry_slug: "alice/demo".into(),
                remote_skill_id: Some("alice-remote".into()),
                upstream_slug: None,
                publish_state: SkillPublishState::Published,
                publish_state_reason: None,
                last_synced_at: Some(1),
                last_error: None,
            },
        );

        let bob_rows = overlay_local_skills_with_cache("bob", vec![local("demo")], &cache);
        assert_eq!(bob_rows.len(), 1);
        assert_eq!(bob_rows[0].cloud.publish_state, SkillPublishState::Unknown);
        assert!(bob_rows[0].cloud.remote_skill_id.is_none());
    }

    #[tokio::test]
    async fn private_list_refreshes_expired_access_token_once() {
        let dir = tempdir().expect("tempdir");
        let store = CloudSessionStore::new(dir.path().join("cloud-session.json"));
        let session = stored_session("old-access");
        store.save(&session).expect("save old session");
        let client = RetryClient::new(
            vec![
                Err(anyhow!("cloud authentication failed")),
                Ok(vec![remote("alice/demo", "private", Some("pending"))]),
            ],
            vec![],
        );

        let records = list_private_skills_with_refresh_retry(&client, &store, &session)
            .await
            .expect("retry list");

        assert_eq!(records[0].registry_slug, "alice/demo");
        assert_eq!(
            store
                .load()
                .expect("load saved session")
                .expect("saved session")
                .access_token,
            "new-access"
        );
    }

    #[tokio::test]
    async fn submit_review_refreshes_expired_access_token_once() {
        let dir = tempdir().expect("tempdir");
        let store = CloudSessionStore::new(dir.path().join("cloud-session.json"));
        let session = stored_session("old-access");
        store.save(&session).expect("save old session");
        let client = RetryClient::new(
            vec![],
            vec![
                Err(anyhow!("cloud authentication failed")),
                Ok(remote("alice/demo", "private", Some("pending"))),
            ],
        );
        let request = SubmitSkillReviewRequest {
            file_name: "demo.zip".into(),
            package_base64: "eA==".into(),
            local_skill_name: Some("demo".into()),
            local_skill_id: Some("demo".into()),
            name: Some("demo".into()),
            description: None,
            version: None,
            author: None,
            tags: None,
        };

        let record =
            submit_review_with_refresh_retry(&client, &store, &session, request, "eA==".into())
                .await
                .expect("retry submit");

        assert_eq!(record.registry_slug, "alice/demo");
        assert_eq!(
            store
                .load()
                .expect("load saved session")
                .expect("saved session")
                .access_token,
            "new-access"
        );
    }

    #[test]
    fn packages_regular_skill_files_without_sensitive_entries() {
        let dir = tempdir().expect("tempdir");
        std::fs::write(dir.path().join("SKILL.md"), "---\nname: demo\n---\nbody").expect("skill");
        std::fs::write(dir.path().join("README.md"), "readme").expect("readme");
        std::fs::write(dir.path().join(".env"), "secret").expect("env");
        std::fs::create_dir_all(dir.path().join(".git")).expect("git dir");
        std::fs::write(dir.path().join(".git").join("config"), "private").expect("git config");

        let package = package_local_skill_for_review(dir.path()).expect("package skill");
        let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("open package");
        let mut names = (0..archive.len())
            .map(|index| archive.by_index(index).expect("entry").name().to_string())
            .collect::<Vec<_>>();
        names.sort();

        assert_eq!(names, vec!["README.md", "SKILL.md"]);
    }

    #[test]
    fn package_excludes_common_credential_paths() {
        let dir = tempdir().expect("tempdir");
        std::fs::write(dir.path().join("SKILL.md"), "---\nname: demo\n---\nbody").expect("skill");
        std::fs::create_dir_all(dir.path().join(".ssh")).expect("ssh dir");
        std::fs::write(dir.path().join(".ssh").join("id_rsa"), "private").expect("ssh key");
        std::fs::write(dir.path().join(".npmrc"), "token=secret").expect("npmrc");
        std::fs::write(dir.path().join("credentials.json"), "secret").expect("credentials");
        std::fs::write(dir.path().join("private.key"), "private").expect("key");

        let package = package_local_skill_for_review(dir.path()).expect("package skill");
        let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("open package");
        let mut names = (0..archive.len())
            .map(|index| archive.by_index(index).expect("entry").name().to_string())
            .collect::<Vec<_>>();
        names.sort();

        assert_eq!(names, vec!["SKILL.md"]);
    }

    #[test]
    fn package_requires_root_skill_md() {
        let dir = tempdir().expect("tempdir");
        let error = package_local_skill_for_review(dir.path()).expect_err("missing skill");

        assert!(error.to_string().contains("SKILL.md"));
    }

    #[test]
    fn installs_downloaded_zip_returns_scanner_visible_frontmatter_name() {
        let root = tempdir().expect("root");
        let zip = skill_zip(&[
            ("SKILL.md", "---\nname: My Skill\n---\nbody"),
            ("README.md", "readme"),
        ]);

        let name =
            install_downloaded_zip_as_local_skill_in_root(&zip, "remote-123", root.path(), false)
                .expect("install zip");

        assert_eq!(name, "My Skill");
        assert_eq!(
            std::fs::read_to_string(root.path().join("my-skill").join("SKILL.md"))
                .expect("installed skill"),
            "---\nname: My Skill\n---\nbody"
        );
    }

    #[test]
    fn rejects_downloaded_zip_when_directory_conflicts_without_overwrite() {
        let root = tempdir().expect("root");
        std::fs::create_dir(root.path().join("my-skill")).expect("existing skill directory");
        let zip = skill_zip(&[("SKILL.md", "---\nname: My Skill\n---\nbody")]);

        let error =
            install_downloaded_zip_as_local_skill_in_root(&zip, "remote-123", root.path(), false)
                .expect_err("conflicting archive must require overwrite");

        assert!(error.to_string().contains("already exists"));
        assert!(!root.path().join("my-skill-remote-1").exists());
    }

    #[test]
    fn overwrites_downloaded_zip_when_directory_conflicts_with_overwrite() {
        let root = tempdir().expect("root");
        let existing = root.path().join("my-skill");
        std::fs::create_dir(&existing).expect("existing skill directory");
        std::fs::write(existing.join("SKILL.md"), "---\nname: Old Skill\n---\nold")
            .expect("old skill");
        std::fs::write(existing.join("LOCAL_ONLY.txt"), "local only").expect("local only");
        let zip = skill_zip(&[
            ("SKILL.md", "---\nname: My Skill\n---\nnew"),
            ("README.md", "new readme"),
        ]);

        let name =
            install_downloaded_zip_as_local_skill_in_root(&zip, "remote-123", root.path(), true)
                .expect("overwrite zip");

        assert_eq!(name, "My Skill");
        assert_eq!(
            std::fs::read_to_string(existing.join("SKILL.md")).expect("installed skill"),
            "---\nname: My Skill\n---\nnew"
        );
        assert!(existing.join("README.md").exists());
        assert!(!existing.join("LOCAL_ONLY.txt").exists());
        assert!(!root.path().join("my-skill-remote-1").exists());
    }

    #[test]
    fn rejects_downloaded_zip_with_unsafe_paths() {
        let root = tempdir().expect("root");
        let zip = skill_zip(&[
            ("SKILL.md", "---\nname: demo\n---\nbody"),
            ("../escape", "no"),
        ]);

        let error =
            install_downloaded_zip_as_local_skill_in_root(&zip, "remote-123", root.path(), false)
                .expect_err("unsafe archive must fail");

        assert!(error.to_string().contains("unsafe ZIP entry"));
        assert!(!root.path().join("demo").exists());
    }

    #[test]
    fn rejects_downloaded_zip_with_windows_sensitive_entry_names() {
        for unsafe_name in ["docs/readme:secret", "CON", "aux.txt", "docs/name. "] {
            let root = tempdir().expect("root");
            let zip = skill_zip(&[
                ("SKILL.md", "---\nname: demo\n---\nbody"),
                (unsafe_name, "no"),
            ]);

            let error = install_downloaded_zip_as_local_skill_in_root(
                &zip,
                "remote-123",
                root.path(),
                false,
            )
            .expect_err("unsafe archive must fail");

            assert!(
                error.to_string().contains("unsafe ZIP entry"),
                "{unsafe_name}"
            );
            assert!(!root.path().join("demo").exists(), "{unsafe_name}");
        }
    }

    fn skill_zip(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(&mut cursor);
        for (name, content) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .expect("start entry");
            writer.write_all(content.as_bytes()).expect("write entry");
        }
        writer.finish().expect("finish zip");
        cursor.into_inner()
    }
}
