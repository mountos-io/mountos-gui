<script lang="ts">
  import { FileArchive, HardDrive, MonitorDot, Plus, RefreshCw, Settings, Unplug } from '@lucide/svelte'
  import * as Command from '$lib/components/ui/command'
  import { appState, createBundle, newProfile, refresh, requestUnmountAll, selectProfile } from '$lib/app-state.svelte'
  import type { View } from '$lib/app-state.svelte'

  let { open = $bindable(false) }: { open?: boolean } = $props()

  const navItems: Array<{ id: View; label: string; icon: typeof MonitorDot }> = [
    { id: 'instances', label: 'Instances', icon: MonitorDot },
    { id: 'profiles', label: 'Profiles', icon: HardDrive },
    { id: 'settings', label: 'Settings', icon: Settings },
  ]

  function run(action: () => void) {
    open = false
    action()
  }
</script>

<Command.CommandDialog bind:open>
  <Command.CommandInput placeholder="Type a command or search..." />
  <Command.CommandList>
    <Command.CommandEmpty>No results found.</Command.CommandEmpty>

    <Command.CommandGroup heading="Navigation">
      {#each navItems as item (item.id)}
        <Command.CommandItem value={item.label} onSelect={() => run(() => (appState.view = item.id))}>
          <item.icon class="mr-2 h-4 w-4" />
          {item.label}
        </Command.CommandItem>
      {/each}
    </Command.CommandGroup>

    {#if appState.profiles.length > 0}
      <Command.CommandSeparator />
      <Command.CommandGroup heading="Profiles">
        {#each appState.profiles as profile (profile.id)}
          <Command.CommandItem
            value="profile {profile.name}"
            onSelect={() => run(() => { selectProfile(profile); appState.view = 'profiles' })}
          >
            <HardDrive class="mr-2 h-4 w-4" />
            {profile.name}
          </Command.CommandItem>
        {/each}
      </Command.CommandGroup>
    {/if}

    <Command.CommandSeparator />

    <Command.CommandGroup heading="Actions">
      <Command.CommandItem value="Refresh" onSelect={() => run(() => refresh())}>
        <RefreshCw class="mr-2 h-4 w-4" />
        Refresh
      </Command.CommandItem>
      <Command.CommandItem value="New profile" onSelect={() => run(() => newProfile())}>
        <Plus class="mr-2 h-4 w-4" />
        New profile
      </Command.CommandItem>
      <Command.CommandItem
        value="Unmount all"
        disabled={appState.systemState.instances.length === 0}
        onSelect={() => run(requestUnmountAll)}
      >
        <Unplug class="mr-2 h-4 w-4" />
        Unmount all
      </Command.CommandItem>
      <Command.CommandItem value="Create diagnostics bundle" onSelect={() => run(() => createBundle())}>
        <FileArchive class="mr-2 h-4 w-4" />
        Create diagnostics bundle
      </Command.CommandItem>
    </Command.CommandGroup>
  </Command.CommandList>
</Command.CommandDialog>
