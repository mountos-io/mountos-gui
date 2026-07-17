<script lang="ts">
  import {
    AlertTriangle,
    Camera,
    Copy,
    FileDown,
    FilePlus,
    FolderOpen,
    HardDrive,
    History,
    KeyRound,
    Network,
    Plus,
    Power,
    Recycle,
    Save,
    Trash2,
  } from '@lucide/svelte'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Textarea } from '$lib/components/ui/textarea'
  import { Select } from '$lib/components/ui/select'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import { Badge } from '$lib/components/ui/badge'
  import Callout from '$lib/components/Callout.svelte'
  import CommandPreview from '$lib/components/CommandPreview.svelte'
  import ForkManagementPanel from '$lib/components/ForkManagementPanel.svelte'
  import { FSKIT_MOUNT_PREFIX } from '$lib/cli'
  import type { Backend } from '$lib/types'
  import {
    ACCESS_KEY_ID_LENGTH,
    appState,
    browseMountPath,
    buildMountArgv,
    computed,
    duplicateSelected,
    exportSelected,
    forgetSecret,
    newProfile,
    patchProfile,
    persistSelected,
    requestDeletedView,
    requestGatewayView,
    requestSnapshotView,
    requestVersionView,
    runMount,
    selectProfile,
    setAccessKeyId,
    setExtraArgs,
    toggleMountHelp,
  } from '$lib/app-state.svelte'

  const secretRefOptions = $derived([
    { value: 'prompt', label: 'Prompt on mount' },
    { value: 'vault', label: 'Vault' },
  ])

  // Only offered while unset: once a real mount detects the volume kind,
  // require_stable_identity (src-tauri/src/lib.rs) locks it server-side and
  // this profile switches to the read-only Badge instead of this Select.
  const volumeKindOptions = [
    { value: '', label: 'Auto-detect on first mount' },
    { value: 'general', label: 'General' },
    { value: 'iceberg', label: 'Iceberg' },
  ]
</script>

<section class="grid grid-cols-[240px_minmax(0,1fr)] gap-4 m-[22px]">
  <div class="surface p-4">
    <h3 class="mb-4">Saved Profiles</h3>
    <div class="grid gap-1.5">
      {#each appState.profiles as profile (profile.id)}
        <button
          class:bg-accent={appState.selectedProfileId === profile.id}
          class="flex min-w-0 items-center gap-2.5 border border-transparent p-2 text-left outline-none hover:bg-accent/50 focus-visible:ring-2 focus-visible:ring-ring"
          type="button"
          title={profile.mountPath || 'No target selected'}
          onclick={() => selectProfile(profile)}
        >
          <HardDrive size={17} aria-hidden="true" class="shrink-0" />
          <strong class="min-w-0 truncate">{profile.name}</strong>
        </button>
      {:else}
        <div class="tech-grid p-7 text-center">
          <p>No saved profiles yet.</p>
        </div>
      {/each}
    </div>
  </div>

  {#if computed.selectedProfile}
    {@const selectedProfile = computed.selectedProfile}
    <form class="surface corner-brackets p-4 grid gap-4" onsubmit={(event) => { event.preventDefault(); void persistSelected() }}>
      <div class="flex items-start justify-between gap-4">
        {#if selectedProfile.volumeKind}
          <Badge title="Volume kind, detected from the mount itself; locks the profile's identity fields">
            {selectedProfile.volumeKind === 'iceberg' ? 'Iceberg' : 'General'}
          </Badge>
        {/if}
        <div class="relative flex flex-wrap items-center gap-2 border border-border/30 p-2 ml-auto">
          <div class="tech-grid absolute inset-0 pointer-events-none opacity-20" aria-hidden="true"></div>
          <Button variant="outline" size="icon" class="relative" title="Duplicate profile" aria-label="Duplicate profile" disabled={appState.busy} onclick={duplicateSelected}>
            <Copy size={16} aria-hidden="true" />
          </Button>
          <Button variant="outline" size="icon" class="relative" title="Export profile (no secret)" aria-label="Export profile" disabled={appState.busy} onclick={exportSelected}>
            <FileDown size={16} aria-hidden="true" />
          </Button>
          <Button variant="destructive" size="icon" class="relative" title="Delete profile" aria-label="Delete profile" disabled={appState.busy} onclick={() => (appState.deletePromptFor = selectedProfile)}>
            <Trash2 size={16} aria-hidden="true" />
          </Button>
          <Button type="button" class="relative" disabled={appState.busy || !!appState.extraArgsError || appState.rejectedArgs.length > 0 || !!computed.mountPathError || !!computed.accessKeyError || !!computed.volumeNameError} onclick={() => runMount(selectedProfile)}>
            <Power size={16} aria-hidden="true" />
            Mount
          </Button>
          {#if selectedProfile.volumeKind !== 'iceberg'}
            <Button type="button" class="relative" title="Open a read-only, point-in-time view of this volume" disabled={appState.busy} onclick={() => requestSnapshotView(selectedProfile)}>
              <Camera size={16} aria-hidden="true" />
              Snapshot
            </Button>
            <Button type="button" class="relative" title="Open a read-only view of this volume's deleted files" disabled={appState.busy} onclick={() => requestDeletedView(selectedProfile)}>
              <Recycle size={16} aria-hidden="true" />
              Deleted files
            </Button>
            <Button type="button" class="relative" title="Open a read-only view of one file's version history" disabled={appState.busy} onclick={() => requestVersionView(selectedProfile)}>
              <History size={16} aria-hidden="true" />
              Versions
            </Button>
            <Button type="button" class="relative" title="Launch an S3/HDFS gateway for this volume" disabled={appState.busy} onclick={() => requestGatewayView(selectedProfile)}>
              <Network size={16} aria-hidden="true" />
              Gateway
            </Button>
          {/if}
          <Button variant="primary" type="submit" class="relative cyberpunk-skewed-sm" disabled={appState.busy || !!appState.extraArgsError || appState.rejectedArgs.length > 0 || !!computed.mountPathError || !!computed.accessKeyError || !!computed.volumeNameError}>
            <Save size={16} aria-hidden="true" />
            Save
          </Button>
        </div>
      </div>

      <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <div class="grid gap-1.5">
          <Label for="profile-name">Name</Label>
          <Input id="profile-name" value={selectedProfile.name} oninput={(e) => patchProfile({ name: e.currentTarget.value })} />
        </div>
        <div class="grid gap-1.5">
          <Label id="profile-backend-label">Backend</Label>
          <Select
            options={computed.backends.map((backend) => ({ value: backend, label: backend }))}
            value={selectedProfile.backend}
            ariaLabelledby="profile-backend-label"
            onchange={(value) => patchProfile({ backend: value as Backend })}
          />
        </div>
        {#if !selectedProfile.volumeKind}
          <div class="grid gap-1.5">
            <!-- Editable only until the first real mount detects it: once set,
                 require_stable_identity (src-tauri/src/lib.rs) locks it
                 server-side and this profile shows the read-only Badge above
                 instead. -->
            <Label id="profile-volume-kind-label">Volume kind</Label>
            <Select
              options={volumeKindOptions}
              value={selectedProfile.volumeKind ?? ''}
              onchange={(value) => patchProfile({ volumeKind: (value || undefined) as 'general' | 'iceberg' | undefined })}
              ariaLabelledby="profile-volume-kind-label"
            />
          </div>
        {/if}
        <div class="grid gap-1.5">
          <Label for="profile-discovery-url">Discovery URL</Label>
          <Input
            id="profile-discovery-url"
            value={selectedProfile.discoveryUrl}
            disabled={Boolean(selectedProfile.volumeKind)}
            oninput={(e) => patchProfile({ discoveryUrl: e.currentTarget.value })}
          />
          {#if selectedProfile.volumeKind}
            <small class="text-muted-foreground text-sm">Locked: this profile's volume kind is known, so its identity can't change. Delete and recreate to point at a different volume.</small>
          {/if}
        </div>
        <div class="grid gap-1.5">
          <Label for="profile-access-key">Access key ID</Label>
          <Input
            id="profile-access-key"
            value={selectedProfile.accessKeyId}
            maxlength={ACCESS_KEY_ID_LENGTH}
            disabled={Boolean(selectedProfile.volumeKind)}
            oninput={(e) => setAccessKeyId(e.currentTarget.value)}
          />
          {#if computed.accessKeyError}
            <small class="text-destructive text-sm">{computed.accessKeyError}</small>
          {/if}
        </div>
        <div class="grid gap-1.5">
          <Label for="profile-volume">Volume name</Label>
          <Input
            id="profile-volume"
            value={selectedProfile.volume}
            disabled={Boolean(selectedProfile.volumeKind)}
            oninput={(e) => patchProfile({ volume: e.currentTarget.value })}
          />
          {#if computed.volumeNameError}
            <small class="text-destructive text-sm">{computed.volumeNameError}</small>
          {:else if selectedProfile.backend === 'fskit'}
            <small class="text-muted-foreground text-sm">Used as the mount point's folder name under {FSKIT_MOUNT_PREFIX}</small>
          {/if}
        </div>
        <div class="grid gap-1.5">
          <Label for="profile-fork">Fork</Label>
          <Input id="profile-fork" value={selectedProfile.fork} oninput={(e) => patchProfile({ fork: e.currentTarget.value })} />
        </div>
        <div class="grid gap-1.5">
          <Label for="mount-path">Mount path</Label>
          {#if computed.mountPathIsManaged}
            <Input id="mount-path" value={selectedProfile.mountPath} disabled placeholder="Managed automatically by the OS" />
            <small class="text-muted-foreground text-sm">
              {selectedProfile.backend === 'fileprovider'
                ? 'FileProvider mounts have no filesystem path; the volume appears in Finder under its volume name.'
                : 'CloudFilter mounts have no filesystem path; the volume appears under its own drive/namespace.'}
            </small>
          {:else}
            <div class="flex gap-2">
              <Input
                id="mount-path"
                value={selectedProfile.mountPath}
                placeholder={selectedProfile.backend === 'fskit' ? '/Volumes/MountOS/<name>' : undefined}
                oninput={(e) => patchProfile({ mountPath: e.currentTarget.value })}
                class="flex-1"
              />
              <Button type="button" onclick={browseMountPath} disabled={appState.busy} title="Choose a folder" class="shrink-0">
                <FolderOpen size={16} aria-hidden="true" />
                Browse
              </Button>
            </div>
          {/if}
        </div>
        <div class="grid gap-1.5">
          <Label id="profile-secret-ref-label">Secret</Label>
          <Select
            options={secretRefOptions}
            value={selectedProfile.secretRef}
            ariaLabelledby="profile-secret-ref-label"
            onchange={(value) => patchProfile({ secretRef: value as 'vault' | 'prompt' })}
          />
          {#if !selectedProfile.accessKeyId}
            <small class="text-muted-foreground text-sm">Vault storage needs an access key ID first.</small>
          {/if}
        </div>
      </div>

      {#if computed.mountPathError}
        <Callout>{computed.mountPathError}</Callout>
      {/if}

      <div class="flex items-center justify-between gap-4">
        <Badge variant={appState.vaultStatus[selectedProfile.id] ? 'success' : 'warning'}>
          <KeyRound size={14} aria-hidden="true" />
          {appState.vaultStatus[selectedProfile.id] ? 'Secret stored' : 'No vaulted secret'}
        </Badge>
        <Button variant="destructive" disabled={!appState.vaultStatus[selectedProfile.id] || appState.busy} onclick={() => forgetSecret(selectedProfile.id)}>
          <Trash2 size={16} aria-hidden="true" />
          Forget secret
        </Button>
      </div>

      <div class="flex flex-wrap gap-4">
        <Checkbox checked={selectedProfile.readOnly} onchange={(e) => patchProfile({ readOnly: e.currentTarget.checked })} label="Read only" />
        <Checkbox
          checked={selectedProfile.temporaryFork}
          onchange={(e) => patchProfile({ temporaryFork: e.currentTarget.checked })}
          label="Temporary fork"
          title="Creates an ephemeral per-session fork for this mount; discarded when it unmounts, the underlying volume is never touched"
        />
      </div>

      <div class="grid gap-1.5">
        <div class="flex items-center justify-between gap-3">
          <Label for="advanced-options">Advanced options</Label>
          <Button type="button" variant="ghost" size="sm" onclick={toggleMountHelp} disabled={appState.busy} aria-expanded={appState.mountHelpVisible}>
            {appState.mountHelpVisible ? 'Hide help' : 'mountos mount -h'}
          </Button>
        </div>
        <Textarea
          id="advanced-options"
          value={appState.extraArgsInput}
          oninput={(e) => setExtraArgs(e.currentTarget.value)}
          placeholder="Flags mountos mount accepts but this form doesn't manage, e.g. --disk-cache-size 10G"
        />
      </div>

      {#if appState.mountHelpVisible}
        <CommandPreview label="MOUNTOS MOUNT -H">
          <pre class="m-0 whitespace-pre-wrap break-words"><code>{appState.mountHelpText}</code></pre>
        </CommandPreview>
      {/if}

      {#if appState.settings.advancedOpsEnabled}
        <ForkManagementPanel />
      {/if}

      {#if appState.extraArgsError}
        <Callout>{appState.extraArgsError}</Callout>
      {/if}

      {#if appState.rejectedArgs.length}
        <Callout>Rejected managed flags: {appState.rejectedArgs.join(', ')}</Callout>
      {/if}

      <CommandPreview label="COMMAND PREVIEW">
        <code>{appState.commandText || `mountos ${buildMountArgv(selectedProfile).join(' ')}`}</code>
      </CommandPreview>
    </form>
  {:else}
    <div class="surface tech-grid grid justify-items-center gap-2 p-7 text-center">
      <FilePlus size={28} aria-hidden="true" />
      <strong>No profile selected</strong>
      <p>Save a profile to mount a mountOS volume in one click, with credentials in the OS vault and the exact CLI command shown before every action.</p>
      <Button variant="primary" type="button" class="cyberpunk-skewed-sm" onclick={() => newProfile()}>
        <Plus size={17} aria-hidden="true" />
        New profile
      </Button>
    </div>
  {/if}
</section>
