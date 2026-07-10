<script lang="ts">
  import {
    AlertTriangle,
    CheckCircle2,
    Copy,
    Database,
    FileArchive,
    FileDown,
    FilePlus,
    FolderOpen,
    HardDrive,
    KeyRound,
    MonitorDot,
    Plus,
    Power,
    RefreshCw,
    Save,
    Search,
    Settings,
    ShieldCheck,
    Trash2,
    Unplug,
    X,
  } from '@lucide/svelte'
  import { buildMountArgv, classifyMountError, errorClassLabel, parseArgvInput, validateExtraArgs } from './lib/cli'
  import {
    createDiagnosticsBundle,
    deleteProfile,
    deleteProfileSecret,
    exportProfile,
    getProfileSecretStatus,
    getSettings,
    getSystemState,
    listProfiles,
    mountProfile,
    openTarget,
    saveProfile,
    saveSettings,
    setProfileSecret,
    unmountTarget,
  } from './lib/tauri'
  import type { Backend, DesktopSettings, MountInstance, MountProfile, SystemState } from './lib/types'

  type View = 'instances' | 'profiles' | 'health' | 'settings'

  let view = $state<View>('instances')
  let profiles = $state<MountProfile[]>([])
  let systemState = $state<SystemState>({ platform: 'macos', checkOk: false, issues: [], instances: [] })
  let selectedProfileId = $state<string | null>(null)
  let query = $state('')
  let busy = $state(false)
  let message = $state('')
  let messageKind = $state<'info' | 'error'>('info')
  let commandText = $state('')
  let rejectedArgs = $state<string[]>([])
  let extraArgsInput = $state('')
  let extraArgsError = $state('')
  let secretPromptFor = $state<string | null>(null)
  let secretValue = $state('')
  let secretError = $state('')
  let savePromptedSecret = $state(false)
  let secretDialog = $state<HTMLDialogElement | undefined>()
  let deletePromptFor = $state<MountProfile | null>(null)
  let deleteDialog = $state<HTMLDialogElement | undefined>()
  let settings = $state<DesktopSettings>({ defaultBackend: 'auto' })
  let vaultStatus = $state<Record<string, boolean>>({})
  let diagnosticsPath = $state('')

  // Lost-mount detection compares only against snapshots taken during THIS
  // session, so pre-existing state at startup is never classified as a loss.
  let knownInstanceKeys = new Set<string>()
  const expectedGone = new Set<string>()

  const selectedProfile = $derived(profiles.find((profile) => profile.id === selectedProfileId) ?? profiles[0])
  const filteredInstances = $derived(
    systemState.instances.filter((instance) => {
      const haystack = `${instance.name} ${instance.mountPath} ${instance.fsName ?? ''} ${instance.volumeId ?? ''}`.toLowerCase()
      return haystack.includes(query.toLowerCase())
    }),
  )
  const limitedCount = $derived(systemState.instances.filter((instance) => instance.health === 'limited').length)

  function notify(text: string, kind: 'info' | 'error' = 'info') {
    message = text
    messageKind = kind
  }

  function describeError(error: unknown) {
    const text = error instanceof Error ? error.message : String(error)
    return `${errorClassLabel(classifyMountError(text))}. ${text}`
  }

  function detectLost(next: SystemState) {
    const nextKeys = new Set(next.instances.map((instance) => instance.key))
    for (const key of knownInstanceKeys) {
      if (nextKeys.has(key)) continue
      if (expectedGone.delete(key)) continue
      notify(`Mount disappeared: ${key}`, 'error')
    }
    knownInstanceKeys = nextKeys
  }

  async function pollSystem() {
    if (busy || secretPromptFor || deletePromptFor) return
    try {
      const nextState = await getSystemState()
      detectLost(nextState)
      systemState = nextState
    } catch {
      // Silent; the manual refresh path reports errors.
    }
  }

  async function refresh(announce = true) {
    busy = true
    try {
      const [nextState, nextProfiles] = await Promise.all([getSystemState(), listProfiles()])
      detectLost(nextState)
      systemState = nextState
      profiles = nextProfiles
      selectedProfileId ??= nextProfiles[0]?.id ?? null
      const selected = nextProfiles.find((profile) => profile.id === selectedProfileId) ?? nextProfiles[0]
      extraArgsInput = selected ? selected.extraArgs.map(quoteArg).join(' ') : ''
      await refreshVaultStatus(nextProfiles)
      updatePreview()
      if (announce) notify(`Refreshed ${nextState.instances.length} instance${nextState.instances.length === 1 ? '' : 's'}`)
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Refresh failed', 'error')
    } finally {
      busy = false
    }
  }

  function newProfile(preset: Partial<MountProfile> = {}) {
    const now = new Date().toISOString()
    const profile: MountProfile = {
      id: crypto.randomUUID(),
      schemaVersion: 1,
      kind: 'mount',
      name: 'New profile',
      volume: '',
      fork: 'main',
      mountPath: '',
      discoveryUrl: '',
      accessKeyId: '',
      secretRef: 'prompt',
      backend: backends.includes(settings.defaultBackend) ? settings.defaultBackend : 'auto',
      readOnly: false,
      autoRemount: false,
      extraArgs: [],
      createdAt: now,
      updatedAt: now,
      ...preset,
    }
    profiles = [profile, ...profiles]
    selectedProfileId = profile.id
    extraArgsInput = profile.extraArgs.map(quoteArg).join(' ')
    extraArgsError = ''
    view = 'profiles'
    updatePreview(profile)
  }

  function saveAsProfile(instance: MountInstance) {
    newProfile({
      name: instance.name || 'External mount',
      volume: instance.name ?? '',
      mountPath: instance.mountPath,
    })
    notify('Profile created from the running mount. Add the discovery URL and credentials, then save.')
  }

  function duplicateSelected() {
    if (!selectedProfile) return
    const { id: _id, createdAt: _created, updatedAt: _updated, ...rest } = selectedProfile
    newProfile({
      ...rest,
      name: `${selectedProfile.name} copy`,
      secretRef: 'prompt',
    })
  }

  async function exportSelected() {
    if (!selectedProfile) return
    busy = true
    try {
      const exported = await exportProfile(selectedProfile.id)
      notify(`Profile exported to ${exported.path}`)
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Profile export failed', 'error')
    } finally {
      busy = false
    }
  }

  function cancelDelete() {
    deletePromptFor = null
  }

  async function confirmDelete() {
    const target = deletePromptFor
    if (!target) return
    busy = true
    try {
      await deleteProfile(target.id)
      profiles = profiles.filter((profile) => profile.id !== target.id)
      if (selectedProfileId === target.id) selectedProfileId = profiles[0]?.id ?? null
      const { [target.id]: _removed, ...rest } = vaultStatus
      vaultStatus = rest
      notify(`Deleted profile ${target.name} and its vaulted secret`)
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Profile delete failed', 'error')
    } finally {
      busy = false
      deletePromptFor = null
    }
  }

  $effect(() => {
    const dialog = deleteDialog
    if (!dialog) return
    if (deletePromptFor && !dialog.open) dialog.showModal()
    else if (!deletePromptFor && dialog.open) dialog.close()
  })

  async function persistSelected() {
    if (!selectedProfile) return
    busy = true
    try {
      const saved = await saveProfile({ ...selectedProfile, updatedAt: new Date().toISOString() })
      profiles = profiles.map((profile) => (profile.id === saved.id ? saved : profile))
      selectedProfileId = saved.id
      notify('Profile saved')
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Profile save failed', 'error')
    } finally {
      busy = false
    }
  }

  function updatePreview(profile = selectedProfile) {
    if (!profile) {
      commandText = ''
      rejectedArgs = []
      return
    }
    commandText = `mountos ${buildMountArgv(profile).map(quoteArg).join(' ')}`
    rejectedArgs = validateExtraArgs(profile.extraArgs)
  }

  function quoteArg(arg: string) {
    if (/^[A-Za-z0-9_./:@%+=,-]+$/.test(arg)) return arg
    return `'${arg.replaceAll("'", "'\\''")}'`
  }

  function patchProfile(patch: Partial<MountProfile>) {
    if (!selectedProfile) return
    const next = { ...selectedProfile, ...patch }
    profiles = profiles.map((profile) => (profile.id === selectedProfile.id ? next : profile))
    updatePreview(next)
  }

  function setExtraArgs(value: string) {
    extraArgsInput = value
    try {
      patchProfile({ extraArgs: parseArgvInput(value) })
      extraArgsError = ''
    } catch (error) {
      extraArgsError = error instanceof Error ? error.message : 'Invalid extra args'
    }
  }

  async function runMount(profile: MountProfile) {
    busy = true
    let saved: MountProfile
    try {
      saved = await saveProfile({ ...profile, updatedAt: new Date().toISOString() })
      profiles = profiles.map((candidate) => (candidate.id === saved.id ? saved : candidate))
      if (!profiles.some((candidate) => candidate.id === saved.id)) profiles = [saved, ...profiles]
      selectedProfileId = saved.id
      const stored = saved.secretRef === 'vault' ? (await getProfileSecretStatus(saved.id)).stored : false
      vaultStatus = { ...vaultStatus, [saved.id]: stored }
      if (saved.secretRef === 'prompt' || !stored) {
        secretPromptFor = saved.id
        secretValue = ''
        secretError = ''
        savePromptedSecret = saved.secretRef === 'vault'
        return
      }
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Profile save failed', 'error')
      return
    } finally {
      busy = false
    }
    await doMount(saved.id)
  }

  async function doMount(profileId: string, secret?: string) {
    busy = true
    try {
      if (secret && savePromptedSecret) {
        await setProfileSecret(profileId, secret)
        profiles = profiles.map((profile) => (profile.id === profileId ? { ...profile, secretRef: 'vault' } : profile))
        const profile = profiles.find((candidate) => candidate.id === profileId)
        if (profile) await saveProfile({ ...profile, secretRef: 'vault', updatedAt: new Date().toISOString() })
        vaultStatus = { ...vaultStatus, [profileId]: true }
      }
      const result = await mountProfile(profileId, secret)
      secretPromptFor = null
      secretValue = ''
      secretError = ''
      await refresh(false)
      notify(`Mount ready at ${result.target}`)
    } catch (error) {
      if (secret !== undefined && secretPromptFor) {
        secretError = describeError(error)
      } else {
        notify(describeError(error), 'error')
      }
    } finally {
      busy = false
    }
  }

  function cancelSecret() {
    secretPromptFor = null
    secretValue = ''
    secretError = ''
  }

  $effect(() => {
    const dialog = secretDialog
    if (!dialog) return
    if (secretPromptFor && !dialog.open) dialog.showModal()
    else if (!secretPromptFor && dialog.open) dialog.close()
  })

  async function refreshVaultStatus(nextProfiles = profiles) {
    const entries = await Promise.all(nextProfiles.map(async (profile) => [profile.id, (await getProfileSecretStatus(profile.id)).stored] as const))
    vaultStatus = Object.fromEntries(entries)
  }

  async function forgetSecret(profileId: string) {
    busy = true
    try {
      await deleteProfileSecret(profileId)
      vaultStatus = { ...vaultStatus, [profileId]: false }
      const profile = profiles.find((candidate) => candidate.id === profileId)
      if (profile?.secretRef === 'vault') {
        const updated = { ...profile, secretRef: 'prompt' as const, updatedAt: new Date().toISOString() }
        await saveProfile(updated)
        profiles = profiles.map((candidate) => (candidate.id === profileId ? updated : candidate))
      }
      notify('Secret forgotten')
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Failed to forget secret', 'error')
    } finally {
      busy = false
    }
  }

  async function createBundle() {
    busy = true
    try {
      const bundle = await createDiagnosticsBundle()
      diagnosticsPath = bundle.path
      notify('Diagnostics bundle created')
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Diagnostics bundle failed', 'error')
    } finally {
      busy = false
    }
  }

  async function runUnmount(instance: MountInstance) {
    busy = true
    expectedGone.add(instance.key)
    try {
      const result = await unmountTarget(instance.domainId || instance.mountPath)
      await refresh(false)
      notify(result.state === 'idle' ? 'Unmount complete' : 'Unmount is still flushing in the background')
    } catch (error) {
      expectedGone.delete(instance.key)
      notify(error instanceof Error ? error.message : 'Unmount failed', 'error')
    } finally {
      busy = false
    }
  }

  async function runOpen(instance: MountInstance) {
    try {
      await openTarget(instance.mountPath)
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Failed to open mount target', 'error')
    }
  }

  function canOpen(instance: MountInstance) {
    return instance.mountPath.startsWith('/') || /^[A-Za-z]:[\\/]?$/.test(instance.mountPath) || /^[A-Za-z]:[\\/]/.test(instance.mountPath)
  }

  const navItems: Array<{ id: View; label: string; icon: typeof MonitorDot }> = [
    { id: 'instances', label: 'Instances', icon: MonitorDot },
    { id: 'profiles', label: 'Profiles', icon: HardDrive },
    { id: 'health', label: 'Health', icon: ShieldCheck },
    { id: 'settings', label: 'Settings', icon: Settings },
  ]

  const backends = $derived<Backend[]>(
    systemState.platform === 'windows'
      ? ['auto', 'mountosio', 'cloudfilter']
      : systemState.platform === 'macos'
        ? ['auto', 'macfuse', 'fskit', 'nfs', 'smb', 'fileprovider']
        : ['auto', 'nfs'],
  )

  function initialDark() {
    if (typeof localStorage === 'undefined') return false
    const stored = localStorage.getItem('mountos-desktop-dark')
    if (stored !== null) return stored === 'true'
    return typeof matchMedia !== 'undefined' && matchMedia('(prefers-color-scheme: dark)').matches
  }
  let dark = $state(initialDark())

  function setDark(next: boolean) {
    dark = next
    if (typeof localStorage !== 'undefined') localStorage.setItem('mountos-desktop-dark', String(next))
  }

  $effect(() => {
    document.documentElement.classList.toggle('dark', dark)
  })

  async function loadSettings() {
    try {
      settings = await getSettings()
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Failed to load settings', 'error')
    }
  }

  async function changeDefaultBackend(backend: Backend) {
    try {
      settings = await saveSettings({ ...settings, defaultBackend: backend })
      notify(`New profiles default to the ${backend} backend`)
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Failed to save settings', 'error')
    }
  }

  $effect(() => {
    void loadSettings()
    void refresh(false)
  })

  $effect(() => {
    let timer: ReturnType<typeof setInterval> | undefined
    const schedule = () => {
      clearInterval(timer)
      timer = setInterval(() => {
        void pollSystem()
      }, document.hidden ? 30_000 : 5_000)
    }
    schedule()
    document.addEventListener('visibilitychange', schedule)
    return () => {
      clearInterval(timer)
      document.removeEventListener('visibilitychange', schedule)
    }
  })

  function viewTitle(nextView: View) {
    return nextView === 'instances' ? 'Instances' : nextView === 'profiles' ? 'Profiles' : nextView === 'health' ? 'Health' : 'Settings'
  }
</script>

<svelte:head>
  <title>mountOS Desktop</title>
</svelte:head>

<div class="app-shell">
  <aside class="sidebar">
    <div class="brand">
      <div class="mark" aria-hidden="true"><Database size={18} /></div>
      <h1>mountOS Desktop</h1>
    </div>

    <nav aria-label="Primary">
      {#each navItems as item}
        <button
          class:active={view === item.id}
          class="nav-btn"
          type="button"
          aria-current={view === item.id ? 'page' : undefined}
          onclick={() => (view = item.id)}
        >
          <item.icon size={18} aria-hidden="true" />
          <span>{item.label}</span>
        </button>
      {/each}
    </nav>

    <div class="sidebar-footer">
      <span class="badge {systemState.checkOk ? 'success' : 'warning'}">
        {#if systemState.checkOk}<CheckCircle2 size={14} aria-hidden="true" />{:else}<AlertTriangle size={14} aria-hidden="true" />{/if}
        {systemState.cliVersion ?? 'CLI pending'}
      </span>
    </div>
  </aside>

  <main class="main" aria-busy={busy}>
    <header class="topbar">
      <h2>{viewTitle(view)}</h2>
      <div class="topbar-actions">
        {#if view === 'instances'}
          <label class="search">
            <Search size={16} aria-hidden="true" />
            <span class="sr-only">Search instances</span>
            <input bind:value={query} placeholder="Filter mounts" />
          </label>
        {/if}
        <button class="btn icon-btn" type="button" title="Refresh" aria-label="Refresh" onclick={() => refresh()} disabled={busy}>
          <span class="refresh-icon" class:spin={busy}><RefreshCw size={17} aria-hidden="true" /></span>
        </button>
        <button class="btn primary" type="button" onclick={() => newProfile()} disabled={busy}>
          <Plus size={17} aria-hidden="true" />
          Profile
        </button>
      </div>
    </header>

    {#if message}
      <div class="notice" class:error={messageKind === 'error'} role={messageKind === 'error' ? 'alert' : 'status'}>
        <span>{message}</span>
        <button class="btn ghost icon-btn notice-dismiss" type="button" aria-label="Dismiss message" onclick={() => (message = '')}>
          <X size={15} aria-hidden="true" />
        </button>
      </div>
    {/if}

    {#if view === 'instances'}
      <section class="surface corner-brackets panel">
        <div class="panel-head">
          <h3>Running instances</h3>
          <div class="row-actions">
            <span class="badge">{systemState.instances.length} running</span>
            {#if limitedCount > 0}
              <span class="badge warning">{limitedCount} limited</span>
            {/if}
          </div>
        </div>
        <div class="table-wrap">
          <table class="table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Target</th>
                <th>Backend</th>
                <th>Health</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {#each filteredInstances as instance}
                <tr>
                  <td>
                    <strong>{instance.name || instance.volumeId || 'mountOS volume'}</strong>
                    {#if instance.external}<span class="badge">External</span>{/if}
                  </td>
                  <td><code>{instance.mountPath}</code></td>
                  <td><span class="badge mount">{instance.fsName ?? 'unknown'}</span></td>
                  <td>{@render HealthBadge(instance.health)}</td>
                  <td>
                    <div class="row-actions">
                      <button
                        class="btn icon-btn"
                        type="button"
                        title={canOpen(instance) ? 'Open folder' : 'No local folder for this mount'}
                        aria-label="Open folder"
                        disabled={busy || !canOpen(instance)}
                        onclick={() => runOpen(instance)}
                      >
                        <FolderOpen size={16} aria-hidden="true" />
                      </button>
                      {#if instance.external && canOpen(instance)}
                        <button class="btn icon-btn" type="button" title="Save as profile" aria-label="Save as profile" disabled={busy} onclick={() => saveAsProfile(instance)}>
                          <FilePlus size={16} aria-hidden="true" />
                        </button>
                      {/if}
                      <button class="btn icon-btn destructive" type="button" title="Unmount" aria-label="Unmount" disabled={busy} onclick={() => runUnmount(instance)}>
                        <Unplug size={16} aria-hidden="true" />
                      </button>
                    </div>
                  </td>
                </tr>
              {:else}
                <tr>
                  <td colspan="5">
                    <div class="empty">
                      <strong>No instances</strong>
                      <p>Mount a saved profile, or mount from the CLI; same-user mounts appear here after refresh.</p>
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>
    {:else if view === 'profiles'}
      <section class="profiles-layout">
        <div class="surface panel">
          <h3>Saved profiles</h3>
          <div class="profile-list">
            {#each profiles as profile}
              <button class:active={selectedProfileId === profile.id} class="profile-row" type="button" onclick={() => { selectedProfileId = profile.id; extraArgsInput = profile.extraArgs.map(quoteArg).join(' '); extraArgsError = ''; updatePreview(profile) }}>
                <HardDrive size={17} aria-hidden="true" />
                <span>
                  <strong>{profile.name}</strong>
                  <small>{profile.mountPath || 'No target selected'}</small>
                </span>
              </button>
            {/each}
          </div>
        </div>

        {#if selectedProfile}
          <form class="surface corner-brackets panel editor" onsubmit={(event) => { event.preventDefault(); void persistSelected() }}>
            <div class="panel-head">
              <h3>{selectedProfile.name}</h3>
              <div class="row-actions">
                <button class="btn icon-btn" type="button" title="Duplicate profile" aria-label="Duplicate profile" disabled={busy} onclick={duplicateSelected}>
                  <Copy size={16} aria-hidden="true" />
                </button>
                <button class="btn icon-btn" type="button" title="Export profile (no secret)" aria-label="Export profile" disabled={busy} onclick={exportSelected}>
                  <FileDown size={16} aria-hidden="true" />
                </button>
                <button class="btn icon-btn destructive" type="button" title="Delete profile" aria-label="Delete profile" disabled={busy} onclick={() => (deletePromptFor = selectedProfile)}>
                  <Trash2 size={16} aria-hidden="true" />
                </button>
                <button class="btn" type="button" disabled={busy || !!extraArgsError || rejectedArgs.length > 0} onclick={() => runMount(selectedProfile)}>
                  <Power size={16} aria-hidden="true" />
                  Mount
                </button>
                <button class="btn primary" type="submit" disabled={busy || !!extraArgsError || rejectedArgs.length > 0}>
                  <Save size={16} aria-hidden="true" />
                  Save
                </button>
              </div>
            </div>

            <div class="form-grid">
              <label class="field">
                <span>Name</span>
                <input class="input" value={selectedProfile.name} oninput={(e) => patchProfile({ name: e.currentTarget.value })} />
              </label>
              <label class="field">
                <span>Backend</span>
                <select class="select" value={selectedProfile.backend} onchange={(e) => patchProfile({ backend: e.currentTarget.value as Backend })}>
                  {#each backends as backend}<option value={backend}>{backend}</option>{/each}
                </select>
              </label>
              <label class="field">
                <span>Discovery URL</span>
                <input class="input" value={selectedProfile.discoveryUrl} oninput={(e) => patchProfile({ discoveryUrl: e.currentTarget.value })} />
              </label>
              <label class="field">
                <span>Access key ID</span>
                <input class="input" value={selectedProfile.accessKeyId} maxlength="20" oninput={(e) => patchProfile({ accessKeyId: e.currentTarget.value })} />
              </label>
              <label class="field">
                <span>Volume name</span>
                <input class="input" value={selectedProfile.volume} oninput={(e) => patchProfile({ volume: e.currentTarget.value })} />
              </label>
              <label class="field">
                <span>Fork</span>
                <input class="input" value={selectedProfile.fork} oninput={(e) => patchProfile({ fork: e.currentTarget.value })} />
              </label>
              <label class="field">
                <span>Mount path</span>
                <input class="input" value={selectedProfile.mountPath} oninput={(e) => patchProfile({ mountPath: e.currentTarget.value })} />
              </label>
              <label class="field">
                <span>Secret</span>
                <select class="select" value={selectedProfile.secretRef} onchange={(e) => patchProfile({ secretRef: e.currentTarget.value as 'vault' | 'prompt' })}>
                  <option value="prompt">Prompt on mount</option>
                  <option value="vault">Vault</option>
                </select>
              </label>
            </div>

            <div class="vault-row">
              <span class="badge {vaultStatus[selectedProfile.id] ? 'success' : 'warning'}">
                <KeyRound size={14} aria-hidden="true" />
                {vaultStatus[selectedProfile.id] ? 'Secret stored' : 'No vaulted secret'}
              </span>
              <button class="btn destructive" type="button" disabled={!vaultStatus[selectedProfile.id] || busy} onclick={() => forgetSecret(selectedProfile.id)}>
                <Trash2 size={16} aria-hidden="true" />
                Forget secret
              </button>
            </div>

            <div class="toggles">
              <label><input type="checkbox" checked={selectedProfile.readOnly} onchange={(e) => patchProfile({ readOnly: e.currentTarget.checked })} /> Read only</label>
            </div>

            <label class="field">
              <span>Extra args</span>
              <textarea class="textarea" value={extraArgsInput} oninput={(e) => setExtraArgs(e.currentTarget.value)} placeholder="Only unmanaged mount flags"></textarea>
            </label>

            {#if extraArgsError}
              <div class="callout warning">
                <AlertTriangle size={17} aria-hidden="true" />
                <span>{extraArgsError}</span>
              </div>
            {/if}

            {#if rejectedArgs.length}
              <div class="callout warning">
                <AlertTriangle size={17} aria-hidden="true" />
                <span>Rejected managed flags: {rejectedArgs.join(', ')}</span>
              </div>
            {/if}

            <div class="command-preview">
              <p class="mono-label">COMMAND PREVIEW</p>
              <code>{commandText || `mountos ${buildMountArgv(selectedProfile).join(' ')}`}</code>
            </div>
          </form>
        {/if}
      </section>
    {:else if view === 'health'}
      <section class="surface corner-brackets panel">
        <div class="panel-head">
          <h3>Backend readiness</h3>
          <div class="row-actions">
            <button class="btn" type="button" onclick={createBundle} disabled={busy}>
              <FileArchive size={16} aria-hidden="true" />
              Bundle
            </button>
            <span class="badge {systemState.checkOk ? 'success' : 'warning'}">{systemState.checkOk ? 'Ready' : 'Needs attention'}</span>
          </div>
        </div>
        {#if diagnosticsPath}
          <div class="command-preview diagnostics-path">
            <p class="mono-label">LOCAL BUNDLE</p>
            <code>{diagnosticsPath}</code>
          </div>
        {/if}
        <div class="issue-list">
          {#each systemState.issues as issue}
            <article class="issue">
              <AlertTriangle size={18} class={issue.severity} aria-hidden="true" />
              <div>
                <strong>{issue.title}</strong>
                {#if issue.detail}<p>{issue.detail}</p>{/if}
                {#if issue.fixCommand}<code>{issue.fixCommand}</code>{/if}
              </div>
            </article>
          {:else}
            <article class="issue">
              <CheckCircle2 size={18} aria-hidden="true" />
              <div><strong>No reported issues</strong><p>The CLI check command did not return repair items.</p></div>
            </article>
          {/each}
        </div>
      </section>
    {:else}
      <section class="surface panel settings-panel">
        <h3>Desktop policies</h3>
        <label class="setting-row">
          <span><strong>Dark mode</strong><small>Low-light console palette. Follows the system setting until you choose.</small></span>
          <input type="checkbox" checked={dark} onchange={(e) => setDark(e.currentTarget.checked)} />
        </label>
        <label class="setting-row">
          <span><strong>Default backend</strong><small>Applied to new profiles. Auto follows the CLI's platform order.</small></span>
          <select class="select setting-select" value={settings.defaultBackend} onchange={(e) => changeDefaultBackend(e.currentTarget.value as Backend)}>
            {#each backends as backend}<option value={backend}>{backend}</option>{/each}
          </select>
        </label>
      </section>
    {/if}
  </main>
</div>

<dialog class="modal" bind:this={secretDialog} onclose={cancelSecret} aria-labelledby="secret-dialog-title">
  {#if secretPromptFor}
    <form onsubmit={(event) => { event.preventDefault(); void doMount(secretPromptFor!, secretValue) }}>
      <KeyRound size={22} aria-hidden="true" />
      <h3 id="secret-dialog-title">Enter secret access key</h3>
      <p>The secret is written to child stdin with a trailing newline. It is never placed on argv or in the environment by the GUI.</p>
      <!-- svelte-ignore a11y_autofocus -->
      <input class="input" type="password" bind:value={secretValue} autocomplete="current-password" autofocus aria-label="Secret access key" />
      <label class="save-secret">
        <input type="checkbox" bind:checked={savePromptedSecret} />
        Store in OS vault for this profile
      </label>
      {#if secretError}
        <div class="callout warning" role="alert">
          <AlertTriangle size={17} aria-hidden="true" />
          <span>{secretError}</span>
        </div>
      {/if}
      <div class="row-actions">
        <button class="btn" type="button" onclick={cancelSecret}>Cancel</button>
        <button class="btn primary" type="submit" disabled={busy || !secretValue}>Mount</button>
      </div>
    </form>
  {/if}
</dialog>

<dialog class="modal" bind:this={deleteDialog} onclose={cancelDelete} aria-labelledby="delete-dialog-title">
  {#if deletePromptFor}
    <form onsubmit={(event) => { event.preventDefault(); void confirmDelete() }}>
      <Trash2 size={22} aria-hidden="true" />
      <h3 id="delete-dialog-title">Delete profile</h3>
      <p>Deletes "{deletePromptFor.name}" and its vaulted secret. Running mounts are not affected.</p>
      <div class="row-actions">
        <button class="btn" type="button" onclick={cancelDelete}>Cancel</button>
        <button class="btn destructive" type="submit" disabled={busy}>Delete</button>
      </div>
    </form>
  {/if}
</dialog>

{#snippet HealthBadge(health: string)}
  <span class="badge {health === 'healthy' ? 'success' : health === 'lost' ? 'destructive' : health === 'limited' || health === 'launching' ? 'warning' : ''}">
    <span class="led {health === 'lost' ? 'destructive' : health === 'limited' || health === 'launching' ? 'warning' : ''}" aria-hidden="true"></span>
    {health}
  </span>
{/snippet}

<style>
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 18px 16px;
    border-bottom: 1px solid var(--border);
  }

  .brand h1 {
    font-size: 1.25rem;
  }

  .mark {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border: 1px solid var(--primary);
    color: var(--primary);
    flex-shrink: 0;
    clip-path: polygon(0 8px, 8px 0, 100% 0, 100% calc(100% - 8px), calc(100% - 8px) 100%, 0 100%);
  }

  nav {
    display: grid;
    gap: 4px;
    padding: 12px;
  }

  .nav-btn,
  .profile-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 44px;
    border: 1px solid transparent;
    border-radius: 0;
    background: transparent;
    color: var(--foreground);
    cursor: pointer;
    padding: 8px 10px;
    text-align: left;
  }

  .nav-btn:hover,
  .nav-btn.active,
  .profile-row:hover,
  .profile-row.active {
    border-color: var(--border);
    background: var(--accent);
  }

  .sidebar-footer {
    padding: 12px;
  }

  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 22px;
    border-bottom: 1px solid var(--border);
  }

  .topbar h2 {
    font-size: 1.875rem;
  }

  .topbar-actions,
  .row-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 220px;
    border: 1px solid var(--border);
    background: var(--input);
    padding: 0 10px;
  }

  .search input {
    width: 100%;
    height: 34px;
    border: 0;
    background: transparent;
    color: var(--foreground);
    outline: 0;
  }

  .notice {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin: 16px 22px 0;
    border: 1px solid var(--border);
    background: var(--accent);
    padding: 4px 4px 4px 10px;
  }

  .notice.error {
    border-color: oklch(from var(--destructive) l c h / 0.45);
    background: oklch(from var(--destructive) l c h / 0.08);
    color: var(--destructive);
  }

  .notice-dismiss {
    width: 30px;
    height: 30px;
    min-width: 30px;
    min-height: 30px;
    flex-shrink: 0;
  }

  .refresh-icon {
    display: inline-flex;
  }

  .refresh-icon.spin {
    animation: spin 1.2s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .panel {
    margin: 22px;
    padding: 16px;
  }

  .panel-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 14px;
  }

  .panel-head h3,
  .panel h3 {
    font-size: 1.25rem;
  }

  .table-wrap {
    overflow: auto;
  }

  code {
    color: var(--foreground);
    font-family: ui-monospace, Menlo, monospace;
    font-size: 0.95rem;
    letter-spacing: 0;
  }

  .empty {
    display: grid;
    gap: 6px;
    padding: 28px;
    text-align: center;
  }

  .profiles-layout {
    display: grid;
    grid-template-columns: minmax(220px, 0.34fr) minmax(0, 1fr);
    gap: 16px;
    padding: 22px;
  }

  .profiles-layout .panel {
    margin: 0;
  }

  .profile-list {
    display: grid;
    gap: 4px;
    margin-top: 12px;
  }

  .profile-row span {
    min-width: 0;
  }

  .profile-row strong,
  .profile-row small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .profile-row small,
  .setting-row small {
    color: var(--muted-foreground);
    font-size: 1rem;
  }

  .editor {
    display: grid;
    gap: 16px;
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
  }

  .field {
    display: grid;
    gap: 6px;
  }

  .field > span {
    color: var(--label-foreground);
    font-size: 1rem;
    font-weight: 500;
  }

  .toggles {
    display: flex;
    gap: 18px;
    flex-wrap: wrap;
  }

  .toggles label,
  .save-secret,
  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .save-secret {
    justify-content: flex-start;
  }

  .setting-row > span {
    display: grid;
    gap: 2px;
  }

  .setting-select {
    width: auto;
    min-width: 160px;
  }

  .settings-panel {
    display: grid;
    gap: 12px;
  }

  .vault-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    border: 1px solid var(--border);
    padding: 10px;
  }

  .callout {
    display: flex;
    align-items: center;
    gap: 10px;
    border: 1px solid var(--border);
    padding: 10px;
  }

  .callout.warning {
    border-color: oklch(from var(--warning) l c h / 0.45);
    color: var(--warning);
  }

  .command-preview {
    display: grid;
    gap: 8px;
    overflow: auto;
    border: 1px solid var(--border);
    background: var(--muted);
    padding: 12px;
  }

  .issue-list {
    display: grid;
    gap: 12px;
  }

  .issue {
    display: flex;
    gap: 12px;
    border: 1px solid var(--border);
    padding: 12px;
  }

  .modal {
    width: min(460px, calc(100% - 40px));
    border: 1px solid var(--border);
    background: var(--card);
    color: var(--card-foreground);
    padding: 18px;
  }

  .modal form {
    display: grid;
    gap: 14px;
  }

  .modal::backdrop {
    background: oklch(0.07 0.005 200 / 0.72);
  }

  @media (max-width: 860px) {
    .profiles-layout {
      grid-template-columns: 1fr;
    }

    .form-grid {
      grid-template-columns: 1fr;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .refresh-icon.spin {
      animation: none;
    }
  }
</style>
