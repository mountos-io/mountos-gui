import { getCurrentWindow } from '@tauri-apps/api/window'
import { hasDesktopBridge } from './tauri'
import { applySkin, clearSkin, familyVariant, findPreset } from './themes'

export type Theme = 'system' | 'light' | 'dark'

export const THEME_STORAGE_KEY = 'mountos-desktop-theme'
const STORAGE_KEY = THEME_STORAGE_KEY
export const SKIN_STORAGE_KEY = 'mountos-desktop-skin'

function loadTheme(): Theme {
  if (typeof localStorage === 'undefined') return 'system'
  const stored = localStorage.getItem(STORAGE_KEY)
  return stored === 'light' || stored === 'dark' || stored === 'system' ? stored : 'system'
}

function saveTheme(theme: Theme) {
  if (typeof localStorage !== 'undefined') localStorage.setItem(STORAGE_KEY, theme)
}

function loadSkin(): string {
  if (typeof localStorage === 'undefined') return ''
  return localStorage.getItem(SKIN_STORAGE_KEY) ?? ''
}

function saveSkin(skin: string) {
  if (typeof localStorage !== 'undefined') localStorage.setItem(SKIN_STORAGE_KEY, skin)
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
  applySkinPreset()
  if (hasDesktopBridge()) {
    getCurrentWindow()
      .setTheme(mode)
      .catch(() => {})
  }
}

// Reconciles the picked skin against the CURRENT resolved mode (state.
// resolvedMode is always kept in sync with state.theme before this runs --
// see setTheme/initThemeSync). A skin's `family` pairs a light/dark variant
// (e.g. Dracula <-> Alucard) so flipping mode re-derives the matching member
// instead of showing a light skin's colors under a dark toggle. No family
// (or no match) falls back to no skin, i.e. this app's own plain palette.
function applySkinPreset() {
  if (typeof document === 'undefined') return
  const mode = state.resolvedMode
  if (!state.skin) {
    clearSkin()
    if (mode === 'dark') {
      const mountOSDark = findPreset('mountOS Dark')
      if (mountOSDark) applySkin(mountOSDark.colors, 'dark')
    }
    return
  }
  let preset = findPreset(state.skin)
  if (preset && preset.mode !== mode) {
    const variant = familyVariant(state.skin, mode)
    if (variant) {
      preset = variant
      state.skin = variant.name
      saveSkin(variant.name)
    } else {
      state.skin = ''
      saveSkin('')
      clearSkin()
      return
    }
  }
  if (!preset) {
    state.skin = ''
    saveSkin('')
    clearSkin()
    return
  }
  clearSkin()
  applySkin(preset.colors, preset.mode)
}

// Reactive across every component in THIS webview (module-level $state is a
// singleton). Each of App.svelte/TrayPopover.svelte is its own separate Tauri
// window/webview though, so this does NOT sync between them on its own --
// initThemeSync()'s 'storage' listener still does that part, same as before.
const state = $state({ theme: loadTheme(), resolvedMode: resolveTheme(loadTheme()), skin: loadSkin() })

export const themeState = state

export function setTheme(next: Theme) {
  state.theme = next
  state.resolvedMode = resolveTheme(next)
  saveTheme(next)
  applyTheme(next)
}

export function setSkin(next: string) {
  state.skin = next
  saveSkin(next)
  applySkinPreset()
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
      if (event.key !== STORAGE_KEY && event.key !== SKIN_STORAGE_KEY) return
      state.theme = loadTheme()
      state.skin = loadSkin()
      state.resolvedMode = resolveTheme(state.theme)
      applyTheme(state.theme)
    }
    window.addEventListener('storage', onStorage)
    cleanups.push(() => window.removeEventListener('storage', onStorage))
  }

  return () => cleanups.forEach((cleanup) => cleanup())
}
