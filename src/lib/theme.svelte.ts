import { getCurrentWindow } from '@tauri-apps/api/window'
import { hasDesktopBridge } from './tauri'

export type Theme = 'system' | 'light' | 'dark'

export const THEME_STORAGE_KEY = 'mountos-desktop-theme'
const STORAGE_KEY = THEME_STORAGE_KEY

function loadTheme(): Theme {
  if (typeof localStorage === 'undefined') return 'system'
  const stored = localStorage.getItem(STORAGE_KEY)
  return stored === 'light' || stored === 'dark' || stored === 'system' ? stored : 'system'
}

function saveTheme(theme: Theme) {
  if (typeof localStorage !== 'undefined') localStorage.setItem(STORAGE_KEY, theme)
}

function resolveTheme(theme: Theme): 'light' | 'dark' {
  if (theme === 'system') {
    return typeof matchMedia !== 'undefined' && matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }
  return theme
}

// Sets color-scheme inline (not just the .dark class) so native form controls
// match the chosen theme even when a browser extension injects its own.
// Also pushes the resolved mode to the native window (setTheme) -- without
// this, macOS keeps the titlebar/traffic-lights on the OS-level appearance,
// which can go native-dark against light webview content (or vice versa).
function applyTheme(theme: Theme) {
  if (typeof document === 'undefined') return
  const mode = resolveTheme(theme)
  document.documentElement.classList.toggle('dark', mode === 'dark')
  document.documentElement.style.colorScheme = mode
  if (hasDesktopBridge()) {
    getCurrentWindow()
      .setTheme(mode)
      .catch(() => {})
  }
}

// Reactive across every component in THIS webview (module-level $state is a
// singleton). Each of App.svelte/TrayPopover.svelte is its own separate Tauri
// window/webview though, so this does NOT sync between them on its own --
// initThemeSync()'s 'storage' listener still does that part, same as before.
const state = $state({ theme: loadTheme(), resolvedMode: resolveTheme(loadTheme()) })

export const themeState = state

export function setTheme(next: Theme) {
  state.theme = next
  state.resolvedMode = resolveTheme(next)
  saveTheme(next)
  applyTheme(next)
}

// Applies the current theme immediately and wires up the two things that can
// change it without a local setTheme() call: the OS appearance (when
// following "system") and another window writing THEME_STORAGE_KEY. Call
// once per window (App.svelte's root shell, TrayPopover.svelte).
export function initThemeSync(): () => void {
  applyTheme(state.theme)

  const cleanups: Array<() => void> = []

  if (typeof matchMedia !== 'undefined') {
    const query = matchMedia('(prefers-color-scheme: dark)')
    const onChange = () => {
      if (state.theme === 'system') {
        state.resolvedMode = resolveTheme(state.theme)
        applyTheme(state.theme)
      }
    }
    query.addEventListener('change', onChange)
    cleanups.push(() => query.removeEventListener('change', onChange))
  }

  if (typeof window !== 'undefined') {
    const onStorage = (event: StorageEvent) => {
      if (event.key !== STORAGE_KEY) return
      state.theme = loadTheme()
      state.resolvedMode = resolveTheme(state.theme)
      applyTheme(state.theme)
    }
    window.addEventListener('storage', onStorage)
    cleanups.push(() => window.removeEventListener('storage', onStorage))
  }

  return () => cleanups.forEach((cleanup) => cleanup())
}
