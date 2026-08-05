/**
 * The initialization contract.
 *
 * The builds are `--target web` (scripts/build-wasm.sh), so the classes export
 * synchronously and the instance behind them arrives via `init`. This suite
 * pins both halves of that: reaching the surface early fails loudly and
 * legibly, and the gate itself is idempotent, concurrency-safe, and identical
 * in every environment.
 *
 * Its own file because the assertions are init-order-sensitive: the "before
 * init" cases need a module registry where nothing has initialized yet, which
 * vitest's per-file isolation provides and a shared suite would not.
 *
 * Aliased to pkg/runtime/runtime.js in vitest.config.js.
 */
import { describe, it, expect } from 'vitest'
import { execFileSync } from 'node:child_process'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { dirname, join } from 'node:path'
import {
  init,
  Quill,
  Document,
  importMarkdown,
  isQuillmarkError,
} from '@quillmark-wasm/runtime'

const PKG_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', 'pkg')

/** A parseable document: the root card-yaml block every document opens with. */
const MAIN_CARD_DOC = `~~~
$quill: test_quill
$kind: main
title: Init
~~~

# Hello`

/** Invoke `fn`, expect a throw, and return its primary diagnostic. */
const diagnosticFrom = (fn) => {
  let thrown
  try {
    fn()
  } catch (err) {
    thrown = err
  }
  expect(thrown, 'expected a throw, got none').toBeDefined()
  expect(isQuillmarkError(thrown)).toBe(true)
  return thrown.diagnostics[0]
}

/** Await `promise`, expect a rejection, and return its primary diagnostic. */
const rejectionFrom = async (promise) => {
  let thrown
  try {
    await promise
  } catch (err) {
    thrown = err
  }
  expect(thrown, 'expected a rejection, got none').toBeDefined()
  expect(isQuillmarkError(thrown)).toBe(true)
  return thrown.diagnostics[0]
}

// Reaching a build before `init` resolves is the one mistake the contract
// invites, so it is the one that must not surface as a generated-code
// `TypeError`. Every generated path reads the same module-level binding, so
// every generated path is covered: the constructor case is the reason the guard
// lives there rather than on the prototypes (a public constructor cannot be
// guarded without wrapping the class, which the canonical invariant forbids).
describe('before init', () => {
  it('rejects a static with a named diagnostic', () => {
    const d = diagnosticFrom(() => Quill.fromTree(new Map()))
    expect(d.code).toBe('runtime::not_initialized')
    expect(d.message).toMatch(/await init\(\)/)
    expect(d.hint).toMatch(/@quillmark\/wasm/)
  })

  it('rejects a constructor', () => {
    expect(diagnosticFrom(() => new Document('x')).code).toBe('runtime::not_initialized')
  })

  it('rejects a free function', () => {
    expect(diagnosticFrom(() => importMarkdown('x')).code).toBe('runtime::not_initialized')
  })
})

describe('init', () => {
  // Identity, not just equivalence: the memo is RETURNED, which is why `init`
  // is not declared `async` (an `async` body wraps a fresh promise per call).
  it('memoizes the promise, so concurrent callers share one instantiation', async () => {
    const a = init()
    const b = init()
    expect(a).toBe(b)
    await expect(Promise.all([a, b, init()])).resolves.toEqual([
      undefined,
      undefined,
      undefined,
    ])
  })

  it('unlocks the sync surface', () => {
    expect(importMarkdown('# Hi')).toBeDefined()
    expect(Document.fromMarkdown(MAIN_CARD_DOC)).toBeDefined()
  })

  // Silently ignoring a second, different source would leave a consumer
  // believing they chose the binary they are running.
  it('refuses a different source once initialized', async () => {
    const d = await rejectionFrom(init(new Uint8Array(8)))
    expect(d.code).toBe('runtime::init_conflict')
  })

  // The rule (delivery follows the function kind) at the one export that could
  // break it: `Promise<void>` cannot declare a synchronous throw, so a throw
  // here would escape the `init(BYTES).catch(…)` the declaration invites.
  it('delivers the conflict as a rejection, not a synchronous throw', () => {
    /** @type {Promise<void> | undefined} */
    let returned
    expect(() => {
      returned = init(new Uint8Array(8))
    }).not.toThrow()
    expect(returned).toBeInstanceOf(Promise)
    returned.catch(() => {})
  })
})

// The package imports and runs under plain Node, with no bundler, no plugin,
// and no aliases. Nothing inside vitest can prove it, since vite resolves
// `#quillmark-env` and the asset URL itself, so this shells out to a bare node.
describe('plain Node, no bundler', () => {
  it('imports, initializes with no arguments, and works', () => {
    const runtimeUrl = pathToFileURL(join(PKG_DIR, 'runtime', 'runtime.js')).href
    const script = `
      const { init, importMarkdown } = await import(${JSON.stringify(runtimeUrl)});
      await init();
      if (!importMarkdown('# Hello')) throw new Error('no content');
      console.log('OK');
    `
    const out = execFileSync(process.execPath, ['--input-type=module', '-e', script], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    })
    expect(out.trim()).toBe('OK')
  })

  it('reports a bad source as init_failed, and recovers on retry', () => {
    const runtimeUrl = pathToFileURL(join(PKG_DIR, 'runtime', 'runtime.js')).href
    const script = `
      const { init, importMarkdown } = await import(${JSON.stringify(runtimeUrl)});
      let code;
      try { await init(new Uint8Array([0, 1, 2, 3])); }
      catch (e) { code = e.diagnostics?.[0]?.code; }
      if (code !== 'runtime::init_failed') throw new Error('got ' + code);
      // Self-heal: the failed attempt must not poison the retry.
      await init();
      if (!importMarkdown('# Hello')) throw new Error('no content');
      console.log('OK');
    `
    const out = execFileSync(process.execPath, ['--input-type=module', '-e', script], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    })
    expect(out.trim()).toBe('OK')
  })
})
