use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};
use tauri::{
    menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WindowEvent,
};

#[derive(Debug, thiserror::Error)]
enum DesktopError {
    #[error("mountos CLI not found in PATH")]
    CliNotFound,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("keychain error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("tauri path error: {0}")]
    Path(#[from] tauri::Error),
    #[error("{0}")]
    Message(String),
}

impl Serialize for DesktopError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Backend {
    Auto,
    Macfuse,
    Fskit,
    Nfs,
    Smb,
    Fileprovider,
    Mountosio,
    Cloudfilter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MountProfile {
    id: String,
    schema_version: u32,
    kind: String,
    name: String,
    volume: String,
    fork: String,
    mount_path: String,
    discovery_url: String,
    access_key_id: String,
    secret_ref: String,
    backend: Backend,
    cache_dir: Option<String>,
    read_only: bool,
    auto_remount: bool,
    // Added after the original schema; #[serde(default)] so profiles saved
    // before this field existed still deserialize (bool has no implicit
    // default the way Option<T> does).
    #[serde(default)]
    temporary_fork: bool,
    trusted_discovery_host: Option<String>,
    extra_args: Vec<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MountInstance {
    key: String,
    name: String,
    mount_path: String,
    fs_name: Option<String>,
    // The transport the mount actually runs on (macfuse/fskit/nfs/smb/
    // fileprovider/mountosio/cloudfilter), as reported by `mountos list`.
    // fs_name is the device string ("mountos:<volume>") and says nothing about
    // the backend, so it is not a stand-in for this.
    backend: Option<String>,
    view_mode: Option<String>,
    project_volume_id: Option<String>,
    volume_id: Option<u32>,
    domain_id: Option<String>,
    unc_path: Option<String>,
    version_inode: Option<String>,
    orphaned: Option<bool>,
    external: bool,
    // Id of the saved profile whose mount path matches this mount, if any.
    // external is simply this being absent.
    profile_id: Option<String>,
    health: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckIssue {
    id: String,
    severity: String,
    title: String,
    detail: Option<String>,
    fix_command: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemState {
    platform: String,
    cli_path: Option<String>,
    cli_version: Option<String>,
    check_ok: bool,
    issues: Vec<CheckIssue>,
    instances: Vec<MountInstance>,
    // Other mountos binaries found on PATH besides the one actually in use
    // (empty when there's exactly one, the common case). Surfaces ambiguity
    // instead of silently trusting whichever which() happened to resolve.
    cli_path_alternates: Vec<String>,
    // Terminal emulators detected on this machine, in preference order. The
    // settings picker lists exactly these, so it can only ever offer a
    // terminal that is actually installed.
    terminals: Vec<TerminalOption>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalOption {
    id: String,
    label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SecretStatus {
    profile_id: String,
    stored: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsCommandOutput {
    status: Option<i32>,
    stdout: Value,
    stderr: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsProfileSummary {
    id: String,
    name: String,
    kind: String,
    mount_path: String,
    discovery_url: String,
    backend: Backend,
    secret_ref: String,
    extra_args_count: usize,
    auto_remount: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsContent {
    created_at_unix: u64,
    cli_path: Option<String>,
    cli_version: Option<String>,
    check: Option<DiagnosticsCommandOutput>,
    list: Option<DiagnosticsCommandOutput>,
    profiles: Vec<DiagnosticsProfileSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsBundle {
    path: String,
    content: DiagnosticsContent,
}

#[derive(Debug, Serialize)]
struct ExportedProfile {
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopSettings {
    default_backend: Backend,
    // How often the mount list is refreshed while the window is visible, in
    // seconds. None means the default. A hidden window always backs off
    // regardless (see the poll effect).
    poll_seconds: Option<u32>,
    // Terminal emulator id for the dashboard launcher (see KNOWN_TERMINALS).
    // None means the platform's stock terminal. A pinned terminal that is no
    // longer installed silently falls back rather than failing: it is a
    // preference, not a hard pin like cli_path_override.
    terminal: Option<String>,
    // Seeds new profiles' discoveryUrl; each profile can still override it
    // independently afterward. Existing profiles are never retroactively
    // rewritten when this changes. Option<T> deserializes missing older
    // settings.json files (pre-dating this field) as None automatically.
    default_discovery_url: Option<String>,
    // Pins an exact mountos binary instead of the first PATH match. Once
    // set, a moved/missing pinned binary is a hard error (see
    // mountos_path) rather than a silent fallback to a different install.
    cli_path_override: Option<String>,
}

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            default_backend: Backend::Auto,
            default_discovery_url: None,
            cli_path_override: None,
            poll_seconds: None,
            terminal: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct MountResult {
    state: String,
    target: String,
}

#[derive(Debug, Serialize)]
struct UnmountResult {
    state: String,
    target: String,
}

#[derive(Debug, Serialize)]
struct UnmountAllResult {
    attempted: usize,
    failed: Vec<String>,
}

const KEYRING_SERVICE: &str = "sh.mountos.desktop";
const ACCESS_KEY_ID_LENGTH: usize = 20;
const LAUNCH_TIMEOUT: Duration = Duration::from_secs(65);
#[cfg(windows)]
const WINDOWS_READY_TIMEOUT: Duration = Duration::from_secs(60);
const INDETERMINATE_TIMEOUT: Duration = Duration::from_secs(120);
const UNMOUNT_TIMEOUT: Duration = Duration::from_secs(120);

fn keyring_entry(profile_id: &str) -> Result<keyring::Entry, DesktopError> {
    validate_profile_id(profile_id)?;
    Ok(keyring::Entry::new(
        KEYRING_SERVICE,
        &format!("profile/{profile_id}"),
    )?)
}

fn read_profile_secret(
    profile: &MountProfile,
    provided: Option<String>,
) -> Result<Option<String>, DesktopError> {
    if let Some(secret) = provided {
        return Ok(Some(secret));
    }
    if profile.secret_ref == "vault" {
        return Ok(Some(keyring_entry(&profile.id)?.get_password()?));
    }
    Ok(None)
}

static CLI_PATH_OVERRIDE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

fn cli_path_override_cell() -> &'static Mutex<Option<PathBuf>> {
    CLI_PATH_OVERRIDE.get_or_init(|| Mutex::new(None))
}

// Populated from DesktopSettings.cli_path_override whenever settings are
// read or saved (get_settings/save_settings), not read from disk on every
// mountos_path() call. There is a small startup race: the very first
// get_system_state poll can land before the frontend's initial
// loadSettings() call finishes, using the plain PATH lookup for one cycle.
// Self-corrects on the next poll (5s) and every settings.json write races
// nothing since it happens strictly after this function is populated.
fn set_cli_path_override(path: Option<PathBuf>) {
    *cli_path_override_cell().lock().unwrap_or_else(|e| e.into_inner()) = path;
}

fn cli_path_override() -> Option<PathBuf> {
    cli_path_override_cell().lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn mountos_path() -> Result<PathBuf, DesktopError> {
    if let Some(pinned) = cli_path_override() {
        return if pinned.is_file() {
            Ok(pinned)
        } else {
            // Fail loudly rather than silently falling back to a different
            // PATH match — that would defeat the point of pinning a path.
            Err(DesktopError::Message(format!(
                "pinned CLI path no longer exists: {}",
                pinned.display()
            )))
        };
    }
    which::which("mountos").map_err(|_| DesktopError::CliNotFound)
}

// Every mountos binary on PATH other than the one actually in use. which
// 8.0.4's which_all() walks the full PATH; used to warn about ambiguous
// installs (e.g. a stale dev build shadowing a real one) rather than
// silently trusting whichever happens to resolve first.
fn other_cli_paths_on_path(resolved: Option<&Path>) -> Vec<String> {
    let Ok(all) = which::which_all("mountos") else {
        return Vec::new();
    };
    all.filter(|p| resolved.is_none_or(|r| p != r))
        .map(|p| p.display().to_string())
        .collect()
}

fn profile_dir(app: &AppHandle) -> Result<PathBuf, DesktopError> {
    let dir = app.path().app_config_dir()?.join("profiles");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn profile_path(app: &AppHandle, id: &str) -> Result<PathBuf, DesktopError> {
    validate_profile_id(id)?;
    Ok(profile_dir(app)?.join(format!("{id}.json")))
}

fn validate_profile_id(id: &str) -> Result<(), DesktopError> {
    if !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        Ok(())
    } else {
        Err(DesktopError::Message("invalid profile id".to_string()))
    }
}

fn read_profiles(app: &AppHandle) -> Result<Vec<MountProfile>, DesktopError> {
    let mut profiles: Vec<MountProfile> = Vec::new();
    for entry in fs::read_dir(profile_dir(app)?)? {
        let entry = entry?;
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let bytes = fs::read(entry.path())?;
        profiles.push(serde_json::from_slice(&bytes)?);
    }
    profiles.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(profiles)
}

// SECURITY BOUNDARY, hand-synced against a separate repo. These two flag
// sets (and their TS mirrors in src/lib/cli.ts) are what stops a profile's
// "extra args" field from overriding security-sensitive managed flags
// (discovery URL, credentials, mount target). They must be updated whenever
// mountos-servers' cmd/mfuse CLI adds a new flag — especially a new
// value-taking SHORT flag, since validate_extra_args's short-cluster scan
// only special-cases '-o' as a value-absorbing flag; any other new
// value-taking short flag would need the same treatment or it risks being
// silently misparsed. No automated check currently catches drift here.
fn managed_flags() -> HashSet<&'static str> {
    [
        "a",
        "access-key-id",
        "s",
        "secret-access-key",
        "discovery-url",
        "m",
        "mount",
        "mount-point",
        "f",
        "foreground",
        "gateway",
        "gateway-only",
        "fork-name",
        "volname",
        "n",
        "read-only",
        "r",
        "disk-cache-dir",
        "backend",
        "macfuse",
        "fskit",
        "k",
        "nfs",
        "N",
        "smb",
        "fileprovider",
        "F",
        "temporary-fork",
    ]
    .into_iter()
    .collect()
}

// gateway-* flags are deliberately excluded: validate_extra_args rejects any
// long flag starting with "gateway-" outright (see the `name.starts_with
// ("gateway-")` check below), regardless of what's listed here, since
// gateway profiles aren't supported yet (save_profile hard-rejects any kind
// other than "mount"). Revisit once gateway profile support ships.
fn boolean_long_flags() -> HashSet<&'static str> {
    [
        "acl",
        "agent",
        "blockserv-auto-degrade",
        "browse",
        "debug",
        "disable-cache-dir",
        "ioctl",
        "null-permissions",
        "session-audit",
        "xattr",
    ]
    .into_iter()
    .collect()
}

fn validate_extra_args(args: &[String]) -> Vec<String> {
    let managed = managed_flags();
    let boolean_flags = boolean_long_flags();
    let mut rejected = Vec::new();

    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if !arg.starts_with('-') || arg == "--" {
            rejected.push(arg.clone());
            index += 1;
            continue;
        }
        if let Some(long) = arg.strip_prefix("--") {
            let name = long.split('=').next().unwrap_or(long);
            if managed.contains(name) || name.starts_with("gateway-") {
                rejected.push(arg.clone());
                if !arg.contains('=')
                    && args
                        .get(index + 1)
                        .is_some_and(|value| !value.starts_with('-'))
                {
                    index += 1;
                    rejected.push(args[index].clone());
                }
            } else if !arg.contains('=')
                && !boolean_flags.contains(name)
                && args
                    .get(index + 1)
                    .is_some_and(|value| !value.starts_with('-'))
            {
                index += 1;
            }
            index += 1;
            continue;
        }
        for ch in arg.trim_start_matches('-').chars() {
            if managed.contains(ch.to_string().as_str()) {
                rejected.push(arg.clone());
                break;
            }
            if ch == 'o' {
                break;
            }
        }
        index += 1;
    }

    rejected
}

// backend_needs_mount_path reports whether `backend` uses a real,
// user-chosen filesystem path at all. FileProvider and CloudFilter mounts
// are entirely OS-managed (FileProvider via NSFileProviderManager, keyed by
// --name / the volume's Finder display name; CloudFilter's own CLI-side
// mount-point requirement is waived identically, cmd/mfuse/cmd_mount.go)
// — mountos never reads -m for either, so passing one is meaningless.
fn backend_needs_mount_path(backend: &Backend) -> bool {
    !matches!(backend, Backend::Fileprovider | Backend::Cloudfilter)
}

fn build_mount_argv(profile: &MountProfile) -> Vec<String> {
    let mut argv = vec!["mount".to_string()];
    if !profile.discovery_url.is_empty() {
        argv.extend(["--discovery-url".to_string(), profile.discovery_url.clone()]);
    }
    if !profile.volume.is_empty() {
        argv.extend(["--volname".to_string(), profile.volume.clone()]);
    }
    if !profile.fork.is_empty() {
        argv.extend(["--fork-name".to_string(), profile.fork.clone()]);
    }
    if backend_needs_mount_path(&profile.backend) && !profile.mount_path.is_empty() {
        argv.extend(["-m".to_string(), profile.mount_path.clone()]);
    }
    if !profile.access_key_id.is_empty() {
        argv.extend([
            "-a".to_string(),
            profile.access_key_id.clone(),
            "-s".to_string(),
        ]);
    }
    if profile.read_only {
        argv.push("--read-only".to_string());
    }
    if profile.temporary_fork {
        argv.push("--temporary-fork".to_string());
    }
    if let Some(cache_dir) = &profile.cache_dir {
        if !cache_dir.is_empty() {
            argv.extend(["--disk-cache-dir".to_string(), cache_dir.clone()]);
        }
    }
    match profile.backend {
        Backend::Auto => {}
        Backend::Macfuse => argv.push("--macfuse".to_string()),
        Backend::Fskit => argv.push("--fskit".to_string()),
        Backend::Nfs => argv.push("--nfs".to_string()),
        Backend::Smb => argv.push("--smb".to_string()),
        Backend::Fileprovider => argv.push("--fileprovider".to_string()),
        Backend::Mountosio => argv.extend(["--backend".to_string(), "mountosio".to_string()]),
        Backend::Cloudfilter => argv.extend(["--backend".to_string(), "cloudfilter".to_string()]),
    }
    argv.extend(profile.extra_args.clone());
    argv
}

fn validate_backend_for_platform(backend: &Backend) -> Result<(), DesktopError> {
    let valid = match std::env::consts::OS {
        "macos" => matches!(
            backend,
            Backend::Auto
                | Backend::Macfuse
                | Backend::Fskit
                | Backend::Nfs
                | Backend::Smb
                | Backend::Fileprovider
        ),
        "windows" => matches!(
            backend,
            Backend::Auto | Backend::Mountosio | Backend::Cloudfilter
        ),
        _ => matches!(backend, Backend::Auto | Backend::Nfs),
    };
    if valid {
        Ok(())
    } else {
        Err(DesktopError::Message(format!(
            "backend is unavailable on {}",
            std::env::consts::OS
        )))
    }
}

// FSKit requires its mount point to live under this exact directory — the
// kernel-side FSKit extension registration only resolves volumes rooted
// here, mirroring the same constraint the mos-sanity/mos-tests skills
// already document for manual FSKit testing.
const FSKIT_MOUNT_PREFIX: &str = "/Volumes/MountOS/";

fn validate_mount_path_for_backend(
    backend: &Backend,
    mount_path: &str,
) -> Result<(), DesktopError> {
    // Empty stays legal for backends that need a path: build_mount_argv omits
    // -m entirely in that case and the mountos CLI picks its own default.
    // What's rejected here is a NON-empty value that isn't a real absolute
    // path for this OS (Unix "/..." or a Windows drive-letter path) — e.g.
    // a relative path or garbage typed into the field.
    if backend_needs_mount_path(backend) && !mount_path.is_empty() && !is_openable_target(mount_path) {
        return Err(DesktopError::Message(format!(
            "mount path must be an absolute filesystem path, got {mount_path:?}"
        )));
    }
    if matches!(backend, Backend::Fskit) {
        let trimmed = mount_path.trim_end_matches('/');
        if trimmed.is_empty() || !(trimmed.starts_with(FSKIT_MOUNT_PREFIX) && trimmed.len() > FSKIT_MOUNT_PREFIX.len())
        {
            return Err(DesktopError::Message(format!(
                "FSKit requires a mount point under {FSKIT_MOUNT_PREFIX}<name>, got {mount_path:?}"
            )));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn resolve_auto_backend(profile: &mut MountProfile) -> Result<(), DesktopError> {
    if !matches!(profile.backend, Backend::Auto) {
        return Ok(());
    }
    let output = command_output(&["check", "--json"])?;
    let value = serde_json::from_slice::<Value>(&output.stdout)?;
    let outputs = value.as_array().cloned().unwrap_or_else(|| vec![value]);
    let usable = |feature: &str| {
        outputs.iter().any(|item| {
            item.get("feature")
                .and_then(Value::as_str)
                .is_some_and(|name| name.eq_ignore_ascii_case(feature))
                && (item
                    .get("supported")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                    || item
                        .get("capable")
                        .and_then(Value::as_bool)
                        .unwrap_or(false))
        })
    };
    profile.backend = if usable("MountOsIo") {
        Backend::Mountosio
    } else if usable("CloudFilter") {
        Backend::Cloudfilter
    } else {
        return Err(DesktopError::Message(
            "no usable Windows mount backend was reported by mountos check".to_string(),
        ));
    };
    Ok(())
}

#[cfg(not(windows))]
fn resolve_auto_backend(_profile: &mut MountProfile) -> Result<(), DesktopError> {
    Ok(())
}

fn command_output(args: &[&str]) -> Result<std::process::Output, DesktopError> {
    let path = mountos_path()?;
    Ok(Command::new(path).args(args).output()?)
}

fn normalized_target(target: &str) -> String {
    let target = fs::canonicalize(target)
        .unwrap_or_else(|_| PathBuf::from(target))
        .to_string_lossy()
        .trim_end_matches(['/', '\\'])
        .to_string();
    #[cfg(windows)]
    {
        target.to_ascii_lowercase()
    }
    #[cfg(not(windows))]
    {
        target
    }
}

fn targets_equal(left: &str, right: &str) -> bool {
    normalized_target(left) == normalized_target(right)
}

fn is_openable_target(target: &str) -> bool {
    if PathBuf::from(target).is_absolute() {
        return true;
    }
    #[cfg(windows)]
    {
        let bytes = target.as_bytes();
        return bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn list_contains_target(target: &str) -> Result<bool, DesktopError> {
    let output = command_output(&["list", "--json"])?;
    if !output.status.success() {
        return Err(DesktopError::Message(format!(
            "mountos list failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let value = serde_json::from_slice::<Value>(&output.stdout)?;
    let entries = value
        .as_array()
        .cloned()
        .or_else(|| value.get("mounts").and_then(Value::as_array).cloned())
        .unwrap_or_default();
    Ok(entries.iter().any(|entry| {
        entry
            .get("mountPath")
            .and_then(Value::as_str)
            .is_some_and(|candidate| targets_equal(candidate, target))
            || entry
                .get("domainId")
                .and_then(Value::as_str)
                .is_some_and(|candidate| candidate == target)
    }))
}

fn poll_target(target: &str, expected_present: bool, timeout: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if list_contains_target(target).is_ok_and(|present| present == expected_present) {
            return true;
        }
        thread::sleep(Duration::from_millis(500));
    }
    false
}

fn wait_child(child: &mut Child, timeout: Duration) -> Result<Option<ExitStatus>, DesktopError> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn runtime_dir(app: &AppHandle) -> Result<PathBuf, DesktopError> {
    let dir = app.path().app_cache_dir()?.join("runtime");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

// Raw plain-text output from `mountos mcp <sub>`, trimmed. Per-client status
// shapes vary too much to usefully parse ("registered but stale" vs "CLI
// present, check with `claude mcp list`"), and showing the exact CLI output
// matches this app's own convention of never obscuring what the CLI said.
fn mcp_subcommand_output(sub: &str) -> Result<String, DesktopError> {
    let output = command_output(&["mcp", sub])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}").trim().to_string();
    if !output.status.success() && combined.is_empty() {
        return Err(DesktopError::Message(format!(
            "mountos mcp {sub} exited with {}",
            output.status
        )));
    }
    Ok(combined)
}

fn mcp_status_blocking() -> Result<String, DesktopError> {
    mcp_subcommand_output("status")
}

fn mcp_install_blocking() -> Result<String, DesktopError> {
    mcp_subcommand_output("install")
}

fn mcp_uninstall_blocking() -> Result<String, DesktopError> {
    mcp_subcommand_output("uninstall")
}

#[tauri::command]
async fn mcp_status() -> Result<String, DesktopError> {
    tauri::async_runtime::spawn_blocking(mcp_status_blocking)
        .await
        .map_err(|error| DesktopError::Message(format!("mcp status task failed: {error}")))?
}

#[tauri::command]
async fn mcp_install() -> Result<String, DesktopError> {
    tauri::async_runtime::spawn_blocking(mcp_install_blocking)
        .await
        .map_err(|error| DesktopError::Message(format!("mcp install task failed: {error}")))?
}

fn mount_help_blocking() -> Result<String, DesktopError> {
    let output = command_output(&["mount", "-h"])?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Err(DesktopError::Message(format!(
            "mountos mount -h exited with {}",
            output.status
        )));
    }
    Ok(text)
}

#[tauri::command]
async fn mount_help() -> Result<String, DesktopError> {
    tauri::async_runtime::spawn_blocking(mount_help_blocking)
        .await
        .map_err(|error| DesktopError::Message(format!("mount help task failed: {error}")))?
}

#[tauri::command]
async fn mcp_uninstall() -> Result<String, DesktopError> {
    tauri::async_runtime::spawn_blocking(mcp_uninstall_blocking)
        .await
        .map_err(|error| DesktopError::Message(format!("mcp uninstall task failed: {error}")))?
}

fn cli_version() -> Option<String> {
    command_output(&["--version"]).ok().and_then(|output| {
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    })
}

fn parse_check(output: &std::process::Output) -> (bool, Vec<CheckIssue>) {
    parse_check_bytes(&output.stdout, &output.stderr)
}

fn parse_check_bytes(stdout: &[u8], stderr: &[u8]) -> (bool, Vec<CheckIssue>) {
    let value = match serde_json::from_slice::<Value>(stdout) {
        Ok(value) => value,
        Err(error) => {
            let stderr = String::from_utf8_lossy(stderr).trim().to_string();
            return (
                false,
                vec![CheckIssue {
                    id: "check-invalid-json".to_string(),
                    severity: "error".to_string(),
                    title: "mountos check returned invalid JSON".to_string(),
                    detail: Some(if stderr.is_empty() {
                        error.to_string()
                    } else {
                        stderr
                    }),
                    fix_command: None,
                }],
            );
        }
    };

    let outputs = value.as_array().cloned().unwrap_or_else(|| vec![value]);
    let any_supported = outputs.iter().any(|item| {
        item.get("supported")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    });
    let mut issues = Vec::new();

    for (index, item) in outputs.iter().enumerate() {
        let feature = item
            .get("feature")
            .and_then(Value::as_str)
            .unwrap_or("Backend");
        let supported = item
            .get("supported")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if supported {
            continue;
        }
        let capable = item
            .get("capable")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut details = item
            .get("checks")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|check| !check.get("ok").and_then(Value::as_bool).unwrap_or(false))
            .map(|check| {
                let name = check.get("name").and_then(Value::as_str).unwrap_or("Check");
                let reason = check
                    .get("error")
                    .or_else(|| check.get("required"))
                    .and_then(Value::as_str)
                    .unwrap_or("not ready");
                format!("{name}: {reason}")
            })
            .collect::<Vec<_>>();
        if let Some(hint) = item
            .get("hint")
            .and_then(Value::as_str)
            .filter(|hint| !hint.is_empty())
        {
            details.push(hint.to_string());
        }
        let fix_command = item
            .get("diagnostics")
            .and_then(Value::as_object)
            .and_then(|diagnostics| {
                ["openHostApp", "pluginKitAdd", "pluginKitReset"]
                    .into_iter()
                    .find_map(|key| diagnostics.get(key).and_then(Value::as_str))
            })
            .map(ToString::to_string);
        issues.push(CheckIssue {
            id: format!(
                "backend-{}-{index}",
                feature.to_ascii_lowercase().replace(' ', "-")
            ),
            severity: if any_supported { "warning" } else { "error" }.to_string(),
            title: if capable {
                format!("{feature} requires setup")
            } else {
                format!("{feature} unavailable")
            },
            detail: (!details.is_empty()).then(|| details.join("\n")),
            fix_command,
        });
    }

    if outputs.is_empty() {
        issues.push(CheckIssue {
            id: "check-empty".to_string(),
            severity: "error".to_string(),
            title: "mountos check returned no backend records".to_string(),
            detail: None,
            fix_command: None,
        });
    }
    (any_supported, issues)
}

fn parse_instances(output: &std::process::Output) -> Result<Vec<MountInstance>, DesktopError> {
    if !output.status.success() {
        return Err(DesktopError::Message(format!(
            "mountos list failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let value = serde_json::from_slice::<Value>(&output.stdout)?;
    Ok(parse_instances_value(&value))
}

fn parse_instances_value(value: &Value) -> Vec<MountInstance> {
    let entries = value
        .as_array()
        .cloned()
        .or_else(|| value.get("mounts").and_then(Value::as_array).cloned())
        .unwrap_or_default();
    entries
        .into_iter()
        .map(|entry| {
            let mount_path = entry
                .get("mountPath")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let domain_id = entry
                .get("domainId")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let fs_name = entry
                .get("fsName")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let orphaned = entry.get("orphaned").and_then(Value::as_bool);
            let limited = domain_id.is_some()
                || fs_name.as_deref().is_some_and(|name| {
                    let name = name.to_ascii_lowercase();
                    name.starts_with("fileprovider") || name.contains("cloudfilter")
                });
            MountInstance {
                key: domain_id.clone().unwrap_or_else(|| mount_path.clone()),
                name: entry
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("mountOS volume")
                    .to_string(),
                mount_path,
                fs_name,
                backend: entry
                    .get("backend")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                view_mode: entry
                    .get("viewMode")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                project_volume_id: entry
                    .get("projectVolumeId")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                volume_id: entry
                    .get("volumeId")
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok()),
                domain_id,
                unc_path: entry
                    .get("uncPath")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                version_inode: entry
                    .get("versionInode")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                orphaned,
                // Both are corrected once the saved profiles are read (see
                // get_system_state); listing alone cannot know.
                external: true,
                profile_id: None,
                health: if orphaned == Some(true) {
                    "lost"
                } else if limited {
                    "limited"
                } else {
                    "healthy"
                }
                .to_string(),
            }
        })
        .collect()
}

fn scrub_secrets(input: &str) -> String {
    static ASSIGNMENT: OnceLock<Regex> = OnceLock::new();
    static BEARER: OnceLock<Regex> = OnceLock::new();
    static SECRET_TOKEN: OnceLock<Regex> = OnceLock::new();
    let assignment = ASSIGNMENT.get_or_init(|| {
        Regex::new(r#"(?i)(secret(?:[ _-]?access[ _-]?key)?|password|authorization|auth[_-]?token)(\s*[:=]\s*)(?:\"[^\"\r\n]*\"|'[^'\r\n]*'|[^\s,}\]]+)"#)
            .expect("valid secret assignment regex")
    });
    let bearer = BEARER
        .get_or_init(|| Regex::new(r"(?i)\bbearer\s+[^\s,}\]]+").expect("valid bearer regex"));
    let secret_token = SECRET_TOKEN
        .get_or_init(|| Regex::new(r"\b[A-Za-z0-9_+/=-]{40}\b").expect("valid secret token regex"));
    let scrubbed = assignment.replace_all(input, "$1$2[REDACTED]");
    let scrubbed = bearer.replace_all(&scrubbed, "Bearer [REDACTED]");
    secret_token
        .replace_all(&scrubbed, "[REDACTED]")
        .into_owned()
}

fn scrub_json(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let key = key.to_ascii_lowercase();
                if key.contains("secret")
                    || key.contains("password")
                    || key.contains("authorization")
                    || key.contains("auth_token")
                    || key.contains("auth-token")
                {
                    *child = Value::String("[REDACTED]".to_string());
                } else {
                    scrub_json(child);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(scrub_json),
        Value::String(text) => *text = scrub_secrets(text),
        _ => {}
    }
}

fn scrub_output(bytes: &[u8]) -> Value {
    match serde_json::from_slice::<Value>(bytes) {
        Ok(mut value) => {
            scrub_json(&mut value);
            value
        }
        Err(_) => Value::String(scrub_secrets(&String::from_utf8_lossy(bytes))),
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, DesktopError> {
    Ok(app.path().app_config_dir()?.join("settings.json"))
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Result<DesktopSettings, DesktopError> {
    let settings = match fs::read(settings_path(&app)?) {
        Ok(bytes) => serde_json::from_slice(&bytes)?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => DesktopSettings::default(),
        Err(err) => return Err(err.into()),
    };
    set_cli_path_override(settings.cli_path_override.as_ref().map(PathBuf::from));
    Ok(settings)
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: DesktopSettings) -> Result<DesktopSettings, DesktopError> {
    validate_backend_for_platform(&settings.default_backend)?;
    // Bounded here, not just in the picker: settings.json is a plain file a user
    // can hand-edit, and 0 would busy-loop the CLI while a huge value would look
    // like the list had frozen.
    if let Some(seconds) = settings.poll_seconds {
        if !(1..=3600).contains(&seconds) {
            return Err(DesktopError::Message(format!(
                "refresh interval must be between 1 and 3600 seconds, got {seconds}"
            )));
        }
    }
    if let Some(pinned) = &settings.cli_path_override {
        if !PathBuf::from(pinned).is_file() {
            return Err(DesktopError::Message(format!(
                "not a file: {pinned}"
            )));
        }
    }
    let path = settings_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(&settings)?)?;
    set_cli_path_override(settings.cli_path_override.as_ref().map(PathBuf::from));
    Ok(settings)
}

#[tauri::command]
fn list_profiles(app: AppHandle) -> Result<Vec<MountProfile>, DesktopError> {
    read_profiles(&app)
}

#[tauri::command]
fn delete_profile(app: AppHandle, profile_id: String) -> Result<(), DesktopError> {
    let path = profile_path(&app, &profile_id)?;
    match keyring_entry(&profile_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => {}
        Err(err) => return Err(DesktopError::Keyring(err)),
    }
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

// Export never carries a secret; the vault entry is keyed to the local
// profile id, so the file downgrades to prompt-on-mount.
fn export_file_stem(name: &str, fallback: &str) -> String {
    let stem: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ' ') {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let stem = stem.trim().trim_matches('-').to_string();
    if stem.is_empty() {
        fallback.to_string()
    } else {
        stem
    }
}

#[tauri::command]
fn export_profile(app: AppHandle, profile_id: String) -> Result<ExportedProfile, DesktopError> {
    validate_profile_id(&profile_id)?;
    let mut profile = read_profiles(&app)?
        .into_iter()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| DesktopError::Message("profile not found".to_string()))?;
    profile.secret_ref = "prompt".to_string();
    let path = app.path().download_dir()?.join(format!(
        "{}.mosprofile",
        export_file_stem(&profile.name, &profile.id)
    ));
    fs::write(&path, serde_json::to_vec_pretty(&profile)?)?;
    Ok(ExportedProfile {
        path: path.display().to_string(),
    })
}

#[tauri::command]
fn save_profile(app: AppHandle, profile: MountProfile) -> Result<MountProfile, DesktopError> {
    validate_profile_id(&profile.id)?;
    if profile.schema_version != 1 {
        return Err(DesktopError::Message(format!(
            "unsupported profile schema version {}",
            profile.schema_version
        )));
    }
    if profile.kind != "mount" {
        return Err(DesktopError::Message(
            "unsupported profile kind".to_string(),
        ));
    }
    if profile.secret_ref == "vault" && profile.access_key_id.is_empty() {
        return Err(DesktopError::Message(
            "vault storage requires an access key ID".to_string(),
        ));
    }
    // Length-only: mountOS access key IDs are fixed-length, but the exact
    // charset isn't this GUI's contract to police.
    if !profile.access_key_id.is_empty() && profile.access_key_id.chars().count() != ACCESS_KEY_ID_LENGTH {
        return Err(DesktopError::Message(format!(
            "access key ID must be {ACCESS_KEY_ID_LENGTH} characters"
        )));
    }
    validate_mount_path_for_backend(&profile.backend, &profile.mount_path)?;
    let rejected = validate_extra_args(&profile.extra_args);
    if !rejected.is_empty() {
        return Err(DesktopError::Message(format!(
            "managed extra args rejected: {}",
            rejected.join(", ")
        )));
    }
    let path = profile_path(&app, &profile.id)?;
    fs::write(path, serde_json::to_vec_pretty(&profile)?)?;
    Ok(profile)
}

#[tauri::command]
fn set_profile_secret(profile_id: String, secret: String) -> Result<SecretStatus, DesktopError> {
    keyring_entry(&profile_id)?.set_password(&secret)?;
    Ok(SecretStatus {
        profile_id,
        stored: true,
    })
}

#[tauri::command]
fn delete_profile_secret(profile_id: String) -> Result<SecretStatus, DesktopError> {
    match keyring_entry(&profile_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(SecretStatus {
            profile_id,
            stored: false,
        }),
        Err(err) => Err(DesktopError::Keyring(err)),
    }
}

#[tauri::command]
fn get_profile_secret_status(profile_id: String) -> Result<SecretStatus, DesktopError> {
    let stored = match keyring_entry(&profile_id)?.get_password() {
        Ok(_) => true,
        Err(keyring::Error::NoEntry) => false,
        Err(err) => return Err(DesktopError::Keyring(err)),
    };
    Ok(SecretStatus { profile_id, stored })
}

fn get_system_state_blocking(app: AppHandle) -> Result<SystemState, DesktopError> {
    let cli_path = mountos_path().ok();
    let cli_path_string = cli_path.as_ref().map(|path| path.display().to_string());
    let cli_version = cli_version();
    let check_output = command_output(&["check", "--json"]);
    let (mut check_ok, mut issues) = match check_output {
        Ok(output) => parse_check(&output),
        Err(DesktopError::CliNotFound) => (
            false,
            vec![CheckIssue {
                id: "cli-missing".to_string(),
                severity: "error".to_string(),
                title: "mountos binary not found".to_string(),
                detail: Some(
                    "No mountos binary was found on PATH. Install the mountos CLI, or if it's \
                     already installed somewhere non-standard, pin its exact path under \
                     Settings \u{2192} About."
                        .to_string(),
                ),
                fix_command: None,
            }],
        ),
        Err(err) => (
            false,
            vec![CheckIssue {
                id: "cli-unavailable".to_string(),
                severity: "error".to_string(),
                title: "mountos CLI unavailable".to_string(),
                detail: Some(err.to_string()),
                fix_command: None,
            }],
        ),
    };
    let mut instances =
        match command_output(&["list", "--json"]).and_then(|output| parse_instances(&output)) {
            Ok(instances) => instances,
            Err(error) => {
                check_ok = false;
                issues.push(CheckIssue {
                    id: "list-failed".to_string(),
                    severity: "error".to_string(),
                    title: "Unable to enumerate mounts".to_string(),
                    detail: Some(error.to_string()),
                    fix_command: None,
                });
                Vec::new()
            }
        };
    let profile_targets = read_profiles(&app)
        .unwrap_or_default()
        .into_iter()
        .filter(|profile| !profile.mount_path.is_empty())
        .map(|profile| (profile.id, profile.mount_path))
        .collect::<Vec<_>>();
    for instance in &mut instances {
        // Report WHICH profile matched, not just that one did: the row offers to
        // clone it, and re-deriving the match in the frontend would mean
        // duplicating targets_equal's path normalisation (/tmp vs /private/tmp)
        // where it could silently drift from this.
        instance.profile_id = profile_targets
            .iter()
            .find(|(_, target)| targets_equal(target, &instance.mount_path))
            .map(|(id, _)| id.clone());
        instance.external = instance.profile_id.is_none();
    }
    let cli_path_alternates = other_cli_paths_on_path(cli_path.as_deref());
    Ok(SystemState {
        platform: match std::env::consts::OS {
            "macos" => "macos",
            "windows" => "windows",
            value => value,
        }
        .to_string(),
        cli_path: cli_path_string,
        cli_version,
        check_ok,
        issues,
        instances,
        cli_path_alternates,
        terminals: available_terminals(),
    })
}

#[tauri::command]
async fn get_system_state(app: AppHandle) -> Result<SystemState, DesktopError> {
    tauri::async_runtime::spawn_blocking(move || get_system_state_blocking(app))
        .await
        .map_err(|error| DesktopError::Message(format!("system state task failed: {error}")))?
}

fn mount_profile_blocking(
    app: AppHandle,
    profile_id: String,
    secret: Option<String>,
) -> Result<MountResult, DesktopError> {
    validate_profile_id(&profile_id)?;
    let mut profile = read_profiles(&app)?
        .into_iter()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| DesktopError::Message("profile not found".to_string()))?;
    validate_backend_for_platform(&profile.backend)?;
    resolve_auto_backend(&mut profile)?;
    validate_mount_path_for_backend(&profile.backend, &profile.mount_path)?;
    let rejected = validate_extra_args(&profile.extra_args);
    if !rejected.is_empty() {
        return Err(DesktopError::Message(format!(
            "managed extra args rejected: {}",
            rejected.join(", ")
        )));
    }
    let needs_mount_path = backend_needs_mount_path(&profile.backend);
    if needs_mount_path && profile.mount_path.is_empty() {
        return Err(DesktopError::Message("mount path is required".to_string()));
    }
    if needs_mount_path && list_contains_target(&profile.mount_path)? {
        return Err(DesktopError::Message(format!(
            "target is already mounted: {}",
            profile.mount_path
        )));
    }
    let mount_secret = if profile.access_key_id.is_empty() {
        None
    } else {
        Some(
            read_profile_secret(&profile, secret)?
                .ok_or_else(|| DesktopError::Message("secret required for mount".to_string()))?,
        )
    };
    let args = build_mount_argv(&profile);
    let stderr_path = runtime_dir(&app)?.join(format!("mount-{}-stderr.log", profile.id));
    let stdout_path = runtime_dir(&app)?.join(format!("mount-{}-stdout.log", profile.id));
    let stderr_file = fs::File::create(&stderr_path)?;
    let stdout_file = fs::File::create(&stdout_path)?;
    let mut child = Command::new(mountos_path()?)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()?;

    if let Some(secret) = mount_secret {
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(secret.as_bytes())?;
            stdin.write_all(b"\n")?;
        }
    }
    drop(child.stdin.take());

    let Some(status) = wait_child(&mut child, LAUNCH_TIMEOUT)? else {
        return Err(DesktopError::Message(
            "mount launch timed out and the child process was terminated".to_string(),
        ));
    };
    if status.success() {
        #[cfg(windows)]
        if !poll_target(&profile.mount_path, true, WINDOWS_READY_TIMEOUT) {
            let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
            let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
            let detail = format!("{stderr}\n{stdout}").trim().to_string();
            return Err(DesktopError::Message(if detail.is_empty() {
                "mount process exited, but the target did not become ready within 60 seconds"
                    .to_string()
            } else {
                detail
            }));
        }
        return Ok(MountResult {
            state: "ready".to_string(),
            target: profile.mount_path,
        });
    }
    let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
    let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
    let detail = format!("{stderr}\n{stdout}").trim().to_string();
    let indeterminate =
        detail.contains("did not become ready within") || detail.contains("no readiness signal");
    if indeterminate && poll_target(&profile.mount_path, true, INDETERMINATE_TIMEOUT) {
        return Ok(MountResult {
            state: "ready".to_string(),
            target: profile.mount_path,
        });
    }
    if detail.is_empty() {
        Err(DesktopError::Message(format!(
            "mountos mount exited with {status}"
        )))
    } else if indeterminate {
        Err(DesktopError::Message(format!(
            "indeterminate launch did not appear in the mount table after reconciliation: {detail}"
        )))
    } else {
        Err(DesktopError::Message(detail))
    }
}

#[tauri::command]
async fn mount_profile(
    app: AppHandle,
    profile_id: String,
    secret: Option<String>,
) -> Result<MountResult, DesktopError> {
    tauri::async_runtime::spawn_blocking(move || mount_profile_blocking(app, profile_id, secret))
        .await
        .map_err(|error| DesktopError::Message(format!("mount task failed: {error}")))?
}

fn unmount_target_blocking(target: String) -> Result<UnmountResult, DesktopError> {
    if target.trim().is_empty() {
        return Err(DesktopError::Message(
            "unmount target is required".to_string(),
        ));
    }
    // Mirrors open_target's guard: the UI's mount list can be up to 30s
    // stale (background polling interval), so re-verify against a fresh
    // `list --json` immediately before acting rather than trusting a
    // possibly-stale row the user is looking at. Closes the window where a
    // different volume could have taken over the same mount path since the
    // last poll.
    if !list_contains_target(&target)? {
        return Err(DesktopError::Message(format!(
            "refusing to unmount a path that is not an active mount target: {target}"
        )));
    }
    let output = Command::new(mountos_path()?)
        .args(["unmount", "-y", &target])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() || stdout.contains("Unmount cancelled.") {
        let detail = format!("{}\n{}", String::from_utf8_lossy(&output.stderr), stdout)
            .trim()
            .to_string();
        return Err(DesktopError::Message(if detail.is_empty() {
            format!("mountos unmount exited with {}", output.status)
        } else {
            detail
        }));
    }
    let removed = poll_target(&target, false, UNMOUNT_TIMEOUT);
    Ok(UnmountResult {
        state: if removed { "idle" } else { "flushing" }.to_string(),
        target,
    })
}

#[tauri::command]
async fn unmount_target(target: String) -> Result<UnmountResult, DesktopError> {
    tauri::async_runtime::spawn_blocking(move || unmount_target_blocking(target))
        .await
        .map_err(|error| DesktopError::Message(format!("unmount task failed: {error}")))?
}

fn list_active_targets() -> Result<Vec<String>, DesktopError> {
    let output = command_output(&["list", "--json"])?;
    if !output.status.success() {
        return Err(DesktopError::Message(format!(
            "mountos list failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let value = serde_json::from_slice::<Value>(&output.stdout)?;
    let entries = value
        .as_array()
        .cloned()
        .or_else(|| value.get("mounts").and_then(Value::as_array).cloned())
        .unwrap_or_default();
    Ok(entries
        .iter()
        .filter_map(|entry| {
            entry
                .get("domainId")
                .and_then(Value::as_str)
                .or_else(|| entry.get("mountPath").and_then(Value::as_str))
                .map(ToString::to_string)
        })
        .collect())
}

// unmount --all is one shell-out for the whole fleet rather than N individual
// unmount_target calls: the CLI's own combined confirmation-free (-y) batch
// unmount is both faster (no N separate process spawns) and matches the CLI's
// own --all semantics exactly. Success/failure per target isn't parsed out of
// CLI text (fragile); instead the active target list is diffed before/after —
// anything still present after the call didn't unmount.
fn unmount_all_targets_blocking() -> Result<UnmountAllResult, DesktopError> {
    let before = list_active_targets()?;
    if before.is_empty() {
        return Ok(UnmountAllResult {
            attempted: 0,
            failed: Vec::new(),
        });
    }
    let _ = Command::new(mountos_path()?)
        .args(["unmount", "--all", "-y"])
        .output()?;
    let after = list_active_targets()?;
    let failed: Vec<String> = before
        .iter()
        .filter(|target| after.iter().any(|remaining| remaining == *target))
        .cloned()
        .collect();
    Ok(UnmountAllResult {
        attempted: before.len(),
        failed,
    })
}

#[tauri::command]
async fn unmount_all_targets() -> Result<UnmountAllResult, DesktopError> {
    tauri::async_runtime::spawn_blocking(unmount_all_targets_blocking)
        .await
        .map_err(|error| DesktopError::Message(format!("unmount-all task failed: {error}")))?
}

// Reads .mountOS/.config directly off disk rather than shelling out — it's
// a plain JSON file the mfuse process already writes for its own TUI/CLI
// tooling (cmd/mfuse/reserved.go getConfigData), so no extra CLI round
// trip is needed. Scrubbing is unnecessary: MountConfig carries the access
// key ID (a public 20-char identifier) but never the secret.
#[tauri::command]
fn get_instance_config(target: String) -> Result<String, DesktopError> {
    if !is_openable_target(&target) {
        return Err(DesktopError::Message(
            "mount target is not an absolute filesystem path".to_string(),
        ));
    }
    if !list_contains_target(&target)? {
        return Err(DesktopError::Message(
            "refusing to read config for a path that is not an active mount target".to_string(),
        ));
    }
    let config_path = PathBuf::from(&target).join(".mountOS").join(".config");
    let bytes = fs::read(&config_path).map_err(|err| {
        DesktopError::Message(format!(
            "failed to read {}: {err}",
            config_path.display()
        ))
    })?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    Ok(serde_json::to_string_pretty(&value)?)
}

// POSIX single-quote wrap: closes the quote, emits an escaped literal quote,
// reopens it. Safe for any byte sequence, including embedded quotes.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

// AppleScript double-quoted string literal escaping (backslash and quote).
#[cfg(target_os = "macos")]
fn applescript_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

// Launches the platform's terminal running `mountos dashboard [--gui]
// <target>`, left open afterward exactly like a terminal the user opened
// themselves (Terminal.app's `do script` and `cmd /k` both do this
// natively; the Linux fallback re-execs the shell to match).
// The dashboard TUI refuses to render below 118x35 and shows a "Terminal too
// small" placeholder instead, so every launcher below sizes its window up
// front rather than inheriting the terminal's default.
const TUI_COLS: &str = "140";
const TUI_ROWS: &str = "42";

// Terminals the launcher supports, in the order `auto` falls back through. The
// platform's stock terminal is first: it is always installed and is the path
// that has always shipped, so an unset or unavailable preference behaves
// exactly as before.
//
// Membership is by verified launch, not popularity. Each entry was confirmed to
// actually run a command in a new window at the size above. Warp is absent for
// that reason: it launches, but silently ignores a command passed to it, which
// would look like a working dashboard button that opens an empty terminal.
// kitty and WezTerm are absent only because they were not available to verify.
#[cfg(target_os = "macos")]
const KNOWN_TERMINALS: &[(&str, &str, &str)] = &[
    // (id, label, .app bundle name)
    ("terminal", "Terminal", "Terminal"),
    ("iterm2", "iTerm2", "iTerm"),
    ("ghostty", "Ghostty", "Ghostty"),
    ("alacritty", "Alacritty", "Alacritty"),
    ("kitty", "kitty", "kitty"),
    ("wezterm", "WezTerm", "WezTerm"),
];

#[cfg(target_os = "macos")]
fn terminal_bundle_path(bundle: &str) -> Option<PathBuf> {
    let mut roots = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        roots.push(PathBuf::from(home).join("Applications"));
    }
    roots
        .into_iter()
        .map(|root| root.join(format!("{bundle}.app")))
        .find(|path| path.exists())
}

#[cfg(target_os = "macos")]
fn available_terminals() -> Vec<TerminalOption> {
    KNOWN_TERMINALS
        .iter()
        .filter(|(_, _, bundle)| terminal_bundle_path(bundle).is_some())
        .map(|(id, label, _)| TerminalOption {
            id: (*id).to_string(),
            label: (*label).to_string(),
        })
        .collect()
}

// Ghostty and Alacritty are launched through `open -na`, not their in-bundle
// binary: Ghostty's own CLI refuses to start the emulator on macOS ("launching
// the terminal emulator from the CLI is not supported ... use open -na
// Ghostty.app") and the same form works for Alacritty, so both take one path.
#[cfg(target_os = "macos")]
fn open_app_with_args(bundle: &str, args: &[&str]) -> Result<(), DesktopError> {
    let mut command = Command::new("open");
    command.arg("-na").arg(bundle).arg("--args").args(args);
    let output = command.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(DesktopError::Message(if stderr.is_empty() {
            format!("open -na {bundle} exited with {}", output.status)
        } else {
            stderr
        }));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn spawn_iterm2(shell_cmd: &str) -> Result<(), DesktopError> {
    // `write text` into a freshly created window, which is iTerm2's own
    // scripting idiom; unlike Terminal.app it has no cold-launch window race,
    // since `create window with default profile` returns the window itself.
    let script = format!(
        "tell application \"iTerm\"\n  activate\n  set newWindow to (create window with default profile)\n  tell current session of newWindow\n    set columns to {TUI_COLS}\n    set rows to {TUI_ROWS}\n    write text {}\n  end tell\nend tell",
        applescript_quote(shell_cmd)
    );
    run_osascript(script, "iTerm")
}

#[cfg(target_os = "macos")]
fn spawn_dashboard_terminal(
    mountos: &Path,
    target: &str,
    gui: bool,
    preferred: Option<&str>,
) -> Result<(), DesktopError> {
    let mut shell_cmd = shell_quote(&mountos.display().to_string());
    shell_cmd.push_str(" dashboard");
    if gui {
        shell_cmd.push_str(" --gui");
    }
    shell_cmd.push(' ');
    shell_cmd.push_str(&shell_quote(target));

    // A preference for a terminal that has since been uninstalled falls back to
    // the stock one rather than failing: the user asked to see a dashboard, not
    // to see it in one specific app.
    let available = available_terminals();
    let chosen = preferred
        .filter(|id| available.iter().any(|option| option.id == *id))
        .unwrap_or("terminal");

    match chosen {
        "iterm2" => return spawn_iterm2(&shell_cmd),
        "ghostty" => {
            return open_app_with_args(
                "Ghostty",
                &[
                    &format!("--window-width={TUI_COLS}"),
                    &format!("--window-height={TUI_ROWS}"),
                    "-e",
                    "/bin/sh",
                    "-c",
                    &shell_cmd,
                ],
            )
        }
        "alacritty" => {
            return open_app_with_args(
                "Alacritty",
                &[
                    "-o",
                    &format!("window.dimensions.columns={TUI_COLS}"),
                    "-o",
                    &format!("window.dimensions.lines={TUI_ROWS}"),
                    "-e",
                    "/bin/sh",
                    "-c",
                    &shell_cmd,
                ],
            )
        }
        // kitty takes the command as trailing args, with no -e separator, and
        // sizes in cells via the `c` suffix.
        "kitty" => {
            return open_app_with_args(
                "kitty",
                &[
                    "-o",
                    &format!("initial_window_width={TUI_COLS}c"),
                    "-o",
                    &format!("initial_window_height={TUI_ROWS}c"),
                    "/bin/sh",
                    "-c",
                    &shell_cmd,
                ],
            )
        }
        // WezTerm is the one that does NOT go through `open -na`: passed that
        // way it launches but silently drops the command. Its in-bundle CLI
        // works, and --config is a global flag that must precede `start` --
        // after it, wezterm rejects it outright.
        "wezterm" => {
            let bundle = terminal_bundle_path("WezTerm")
                .ok_or_else(|| DesktopError::Message("WezTerm.app not found".to_string()))?;
            Command::new(bundle.join("Contents/MacOS/wezterm"))
                .arg("--config")
                .arg(format!("initial_cols={TUI_COLS}"))
                .arg("--config")
                .arg(format!("initial_rows={TUI_ROWS}"))
                .arg("start")
                .arg("--")
                .arg("/bin/sh")
                .arg("-c")
                .arg(&shell_cmd)
                .spawn()?;
            return Ok(());
        }
        _ => {}
    }

    // Two `do script` calls, not one: sending the real command to a cold-launched
    // Terminal (no windows yet) can race the new window's shell coming up and the
    // typed text is silently dropped, leaving an empty window. Forcing a window to
    // exist first (an empty `do script`) and then targeting it explicitly for the
    // real command avoids that race. The column/row resize (before the real command
    // runs) works around Terminal's default new-window size (83x24) being smaller
    // than the dashboard TUI's own minimum (118x35), which otherwise replaces the
    // dashboard with a "Terminal too small" placeholder instead of rendering it.
    let script = format!(
        "tell application \"Terminal\"\n  activate\n  if (count of windows) = 0 then\n    do script \"\"\n  end if\n  set number of columns of front window to {TUI_COLS}\n  set number of rows of front window to {TUI_ROWS}\n  do script {} in front window\nend tell",
        applescript_quote(&shell_cmd)
    );
    run_osascript(script, "Terminal")
}

// .output(), not .spawn(): the scripting calls here are synchronous AppleEvents
// that return once the terminal has started the command (not once it finishes),
// so this doesn't block on the dashboard session itself — but it DOES need to be
// awaited to catch a real failure, most notably "AppleEvent timed out" (-1712)
// when this app hasn't been granted Automation permission for that terminal yet
// (System Settings > Privacy & Security > Automation). A bare .spawn() would
// silently report success even when osascript itself failed.
#[cfg(target_os = "macos")]
fn run_osascript(script: String, app: &str) -> Result<(), DesktopError> {
    let output = Command::new("osascript").arg("-e").arg(script).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(DesktopError::Message(if stderr.contains("-1712") {
            format!(
                "macOS blocked mountOS Desktop from controlling {app}. Grant it in System \
                 Settings \u{2192} Privacy & Security \u{2192} Automation, then try again."
            )
        } else if stderr.is_empty() {
            format!("osascript exited with {}", output.status)
        } else {
            stderr
        }));
    }
    Ok(())
}

// Windows Terminal is the Win11 default host and cmd is guaranteed present.
// Both are the forms this launcher already shipped with, i.e. the ones known to
// work. Alacritty/WezTerm/PowerShell are deliberately absent rather than added
// from documentation alone -- there was no Windows host here to verify them on,
// and an unverified entry in this list is a dashboard button that opens the
// wrong thing or nothing.
#[cfg(windows)]
const KNOWN_TERMINALS: &[(&str, &str, &str)] = &[
    // (id, label, binary to probe on PATH)
    ("wt", "Windows Terminal", "wt"),
    ("cmd", "Command Prompt", "cmd"),
];

#[cfg(windows)]
fn available_terminals() -> Vec<TerminalOption> {
    KNOWN_TERMINALS
        .iter()
        // cmd is the guaranteed fallback: ComSpec always resolves, and probing
        // PATH for it would be the one lookup that must never fail.
        .filter(|(id, _, bin)| *id == "cmd" || which::which(bin).is_ok())
        .map(|(id, label, _)| TerminalOption {
            id: (*id).to_string(),
            label: (*label).to_string(),
        })
        .collect()
}

#[cfg(windows)]
fn spawn_dashboard_terminal(
    mountos: &Path,
    target: &str,
    gui: bool,
    preferred: Option<&str>,
) -> Result<(), DesktopError> {
    let mut shell_cmd = format!("mode con: cols={TUI_COLS} lines={TUI_ROWS} && \"{}\" dashboard", mountos.display());
    if gui {
        shell_cmd.push_str(" --gui");
    }
    shell_cmd.push_str(&format!(" \"{target}\""));

    // cmd /k keeps the window open after the command finishes, like
    // Terminal.app's do script. `mode con` resizes the console before the
    // dashboard TUI starts — its own minimum (118x35) is larger than cmd/wt's
    // default new-window/new-tab size, which otherwise shows a "Terminal too
    // small" placeholder instead of the dashboard (mirrors the macOS fix).
    //
    // An uninstalled preference falls back to cmd rather than failing: the user
    // asked to see a dashboard, not to see it in one specific app.
    let available = available_terminals();
    let chosen = preferred
        .filter(|id| available.iter().any(|option| option.id == *id))
        .unwrap_or(if which::which("wt").is_ok() { "wt" } else { "cmd" });

    if chosen == "wt" {
        if let Ok(wt) = which::which("wt") {
            Command::new(wt)
                .arg("--size")
                .arg(format!("{TUI_COLS},{TUI_ROWS}"))
                .arg("new-tab")
                .arg("cmd")
                .arg("/k")
                .arg(&shell_cmd)
                .spawn()?;
            return Ok(());
        }
    }
    Command::new("cmd").arg("/k").arg(&shell_cmd).spawn()?;
    Ok(())
}

// The chain this launcher already shipped with, now also pickable. Nothing new
// was added from documentation alone: there was no Linux host here to verify a
// launch on, and an unverified entry is a dashboard button that opens the wrong
// thing or nothing.
#[cfg(not(any(windows, target_os = "macos")))]
const KNOWN_TERMINALS: &[(&str, &str, &str)] = &[
    // (id, label, binary to probe on PATH)
    ("x-terminal-emulator", "System default", "x-terminal-emulator"),
    ("gnome-terminal", "GNOME Terminal", "gnome-terminal"),
    ("konsole", "Konsole", "konsole"),
    ("xterm", "xterm", "xterm"),
];

#[cfg(not(any(windows, target_os = "macos")))]
fn available_terminals() -> Vec<TerminalOption> {
    KNOWN_TERMINALS
        .iter()
        .filter(|(_, _, bin)| which::which(bin).is_ok())
        .map(|(id, label, _)| TerminalOption {
            id: (*id).to_string(),
            label: (*label).to_string(),
        })
        .collect()
}

#[cfg(not(any(windows, target_os = "macos")))]
fn spawn_dashboard_terminal(
    mountos: &Path,
    target: &str,
    gui: bool,
    preferred: Option<&str>,
) -> Result<(), DesktopError> {
    let mut shell_cmd = shell_quote(&mountos.display().to_string());
    shell_cmd.push_str(" dashboard");
    if gui {
        shell_cmd.push_str(" --gui");
    }
    shell_cmd.push(' ');
    shell_cmd.push_str(&shell_quote(target));
    // bash -c exits once the command finishes; re-exec the shell so the
    // window behaves like one the user opened themselves, matching
    // Terminal.app / cmd /k's native behavior.
    shell_cmd.push_str("; exec \"$SHELL\"");

    // Try the preference first, then fall through the chain: an uninstalled
    // preference must not fail when another terminal is right there.
    let ordered: Vec<&str> = preferred
        .into_iter()
        .chain(KNOWN_TERMINALS.iter().map(|(id, _, _)| *id))
        .collect();

    for terminal in ordered {
        if which::which(terminal).is_err() {
            continue;
        }
        let mut cmd = Command::new(terminal);
        if terminal == "gnome-terminal" || terminal == "konsole" {
            cmd.arg("--");
        } else {
            cmd.arg("-e");
        }
        cmd.arg("bash").arg("-c").arg(&shell_cmd);
        cmd.spawn()?;
        return Ok(());
    }
    Err(DesktopError::Message(
        "no terminal emulator found on PATH".to_string(),
    ))
}

fn launch_dashboard_blocking(app: AppHandle, target: String, gui: bool) -> Result<(), DesktopError> {
    if !is_openable_target(&target) {
        return Err(DesktopError::Message(
            "mount target is not an absolute filesystem path".to_string(),
        ));
    }
    if !list_contains_target(&target)? {
        return Err(DesktopError::Message(
            "refusing to launch a dashboard for a path that is not an active mount target"
                .to_string(),
        ));
    }
    // A broken/missing settings.json must not block the dashboard; fall back to
    // no preference (the stock terminal) rather than surfacing a settings error
    // from a button that has nothing to do with settings.
    let preferred = get_settings(app).ok().and_then(|settings| settings.terminal);
    spawn_dashboard_terminal(&mountos_path()?, &target, gui, preferred.as_deref())
}

// async + spawn_blocking: on macOS a denied/pending Automation permission
// can make the underlying osascript call take up to its own AppleEvent
// timeout (~2 minutes) to fail, and this must not freeze the Tauri IPC
// thread for that long.
#[tauri::command]
async fn launch_dashboard(app: AppHandle, target: String, gui: bool) -> Result<(), DesktopError> {
    tauri::async_runtime::spawn_blocking(move || launch_dashboard_blocking(app, target, gui))
        .await
        .map_err(|error| DesktopError::Message(format!("dashboard launch task failed: {error}")))?
}

#[tauri::command]
fn open_target(target: String) -> Result<(), DesktopError> {
    if !is_openable_target(&target) {
        return Err(DesktopError::Message(
            "mount target is not an absolute filesystem path".to_string(),
        ));
    }
    if !list_contains_target(&target)? {
        return Err(DesktopError::Message(
            "refusing to open a path that is not an active mount target".to_string(),
        ));
    }
    open::that_detached(target)?;
    Ok(())
}

// Opens a diagnostics bundle this app wrote.
//
// Mirrors open_target's guard: the path is confined to our own diagnostics
// directory rather than opened because the frontend asked. Both sides are
// canonicalized first, so a symlink or `..` inside the argument cannot escape
// the directory and turn this into an arbitrary "open any file" primitive.
#[tauri::command]
fn open_diagnostics_bundle(app: AppHandle, path: String) -> Result<(), DesktopError> {
    let dir = app.path().app_cache_dir()?.join("diagnostics").canonicalize()?;
    let target = PathBuf::from(&path).canonicalize()?;
    if !target.starts_with(&dir) {
        return Err(DesktopError::Message(
            "refusing to open a path outside the diagnostics directory".to_string(),
        ));
    }
    open::that_detached(target)?;
    Ok(())
}

// The Dock icon (and Cmd+Tab entry) tracks the main window's own visibility
// rather than being permanently off: hidden behind the tray only while the
// window is actually hidden, so it doesn't strand an actively-open window
// with no OS-level way back to it (no Dock icon, no Cmd+Tab).
#[cfg(target_os = "macos")]
fn set_dock_visible(app: &AppHandle, visible: bool) {
    let _ = app.set_activation_policy(if visible {
        tauri::ActivationPolicy::Regular
    } else {
        tauri::ActivationPolicy::Accessory
    });
    // set_activation_policy alone is unreliable on its own for the Dock icon
    // actually appearing/disappearing; pairing it with set_dock_visibility is
    // the combination that behaves consistently (confirmed against an older
    // sibling client's tray implementation that hit the same gap).
    let _ = app.set_dock_visibility(visible);
}

#[cfg(not(target_os = "macos"))]
fn set_dock_visible(_app: &AppHandle, _visible: bool) {}

fn show_main_window_internal(app: &AppHandle) -> Result<(), DesktopError> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| DesktopError::Message("main window not found".to_string()))?;
    set_dock_visible(app, true);
    window.show()?;
    window.set_focus()?;
    Ok(())
}

#[tauri::command]
fn show_main_window(app: AppHandle) -> Result<(), DesktopError> {
    show_main_window_internal(&app)
}

#[tauri::command]
async fn create_diagnostics_bundle(app: AppHandle) -> Result<DiagnosticsBundle, DesktopError> {
    tauri::async_runtime::spawn_blocking(move || create_diagnostics_bundle_blocking(app))
        .await
        .map_err(|error| DesktopError::Message(format!("diagnostics task failed: {error}")))?
}

fn create_diagnostics_bundle_blocking(app: AppHandle) -> Result<DiagnosticsBundle, DesktopError> {
    let dir = app.path().app_cache_dir()?.join("diagnostics");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!(
        "mountos-desktop-diagnostics-{}.json",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    ));

    let check = command_output(&["check", "--json"]).ok();
    let list = command_output(&["list", "--json"]).ok();
    let profiles = read_profiles(&app).unwrap_or_default();
    let content = DiagnosticsContent {
        created_at_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        cli_path: mountos_path().ok().map(|path| path.display().to_string()),
        cli_version: cli_version(),
        check: check.as_ref().map(|output| DiagnosticsCommandOutput {
            status: output.status.code(),
            stdout: scrub_output(&output.stdout),
            stderr: scrub_output(&output.stderr),
        }),
        list: list.as_ref().map(|output| DiagnosticsCommandOutput {
            status: output.status.code(),
            stdout: scrub_output(&output.stdout),
            stderr: scrub_output(&output.stderr),
        }),
        profiles: profiles
            .into_iter()
            .map(|profile| DiagnosticsProfileSummary {
                id: profile.id,
                name: profile.name,
                kind: profile.kind,
                mount_path: profile.mount_path,
                discovery_url: profile.discovery_url,
                backend: profile.backend,
                secret_ref: profile.secret_ref,
                extra_args_count: profile.extra_args.len(),
                auto_remount: profile.auto_remount,
            })
            .collect(),
    };
    fs::write(&path, serde_json::to_vec_pretty(&content)?)?;
    Ok(DiagnosticsBundle {
        path: path.display().to_string(),
        content,
    })
}

fn build_tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let instances = get_system_state_blocking(app.clone())
        .map(|state| state.instances)
        .unwrap_or_default();

    let mut items: Vec<Box<dyn IsMenuItem<tauri::Wry>>> = Vec::new();
    if instances.is_empty() {
        items.push(Box::new(MenuItem::with_id(
            app,
            "no-mounts",
            "No active mounts",
            false,
            None::<&str>,
        )?));
    } else {
        // A menubar dropdown listing every mount doesn't scale — cap it and
        // fold the rest into a single "+N more" item that opens the full app
        // (same id as "Open mountOS" below, same action) instead of growing
        // the native menu unboundedly.
        const MAX_MOUNT_ITEMS: usize = 15;
        let total = instances.len();
        for instance in instances.iter().take(MAX_MOUNT_ITEMS) {
            let label = if instance.name.is_empty() {
                instance.mount_path.clone()
            } else {
                instance.name.clone()
            };
            items.push(Box::new(MenuItem::with_id(
                app,
                format!("open-mount:{}", instance.mount_path),
                format!("{label} \u{2014} {}", instance.health),
                is_openable_target(&instance.mount_path),
                None::<&str>,
            )?));
        }
        if total > MAX_MOUNT_ITEMS {
            let remaining = total - MAX_MOUNT_ITEMS;
            items.push(Box::new(MenuItem::with_id(
                app,
                "show",
                format!("+{remaining} more"),
                true,
                None::<&str>,
            )?));
        }
    }
    items.push(Box::new(PredefinedMenuItem::separator(app)?));
    items.push(Box::new(MenuItem::with_id(app, "show", "Open mountOS", true, None::<&str>)?));
    items.push(Box::new(MenuItem::with_id(app, "quit", "Quit mountOS", true, None::<&str>)?));

    let refs: Vec<&dyn IsMenuItem<tauri::Wry>> = items.iter().map(|item| item.as_ref()).collect();
    Menu::with_items(app, &refs)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            let _ = show_main_window_internal(app);
        }))
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Dock icon / Cmd+Tab entry tracks the main window's own
            // visibility (see set_dock_visible) rather than being
            // permanently off, so it doesn't strand an actively-open window
            // with no OS-level way back to it. Sync once to the window's
            // actual starting visibility (visible by default at first run).
            let main_visible = app
                .get_webview_window("main")
                .and_then(|window| window.is_visible().ok())
                .unwrap_or(true);
            set_dock_visible(app.handle(), main_visible);

            let menu = build_tray_menu(app.handle())?;
            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png"))?;

            TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(true)
                .tooltip("mountOS")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    let id = event.id().as_ref();
                    if let Some(target) = id.strip_prefix("open-mount:") {
                        let _ = open_target(target.to_string());
                        return;
                    }
                    match id {
                        "show" => {
                            let _ = show_main_window_internal(app);
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // Native menus render reliably everywhere (including over
                    // another app's fullscreen Space) since AppKit itself owns
                    // showing them — unlike the tray-popover webview window,
                    // which repeated attempts couldn't make reliably visible
                    // there. Rebuilt on hover so the mount list is current by
                    // the time an actual click follows.
                    if let TrayIconEvent::Enter { .. } = event {
                        if let Ok(menu) = build_tray_menu(tray.app_handle()) {
                            let _ = tray.set_menu(Some(menu));
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| match window.label() {
            "main" => {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                    set_dock_visible(window.app_handle(), false);
                }
            }
            "tray-popover" => {
                if let WindowEvent::Focused(false) = event {
                    let _ = window.hide();
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_system_state,
            get_settings,
            save_settings,
            list_profiles,
            save_profile,
            delete_profile,
            export_profile,
            set_profile_secret,
            delete_profile_secret,
            get_profile_secret_status,
            mount_profile,
            unmount_target,
            unmount_all_targets,
            open_target,
            get_instance_config,
            launch_dashboard,
            create_diagnostics_bundle,
            open_diagnostics_bundle,
            mcp_status,
            mcp_install,
            mcp_uninstall,
            mount_help,
            show_main_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running mountOS Desktop");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> MountProfile {
        MountProfile {
            id: "profile-1".to_string(),
            schema_version: 1,
            kind: "mount".to_string(),
            name: "Team files".to_string(),
            volume: "Team files".to_string(),
            fork: "main".to_string(),
            mount_path: "/Volumes/MountOS/Team".to_string(),
            discovery_url: "https://hub.example.com".to_string(),
            access_key_id: "ABCDEFGHIJKLMNOPQRST".to_string(),
            secret_ref: "prompt".to_string(),
            backend: Backend::Nfs,
            cache_dir: Some("/tmp/mountos cache".to_string()),
            read_only: true,
            auto_remount: false,
            temporary_fork: false,
            trusted_discovery_host: None,
            extra_args: vec!["--disk-cache-size".to_string(), "10G".to_string()],
            created_at: "2026-07-10T00:00:00Z".to_string(),
            updated_at: "2026-07-10T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn builds_current_cli_flags_without_a_secret_value() {
        let argv = build_mount_argv(&profile());
        assert!(argv
            .windows(2)
            .any(|args| args == ["--volname", "Team files"]));
        assert!(argv.windows(2).any(|args| args == ["--fork-name", "main"]));
        assert!(argv
            .windows(2)
            .any(|args| args == ["--disk-cache-dir", "/tmp/mountos cache"]));
        assert!(argv.contains(&"--nfs".to_string()));
        assert!(argv.contains(&"-s".to_string()));
        assert!(!argv.contains(&"--volume".to_string()));
        assert!(!argv.contains(&"--fork".to_string()));
        assert!(!argv.contains(&"--cache-dir".to_string()));
        assert!(!argv.iter().any(|arg| arg.len() == 40));
    }

    #[test]
    fn omits_mount_flag_for_os_managed_backends() {
        let mut p = profile();
        p.mount_path = "/some/leftover/path".to_string();
        p.backend = Backend::Fileprovider;
        assert!(!build_mount_argv(&p).contains(&"-m".to_string()));

        p.backend = Backend::Cloudfilter;
        assert!(!build_mount_argv(&p).contains(&"-m".to_string()));

        p.backend = Backend::Nfs;
        assert!(build_mount_argv(&p).contains(&"-m".to_string()));
    }

    #[test]
    fn validates_fskit_mount_path_prefix() {
        assert!(validate_mount_path_for_backend(&Backend::Fskit, "/Volumes/MountOS/Team").is_ok());
        assert!(validate_mount_path_for_backend(&Backend::Fskit, "/Volumes/MountOS/Team/").is_ok());
        assert!(validate_mount_path_for_backend(&Backend::Fskit, "/Volumes/MountOS/").is_err());
        assert!(validate_mount_path_for_backend(&Backend::Fskit, "/Volumes/MountOS").is_err());
        assert!(validate_mount_path_for_backend(&Backend::Fskit, "/tmp/Team").is_err());
        assert!(validate_mount_path_for_backend(&Backend::Fskit, "").is_err());
        // Case-sensitive: the real FSKit registration is a literal path match.
        assert!(validate_mount_path_for_backend(&Backend::Fskit, "/volumes/mountos/Team").is_err());
        // Non-FSKit backends are never gated by this check.
        assert!(validate_mount_path_for_backend(&Backend::Nfs, "/tmp/anything").is_ok());
        assert!(validate_mount_path_for_backend(&Backend::Nfs, "").is_ok());
    }

    #[test]
    fn rejects_non_absolute_mount_paths_for_backends_that_need_one() {
        // Empty stays legal (build_mount_argv omits -m, CLI picks a default).
        assert!(validate_mount_path_for_backend(&Backend::Nfs, "").is_ok());
        // A non-empty value has to actually be an absolute path, not garbage.
        assert!(validate_mount_path_for_backend(&Backend::Nfs, "relative/path").is_err());
        assert!(validate_mount_path_for_backend(&Backend::Nfs, "not-a-path").is_err());
        if cfg!(windows) {
            assert!(validate_mount_path_for_backend(&Backend::Mountosio, "C:\\Mounts\\Team").is_ok());
            assert!(validate_mount_path_for_backend(&Backend::Mountosio, "D:").is_ok());
            assert!(validate_mount_path_for_backend(&Backend::Mountosio, "/Volumes/Team").is_err());
        } else {
            assert!(validate_mount_path_for_backend(&Backend::Nfs, "/Volumes/Team").is_ok());
        }
        // Backends with no real filesystem path (FileProvider/CloudFilter)
        // are never gated by this check either.
        assert!(validate_mount_path_for_backend(&Backend::Fileprovider, "not-a-path").is_ok());
        assert!(validate_mount_path_for_backend(&Backend::Cloudfilter, "not-a-path").is_ok());
    }

    #[test]
    fn shell_quote_survives_a_shell_round_trip() {
        // Every quoted value, fed through `sh -c 'printf %s ' + quoted`,
        // must come back byte-for-byte identical — this is the actual
        // property that matters for dashboard-launcher command construction.
        for raw in [
            "/Volumes/MountOS/Team",
            "/tmp/has space",
            "it's a mount",
            "''; rm -rf /tmp/pwned; echo '",
            "$(echo pwned)",
            "back\\slash",
            "",
        ] {
            let quoted = shell_quote(raw);
            let output = Command::new("sh")
                .arg("-c")
                .arg(format!("printf '%s' {quoted}"))
                .output()
                .expect("sh must be available in test environment");
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                raw,
                "shell_quote({raw:?}) = {quoted:?} did not round-trip"
            );
        }
    }

    #[test]
    fn validates_extra_arg_values_without_treating_them_as_positionals() {
        assert!(validate_extra_args(&[
            "--disk-cache-size".to_string(),
            "10G".to_string(),
            "--debug".to_string(),
        ])
        .is_empty());
        assert_eq!(
            validate_extra_args(&["--mount".to_string(), "/tmp/other".to_string()]),
            vec!["--mount".to_string(), "/tmp/other".to_string()]
        );
    }

    #[test]
    fn validates_short_flag_clusters() {
        // A managed short flag anywhere before the '-o' value-absorbing
        // point is caught, regardless of position in the cluster.
        assert_eq!(
            validate_extra_args(&["-am".to_string()]),
            vec!["-am".to_string()]
        );
        assert_eq!(
            validate_extra_args(&["-ma".to_string()]),
            vec!["-ma".to_string()]
        );
        // '-o' takes a fused value (mirrors real short-opt parsing: once a
        // value-taking flag is hit in a cluster, the rest of the token is
        // its value, not further flags) — bare '-o' and '-o<value>' are both
        // accepted even when the value text collides with a managed letter.
        assert!(validate_extra_args(&["-o".to_string()]).is_empty());
        assert!(validate_extra_args(&["-oallow_other".to_string()]).is_empty());
        assert!(validate_extra_args(&["-oa".to_string()]).is_empty());
        // Bare "--" (positional separator) is rejected like any other
        // non-managed-but-suspicious positional.
        assert_eq!(
            validate_extra_args(&["--".to_string()]),
            vec!["--".to_string()]
        );
    }

    #[test]
    fn parses_backend_check_records() {
        let payload = br#"[
          {"feature":"NFS","supported":true,"checks":[{"name":"mount_nfs","ok":true}]},
          {"feature":"FSKit","supported":false,"capable":true,"checks":[{"name":"Extension","ok":false,"error":"not enabled"}]}
        ]"#;
        let (ok, issues) = parse_check_bytes(payload, b"");
        assert!(ok);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].title, "FSKit requires setup");
        assert!(issues[0]
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("not enabled")));
    }

    #[test]
    fn preserves_list_types_and_marks_orphans_lost() {
        let value = serde_json::json!([{
            "name": "Team",
            "mountPath": "/Volumes/Team",
            "fsName": "mountos",
            "volumeId": 42,
            "versionInode": "9007199254740993",
            "orphaned": true
        }]);
        let instances = parse_instances_value(&value);
        assert_eq!(instances[0].volume_id, Some(42));
        assert_eq!(
            instances[0].version_inode.as_deref(),
            Some("9007199254740993")
        );
        assert_eq!(instances[0].health, "lost");
    }

    #[test]
    fn scrubs_key_value_bearer_and_raw_secret_patterns() {
        let input = "secret: hunter2\nauthorization=abc\nBearer token-value\nABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890abcd";
        let scrubbed = scrub_secrets(input);
        assert!(!scrubbed.contains("hunter2"));
        assert!(!scrubbed.contains("abc\n"));
        assert!(!scrubbed.contains("token-value"));
        assert!(!scrubbed.contains("ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890abcd"));
    }

    #[test]
    fn rejects_profile_path_traversal_ids() {
        assert!(validate_profile_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(validate_profile_id("../outside").is_err());
        assert!(validate_profile_id("").is_err());
    }

    #[test]
    fn rejects_non_path_open_targets() {
        assert!(is_openable_target("/Volumes/MountOS/Team"));
        assert!(!is_openable_target("fileprovider-domain-id"));
    }

    #[test]
    fn sanitizes_export_file_stems() {
        assert_eq!(export_file_stem("Team files", "id"), "Team files");
        assert_eq!(export_file_stem("a/b\\c:d", "id"), "a-b-c-d");
        assert_eq!(export_file_stem("///", "fallback-id"), "fallback-id");
        assert_eq!(export_file_stem("", "fallback-id"), "fallback-id");
    }
}
