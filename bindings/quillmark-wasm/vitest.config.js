import { defineConfig } from 'vitest/config'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'
import path from 'path'
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

// Centralized workspace root and bundle path
export const WORKSPACE_ROOT = path.resolve(__dirname, '..', '..')
export const WASM_BUNDLE_PATH = path.join(WORKSPACE_ROOT, 'pkg', 'bundler', 'wasm.js')

export default defineConfig({
  plugins: [wasm(), topLevelAwait()],
  resolve: {
    alias: {
      '@quillmark-wasm': WASM_BUNDLE_PATH,
    },
  },
  test: {
    environment: 'node',
    testTimeout: 40000,
  },
})
