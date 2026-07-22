use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

// ── Root Directory ───────────────────────────────────────────────

/// Returns the root directory for all TPA CoWork data.
///
/// Resolution order:
/// 1. `TPA_DATA_DIR`, then legacy `HA_DATA_DIR`, used as-is.
///    Lets users run in portable mode and lets cross-platform integration
///    tests redirect into a tempdir — `dirs::home_dir()` on Windows reads
///    `SHGetKnownFolderPath`, not `%USERPROFILE%`, so HOME-style overrides
///    don't work there.
/// 2. `dirs::home_dir().join(".tpa-cowork")` for the normal install path.
pub fn root_dir() -> Result<PathBuf> {
    if let Some(override_dir) = std::env::var_os("TPA_DATA_DIR") {
        let path = PathBuf::from(override_dir);
        if !path.as_os_str().is_empty() {
            return Ok(path);
        }
    }
    if let Some(override_dir) = std::env::var_os("HA_DATA_DIR") {
        let path = PathBuf::from(override_dir);
        if !path.as_os_str().is_empty() {
            return Ok(path);
        }
    }
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    Ok(home.join(".tpa-cowork"))
}

/// Migrate the old home-directory data root before creating the new root.
/// Explicit data-directory overrides are isolated and never trigger this move.
pub fn migrate_legacy_data_dir() -> Result<()> {
    if has_data_dir_override() {
        return Ok(());
    }

    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    migrate_legacy_data(&home.join(".hope-agent"), &home.join(".tpa-cowork"))
}

fn has_data_dir_override() -> bool {
    ["TPA_DATA_DIR", "HA_DATA_DIR"]
        .into_iter()
        .filter_map(std::env::var_os)
        .any(|value| !value.is_empty())
}

pub(crate) fn migrate_legacy_data(source: &Path, destination: &Path) -> Result<()> {
    if path_exists(destination) || !path_exists(source) {
        return Ok(());
    }
    if !fs::symlink_metadata(source)?.is_dir() {
        anyhow::bail!("migration source is not a directory: {}", source.display());
    }

    match fs::rename(source, destination) {
        Ok(()) => return Ok(()),
        Err(_rename_error) if path_exists(destination) => return Ok(()),
        Err(rename_error) => {
            migrate_legacy_data_by_copy(source, destination).with_context(|| {
                format!(
                    "rename {} to {} failed ({rename_error}); fallback copy failed",
                    source.display(),
                    destination.display()
                )
            })?;
        }
    }

    Ok(())
}

fn migrate_legacy_data_by_copy(source: &Path, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow::anyhow!("destination has no parent directory"))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("create migration parent {}", parent.display()))?;

    let staging = tempfile::Builder::new()
        .prefix(".tpa-cowork-migration-")
        .tempdir_in(parent)
        .with_context(|| format!("create migration staging directory in {}", parent.display()))?;
    let staged_root = staging.path().join("data");

    copy_tree(source, &staged_root)?;
    verify_tree(source, &staged_root)?;

    if path_exists(destination) {
        return Ok(());
    }
    fs::rename(&staged_root, destination).with_context(|| {
        format!(
            "promote migration staging {} to {}",
            staged_root.display(),
            destination.display()
        )
    })?;

    fs::remove_dir_all(source)
        .with_context(|| format!("remove migrated source {}", source.display()))?;
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)
        .with_context(|| format!("read migration source {}", source.display()))?;
    if !metadata.is_dir() {
        anyhow::bail!("migration source is not a directory: {}", source.display());
    }
    fs::create_dir(destination)
        .with_context(|| format!("create migration staging {}", destination.display()))?;

    for entry in fs::read_dir(source).with_context(|| format!("read {}", source.display()))? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path).with_context(|| {
                format!("copy {} to {}", source_path.display(), destination_path.display())
            })?;
        } else if metadata.file_type().is_symlink() {
            copy_symlink(&source_path, &destination_path)?;
        } else {
            anyhow::bail!("unsupported migration entry: {}", source_path.display());
        }
    }
    Ok(())
}

fn verify_tree(source: &Path, destination: &Path) -> Result<()> {
    let source_metadata = fs::symlink_metadata(source)?;
    let destination_metadata = fs::symlink_metadata(destination)?;
    if source_metadata.is_dir() {
        if !destination_metadata.is_dir() {
            anyhow::bail!("migration verification type mismatch: {}", destination.display());
        }
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            verify_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else if source_metadata.is_file() {
        if !destination_metadata.is_file()
            || source_metadata.len() != destination_metadata.len()
            || !files_equal(source, destination)?
        {
            anyhow::bail!("migration verification failed: {}", source.display());
        }
    } else if source_metadata.file_type().is_symlink() {
        if !destination_metadata.file_type().is_symlink()
            || fs::read_link(source)? != fs::read_link(destination)?
        {
            anyhow::bail!("migration symlink verification failed: {}", source.display());
        }
    }
    Ok(())
}

fn path_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn files_equal(source: &Path, destination: &Path) -> Result<bool> {
    use std::io::{BufReader, Read};

    let mut source = BufReader::new(fs::File::open(source)?);
    let mut destination = BufReader::new(fs::File::open(destination)?);
    let mut source_buffer = [0_u8; 64 * 1024];
    let mut destination_buffer = [0_u8; 64 * 1024];
    loop {
        let source_size = source.read(&mut source_buffer)?;
        let destination_size = destination.read(&mut destination_buffer)?;
        if source_size != destination_size {
            return Ok(false);
        }
        if source_size == 0 {
            return Ok(true);
        }
        if source_buffer[..source_size] != destination_buffer[..destination_size] {
            return Ok(false);
        }
    }
}

fn copy_symlink(source: &Path, destination: &Path) -> Result<()> {
    let target = fs::read_link(source)?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, destination)?;
    #[cfg(windows)]
    {
        let metadata = fs::metadata(source);
        if metadata.map(|value| value.is_dir()).unwrap_or(false) {
            std::os::windows::fs::symlink_dir(&target, destination)?;
        } else {
            std::os::windows::fs::symlink_file(&target, destination)?;
        }
    }
    Ok(())
}

/// Emergency append-only stream spool. Used only for non-incognito chat runs
/// when SQLite cannot accept a journal batch.
pub fn stream_spool_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("stream_spool"))
}

pub fn stream_spool_path(run_id: &str) -> Result<PathBuf> {
    uuid::Uuid::parse_str(run_id).map_err(|_| anyhow::anyhow!("invalid stream run id"))?;
    Ok(stream_spool_dir()?.join(format!("{run_id}.log")))
}

/// Evaluation Center state. This is deliberately separate from sessions.db:
/// synthetic trials must never become user conversations or memory input.
pub fn evals_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("evals"))
}

pub fn evals_db_path() -> Result<PathBuf> {
    Ok(evals_dir()?.join("evals.db"))
}

pub fn eval_artifacts_dir() -> Result<PathBuf> {
    Ok(evals_dir()?.join("artifacts"))
}

/// Ephemeral files used while a project chat is preparing a managed worktree.
pub fn bootstrap_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("bootstrap"))
}

/// Bootstrap request ids are restricted to portable filename characters at
/// the API boundary and checked again here before becoming a path component.
pub fn bootstrap_run_dir(request_id: &str) -> Result<PathBuf> {
    if request_id.is_empty()
        || request_id.len() > 128
        || !request_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        anyhow::bail!("invalid bootstrap request id");
    }
    Ok(bootstrap_dir()?.join(request_id))
}

/// Temporary snapshots used by user-initiated Git operations such as a
/// Local/Worktree handoff. The request id is validated before it becomes a
/// path component so cleanup can stay constrained to Hope's data directory.
pub fn git_operations_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("git-operations"))
}

pub fn git_operation_run_dir(request_id: &str) -> Result<PathBuf> {
    validate_portable_request_id(request_id)?;
    Ok(git_operations_dir()?.join(request_id))
}

/// Cross-process advisory locks for repository mutations. The filename is a
/// digest of the canonical repository root and never contains user path text.
pub fn git_locks_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("git-locks"))
}

pub fn git_repo_lock_path(repo_root: &std::path::Path) -> Result<PathBuf> {
    let canonical = repo_root.canonicalize()?;
    let digest = blake3::hash(canonical.to_string_lossy().as_bytes());
    Ok(git_locks_dir()?.join(format!("{}.lock", digest.to_hex())))
}

fn validate_portable_request_id(request_id: &str) -> Result<()> {
    if request_id.is_empty()
        || request_id.len() > 128
        || !request_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        anyhow::bail!("invalid git operation request id");
    }
    Ok(())
}

// ── Config ───────────────────────────────────────────────────────

/// Global config file path: ~/.tpa-cowork/config.json
pub fn config_path() -> Result<PathBuf> {
    Ok(root_dir()?.join("config.json"))
}

// ── Agents ───────────────────────────────────────────────────────

/// Agents root directory: ~/.tpa-cowork/agents/
pub fn agents_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("agents"))
}

/// Specific agent directory: ~/.tpa-cowork/agents/{id}/
pub fn agent_dir(id: &str) -> Result<PathBuf> {
    validate_agent_id(id)?;
    Ok(agents_dir()?.join(id))
}

/// Validate an Agent id before it participates in any filesystem path.
///
/// Agent ids are durable foreign keys across config and SQLite, so the
/// accepted shape is deliberately narrow and aligned with the GUI/import
/// creation surfaces. Keeping this guard in `paths` makes every owner-plane
/// read/write/delete path fail closed instead of relying on frontend checks.
pub fn validate_agent_id(id: &str) -> Result<()> {
    const MAX_LEN: usize = 64;
    let valid = !id.is_empty()
        && id.len() <= MAX_LEN
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        anyhow::bail!("Invalid agent ID: expected 1-{MAX_LEN} ASCII letters, digits, '-' or '_'");
    }
    Ok(())
}

// ── User Config ─────────────────────────────────────────────────

/// User config file path: ~/.tpa-cowork/user.json
pub fn user_config_path() -> Result<PathBuf> {
    Ok(root_dir()?.join("user.json"))
}

// ── Credentials ──────────────────────────────────────────────────

/// Credentials directory: ~/.tpa-cowork/credentials/
pub fn credentials_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("credentials"))
}

/// OAuth auth token path: ~/.tpa-cowork/credentials/auth.json
pub fn auth_path() -> Result<PathBuf> {
    Ok(credentials_dir()?.join("auth.json"))
}

/// MCP credentials directory: ~/.tpa-cowork/credentials/mcp/
pub fn mcp_credentials_dir() -> Result<PathBuf> {
    Ok(credentials_dir()?.join("mcp"))
}

/// Per-server MCP credentials file: ~/.tpa-cowork/credentials/mcp/{server_id}.json
pub fn mcp_credential_path(server_id: &str) -> Result<PathBuf> {
    Ok(mcp_credentials_dir()?.join(format!("{server_id}.json")))
}

/// External memory provider credentials directory:
/// `~/.tpa-cowork/credentials/external-memory/`.
pub fn external_memory_credentials_dir() -> Result<PathBuf> {
    Ok(credentials_dir()?.join("external-memory"))
}

/// Per-provider external memory credential file. Callers must validate that
/// `provider_id` is a normalized config id before using this path.
pub fn external_memory_credential_path(provider_id: &str) -> Result<PathBuf> {
    Ok(external_memory_credentials_dir()?.join(format!("{provider_id}.json")))
}

/// Durable per-provider sync ledger. It intentionally lives beside the
/// credential record and is written with the same restricted permissions:
/// hashes and remote ids can still reveal information about a user's memory
/// inventory even though the ledger contains no API key.
pub fn external_memory_sync_state_path(provider_id: &str) -> Result<PathBuf> {
    Ok(external_memory_credentials_dir()?.join(format!("{provider_id}.sync.json")))
}

/// GitHub token used only by the Issue Reporting tool.
pub fn github_issue_credential_path() -> Result<PathBuf> {
    Ok(credentials_dir()?.join("github-issue.json"))
}

// ── Channels ─────────────────────────────────────────────────────

/// Channels runtime state directory: ~/.tpa-cowork/channels/
pub fn channels_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("channels"))
}

/// Specific channel runtime state directory: ~/.tpa-cowork/channels/{channel_id}/
pub fn channel_dir(channel_id: &str) -> Result<PathBuf> {
    Ok(channels_dir()?.join(channel_id))
}

// ── Skills ───────────────────────────────────────────────────────

/// Skills directory: ~/.tpa-cowork/skills/
pub fn skills_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("skills"))
}

/// Extraction cache for skills embedded in the binary:
/// ~/.tpa-cowork/bundled-skills/<content-hash>/
/// Safe to delete — rebuilt from the binary on next use.
pub fn bundled_skills_cache_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("bundled-skills"))
}

// ── Manual (built-in user guide) ─────────────────────────────────

/// Stable mirror of the embedded bilingual user manual:
/// ~/.tpa-cowork/manual/{zh,en}/NN.md — read/grepped by the `ha-manual`
/// skill. Safe to delete — rebuilt from the binary on next use.
pub fn manual_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("manual"))
}

/// Completion marker for the manual mirror (stores the embedded source-set
/// fingerprint): ~/.tpa-cowork/.manual-synced. Lives beside `manual/` (not
/// inside it) so the mirror's prune never sweeps it.
pub fn manual_marker() -> Result<PathBuf> {
    Ok(root_dir()?.join(".manual-synced"))
}

// ── Permission ───────────────────────────────────────────────────

/// Permission system directory: ~/.tpa-cowork/permission/
/// Holds `protected-paths.json`, `dangerous-commands.json`,
/// `edit-commands.json`, `global-allowlist.json`.
pub fn permission_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("permission"))
}

// ── ffmpeg runtime (on-demand static build for MP4 export) ───────

/// ffmpeg runtime root: ~/.tpa-cowork/ffmpeg/
pub fn ffmpeg_runtime_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("ffmpeg"))
}

/// Versioned ffmpeg install dir: ~/.tpa-cowork/ffmpeg/{version}/
pub fn ffmpeg_version_dir(version: &str) -> Result<PathBuf> {
    Ok(ffmpeg_runtime_dir()?.join(version))
}

// ── Agent Home ───────────────────────────────────────────────────

/// Main agent home directory: ~/.tpa-cowork/home/
pub fn home_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("home"))
}

/// Named agent home directory: ~/.tpa-cowork/{name}-home/
pub fn agent_home_dir(name: &str) -> Result<PathBuf> {
    validate_agent_id(name)?;
    Ok(root_dir()?.join(format!("{}-home", name)))
}

// ── Attachments ──────────────────────────────────────────────────

/// Attachments directory for a session: ~/.tpa-cowork/attachments/{session_id}/
pub fn attachments_dir(session_id: &str) -> Result<PathBuf> {
    Ok(root_dir()?.join("attachments").join(session_id))
}

// ── Sessions (per-session artifacts: hook transcript mirror, …) ─────

/// Root for per-session artifact directories: ~/.tpa-cowork/sessions/
pub fn sessions_root() -> Result<PathBuf> {
    Ok(root_dir()?.join("sessions"))
}

/// Per-session artifact directory: ~/.tpa-cowork/sessions/{session_id}/
///
/// Like [`attachments_dir`], this only computes the path — callers create it
/// lazily (e.g. the hooks transcript mirror on first write).
pub fn session_dir(session_id: &str) -> Result<PathBuf> {
    Ok(sessions_root()?.join(session_id))
}

// ── Managed Worktrees ─────────────────────────────────────────────

/// Root for TPA CoWork managed git worktrees:
/// `~/.tpa-cowork/worktrees/{repo-slug}/{worktree-id}/`.
pub fn worktrees_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("worktrees"))
}

// ── Hooks ───────────────────────────────────────────────────────────

/// Hooks working directory: ~/.tpa-cowork/hooks/ (overflow files, env files).
pub fn hooks_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("hooks"))
}

// ── macOS Control ─────────────────────────────────────────────────

/// macOS control snapshot image directory:
/// ~/.tpa-cowork/mac-control/snapshots/
pub fn mac_control_snapshots_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("mac-control").join("snapshots"))
}

/// macOS control diagnostics bundle directory:
/// ~/.tpa-cowork/mac-control/diagnostics/
pub fn mac_control_diagnostics_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("mac-control").join("diagnostics"))
}

// ── Avatars ──────────────────────────────────────────────────────

/// Avatars directory: ~/.tpa-cowork/avatars/
pub fn avatars_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("avatars"))
}

// ── Logs ──────────────────────────────────────────────────────────

/// Logs database path: ~/.tpa-cowork/logs.db
pub fn logs_db_path() -> Result<PathBuf> {
    Ok(root_dir()?.join("logs.db"))
}

/// Logs directory for plain text log files: ~/.tpa-cowork/logs/
pub fn logs_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("logs"))
}

// ── Share ────────────────────────────────────────────────────────

/// Shared directory for inter-agent data: ~/.tpa-cowork/share/
#[allow(dead_code)]
pub fn share_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("share"))
}

/// Temporary large-object store for Chrome Extension native-messaging blobs:
/// ~/.tpa-cowork/browser-extension/blobs/
pub fn browser_extension_blobs_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("browser-extension").join("blobs"))
}

// ── Cron ────────────────────────────────────────────────────────

/// Cron database path: ~/.tpa-cowork/cron.db
pub fn cron_db_path() -> Result<PathBuf> {
    Ok(root_dir()?.join("cron.db"))
}

// ── Background Jobs ─────────────────────────────────────────────

/// Background jobs database path: ~/.tpa-cowork/background_jobs.db
/// (R1: was `async_jobs.db`; pure rebuildable cache, so the rename just points
/// at a fresh file — the legacy file is best-effort removed at startup.)
pub fn background_jobs_db_path() -> Result<PathBuf> {
    Ok(root_dir()?.join("background_jobs.db"))
}

/// Background jobs result spool directory: ~/.tpa-cowork/background_jobs/
pub fn background_jobs_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("background_jobs"))
}

/// Per-job result file: ~/.tpa-cowork/background_jobs/{job_id}.txt
pub fn background_job_result_path(job_id: &str) -> Result<PathBuf> {
    Ok(background_jobs_dir()?.join(format!("{}.txt", job_id)))
}

/// Legacy pre-R1 paths (`async_jobs.db` + `async_jobs/`), best-effort removed at
/// startup so the renamed cache doesn't leave orphans on disk. Not a migration —
/// the data is a rebuildable cache and is simply discarded.
pub fn legacy_async_jobs_db_path() -> Result<PathBuf> {
    Ok(root_dir()?.join("async_jobs.db"))
}

pub fn legacy_async_jobs_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("async_jobs"))
}

/// Local model install/pull jobs database path: ~/.tpa-cowork/local_model_jobs.db
pub fn local_model_jobs_db_path() -> Result<PathBuf> {
    Ok(root_dir()?.join("local_model_jobs.db"))
}

/// Agent self-scheduled wakeups database path: ~/.tpa-cowork/wakeups.db
///
/// Backs the `schedule_wakeup` tool (R10): one-shot timers that re-enter the
/// originating session after a delay. Rebuildable/transient — incognito
/// wakeups are never written here (close-and-burn), and unfired rows are
/// re-armed on the next Primary startup.
pub fn wakeups_db_path() -> Result<PathBuf> {
    Ok(root_dir()?.join("wakeups.db"))
}

/// Cached Ollama Library search/tag metadata: ~/.tpa-cowork/local_llm_library_cache.db
pub fn local_llm_library_cache_db_path() -> Result<PathBuf> {
    Ok(root_dir()?.join("local_llm_library_cache.db"))
}

// ── Recap ───────────────────────────────────────────────────────

/// Recap directory: ~/.tpa-cowork/recap/
pub fn recap_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("recap"))
}

/// Recap database path: ~/.tpa-cowork/recap/recap.db
pub fn recap_db_path() -> Result<PathBuf> {
    Ok(recap_dir()?.join("recap.db"))
}

/// Generated reports output directory: ~/.tpa-cowork/reports/
pub fn reports_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("reports"))
}

// ── Memory ──────────────────────────────────────────────────────

/// Memory database path: ~/.tpa-cowork/memory.db
pub fn memory_db_path() -> Result<PathBuf> {
    Ok(root_dir()?.join("memory.db"))
}

/// Embedding model cache directory: ~/.tpa-cowork/models/
pub fn models_cache_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("models"))
}

/// Dream Diary directory: ~/.tpa-cowork/memory/dreams/
/// Holds one markdown file per cycle (by default named with the local date),
/// created by the Dreaming Light pipeline (Phase B3).
pub fn dreams_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("memory").join("dreams"))
}

/// Memory attachments directory: ~/.tpa-cowork/memory_attachments/
pub fn memory_attachments_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("memory_attachments"))
}

// ── Browser Profiles ────────────────────────────────────────────

/// Browser profiles root directory: ~/.tpa-cowork/browser-profiles/
pub fn browser_profiles_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("browser-profiles"))
}

/// Specific browser profile directory: ~/.tpa-cowork/browser-profiles/{profile_name}/
pub fn browser_profile_dir(profile_name: &str) -> Result<PathBuf> {
    Ok(browser_profiles_dir()?.join(profile_name))
}

/// User-attach Chrome profile directory: ~/.tpa-cowork/browser/user-attach/
///
/// Used by the "Take over user Chrome" path in settings: hope-agent spawns
/// a Chrome instance pointed at this directory so the user's daily browsing
/// (their real `Default` / per-OS profile) is never touched.
pub fn browser_user_attach_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("browser").join("user-attach"))
}

/// Managed-launch Chrome user-data-dir: `~/.tpa-cowork/browser/managed-runner/`.
///
/// Used by `profile.op=launch target=managed` when no `profile` arg is
/// given. chromiumoxide's default behaviour is to pick a random `/tmp`
/// directory which makes SingletonLock observability impossible — a crashed
/// Chrome leaves a stale lock there and the next launch fails with
/// `File exists (17)`. Pinning a stable path lets
/// [`crate::browser::singleton_lock`] detect and clean stale locks.
pub fn browser_managed_runner_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("browser").join("managed-runner"))
}

/// Root for hope-agent–managed browser runtimes:
/// `~/.tpa-cowork/browser/runtime/`. Holds the unzipped Chromium snapshot
/// when the system has no Chrome / Edge / Brave / Chromium installed.
///
/// Pinned revisions live in [`crate::browser::runtime`] (per-platform
/// constants — Chromium snapshots build each OS independently, so a
/// single workspace-wide revision isn't representable).
pub fn browser_runtime_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("browser").join("runtime"))
}

/// Chrome Extension integration runtime directory:
/// `~/.tpa-cowork/browser-extension/`.
pub fn browser_extension_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("browser-extension"))
}

/// Discovery file read by the Native Messaging host to find the local Core
/// broker. Rebuildable runtime state, rewritten on broker startup.
pub fn browser_extension_broker_discovery_path() -> Result<PathBuf> {
    Ok(browser_extension_dir()?.join("broker.json"))
}

pub fn browser_extension_broker_socket_path() -> Result<PathBuf> {
    Ok(browser_extension_dir()?.join("broker.sock"))
}

pub fn browser_extension_registry_path() -> Result<PathBuf> {
    Ok(browser_extension_dir()?.join("registry.json"))
}

/// Stable copy of the unpacked browser extension for local ("Load unpacked")
/// install: `~/.tpa-cowork/extension/browser/`. The app bundle's own copy lives
/// inside the `.app` (or the platform resource dir) and its path changes when
/// the app is updated or moved; loading that path in Chrome would break on
/// update. This stable copy is what the user loads, so it survives app updates
/// (refreshed in place). Built with `join`, so the separator is correct on
/// Windows / Linux / macOS; the `extension/` parent leaves room for other
/// browser engines later (e.g. `extension/firefox`).
pub fn browser_extension_unpacked_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("extension").join("browser"))
}

/// Completion marker for the stable unpacked-extension copy:
/// `~/.tpa-cowork/extension/.browser-synced`. Written only after a FULL mirror
/// succeeds; readers treat the stable copy as usable only when this marker is
/// present, so a copy interrupted partway (crash / disk full) — which may have
/// `manifest.json` but be missing other files — never shadows the bundled
/// source with a broken extension. Lives beside `browser/` (not inside it) so
/// it is never pruned by the mirror and Chrome (which ignores dotfiles anyway)
/// never sees it as part of the loaded extension.
pub fn browser_extension_unpacked_marker() -> Result<PathBuf> {
    Ok(root_dir()?.join("extension").join(".browser-synced"))
}

/// Per-revision Chromium runtime directory:
/// `~/.tpa-cowork/browser/runtime/chromium-{revision}/`. Versioned so
/// bumping the per-platform pinned revision doesn't collide with an
/// older cached binary (old dirs can be hand-cleaned).
pub fn chromium_runtime_dir(revision: u32) -> Result<PathBuf> {
    Ok(browser_runtime_dir()?.join(format!("chromium-{revision}")))
}

// ── Generated Images ────────────────────────────────────────────────

/// Generated images directory: ~/.tpa-cowork/generated-images/
pub fn generated_images_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("generated-images"))
}

// ── Crash Journal ──────────────────────────────────────────────────

/// Crash journal file path: ~/.tpa-cowork/crash_journal.json
pub fn crash_journal_path() -> Result<PathBuf> {
    Ok(root_dir()?.join("crash_journal.json"))
}

// ── Desktop Window State ───────────────────────────────────────────

/// Desktop window state file path: ~/.tpa-cowork/window-state.json
pub fn window_state_path() -> Result<PathBuf> {
    Ok(root_dir()?.join("window-state.json"))
}

// ── Self-Update ─────────────────────────────────────────────────────

/// Self-update working directory: ~/.tpa-cowork/updater/
pub fn updater_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("updater"))
}

/// Per-version download staging directory: ~/.tpa-cowork/updater/staging/{version}/
pub fn updater_staging_dir(version: &str) -> Result<PathBuf> {
    Ok(updater_dir()?
        .join("staging")
        .join(sanitize_path_segment(version)))
}

/// Per-version backup directory: ~/.tpa-cowork/updater/backup/{version}/
/// Holds the prior binary so `app_update rollback` can restore it.
pub fn updater_backup_dir(version: &str) -> Result<PathBuf> {
    Ok(updater_dir()?
        .join("backup")
        .join(sanitize_path_segment(version)))
}

// ── Backups ────────────────────────────────────────────────────────

/// Backups directory: ~/.tpa-cowork/backups/
pub fn backups_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("backups"))
}

/// Automatic-snapshot directory for config / user_config changes:
/// ~/.tpa-cowork/backups/autosave/
pub fn autosave_dir() -> Result<PathBuf> {
    Ok(backups_dir()?.join("autosave"))
}

// ── Canvas ──────────────────────────────────────────────────────

/// Canvas root directory: ~/.tpa-cowork/canvas/
pub fn canvas_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("canvas"))
}

/// Canvas projects directory: ~/.tpa-cowork/canvas/projects/
pub fn canvas_projects_dir() -> Result<PathBuf> {
    Ok(canvas_dir()?.join("projects"))
}

/// Specific canvas project directory: ~/.tpa-cowork/canvas/projects/{id}/
pub fn canvas_project_dir(project_id: &str) -> Result<PathBuf> {
    if project_id.is_empty()
        || project_id.len() > 128
        || !project_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        anyhow::bail!("invalid canvas project id");
    }
    Ok(canvas_projects_dir()?.join(project_id))
}

/// Canvas database path: ~/.tpa-cowork/canvas/canvas.db
pub fn canvas_db_path() -> Result<PathBuf> {
    Ok(canvas_dir()?.join("canvas.db"))
}

// ── Design Space ────────────────────────────────────────────────

/// Design Space root directory: ~/.tpa-cowork/design/
pub fn design_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("design"))
}

/// Design database path: ~/.tpa-cowork/design/design.db
pub fn design_db_path() -> Result<PathBuf> {
    Ok(design_dir()?.join("design.db"))
}

/// Design systems directory: ~/.tpa-cowork/design/systems/
pub fn design_systems_dir() -> Result<PathBuf> {
    Ok(design_dir()?.join("systems"))
}

/// Specific design system directory: ~/.tpa-cowork/design/systems/{id}/
pub fn design_system_dir(system_id: &str) -> Result<PathBuf> {
    Ok(design_systems_dir()?.join(system_id))
}

/// Design projects directory: ~/.tpa-cowork/design/projects/
pub fn design_projects_dir() -> Result<PathBuf> {
    Ok(design_dir()?.join("projects"))
}

/// Specific design project directory: ~/.tpa-cowork/design/projects/{id}/
pub fn design_project_dir(project_id: &str) -> Result<PathBuf> {
    Ok(design_projects_dir()?.join(project_id))
}

/// Specific design artifact directory:
/// ~/.tpa-cowork/design/projects/{pid}/artifacts/{aid}/
pub fn design_artifact_dir(project_id: &str, artifact_id: &str) -> Result<PathBuf> {
    Ok(design_project_dir(project_id)?
        .join("artifacts")
        .join(artifact_id))
}

// ── Projects ────────────────────────────────────────────────────

/// Projects root directory: ~/.tpa-cowork/projects/
pub fn projects_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("projects"))
}

/// Specific project directory: ~/.tpa-cowork/projects/{id}/
pub fn project_dir(project_id: &str) -> Result<PathBuf> {
    Ok(projects_dir()?.join(project_id))
}

/// Default project workspace directory: ~/.tpa-cowork/projects/{id}/workspace/
///
/// Used as the per-project working directory when the user has not selected an
/// explicit one. Created lazily on first resolution (see
/// `session::helpers::effective_session_working_dir`); never written into the
/// DB so the `~/.tpa-cowork` tree stays relocatable via `HA_DATA_DIR`.
pub fn project_workspace_dir(project_id: &str) -> Result<PathBuf> {
    Ok(project_dir(project_id)?.join("workspace"))
}

// ── Knowledge Base ──────────────────────────────────────────────

/// Knowledge base root directory: ~/.tpa-cowork/knowledge/
pub fn knowledge_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join("knowledge"))
}

/// Global knowledge index database: ~/.tpa-cowork/knowledge/index.db
///
/// Pure rebuildable cache (note / note_chunk / note_link / note_tag + FTS5 +
/// vec). Deleting it loses nothing — the `.md` files + the `knowledge_bases`
/// registry in `sessions.db` are the single source of truth.
pub fn knowledge_index_db_path() -> Result<PathBuf> {
    Ok(knowledge_dir()?.join("index.db"))
}

/// Per-KB default notes directory: ~/.tpa-cowork/knowledge/{kb_id}/notes/
///
/// Used when a knowledge base's `root_dir` is NULL (internal, app-managed).
/// Created lazily on first resolution (see `knowledge::resolve_kb_dir`); never
/// written into the DB so the `~/.tpa-cowork` tree stays relocatable via
/// `HA_DATA_DIR`. A non-NULL `root_dir` points at an external vault instead.
pub fn knowledge_kb_notes_dir(kb_id: &str) -> Result<PathBuf> {
    Ok(knowledge_dir()?
        .join(sanitize_path_segment(kb_id))
        .join("notes"))
}

/// Per-KB raw-source directory: ~/.tpa-cowork/knowledge/{kb_id}/sources/
///
/// Raw sources are Hope-managed even for external/bound vaults. This keeps the
/// "raw inbox" writable without mutating a user's external notes folder and
/// preserves D11's default read-only posture for bound vaults.
pub fn knowledge_kb_sources_dir(kb_id: &str) -> Result<PathBuf> {
    Ok(knowledge_dir()?
        .join(sanitize_path_segment(kb_id))
        .join("sources"))
}

// ── Plans ───────────────────────────────────────────────────────

/// Plans directory: uses custom `plansDirectory` config if set,
/// otherwise `~/.tpa-cowork/plans/`.
pub fn plans_dir() -> Result<PathBuf> {
    let store = crate::config::cached_config();
    if let Some(ref custom_dir) = store.plans_directory {
        if !custom_dir.is_empty() {
            let expanded = if custom_dir.starts_with('~') {
                if let Some(home) = dirs::home_dir() {
                    let suffix = custom_dir
                        .strip_prefix("~/")
                        .or_else(|| custom_dir.strip_prefix("~"))
                        .unwrap_or(custom_dir);
                    if suffix.is_empty() {
                        home
                    } else {
                        home.join(suffix)
                    }
                } else {
                    PathBuf::from(custom_dir)
                }
            } else {
                PathBuf::from(custom_dir)
            };
            return Ok(expanded);
        }
    }
    Ok(root_dir()?.join("plans"))
}

/// Per-session plan directory: `<plans_dir>/<agent_id>/<session_id>/`.
///
/// Two-level isolation: keeps each session's plan files (current + version
/// backups + result) physically separate so a model `ls`-ing the plans dir
/// can only see its own work. Grouping by agent first means historical
/// plans are also browseable per agent (handy for export / archival).
///
/// Both `agent_id` and `session_id` are sanitized to bare alphanumerics +
/// `-` / `_` to defang any path-traversal attempt from upstream — session
/// ids are UUIDs and agent ids are slug-validated, so this is defense in
/// depth, not the primary boundary.
pub fn session_plans_dir(agent_id: &str, session_id: &str) -> Result<PathBuf> {
    validate_agent_id(agent_id)?;
    Ok(plans_dir()?
        .join(sanitize_path_segment(agent_id))
        .join(sanitize_path_segment(session_id)))
}

/// Sanitize an untrusted id (agent / session / version / kb) into a bare path
/// segment: ASCII alphanumerics plus `-` / `_`, with everything else (including
/// `.` and `/`) collapsed to `_`, defanging `..` / separator traversal. Shared
/// by `paths.rs`, `tools::execution` (large-result spill + `tool_results` purge)
/// and `tools::image_markers` (materialized vision files) so all three derive
/// the same `tool_results/<segment>/` directory for a given session — otherwise
/// materialization and purge can diverge into different directories.
pub(crate) fn sanitize_path_segment(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

const AGENT_VENV_SENTINEL: &str = ".tpa-cowork-venv-complete";

pub fn agent_venv_bin_dir() -> Option<PathBuf> {
    if let Some(bin) = find_agent_venv_bin_dir() {
        return Some(bin);
    }

    #[cfg(windows)]
    {
        if let Err(error) = ensure_agent_venv_extracted() {
            eprintln!("[agent-venv] recovery extract failed: {error}");
        }
        return find_agent_venv_bin_dir();
    }

    #[cfg(not(windows))]
    None
}

fn find_agent_venv_bin_dir() -> Option<PathBuf> {
    for key in ["TPA_COWORK_VENV_DIR", "HOPE_AGENT_VENV_DIR"] {
        if let Ok(value) = std::env::var(key) {
            let root = PathBuf::from(value.trim());
            if let Some(bin) = usable_agent_venv_bin(&root, false) {
                return Some(bin);
            }
        }
    }

    if let Ok(executable) = std::env::current_exe() {
        if let Some(executable_dir) = executable.parent() {
            let mut candidates = vec![
                executable_dir.join("agent-venv"),
                executable_dir.join("resources").join("agent-venv"),
            ];
            if let Some(parent) = executable_dir.parent() {
                candidates.push(parent.join("Resources").join("agent-venv"));
            }
            for candidate in candidates {
                if let Some(bin) = usable_agent_venv_bin(&candidate, true) {
                    return Some(bin);
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    {
        if let Ok(current_dir) = std::env::current_dir() {
            for base in current_dir.ancestors().take(6) {
                if let Some(bin) = usable_agent_venv_bin(&base.join("agent-venv"), false) {
                    return Some(bin);
                }
            }
        }
    }

    None
}

fn usable_agent_venv_bin(root: &Path, require_sentinel: bool) -> Option<PathBuf> {
    let bin = agent_venv_scripts_dir(root);
    let python = if cfg!(windows) {
        bin.join("python.exe")
    } else {
        bin.join("python")
    };
    if !python.is_file() {
        return None;
    }
    if require_sentinel && !root.join(AGENT_VENV_SENTINEL).is_file() {
        return None;
    }
    Some(bin)
}

fn agent_venv_scripts_dir(root: &Path) -> PathBuf {
    if cfg!(windows) {
        root.join("Scripts")
    } else {
        root.join("bin")
    }
}

#[cfg(windows)]
pub fn ensure_agent_venv_extracted() -> Result<()> {
    use std::sync::Mutex;

    static EXTRACT_LOCK: Mutex<()> = Mutex::new(());
    let _guard = EXTRACT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if find_agent_venv_bin_dir().is_some() {
        return Ok(());
    }

    let executable = std::env::current_exe().context("resolve executable for agent-venv")?;
    let executable_dir = executable
        .parent()
        .ok_or_else(|| anyhow::anyhow!("current executable has no parent directory"))?;
    let archive = [
        executable_dir.join("resources").join("agent-venv.zip"),
        executable_dir.join("agent-venv.zip"),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .ok_or_else(|| anyhow::anyhow!("agent-venv.zip is not available"))?;

    extract_agent_venv_zip(&archive, executable_dir)?;
    fs::remove_file(&archive)
        .with_context(|| format!("remove extracted archive {}", archive.display()))?;
    Ok(())
}

#[cfg(windows)]
fn extract_agent_venv_zip(archive: &Path, target_dir: &Path) -> Result<()> {
    use std::io::Write;

    fs::create_dir_all(target_dir)?;
    let staging = tempfile::Builder::new()
        .prefix(".agent-venv-staging-")
        .tempdir_in(target_dir)
        .with_context(|| format!("create agent-venv staging in {}", target_dir.display()))?;
    let file = fs::File::open(archive)
        .with_context(|| format!("open agent-venv archive {}", archive.display()))?;
    let mut zip = zip::ZipArchive::new(file)
        .with_context(|| format!("read agent-venv archive {}", archive.display()))?;

    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let relative = entry.mangled_name();
        if relative.as_os_str().is_empty() {
            continue;
        }
        if !relative.starts_with("agent-venv") {
            anyhow::bail!("agent-venv archive contains an unexpected root entry");
        }
        let output = staging.path().join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output)?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut destination = fs::File::create(&output)?;
        std::io::copy(&mut entry, &mut destination)?;
        destination.flush()?;
    }

    let staged_root = staging.path().join("agent-venv");
    let staged_python = agent_venv_scripts_dir(&staged_root).join("python.exe");
    if !staged_python.is_file() {
        anyhow::bail!("agent-venv archive is missing Scripts/python.exe");
    }
    fs::write(staged_root.join(AGENT_VENV_SENTINEL), b"complete\n")?;

    let final_root = target_dir.join("agent-venv");
    let previous_root = staging.path().join("previous-agent-venv");
    let had_previous = path_exists(&final_root);
    if had_previous {
        let metadata = fs::symlink_metadata(&final_root)?;
        if !metadata.is_dir() {
            anyhow::bail!("existing agent-venv path is not a directory");
        }
        fs::rename(&final_root, &previous_root)?;
    }

    if let Err(error) = fs::rename(&staged_root, &final_root) {
        if had_previous {
            let _ = fs::rename(&previous_root, &final_root);
        }
        return Err(error).context("promote extracted agent-venv");
    }

    Ok(())
}

// ── Directory Initialization ──────────────────────────────────────

/// Ensure all required directories exist.
pub fn ensure_dirs() -> Result<()> {
    migrate_legacy_data_dir()?;
    let dirs_to_create = [
        root_dir()?,
        credentials_dir()?,
        channels_dir()?,
        skills_dir()?,
        agents_dir()?,
        home_dir()?,
        avatars_dir()?,
        share_dir()?,
        logs_dir()?,
        models_cache_dir()?,
        browser_profiles_dir()?,
        browser_extension_dir()?,
        backups_dir()?,
        generated_images_dir()?,
        canvas_dir()?,
        canvas_projects_dir()?,
        projects_dir()?,
        plans_dir()?,
        recap_dir()?,
        reports_dir()?,
        background_jobs_dir()?,
        knowledge_dir()?,
    ];
    for dir in &dirs_to_create {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{migrate_legacy_data, root_dir, validate_agent_id};
    use std::path::Path;

    #[test]
    fn tpa_data_dir_takes_precedence_over_ha_data_dir() {
        let tpa = std::path::PathBuf::from("C:/tpa-data");
        let ha = std::path::PathBuf::from("C:/ha-data");
        crate::test_support::with_env_vars(
            &[("TPA_DATA_DIR", &tpa), ("HA_DATA_DIR", &ha)],
            || assert_eq!(root_dir().expect("root_dir"), tpa),
        );
    }

    #[test]
    fn default_root_uses_tpa_cowork_directory() {
        crate::test_support::with_env_vars(
            &[("TPA_DATA_DIR", Path::new("")), ("HA_DATA_DIR", Path::new(""))],
            || {
                let home = dirs::home_dir().expect("home directory");
                assert_eq!(root_dir().expect("root_dir"), home.join(".tpa-cowork"));
            },
        );
    }

    #[test]
    fn agent_id_rejects_path_segments_and_absolute_paths() {
        for invalid in ["", ".", "..", "a/b", "a\\b", "/tmp/agent"] {
            assert!(validate_agent_id(invalid).is_err(), "accepted {invalid:?}");
        }
        for valid in ["ha-main", "researcher", "Agent2", "agent_name"] {
            assert!(validate_agent_id(valid).is_ok(), "rejected {valid:?}");
        }
    }

    #[test]
    fn legacy_only_migration_moves_nested_files() {
        let root = tempfile::tempdir().expect("tempdir");
        let source = root.path().join("legacy");
        let destination = root.path().join("current");
        std::fs::create_dir_all(source.join("nested")).expect("nested source dir");
        std::fs::write(source.join("config.json"), b"config").expect("config");
        std::fs::write(source.join("nested/data.db"), b"data").expect("nested data");

        migrate_legacy_data(&source, &destination).expect("migration succeeds");

        assert!(!source.exists());
        assert_eq!(std::fs::read(destination.join("config.json")).unwrap(), b"config");
        assert_eq!(std::fs::read(destination.join("nested/data.db")).unwrap(), b"data");
    }

    #[test]
    fn migration_leaves_both_directories_untouched() {
        let root = tempfile::tempdir().expect("tempdir");
        let source = root.path().join("legacy");
        let destination = root.path().join("current");
        std::fs::create_dir_all(&source).expect("source");
        std::fs::create_dir_all(&destination).expect("destination");
        std::fs::write(source.join("legacy.txt"), b"legacy").expect("legacy file");
        std::fs::write(destination.join("current.txt"), b"current").expect("current file");

        migrate_legacy_data(&source, &destination).expect("migration succeeds");

        assert_eq!(std::fs::read(source.join("legacy.txt")).unwrap(), b"legacy");
        assert_eq!(std::fs::read(destination.join("current.txt")).unwrap(), b"current");
    }

    #[test]
    fn copy_migration_verifies_and_promotes_nested_files() {
        let root = tempfile::tempdir().expect("tempdir");
        let source = root.path().join("legacy");
        let destination = root.path().join("current");
        std::fs::create_dir_all(source.join("nested")).expect("nested source dir");
        std::fs::write(source.join("nested/data.db"), vec![b'x'; 128 * 1024])
            .expect("nested data");

        super::migrate_legacy_data_by_copy(&source, &destination)
            .expect("copy migration succeeds");

        assert!(!source.exists());
        assert_eq!(
            std::fs::read(destination.join("nested/data.db")).unwrap(),
            vec![b'x'; 128 * 1024]
        );
    }

    #[test]
    fn failed_migration_preserves_source_and_does_not_create_partial_destination() {
        let root = tempfile::tempdir().expect("tempdir");
        let source = root.path().join("legacy");
        let destination_parent = root.path().join("not-a-directory");
        let destination = destination_parent.join("current");
        std::fs::create_dir_all(&source).expect("source");
        std::fs::write(source.join("important.txt"), b"important").expect("source file");
        std::fs::write(&destination_parent, b"block destination parent").expect("blocker");

        assert!(migrate_legacy_data(&source, &destination).is_err());
        assert_eq!(std::fs::read(source.join("important.txt")).unwrap(), b"important");
        assert!(!destination.exists());
    }

    #[cfg(windows)]
    #[test]
    fn agent_venv_zip_extracts_atomically_and_writes_completion_marker() {
        use std::io::Write;

        let root = tempfile::tempdir().expect("tempdir");
        let zip_path = root.path().join("agent-venv.zip");
        let file = std::fs::File::create(&zip_path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.add_directory("agent-venv/", options).expect("venv dir");
        zip.add_directory("agent-venv/Scripts/", options)
            .expect("scripts dir");
        zip.start_file("agent-venv/Scripts/python.exe", options)
            .expect("python entry");
        zip.write_all(b"fake-python").expect("python bytes");
        zip.finish().expect("finish zip");

        super::extract_agent_venv_zip(&zip_path, root.path()).expect("extract venv");

        let venv = root.path().join("agent-venv");
        assert!(venv.join("Scripts/python.exe").is_file());
        assert!(venv.join(super::AGENT_VENV_SENTINEL).is_file());
        assert!(!root.path().join(".agent-venv-staging").exists());
    }
}
