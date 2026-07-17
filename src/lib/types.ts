export type Backend = 'auto' | 'macfuse' | 'fskit' | 'nfs' | 'smb' | 'fileprovider' | 'mountosio' | 'cloudfilter'
export type SecretRef = 'vault' | 'prompt'
export type ProfileKind = 'mount' | 'gateway'
export type HealthState = 'healthy' | 'launching' | 'flushing' | 'limited' | 'lost' | 'stalled' | 'reconnecting' | 'idle'
export type ErrorClass =
  | 'cli-unavailable'
  | 'auth'
  | 'network-discovery'
  | 'backend-missing'
  | 'mountpoint'
  | 'capacity'
  | 'volume-state'
  | 'indeterminate'
  | 'unknown'

export interface MountProfile {
  id: string
  schemaVersion: 1
  kind: ProfileKind
  name: string
  volume: string
  fork: string
  mountPath: string
  discoveryUrl: string
  accessKeyId: string
  secretRef: SecretRef
  backend: Backend
  cacheDir?: string
  readOnly: boolean
  autoRemount: boolean
  temporaryFork: boolean
  trustedDiscoveryHost?: string
  extraArgs: string[]
  createdAt: string
  updatedAt: string
}

export interface MountInstance {
  key: string
  name: string
  mountPath: string
  /** Device string ("mountos:<volume>"). Identifies the volume, not the backend. */
  fsName?: string
  /** Transport the mount runs on, from `mountos list`. Absent on older CLIs. */
  backend?: Backend
  viewMode?: string
  projectVolumeId?: string
  volumeId?: number
  domainId?: string
  uncPath?: string
  versionInode?: string
  orphaned?: boolean
  external: boolean
  health: HealthState
}

export interface CheckIssue {
  id: string
  severity: 'info' | 'warning' | 'error'
  title: string
  detail?: string
  fixCommand?: string
}

export interface TerminalOption {
  id: string
  label: string
}

export interface SystemState {
  platform: 'macos' | 'windows' | 'linux' | string
  cliPath?: string
  cliVersion?: string
  checkOk: boolean
  issues: CheckIssue[]
  instances: MountInstance[]
  // Other mountos binaries found on PATH besides the one in use (empty in
  // the common single-install case). Surfaces ambiguity instead of
  // silently trusting whichever PATH match resolved first.
  cliPathAlternates: string[]
  // Terminal emulators detected on this machine, in preference order. The
  // settings picker offers exactly these, so it can only list installed ones.
  terminals: TerminalOption[]
}

export interface SecretStatus {
  profileId: string
  stored: boolean
}

export interface DesktopSettings {
  defaultBackend: Backend
  // Seeds new profiles' discoveryUrl; each profile can still override it
  // independently afterward. Existing profiles are never retroactively
  // rewritten when this changes.
  defaultDiscoveryUrl?: string
  // Pins an exact mountos binary instead of the first PATH match. Once
  // set, a moved/missing pinned binary is a hard error rather than a
  // silent fallback to a different install.
  cliPathOverride?: string
  // How often the mount list refreshes while the window is visible, in seconds.
  // Undefined means the default. A hidden window backs off regardless.
  pollSeconds?: number
  // Terminal emulator id for the dashboard launcher. Empty/undefined means the
  // platform's stock terminal. Unlike cliPathOverride this is a preference, not
  // a pin: an uninstalled choice falls back instead of failing.
  terminal?: string
}

export interface ExportedProfile {
  path: string
}

export interface DiagnosticsCommandOutput {
  status: number | null
  stdout: unknown
  stderr: unknown
}

export interface DiagnosticsProfileSummary {
  id: string
  name: string
  kind: string
  mountPath: string
  discoveryUrl: string
  backend: Backend
  secretRef: SecretRef
  extraArgsCount: number
  autoRemount: boolean
}

export interface DiagnosticsContent {
  createdAtUnix: number
  cliPath?: string
  cliVersion?: string
  check?: DiagnosticsCommandOutput
  list?: DiagnosticsCommandOutput
  profiles: DiagnosticsProfileSummary[]
}

export interface DiagnosticsBundle {
  path: string
  content?: DiagnosticsContent
}

export interface MountResult {
  state: 'ready' | 'indeterminate'
  target: string
}

export interface UnmountResult {
  state: 'idle' | 'flushing'
  target: string
}

export interface UnmountAllResult {
  attempted: number
  failed: string[]
}
