import type { Backend, ErrorClass, MountProfile } from './types'

// UI-only mirror of src-tauri/src/lib.rs's validate_extra_args — gives the
// user inline "rejected" feedback before they hit Save/Mount. The Rust side
// independently re-validates everything from the on-disk profile before
// acting, so this copy is not itself a security boundary, but it IS a
// hand-synced duplicate: any change to the flag sets below (or to the
// short-cluster scan logic) must be mirrored in lib.rs's managed_flags()/
// boolean_long_flags(), and both must stay in sync with mountos-servers'
// cmd/mfuse CLI flag surface. No automated check currently catches drift.
const managedFlags = new Set([
  'a',
  'access-key-id',
  's',
  'secret-access-key',
  'discovery-url',
  'm',
  'mount',
  'mount-point',
  'foreground',
  'f',
  'gateway',
  'gateway-only',
  'fork-name',
  'volname',
  'n',
  'read-only',
  'r',
  'disk-cache-dir',
  'backend',
  'macfuse',
  'fskit',
  'k',
  'nfs',
  'N',
  'smb',
  'fileprovider',
  'F',
])

// gateway-* flags are deliberately excluded: validateExtraArgs rejects any
// long flag starting with "gateway-" outright (see the `rawName.startsWith
// ('gateway-')` check below), regardless of what's listed here, since
// gateway profiles aren't supported yet (save_profile hard-rejects any kind
// other than "mount"). Revisit once gateway profile support ships.
const booleanLongFlags = new Set([
  'acl',
  'agent',
  'blockserv-auto-degrade',
  'browse',
  'debug',
  'disable-cache-dir',
  'ioctl',
  'null-permissions',
  'session-audit',
  'xattr',
])

const shortValueFlags = new Set(['o'])

const backendArgv: Partial<Record<Backend, string[]>> = {
  macfuse: ['--macfuse'],
  fskit: ['--fskit'],
  nfs: ['--nfs'],
  smb: ['--smb'],
  fileprovider: ['--fileprovider'],
  mountosio: ['--backend', 'mountosio'],
  cloudfilter: ['--backend', 'cloudfilter'],
}

export function validateExtraArgs(args: string[]): string[] {
  const rejected: string[] = []

  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i]
    if (!arg.startsWith('-')) {
      rejected.push(arg)
      continue
    }

    if (arg === '--') {
      rejected.push(arg)
      continue
    }

    if (arg.startsWith('--')) {
      const [rawName] = arg.slice(2).split('=', 1)
      if (managedFlags.has(rawName) || rawName.startsWith('gateway-')) {
        rejected.push(arg)
        if (!arg.includes('=') && args[i + 1] && !args[i + 1].startsWith('-')) {
          rejected.push(args[i + 1])
          i += 1
        }
      } else if (!arg.includes('=') && !booleanLongFlags.has(rawName) && args[i + 1] && !args[i + 1].startsWith('-')) {
        i += 1
      }
      continue
    }

    const cluster = arg.slice(1)
    for (const ch of cluster) {
      if (managedFlags.has(ch)) {
        rejected.push(arg)
        break
      }
      if (shortValueFlags.has(ch)) break
    }
  }

  return rejected
}

export function buildMountArgv(profile: MountProfile): string[] {
  const argv = ['mount']

  if (profile.discoveryUrl) argv.push('--discovery-url', profile.discoveryUrl)
  if (profile.volume) argv.push('--volname', profile.volume)
  if (profile.fork) argv.push('--fork-name', profile.fork)
  if (profile.mountPath) argv.push('-m', profile.mountPath)
  if (profile.accessKeyId) argv.push('-a', profile.accessKeyId, '-s')
  if (profile.readOnly) argv.push('--read-only')
  if (profile.cacheDir) argv.push('--disk-cache-dir', profile.cacheDir)

  const backend = backendArgv[profile.backend]
  if (backend) argv.push(...backend)

  argv.push(...profile.extraArgs)
  return argv
}

export function parseArgvInput(input: string): string[] {
  const args: string[] = []
  let current = ''
  let quote: 'single' | 'double' | null = null
  let escaped = false
  let started = false

  for (const ch of input) {
    if (escaped) {
      current += ch
      escaped = false
      started = true
      continue
    }
    if (ch === '\\' && quote !== 'single') {
      escaped = true
      started = true
      continue
    }
    if (ch === "'" && quote !== 'double') {
      quote = quote === 'single' ? null : 'single'
      started = true
      continue
    }
    if (ch === '"' && quote !== 'single') {
      quote = quote === 'double' ? null : 'double'
      started = true
      continue
    }
    if (/\s/.test(ch) && quote === null) {
      if (started) args.push(current)
      current = ''
      started = false
      continue
    }
    current += ch
    started = true
  }

  if (escaped || quote !== null) throw new Error('Unterminated quote or escape in extra args')
  if (started) args.push(current)
  return args
}

const errorClassLabels: Record<ErrorClass, string> = {
  'cli-unavailable': 'mountos CLI unavailable',
  auth: 'Authentication failed',
  'network-discovery': 'Network or discovery problem',
  'backend-missing': 'Backend not ready',
  mountpoint: 'Mount point problem',
  capacity: 'Cache or capacity problem',
  'volume-state': 'Volume state prevents this mount',
  indeterminate: 'Launch did not confirm in time',
  unknown: 'Mount failed',
}

export function errorClassLabel(errorClass: ErrorClass): string {
  return errorClassLabels[errorClass]
}

export function classifyMountError(text: string): ErrorClass {
  const stderr = text.toLowerCase()
  if (!stderr.trim()) return 'unknown'
  if (stderr.includes('no such file') || stderr.includes('not found') || stderr.includes('quarantine')) return 'cli-unavailable'
  if (stderr.includes('authentication failed') || stderr.includes('invalid access key or secret')) return 'auth'
  if (stderr.includes('discovery') || stderr.includes('connection refused') || stderr.includes('timeout') || stderr.includes('dns')) return 'network-discovery'
  if (stderr.includes('backend') || stderr.includes('driver') || stderr.includes('macfuse') || stderr.includes('fskit')) return 'backend-missing'
  if (stderr.includes('mount point') || stderr.includes('not empty') || stderr.includes('already exists') || stderr.includes('busy')) return 'mountpoint'
  if (stderr.includes('cache') || stderr.includes('no space') || stderr.includes('quota')) return 'capacity'
  if (stderr.includes('deleted volume') || stderr.includes('lake') || stderr.includes('iceberg')) return 'volume-state'
  if (stderr.includes('did not become ready within') || stderr.includes('no readiness signal')) return 'indeterminate'
  return 'unknown'
}
