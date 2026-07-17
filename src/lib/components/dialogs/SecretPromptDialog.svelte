<script lang="ts">
  import { KeyRound } from '@lucide/svelte'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import Callout from '$lib/components/Callout.svelte'
  import { appState, cancelSecret, computed, doMount } from '$lib/app-state.svelte'
</script>

<Dialog.Root bind:open={() => appState.secretPromptFor !== null, (open) => { if (!open) cancelSecret() }}>
  <Dialog.Content class="sm:max-w-md" aria-describedby={undefined}>
    {#if appState.secretPromptFor}
      <form onsubmit={(event) => { event.preventDefault(); void doMount(appState.secretPromptFor!, computed.trimmedSecret) }}>
        <Dialog.Header>
          <Dialog.Title class="flex items-center gap-2"><KeyRound size={20} aria-hidden="true" /> Enter secret access key</Dialog.Title>
        </Dialog.Header>
        <!-- No prose here on purpose: how the secret reaches the CLI (stdin, never
             argv or env) is implementation the operator cannot act on, and this
             dialog appears on every prompted mount. The checkbox below is the only
             decision, and it states what happens to the value. -->
        <div class="grid gap-1.5 py-4">
          <span class="sr-only" id="secret-value-label">Secret access key</span>
          <!-- svelte-ignore a11y_autofocus -->
          <Input
            type="password"
            bind:value={appState.secretValue}
            autocomplete="current-password"
            autofocus
            aria-labelledby="secret-value-label"
            aria-invalid={Boolean(appState.secretValue) && Boolean(computed.secretLengthError)}
          />
          <!-- Only once they have typed: the length rule is not news on an empty
               field, but a disabled Mount with no stated reason is. -->
          {#if appState.secretValue && computed.secretLengthError}
            <small class="text-destructive text-sm">{computed.secretLengthError}</small>
          {/if}
        </div>
        <Checkbox bind:checked={appState.savePromptedSecret} label="Store in OS vault for this profile" />
        {#if appState.secretError}
          <Callout role="alert">{appState.secretError}</Callout>
        {/if}
        <Dialog.Footer class="mt-4">
          <Button type="button" variant="outline" class="cyberpunk-skewed-sm" onclick={cancelSecret}>Cancel</Button>
          <Button type="submit" variant="primary" class="cyberpunk-skewed-sm" disabled={appState.busy || !appState.secretValue}>Mount</Button>
        </Dialog.Footer>
      </form>
    {/if}
  </Dialog.Content>
</Dialog.Root>
