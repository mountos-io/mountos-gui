<script lang="ts">
  import { Command as CommandPrimitive } from "bits-ui";
  import SearchIcon from "@lucide/svelte/icons/search";
  import { cn } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    class: className,
    value = $bindable(""),
    ...restProps
  }: CommandPrimitive.InputProps = $props();
</script>

<div class="flex h-9 items-center gap-2 px-3" data-slot="command-input-wrapper">
  <SearchIcon class="size-4 shrink-0 opacity-50" aria-hidden="true" />
  <CommandPrimitive.Input
    data-slot="command-input"
    class={cn(
      "placeholder:text-muted-foreground flex h-10 w-full rounded-sm border-none bg-transparent py-3 text-sm outline-none focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50",
      className
    )}
    bind:ref
    {...restProps}
    bind:value
  />
</div>
