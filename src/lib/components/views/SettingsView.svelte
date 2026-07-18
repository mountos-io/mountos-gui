<script lang="ts">
  import { AlertTriangle, Bot, FileArchive, FolderOpen, Monitor, Moon, RefreshCw, ScrollText, ShieldCheck, Sun } from '@lucide/svelte'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Select } from '$lib/components/ui/select'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import { Badge } from '$lib/components/ui/badge'
  import { Separator } from '$lib/components/ui/separator'
  import Callout from '$lib/components/Callout.svelte'
  import CommandPreview from '$lib/components/CommandPreview.svelte'
  import InfoTip from '$lib/components/shared/InfoTip.svelte'
  import { themeState, setTheme, setSkin, setFontSize, setGrayscale, setBrightness } from '$lib/theme.svelte'
  import type { Theme, FontSize } from '$lib/theme.svelte'
  import { presetsForMode, defaultSkin } from '$lib/themes'
  import type { Backend } from '$lib/types'
  import {
    appState,
    changeAllowForkForceDelete,
    changeCliPathOverride,
    changeDefaultBackend,
    changeDefaultDiscoveryUrl,
    changePollSeconds,
    changeTerminal,
    checkMcpStatus,
    computed,
    createBundle,
    DEFAULT_POLL_SECONDS,
    installMcp,
    openBundle,
    POLL_CHOICES,
    refresh,
    setSkipUnmountConfirm,
    showLicenses,
    uninstallMcp,
  } from '$lib/app-state.svelte'

  const themeOptions: Array<{ value: Theme; label: string; icon: typeof Sun }> = [
    { value: 'light', label: 'Light', icon: Sun },
    { value: 'dark', label: 'Dark', icon: Moon },
    { value: 'system', label: 'System', icon: Monitor },
  ]

  const fontSizeOptions: Array<{ value: FontSize; label: string }> = [
    { value: 'standard', label: 'Standard' },
    { value: 'medium', label: 'Medium' },
    { value: 'large', label: 'Large' },
    { value: 'extra-large', label: 'Extra Large' },
    { value: 'jumbo', label: 'Jumbo' },
  ]

  const backendOptions = $derived(computed.backends.map((backend) => ({ value: backend, label: backend })))
  const terminalOptions = $derived([
    { value: '', label: 'System default' },
    ...appState.systemState.terminals.map((option) => ({ value: option.id, label: option.label })),
  ])
  const pollOptions = $derived(
    POLL_CHOICES.map((seconds) => ({
      value: String(seconds),
      label: seconds === 0 ? 'Off' : `${seconds}s${seconds === DEFAULT_POLL_SECONDS ? ' (default)' : ''}`,
    })),
  )

  // Filtered by the live resolved mode (not just an explicit Light/Dark
  // pick) so the picker still works under "System" -- and stays correct if
  // the OS appearance flips while this view is open.
  const skinPresets = $derived(presetsForMode(themeState.resolvedMode))
  const defaultSkinName = $derived(defaultSkin(themeState.resolvedMode))
</script>

<section class="corner-brackets surface m-[22px] p-4 grid gap-5">
  <h3>Desktop policies</h3>

  <div class="grid gap-3" id="settings-appearance">
    <span class="mono-label">Appearance</span>
    <div class="flex items-center justify-between gap-4">
      <span class="inline-flex items-center gap-1"><strong>Theme</strong><InfoTip text="Follows the system appearance until you pick Light or Dark." /></span>
      <div class="flex gap-1.5" role="group" aria-label="Theme">
        {#each themeOptions as option (option.value)}
          <Button type="button" size="sm" variant={themeState.theme === option.value ? 'primary' : 'outline'} aria-pressed={themeState.theme === option.value} onclick={() => setTheme(option.value)}>
            <option.icon size={15} aria-hidden="true" />
            {option.label}
          </Button>
        {/each}
      </div>
    </div>
    <div class="grid gap-1.5">
      <span class="inline-flex items-center gap-1"><strong>Skin</strong><InfoTip text="Named color palette for the current mode." /></span>
      <div class="flex flex-wrap gap-2" role="group" aria-label="Skin">
        {#each skinPresets as preset (preset.name)}
          {@const isDefault = preset.name === defaultSkinName}
          {@const active = isDefault ? !themeState.skin || themeState.skin === defaultSkinName : themeState.skin === preset.name}
          <button
            type="button"
            class="skin-swatch {active ? 'ring-2 ring-primary' : ''}"
            style="--sw-bg: {preset.colors.background}; --sw-fg: {preset.colors.primary};"
            onclick={() => setSkin(isDefault ? '' : preset.name)}
            aria-pressed={active}
            title={isDefault ? 'mountOS (default)' : preset.name}
          >
            <span class="sw-dot"></span>
            <span class="sw-label">{isDefault ? 'mountOS' : preset.name.replace(/ (Light|Dark)$/, '')}</span>
          </button>
        {/each}
      </div>
    </div>
    <div class="grid gap-1.5">
      <span class="inline-flex items-center gap-1"><strong>Font size</strong><InfoTip text="Scales all text in the app." /></span>
      <div class="flex flex-wrap gap-1.5" role="group" aria-label="Font size">
        {#each fontSizeOptions as option (option.value)}
          <Button type="button" size="sm" variant={themeState.fontSize === option.value ? 'primary' : 'outline'} aria-pressed={themeState.fontSize === option.value} onclick={() => setFontSize(option.value)}>
            {option.label}
          </Button>
        {/each}
      </div>
    </div>
    <div class="flex items-center justify-between gap-4">
      <span class="inline-flex items-center gap-1"><strong>Grayscale</strong><InfoTip text="Reduces color for low-light comfort." /></span>
      <Checkbox checked={themeState.grayscale} onchange={(e) => setGrayscale(e.currentTarget.checked)} />
    </div>
    <div class="grid gap-1.5">
      <div class="flex items-center justify-between gap-4">
        <span class="inline-flex items-center gap-1"><strong>Brightness</strong><InfoTip text="Dims or brightens the whole app." /></span>
        <span class="mono-label">{themeState.brightness}%</span>
      </div>
      <div class="flex items-center gap-2">
        <input
          type="range"
          min="50"
          max="150"
          step="5"
          value={themeState.brightness}
          oninput={(e) => setBrightness(Number(e.currentTarget.value))}
          aria-label="Brightness"
          class="w-full accent-primary"
        />
        {#if themeState.brightness !== 100}
          <Button type="button" size="sm" variant="ghost" onclick={() => setBrightness(100)}>Reset</Button>
        {/if}
      </div>
    </div>
  </div>

  <Separator />

  <div class="grid gap-3">
    <span class="mono-label">Mounting defaults</span>
    <div class="flex items-center justify-between gap-4">
      <span class="inline-flex items-center gap-1"><strong>Default backend</strong><InfoTip text="Used for new profiles; Auto follows the CLI's platform order." /></span>
      <Select options={backendOptions} value={appState.settings.defaultBackend} onchange={(value) => changeDefaultBackend(value as Backend)} class="w-48" />
    </div>
    <div class="grid gap-1.5">
      <span class="inline-flex items-center gap-1"><strong>Default discovery URL</strong><InfoTip text="Seeds new profiles only; existing ones stay unchanged." /></span>
      <Input type="text" placeholder="https://hub.example.com" value={appState.settings.defaultDiscoveryUrl ?? ''} onchange={(e) => changeDefaultDiscoveryUrl(e.currentTarget.value)} />
    </div>
  </div>

  <Separator />

  <div class="grid gap-3">
    <span class="mono-label">Monitoring &amp; dashboard</span>
    <div class="flex items-center justify-between gap-4">
      <span class="inline-flex items-center gap-1"><strong>Refresh interval</strong><InfoTip text="How often mounts refresh. Off disables auto-refresh; use the Refresh button instead." /></span>
      <Select options={pollOptions} value={String(appState.settings.pollSeconds ?? DEFAULT_POLL_SECONDS)} onchange={(value) => changePollSeconds(Number(value))} class="w-48" />
    </div>
    <div class="flex items-center justify-between gap-4" id="settings-terminal">
      <span class="inline-flex items-center gap-1"><strong>Terminal</strong><InfoTip text="Where the dashboard opens. Falls back to the system default if uninstalled." /></span>
      <Select options={terminalOptions} value={appState.settings.terminal ?? ''} onchange={(value) => changeTerminal(value)} class="w-48" />
    </div>
  </div>

  <Separator />

  <div class="grid gap-3">
    <span class="mono-label">Actions</span>
    <div class="flex items-center justify-between gap-4">
      <span class="inline-flex items-center gap-1">
        <strong>Skip unmount confirmation</strong>
        <InfoTip text="Skips the confirmation dialog on Unmount and Unmount all." />
        <Badge variant="warning"><AlertTriangle size={12} aria-hidden="true" />Not recommended</Badge>
      </span>
      <Checkbox checked={appState.skipUnmountConfirm} onchange={(e) => setSkipUnmountConfirm(e.currentTarget.checked)} />
    </div>
    <div class="flex items-center justify-between gap-4">
      <span class="inline-flex items-center gap-1"><strong>Allow force fork delete</strong><InfoTip text="Adds --force to fork delete, removing the whole subtree from the shared volume." /></span>
      <Checkbox checked={appState.settings.allowForkForceDelete} onchange={(e) => changeAllowForkForceDelete(e.currentTarget.checked)} />
    </div>
    {#if appState.settings.allowForkForceDelete}
      <Callout>--force fork delete acts on the shared volume, not just this profile, and also removes the fork's entire subtree. Deleting a fork is recoverable only within its grace period.</Callout>
    {/if}
  </div>
</section>

<section class="surface m-[22px] p-4 grid gap-4">
  <h3>About mountOS</h3>
  <div class="flex items-center justify-between gap-4">
    <span><strong>Platform</strong></span>
    <span class="mono-label">{appState.systemState.platform}</span>
  </div>
  <div class="flex items-center justify-between gap-4">
    <span><strong>CLI version</strong></span>
    <span class="mono-label">{appState.systemState.cliVersion ?? 'unavailable'}</span>
  </div>
  <div class="flex items-center justify-between gap-4">
    <span><strong>CLI path</strong></span>
    <code>{appState.systemState.cliPath ?? 'not found on PATH'}</code>
  </div>

  {#if appState.systemState.cliPathAlternates.length}
    <Callout>
      {appState.systemState.cliPathAlternates.length} other mountos {appState.systemState.cliPathAlternates.length === 1 ? 'binary was' : 'binaries were'} found on PATH and ignored:
      {appState.systemState.cliPathAlternates.join(', ')}. Pin the one you want below to stop relying on PATH order.
    </Callout>
  {/if}

  <div class="grid gap-1.5">
    <span class="inline-flex items-center gap-1"><strong>Pin CLI path</strong><InfoTip text="Overrides PATH lookup with this exact binary; empty uses PATH." /></span>
    <Input type="text" placeholder={appState.systemState.cliPath ?? '/usr/local/bin/mountos'} value={appState.settings.cliPathOverride ?? ''} onchange={(e) => changeCliPathOverride(e.currentTarget.value)} />
  </div>
  <div class="flex items-center justify-between gap-4">
    <span><strong>Third party licenses</strong></span>
    <Button type="button" variant="outline" onclick={showLicenses}>
      <ScrollText size={16} aria-hidden="true" />
      View
    </Button>
  </div>
</section>

<section class="surface m-[22px] p-4 grid gap-4" id="settings-mcp">
  <div class="flex items-start justify-between gap-4">
    <h3 class="flex items-center gap-2"><Bot size={19} aria-hidden="true" /> MCP for AI agents</h3>
    <div class="flex flex-wrap items-center gap-2">
      <Button type="button" onclick={checkMcpStatus} disabled={appState.busy}>
        <RefreshCw size={16} aria-hidden="true" />
        Check status
      </Button>
      <Button type="button" onclick={installMcp} disabled={appState.busy}>Install</Button>
      <Button type="button" variant="destructive" onclick={uninstallMcp} disabled={appState.busy}>Uninstall</Button>
    </div>
  </div>
  <p>Registers this mountos binary as a read-only Model Context Protocol server for Claude Desktop, Claude Code, Codex and Gemini, so an AI agent can inspect mounts, stats and diagnostics without file access.</p>
  {#if appState.mcpStatusText}
    <CommandPreview label="MCP STATUS">
      <pre class="m-0 whitespace-pre-wrap break-words"><code>{appState.mcpStatusText}</code></pre>
    </CommandPreview>
  {/if}
</section>

<section class="surface m-[22px] p-4 grid gap-4">
  <div class="flex items-start justify-between gap-4">
    <h3 class="flex items-center gap-2"><ShieldCheck size={19} aria-hidden="true" /> Diagnostics</h3>
    <div class="flex flex-wrap items-center gap-2">
      <Badge variant={appState.systemState.checkOk ? 'success' : 'warning'}>{appState.systemState.checkOk ? 'Ready' : 'Needs attention'}</Badge>
      <Button type="button" onclick={() => refresh()} disabled={appState.busy} title="Re-run mountos check --json">
        <RefreshCw size={16} aria-hidden="true" />
        Run check
      </Button>
    </div>
  </div>

  <!-- Setup problems are the actionable half of the readiness check, so they
       stay on screen. The rest of the old Health page was a dump of what the
       bundle already contains. -->
  {#if appState.systemState.issues.length}
    <div class="grid gap-3">
      {#each appState.systemState.issues as issue}
        <article class="flex items-start gap-3">
          <AlertTriangle size={18} class={issue.severity === 'error' ? 'text-destructive' : issue.severity === 'warning' ? 'text-warning' : 'text-muted-foreground'} aria-hidden="true" />
          <div>
            <strong>{issue.title}</strong>
            {#if issue.detail}<p>{issue.detail}</p>{/if}
            {#if issue.fixCommand}<code>{issue.fixCommand}</code>{/if}
          </div>
        </article>
      {/each}
    </div>
  {/if}

  <div class="flex items-center justify-between gap-4" id="settings-diagnostics-bundle">
    <span class="inline-flex items-center gap-1"><strong>Diagnostics bundle</strong><InfoTip text="Writes a JSON file with CLI info, check/list output, and saved profiles." /></span>
    <Button type="button" onclick={createBundle} disabled={appState.busy}>
      <FileArchive size={16} aria-hidden="true" />
      Create
    </Button>
  </div>
  {#if appState.diagnosticsBundle}
    <div class="grid gap-1.5">
      <span class="mono-label">BUNDLE</span>
      <div class="flex items-center justify-between gap-2.5">
        <code class="break-all">{appState.diagnosticsBundle.path}</code>
        <Button type="button" onclick={openBundle} disabled={appState.busy} class="shrink-0">
          <FolderOpen size={16} aria-hidden="true" />
          Open
        </Button>
      </div>
    </div>
  {/if}
</section>

<style>
  .skin-swatch {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--sw-bg);
    cursor: pointer;
    transition: transform 0.15s;
  }

  .skin-swatch:hover {
    transform: scale(1.05);
  }

  .skin-swatch:focus-visible {
    outline: 2px solid var(--ring);
    outline-offset: 2px;
  }

  .sw-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--sw-fg);
    border: 1px solid color-mix(in oklch, var(--sw-fg) 60%, transparent);
  }

  .sw-label {
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--sw-fg);
    white-space: nowrap;
  }
</style>
