<script lang="ts">
  import { cn } from '$lib/utils.js'
  import { Popover, PopoverTrigger, PopoverContent } from '$lib/components/ui/popover'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import Check from '@lucide/svelte/icons/check'

  type Props = {
    options: { value: string; label: string }[]
    value?: string
    placeholder?: string
    id?: string
    ariaLabelledby?: string
    disabled?: boolean
    class?: string
    onchange?: (value: string) => void
  }

  let {
    options, value = $bindable(''), placeholder,
    id, ariaLabelledby, disabled = false, class: className, onchange,
  }: Props = $props()

  let open = $state(false)
  let listboxEl = $state<HTMLDivElement | null>(null)
  const selectedLabel = $derived(options.find(o => o.value === value)?.label ?? '')
  const selectedIndex = $derived(Math.max(0, options.findIndex(o => o.value === value)))
  const listboxId = $derived(id ? `${id}-listbox` : undefined)

  // Roving-tabindex focus for the listbox keyboard contract. Reset to the selected
  // option each time the popover opens so arrow keys start from the current value.
  let activeIndex = $state(0)
  $effect(() => { if (open) activeIndex = selectedIndex })

  function optionEls() {
    return listboxEl ? Array.from(listboxEl.querySelectorAll<HTMLButtonElement>('[role="option"]')) : []
  }

  function moveActive(next: number) {
    const clamped = Math.max(0, Math.min(next, options.length - 1))
    activeIndex = clamped
    optionEls()[clamped]?.focus()
  }

  function onListboxKeydown(e: KeyboardEvent) {
    switch (e.key) {
      case 'ArrowDown': e.preventDefault(); moveActive(activeIndex + 1); break
      case 'ArrowUp': e.preventDefault(); moveActive(activeIndex - 1); break
      case 'Home': e.preventDefault(); moveActive(0); break
      case 'End': e.preventDefault(); moveActive(options.length - 1); break
    }
  }

  function select(optValue: string) {
    value = optValue
    open = false
    onchange?.(optValue)
  }
</script>

<Popover bind:open>
  <PopoverTrigger {disabled}>
    {#snippet child({ props })}
      <button {...props} {id} type="button"
        aria-labelledby={ariaLabelledby}
        aria-label={ariaLabelledby ? undefined : (placeholder || 'Select option')}
        aria-haspopup="listbox" aria-expanded={open}
        aria-controls={open ? listboxId : undefined}
        class={cn(
          "border-input bg-background dark:bg-input/20 ring-offset-background flex h-9 min-h-[44px] sm:min-h-0 w-full items-center justify-between rounded-sm border px-3 py-1 text-base outline-none transition-[border-color] md:text-sm",
          "focus-visible:border-foreground/40 focus-visible:ring-2 focus-visible:ring-ring",
          "disabled:cursor-not-allowed disabled:opacity-50",
          !value && "text-muted-foreground",
          className
        )}>
        <span class="truncate">{selectedLabel || placeholder || ''}</span>
        <ChevronDown class="h-4 w-4 shrink-0 opacity-50" />
      </button>
    {/snippet}
  </PopoverTrigger>
  <PopoverContent class="w-[--bits-popover-anchor-width] min-w-36 p-1" align="start"
    onOpenAutoFocus={(e) => {
      const opts = optionEls()
      if (opts.length) {
        e.preventDefault()
        activeIndex = selectedIndex
        opts[Math.min(selectedIndex, opts.length - 1)]?.focus()
      }
    }}>
    <div bind:this={listboxEl} id={listboxId} role="listbox"
      aria-labelledby={ariaLabelledby} aria-label={ariaLabelledby ? undefined : (placeholder || 'Select option')}>
      {#each options as opt, i (opt.value)}
        <button type="button"
          role="option"
          aria-selected={opt.value === value}
          tabindex={i === activeIndex ? 0 : -1}
          onkeydown={onListboxKeydown}
          class={cn(
            "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-none cursor-pointer min-h-[44px] sm:min-h-0",
            "hover:bg-accent hover:text-accent-foreground",
            "focus-visible:bg-accent focus-visible:text-accent-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset",
            opt.value === value && "bg-accent/50"
          )}
          onclick={() => select(opt.value)}
        >
          <Check class={cn("h-4 w-4 shrink-0", opt.value === value ? "opacity-100" : "opacity-0")} aria-hidden="true" />
          <span class="truncate">{opt.label}</span>
        </button>
      {/each}
    </div>
  </PopoverContent>
</Popover>
