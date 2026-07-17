<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { FolderOpen, Maximize2, Unplug } from '@lucide/svelte'
  import { isAbsolutePath } from './lib/cli'
  import { healthTone } from './lib/health'
  import { getSystemState, openTarget, showMainWindow, unmountTarget } from './lib/tauri'
  import { applyTheme, loadTheme, THEME_STORAGE_KEY } from './lib/theme'
  import type { MountInstance, SystemState } from './lib/types'

  // alwaysOnTop keeps this popover visually above the main window even after
  // it loses focus, so opening the main window must hide this one explicitly
  // rather than relying on the blur-to-hide path in src-tauri/src/lib.rs.
  async function openMainWindow() {
    await showMainWindow()
    await getCurrentWindow().hide()
  }

  let systemState = $state<SystemState>({ platform: 'macos', checkOk: false, issues: [], instances: [], cliPathAlternates: [], terminals: [] })
  let loaded = $state(false)
  let busy = $state(false)

  let theme = $state(loadTheme())
  $effect(() => {
    applyTheme(theme)
  })

  // Themed in the main window; this popover's own webview persists across
  // show/hide cycles, so it must react to both a system-preference change and
  // a theme change made in the main window's Settings (a cross-window
  // localStorage write, which fires 'storage' here but not in the writer).
  $effect(() => {
    if (typeof matchMedia === 'undefined') return
    const query = matchMedia('(prefers-color-scheme: dark)')
    const onChange = () => {
      if (theme === 'system') applyTheme(theme)
    }
    query.addEventListener('change', onChange)
    return () => query.removeEventListener('change', onChange)
  })

  $effect(() => {
    const onStorage = (event: StorageEvent) => {
      if (event.key === THEME_STORAGE_KEY) theme = loadTheme()
    }
    window.addEventListener('storage', onStorage)
    return () => window.removeEventListener('storage', onStorage)
  })

  function canOpen(instance: MountInstance) {
    return isAbsolutePath(instance.mountPath)
  }

  async function refresh() {
    try {
      systemState = await getSystemState()
    } catch {
      // Quick-glance surface; detailed errors are reported in the main window.
    } finally {
      loaded = true
    }
  }

  async function runOpen(instance: MountInstance) {
    try {
      await openTarget(instance.mountPath)
    } catch {
      // Quick-glance surface; detailed errors are reported in the main window.
    }
  }

  async function runUnmount(instance: MountInstance) {
    busy = true
    try {
      await unmountTarget(instance.domainId || instance.mountPath)
      await refresh()
    } catch {
      // Quick-glance surface; detailed errors are reported in the main window.
    } finally {
      busy = false
    }
  }

  $effect(() => {
    void refresh()
    const onFocus = () => void refresh()
    window.addEventListener('focus', onFocus)
    return () => window.removeEventListener('focus', onFocus)
  })
</script>

<svelte:head>
  <title>mountOS</title>
</svelte:head>

<div class="popover">
  <header>
    <div class="brand">
      <img src="/logo.png" alt="" width="18" height="18" />
      <span>mountOS</span>
    </div>
    <button class="btn icon-btn ghost" type="button" title="Open mountOS" aria-label="Open mountOS" onclick={openMainWindow}>
      <Maximize2 size={14} aria-hidden="true" />
    </button>
  </header>

  <div class="list">
    {#if !loaded}
      <p class="empty">Loading…</p>
    {:else if systemState.instances.length === 0}
      <div class="empty">
        <p>No active mounts.</p>
        <button class="btn" type="button" onclick={openMainWindow}>
          <Maximize2 size={14} aria-hidden="true" />
          Open mountOS
        </button>
      </div>
    {:else}
      {#each systemState.instances as instance (instance.key)}
        <div class="row">
          <span class="led {healthTone(instance.health)}" title={instance.health} aria-hidden="true"></span>
          <div class="row-main">
            <strong>{instance.name || instance.volumeId || 'mountOS volume'} <span class="sr-only">({instance.health})</span></strong>
            <span class="mono-label">{instance.mountPath}</span>
          </div>
          <div class="row-actions">
            <button class="btn icon-btn" type="button" title="Open folder" aria-label="Open folder" disabled={busy || !canOpen(instance)} onclick={() => runOpen(instance)}>
              <FolderOpen size={13} aria-hidden="true" />
            </button>
            <button class="btn icon-btn destructive" type="button" title="Unmount" aria-label="Unmount" disabled={busy} onclick={() => runUnmount(instance)}>
              <Unplug size={13} aria-hidden="true" />
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .popover {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--card);
    color: var(--card-foreground);
    border: 1px solid var(--border);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    flex-shrink: 0;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }

  .list {
    flex: 1 1 auto;
    overflow-y: auto;
    padding: 6px;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 24px 10px;
    text-align: center;
  }

  .empty p {
    color: var(--muted-foreground);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px;
    border-bottom: 1px solid var(--border);
  }

  .row:last-child {
    border-bottom: none;
  }

  .row-main {
    display: flex;
    min-width: 0;
    flex: 1 1 auto;
    flex-direction: column;
    gap: 2px;
  }

  .row-main strong {
    overflow: hidden;
    font-size: 1rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-main .mono-label {
    overflow: hidden;
    text-overflow: ellipsis;
    text-transform: none;
    letter-spacing: 0;
    white-space: nowrap;
  }

  .row-actions {
    display: flex;
    flex-shrink: 0;
    gap: 4px;
  }

  .row-actions .btn.icon-btn {
    width: 28px;
    height: 28px;
    min-width: 28px;
    min-height: 28px;
  }
</style>
