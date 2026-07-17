<script lang="ts">
  import { cn } from '$lib/utils'
  import type { Snippet } from 'svelte'

  let {
    label,
    class: className,
    children,
  }: {
    label?: string
    class?: string
    children: Snippet
  } = $props()
</script>

<div class={cn('grid gap-2 overflow-auto border border-border bg-muted p-3', className)}>
  {#if label}
    <p class="mono-label">{label}</p>
  {/if}
  {@render children()}
</div>
