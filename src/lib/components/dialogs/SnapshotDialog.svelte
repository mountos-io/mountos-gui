<script lang="ts">
  import { Camera, FolderOpen } from '@lucide/svelte'
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
    browseSnapshotDestination,
    buildSnapshotArgv,
    cancelSnapshotPrompt,
    confirmSnapshotView,
    computed,
  } from '$lib/app-state.svelte'

  const relativeUnitOptions = [
    { value: 'm', label: 'minutes ago' },
    { value: 'h', label: 'hours ago' },
    { value: 'd', label: 'days ago' },
  ]
</script>

<Dialog.Root bind:open={() => appState.snapshotPromptFor !== null, (open) => { if (!open) cancelSnapshotPrompt() }}>
  <Dialog.Content class="sm:max-w-2xl" aria-describedby={undefined}>
    {#if appState.snapshotPromptFor}
      <form onsubmit={(event) => { event.preventDefault(); void confirmSnapshotView() }}>
        <Dialog.Header>
          <Dialog.Title class="flex items-center gap-2"><Camera size={20} aria-hidden="true" /> Open snapshot view</Dialog.Title>
        </Dialog.Header>
        <div class="grid gap-4 py-4">
          <p>Mounts a read-only, point-in-time view of this profile's volume ("{appState.snapshotPromptFor.volume || appState.snapshotPromptFor.name}") at a folder you choose. It appears as its own row once ready.</p>
          <div class="grid gap-1.5">
            <Label>Destination folder</Label>
            <div class="flex gap-2">
              <Input value={appState.snapshotDestination} readonly placeholder="Choose a folder" class="flex-1" />
              <Button type="button" onclick={browseSnapshotDestination} disabled={appState.busy} title="Choose a folder" class="shrink-0">
                <FolderOpen size={16} aria-hidden="true" />
                Browse
              </Button>
            </div>
          </div>
          <div class="grid gap-1.5">
            <Label for="snapshot-timestamp">Timestamp</Label>
            <div class="flex gap-1.5" role="group" aria-label="Timestamp mode">
              <Button type="button" size="sm" variant={appState.snapshotTimeMode === 'absolute' ? 'primary' : 'outline'} onclick={() => (appState.snapshotTimeMode = 'absolute')}>Absolute</Button>
              <Button type="button" size="sm" variant={appState.snapshotTimeMode === 'relative' ? 'primary' : 'outline'} onclick={() => (appState.snapshotTimeMode = 'relative')}>Relative</Button>
            </div>
            {#if appState.snapshotTimeMode === 'absolute'}
              <DateTimePicker id="snapshot-timestamp" bind:value={appState.snapshotAbsoluteValue} />
            {:else}
              <div class="flex gap-2">
                <Input id="snapshot-timestamp" type="number" min="1" bind:value={appState.snapshotRelativeQty} placeholder="e.g. 2" class="flex-1" />
                <Select options={relativeUnitOptions} bind:value={appState.snapshotRelativeUnit} class="shrink-0 w-40" />
              </div>
            {/if}
          </div>
          {#if appState.snapshotPromptFor.secretRef === 'prompt' || !appState.vaultStatus[appState.snapshotPromptFor.id]}
            <div class="grid gap-1.5">
              <Label for="snapshot-secret">Secret access key</Label>
              <Input id="snapshot-secret" type="password" bind:value={appState.snapshotSecretValue} autocomplete="current-password" />
            </div>
          {/if}
          {#if appState.snapshotError}
            <Callout role="alert">{appState.snapshotError}</Callout>
          {/if}
          <CommandPreview>
            <code>{`mountos ${buildSnapshotArgv(appState.snapshotPromptFor, appState.snapshotDestination || '<destination>', computed.snapshotTimestampValue || '<timestamp>').join(' ')}`}</code>
          </CommandPreview>
        </div>
        <Dialog.Footer>
          <Button type="button" variant="outline" class="cyberpunk-skewed-sm" onclick={cancelSnapshotPrompt}>Cancel</Button>
          <Button type="submit" variant="primary" class="cyberpunk-skewed-sm" disabled={appState.busy || !appState.snapshotDestination || !computed.snapshotTimestampValue}>Open</Button>
        </Dialog.Footer>
      </form>
    {/if}
  </Dialog.Content>
</Dialog.Root>
