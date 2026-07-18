<script lang="ts">
  import { FolderOpen, Recycle } from '@lucide/svelte'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Select } from '$lib/components/ui/select'
  import DateTimePicker from '$lib/components/shared/DateTimePicker.svelte'
  import Callout from '$lib/components/Callout.svelte'
  import CommandPreview from '$lib/components/CommandPreview.svelte'
  import {
    appState,
    browseDeletedDestination,
    buildDeletedArgv,
    cancelDeletedPrompt,
    confirmDeletedView,
    computed,
  } from '$lib/app-state.svelte'

  const relativeUnitOptions = [
    { value: 'm', label: 'minutes ago' },
    { value: 'h', label: 'hours ago' },
    { value: 'd', label: 'days ago' },
  ]
</script>

<Dialog.Root bind:open={() => appState.deletedPromptFor !== null, (open) => { if (!open) cancelDeletedPrompt() }}>
  <Dialog.Content class="sm:max-w-2xl" aria-describedby={undefined}>
    {#if appState.deletedPromptFor}
      <form onsubmit={(event) => { event.preventDefault(); void confirmDeletedView() }}>
        <Dialog.Header>
          <Dialog.Title class="flex items-center gap-2"><Recycle size={20} aria-hidden="true" /> Open deleted-files view</Dialog.Title>
        </Dialog.Header>
        <div class="grid gap-4 py-4">
          <p>Mounts a flat, read-only listing of deleted files from this profile's volume ("{appState.deletedPromptFor.volume || appState.deletedPromptFor.name}") at a folder you choose. It appears as its own row once ready.</p>
          <div class="grid gap-1.5">
            <Label>Destination folder (optional)</Label>
            <div class="flex gap-2">
              <Input value={appState.deletedDestination} readonly placeholder="Auto-generated in a temp folder" class="flex-1" />
              <Button type="button" onclick={browseDeletedDestination} disabled={appState.busy} title="Choose a folder" class="shrink-0">
                <FolderOpen size={16} aria-hidden="true" />
                Browse
              </Button>
            </div>
          </div>
          <div class="grid gap-1.5">
            <Label for="deleted-from">From (optional, defaults to 7 days)</Label>
            <div class="flex gap-1.5" role="group" aria-label="From mode">
              <Button type="button" size="sm" variant={appState.deletedFromMode === 'default' ? 'primary' : 'outline'} onclick={() => (appState.deletedFromMode = 'default')}>Default</Button>
              <Button type="button" size="sm" variant={appState.deletedFromMode === 'absolute' ? 'primary' : 'outline'} onclick={() => (appState.deletedFromMode = 'absolute')}>Absolute</Button>
              <Button type="button" size="sm" variant={appState.deletedFromMode === 'relative' ? 'primary' : 'outline'} onclick={() => (appState.deletedFromMode = 'relative')}>Relative</Button>
            </div>
            {#if appState.deletedFromMode === 'absolute'}
              <DateTimePicker id="deleted-from" bind:value={appState.deletedFromAbsoluteValue} />
            {:else if appState.deletedFromMode === 'relative'}
              <div class="flex gap-2">
                <Input id="deleted-from" type="number" min="1" bind:value={appState.deletedFromRelativeQty} placeholder="e.g. 7" class="flex-1" />
                <Select options={relativeUnitOptions} bind:value={appState.deletedFromRelativeUnit} class="shrink-0 w-40" />
              </div>
            {/if}
          </div>
          <div class="grid gap-1.5">
            <Label for="deleted-idle-timeout">Idle timeout (optional)</Label>
            <Input id="deleted-idle-timeout" bind:value={appState.deletedIdleTimeout} placeholder="30m" />
          </div>
          {#if appState.deletedPromptFor.secretRef === 'prompt' || !appState.vaultStatus[appState.deletedPromptFor.id]}
            <div class="grid gap-1.5">
              <Label for="deleted-secret">Secret access key</Label>
              <Input id="deleted-secret" type="password" bind:value={appState.deletedSecretValue} autocomplete="current-password" />
            </div>
          {/if}
          {#if appState.deletedError}
            <Callout role="alert">{appState.deletedError}</Callout>
          {/if}
          <CommandPreview>
            <code>{`mountos ${buildDeletedArgv(appState.deletedPromptFor, appState.deletedDestination || '<destination>', computed.deletedFromValue, appState.deletedIdleTimeout).join(' ')}`}</code>
          </CommandPreview>
        </div>
        <Dialog.Footer>
          <Button type="button" variant="outline" onclick={cancelDeletedPrompt}>Cancel</Button>
          <Button type="submit" variant="primary" class="cyberpunk-skewed-sm" disabled={appState.busy}>Open</Button>
        </Dialog.Footer>
      </form>
    {/if}
  </Dialog.Content>
</Dialog.Root>
