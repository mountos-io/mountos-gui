import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vitest/config'

const host = process.env.TAURI_DEV_HOST

export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  // Matches SvelteKit's own $lib alias (already declared in tsconfig.json's
  // "paths") so components copied from mountos-admin-client (a SvelteKit
  // app) resolve their `$lib/...` imports here without editing every import.
  resolve: {
    alias: {
      $lib: path.resolve(__dirname, './src/lib'),
    },
  },
  clearScreen: false,
  server: {
    host: host || '127.0.0.1',
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
    hmr: {
      overlay: false,
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
  },
})
