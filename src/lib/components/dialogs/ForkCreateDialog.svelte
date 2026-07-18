<script lang="ts">
  import { Plus } from '@lucide/svelte'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Select } from '$lib/components/ui/select'
  import Callout from '$lib/components/Callout.svelte'
  import CommandPreview from '$lib/components/CommandPreview.svelte'
  import { appState, buildForkCreateArgv, cancelForkCreate, computed, confirmForkCreate } from '$lib/app-state.svelte'

  const parentOptions = $derived([
    { value: '', label: 'main' },
    ...appState.forks.filter((fork) => fork.fid !== 0).map((fork) => ({ value: fork.name, label: fork.name })),
  ])
</script>

<Dialog.Root bind:open={() => appState.forkCreatePromptFor !== null, (open) => { if (!open) cancelForkCreate() }}>
  <Dialog.Content class="sm:max-w-lg" aria-describedby={undefined}>
    {#if appState.forkCreatePromptFor}
      <form onsubmit={(event) => { event.preventDefault(); void confirmForkCreate() }}>
        <Dialog.Header>
          <Dialog.Title class="flex items-center gap-2"><Plus size={20} aria-hidden="true" /> New fork</Dialog.Title>
        </Dialog.Header>
        <div class="grid gap-4 py-4">
          <div class="grid gap-1.5">
            <Label for="fork-create-name">Fork name</Label>
            <Input id="fork-create-name" bind:value={appState.forkCreateName} />
          </div>
          <div class="grid gap-1.5">
            <Label id="fork-create-parent-label">Parent fork (optional)</Label>
            <Select options={parentOptions} bind:value={appState.forkCreateParent} ariaLabelledby="fork-create-parent-label" />
          </div>
          <div class="grid gap-1.5">
            <Label for="fork-create-as-of">As of (optional)</Label>
            <Input id="fork-create-as-of" type="datetime-local" bind:value={appState.forkCreateAsOfLocal} />
            <small class="text-muted-foreground text-sm">Leave blank to branch from the parent's current state.</small>
          </div>
          {#if appState.forkCreatePromptFor.secretRef === 'prompt' || !appState.vaultStatus[appState.forkCreatePromptFor.id]}
            <div class="grid gap-1.5">
              <Label for="fork-create-secret">Secret access key</Label>
              <Input id="fork-create-secret" type="password" bind:value={appState.forkCreateSecretValue} autocomplete="current-password" />
            </div>
          {/if}
          {#if appState.forkCreateError}
            <Callout role="alert">{appState.forkCreateError}</Callout>
          {/if}
          <CommandPreview>
            <code>{`mountos ${buildForkCreateArgv(appState.forkCreatePromptFor, appState.forkCreateName.trim() || '<name>', appState.forkCreateParent, computed.forkCreateAsOf).join(' ')}`}</code>
          </CommandPreview>
        </div>
        <Dialog.Footer>
          <Button type="button" variant="outline" onclick={cancelForkCreate}>Cancel</Button>
          <Button type="submit" variant="primary" class="cyberpunk-skewed-sm" disabled={appState.forkBusy || !appState.forkCreateName.trim()}>Create</Button>
        </Dialog.Footer>
      </form>
    {/if}
  </Dialog.Content>
</Dialog.Root>
