<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";
  import { cn, type WithElementRef } from "$lib/utils.js";

  type Props = WithElementRef<Omit<HTMLInputAttributes, "type">, HTMLInputElement> & {
    checked?: boolean;
    label?: string;
  };

  let {
    ref = $bindable(null), checked = $bindable(false),
    label, disabled, class: className, ...restProps
  }: Props = $props();
</script>

<label class={cn(
  "inline-flex min-h-[44px] sm:min-h-8 items-center gap-2.5 px-1 select-none group",
  disabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer",
  className
)}>
  <span class={cn(
    "relative inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-[2px] border border-foreground/35 bg-background/60 shadow-sm transition-[border-color,background-color]",
    !disabled && "group-hover:border-foreground/60",
    "has-[:focus-visible]:outline-none has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring has-[:focus-visible]:ring-offset-2 has-[:focus-visible]:ring-offset-background",
    checked && "border-primary bg-primary/10",
  )}>
    <input bind:this={ref} type="checkbox" data-slot="checkbox"
      class="absolute inset-0 h-full w-full opacity-0"
      class:cursor-pointer={!disabled}
      class:cursor-not-allowed={disabled}
      {disabled} bind:checked {...restProps} />
    {#if checked}
      <svg aria-hidden="true" class="h-4 w-4 text-primary pointer-events-none" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M2.5 6L5 8.5L9.5 3.5" />
      </svg>
    {/if}
  </span>
  {#if label}
    <span class="text-sm">{label}</span>
  {/if}
</label>
