<script lang="ts">
  import Lightbulb from '@lucide/svelte/icons/lightbulb'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Button } from '$lib/components/ui/button'
  import { Separator } from '$lib/components/ui/separator'
  import CommandPreview from '$lib/components/CommandPreview.svelte'
  import { appState, hideTips } from '$lib/app-state.svelte'
  import { TIPS } from '$lib/tips'
</script>

<Dialog.Root bind:open={() => appState.tipsOpen, (open) => { if (!open) hideTips() }}>
  <Dialog.Content class="sm:max-w-2xl" aria-describedby={undefined}>
    <Dialog.Header>
      <Dialog.Title class="flex items-center gap-2"><Lightbulb size={20} aria-hidden="true" class="text-warning" /> Tips</Dialog.Title>
    </Dialog.Header>
    <div class="grid gap-4 py-4 max-h-[60vh] overflow-auto">
      {#each TIPS as tip, index (tip.title)}
        {#if index > 0}<Separator />{/if}
        <div class="grid gap-1.5">
          <strong>{tip.title}</strong>
          <p class="text-sm text-muted-foreground">{tip.body}</p>
          {#if tip.command}
            <CommandPreview><code>{tip.command}</code></CommandPreview>
          {/if}
        </div>
      {/each}
    </div>
    <Dialog.Footer>
      <Button type="button" variant="outline" onclick={hideTips}>Close</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
