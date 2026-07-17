<script lang="ts">
  import type { HTMLTableAttributes } from "svelte/elements";
  import { cn, type WithElementRef } from "$lib/utils.js";

  let {
    ref = $bindable(null), class: className, containerLabel = "Data table", children, ...restProps
  }: WithElementRef<HTMLTableAttributes> & { containerLabel?: string } = $props();
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div data-slot="table-container" role="region" aria-label={containerLabel} tabindex={0} class="relative w-full overflow-x-auto">
  <table bind:this={ref} data-slot="table" class={cn("w-full caption-bottom text-sm tabular-nums", className)} {...restProps}>
    {@render children?.()}
  </table>
</div>
