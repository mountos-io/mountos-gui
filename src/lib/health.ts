import type { HealthState } from './types'

export function healthTone(health: HealthState | string): 'success' | 'destructive' | 'warning' | '' {
  if (health === 'healthy') return 'success'
  if (health === 'lost') return 'destructive'
  if (health === 'limited' || health === 'launching') return 'warning'
  return ''
}
