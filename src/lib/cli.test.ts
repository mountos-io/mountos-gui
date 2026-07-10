import { describe, expect, it } from 'vitest'
import { buildMountArgv, classifyMountError, parseArgvInput, validateExtraArgs } from './cli'
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
