import { describe, expect, it } from 'vitest'
import { backendNeedsMountPath, buildMountArgv, classifyMountError, parseArgvInput, validateExtraArgs, validateMountPathForBackend } from './cli'
import type { MountProfile } from './types'

const profile: MountProfile = {
  id: 'p1',
  schemaVersion: 1,
  kind: 'mount',
  name: 'Demo',
  volume: 'vol-1',
  fork: 'main',
  mountPath: '/Volumes/MountOS/Demo',
  discoveryUrl: 'https://hub.example.com',
  accessKeyId: 'ABCDEFGHIJKLMNOPQRST',
  secretRef: 'vault',
  backend: 'nfs',
  readOnly: true,
  autoRemount: false,
  extraArgs: ['--cache-size', '10G'],
  createdAt: '2026-07-10T00:00:00Z',
  updatedAt: '2026-07-10T00:00:00Z',
}

describe('cli helpers', () => {
  it('builds managed mount argv without the secret value', () => {
    expect(buildMountArgv(profile)).toEqual([
      'mount',
      '--discovery-url',
      'https://hub.example.com',
      '--volname',
      'vol-1',
      '--fork-name',
      'main',
      '-m',
      '/Volumes/MountOS/Demo',
      '-a',
      'ABCDEFGHIJKLMNOPQRST',
      '-s',
      '--read-only',
      '--nfs',
      '--cache-size',
      '10G',
    ])
  })

  it('does not add secret flags when access key id is empty', () => {
    const argv = buildMountArgv({ ...profile, accessKeyId: '' })
    expect(argv).not.toContain('-a')
    expect(argv).not.toContain('-s')
  })

  it('omits -m for FileProvider/CloudFilter even with a leftover mountPath', () => {
    const argv = buildMountArgv({ ...profile, backend: 'fileprovider', mountPath: '/some/leftover/path' })
    expect(argv).not.toContain('-m')
  })

  it('rejects managed extra args and duplicate positionals', () => {
    expect(validateExtraArgs(['--smb', '--secret-access-key=x', '-sa', '/tmp/mount'])).toEqual([
      '--smb',
      '--secret-access-key=x',
      '-sa',
      '/tmp/mount',
    ])
  })

  it('allows separate values for unmanaged flags', () => {
    expect(validateExtraArgs(['--disk-cache-size', '10G', '--debug'])).toEqual([])
  })

  it('reports FileProvider and CloudFilter as not needing a mount path', () => {
    expect(backendNeedsMountPath('fileprovider')).toBe(false)
    expect(backendNeedsMountPath('cloudfilter')).toBe(false)
    expect(backendNeedsMountPath('nfs')).toBe(true)
    expect(backendNeedsMountPath('fskit')).toBe(true)
  })

  it('validates FSKit mount path prefix', () => {
    expect(validateMountPathForBackend('fskit', '/Volumes/MountOS/Team')).toBeNull()
    expect(validateMountPathForBackend('fskit', '/Volumes/MountOS/Team/')).toBeNull()
    expect(validateMountPathForBackend('fskit', '/Volumes/MountOS/')).not.toBeNull()
    expect(validateMountPathForBackend('fskit', '/Volumes/MountOS')).not.toBeNull()
    expect(validateMountPathForBackend('fskit', '/tmp/Team')).not.toBeNull()
    expect(validateMountPathForBackend('fskit', '')).not.toBeNull()
    expect(validateMountPathForBackend('nfs', '/tmp/anything')).toBeNull()
    expect(validateMountPathForBackend('nfs', '')).toBeNull()
  })

  it('validates short flag clusters', () => {
    // A managed short flag anywhere before the '-o' value-absorbing point
    // is caught, regardless of position in the cluster.
    expect(validateExtraArgs(['-am'])).toEqual(['-am'])
    expect(validateExtraArgs(['-ma'])).toEqual(['-ma'])
    // '-o' takes a fused value (mirrors real short-opt parsing: once a
    // value-taking flag is hit in a cluster, the rest of the token is its
    // value, not further flags) — bare '-o' and '-o<value>' are both
    // accepted even when the value text collides with a managed letter.
    expect(validateExtraArgs(['-o'])).toEqual([])
    expect(validateExtraArgs(['-oallow_other'])).toEqual([])
    expect(validateExtraArgs(['-oa'])).toEqual([])
    // Bare "--" (positional separator) is rejected like any other
    // non-managed-but-suspicious positional.
    expect(validateExtraArgs(['--'])).toEqual(['--'])
  })

  it('preserves quoted and escaped extra-argument values', () => {
    expect(parseArgvInput('--disk-cache-size 10G --mount-opts "allow_other,volname=Team Files"')).toEqual([
      '--disk-cache-size',
      '10G',
      '--mount-opts',
      'allow_other,volname=Team Files',
    ])
  })

  it('classifies pinned error strings', () => {
    expect(classifyMountError('authentication failed - invalid access key or secret')).toBe('auth')
    expect(classifyMountError('mount point /x is not empty')).toBe('mountpoint')
    expect(classifyMountError('did not become ready within 30s')).toBe('indeterminate')
  })
})
