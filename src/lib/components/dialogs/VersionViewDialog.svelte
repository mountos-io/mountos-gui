<script lang="ts">
  import { FolderOpen, History, X } from '@lucide/svelte'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Select } from '$lib/components/ui/select'
  import CliErrorOutput from '$lib/components/CliErrorOutput.svelte'
  import CommandPreview from '$lib/components/CommandPreview.svelte'
  import InfoTip from '$lib/components/shared/InfoTip.svelte'
  import {
    appState,
    browseVersionDestination,
    browseVersionFile,
    buildVersionArgv,
    cancelVersionPrompt,
    confirmVersionView,
    mountAndBrowseVersionFile,
    primaryInstanceForProfile,
  } from '$lib/app-state.svelte'

  const versionFormatOptions = [
    { value: 'number', label: 'number (v1, v2, ...)' },
    { value: 'date', label: 'date' },
  ]

  const mountedInstance = $derived(appState.versionPromptFor ? primaryInstanceForProfile(appState.versionPromptFor.id) : undefined)
  const selector = $derived(appState.versionPath.trim() ? { path: appState.versionPath.trim() } : { inode: appState.versionInode.trim() })
  const canSubmit = $derived(Boolean(appState.versionPath.trim()) || /^\d+$/.test(appState.versionInode.trim()))
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
            <Label>File</Label>
            <div class="flex gap-2">
              <Input value={appState.versionPath} readonly placeholder="Browse to a file..." class="flex-1" />
              {#if appState.versionPath}
                <Button type="button" variant="outline" onclick={() => { appState.versionPath = '' }} disabled={appState.busy} title="Clear, to enter an inode instead" class="shrink-0">
                  <X size={16} aria-hidden="true" />
                </Button>
              {:else if mountedInstance}
                <Button type="button" onclick={() => appState.versionPromptFor && browseVersionFile(appState.versionPromptFor)} disabled={appState.busy} title="Choose a file" class="shrink-0">
                  <FolderOpen size={16} aria-hidden="true" />
                  Browse
                </Button>
              {:else}
                <Button type="button" onclick={() => appState.versionPromptFor && mountAndBrowseVersionFile(appState.versionPromptFor)} disabled={appState.busy} title="Mount this profile, then choose a file" class="shrink-0">
                  <FolderOpen size={16} aria-hidden="true" />
                  Mount &amp; Browse
                </Button>
              {/if}
            </div>
          </div>

          <Checkbox bind:checked={appState.versionFullChain} label="Full chain (also follow this file across past moves)" />

          <details class="text-sm">
            <summary class="cursor-pointer select-none text-muted-foreground">Advanced: enter an inode number instead</summary>
            <div class="grid gap-1.5 pt-2">
              <span class="inline-flex items-center gap-1"><Label for="version-inode">Inode number</Label><InfoTip text="**ls -i** shows a file's inode. Bypasses Browse; only a plain by-inode lookup (no multi-key discovery)." /></span>
              <Input id="version-inode" bind:value={appState.versionInode} inputmode="numeric" placeholder="12345" disabled={Boolean(appState.versionPath)} />
            </div>
          </details>

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
            <CliErrorOutput
              role="alert"
              text={appState.versionError}
              command={`mountos ${buildVersionArgv(appState.versionPromptFor, appState.versionDestination || '<destination>', selector, appState.versionFormat, appState.versionIdleTimeout, appState.versionFullChain).join(' ')}`}
            />
          {/if}
          <CommandPreview>
            <code>{`mountos ${buildVersionArgv(appState.versionPromptFor, appState.versionDestination || '<destination>', selector, appState.versionFormat, appState.versionIdleTimeout, appState.versionFullChain).join(' ')}`}</code>
          </CommandPreview>
        </div>
        <Dialog.Footer>
          <Button type="button" variant="outline" onclick={cancelVersionPrompt}>Cancel</Button>
          <Button type="submit" variant="primary" class="cyberpunk-skewed-sm" disabled={appState.busy || !canSubmit}>Open</Button>
        </Dialog.Footer>
      </form>
    {/if}
  </Dialog.Content>
</Dialog.Root>
