<script lang="ts">
  import { FolderOpen, Network } from '@lucide/svelte'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import Callout from '$lib/components/Callout.svelte'
  import CommandPreview from '$lib/components/CommandPreview.svelte'
  import {
    appState,
    browseGatewayCert,
    browseGatewayKey,
    buildGatewayArgv,
    cancelGatewayPrompt,
    confirmGatewayView,
    computed,
  } from '$lib/app-state.svelte'
</script>

<Dialog.Root bind:open={() => appState.gatewayPromptFor !== null, (open) => { if (!open) cancelGatewayPrompt() }}>
  <Dialog.Content class="sm:max-w-2xl" aria-describedby={undefined}>
    {#if appState.gatewayPromptFor}
      <form onsubmit={(event) => { event.preventDefault(); void confirmGatewayView() }}>
        <Dialog.Header>
          <Dialog.Title class="flex items-center gap-2"><Network size={20} aria-hidden="true" /> Launch gateway</Dialog.Title>
        </Dialog.Header>
        <div class="grid gap-4 py-4">
          <p>
            Exposes "{appState.gatewayPromptFor.volume || appState.gatewayPromptFor.name}" over S3/HDFS.
            {appState.gatewayOnly ? 'Runs without a FUSE mount.' : `Combines with this profile's own mount at ${appState.gatewayPromptFor.mountPath}.`}
          </p>
          <div class="flex flex-wrap gap-4">
            <Checkbox bind:checked={appState.gatewayS3} label="S3" />
            <Checkbox bind:checked={appState.gatewayHdfs} label="HDFS" />
          </div>
          <div class="grid gap-1.5">
            <Label for="gateway-port">Port (optional)</Label>
            <Input id="gateway-port" bind:value={appState.gatewayPort} placeholder="0 (auto)" />
          </div>
          <div class="flex flex-wrap gap-4">
            <Checkbox bind:checked={appState.gatewayOnly} label="Gateway only (no FUSE mount)" />
            <Checkbox bind:checked={appState.gatewayNoLoopback} label="Bind on all interfaces (requires TLS)" />
          </div>
          {#if appState.gatewayNoLoopback}
            <div class="grid grid-cols-2 gap-4">
              <div class="grid gap-1.5">
                <Label>TLS certificate</Label>
                <div class="flex gap-2">
                  <Input value={appState.gatewayCertPath} readonly placeholder="Choose a file" class="flex-1" />
                  <Button type="button" onclick={browseGatewayCert} disabled={appState.busy} class="shrink-0">
                    <FolderOpen size={16} aria-hidden="true" /> Browse
                  </Button>
                </div>
              </div>
              <div class="grid gap-1.5">
                <Label>TLS key</Label>
                <div class="flex gap-2">
                  <Input value={appState.gatewayKeyPath} readonly placeholder="Choose a file" class="flex-1" />
                  <Button type="button" onclick={browseGatewayKey} disabled={appState.busy} class="shrink-0">
                    <FolderOpen size={16} aria-hidden="true" /> Browse
                  </Button>
                </div>
              </div>
            </div>
          {/if}
          {#if appState.gatewayPromptFor.secretRef === 'prompt' || !appState.vaultStatus[appState.gatewayPromptFor.id]}
            <div class="grid gap-1.5">
              <Label for="gateway-secret">Secret access key</Label>
              <Input id="gateway-secret" type="password" bind:value={appState.gatewaySecretValue} autocomplete="current-password" />
            </div>
          {/if}
          {#if appState.gatewayError}
            <Callout role="alert">{appState.gatewayError}</Callout>
          {/if}
          <CommandPreview>
            <code
              >{`mountos ${buildGatewayArgv(appState.gatewayPromptFor, {
                protocols: computed.gatewayProtocols,
                port: appState.gatewayPort,
                gatewayOnly: appState.gatewayOnly,
                noLoopback: appState.gatewayNoLoopback,
                certPath: appState.gatewayCertPath,
                keyPath: appState.gatewayKeyPath,
              }).join(' ')}`}</code
            >
          </CommandPreview>
        </div>
        <Dialog.Footer>
          <Button type="button" variant="outline" class="cyberpunk-skewed-sm" onclick={cancelGatewayPrompt}>Cancel</Button>
          <Button
            type="submit"
            variant="primary"
            class="cyberpunk-skewed-sm"
            disabled={appState.busy || computed.gatewayProtocols.length === 0 || (appState.gatewayNoLoopback && (!appState.gatewayCertPath.trim() || !appState.gatewayKeyPath.trim()))}
          >
            Launch
          </Button>
        </Dialog.Footer>
      </form>
    {/if}
  </Dialog.Content>
</Dialog.Root>
