use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};

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
    view_mode: Option<String>,
    project_volume_id: Option<String>,
    volume_id: Option<u32>,
    domain_id: Option<String>,
    unc_path: Option<String>,
    version_inode: Option<String>,
    orphaned: Option<bool>,
    external: bool,
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SecretStatus {
    profile_id: String,
    stored: bool,
}

#[derive(Debug, Serialize)]
struct DiagnosticsBundle {
    path: String,
}

#[derive(Debug, Serialize)]
struct ExportedProfile {
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopSettings {
    default_backend: Backend,
    // Seeds new profiles' discoveryUrl; each profile can still override it
    // independently afterward. Existing profiles are never retroactively
    // rewritten when this changes. Option<T> deserializes missing older
    // settings.json files (pre-dating this field) as None automatically.
    default_discovery_url: Option<String>,
}

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            default_backend: Backend::Auto,
            default_discovery_url: None,
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

const KEYRING_SERVICE: &str = "sh.mountos.desktop";
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

fn mountos_path() -> Result<PathBuf, DesktopError> {
    which::which("mountos").map_err(|_| DesktopError::CliNotFound)
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
                external: true,
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
    match fs::read(settings_path(&app)?) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(DesktopSettings::default()),
        Err(err) => Err(err.into()),
    }
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: DesktopSettings) -> Result<DesktopSettings, DesktopError> {
    validate_backend_for_platform(&settings.default_backend)?;
    let path = settings_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(&settings)?)?;
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
        .map(|profile| profile.mount_path)
        .filter(|target| !target.is_empty())
        .collect::<Vec<_>>();
    for instance in &mut instances {
        instance.external = !profile_targets
            .iter()
            .any(|target| targets_equal(target, &instance.mount_path));
    }
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
    let payload = serde_json::json!({
        "createdAtUnix": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        "cliPath": mountos_path().ok().map(|path| path.display().to_string()),
        "cliVersion": cli_version(),
        "check": check.as_ref().map(|output| serde_json::json!({
            "status": output.status.code(),
            "stdout": scrub_output(&output.stdout),
            "stderr": scrub_output(&output.stderr),
        })),
        "list": list.as_ref().map(|output| serde_json::json!({
            "status": output.status.code(),
            "stdout": scrub_output(&output.stdout),
            "stderr": scrub_output(&output.stderr),
        })),
        "profiles": profiles.into_iter().map(|profile| serde_json::json!({
            "id": profile.id,
            "name": profile.name,
            "kind": profile.kind,
            "mountPath": profile.mount_path,
            "discoveryUrl": profile.discovery_url,
            "backend": profile.backend,
            "secretRef": profile.secret_ref,
            "extraArgsCount": profile.extra_args.len(),
            "autoRemount": profile.auto_remount,
        })).collect::<Vec<_>>(),
    });
    fs::write(&path, serde_json::to_vec_pretty(&payload)?)?;
    Ok(DiagnosticsBundle {
        path: path.display().to_string(),
    })
}

pub fn run() {
    tauri::Builder::default()
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
            open_target,
            create_diagnostics_bundle,
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
