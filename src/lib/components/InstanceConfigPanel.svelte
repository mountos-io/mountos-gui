<script lang="ts">
  import Copy from '@lucide/svelte/icons/copy'
  import { Button } from '$lib/components/ui/button'
  import ConfigSection from './ConfigSection.svelte'

  let { raw, onCopy }: { raw: string; onCopy: () => void } = $props()

  let mode = $state<'parsed' | 'raw'>('parsed')

  const parsed = $derived.by(() => {
    try {
      return JSON.parse(raw) as Record<string, unknown>
    } catch {
      return null
    }
  })
</script>

<div class="relative overflow-auto border border-border bg-card">
  <div class="tech-grid absolute inset-0 pointer-events-none" aria-hidden="true"></div>
  <div class="relative grid gap-2 p-2.5">
    <div class="flex items-center justify-between gap-2.5">
      <p class="mono-label">MOUNT FLAGS (.mountOS/.config)</p>
      <div class="flex items-center gap-1" role="group" aria-label="Config view">
        <Button type="button" size="sm" variant={mode === 'parsed' && parsed ? 'primary' : 'outline'} aria-pressed={mode === 'parsed' && Boolean(parsed)} onclick={() => (mode = 'parsed')} disabled={!parsed}>Parsed</Button>
        <Button type="button" size="sm" variant={mode === 'raw' ? 'primary' : 'outline'} aria-pressed={mode === 'raw'} onclick={() => (mode = 'raw')}>Raw</Button>
        <Button variant="outline" size="icon" title="Copy mount flags" aria-label="Copy mount flags" onclick={onCopy}>
          <Copy size={16} aria-hidden="true" />
        </Button>
      </div>
    </div>
    {#if mode === 'raw' || !parsed}
      <pre class="m-0 whitespace-pre-wrap break-words"><code>{raw}</code></pre>
    {:else}
      <ConfigSection data={parsed} />
    {/if}
  </div>
</div>
