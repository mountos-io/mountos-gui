<script lang="ts">
  import { Toaster as Sonner, type ToasterProps as SonnerProps } from "svelte-sonner";
  import { themeState } from "$lib/theme.svelte";

  let { ...restProps }: SonnerProps = $props();
</script>

<Sonner
  theme={themeState.resolvedMode}
  class="toaster group"
  closeButton={true}
  position="top-right"
  expand={true}
  visibleToasts={5}
  gap={16}
  style="--normal-bg: var(--card); --normal-text: var(--card-foreground); --normal-border: var(--border);"
  {...restProps}
/>
