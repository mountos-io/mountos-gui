<script lang="ts">
  import type { HTMLTextareaAttributes } from "svelte/elements";
  import { cn, type WithElementRef } from "$lib/utils.js";

  type Props = WithElementRef<HTMLTextareaAttributes, HTMLTextAreaElement>;

  let {
    ref = $bindable(null), value = $bindable(), class: className, ...restProps
  }: Props = $props();
</script>

<textarea bind:this={ref} data-slot="textarea"
  class={cn(
    "border-input bg-background selection:bg-primary/20 dark:bg-input/20 selection:text-foreground ring-offset-background placeholder:text-muted-foreground shadow-none flex min-h-[80px] w-full min-w-0 resize-y rounded-sm border px-3 py-2 text-base outline-none transition-[border-color] disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
    "focus-visible:border-foreground/40 focus-visible:ring-2 focus-visible:ring-ring",
    "aria-invalid:ring-0 aria-invalid:border-destructive",
    className
  )}
  bind:value {...restProps}></textarea>
