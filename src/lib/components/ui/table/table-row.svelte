<script lang="ts">
  import { cn, type WithElementRef } from "$lib/utils.js";
  import type { HTMLAttributes } from "svelte/elements";

  let {
    ref = $bindable(null), class: className, children, ...restProps
  }: WithElementRef<HTMLAttributes<HTMLTableRowElement>> = $props();
</script>

<tr bind:this={ref} data-slot="table-row"
  class={cn("hover:bg-accent/5 data-[state=selected]:bg-accent/10 border-b border-border/50 transition-colors", className)}
  {...restProps}>
  {@render children?.()}
</tr>
