import { getCurrentWindow } from '@tauri-apps/api/window'
import { hasDesktopBridge } from './tauri'

export type Theme = 'system' | 'light' | 'dark'

export const THEME_STORAGE_KEY = 'mountos-desktop-theme'
const STORAGE_KEY = THEME_STORAGE_KEY

export function loadTheme(): Theme {
  if (typeof localStorage === 'undefined') return 'system'
  const stored = localStorage.getItem(STORAGE_KEY)
  return stored === 'light' || stored === 'dark' || stored === 'system' ? stored : 'system'
}

export function saveTheme(theme: Theme) {
  if (typeof localStorage !== 'undefined') localStorage.setItem(STORAGE_KEY, theme)
}

export function resolveTheme(theme: Theme): 'light' | 'dark' {
  if (theme === 'system') {
    return typeof matchMedia !== 'undefined' && matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }
  return theme
}

// Sets color-scheme inline (not just the .dark class) so native form controls
// match the chosen theme even when a browser extension injects its own.
// Also pushes the resolved mode to the native window (setTheme) — without
// this, macOS keeps the titlebar/traffic-lights on the OS-level appearance,
// which can go native-dark against light webview content (or vice versa).
export function applyTheme(theme: Theme) {
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
