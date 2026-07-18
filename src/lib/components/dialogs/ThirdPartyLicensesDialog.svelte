<script lang="ts">
  import ScrollText from '@lucide/svelte/icons/scroll-text'
  import Search from '@lucide/svelte/icons/search'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Badge } from '$lib/components/ui/badge'
  import { appState, hideLicenses, setLicensesKind } from '$lib/app-state.svelte'
  import type { LicenseGroup } from '$lib/types'

  let query = $state('')

  const active = $derived(appState.licensesData[appState.licensesKind])
  const filtered = $derived.by(() => {
    const licenses = active?.licenses ?? []
    const q = query.trim().toLowerCase()
    if (!q) return licenses
    return licenses.filter(
      (license: LicenseGroup) =>
        license.id.toLowerCase().includes(q) ||
        license.name.toLowerCase().includes(q) ||
        license.packages.some((pkg) => pkg.name.toLowerCase().includes(q)),
    )
  })
</script>

<Dialog.Root bind:open={() => appState.licensesOpen, (open) => { if (!open) hideLicenses() }}>
  <Dialog.Content class="sm:max-w-3xl" aria-describedby={undefined}>
    <Dialog.Header>
      <Dialog.Title class="flex items-center gap-2"><ScrollText size={20} aria-hidden="true" /> Third Party Licenses</Dialog.Title>
    </Dialog.Header>

    <div class="grid gap-3">
      <div class="flex items-center gap-2">
        <Button type="button" size="sm" variant={appState.licensesKind === 'rust' ? 'default' : 'outline'} onclick={() => setLicensesKind('rust')}>Rust</Button>
        <Button type="button" size="sm" variant={appState.licensesKind === 'js' ? 'default' : 'outline'} onclick={() => setLicensesKind('js')}>JavaScript</Button>
      </div>
      <div class="relative">
        <Search size={15} aria-hidden="true" class="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
        <span class="sr-only">Search licenses</span>
        <Input bind:value={query} placeholder="Search by package or license" class="pl-8" />
      </div>

      <div class="grid gap-2 max-h-[55vh] overflow-auto">
        {#if appState.licensesLoading}
          <p class="text-sm text-muted-foreground">Loading licenses...</p>
        {:else if appState.licensesError}
          <p class="text-sm text-destructive">{appState.licensesError}</p>
        {:else if filtered.length === 0}
          <p class="text-sm text-muted-foreground">{active?.licenses.length ? `No licenses match "${query}".` : 'No license data available.'}</p>
        {:else}
          {#each filtered as license (`${license.id}|${license.name}|${license.packages.map((p) => p.name).join(',')}`)}
            <details class="border border-border p-2">
              <summary class="cursor-pointer flex items-center gap-2">
                <Badge variant="outline">{license.id}</Badge>
                <span class="text-sm">{license.packages.map((pkg) => pkg.name).join(', ')}</span>
              </summary>
              <div class="grid gap-2 pt-2">
                <ul class="text-xs text-muted-foreground grid gap-0.5">
                  {#each license.packages as pkg (pkg.name + pkg.version)}
                    <li>
                      {#if pkg.repository}
                        <a href={pkg.repository} target="_blank" rel="noopener noreferrer" class="hover:underline">{pkg.name}@{pkg.version}</a>
                      {:else}
                        {pkg.name}@{pkg.version}
                      {/if}
                    </li>
                  {/each}
                </ul>
                <pre class="m-0 max-h-64 overflow-auto whitespace-pre-wrap break-words border border-border bg-muted p-2 text-xs"><code>{license.text}</code></pre>
              </div>
            </details>
          {/each}
        {/if}
      </div>
    </div>

    <Dialog.Footer>
      <Button type="button" variant="outline" onclick={hideLicenses}>Close</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
