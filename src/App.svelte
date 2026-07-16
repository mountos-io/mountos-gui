<script lang="ts">
  import {
    AlertTriangle,
    Bot,
    CheckCircle2,
    ChevronDown,
    ChevronRight,
    Copy,
    FileArchive,
    FileDown,
    FilePlus,
    FolderOpen,
    HardDrive,
    KeyRound,
    LayoutDashboard,
    Lightbulb,
    Monitor,
    MonitorDot,
    Moon,
    PanelLeftClose,
    PanelLeftOpen,
    Plus,
    Power,
    RefreshCw,
    Save,
    Search,
    Settings,
    ShieldCheck,
    SquareTerminal,
    Sun,
    Trash2,
    Unplug,
    X,
  } from '@lucide/svelte'
  import { backendNeedsMountPath, buildMountArgv, classifyMountError, errorClassLabel, FSKIT_MOUNT_PREFIX, isAbsolutePath, isValidFolderName, parseArgvInput, validateExtraArgs, validateMountPathForBackend } from './lib/cli'
  import { healthTone } from './lib/health'
  import { applyTheme, loadTheme, saveTheme } from './lib/theme'
  import type { Theme } from './lib/theme'
  import {
    browseFolder,
    createDiagnosticsBundle,
    deleteProfile,
    deleteProfileSecret,
    exportProfile,
    getInstanceConfig,
    getProfileSecretStatus,
    getSettings,
    getSystemState,
    launchDashboard,
    listProfiles,
    mcpInstall,
    mcpStatus,
    mcpUninstall,
    mountHelp,
    mountProfile,
    openTarget,
    saveProfile,
    saveSettings,
    setProfileSecret,
    unmountTarget,
    unmountAllTargets,
  } from './lib/tauri'
  import type { Backend, DesktopSettings, DiagnosticsBundle, DiagnosticsCommandOutput, MountInstance, MountProfile, SystemState } from './lib/types'

  type View = 'instances' | 'profiles' | 'health' | 'settings'

  // mountOS access key IDs are fixed-length; this only checks length (not
  // charset) since that's the one constraint the GUI can enforce cheaply.
  const ACCESS_KEY_ID_LENGTH = 20

  let view = $state<View>('instances')
  let loaded = $state(false)
  let profiles = $state<MountProfile[]>([])
  let systemState = $state<SystemState>({ platform: 'macos', checkOk: false, issues: [], instances: [], cliPathAlternates: [] })
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
  let unmountPromptFor = $state<MountInstance | 'all' | null>(null)
  let unmountDialog = $state<HTMLDialogElement | undefined>()
  let settings = $state<DesktopSettings>({ defaultBackend: 'auto' })
  let vaultStatus = $state<Record<string, boolean>>({})
  let diagnosticsBundle = $state<DiagnosticsBundle | null>(null)
  let mcpStatusText = $state('')
  let expandedConfig = $state<Record<string, string>>({})
  let mountHelpText = $state('')
  let mountHelpVisible = $state(false)

  // Lost-mount detection compares only against snapshots taken during THIS
  // session, so pre-existing state at startup is never classified as a loss.
  let knownInstances = new Map<string, string>()
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
    const nextInstances = new Map(next.instances.map((instance) => [instance.key, instance.mountPath || instance.name]))
    for (const [key, label] of knownInstances) {
      if (nextInstances.has(key)) continue
      if (expectedGone.delete(key)) continue
      notify(`Mount disappeared: ${label}`, 'error')
    }
    knownInstances = nextInstances
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
      loaded = true
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
      discoveryUrl: settings.defaultDiscoveryUrl ?? '',
      accessKeyId: '',
      secretRef: 'prompt',
      backend: backends.includes(settings.defaultBackend) ? settings.defaultBackend : 'auto',
      readOnly: false,
      autoRemount: false,
      temporaryFork: false,
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

  function setAccessKeyId(value: string) {
    if (!selectedProfile) return
    // Vault storage needs an access key ID to pair the vaulted secret with;
    // clearing it while Vault is selected would leave an orphaned choice.
    const clearingVault = !value && selectedProfile.secretRef === 'vault'
    patchProfile({ accessKeyId: value, ...(clearingVault ? { secretRef: 'prompt' } : {}) })
  }

  async function browseMountPath() {
    if (!selectedProfile) return
    try {
      const isFskit = selectedProfile.backend === 'fskit'
      const selected = await browseFolder('Choose mount folder', isFskit ? '/Volumes' : undefined)
      if (!selected) return
      // FSKit's mount point is a fixed container (/Volumes/MountOS/<name>);
      // browsing picks that container, and the leaf folder name comes from
      // the volume name already set above, so it isn't retyped here too.
      const volume = selectedProfile.volume
      const mountPath = isFskit && volume && isValidFolderName(volume) ? `${selected.replace(/\/+$/, '')}/${volume}` : selected
      patchProfile({ mountPath })
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Failed to open folder picker', 'error')
    }
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
      diagnosticsBundle = await createDiagnosticsBundle()
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

  async function runUnmountAll() {
    const keys = systemState.instances.map((instance) => instance.key)
    if (keys.length === 0) return
    busy = true
    for (const key of keys) expectedGone.add(key)
    try {
      const result = await unmountAllTargets()
      for (const failedTarget of result.failed) expectedGone.delete(failedTarget)
      await refresh(false)
      if (result.failed.length === 0) {
        notify(`Unmounted all ${result.attempted} mounts`)
      } else {
        notify(`Unmounted ${result.attempted - result.failed.length} of ${result.attempted}; ${result.failed.length} failed`, 'error')
      }
    } catch (error) {
      for (const key of keys) expectedGone.delete(key)
      notify(error instanceof Error ? error.message : 'Unmount all failed', 'error')
    } finally {
      busy = false
    }
  }

  function requestUnmount(instance: MountInstance) {
    if (skipUnmountConfirm) {
      void runUnmount(instance)
    } else {
      unmountPromptFor = instance
    }
  }

  function requestUnmountAll() {
    if (systemState.instances.length === 0) return
    if (skipUnmountConfirm) {
      void runUnmountAll()
    } else {
      unmountPromptFor = 'all'
    }
  }

  function cancelUnmountPrompt() {
    unmountPromptFor = null
  }

  async function confirmUnmountPrompt() {
    const target = unmountPromptFor
    unmountPromptFor = null
    if (target === 'all') await runUnmountAll()
    else if (target) await runUnmount(target)
  }

  $effect(() => {
    const dialog = unmountDialog
    if (!dialog) return
    if (unmountPromptFor && !dialog.open) dialog.showModal()
    else if (!unmountPromptFor && dialog.open) dialog.close()
  })

  async function runOpen(instance: MountInstance) {
    try {
      await openTarget(instance.mountPath)
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Failed to open mount target', 'error')
    }
  }

  function canOpen(instance: MountInstance) {
    return isAbsolutePath(instance.mountPath)
  }

  async function toggleInstanceConfig(instance: MountInstance) {
    if (instance.key in expandedConfig) {
      const next = { ...expandedConfig }
      delete next[instance.key]
      expandedConfig = next
      return
    }
    try {
      const config = await getInstanceConfig(instance.mountPath)
      expandedConfig = { ...expandedConfig, [instance.key]: config }
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Failed to read mount config', 'error')
    }
  }

  async function openDashboard(instance: MountInstance, gui: boolean) {
    try {
      await launchDashboard(instance.mountPath, gui)
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Failed to launch dashboard', 'error')
    }
  }

  async function toggleMountHelp() {
    if (mountHelpVisible) {
      mountHelpVisible = false
      return
    }
    if (!mountHelpText) {
      try {
        mountHelpText = await mountHelp()
      } catch (error) {
        notify(error instanceof Error ? error.message : 'Failed to load mountos mount -h', 'error')
        return
      }
    }
    mountHelpVisible = true
  }

  const navItems: Array<{ id: View; label: string; icon: typeof MonitorDot }> = [
    { id: 'instances', label: 'Instances', icon: MonitorDot },
    { id: 'profiles', label: 'Profiles', icon: HardDrive },
    { id: 'health', label: 'Health', icon: ShieldCheck },
    { id: 'settings', label: 'Settings', icon: Settings },
  ]

  const themeOptions: Array<{ value: Theme; label: string; icon: typeof Sun }> = [
    { value: 'light', label: 'Light', icon: Sun },
    { value: 'dark', label: 'Dark', icon: Moon },
    { value: 'system', label: 'System', icon: Monitor },
  ]

  const backends = $derived<Backend[]>(
    systemState.platform === 'windows'
      ? ['auto', 'mountosio', 'cloudfilter']
      : systemState.platform === 'macos'
        ? ['auto', 'macfuse', 'fskit', 'nfs', 'smb', 'fileprovider']
        : ['auto', 'nfs'],
  )

  const mountPathIsManaged = $derived(selectedProfile ? !backendNeedsMountPath(selectedProfile.backend) : false)
  const mountPathError = $derived.by(() => {
    if (!selectedProfile || mountPathIsManaged) return ''
    if (!selectedProfile.mountPath.trim()) return 'Mount path is required for this backend'
    return validateMountPathForBackend(selectedProfile.backend, selectedProfile.mountPath) ?? ''
  })
  const accessKeyError = $derived.by(() => {
    if (!selectedProfile || !selectedProfile.accessKeyId) return ''
    return selectedProfile.accessKeyId.length === ACCESS_KEY_ID_LENGTH ? '' : `Access key ID must be ${ACCESS_KEY_ID_LENGTH} characters`
  })
  // Only FSKit turns the volume name into a filesystem path segment
  // (browseMountPath appends it to the picked folder); other backends just
  // pass it through as --volname, so it isn't constrained there.
  const volumeNameError = $derived.by(() => {
    if (!selectedProfile || selectedProfile.backend !== 'fskit' || !selectedProfile.volume) return ''
    return isValidFolderName(selectedProfile.volume) ? '' : 'Volume name must be a valid folder name (no /, \\, or control characters)'
  })

  let theme = $state<Theme>(loadTheme())

  function setTheme(next: Theme) {
    theme = next
    saveTheme(theme)
  }

  function initialSidebarCollapsed() {
    if (typeof localStorage === 'undefined') return false
    return localStorage.getItem('mountos-desktop-sidebar-collapsed') === 'true'
  }
  let sidebarCollapsed = $state(initialSidebarCollapsed())

  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed
    if (typeof localStorage !== 'undefined') localStorage.setItem('mountos-desktop-sidebar-collapsed', String(sidebarCollapsed))
  }

  function initialSkipUnmountConfirm() {
    if (typeof localStorage === 'undefined') return false
    return localStorage.getItem('mountos-desktop-skip-unmount-confirm') === 'true'
  }
  let skipUnmountConfirm = $state(initialSkipUnmountConfirm())

  function setSkipUnmountConfirm(next: boolean) {
    skipUnmountConfirm = next
    if (typeof localStorage !== 'undefined') localStorage.setItem('mountos-desktop-skip-unmount-confirm', String(next))
  }

  $effect(() => {
    applyTheme(theme)
  })

  $effect(() => {
    if (typeof matchMedia === 'undefined') return
    const query = matchMedia('(prefers-color-scheme: dark)')
    const onChange = () => {
      if (theme === 'system') applyTheme(theme)
    }
    query.addEventListener('change', onChange)
    return () => query.removeEventListener('change', onChange)
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

  async function changeDefaultDiscoveryUrl(discoveryUrl: string) {
    const trimmed = discoveryUrl.trim()
    try {
      settings = await saveSettings({ ...settings, defaultDiscoveryUrl: trimmed || undefined })
      notify(trimmed ? 'New profiles default to this discovery URL' : 'Default discovery URL cleared')
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Failed to save settings', 'error')
    }
  }

  async function changeCliPathOverride(path: string) {
    const trimmed = path.trim()
    try {
      settings = await saveSettings({ ...settings, cliPathOverride: trimmed || undefined })
      notify(trimmed ? 'Pinned mountos CLI path' : 'CLI path pin cleared, using PATH lookup again')
      await refresh(false)
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Failed to save settings', 'error')
    }
  }

  async function checkMcpStatus() {
    busy = true
    try {
      mcpStatusText = await mcpStatus()
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Failed to check MCP status', 'error')
    } finally {
      busy = false
    }
  }

  async function installMcp() {
    busy = true
    try {
      mcpStatusText = await mcpInstall()
      notify('mountos registered as an MCP server')
    } catch (error) {
      notify(error instanceof Error ? error.message : 'MCP install failed', 'error')
    } finally {
      busy = false
    }
  }

  async function uninstallMcp() {
    busy = true
    try {
      mcpStatusText = await mcpUninstall()
      notify('mountos MCP server removed')
    } catch (error) {
      notify(error instanceof Error ? error.message : 'MCP uninstall failed', 'error')
    } finally {
      busy = false
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

<div class="app-shell" class:sidebar-collapsed={sidebarCollapsed}>
  <aside class="sidebar">
    <div class="brand" data-tauri-drag-region="deep">
      <img class="mark" src="/logo.png" alt="" width="36" height="36" />
      {#if !sidebarCollapsed}<h1>mountOS</h1>{/if}
    </div>

    <nav aria-label="Primary">
      {#each navItems as item}
        <button
          class:active={view === item.id}
          class="nav-btn"
          type="button"
          title={sidebarCollapsed ? item.label : undefined}
          aria-current={view === item.id ? 'page' : undefined}
          onclick={() => (view = item.id)}
        >
          <item.icon size={18} aria-hidden="true" />
          {#if !sidebarCollapsed}<span>{item.label}</span>{/if}
        </button>
      {/each}
      <button class="nav-btn" type="button" title={sidebarCollapsed ? 'Expand sidebar' : undefined} onclick={toggleSidebar}>
        {#if sidebarCollapsed}<PanelLeftOpen size={18} aria-hidden="true" />{:else}<PanelLeftClose size={18} aria-hidden="true" />{/if}
        {#if !sidebarCollapsed}<span>Collapse</span>{/if}
      </button>
    </nav>

    <button class="sidebar-footer" type="button" title="mountOS CLI status, see Settings for details" onclick={() => (view = 'settings')}>
      <span class="led" class:warning={!systemState.checkOk}></span>
      {#if !sidebarCollapsed}<span>{systemState.checkOk ? 'CLI ready' : 'CLI issue'}</span>{/if}
    </button>
  </aside>

  <main class="main" aria-busy={busy}>
    <header class="topbar" data-tauri-drag-region="deep">
      <h2>{viewTitle(view)}</h2>
      <div class="topbar-actions">
        {#if view === 'instances'}
          <label class="search">
            <Search size={16} aria-hidden="true" />
            <span class="sr-only">Search instances</span>
            <input bind:value={query} placeholder="Filter mounts" />
          </label>
        {/if}
        <button class="btn icon-btn ghost" type="button" title="Refresh" aria-label="Refresh" onclick={() => refresh()} disabled={busy}>
          <span class="refresh-icon" class:spin={busy}><RefreshCw size={17} aria-hidden="true" /></span>
        </button>
        <button class="btn primary" type="button" onclick={() => newProfile()} disabled={busy}>
          <Plus size={17} aria-hidden="true" />
          Profile
        </button>
      </div>
    </header>

    <div class="main-content">
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
            <button
              class="btn destructive"
              type="button"
              disabled={busy || systemState.instances.length === 0}
              onclick={requestUnmountAll}
            >
              <Unplug size={16} aria-hidden="true" />
              Unmount all
            </button>
          </div>
        </div>
        <div class="table-wrap">
          <table class="table">
            <thead>
              <tr>
                <th scope="col">Name</th>
                <th scope="col">Target</th>
                <th scope="col">Backend</th>
                <th scope="col">Health</th>
                <th scope="col">Actions</th>
              </tr>
            </thead>
            <tbody>
              {#if !loaded}
                {#each { length: 3 } as _placeholder}
                  <tr aria-hidden="true">
                    <td><span class="skeleton"></span></td>
                    <td><span class="skeleton wide"></span></td>
                    <td><span class="skeleton narrow"></span></td>
                    <td><span class="skeleton narrow"></span></td>
                    <td><span class="skeleton narrow"></span></td>
                  </tr>
                {/each}
              {:else}
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
                      {#if canOpen(instance)}
                        <button
                          class="btn icon-btn"
                          type="button"
                          title={instance.key in expandedConfig ? 'Hide mount flags' : 'Show mount flags'}
                          aria-label={instance.key in expandedConfig ? 'Hide mount flags' : 'Show mount flags'}
                          aria-expanded={instance.key in expandedConfig}
                          disabled={busy}
                          onclick={() => toggleInstanceConfig(instance)}
                        >
                          {#if instance.key in expandedConfig}<ChevronDown size={16} aria-hidden="true" />{:else}<ChevronRight size={16} aria-hidden="true" />{/if}
                        </button>
                      {/if}
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
                      {#if canOpen(instance)}
                        <button class="btn icon-btn" type="button" title="Launch TUI dashboard in a terminal" aria-label="Launch TUI dashboard" disabled={busy} onclick={() => openDashboard(instance, false)}>
                          <SquareTerminal size={16} aria-hidden="true" />
                        </button>
                        <button class="btn icon-btn" type="button" title="Launch GUI dashboard in a terminal" aria-label="Launch GUI dashboard" disabled={busy} onclick={() => openDashboard(instance, true)}>
                          <LayoutDashboard size={16} aria-hidden="true" />
                        </button>
                      {/if}
                      {#if instance.external && canOpen(instance)}
                        <button class="btn icon-btn" type="button" title="Save as profile" aria-label="Save as profile" disabled={busy} onclick={() => saveAsProfile(instance)}>
                          <FilePlus size={16} aria-hidden="true" />
                        </button>
                      {/if}
                      <button class="btn icon-btn destructive" type="button" title="Unmount" aria-label="Unmount" disabled={busy} onclick={() => requestUnmount(instance)}>
                        <Unplug size={16} aria-hidden="true" />
                      </button>
                    </div>
                  </td>
                </tr>
                {#if instance.key in expandedConfig}
                  <tr class="config-row">
                    <td colspan="5">
                      <div class="command-preview">
                        <p class="mono-label">MOUNT FLAGS (.mountOS/.config)</p>
                        <pre><code>{expandedConfig[instance.key]}</code></pre>
                      </div>
                    </td>
                  </tr>
                {/if}
              {:else}
                <tr>
                  <td colspan="5">
                    <div class="empty tech-grid">
                      <strong>No instances</strong>
                      <p>Mount a saved profile, or mount from the CLI; active mounts appear here after refresh.</p>
                    </div>
                  </td>
                </tr>
              {/each}
              {/if}
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
            {:else}
              <div class="empty">
                <p>No saved profiles yet.</p>
              </div>
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
                <button class="btn" type="button" disabled={busy || !!extraArgsError || rejectedArgs.length > 0 || !!mountPathError || !!accessKeyError || !!volumeNameError} onclick={() => runMount(selectedProfile)}>
                  <Power size={16} aria-hidden="true" />
                  Mount
                </button>
                <button class="btn primary" type="submit" disabled={busy || !!extraArgsError || rejectedArgs.length > 0 || !!mountPathError || !!accessKeyError || !!volumeNameError}>
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
                <input class="input" value={selectedProfile.accessKeyId} maxlength={ACCESS_KEY_ID_LENGTH} oninput={(e) => setAccessKeyId(e.currentTarget.value)} />
                {#if accessKeyError}
                  <small class="field-error">{accessKeyError}</small>
                {/if}
              </label>
              <label class="field">
                <span>Volume name</span>
                <input class="input" value={selectedProfile.volume} oninput={(e) => patchProfile({ volume: e.currentTarget.value })} />
                {#if volumeNameError}
                  <small class="field-error">{volumeNameError}</small>
                {:else if selectedProfile.backend === 'fskit'}
                  <small>Used as the mount point's folder name under {FSKIT_MOUNT_PREFIX}</small>
                {/if}
              </label>
              <label class="field">
                <span>Fork</span>
                <input class="input" value={selectedProfile.fork} oninput={(e) => patchProfile({ fork: e.currentTarget.value })} />
              </label>
              <div class="field">
                <label for="mount-path">Mount path</label>
                {#if mountPathIsManaged}
                  <input id="mount-path" class="input" value={selectedProfile.mountPath} disabled placeholder="Managed automatically by the OS" />
                  <small>
                    {selectedProfile.backend === 'fileprovider'
                      ? 'FileProvider mounts have no filesystem path; the volume appears in Finder under its volume name.'
                      : 'CloudFilter mounts have no filesystem path; the volume appears under its own drive/namespace.'}
                  </small>
                {:else}
                  <div class="input-with-action">
                    <input
                      id="mount-path"
                      class="input"
                      value={selectedProfile.mountPath}
                      placeholder={selectedProfile.backend === 'fskit' ? '/Volumes/MountOS/<name>' : undefined}
                      oninput={(e) => patchProfile({ mountPath: e.currentTarget.value })}
                    />
                    <button class="btn" type="button" onclick={browseMountPath} disabled={busy} title="Choose a folder">
                      <FolderOpen size={16} aria-hidden="true" />
                      Browse
                    </button>
                  </div>
                {/if}
              </div>
              <label class="field">
                <span>Secret</span>
                <select class="select" value={selectedProfile.secretRef} onchange={(e) => patchProfile({ secretRef: e.currentTarget.value as 'vault' | 'prompt' })}>
                  <option value="prompt">Prompt on mount</option>
                  <option value="vault" disabled={!selectedProfile.accessKeyId}>Vault</option>
                </select>
                {#if !selectedProfile.accessKeyId}
                  <small>Vault storage needs an access key ID first.</small>
                {/if}
              </label>
            </div>

            {#if mountPathError}
              <div class="callout warning">
                <AlertTriangle size={17} aria-hidden="true" />
                <span>{mountPathError}</span>
              </div>
            {/if}

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
              <label title="Creates an ephemeral per-session fork for this mount; discarded when it unmounts, the underlying volume is never touched">
                <input type="checkbox" checked={selectedProfile.temporaryFork} onchange={(e) => patchProfile({ temporaryFork: e.currentTarget.checked })} /> Temporary fork
              </label>
            </div>

            <div class="field">
              <div class="field-head">
                <label for="advanced-options">Advanced options</label>
                <button class="btn ghost small" type="button" onclick={toggleMountHelp} disabled={busy} aria-expanded={mountHelpVisible}>
                  {mountHelpVisible ? 'Hide help' : 'mountos mount -h'}
                </button>
              </div>
              <textarea
                id="advanced-options"
                class="textarea"
                value={extraArgsInput}
                oninput={(e) => setExtraArgs(e.currentTarget.value)}
                placeholder="Flags mountos mount accepts but this form doesn't manage, e.g. --disk-cache-size 10G"
              ></textarea>
            </div>

            {#if mountHelpVisible}
              <div class="command-preview">
                <p class="mono-label">MOUNTOS MOUNT -H</p>
                <pre><code>{mountHelpText}</code></pre>
              </div>
            {/if}

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
        {:else}
          <div class="surface panel empty tech-grid profile-empty">
            <FilePlus size={28} aria-hidden="true" />
            <strong>No profile selected</strong>
            <p>Save a profile to mount a mountOS volume in one click, with credentials in the OS vault and the exact CLI command shown before every action.</p>
            <button class="btn primary" type="button" onclick={() => newProfile()}>
              <Plus size={17} aria-hidden="true" />
              New profile
            </button>
          </div>
        {/if}
      </section>
    {:else if view === 'health'}
      <section class="surface corner-brackets panel">
        <div class="panel-head">
          <h3>Backend readiness</h3>
          <div class="row-actions">
            <button class="btn" type="button" onclick={() => refresh()} disabled={busy} title="Re-run mountos check --json">
              <RefreshCw size={16} aria-hidden="true" />
              Run check
            </button>
            <button class="btn" type="button" onclick={createBundle} disabled={busy}>
              <FileArchive size={16} aria-hidden="true" />
              Bundle
            </button>
            <span class="badge {systemState.checkOk ? 'success' : 'warning'}">{systemState.checkOk ? 'Ready' : 'Needs attention'}</span>
          </div>
        </div>
        {#if diagnosticsBundle}
          <div class="diagnostics-report">
            <div class="command-preview diagnostics-path">
              <p class="mono-label">LOCAL BUNDLE</p>
              <code>{diagnosticsBundle.path}</code>
            </div>
            {#if diagnosticsBundle.content}
              {@const content = diagnosticsBundle.content}
              <div class="diagnostics-grid">
                <div class="setting-row">
                  <span class="mono-label">CLI PATH</span>
                  <code>{content.cliPath ?? 'not found'}</code>
                </div>
                <div class="setting-row">
                  <span class="mono-label">CLI VERSION</span>
                  <code>{content.cliVersion ?? 'unavailable'}</code>
                </div>
              </div>
              {@render DiagnosticsOutput('CHECK OUTPUT', content.check)}
              {@render DiagnosticsOutput('LIST OUTPUT', content.list)}
              {#if content.profiles.length}
                <p class="mono-label">SAVED PROFILES ({content.profiles.length})</p>
                <div class="table-wrap">
                  <table class="table">
                    <thead>
                      <tr>
                        <th scope="col">Name</th>
                        <th scope="col">Backend</th>
                        <th scope="col">Mount path</th>
                        <th scope="col">Secret</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each content.profiles as profile (profile.id)}
                        <tr>
                          <td>{profile.name}</td>
                          <td><span class="badge">{profile.backend}</span></td>
                          <td><code>{profile.mountPath || '—'}</code></td>
                          <td>{profile.secretRef}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            {/if}
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
        <div class="setting-row">
          <span><strong>Theme</strong>{@render Hint('Follows the system appearance until you pick Light or Dark.')}</span>
          <div class="theme-switch" role="group" aria-label="Theme">
            {#each themeOptions as option (option.value)}
              <button
                class="btn theme-btn"
                class:active={theme === option.value}
                type="button"
                aria-pressed={theme === option.value}
                onclick={() => setTheme(option.value)}
              >
                <option.icon size={15} aria-hidden="true" />
                {option.label}
              </button>
            {/each}
          </div>
        </div>
        <label class="setting-row">
          <span><strong>Default backend</strong>{@render Hint("Applied to new profiles. Auto follows the CLI's platform order.")}</span>
          <select class="select setting-select" value={settings.defaultBackend} onchange={(e) => changeDefaultBackend(e.currentTarget.value as Backend)}>
            {#each backends as backend}<option value={backend}>{backend}</option>{/each}
          </select>
        </label>
        <label class="setting-row stacked">
          <span><strong>Default discovery URL</strong>{@render Hint('Seeds new profiles. Each profile can still override it individually; existing profiles are never rewritten when this changes.')}</span>
          <input
            class="input"
            type="text"
            placeholder="https://hub.example.com"
            value={settings.defaultDiscoveryUrl ?? ''}
            onchange={(e) => changeDefaultDiscoveryUrl(e.currentTarget.value)}
          />
        </label>
        <label class="setting-row">
          <span><strong>Skip unmount confirmation</strong>{@render Hint('Unmount and Unmount all act immediately, with no confirmation dialog.')}</span>
          <input type="checkbox" checked={skipUnmountConfirm} onchange={(e) => setSkipUnmountConfirm(e.currentTarget.checked)} />
        </label>
      </section>

      <section class="surface panel settings-panel">
        <h3>About mountOS</h3>
        <div class="setting-row">
          <span><strong>Platform</strong></span>
          <span class="mono-label">{systemState.platform}</span>
        </div>
        <div class="setting-row">
          <span><strong>CLI version</strong></span>
          <span class="mono-label">{systemState.cliVersion ?? 'unavailable'}</span>
        </div>
        <div class="setting-row">
          <span><strong>CLI path</strong></span>
          <code>{systemState.cliPath ?? 'not found on PATH'}</code>
        </div>

        {#if systemState.cliPathAlternates.length}
          <div class="callout warning">
            <AlertTriangle size={17} aria-hidden="true" />
            <span>
              {systemState.cliPathAlternates.length} other mountos {systemState.cliPathAlternates.length === 1 ? 'binary was' : 'binaries were'} found on PATH and ignored:
              {systemState.cliPathAlternates.join(', ')}. Pin the one you want below to stop relying on PATH order.
            </span>
          </div>
        {/if}

        <label class="setting-row stacked">
          <span><strong>Pin CLI path</strong>{@render Hint('Overrides the PATH lookup with this exact binary. Leave empty to use the first mountos found on PATH.')}</span>
          <input
            class="input"
            type="text"
            placeholder={systemState.cliPath ?? '/usr/local/bin/mountos'}
            value={settings.cliPathOverride ?? ''}
            onchange={(e) => changeCliPathOverride(e.currentTarget.value)}
          />
        </label>
      </section>

      <section class="surface panel settings-panel">
        <div class="panel-head">
          <h3 class="h3-icon"><Bot size={19} aria-hidden="true" /> MCP for AI agents</h3>
          <div class="row-actions">
            <button class="btn" type="button" onclick={checkMcpStatus} disabled={busy}>
              <RefreshCw size={16} aria-hidden="true" />
              Check status
            </button>
            <button class="btn" type="button" onclick={installMcp} disabled={busy}>Install</button>
            <button class="btn destructive" type="button" onclick={uninstallMcp} disabled={busy}>Uninstall</button>
          </div>
        </div>
        <p>Registers this mountos binary as a read-only Model Context Protocol server for Claude Desktop, Claude Code, Codex and Gemini, so an AI agent can inspect mounts, stats and diagnostics without file access.</p>
        {#if mcpStatusText}
          <div class="command-preview">
            <p class="mono-label">MCP STATUS</p>
            <pre><code>{mcpStatusText}</code></pre>
          </div>
        {/if}
      </section>
    {/if}
    </div>
  </main>
</div>

<dialog class="modal" bind:this={secretDialog} onclose={cancelSecret} aria-labelledby="secret-dialog-title">
  {#if secretPromptFor}
    <form onsubmit={(event) => { event.preventDefault(); void doMount(secretPromptFor!, secretValue) }}>
      <div class="modal-head">
        <KeyRound size={20} aria-hidden="true" />
        <h3 id="secret-dialog-title">Enter secret access key</h3>
      </div>
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
      <div class="modal-head">
        <Trash2 size={20} aria-hidden="true" />
        <h3 id="delete-dialog-title">Delete profile</h3>
      </div>
      <p>Deletes "{deletePromptFor.name}" and its vaulted secret. Running mounts are not affected.</p>
      <div class="row-actions">
        <button class="btn" type="button" onclick={cancelDelete}>Cancel</button>
        <button class="btn destructive" type="submit" disabled={busy}>Delete</button>
      </div>
    </form>
  {/if}
</dialog>

<dialog class="modal" bind:this={unmountDialog} onclose={cancelUnmountPrompt} aria-labelledby="unmount-dialog-title">
  {#if unmountPromptFor}
    <form onsubmit={(event) => { event.preventDefault(); void confirmUnmountPrompt() }}>
      <div class="modal-head">
        <Unplug size={20} aria-hidden="true" />
        <h3 id="unmount-dialog-title">{unmountPromptFor === 'all' ? 'Unmount all mounts' : 'Unmount'}</h3>
      </div>
      {#if unmountPromptFor === 'all'}
        <p>Unmount all {systemState.instances.length} running mounts? Each stops flushing in the background once unmounted.</p>
      {:else}
        <p>Unmount "{unmountPromptFor.name || unmountPromptFor.mountPath}"? It stops flushing in the background once unmounted.</p>
      {/if}
      <div class="row-actions">
        <button class="btn" type="button" onclick={cancelUnmountPrompt}>Cancel</button>
        <button class="btn destructive" type="submit" disabled={busy}>Unmount</button>
      </div>
    </form>
  {/if}
</dialog>

{#snippet HealthBadge(health: string)}
  <span class="badge {healthTone(health)}">
    <span class="led {healthTone(health)}" aria-hidden="true"></span>
    {health}
  </span>
{/snippet}

{#snippet Hint(text: string)}
  <small class="hint"><Lightbulb size={13} aria-hidden="true" />{text}</small>
{/snippet}

{#snippet DiagnosticsOutput(title: string, output: DiagnosticsCommandOutput | undefined)}
  {#if output}
    <div class="command-preview">
      <p class="mono-label">{title} <span class="badge {output.status === 0 ? 'success' : 'warning'}">exit {output.status ?? 'unknown'}</span></p>
      <pre><code>{JSON.stringify(output.stdout, null, 2)}</code></pre>
      {#if output.stderr && (typeof output.stderr !== 'string' || output.stderr.trim())}
        <p class="mono-label">STDERR</p>
        <pre><code>{JSON.stringify(output.stderr, null, 2)}</code></pre>
      {/if}
    </div>
  {/if}
{/snippet}

<style>
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    /* Extra top clearance: with titleBarStyle "Overlay" the traffic lights
       float directly over this row instead of sitting in their own strip. */
    padding: 30px 16px 18px;
  }

  .app-shell.sidebar-collapsed .brand {
    justify-content: center;
    padding-inline: 0;
  }

  .brand h1 {
    font-size: 1.25rem;
    white-space: nowrap;
  }

  .mark {
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    object-fit: contain;
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

  .app-shell.sidebar-collapsed .nav-btn {
    justify-content: center;
    padding-inline: 0;
  }

  .nav-btn:hover,
  .nav-btn.active {
    background: var(--accent);
  }

  .profile-row:hover,
  .profile-row.active {
    border-color: var(--border);
    background: var(--accent);
  }

  .sidebar-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    margin-top: auto;
    border: none;
    border-top: 1px solid var(--border);
    background: transparent;
    color: var(--muted-foreground);
    font-size: 1rem;
    cursor: pointer;
    padding: 12px 16px;
    text-align: left;
  }

  .sidebar-footer:hover {
    background: var(--accent);
    color: var(--foreground);
  }

  .app-shell.sidebar-collapsed .sidebar-footer {
    justify-content: center;
    padding-inline: 0;
  }

  .topbar {
    position: sticky;
    top: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    /* Top padding approximates .brand's traffic-light clearance so both
       header rows line up on the same baseline. */
    padding: 25px 22px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--background);
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
    flex: 1 1 160px;
    min-width: 160px;
    max-width: 260px;
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

  .h3-icon {
    display: flex;
    align-items: center;
    gap: 8px;
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

  .profile-empty {
    justify-items: center;
    align-content: center;
    gap: 12px;
    min-height: 320px;
    margin: 0;
    color: var(--muted-foreground);
  }

  .profile-empty p {
    max-width: 46ch;
  }

  .profile-empty .btn {
    margin-top: 8px;
  }

  .profiles-layout {
    display: grid;
    flex: 1;
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
  .setting-row small,
  .field small {
    color: var(--muted-foreground);
    font-size: 1rem;
  }

  .field small.field-error {
    color: var(--warning);
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

  .field > span,
  .field > label {
    color: var(--label-foreground);
    font-size: 1rem;
    font-weight: 500;
  }

  .field-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .input-with-action {
    display: flex;
    gap: 8px;
  }

  .input-with-action .input {
    flex: 1;
    min-width: 0;
  }

  .input-with-action .btn {
    flex-shrink: 0;
  }

  .field-head span,
  .field-head label {
    color: var(--label-foreground);
    font-size: 1rem;
    font-weight: 500;
  }

  .btn.small {
    min-height: 28px;
    padding: 4px 10px;
    font-size: 0.875rem;
    font-family: ui-monospace, Menlo, monospace;
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

  .setting-row.stacked {
    flex-direction: column;
    align-items: stretch;
  }

  .setting-row.stacked .input {
    width: 100%;
  }

  .theme-switch {
    display: flex;
    gap: 6px;
  }

  .theme-btn.active {
    border-color: var(--primary);
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .theme-btn.active:hover {
    background: oklch(from var(--primary) l c h / 0.9);
  }

  .save-secret {
    justify-content: flex-start;
  }

  .setting-row > span {
    display: grid;
    gap: 2px;
  }

  .hint {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    color: var(--muted-foreground);
    font-size: 1rem;
  }

  .hint :global(svg) {
    flex-shrink: 0;
    margin-top: 2px;
    color: var(--warning);
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

  .diagnostics-report {
    display: grid;
    gap: 12px;
    margin-bottom: 14px;
  }

  .diagnostics-grid {
    display: grid;
    gap: 6px;
  }

  .command-preview pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .config-row td {
    background: var(--muted);
    padding-top: 0;
  }

  .config-row .command-preview {
    background: var(--card);
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

  .issue :global(.error) {
    color: var(--destructive);
  }

  .issue :global(.warning) {
    color: var(--warning);
  }

  .issue :global(.info) {
    color: var(--muted-foreground);
  }

  .skeleton.narrow {
    max-width: 80px;
  }

  .skeleton.wide {
    max-width: 240px;
  }

  .modal {
    /* WKWebView doesn't reliably apply the UA's default dialog:modal
       centering (margin: auto within a fixed inset), so it's forced here. */
    position: fixed;
    inset: 0;
    margin: auto;
    width: min(460px, calc(100% - 40px));
    max-height: calc(100% - 40px);
    border: 1px solid var(--border);
    background: var(--card);
    color: var(--card-foreground);
    padding: 18px;
  }

  .modal form {
    display: grid;
    gap: 14px;
  }

  .modal-head {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .modal-head h3 {
    margin: 0;
  }

  .modal .row-actions {
    justify-content: flex-end;
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
