<script lang="ts">
  import { fade } from 'svelte/transition'
  import { AlertTriangle, CheckCircle2, Info, X, XCircle } from '@lucide/svelte'
  import { dismissToast, toastState, type ToastItem } from '$lib/toast.svelte'
  import { prefersReducedMotion } from '$lib/utils'

  const icons: Record<ToastItem['type'], typeof Info> = {
    success: CheckCircle2,
    error: XCircle,
    warning: AlertTriangle,
    info: Info,
  }

  // Tailwind's scanner needs each full class name as a literal substring
  // somewhere in this file -- a template-interpolated `border-{type}/45`
  // would only ever emit the class actually reached at runtime, not one
  // Tailwind can see ahead of time, so the rest silently never generate.
  const toneClasses: Record<ToastItem['type'], string> = {
    success: 'border-success/45 text-success',
    error: 'border-destructive/45 text-destructive',
    warning: 'border-warning/45 text-warning',
    info: 'border-primary/45 text-primary',
  }

  const exitDuration = $derived(prefersReducedMotion() ? 0 : 150)

  // Owns the actual auto-dismiss timer per toast so it can pause while the
  // toast is hovered or keyboard-focused (WCAG 2.2.1 Timing Adjustable) --
  // sonner paused on hover by default and this hand-rolled replacement must
  // not quietly drop that. `duration: Infinity` (error toasts) never starts
  // a timer, matching the "stays until dismissed" contract in toast.svelte.ts.
  function autoDismiss(node: HTMLElement, { id, duration }: { id: number; duration: number }) {
    let remaining = duration
    let startedAt = 0
    let timer: ReturnType<typeof setTimeout> | undefined

    function start() {
      if (!Number.isFinite(remaining)) return
      startedAt = Date.now()
      timer = setTimeout(() => dismissToast(id), remaining)
    }
    function pause() {
      if (timer === undefined) return
      clearTimeout(timer)
      timer = undefined
      remaining -= Date.now() - startedAt
    }

    start()
    node.addEventListener('pointerenter', pause)
    node.addEventListener('pointerleave', start)
    node.addEventListener('focusin', pause)
    node.addEventListener('focusout', start)

    return {
      destroy() {
        if (timer !== undefined) clearTimeout(timer)
        node.removeEventListener('pointerenter', pause)
        node.removeEventListener('pointerleave', start)
        node.removeEventListener('focusin', pause)
        node.removeEventListener('focusout', start)
      },
    }
  }
</script>

<!-- z-[100]: above bits-ui's Dialog (z-50) -- a toast fired from inside a
     dialog (e.g. "Profile saved" while a satellite dialog is still open)
     must stay visible, not sit behind the modal backdrop. -->
<div class="pointer-events-none fixed right-4 top-4 z-[100] flex w-80 max-w-[calc(100vw-2rem)] flex-col gap-2" aria-live="polite" role="region" aria-label="Notifications">
  {#each [...toastState.items].reverse() as item (item.id)}
    {@const Icon = icons[item.type]}
    <div
      class="toast-item pointer-events-auto surface flex items-start gap-2.5 border p-3 text-sm {toneClasses[item.type]}"
      role={item.type === 'error' ? 'alert' : 'status'}
      out:fade={{ duration: exitDuration }}
      use:autoDismiss={{ id: item.id, duration: item.duration }}
    >
      <Icon size={17} aria-hidden="true" class="mt-0.5 shrink-0" />
      <p class="min-w-0 flex-1 text-foreground">{item.message}</p>
      <button
        type="button"
        class="shrink-0 text-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
        title="Dismiss"
        aria-label="Dismiss notification"
        onclick={() => dismissToast(item.id)}
      >
        <X size={15} aria-hidden="true" />
      </button>
    </div>
  {/each}
</div>

<style>
  .toast-item {
    animation: toast-in 0.2s ease-out;
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
