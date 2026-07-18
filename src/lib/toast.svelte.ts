export type ToastType = 'success' | 'error' | 'warning' | 'info'

export interface ToastItem {
  id: number
  type: ToastType
  message: string
  duration: number
}

// Errors stay until dismissed: an operator-requested CLI action that failed
// needs a decision, not something that quietly vanishes while they're
// looking elsewhere. success/warning/info auto-dismiss.
const DURATIONS: Record<ToastType, number> = {
  success: 4000,
  error: Infinity,
  warning: 6000,
  info: 4000,
}

let items = $state<ToastItem[]>([])
let nextId = 0

export const toastState = {
  get items() {
    return items
  },
}

// Duration is carried on the item, not scheduled here: Toaster.svelte owns
// the actual timer so it can pause it while a toast is hovered or focused
// (WCAG 2.2.1, and parity with the sonner behavior this replaced).
function show(type: ToastType, message: string, duration = DURATIONS[type]): number {
  const id = ++nextId
  items = [...items, { id, type, message, duration }]
  return id
}

export function showSuccessToast(message: string, duration?: number) {
  return show('success', message, duration)
}
export function showErrorToast(message: string, duration?: number) {
  return show('error', message, duration)
}
export function showWarningToast(message: string, duration?: number) {
  return show('warning', message, duration)
}
export function showInfoToast(message: string, duration?: number) {
  return show('info', message, duration)
}
export function showToast(type: ToastType, message: string, duration?: number) {
  return show(type, message, duration)
}

export function dismissToast(id: number) {
  items = items.filter((item) => item.id !== id)
}
export function dismissAllToasts() {
  items = []
}
