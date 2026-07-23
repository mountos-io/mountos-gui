// Moves focus to a newly-mounted view's landing element so keyboard/screen-reader
// users get a signal that navigation happened, instead of losing focus to
// document.body when the previous view's trigger unmounts. Pair with
// tabindex="-1" on the target (landing point only, not a permanent tab stop).
export function focusOnMount(node: HTMLElement) {
  // preventScroll: some callers (e.g. goToSettingsSection) scroll to a specific
  // descendant right after mount; a scrolling focus here would fight that.
  node.focus({ preventScroll: true })
}
