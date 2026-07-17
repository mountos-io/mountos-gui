<script lang="ts">
  import { AlertTriangle } from '@lucide/svelte'
  import { cn } from '$lib/utils'
  import type { Snippet } from 'svelte'

  let {
    variant = 'warning',
    role,
    class: className,
    children,
  }: {
    variant?: 'warning'
    role?: string
    class?: string
    children: Snippet
  } = $props()
</script>

<div class={cn('flex items-center gap-2.5 border border-warning/45 p-2.5 text-warning text-sm', className)} {role}>
  <AlertTriangle size={17} aria-hidden="true" class="shrink-0" />
  <span>{@render children()}</span>
</div>
