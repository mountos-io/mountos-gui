<script lang="ts">
  import { CalendarDateTime, parseDateTime, type DateValue } from '@internationalized/date'
  import * as Popover from '$lib/components/ui/popover'
  import { Calendar } from '$lib/components/ui/calendar'
  import CalendarIcon from '@lucide/svelte/icons/calendar'
  import ClockIcon from '@lucide/svelte/icons/clock'
  import ChevronUp from '@lucide/svelte/icons/chevron-up'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import X from '@lucide/svelte/icons/x'
  import { cn } from '$lib/utils'

  let {
    value = $bindable<string>(''),
    min = '',
    max = '',
    placeholder = 'Pick a date and time',
    clearable = false,
    id,
    class: className,
  }: {
    value?: string
    min?: string
    max?: string
    placeholder?: string
    clearable?: boolean
    id?: string
    class?: string
  } = $props()

  function pad(n: number): string { return String(n).padStart(2, '0') }

  function fromLocal(s: string): CalendarDateTime | undefined {
    if (!s) return undefined
    try { return parseDateTime(s.length === 16 ? `${s}:00` : s) } catch { return undefined }
  }

  function toLocal(d: CalendarDateTime | undefined): string {
    if (!d) return ''
    return `${d.year}-${pad(d.month)}-${pad(d.day)}T${pad(d.hour)}:${pad(d.minute)}`
  }

  const initial = fromLocal(value)
  let datePart = $state<DateValue | undefined>(initial)
  let hour = $state<string>(initial ? pad(initial.hour) : '00')
  let minute = $state<string>(initial ? pad(initial.minute) : '00')
  let open = $state(false)

  const minDV = $derived(fromLocal(min))
  const maxDV = $derived(fromLocal(max))

  const hourInvalid = $derived.by(() => {
    if (hour === '') return false
    const n = Number(hour)
    return !Number.isFinite(n) || n < 0 || n > 23
  })
  const minuteInvalid = $derived.by(() => {
    if (minute === '') return false
    const n = Number(minute)
    return !Number.isFinite(n) || n < 0 || n > 59
  })

  function clamp(dt: CalendarDateTime): CalendarDateTime {
    if (minDV && dt.compare(minDV) < 0) return minDV as CalendarDateTime
    if (maxDV && dt.compare(maxDV) > 0) return maxDV as CalendarDateTime
    return dt
  }

  function commit(next: CalendarDateTime) {
    const clamped = clamp(next)
    datePart = clamped
    hour = pad(clamped.hour)
    minute = pad(clamped.minute)
    value = toLocal(clamped)
  }

  function baseDT(): CalendarDateTime {
    return (datePart instanceof CalendarDateTime ? datePart : fromLocal(value)) ?? nowDT()
  }

  function onDateChange(v: DateValue | undefined) {
    if (!v) return
    const h = Math.min(23, Math.max(0, Math.floor(Number(hour) || 0)))
    const m = Math.min(59, Math.max(0, Math.floor(Number(minute) || 0)))
    commit(new CalendarDateTime(v.year, v.month, v.day, h, m, 0))
  }

  function onTimeBlur() {
    const base = baseDT()
    const h = Math.min(23, Math.max(0, Math.floor(Number(hour) || 0)))
    const m = Math.min(59, Math.max(0, Math.floor(Number(minute) || 0)))
    commit(new CalendarDateTime(base.year, base.month, base.day, h, m, 0))
  }

  function nowDT(): CalendarDateTime {
    const d = new Date()
    return new CalendarDateTime(d.getFullYear(), d.getMonth() + 1, d.getDate(), d.getHours(), d.getMinutes(), 0)
  }

  function fromMs(ms: number): CalendarDateTime {
    const d = new Date(ms)
    return new CalendarDateTime(d.getFullYear(), d.getMonth() + 1, d.getDate(), d.getHours(), d.getMinutes(), 0)
  }

  function canJump(deltaMs: number): boolean {
    if (!minDV) return true
    return fromMs(Date.now() + deltaMs).compare(minDV) >= 0
  }

  function setRelative(deltaMs: number) {
    commit(fromMs(Date.now() + deltaMs))
  }

  function setNow() { commit(nowDT()) }

  function clear(e: MouseEvent) {
    e.stopPropagation()
    datePart = undefined
    hour = '00'
    minute = '00'
    value = ''
    open = false
  }

  function bumpHour(delta: number) {
    const base = baseDT()
    const h = ((Number(hour) || 0) + delta + 24) % 24
    commit(new CalendarDateTime(base.year, base.month, base.day, h, Number(minute) || 0, 0))
  }

  function bumpMinute(delta: number) {
    const base = baseDT()
    const m = ((Number(minute) || 0) + delta + 60) % 60
    commit(new CalendarDateTime(base.year, base.month, base.day, Number(hour) || 0, m, 0))
  }

  $effect(() => {
    const parsed = fromLocal(value)
    if (parsed && (!(datePart instanceof CalendarDateTime) || datePart.compare(parsed) !== 0)) {
      datePart = parsed
      hour = pad(parsed.hour)
      minute = pad(parsed.minute)
    }
  })

  const calendarPlaceholder = $derived<DateValue | undefined>(
    (datePart as CalendarDateTime | undefined) ?? (minDV as CalendarDateTime | undefined)
  )

  const display = $derived.by(() => {
    if (!value) return placeholder
    const dt = fromLocal(value)
    if (!dt) return placeholder
    return `${dt.year}-${pad(dt.month)}-${pad(dt.day)} ${pad(dt.hour)}:${pad(dt.minute)}`
  })

  const canJumpH = $derived(canJump(-3600_000))
  const canJumpD = $derived(canJump(-86_400_000))
  const canJumpW = $derived(canJump(-7 * 86_400_000))
</script>

<div class={cn('relative', className)}>
  <Popover.Root bind:open>
    <Popover.Trigger>
      {#snippet child({ props })}
        <button
          {...props}
          type="button"
          {id}
          aria-haspopup="dialog"
          aria-expanded={open}
          aria-label={value ? undefined : placeholder}
          class={cn(
            'flex h-9 pointer-coarse:h-11 w-full items-center justify-between gap-2 rounded-sm border border-input bg-transparent px-3 py-1 text-sm transition-colors',
            'hover:border-ring focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
            !value && 'text-muted-foreground',
            clearable && value && 'pr-8'
          )}
        >
          <span class="inline-flex items-center gap-2 truncate tabular-nums font-mono">
            <CalendarIcon class="h-4 w-4 shrink-0 text-muted-foreground" aria-hidden="true" />
            {display}
          </span>
        </button>
      {/snippet}
    </Popover.Trigger>
    <Popover.Content class="w-[min(320px,calc(100vw-1.5rem))] p-0" align="start">
      <Calendar
        value={datePart}
        placeholder={calendarPlaceholder}
        onValueChange={onDateChange}
        minValue={minDV}
        maxValue={maxDV}
      />
      <div class="flex items-center gap-0.5 border-t border-border px-3 py-2 text-xs">
        <span class="text-muted-foreground/70 mr-2 uppercase tracking-wider">Jump</span>
        <button
          type="button"
          disabled={!canJumpH}
          class="flex-1 px-2 py-1.5 pointer-coarse:py-3 rounded-sm text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-40 disabled:pointer-events-none focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring transition-colors font-mono tabular-nums"
          onclick={() => setRelative(-3600_000)}
        >−1h</button>
        <button
          type="button"
          disabled={!canJumpD}
          class="flex-1 px-2 py-1.5 pointer-coarse:py-3 rounded-sm text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-40 disabled:pointer-events-none focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring transition-colors font-mono tabular-nums"
          onclick={() => setRelative(-86_400_000)}
        >−1d</button>
        <button
          type="button"
          disabled={!canJumpW}
          class="flex-1 px-2 py-1.5 pointer-coarse:py-3 rounded-sm text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-40 disabled:pointer-events-none focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring transition-colors font-mono tabular-nums"
          onclick={() => setRelative(-7 * 86_400_000)}
        >−1w</button>
        <button
          type="button"
          class="flex-1 ml-1 px-2 py-1.5 pointer-coarse:py-3 rounded-sm text-primary-foreground bg-primary/90 hover:bg-primary focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring transition-colors font-mono uppercase tracking-wider"
          onclick={setNow}
        >Now</button>
      </div>
      <div class="flex items-center gap-2 border-t border-border px-3 py-2.5">
        <ClockIcon class="h-4 w-4 text-muted-foreground shrink-0" aria-hidden="true" />
        <span class="text-[0.65rem] uppercase tracking-wider text-muted-foreground/70">Time</span>
        <div class="ml-auto inline-flex items-center gap-1.5">
          <div class={cn(
            'inline-flex items-stretch border rounded-sm overflow-hidden transition-colors',
            hourInvalid ? 'border-destructive' : 'border-border focus-within:border-primary'
          )}>
            <input
              type="text" inputmode="numeric" maxlength="2"
              name="hour" aria-label="Hour" aria-invalid={hourInvalid}
              class="w-11 bg-transparent text-center tabular-nums font-mono text-sm py-1 focus:outline-none focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset aria-[invalid=true]:text-destructive"
              bind:value={hour}
              onblur={onTimeBlur}
            />
            <div class="flex flex-col border-l border-border">
              <button type="button" aria-label="Increment hour"
                class="flex h-4 w-6 pointer-coarse:h-8 pointer-coarse:w-11 items-center justify-center text-muted-foreground hover:bg-accent hover:text-foreground active:bg-primary/10 active:text-primary focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary focus-visible:z-10 transition-colors"
                onclick={() => bumpHour(1)}>
                <ChevronUp class="h-3 w-3" />
              </button>
              <button type="button" aria-label="Decrement hour"
                class="flex h-4 w-6 pointer-coarse:h-8 pointer-coarse:w-11 items-center justify-center border-t border-border text-muted-foreground hover:bg-accent hover:text-foreground active:bg-primary/10 active:text-primary focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary focus-visible:z-10 transition-colors"
                onclick={() => bumpHour(-1)}>
                <ChevronDown class="h-3 w-3" />
              </button>
            </div>
          </div>
          <span class="text-muted-foreground font-mono text-sm">:</span>
          <div class={cn(
            'inline-flex items-stretch border rounded-sm overflow-hidden transition-colors',
            minuteInvalid ? 'border-destructive' : 'border-border focus-within:border-primary'
          )}>
            <input
              type="text" inputmode="numeric" maxlength="2"
              name="minute" aria-label="Minute" aria-invalid={minuteInvalid}
              class="w-11 bg-transparent text-center tabular-nums font-mono text-sm py-1 focus:outline-none focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset aria-[invalid=true]:text-destructive"
              bind:value={minute}
              onblur={onTimeBlur}
            />
            <div class="flex flex-col border-l border-border">
              <button type="button" aria-label="Increment minute"
                class="flex h-4 w-6 pointer-coarse:h-8 pointer-coarse:w-11 items-center justify-center text-muted-foreground hover:bg-accent hover:text-foreground active:bg-primary/10 active:text-primary focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary focus-visible:z-10 transition-colors"
                onclick={() => bumpMinute(1)}>
                <ChevronUp class="h-3 w-3" />
              </button>
              <button type="button" aria-label="Decrement minute"
                class="flex h-4 w-6 pointer-coarse:h-8 pointer-coarse:w-11 items-center justify-center border-t border-border text-muted-foreground hover:bg-accent hover:text-foreground active:bg-primary/10 active:text-primary focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary focus-visible:z-10 transition-colors"
                onclick={() => bumpMinute(-1)}>
                <ChevronDown class="h-3 w-3" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </Popover.Content>
  </Popover.Root>
  {#if clearable && value}
    <button
      type="button"
      aria-label="Clear"
      class="absolute right-2 top-1/2 -translate-y-1/2 flex h-5 w-5 min-h-[44px] min-w-[44px] sm:min-h-5 sm:min-w-5 items-center justify-center rounded-sm text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring transition-colors"
      onclick={clear}
    >
      <X class="h-3.5 w-3.5" />
    </button>
  {/if}
</div>
