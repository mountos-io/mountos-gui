import './app.css'
import { mount } from 'svelte'
import App from './App.svelte'
import TrayPopover from './TrayPopover.svelte'

// WKWebView/WebView2 show a native browser context menu (Inspect Element,
// Look Up, Services, ...) on right-click; a desktop app shell has no use
// for it, so suppress it everywhere.
document.addEventListener('contextmenu', (event) => event.preventDefault())

const isTrayPopover = new URLSearchParams(location.search).has('tray')

const app = mount(isTrayPopover ? TrayPopover : App, {
  target: document.getElementById('app')!,
})

export default app
