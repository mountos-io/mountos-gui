<script lang="ts">
  import type { Command as CommandPrimitive, Dialog as DialogPrimitive } from "bits-ui";
  import type { Snippet } from "svelte";
  import Command from "./command.svelte";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import type { WithoutChildrenOrChild } from "$lib/utils.js";
  import { cn } from "$lib/utils.js";

  let {
    open = $bindable(false),
    ref = $bindable(null),
    value = $bindable(""),
    title = "Command Palette",
    description = "Search for a command to run",
    children,
    ...restProps
  }: WithoutChildrenOrChild<DialogPrimitive.RootProps> &
    WithoutChildrenOrChild<CommandPrimitive.RootProps> & {
      children: Snippet;
      title?: string;
      description?: string;
    } = $props();

  function handleKeydownInternal(event: KeyboardEvent) {
    if (event.key === "Escape" && open) {
      event.stopPropagation();
      open = false;
    }
  }
</script>

<Dialog.Root bind:open {...restProps}>
  <Dialog.Content
    class={cn(
      "overflow-hidden p-0 fixed left-[50%] top-[20%] z-50 grid w-[calc(100vw-2rem)] max-w-[640px] translate-x-[-50%] translate-y-0 gap-0 rounded-sm border shadow-none bg-background data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 duration-200"
    )}
    showCloseButton={false}
    onkeydown={handleKeydownInternal}
    tabindex={-1}
  >
    <Dialog.Title class="sr-only">{title}</Dialog.Title>
    <Dialog.Description class="sr-only">{description}</Dialog.Description>
    <Command
      class="**:data-[slot=command-input-wrapper]:h-12 [&_[data-command-group]:not([hidden])_~[data-command-group]]:pt-0 [&_[data-command-group]]:px-2 [&_[data-command-input-wrapper]_svg]:h-5 [&_[data-command-input-wrapper]_svg]:w-5 [&_[data-command-input]]:h-12 [&_[data-command-item]]:px-2 [&_[data-command-item]]:py-3 [&_[data-command-item]_svg]:h-5 [&_[data-command-item]_svg]:w-5"
      {...restProps}
      bind:value
      bind:ref
      {children}
    />
  </Dialog.Content>
</Dialog.Root>
