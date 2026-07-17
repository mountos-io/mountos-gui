<script lang="ts" module>
  let _counter = 0
</script>

<script lang="ts">
  import Lightbulb from '@lucide/svelte/icons/lightbulb'

  let { text, width = 360 }: { text: string; width?: number } = $props()

  let show = $state(false)
  let pos = $state({ left: '0px', top: '0px', transform: 'translate(-50%, -100%)' })
  let el: HTMLButtonElement | undefined = $state()
  const tipId = `infotip-${++_counter}`

  // Minimal **bold** parser so consumers can mark section headers without
  // pulling in a markdown lib. Splits on `**…**` runs; even indices are
  // plain spans, odd indices are bold. Newlines preserved by the
  // `whitespace-pre-line` class on the container.
  const segments = $derived.by(() => {
    const out: { bold: boolean; value: string }[] = []
    const re = /\*\*([^*]+)\*\*/g
    let last = 0
    let m: RegExpExecArray | null
    while ((m = re.exec(text)) !== null) {
      if (m.index > last) out.push({ bold: false, value: text.slice(last, m.index) })
      out.push({ bold: true, value: m[1] })
      last = m.index + m[0].length
    }
    if (last < text.length) out.push({ bold: false, value: text.slice(last) })
    return out
  })

  function open(e: PointerEvent | FocusEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    const vw = window.innerWidth
    const pad = 12
    const pw = Math.min(width, vw - pad * 2)
    let left = r.left + r.width / 2
    let top = r.top - 8
    let transform = 'translate(-50%, -100%)'
    if (left - pw / 2 < pad) left = pad + pw / 2
    else if (left + pw / 2 > vw - pad) left = vw - pad - pw / 2
    if (top - 120 < pad) { top = r.bottom + 8; transform = 'translate(-50%, 0)' }
    pos = { left: `${left}px`, top: `${top}px`, transform }
    show = true
  }

  function close() { show = false }

  // Dismiss on Escape, scroll, or resize; WCAG 1.4.13 (Content on Hover or Focus).
  $effect(() => {
    if (!show) return
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') close() }
    const onScrollOrResize = () => close()
    window.addEventListener('keydown', onKey)
    window.addEventListener('scroll', onScrollOrResize, true)
    window.addEventListener('resize', onScrollOrResize)
    return () => {
      window.removeEventListener('keydown', onKey)
      window.removeEventListener('scroll', onScrollOrResize, true)
      window.removeEventListener('resize', onScrollOrResize)
    }
  })

  // Portal the tooltip to document.body so `position: fixed` resolves to the
  // viewport even when an ancestor has a transform (e.g. Dialog content).
  function portal(node: HTMLElement) {
    document.body.appendChild(node)
    return {
      destroy() {
        if (node.parentNode) node.parentNode.removeChild(node)
      },
    }
  }
</script>

<button
  type="button"
  bind:this={el}
  class="inline-flex cursor-help bg-transparent border-none px-1 py-2.5 pointer-coarse:p-2 pointer-coarse:-m-2 items-center justify-center focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring rounded-sm"
  aria-label="More info"
  aria-describedby={show ? tipId : undefined}
  onpointerenter={open}
  onpointerleave={close}
  onfocus={open}
  onblur={close}
>
  <Lightbulb class="size-3.5 text-warning" aria-hidden="true" />
  <span class="sr-only">More info</span>
</button>

{#if show}
  <div
    use:portal
    id={tipId}
    role="tooltip"
    class="fixed z-50 pointer-events-none rounded-sm border border-border bg-card px-3 py-2"
    style:left={pos.left}
    style:top={pos.top}
    style:transform={pos.transform}
    style:max-width="min({width}px, calc(100vw - 1.5rem))"
  >
    <p class="text-sm leading-relaxed text-foreground whitespace-pre-line">{#each segments as seg}{#if seg.bold}<strong class="font-semibold text-foreground">{seg.value}</strong>{:else}{seg.value}{/if}{/each}</p>
  </div>
{/if}
