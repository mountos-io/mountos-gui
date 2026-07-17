<script lang="ts">
  import { Popover as PopoverPrimitive } from "bits-ui";

  let {
    ref = $bindable(null), children, child, ...restProps
  }: PopoverPrimitive.TriggerProps = $props();
</script>

<PopoverPrimitive.Trigger bind:ref data-slot="popover-trigger" {...restProps} {child}>
  {#if children}{@render children()}{/if}
</PopoverPrimitive.Trigger>
