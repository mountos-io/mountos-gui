import type { Backend, HealthState } from './types'

export function healthTone(health: HealthState | string): 'success' | 'destructive' | 'warning' | '' {
  if (health === 'healthy') return 'success'
  if (health === 'lost') return 'destructive'
  if (health === 'limited' || health === 'launching') return 'warning'
  return ''
}

/**
 * Explains the state rather than restating it: the dot is the only thing on
 * screen once the Health column is gone, and "limited" or "lost" mean nothing
 * on their own. Doubles as the accessible name, so it must read as a sentence.
 */
export function healthTitle(health: HealthState | string): string {
  if (health === 'healthy') return 'Healthy: mounted and reporting live stats'
  if (health === 'lost') return 'Lost: no live process owns this mount; unmount to reclaim it'
  if (health === 'limited') return 'Limited: mounted, but this backend cannot report live stats'
  if (health === 'launching') return 'Launching: the mount is still starting up'
  return `Unknown state: ${health}`
}

/**
 * `viewMode` is a comma-joined flag string ("r,del", "r,ver", "r,snap", "rw",
 * "r") from Go's MountMode.String(), never an exact-match value — an equality
 * check against a single literal would never fire. Identifies a satellite
 * Deleted/Version/Snapshot view so its row can carry a badge and skip
 * Save-as-profile/Clone-profile (cloning one would silently produce a
 * profile that mounts the wrong thing at that path).
 */
export function viewModeBadge(viewMode?: string): string | null {
  if (!viewMode) return null
  if (viewMode.includes('del')) return 'Deleted view'
  if (viewMode.includes('ver')) return 'Version view'
  if (viewMode.includes('snap')) return 'Snapshot view'
  return null
}

// 'auto' isn't a real backend identity (it means "let the CLI decide"), so it
// stays neutral rather than claiming one of these categorical colors.
const BACKEND_PASTEL: Partial<Record<Backend, string>> = {
  macfuse: 'mount',
  fskit: 'volume',
  nfs: 'storage',
  smb: 'session',
  fileprovider: 'node',
  mountosio: 'region',
  cloudfilter: 'cloudfilter',
}

/** Inline style for a per-backend color-coded Badge; empty for 'auto'/unknown. */
export function backendBadgeStyle(backend?: string): string {
  const tone = BACKEND_PASTEL[backend as Backend]
  if (!tone) return ''
  return pastelBadgeStyle(tone)
}

// Fixed, not derived from the label: the same kind must always read as the
// same color everywhere it appears (Running Instances row, profile form),
// rather than picking one of the 7 tones arbitrarily per render.
const VOLUME_KIND_PASTEL = { general: 'mount', iceberg: 'cloudfilter' } as const

/** Inline style for a color-coded volume-kind Badge (General/Iceberg). */
export function volumeKindBadgeStyle(kind?: string): string {
  const tone = VOLUME_KIND_PASTEL[kind as keyof typeof VOLUME_KIND_PASTEL]
  if (!tone) return ''
  return pastelBadgeStyle(tone)
}

function pastelBadgeStyle(tone: string): string {
  return `background: transparent; color: var(--pastel-${tone}-text); border-color: oklch(from var(--pastel-${tone}) l c h / 0.3);`
}
