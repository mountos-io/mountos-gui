import { invoke } from '@tauri-apps/api/core'
import type {
  DesktopSettings,
  DiagnosticsBundle,
  ExportedProfile,
  MountProfile,
  MountResult,
  SecretStatus,
  SystemState,
  UnmountResult,
} from './types'

export function hasDesktopBridge(): boolean {
  return typeof globalThis !== 'undefined' && '__TAURI_INTERNALS__' in globalThis
}

const fallbackProfiles: MountProfile[] = [
  {
    id: 'sample',
    schemaVersion: 1,
    kind: 'mount',
    name: 'Work volume',
    volume: 'vol_demo',
    fork: 'main',
    mountPath: '/Volumes/MountOS/Work',
    discoveryUrl: 'https://hub.mountos.local',
    accessKeyId: 'ABCDEFGHIJKLMNOPQRST',
    secretRef: 'prompt',
    backend: 'auto',
    readOnly: false,
    autoRemount: false,
    temporaryFork: false,
    extraArgs: [],
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
]

export async function getSystemState(): Promise<SystemState> {
  if (!hasDesktopBridge()) {
    return {
      platform: navigator.platform.toLowerCase().includes('win') ? 'windows' : navigator.platform.toLowerCase().includes('mac') ? 'macos' : 'linux',
      checkOk: false,
      cliVersion: 'web preview',
      issues: [
        {
          id: 'preview',
          severity: 'warning',
          title: 'Desktop bridge unavailable',
          detail: 'Run with Tauri to inspect the local mountOS binary.',
        },
      ],
      instances: [
        {
          key: 'sample',
          name: 'Work volume',
          mountPath: '/Volumes/MountOS/Work',
          fsName: 'nfs',
          volumeId: 1,
          external: false,
          health: 'limited',
        },
      ],
      cliPathAlternates: [],
    }
  }
  return invoke<SystemState>('get_system_state')
}

export async function listProfiles(): Promise<MountProfile[]> {
  if (!hasDesktopBridge()) return fallbackProfiles
  return invoke<MountProfile[]>('list_profiles')
}

export async function saveProfile(profile: MountProfile): Promise<MountProfile> {
  if (!hasDesktopBridge()) return { ...profile, updatedAt: new Date().toISOString() }
  return invoke<MountProfile>('save_profile', { profile })
}

export async function deleteProfile(profileId: string): Promise<void> {
  if (!hasDesktopBridge()) return
  await invoke('delete_profile', { profileId })
}

export async function exportProfile(profileId: string): Promise<ExportedProfile> {
  if (!hasDesktopBridge()) throw new Error('Desktop bridge unavailable')
  return invoke<ExportedProfile>('export_profile', { profileId })
}

export async function getSettings(): Promise<DesktopSettings> {
  if (!hasDesktopBridge()) return { defaultBackend: 'auto' }
  return invoke<DesktopSettings>('get_settings')
}

export async function saveSettings(settings: DesktopSettings): Promise<DesktopSettings> {
  if (!hasDesktopBridge()) return settings
  return invoke<DesktopSettings>('save_settings', { settings })
}

export async function setProfileSecret(profileId: string, secret: string): Promise<SecretStatus> {
  if (!hasDesktopBridge()) return { profileId, stored: true }
  return invoke<SecretStatus>('set_profile_secret', { profileId, secret })
}

export async function deleteProfileSecret(profileId: string): Promise<SecretStatus> {
  if (!hasDesktopBridge()) return { profileId, stored: false }
  return invoke<SecretStatus>('delete_profile_secret', { profileId })
}

export async function getProfileSecretStatus(profileId: string): Promise<SecretStatus> {
  if (!hasDesktopBridge()) return { profileId, stored: false }
  return invoke<SecretStatus>('get_profile_secret_status', { profileId })
}

export async function mountProfile(profileId: string, secret?: string): Promise<MountResult> {
  if (!hasDesktopBridge()) throw new Error('Desktop bridge unavailable')
  return invoke<MountResult>('mount_profile', { profileId, secret })
}

export async function unmountTarget(target: string): Promise<UnmountResult> {
  if (!hasDesktopBridge()) throw new Error('Desktop bridge unavailable')
  return invoke<UnmountResult>('unmount_target', { target })
}

export async function openTarget(target: string): Promise<void> {
  if (!hasDesktopBridge()) return
  await invoke('open_target', { target })
}

export async function getInstanceConfig(target: string): Promise<string> {
  if (!hasDesktopBridge()) throw new Error('Desktop bridge unavailable')
  return invoke<string>('get_instance_config', { target })
}

export async function launchDashboard(target: string, gui: boolean): Promise<void> {
  if (!hasDesktopBridge()) throw new Error('Desktop bridge unavailable')
  await invoke('launch_dashboard', { target, gui })
}

export async function createDiagnosticsBundle(): Promise<DiagnosticsBundle> {
  if (!hasDesktopBridge()) return { path: 'Desktop bridge unavailable' }
  return invoke<DiagnosticsBundle>('create_diagnostics_bundle')
}

export async function mcpStatus(): Promise<string> {
  if (!hasDesktopBridge()) return 'Desktop bridge unavailable'
  return invoke<string>('mcp_status')
}

export async function mcpInstall(): Promise<string> {
  if (!hasDesktopBridge()) throw new Error('Desktop bridge unavailable')
  return invoke<string>('mcp_install')
}

export async function mcpUninstall(): Promise<string> {
  if (!hasDesktopBridge()) throw new Error('Desktop bridge unavailable')
  return invoke<string>('mcp_uninstall')
}

export async function mountHelp(): Promise<string> {
  if (!hasDesktopBridge()) throw new Error('Desktop bridge unavailable')
  return invoke<string>('mount_help')
}

export async function showMainWindow(): Promise<void> {
  if (!hasDesktopBridge()) return
  await invoke('show_main_window')
}
