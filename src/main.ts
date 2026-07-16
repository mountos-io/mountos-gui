import './app.css'
import { mount } from 'svelte'
import App from './App.svelte'
import TrayPopover from './TrayPopover.svelte'

const isTrayPopover = new URLSearchParams(location.search).has('tray')

const app = mount(isTrayPopover ? TrayPopover : App, {
  target: document.getElementById('app')!,
})

export default app
