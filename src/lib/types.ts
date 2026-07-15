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
  trustedDiscoveryHost?: string
  extraArgs: string[]
  createdAt: string
  updatedAt: string
}

export interface MountInstance {
  key: string
  name: string
  mountPath: string
  fsName?: string
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

export interface SystemState {
  platform: 'macos' | 'windows' | 'linux' | string
  cliPath?: string
  cliVersion?: string
  checkOk: boolean
  issues: CheckIssue[]
  instances: MountInstance[]
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
}

export interface ExportedProfile {
  path: string
}

export interface DiagnosticsBundle {
  path: string
}

export interface MountResult {
  state: 'ready' | 'indeterminate'
  target: string
}

export interface UnmountResult {
  state: 'idle' | 'flushing'
  target: string
}
