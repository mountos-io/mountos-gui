<script lang="ts">
  import {
    ChevronDown,
    ChevronRight,
    Copy,
    EllipsisVertical,
    FilePlus,
    FolderOpen,
    History,
    LayoutDashboard,
    OctagonX,
    Recycle,
    Search,
    SquareTerminal,
    Unplug,
  } from '@lucide/svelte'
  import { DropdownMenu } from 'bits-ui'
  import * as Table from '$lib/components/ui/table'
  import { Button } from '$lib/components/ui/button'
  import { Badge } from '$lib/components/ui/badge'
  import { Input } from '$lib/components/ui/input'
  import { Skeleton } from '$lib/components/ui/skeleton'
  import GatewayLaunchesPanel from '$lib/components/GatewayLaunchesPanel.svelte'
  import InstanceConfigPanel from '$lib/components/InstanceConfigPanel.svelte'
  import { backendBadgeStyle, formatMountedSince, formatUptime, healthTitle, healthTone, viewModeBadge, volumeKindBadgeStyle } from '$lib/health'
  import type { MountInstance } from '$lib/types'
  import {
    appState,
    canOpen,
    canOpenViewsFor,
    cloneProfileFor,
    copyConfig,
    computed,
    DEFAULT_POLL_SECONDS,
    gatewayInfoForInstance,
    openDashboard,
    profileForInstance,
    requestDeletedView,
    requestUnmount,
    requestUnmountAll,
    requestVersionView,
    runOpen,
    saveAsProfile,
    stopGatewayLaunch,
    toggleInstanceConfig,
  } from '$lib/app-state.svelte'
</script>

<div class="corner-brackets relative border border-border/30 m-[22px] mb-0 p-4">
  <div class="tech-grid absolute inset-0 pointer-events-none" aria-hidden="true"></div>
  <div class="relative flex flex-wrap items-center justify-between gap-3">
    <label class="relative flex items-center max-w-sm">
      <Search size={16} aria-hidden="true" class="absolute left-3 text-muted-foreground" />
      <span class="sr-only">Search instances</span>
      <!-- dark:bg-background overrides the Input component's own dark:bg-input/20:
           that low-alpha fill is fine over a plain surface, but here it sits
           directly on top of the tech-grid overlay two levels up, so the grid
           pattern shows straight through and the field stops reading as a box. -->
      <Input bind:value={appState.query} placeholder="Filter mounts" class="pl-9 dark:bg-background" />
    </label>
    <div class="flex flex-wrap items-center gap-2">
      <Badge>{appState.systemState.instances.length} running</Badge>
      {#if computed.limitedCount > 0}
        <Badge variant="warning">{computed.limitedCount} limited</Badge>
      {/if}
      <Button variant="destructive" disabled={appState.busy || appState.systemState.instances.length === 0} onclick={requestUnmountAll}>
        <Unplug size={16} aria-hidden="true" />
        Unmount all
      </Button>
    </div>
  </div>
</div>

<section class="surface m-[22px] mt-4 p-4">
  <h3 class="mb-6">Running instances</h3>
  <Table.Root containerLabel="Running instances">
    <Table.Header>
      <Table.Row>
        <Table.Head class="th-cyber">Name</Table.Head>
        <Table.Head class="th-cyber">Target</Table.Head>
        <Table.Head class="th-cyber">Backend</Table.Head>
        <Table.Head class="th-cyber w-40">Actions</Table.Head>
      </Table.Row>
    </Table.Header>
    <Table.Body>
      {#if !appState.loaded}
        {#each { length: 3 } as _placeholder}
          <Table.Row aria-hidden="true">
            <Table.Cell><Skeleton class="h-5 w-32" /></Table.Cell>
            <Table.Cell><Skeleton class="h-5 w-64" /></Table.Cell>
            <Table.Cell><Skeleton class="h-5 w-20" /></Table.Cell>
            <Table.Cell><Skeleton class="h-5 w-20" /></Table.Cell>
          </Table.Row>
        {/each}
      {:else}
        {#each computed.filteredInstances as instance (instance.key)}
          <Table.Row>
            <Table.Cell>
              <span class="flex flex-wrap items-center gap-2">
                <!-- title for pointer users; the sr-only text is what makes
                     this readable at all without colour vision or a mouse,
                     since the dot alone carries the state (WCAG 1.4.1). -->
                <span class="led {healthTone(instance.health)}" title={healthTitle(instance.health)}></span>
                <span class="sr-only">{healthTitle(instance.health)}</span>
                <!-- No "External" badge: the Actions column already says it.
                     An external mount is exactly one offering "Save as
                     profile" rather than "Clone profile", so the badge only
                     restated the row next to it. -->
                <strong>{instance.name || instance.volumeId || 'mountOS volume'}</strong>
                {#if viewModeBadge(instance.viewMode)}
                  <Badge>{viewModeBadge(instance.viewMode)}</Badge>
                {/if}
                {#if profileForInstance(instance)?.volumeKind}
                  <Badge variant="secondary" style={volumeKindBadgeStyle(profileForInstance(instance)?.volumeKind)} title="Volume kind, detected from the mount itself">
                    {profileForInstance(instance)?.volumeKind === 'iceberg' ? 'Iceberg' : 'General'}
                  </Badge>
                {/if}
                {#if gatewayInfoForInstance(instance)}
                  <Badge title="This mount also has an S3/HDFS gateway running, launched from this app">Gateway</Badge>
                {/if}
                {#if (appState.settings.pollSeconds ?? DEFAULT_POLL_SECONDS) === 0}
                  <!-- Polling off: nothing re-renders this row again until a manual
                       refresh, so a relative "Up Xh Ym" would silently freeze and
                       read as live when it isn't. An absolute time stays correct
                       regardless of how long it sits unrefreshed. -->
                  {#if formatMountedSince(instance.mountTime)}
                    <Badge variant="secondary" title="Auto-refresh is off -- hit Refresh to update">Since {formatMountedSince(instance.mountTime)}</Badge>
                  {/if}
                {:else if formatUptime(instance.mountTime)}
                  <Badge variant="secondary" title={`Mounted at ${formatMountedSince(instance.mountTime)}`}>Up {formatUptime(instance.mountTime)}</Badge>
                {/if}
              </span>
            </Table.Cell>
            <Table.Cell><code>{instance.mountPath}</code></Table.Cell>
            <Table.Cell><Badge variant="secondary" style={backendBadgeStyle(instance.backend)}>{instance.backend ?? 'unknown'}</Badge></Table.Cell>
            <Table.Cell>
              <div class="flex flex-nowrap items-center gap-2">
                {#if canOpen(instance)}
                  <Button
                    variant="outline"
                    size="icon"
                    title={instance.key in appState.expandedConfig ? 'Hide mount flags' : 'Show mount flags'}
                    aria-label={instance.key in appState.expandedConfig ? 'Hide mount flags' : 'Show mount flags'}
                    aria-expanded={instance.key in appState.expandedConfig}
                    disabled={appState.busy}
                    onclick={() => toggleInstanceConfig(instance)}
                  >
                    {#if instance.key in appState.expandedConfig}<ChevronDown size={16} aria-hidden="true" />{:else}<ChevronRight size={16} aria-hidden="true" />{/if}
                  </Button>
                {/if}
                <Button variant="destructive" size="icon" title="Unmount" aria-label="Unmount" disabled={appState.busy} onclick={() => requestUnmount(instance)}>
                  <Unplug size={16} aria-hidden="true" />
                </Button>
                <DropdownMenu.Root>
                  <DropdownMenu.Trigger>
                    {#snippet child({ props })}
                      <Button {...props} variant="outline" size="icon" title="More actions" aria-label="More actions" disabled={appState.busy}>
                        <EllipsisVertical size={16} aria-hidden="true" />
                      </Button>
                    {/snippet}
                  </DropdownMenu.Trigger>
                  <DropdownMenu.Portal>
                    <DropdownMenu.Content
                      align="end"
                      sideOffset={6}
                      class="z-50 min-w-[200px] border border-border bg-popover p-1 text-popover-foreground"
                    >
                      <DropdownMenu.Item
                        class="flex cursor-pointer items-center gap-2 px-4 py-2 text-sm outline-none data-disabled:pointer-events-none data-disabled:opacity-50 data-highlighted:bg-accent"
                        disabled={appState.busy || !canOpen(instance)}
                        onSelect={() => runOpen(instance)}
                      >
                        <FolderOpen size={16} aria-hidden="true" /> Open folder
                      </DropdownMenu.Item>
                      {#if canOpen(instance)}
                        <DropdownMenu.Item
                          class="flex cursor-pointer items-center gap-2 px-4 py-2 text-sm outline-none data-highlighted:bg-accent"
                          disabled={appState.busy}
                          onSelect={() => openDashboard(instance, false)}
                        >
                          <SquareTerminal size={16} aria-hidden="true" /> Launch TUI dashboard
                        </DropdownMenu.Item>
                        <DropdownMenu.Item
                          class="flex cursor-pointer items-center gap-2 px-4 py-2 text-sm outline-none data-highlighted:bg-accent"
                          disabled={appState.busy}
                          onSelect={() => openDashboard(instance, true)}
                        >
                          <LayoutDashboard size={16} aria-hidden="true" /> Launch GUI dashboard
                        </DropdownMenu.Item>
                      {/if}
                      <!-- Suppressed entirely on a satellite Deleted/Version/
                           Snapshot row: cloning one would silently produce a
                           profile that mounts the wrong thing (a regular
                           `mount`, not the view command) at that path. -->
                      {#if canOpen(instance) && !viewModeBadge(instance.viewMode)}
                        {#if instance.external}
                          <DropdownMenu.Item
                            class="flex cursor-pointer items-center gap-2 px-4 py-2 text-sm outline-none data-highlighted:bg-accent"
                            disabled={appState.busy}
                            onSelect={() => saveAsProfile(instance)}
                          >
                            <FilePlus size={16} aria-hidden="true" /> Save as profile
                          </DropdownMenu.Item>
                        {:else if instance.profileId}
                          <DropdownMenu.Item
                            class="flex cursor-pointer items-center gap-2 px-4 py-2 text-sm outline-none data-highlighted:bg-accent"
                            disabled={appState.busy}
                            onSelect={() => cloneProfileFor(instance)}
                          >
                            <Copy size={16} aria-hidden="true" /> Clone profile
                          </DropdownMenu.Item>
                        {/if}
                      {/if}
                      {#if canOpenViewsFor(instance)}
                        <DropdownMenu.Separator class="my-1 h-px bg-border" />
                        <DropdownMenu.Item
                          class="flex cursor-pointer items-center gap-2 px-4 py-2 text-sm outline-none data-highlighted:bg-accent"
                          disabled={appState.busy}
                          onSelect={() => requestDeletedView(profileForInstance(instance))}
                        >
                          <Recycle size={16} aria-hidden="true" /> Open deleted-files view
                        </DropdownMenu.Item>
                        <DropdownMenu.Item
                          class="flex cursor-pointer items-center gap-2 px-4 py-2 text-sm outline-none data-highlighted:bg-accent"
                          disabled={appState.busy}
                          onSelect={() => requestVersionView(profileForInstance(instance))}
                        >
                          <History size={16} aria-hidden="true" /> Open file-version view
                        </DropdownMenu.Item>
                      {/if}
                      {#if gatewayInfoForInstance(instance)?.pid}
                        <DropdownMenu.Separator class="my-1 h-px bg-border" />
                        <DropdownMenu.Item
                          class="flex cursor-pointer items-center gap-2 px-4 py-2 text-sm text-destructive outline-none dropdown-item-destructive"
                          disabled={appState.busy}
                          onSelect={() => stopGatewayLaunch(gatewayInfoForInstance(instance)?.id ?? '')}
                        >
                          <OctagonX size={16} aria-hidden="true" /> Stop gateway
                        </DropdownMenu.Item>
                      {/if}
                    </DropdownMenu.Content>
                  </DropdownMenu.Portal>
                </DropdownMenu.Root>
              </div>
            </Table.Cell>
          </Table.Row>
          {#if instance.key in appState.expandedConfig}
            <Table.Row>
              <Table.Cell colspan={4} class="bg-muted">
                <InstanceConfigPanel raw={appState.expandedConfig[instance.key]} onCopy={() => copyConfig(instance.key)} />
              </Table.Cell>
            </Table.Row>
          {/if}
        {:else}
          <Table.Row>
            <Table.Cell colspan={4}>
              <div class="tech-grid grid gap-1.5 p-7 text-center">
                <strong>No instances</strong>
                <p>Mount a saved profile, or mount from the CLI; active mounts appear here after refresh.</p>
              </div>
            </Table.Cell>
          </Table.Row>
        {/each}
      {/if}
    </Table.Body>
  </Table.Root>
</section>
<GatewayLaunchesPanel />
