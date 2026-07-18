<script lang="ts">
  import { FolderOpen, History } from '@lucide/svelte'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Select } from '$lib/components/ui/select'
  import Callout from '$lib/components/Callout.svelte'
  import CommandPreview from '$lib/components/CommandPreview.svelte'
  import InfoTip from '$lib/components/shared/InfoTip.svelte'
  import {
    appState,
    browseVersionDestination,
    buildVersionArgv,
    cancelVersionPrompt,
    confirmVersionView,
  } from '$lib/app-state.svelte'

  const versionFormatOptions = [
    { value: 'number', label: 'number (v1, v2, ...)' },
    { value: 'date', label: 'date' },
  ]
</script>

<Dialog.Root bind:open={() => appState.versionPromptFor !== null, (open) => { if (!open) cancelVersionPrompt() }}>
  <Dialog.Content class="sm:max-w-2xl" aria-describedby={undefined}>
    {#if appState.versionPromptFor}
      <form onsubmit={(event) => { event.preventDefault(); void confirmVersionView() }}>
        <Dialog.Header>
          <Dialog.Title class="flex items-center gap-2"><History size={20} aria-hidden="true" /> Open file-version view</Dialog.Title>
        </Dialog.Header>
        <div class="grid gap-4 py-4">
          <p>Mounts a read-only timeline of every version of one file from this profile's volume ("{appState.versionPromptFor.volume || appState.versionPromptFor.name}") at a folder you choose. It appears as its own row once ready.</p>
          <div class="grid gap-1.5">
            <Label>Destination folder (optional)</Label>
            <div class="flex gap-2">
              <Input value={appState.versionDestination} readonly placeholder="Auto-generated in a temp folder" class="flex-1" />
              <Button type="button" onclick={browseVersionDestination} disabled={appState.busy} title="Choose a folder" class="shrink-0">
                <FolderOpen size={16} aria-hidden="true" />
                Browse
              </Button>
            </div>
          </div>
          <div class="grid grid-cols-2 gap-4">
            <div class="grid gap-1.5">
              <span class="inline-flex items-center gap-1"><Label for="version-inode">Inode number</Label><InfoTip text="**ls -i** shows a file's inode." /></span>
              <Input id="version-inode" bind:value={appState.versionInode} inputmode="numeric" placeholder="12345" />
            </div>
            <div class="grid gap-1.5">
              <Label id="version-format-label">Version naming</Label>
              <Select options={versionFormatOptions} bind:value={appState.versionFormat} ariaLabelledby="version-format-label" />
            </div>
            <div class="grid gap-1.5">
              <Label for="version-idle-timeout">Idle timeout (optional)</Label>
              <Input id="version-idle-timeout" bind:value={appState.versionIdleTimeout} placeholder="30m" />
            </div>
          </div>
          {#if appState.versionPromptFor.secretRef === 'prompt' || !appState.vaultStatus[appState.versionPromptFor.id]}
            <div class="grid gap-1.5">
              <Label for="version-secret">Secret access key</Label>
              <Input id="version-secret" type="password" bind:value={appState.versionSecretValue} autocomplete="current-password" />
            </div>
          {/if}
          {#if appState.versionError}
            <Callout role="alert">{appState.versionError}</Callout>
          {/if}
          <CommandPreview>
            <code>{`mountos ${buildVersionArgv(appState.versionPromptFor, appState.versionDestination || '<destination>', appState.versionInode.trim() || '<inode>', appState.versionFormat, appState.versionIdleTimeout).join(' ')}`}</code>
          </CommandPreview>
        </div>
        <Dialog.Footer>
          <Button type="button" variant="outline" onclick={cancelVersionPrompt}>Cancel</Button>
          <Button type="submit" variant="primary" class="cyberpunk-skewed-sm" disabled={appState.busy || !/^\d+$/.test(appState.versionInode.trim())}>Open</Button>
        </Dialog.Footer>
      </form>
    {/if}
  </Dialog.Content>
</Dialog.Root>
