<script lang="ts">
  import { Unplug } from '@lucide/svelte'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Button } from '$lib/components/ui/button'
  import { appState, cancelUnmountPrompt, confirmUnmountPrompt } from '$lib/app-state.svelte'
</script>

<Dialog.Root bind:open={() => appState.unmountPromptFor !== null, (open) => { if (!open) cancelUnmountPrompt() }}>
  <Dialog.Content class="sm:max-w-md" aria-describedby={undefined}>
    {#if appState.unmountPromptFor}
      <form onsubmit={(event) => { event.preventDefault(); void confirmUnmountPrompt() }}>
        <Dialog.Header>
          <Dialog.Title class="flex items-center gap-2">
            <Unplug size={20} aria-hidden="true" />
            {appState.unmountPromptFor === 'all' ? 'Unmount all mounts' : 'Unmount'}
          </Dialog.Title>
        </Dialog.Header>
        {#if appState.unmountPromptFor === 'all'}
          <p class="py-4">Unmount all {appState.systemState.instances.length} running mounts? Each stops flushing in the background once unmounted.</p>
        {:else}
          <p class="py-4">Unmount "{appState.unmountPromptFor.name || appState.unmountPromptFor.mountPath}"? It stops flushing in the background once unmounted.</p>
        {/if}
        <Dialog.Footer>
          <Button type="button" variant="outline" class="cyberpunk-skewed-sm" onclick={cancelUnmountPrompt}>Cancel</Button>
          <Button type="submit" variant="destructive" class="cyberpunk-skewed-sm" disabled={appState.busy}>Unmount</Button>
        </Dialog.Footer>
      </form>
    {/if}
  </Dialog.Content>
</Dialog.Root>
