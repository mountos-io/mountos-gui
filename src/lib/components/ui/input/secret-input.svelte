<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";
  import { cn, type WithElementRef } from "$lib/utils.js";
  import { Eye, EyeOff } from "@lucide/svelte";

  type Props = WithElementRef<Omit<HTMLInputAttributes, "type">>;

  let {
    ref = $bindable(null), value = $bindable(),
    class: className, ...restProps
  }: Props = $props();

  let visible = $state(false);
</script>

<div class="relative">
  <input bind:this={ref} data-slot="input"
    class={cn(
      "border-input bg-background selection:bg-primary/20 dark:bg-input/20 selection:text-foreground ring-offset-background placeholder:text-muted-foreground shadow-none flex h-9 w-full min-w-0 rounded-sm border px-3 py-1 pr-9 text-base outline-none transition-[border-color] disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
      "focus-visible:border-foreground/40 focus-visible:ring-2 focus-visible:ring-ring",
      "aria-invalid:ring-0 aria-invalid:border-destructive",
      className
    )}
    type={visible ? "text" : "password"} bind:value {...restProps} />
  <button type="button"
    class="text-muted-foreground hover:text-foreground absolute right-2 top-1/2 -translate-y-1/2 inline-flex items-center justify-center min-h-[44px] min-w-[44px] sm:min-h-0 sm:min-w-0 cursor-pointer transition-colors"
    aria-label={visible ? 'Hide password' : 'Show password'}
    onclick={() => visible = !visible}>
    {#if visible}
      <EyeOff class="size-4" />
    {:else}
      <Eye class="size-4" />
    {/if}
  </button>
</div>
