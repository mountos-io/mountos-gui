<script lang="ts">
  import { ChevronLeft, ChevronRight, GitBranch, Plus, RefreshCw, RotateCcw, Trash2 } from '@lucide/svelte'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Select } from '$lib/components/ui/select'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import { Badge } from '$lib/components/ui/badge'
  import Callout from '$lib/components/Callout.svelte'
  import CommandPreview from '$lib/components/CommandPreview.svelte'
  import {
    appState,
    buildForkCreateArgv,
    buildForkDeleteArgv,
    buildForkListArgv,
    buildForkRestoreArgv,
    computed,
    drillIntoFork,
    runForkCreate,
    runForkDelete,
    runForkList,
    runForkRestore,
  } from '$lib/app-state.svelte'

  const profile = $derived(computed.selectedProfile!)

  // "main" (fid=0) is a valid --parent for a new fork, but not something you
  // can delete/restore -- two different option lists for the two pickers.
  const parentOptions = $derived([
    { value: '', label: 'main' },
    ...appState.forks.filter((fork) => fork.fid !== 0).map((fork) => ({ value: fork.name, label: fork.name })),
  ])
  const targetOptions = $derived(appState.forks.filter((fork) => fork.fid !== 0).map((fork) => ({ value: fork.name, label: fork.name })))

  const backTarget = $derived(computed.forkBreadcrumbTrail.length > 1 ? computed.forkBreadcrumbTrail[computed.forkBreadcrumbTrail.length - 2].fid : null)

  $effect(() => {
    if (profile.accessKeyId && appState.forks.length === 0) void runForkList()
  })
</script>

<!-- Fork delete/restore act on the shared volume, not just this profile: the
     warning callout below applies to the whole panel, not one action. -->
<div class="grid gap-4 border-t border-border pt-4">
  <div class="flex items-center justify-between gap-4">
    <div class="flex items-center gap-2">
      <GitBranch size={16} aria-hidden="true" />
      <h4 class="text-sm font-semibold">Fork management</h4>
    </div>
    <Button type="button" size="sm" variant="ghost" onclick={runForkList} disabled={appState.forkBusy || !profile.accessKeyId} title="Refresh fork list">
      <RefreshCw size={15} aria-hidden="true" />
    </Button>
  </div>
  <Callout>Fork delete/restore act on the shared volume, not just this profile. Deleting a fork is recoverable only within its grace period, and --force also removes its entire subtree.</Callout>

  {#if profile.secretRef === 'prompt' || !appState.vaultStatus[profile.id]}
    <div class="grid gap-1.5">
      <Label for="fork-secret">Secret access key</Label>
      <Input id="fork-secret" type="password" bind:value={appState.forkSecretValue} autocomplete="current-password" />
    </div>
  {/if}

  <CommandPreview>
    <code>{`mountos ${buildForkListArgv(profile).join(' ')}`}</code>
  </CommandPreview>

  <div class="corner-brackets relative border border-border/30 p-3">
    <div class="tech-grid absolute inset-0 pointer-events-none opacity-20" aria-hidden="true"></div>
    <div class="relative grid gap-2">
      {#if computed.forkBreadcrumbTrail.length > 0}
        <button type="button" class="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground w-fit" onclick={() => drillIntoFork(backTarget)}>
          <ChevronLeft size={15} aria-hidden="true" /> Back
        </button>
      {/if}
      {#if appState.forkBusy && appState.forks.length === 0}
        <p class="text-muted-foreground text-sm">Loading forks...</p>
      {:else if computed.forkChildren.length === 0}
        <p class="text-muted-foreground text-sm">{computed.currentFork ? 'No child forks.' : 'No forks yet.'}</p>
      {:else}
        {#each computed.forkChildren as fork (fork.fid)}
          <button
            type="button"
            class="flex items-center justify-between gap-2 border border-transparent p-2 text-left hover:bg-accent/50"
            onclick={() => drillIntoFork(fork.fid)}
          >
            <span class="flex items-center gap-2">
              <strong>{fork.name || 'main'}</strong>
              {#if fork.inactive}<Badge variant="warning">Deleted</Badge>{/if}
              {#if fork.isTemporary}<Badge>Temporary</Badge>{/if}
            </span>
            <span class="flex items-center gap-2 text-muted-foreground text-sm">
              {#if fork.childrenCount}{fork.childrenCount} {fork.childrenCount === 1 ? 'child' : 'children'}{/if}
              <ChevronRight size={15} aria-hidden="true" />
            </span>
          </button>
        {/each}
      {/if}
    </div>
  </div>

  <div class="grid gap-4 border-t border-border pt-4">
    <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
      <div class="grid gap-1.5">
        <Label for="fork-create-name">New fork name</Label>
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
    </div>
    <CommandPreview>
      <code>{`mountos ${buildForkCreateArgv(profile, appState.forkCreateName.trim() || '<name>', appState.forkCreateParent, computed.forkCreateAsOf).join(' ')}`}</code>
    </CommandPreview>
    <div>
      <Button type="button" onclick={runForkCreate} disabled={appState.forkBusy || !appState.forkCreateName.trim()}>
        <Plus size={16} aria-hidden="true" /> Create fork
      </Button>
    </div>
  </div>

  <div class="grid gap-4 border-t border-border pt-4">
    <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 items-end">
      <div class="grid gap-1.5">
        <Label id="fork-target-name-label">Fork name</Label>
        <Select options={targetOptions} bind:value={appState.forkTargetName} ariaLabelledby="fork-target-name-label" />
      </div>
      <Checkbox bind:checked={appState.forkDeleteForce} label="Also delete subtree (--force)" />
    </div>
    <CommandPreview label="DELETE">
      <code>{`mountos ${buildForkDeleteArgv(profile, appState.forkTargetName.trim() || '<name>', appState.forkDeleteForce).join(' ')}`}</code>
    </CommandPreview>
    <CommandPreview label="RESTORE">
      <code>{`mountos ${buildForkRestoreArgv(profile, appState.forkTargetName.trim() || '<name>').join(' ')}`}</code>
    </CommandPreview>
    <div class="flex gap-2">
      <Button type="button" variant="destructive" onclick={runForkDelete} disabled={appState.forkBusy || !appState.forkTargetName.trim()}>
        <Trash2 size={16} aria-hidden="true" /> Delete fork
      </Button>
      <Button type="button" onclick={runForkRestore} disabled={appState.forkBusy || !appState.forkTargetName.trim()}>
        <RotateCcw size={16} aria-hidden="true" /> Restore fork
      </Button>
    </div>
  </div>

  {#if appState.forkActionResultText}
    <CommandPreview label="RESULT">
      <pre class="m-0 whitespace-pre-wrap break-words"><code>{appState.forkActionResultText}</code></pre>
    </CommandPreview>
  {/if}
  {#if appState.forkError}
    <Callout role="alert">{appState.forkError}</Callout>
  {/if}
</div>
