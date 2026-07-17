<script lang="ts">
  import { cn, type WithElementRef, type WithoutChildren } from "$lib/utils.js"
  import type { HTMLAttributes } from "svelte/elements"

  let {
    ref = $bindable(null), class: className, ...restProps
  }: WithoutChildren<WithElementRef<HTMLAttributes<HTMLDivElement>>> = $props()
</script>

<div
  bind:this={ref}
  data-slot="cyber-skeleton"
  aria-hidden="true"
  class={cn("bg-muted-foreground/15 animate-pulse rounded-none", className)}
  {...restProps}>
</div>
