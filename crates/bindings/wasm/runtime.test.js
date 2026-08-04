/**
 * Canonical-API (`@quillmark/wasm/runtime`) integration tests.
 *
 * The runtime layer re-exports the core build's `Quill`/`Document` and adds an
 * `Engine` that hides the core→backend WASM-memory crossing. These tests prove,
 * end to end, that a CORE quill + document handed to `Engine` render correctly;
 * i.e. the engine clones them into the Typst backend's memory on demand
 * (`toTree`→`fromTree`, `toJson`→`fromJson`) without the caller ever seeing a
 * backend handle.
 *
 * Aliased to pkg/runtime/runtime.js in vitest.config.js.
 */
import { describe, it, expect, beforeAll } from 'vitest'
import fs from 'node:fs'
import path from 'node:path'
import {
  Quill,
  Document,
  Engine,
  DocumentWriter,
  CardWriter,
  DocumentReader,
  CardReader,
  MAIN_CARD_ADDR,
  isQuillmarkError,
  exportMarkdown,
  isUnknownLine,
  isUnknownContainer,
  isUnknownMark,
  isUnknownIsland,
} from '@quillmark-wasm/runtime'
// Pin that the runtime's Quill IS the internal core build's class (re-export,
// not a parallel wrapper). This imports the internal core artifact directly:
// `pkg/core` is NOT a public package subpath, it is the build the root
// re-exports.
import { Quill as CoreQuill, Document as CoreDocument } from '../../../pkg/core/wasm.js'
import {
  makeQuill,
  makeSampleFormQuill,
  SAMPLE_FORM_MARKDOWN,
  expectEditCode,
} from './test-helpers.js'

const TEST_PLATE = `#import "@local/quillmark-helper:0.1.0": data
#let title = data.title
#let body = data.at("$body")

= #title

#body`

const TEST_MARKDOWN = `~~~card-yaml
$quill: test_quill
$kind: main
title: Test Document
author: Test Author
~~~

# Hello World

This is a test document.`

function makeRuntimeQuill() {
  return Quill.fromTree(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
}

const PKG_DIR = path.resolve(import.meta.dirname, '..', '..', '..', 'pkg')

/** Read a field value from a card's payloadItems list by key. */
const fieldOf = (card, key) =>
  card.payloadItems.find((i) => i.type === 'field' && i.key === key)?.value

describe('@quillmark/wasm/runtime: surface', () => {
  // IMPLEMENTATION PIN: the root re-exports the internal core build's classes
  // verbatim (never wraps). There is exactly one public entry point, so this is
  // an internal structural fact rather than a cross-entry-point contract. If it
  // fails, the re-export was replaced by a wrapper: a breaking change, not a
  // refactor. See runtime.js.
  it('re-exports the internal core build classes verbatim (no parallel wrappers)', () => {
    expect(Quill).toBe(CoreQuill)
    expect(Document).toBe(CoreDocument)
  })

  it('builds a canonical Quill with a backendId and a round-tripping tree', () => {
    const quill = makeRuntimeQuill()
    expect(quill.backendId).toBe('typst')

    // toTree is the inverse of fromTree: re-materializing reproduces an
    // equivalent quill (same backend, same files).
    const tree = quill.toTree()
    expect(tree).toBeInstanceOf(Map)
    expect(tree.has('Quill.yaml')).toBe(true)
    const rebuilt = Quill.fromTree(tree)
    expect(rebuilt.backendId).toBe('typst')
  })

  it('parses a Document via the re-exported core class', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    expect(doc.quillRef).toBe('test_quill')
  })

  // ERROR CONTRACT: every fallible method throws a real Error carrying a
  // non-empty `diagnostics` array (the QuillmarkError structural interface).
  // isQuillmarkError is the exported narrowing guard for it.
  it('throws satisfy isQuillmarkError with non-empty structured diagnostics', () => {
    let caught
    try {
      Document.fromMarkdown('~~~card-yaml\n$quill: test_quill\n$kind: main\ntitle: [unclosed\n~~~\n\nbody')
    } catch (e) {
      caught = e
    }
    expect(caught).toBeInstanceOf(Error)
    expect(isQuillmarkError(caught)).toBe(true)
    expect(caught.diagnostics.length).toBeGreaterThan(0)
    const d = caught.diagnostics[0]
    expect(typeof d.message).toBe('string')
    expect(d.severity).toBeDefined()
    // message derives from the diagnostics (first message or an aggregate)
    expect(caught.message.length).toBeGreaterThan(0)
  })

  it('isQuillmarkError rejects non-quillmark values', () => {
    expect(isQuillmarkError(new Error('plain'))).toBe(false) // no diagnostics
    expect(isQuillmarkError({ diagnostics: [] })).toBe(false) // not an Error
    expect(isQuillmarkError(undefined)).toBe(false)
    expect(isQuillmarkError('boom')).toBe(false)
    // structural acceptance: any Error carrying a diagnostics array narrows,
    // regardless of which build or WASM instance constructed it
    const foreign = Object.assign(new Error('x'), { diagnostics: [] })
    expect(isQuillmarkError(foreign)).toBe(true)
  })
})

// The typed-writer sugar binds a quill to a document once, so writes are bare
// `set` / `setAll` / `reviseField` / `card(i).set`: the JS twin of Rust's
// `quill.writer(doc)`. Every verb is a one-line delegation to the underscored
// `_commit*` / `_reviseField` ABI on the raw `Document` class (hidden from the
// `.d.ts`), whose error-code matrix is exercised at that altitude in
// `basic.test.js`. What is the sugar's own, and what these pin: each verb
// forwards to the right ABI call with the right address, and errors propagate
// rather than being swallowed.
describe('@quillmark/wasm/runtime: DocumentWriter / CardWriter (bind the quill once)', () => {
  const EDITOR_QUILL_YAML = `quill:
  name: editor_test
  version: "1.0"
  backend: typst
  description: Typed writer sugar test

main:
  fields:
    subject:
      type: richtext
      inline: true
    qty:
      type: integer

card_kinds:
  note:
    fields:
      body:
        type: richtext
`
  const buildQuill = () =>
    Quill.fromTree(makeQuill({ name: 'editor_test', plate: TEST_PLATE, quillYaml: EDITOR_QUILL_YAML }))
  const blankDoc = () => Document.fromMarkdown('~~~card-yaml\n$quill: editor_test\n~~~\n\nBody.')

  it('quill.writer(doc) is the front door and returns a DocumentWriter', () => {
    const quill = buildQuill()
    const ed = quill.writer(blankDoc())
    expect(ed).toBeInstanceOf(DocumentWriter)
    // The factory is sugar over the constructor: same class, no wrapping.
    expect(new DocumentWriter(quill, blankDoc())).toBeInstanceOf(DocumentWriter)
  })

  it('set / setAll bind the quill once and strict-commit main-card fields', () => {
    const ed = buildQuill().writer(blankDoc())
    ed.set('qty', '3') // schema field → strict coerce
    expect(fieldOf(ed.document.main, 'qty')).toBe(3)

    ed.setAll({ subject: 'Q3 **results**', qty: '5' })
    expect(fieldOf(ed.document.main, 'qty')).toBe(5)
  })

  it('an ABI error propagates through the sugar rather than being swallowed', () => {
    const ed = buildQuill().writer(blankDoc())
    expectEditCode(() => ed.set('stray', 'x'), 'edit::unknown_field')
    expect(fieldOf(ed.document.main, 'stray')).toBeUndefined()
  })

  it('reviseBody writes the main body from markdown, receipt-free', () => {
    const ed = buildQuill().writer(blankDoc())
    ed.reviseBody('New **body**.')
    expect(ed.document.bodyMarkdown()).toBe('New **body**.')
  })

  it('reviseField writes a richtext field typed, and returns a Delta', () => {
    const quill = buildQuill()
    const ed = quill.writer(blankDoc())
    const delta = ed.reviseField('subject', 'Q3 **results**')
    expect(quill.reader(ed.document).get('subject')).toBe('Q3 **results**')
    // The anchor-preserving receipt is a structured-clone-able change set.
    expect(Array.isArray(delta.ops)).toBe(true)
  })

  it('addCard fuses make + typed commit + push', () => {
    const ed = buildQuill().writer(blankDoc())
    // `body` here is the card's richtext FIELD; the third arg is the card body.
    ed.addCard('note', { body: 'Field **body**.' }, 'Card body text.')
    expect(ed.document.cards).toHaveLength(1)
    expect(ed.document.cards[0].kind).toBe('note')
    expect(exportMarkdown(fieldOf(ed.document.cards[0], 'body'))).toBe('Field **body**.')
    expect(exportMarkdown(ed.document.cards[0].body)).toBe('Card body text.')
  })

  it('removeCard drops the card and returns it', () => {
    const ed = buildQuill().writer(blankDoc())
    ed.addCard('note', { body: 'x' })
    const removed = ed.removeCard(0)
    expect(removed.kind).toBe('note')
    expect(ed.document.cards).toHaveLength(0)
  })

  it('card(i).set / reviseBody / reviseField address the composable card', () => {
    const doc = Document.fromMarkdown(
      '~~~card-yaml\n$quill: editor_test\n~~~\n\nMain.\n\n~~~card-yaml\n$kind: note\n~~~\n\nCard.',
    )
    const ed = buildQuill().writer(doc)
    ed.card(0).set('body', 'Card **body**.')
    expect(exportMarkdown(fieldOf(doc.cards[0], 'body'))).toBe('Card **body**.')
    ed.card(0).reviseBody('Card body md.')
    expect(exportMarkdown(doc.cards[0].body)).toBe('Card body md.')
    // card(i).reviseField is the typed, anchor-preserving field write.
    const delta = ed.card(0).reviseField('body', 'Revised **field**.')
    expect(exportMarkdown(fieldOf(doc.cards[0], 'body'))).toBe('Revised **field**.')
    expect(Array.isArray(delta.ops)).toBe(true)
  })

  it('a bad card index throws at write time, not at card()', () => {
    const ed = buildQuill().writer(blankDoc())
    const cardEd = ed.card(9) // lazy: constructing the CardWriter never throws
    expect(cardEd).toBeInstanceOf(CardWriter)
    expectEditCode(() => cardEd.set('body', 'x'), 'edit::index_out_of_range')
  })

  it('getStored reads raw values quill-free; bodyMarkdown is body-only (field half retired)', () => {
    const quill = buildQuill()
    const ed = quill.writer(blankDoc())
    ed.set('qty', '3')
    ed.set('subject', 'Q3 **results**')
    ed.reviseBody('Main **body**.')
    // Transport reads stay quill-free on the Document.
    expect(ed.document.getStored('qty')).toBe(3)
    expect(ed.document.getStored('missing')).toBeUndefined()
    // bodyMarkdown is the body read; a field address throws: a field's markdown
    // reads through the schema-plane view.
    expect(ed.document.bodyMarkdown()).toBe('Main **body**.')
    expect(() => ed.document.bodyMarkdown({ field: 'subject' })).toThrow(/body-only/)
    expect(quill.reader(ed.document).get('subject')).toBe('Q3 **results**')
    // reader.get carries schema authority: an unknown name throws (vs `undefined`
    // from the quill-free transport `Document.getStored` above).
    expectEditCode(() => quill.reader(ed.document).get('missing'), 'edit::unknown_field')
  })
})

// The typed-reader sugar is the read twin of the writer above: bind the quill
// once and read each field by its declared type (a richtext field to markdown,
// every other type verbatim) with schema authority, so an unknown field name
// throws rather than reading back `undefined` off the quill-free `Document`.
describe('@quillmark/wasm/runtime: DocumentReader / CardReader (the schema-plane read)', () => {
  const VIEW_QUILL_YAML = `quill:
  name: view_test
  version: "1.0"
  backend: typst
  description: Typed reader sugar test

main:
  fields:
    subject:
      type: richtext
      inline: true
    note:
      type: plaintext
    qty:
      type: integer

card_kinds:
  note:
    fields:
      body:
        type: richtext
`
  const buildQuill = () =>
    Quill.fromTree(makeQuill({ name: 'view_test', plate: TEST_PLATE, quillYaml: VIEW_QUILL_YAML }))
  const seededDoc = (quill) => {
    const doc = Document.fromMarkdown('~~~card-yaml\n$quill: view_test\n~~~\n\nMain **body**.')
    const w = quill.writer(doc)
    w.set('subject', 'Q3 **results**')
    w.set('qty', '3')
    w.addCard('note', { body: 'A *card* field.' }, 'Card body.')
    return doc
  }

  it('quill.reader(doc) is the front door and returns a DocumentReader', () => {
    const quill = buildQuill()
    const v = quill.reader(seededDoc(quill))
    expect(v).toBeInstanceOf(DocumentReader)
    expect(new DocumentReader(quill, seededDoc(quill))).toBeInstanceOf(DocumentReader)
  })

  it('interprets by declared type: richtext → markdown, plaintext → literal, scalar → canonical', () => {
    const quill = buildQuill()
    const doc = seededDoc(quill)
    quill.writer(doc).set('note', 'a *literal* line') // marks verbatim under plaintext
    const v = quill.reader(doc)
    expect(v.get('subject')).toBe('Q3 **results**') // richtext projects to markdown
    expect(v.get('note')).toBe('a *literal* line') // plaintext projects verbatim
    expect(v.get('qty')).toBe(3) // scalar returns canonical
  })

  it('absence returns undefined; an unknown name throws (schema authority)', () => {
    const quill = buildQuill()
    const v = quill.reader(Document.fromMarkdown('~~~card-yaml\n$quill: view_test\n~~~\n\nBody.'))
    expect(v.get('subject')).toBeUndefined() // absent, not a typo
    expectEditCode(() => v.get('nope'), 'edit::unknown_field') // typo, not absent
  })

  it('a richtext field holding a scalar throws FieldDecode', () => {
    const quill = buildQuill()
    const doc = Document.fromMarkdown('~~~card-yaml\n$quill: view_test\n~~~\n\nBody.')
    doc.storeField('subject', 3) // opaque write puts a bare number under richtext
    expectEditCode(() => quill.reader(doc).get('subject'), 'edit::field_decode')
  })

  it('an absent field addr reads the body markdown, quill-free', () => {
    const quill = buildQuill()
    const v = quill.reader(seededDoc(quill))
    expect(v.bodyMarkdown()).toBe('Main **body**.')
    expect(v.get({})).toBe('Main **body**.') // {} = main body, equals bodyMarkdown()
  })

  it('card(i).get reads a card field through its $kind schema', () => {
    const quill = buildQuill()
    const v = quill.reader(seededDoc(quill))
    expect(v.card(0).kind).toBe('note')
    expect(v.card(0).get('body')).toBe('A *card* field.')
    expect(v.card(0).bodyMarkdown()).toBe('Card body.')
    expectEditCode(() => v.card(0).get('nope'), 'edit::unknown_field')
  })

  it('a bad card index throws at read time, not at card()', () => {
    const quill = buildQuill()
    const cardReader = quill.reader(seededDoc(quill)).card(9)
    expect(cardReader).toBeInstanceOf(CardReader)
    expectEditCode(() => cardReader.get('body'), 'edit::index_out_of_range')
  })

  // getContent is the same read at the other end of the codec: the `Content`, not
  // the projection. Rest is per-codec now, so what it spans is a document at
  // rest versus one the transport door left as authored.
  it('getContent returns the Content for a conformed field and an authored one', () => {
    const quill = buildQuill()
    // At rest: the writer (like the bound door) stores the canonical Content.
    const committed = seededDoc(quill)
    expect(typeof committed.getStored('subject')).toBe('object')
    // Transport door: a markdown-authored field rests as authored until it is
    // conformed.
    const parsed = Document.fromMarkdown(
      '~~~card-yaml\n$quill: view_test\nsubject: Q3 **results**\n~~~\n\nBody.'
    )
    expect(typeof parsed.getStored('subject')).toBe('string')

    const a = quill.reader(committed).getContent('subject')
    const b = quill.reader(parsed).getContent('subject')
    expect(a.text).toBe('Q3 results')
    expect(b.text).toBe(a.text)
    expect(b.marks).toEqual(a.marks)
  })

  // The bound door is what makes a stored form a property of the codec rather
  // than of the construction lane.
  it('the bound door lands both codecs at their canonical rest', () => {
    const quill = buildQuill()
    const md =
      "~~~card-yaml\n$quill: view_test\nsubject: Q3 **results**\nnote: 'a *literal* line'\n~~~\n\nBody."
    const bound = quill.parse(md)
    expect(typeof bound.getStored('subject')).toBe('object') // richtext: the Content object
    expect(bound.getStored('note')).toBe('a *literal* line') // plaintext: the literal
    expect(bound.warnings).toEqual([])

    // conform is the same walk on a document that arrived any other way, and it
    // converges to identical bytes. A second pass is a no-op.
    const transported = Document.fromMarkdown(md)
    expect(quill.conform(transported)).toEqual([])
    expect(transported.equals(bound)).toBe(true)
    expect(quill.conform(transported)).toEqual([])
    expect(transported.toJson()).toBe(bound.toJson())
  })

  it('a value the strict write refuses rests authored with a conform warning', () => {
    const quill = buildQuill()
    const doc = quill.parse('~~~card-yaml\n$quill: view_test\nsubject: 42\n~~~\n\nBody.')
    expect(doc.getStored('subject')).toBe(42) // no silent retype
    expect(doc.warnings.map((d) => d.code)).toContain('conform::field_decode')
    expect(doc.warnings[0].severity).toBe('warning')
  })

  it('nothing conforms under the wrong quill', () => {
    const quill = buildQuill()
    const md = '~~~card-yaml\n$quill: other_quill\nsubject: hi\n~~~\n\nBody.'
    let caught
    try {
      quill.parse(md)
    } catch (e) {
      caught = e
    }
    expect(isQuillmarkError(caught)).toBe(true)
    expect(caught.diagnostics[0].code).toBe('quill::name_mismatch')

    // The transport door still opens it, and conform reports the same mismatch
    // without touching the document.
    const doc = Document.fromMarkdown(md)
    const before = doc.toJson()
    expectEditCode(() => quill.conform(doc), 'quill::name_mismatch')
    expect(doc.toJson()).toBe(before)
  })

  it('getContent decodes by declared type: markdown for richtext, literal for plaintext', () => {
    const quill = buildQuill()
    const doc = Document.fromMarkdown(
      "~~~card-yaml\n$quill: view_test\nsubject: 'a *literal* line'\nnote: 'a *literal* line'\n~~~\n\nBody."
    )
    const v = quill.reader(doc)
    // Same stored bytes, two codecs: only the declared type says which.
    expect(v.getContent('subject').text).toBe('a literal line')
    expect(v.getContent('note').text).toBe('a *literal* line')
  })

  it('getContent: absence, unknown name, non-content type, body addr, cards', () => {
    const quill = buildQuill()
    const doc = seededDoc(quill)
    const v = quill.reader(doc)
    expectEditCode(() => v.getContent('nope'), 'edit::unknown_field')
    expectEditCode(() => v.getContent('qty'), 'edit::field_not_content')
    expect(v.getContent({}).text).toBe('Main body.') // absent field = body Content
    expect(v.card(0).getContent('body').text).toBe('A card field.')
    expectEditCode(() => v.card(9).getContent('body'), 'edit::index_out_of_range')

    const empty = quill.reader(
      Document.fromMarkdown('~~~card-yaml\n$quill: view_test\n~~~\n\nBody.')
    )
    expect(empty.getContent('subject')).toBeUndefined()
  })
})

// MAIN_CARD_ADDR names the empty main-card address `{}` the card-scoped verbs
// take, so a main-card batch write reads as intent (`storeFields(MAIN_CARD_ADDR,
// fields)`) rather than as an anonymous `{}`. It IS `{}` (frozen), so it is a
// pure alias: `{}` and `undefined` stay equally valid.
describe('@quillmark/wasm/runtime: MAIN_CARD_ADDR (the named main-card address)', () => {
  it('is a frozen, empty card address: {} with a name', () => {
    expect(MAIN_CARD_ADDR).toEqual({})
    expect(Object.isFrozen(MAIN_CARD_ADDR)).toBe(true)
  })

  it('targets the main card on a card-scoped verb, identically to {}', () => {
    const named = new Document('editor_test')
    named.storeFields(MAIN_CARD_ADDR, { title: 'Hello', qty: 3 })
    expect(fieldOf(named.main, 'title')).toBe('Hello')
    expect(fieldOf(named.main, 'qty')).toBe(3)

    // Same effect as the bare empty-address spelling: a pure alias.
    const empty = new Document('editor_test')
    empty.storeFields({}, { title: 'Hello', qty: 3 })
    expect(named.main.payloadItems).toEqual(empty.main.payloadItems)
  })

  it('carries $ext onto the main card too', () => {
    const doc = new Document('editor_test')
    doc.storeExt(MAIN_CARD_ADDR, { editor: { pinned: true } })
    expect(doc.main.ext.editor.pinned).toBe(true)
  })
})

describe('@quillmark/wasm/runtime: open-set membership guards', () => {
  // One known name per axis, not the whole table: membership is a `Set.has`,
  // uniform across members, and the tables themselves are pinned against the
  // Rust constants by `crates/bindings/wasm/tests/known_names_drift.rs`.
  it('answers known-vs-unknown on all four axes', () => {
    expect(isUnknownLine({ kind: 'heading', level: 2, containers: [] })).toBe(false)
    expect(isUnknownLine({ kind: 'callout', attrs: {}, containers: [] })).toBe(true)

    expect(isUnknownContainer({ container: 'quote' })).toBe(false)
    expect(isUnknownContainer({ container: 'indent', attrs: {} })).toBe(true)

    expect(isUnknownMark({ start: 0, end: 1, type: 'strong' })).toBe(false)
    expect(isUnknownMark({ start: 0, end: 1, type: 'highlight', attrs: {} })).toBe(true)

    expect(isUnknownIsland({ id: 'i1', type: 'table', props: {}, loss: 'lossless' })).toBe(false)
    expect(isUnknownIsland({ id: 'i1', type: 'widget', props: {}, loss: 'lossless' })).toBe(true)
  })

  it('reports a missing or non-string discriminant as not-unknown, never throwing', () => {
    // A malformed value is not an unknown construct: it is malformed, and the
    // decoder rejects it. The guard must not turn one into the other.
    for (const bad of [{}, { kind: 7 }, null, undefined]) {
      expect(isUnknownLine(bad)).toBe(false)
    }
  })
})

describe('@quillmark/wasm/runtime: Engine (hidden core→backend crossing)', () => {
  // Warm the lazy Typst-backend import + first Typst compile once, outside any
  // timed test. `Engine.render` dynamically `import()`s the backend wasm binary
  // on first render: a one-time cost (large module instantiation) that on a
  // cold CI runner alone can approach the per-test ceiling. Paying it here keeps
  // the individual render tests warm (sub-second, like the SVG case) so a tight
  // per-test `testTimeout` still catches a genuine hang. The hook carries its own
  // generous timeout for the cold load.
  beforeAll(async () => {
    await new Engine().render(makeRuntimeQuill(), Document.fromMarkdown(TEST_MARKDOWN), {
      format: 'pdf',
    })
  }, 120000)

  it('renders a core Quill + Document to PDF without exposing a backend handle', async () => {
    const engine = new Engine()
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    const result = await engine.render(quill, doc, { format: 'pdf' })
    expect(result.artifacts.length).toBeGreaterThan(0)
    expect(result.outputFormat).toBe('pdf')
    expect(result.artifacts[0].bytes).toBeInstanceOf(Uint8Array)
    expect(result.artifacts[0].bytes.length).toBeGreaterThan(0)

    // The caller's canonical handles survive the render (clones were transient
    // and freed inside the engine; the originals are untouched).
    expect(quill.backendId).toBe('typst')
    expect(doc.quillRef).toBe('test_quill')
  })

  it('renders to SVG and reports supported formats / canvas capability', async () => {
    const engine = new Engine()
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    const svg = await engine.render(quill, doc, { format: 'svg' })
    expect(svg.outputFormat).toBe('svg')

    const formats = await engine.supportedFormats(quill)
    expect(formats).toContain('svg')
    expect(typeof (await engine.supportsCanvas(quill))).toBe('boolean')
  })

  it('manifest-backed capability probes do NOT load the backend', async () => {
    // A descriptor-form counting loader: it carries the same manifest the
    // default registry uses, so probes answer from the manifest (no load),
    // while still counting any real binary load triggered by render.
    let loaded = 0
    const engine = new Engine({
      backends: {
        typst: {
          load: () => {
            loaded++
            return import('../../../pkg/backends/typst/wasm.js')
          },
          formats: ['pdf', 'svg', 'png'],
          canvas: true
        }
      }
    })
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    // Descriptor WITH a manifest → probes answer from the manifest, no load.
    const formats = await engine.supportedFormats(quill)
    expect(formats).toContain('pdf')
    expect(typeof (await engine.supportsCanvas(quill))).toBe('boolean')
    expect(loaded).toBe(0)

    // A real render still triggers exactly one load.
    await engine.render(quill, doc, { format: 'svg' })
    expect(loaded).toBe(1)
  })

  it('manifest formats cannot drift from the loaded backend (drift guard)', async () => {
    const engine = new Engine()
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    // What the static manifest reports (no load).
    const manifestFormats = await engine.supportedFormats(quill)
    const manifestCanvas = await engine.supportsCanvas(quill)

    // Force the backend to actually load, then ask the real engine directly.
    await engine.render(quill, doc, { format: 'svg' })
    const mod = await import('../../../pkg/backends/typst/wasm.js')
    const backendEngine = new mod.Quillmark()
    const backendQuill = mod.Quill.fromTree(quill.toTree())
    try {
      const realFormats = backendEngine.supportedFormats(backendQuill)
      const realCanvas = backendEngine.supportsCanvas(backendQuill)
      // The manifest must match what the binary reports, both directions.
      expect([...manifestFormats].sort()).toEqual([...realFormats].sort())
      expect(manifestCanvas).toBe(realCanvas)
    } finally {
      backendQuill.free()
    }
  })

  it('pdfform manifest cannot drift from the loaded backend (drift guard)', async () => {
    // Same drift guard as typst, but for the pdfform backend: the static
    // `{ formats, canvas }` manifest in DEFAULT_BACKENDS must match what the
    // loaded pdfform binary actually reports.
    const engine = new Engine()
    const quill = Quill.fromTree(makeSampleFormQuill())
    expect(quill.backendId).toBe('pdfform')
    const doc = Document.fromMarkdown(SAMPLE_FORM_MARKDOWN)

    // What the static manifest reports (no load).
    const manifestFormats = await engine.supportedFormats(quill)
    const manifestCanvas = await engine.supportsCanvas(quill)
    expect([...manifestFormats].sort()).toEqual(['pdf', 'png', 'svg'])
    expect(manifestCanvas).toBe(true)

    // Force the pdfform backend to load, then ask the real engine directly.
    await engine.render(quill, doc, { format: 'pdf' })
    const mod = await import('../../../pkg/backends/pdfform/wasm.js')
    const backendEngine = new mod.Quillmark()
    const backendQuill = mod.Quill.fromTree(quill.toTree())
    try {
      const realFormats = backendEngine.supportedFormats(backendQuill)
      const realCanvas = backendEngine.supportsCanvas(backendQuill)
      expect([...manifestFormats].sort()).toEqual([...realFormats].sort())
      expect(manifestCanvas).toBe(realCanvas)
    } finally {
      backendQuill.free()
    }
  })

  it('throws at construction for a malformed backend descriptor (names the id)', () => {
    // A backend entry must be a descriptor `{ load, formats, canvas }`; a bare thunk is rejected.
    expect(() => new Engine({ backends: { typst: () => import('../../../pkg/backends/typst/wasm.js') } })).toThrow(
      /typst/
    )
    // Missing/invalid manifest fields also fail fast at construction.
    expect(
      () => new Engine({ backends: { mybackend: { load: () => Promise.resolve({}), canvas: true } } })
    ).toThrow(/mybackend/)
    expect(
      () =>
        new Engine({
          backends: { mybackend: { load: () => Promise.resolve({}), formats: ['pdf'], canvas: 'yes' } }
        })
    ).toThrow(/mybackend/)
  })

  // A loader that wraps the real backend module so `Quill.fromTree` calls are
  // counted (and still delegate to the real implementation). Used to prove the
  // per-Engine quill-clone cache materializes the backend quill once per
  // canonical instance instead of per render/open call.
  function fromTreeCountingEngine(options) {
    let fromTreeCalls = 0
    const engine = new Engine({
      ...options,
      backends: {
        typst: {
          load: async () => {
            const real = await import('../../../pkg/backends/typst/wasm.js')
            const wrappedQuill = new Proxy(real.Quill, {
              get(target, prop, receiver) {
                if (prop === 'fromTree') {
                  return (...args) => {
                    fromTreeCalls++
                    return target.fromTree(...args)
                  }
                }
                return Reflect.get(target, prop, receiver)
              }
            })
            return new Proxy(real, {
              get(target, prop, receiver) {
                if (prop === 'Quill') return wrappedQuill
                return Reflect.get(target, prop, receiver)
              }
            })
          },
          formats: ['pdf', 'svg', 'png'],
          canvas: true
        }
      }
    })
    return { engine, fromTreeCalls: () => fromTreeCalls }
  }

  it('caches the backend quill clone: rendering twice materializes it once', async () => {
    const { engine, fromTreeCalls } = fromTreeCountingEngine()
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    await engine.render(quill, doc, { format: 'svg' })
    await engine.render(quill, doc, { format: 'svg' })
    expect(fromTreeCalls()).toBe(1)
  })

  it('caches per canonical instance: two different quills → two materializations', async () => {
    const { engine, fromTreeCalls } = fromTreeCountingEngine()
    const quillA = makeRuntimeQuill()
    const quillB = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    await engine.render(quillA, doc, { format: 'svg' })
    await engine.render(quillB, doc, { format: 'svg' })
    expect(fromTreeCalls()).toBe(2)
  })

  it('opens an iterative session, renders pages, and frees it', async () => {
    const engine = new Engine()
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    const session = await engine.open(quill, doc)
    try {
      expect(session.pageCount).toBeGreaterThan(0)
      expect(session.backendId).toBe('typst')
      const page = session.render({ format: 'svg' })
      expect(page.artifacts.length).toBeGreaterThan(0)
    } finally {
      session.free()
    }
  })

  // GUARD for the class of bug where a method is declared in runtime.d.ts and
  // implemented in the backend build, but the hand-written canonical LiveSession
  // wrapper (runtime.js) forgets to forward it: `fieldAt` is the case in point.
  // The type-level drift test (runtime.types.test-d.ts) only checks structural type
  // compatibility, so a wrapper that TYPE-checks but has no matching JS method
  // sails through it and throws `X is not a function` at runtime. This calls
  // EVERY documented LiveSession member on a live canonical session, so a
  // dropped delegation surfaces here instead of only in a consumer.
  it('canonical LiveSession forwards every documented method to the inner session', async () => {
    // paint() downcasts its argument to a 2D context via wasm-bindgen's
    // `instanceof` check, so it needs these globals present (Node has no DOM).
    class FakeImageData {
      constructor(data, width, height) {
        this.data = data
        this.width = width
        this.height = height
      }
    }
    class FakeCanvasRenderingContext2D {
      constructor() {
        this.calls = []
        this.canvas = { width: 0, height: 0 }
      }
      putImageData(img, dx, dy) {
        this.calls.push({ width: img.width, height: img.height, dx, dy })
      }
    }
    globalThis.ImageData = FakeImageData
    globalThis.CanvasRenderingContext2D = FakeCanvasRenderingContext2D

    // A SINGLE-LINE $body, deliberately. `fieldAt` hit-tests per-glyph ink
    // boxes, so the probe point below (the region rect's centre) must land on
    // ink: a one-line body's region rect IS that line's contiguous glyph
    // boxes, so its centre is ink by construction. TEST_MARKDOWN's
    // heading+paragraph body has an inter-line gap at the union rect's
    // centre, where fieldAt correctly answers undefined.
    const SMOKE_MARKDOWN = `~~~card-yaml
$quill: test_quill
$kind: main
title: Smoke Test
author: Smoke Author
~~~

A single line of body ink.`

    const engine = new Engine()
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(SMOKE_MARKDOWN)
    const session = await engine.open(quill, doc)
    try {
      // Getters.
      expect(session.pageCount).toBeGreaterThan(0)
      expect(session.backendId).toBe('typst')
      expect(typeof session.supportsCanvas).toBe('boolean')
      expect(Array.isArray(session.warnings)).toBe(true)

      // render.
      expect(typeof session.render).toBe('function')
      expect(session.render({ format: 'svg' }).artifacts.length).toBeGreaterThan(0)

      // regions: the body markdown content field auto-tags one region, keyed
      // by the canonical DocPath `main.body`.
      expect(typeof session.regions).toBe('function')
      const regions = session.regions()
      const body = regions.find((r) => r.field === 'main.body')
      expect(body).toBeDefined()

      // pageSize.
      const size = session.pageSize(body.page)
      expect(size.widthPt).toBeGreaterThan(0)
      expect(size.heightPt).toBeGreaterThan(0)

      // fieldAt: the delegation that was missing. Hit-test the centre
      // of the body region's rect ([x0, y0, x1, y1], bottom-left PDF points)
      // (guaranteed ink for the single-line body (see SMOKE_MARKDOWN above))
      // and expect it to resolve back through the wrapper as its DocPath. Off
      // any field's ink (the page corner) the contract is undefined.
      expect(typeof session.fieldAt).toBe('function')
      const [x0, y0, x1, y1] = body.rect
      const hit = session.fieldAt(body.page, (x0 + x1) / 2, (y0 + y1) / 2)
      expect(hit).toBe('main.body')
      expect(session.fieldAt(body.page, 1, 1)).toBeUndefined()

      // fieldBoxes: the whole-field union helper. A single-line body has one
      // span-bearing segment, so its box unions to one rect covering that line.
      expect(typeof session.fieldBoxes).toBe('function')
      const boxes = session.fieldBoxes('main.body')
      expect(boxes.length).toBe(1)
      expect(boxes[0].field).toBe('main.body')
      expect(boxes[0].span).toBeDefined()
      // A field with no span-bearing region has no derived content box.
      expect(session.fieldBoxes('does_not_exist')).toEqual([])

      // positionAt: the fine-grained click direction, carrying the granularity
      // signal. A hit on the single line's ink is cluster-exact.
      expect(typeof session.positionAt).toBe('function')
      const chit = session.positionAt(body.page, (x0 + x1) / 2, (y0 + y1) / 2)
      expect(chit.field).toBe('main.body')
      expect(chit.granularity).toBe('cluster')

      // paint.
      expect(typeof session.paint).toBe('function')
      const ctx = new FakeCanvasRenderingContext2D()
      const paintResult = session.paint(ctx, body.page)
      expect(paintResult.pixelWidth).toBeGreaterThan(0)

      // update: recompile in place.
      expect(typeof session.update).toBe('function')
      const cs = session.update(Document.fromMarkdown(SMOKE_MARKDOWN))
      expect(Array.isArray(cs.dirtyPages)).toBe(true)
    } finally {
      session.free()
    }
  })

  it('renders repeatedly from the same quill (clone-on-demand, no shared handle)', async () => {
    const engine = new Engine()
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    const a = await engine.render(quill, doc, { format: 'svg' })
    const b = await engine.render(quill, doc, { format: 'svg' })
    expect(a.artifacts.length).toBeGreaterThan(0)
    expect(b.artifacts.length).toBeGreaterThan(0)
  })

  it('throws a clear error for an unregistered backend', async () => {
    const engine = new Engine()
    // A quill whose declared backend has no loader.
    const yaml = `quill:
  name: mystery
  version: "1.0.0"
  backend: doesnotexist
  description: no backend registered
main:
  fields:
    title:
      type: string
      example: x
`
    const quill = Quill.fromTree(new Map([['Quill.yaml', new TextEncoder().encode(yaml)]]))
    const doc = quill.seedDocument()
    await expect(engine.render(quill, doc)).rejects.toThrow(/no backend registered/)
  })

  it('accepts a custom backend descriptor override', async () => {
    let loaded = 0
    const engine = new Engine({
      backends: {
        typst: {
          load: () => {
            loaded++
            return import('../../../pkg/backends/typst/wasm.js')
          },
          formats: ['pdf', 'svg', 'png'],
          canvas: true
        }
      }
    })
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    await engine.render(quill, doc, { format: 'svg' })
    expect(loaded).toBe(1)
  })

  // A counting descriptor loader for the lazy-load / coalescing invariants below.
  function countingEngine() {
    let loaded = 0
    const engine = new Engine({
      backends: {
        typst: {
          load: () => {
            loaded++
            return import('../../../pkg/backends/typst/wasm.js')
          },
          formats: ['pdf', 'svg', 'png'],
          canvas: true
        }
      }
    })
    return { engine, loaded: () => loaded }
  }

  it('does NOT load the backend for sync core work: only on first render (lazy)', async () => {
    const { engine, loaded } = countingEngine()
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    // Sync core surface (schema / validate / seed) touches no backend.
    expect(quill.schema).toBeDefined()
    quill.validate(doc)
    quill.seedDocument().free?.()
    expect(loaded()).toBe(0)

    // First render triggers exactly one backend load.
    await engine.render(quill, doc, { format: 'svg' })
    expect(loaded()).toBe(1)
  })

  it('coalesces concurrent first renders into a single backend load', async () => {
    const { engine, loaded } = countingEngine()
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    await Promise.all([
      engine.render(quill, doc, { format: 'svg' }),
      engine.render(quill, doc, { format: 'svg' }),
      engine.render(quill, doc, { format: 'svg' })
    ])
    expect(loaded()).toBe(1)
  })

  it('caller may free() its handles as soon as render/open returns (pre-await snapshot)', async () => {
    // Both caller handles are snapshotted before the first await inside
    // render/open (the backend load: a real suspension point on first call),
    // so a synchronous free() right after the call cannot race the clone.
    // Regression pin for the "null pointer passed to rust" panic:
    // each engine below is fresh, so its first call has the load pending when
    // free() runs.
    const renderEngine = new Engine()
    const renderQuill = makeRuntimeQuill()
    const renderDoc = Document.fromMarkdown(TEST_MARKDOWN)
    const pendingRender = renderEngine.render(renderQuill, renderDoc, { format: 'svg' })
    renderDoc.free()
    renderQuill.free()
    const result = await pendingRender
    expect(result.artifacts.length).toBeGreaterThan(0)

    const openEngine = new Engine()
    const openQuill = makeRuntimeQuill()
    const openDoc = Document.fromMarkdown(TEST_MARKDOWN)
    const pendingOpen = openEngine.open(openQuill, openDoc)
    openDoc.free()
    openQuill.free()
    const session = await pendingOpen
    try {
      expect(session.pageCount).toBeGreaterThan(0)
    } finally {
      session.free()
    }
  })

  it('propagates a clone-construction failure (doc clone), leaving the quill clone cached', async () => {
    // Exercises the teardown path when the doc clone (Document.fromJson) throws:
    // the quill clone is already materialized and cached (NOT freed here, that
    // is the T3 caching contract), only the per-call doc clone is freed in the
    // finally. We can only assert the error surfaces (cache/leak state is not
    // observable from JS), but this pins the throw path #withClones guards.
    //
    // The failure is injected through the backend REGISTRY, not through a
    // stand-in Document: both caller handles are checked before the clone runs
    // (see "handles from another copy" below), so they have to be real. Same
    // Proxy-over-the-real-module shape as fromTreeCountingEngine, so the quill
    // clone left cached is a real backend quill and only `fromJson` misbehaves.
    const engine = new Engine({
      backends: {
        typst: {
          load: async () => {
            const real = await import('../../../pkg/backends/typst/wasm.js')
            const refusingDocument = new Proxy(real.Document, {
              get(target, prop, receiver) {
                if (prop === 'fromJson') {
                  return () => {
                    throw new Error('doc clone refused')
                  }
                }
                return Reflect.get(target, prop, receiver)
              },
            })
            return new Proxy(real, {
              get(target, prop, receiver) {
                if (prop === 'Document') return refusingDocument
                return Reflect.get(target, prop, receiver)
              },
            })
          },
          formats: ['pdf', 'svg', 'png'],
          canvas: true,
        },
      },
    })
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    await expect(engine.render(quill, doc)).rejects.toThrow('doc clone refused')
  })
})

// A duplicate install is two copies of this package: two core builds, two
// linear memories, two distinct `Quill`/`Document` classes. A handle never
// crosses between them; every seam taking one checks, and throws in contract
// naming the duplicate install and `npm ls`. See runtime.js § "Handles from
// another copy". These pin that the rule is UNIFORM: the by-reference core
// methods (which wasm-bindgen would reject anyway, as a bare `Error`), the
// writer and reader binds, `Engine`, and `LiveSession.apply`. The last two are
// the seams that cross as data and would otherwise silently work.
//
// A foreign handle is modelled two ways: a stand-in carrying the serializer a
// real handle has (the shape most likely to slip a check), and a second copy of
// the built core artifact on disk, which is a genuinely different class over a
// different linear memory.
describe('@quillmark/wasm/runtime: handles from another copy (duplicate install)', () => {
  const foreignDoc = (doc) => ({ toJson: () => doc.toJson() })
  const foreignQuill = (quill) => ({
    toTree: () => quill.toTree(),
    backendId: quill.backendId,
  })

  /** Every rejection is in contract, codes `runtime::foreign_handle`, and names `npm ls`. */
  const assertForeign = (caught, method) => {
    // wasm-bindgen's bare `_assertClass` throw (`expected instance of Document`
    // at a value that IS a Document) fails every line below.
    expect(isQuillmarkError(caught)).toBe(true)
    expect(caught.diagnostics[0].code).toBe('runtime::foreign_handle')
    expect(caught.message).toContain(method)
    expect(caught.diagnostics[0].hint).toMatch(/npm ls @quillmark\/wasm/)
    return caught
  }
  /** The thrown value, `undefined` if `call` returned. */
  const caughtFrom = (call) => {
    try {
      call()
    } catch (e) {
      return e
    }
  }
  const expectForeign = (call, method) => assertForeign(caughtFrom(call), method)
  // Kept separate from the sync form rather than folded into one async helper:
  // a forgotten `await` on an async assertion passes silently.
  const expectForeignAsync = (promise, method) =>
    promise.then(
      () => expect.unreachable(`${method} resolved instead of refusing a foreign handle`),
      (e) => assertForeign(e, method)
    )

  it('rejects a foreign Document on the by-reference core methods', () => {
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    const other = Document.fromMarkdown(TEST_MARKDOWN)

    expectForeign(() => doc.equals(foreignDoc(other)), 'Document.equals')
    expectForeign(() => quill.validate(foreignDoc(doc)), 'Quill.validate')
    expectForeign(() => quill.resolve(foreignDoc(doc)), 'Quill.resolve')

    // A local handle still takes the generated path unchanged.
    expect(doc.equals(other)).toBe(true)
  })

  it('rejects a non-handle argument with a distinct code, naming the method', () => {
    // A different bug with a different cure: `npm ls` is the wrong advice for a
    // caller who passed null, so the two do not share a diagnostic.
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    for (const [call, method] of [
      [() => doc.equals(null), 'Document.equals'],
      [() => quill.validate({}), 'Quill.validate'],
      [() => quill.resolve(42), 'Quill.resolve'],
    ]) {
      const caught = caughtFrom(call)
      expect(isQuillmarkError(caught)).toBe(true)
      expect(caught.message).toContain(method)
      expect(caught.diagnostics[0].code).toBe('runtime::not_a_document')
    }
    expectEditCode(() => quill.writer(null), 'runtime::not_a_document')
    expect(() => new DocumentWriter(null, doc)).toThrow(/expected a Quill/)
  })

  it('refuses a foreign Document at the writer and reader binds', () => {
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    expectForeign(() => quill.writer(foreignDoc(doc)), 'quill.writer(doc)')
    expectForeign(() => quill.reader(foreignDoc(doc)), 'quill.reader(doc)')
    // The card cursors are their own bind, and construct directly.
    expectForeign(
      () => new CardWriter(quill, foreignDoc(doc), 0),
      'writer.card(index)'
    )
    expectForeign(
      () => new CardReader(quill, foreignDoc(doc), 0),
      'reader.card(index)'
    )
  })

  it('refuses a foreign Quill at the writer and reader binds', () => {
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    expectForeign(() => new DocumentWriter(foreignQuill(quill), doc), 'quill.writer(doc)')
    expectForeign(() => new DocumentReader(foreignQuill(quill), doc), 'quill.reader(doc)')
  })

  // Engine is the seam with no `_assertClass` to front-run: it crosses into
  // backend memory as data, so a foreign handle would render correctly and pay
  // a per-copy quill clone cache for it. It checks instead.
  it('refuses a foreign handle at every Engine entry point', async () => {
    const engine = new Engine()
    const quill = makeRuntimeQuill()
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    const m = 'engine.render(quill, doc)'
    await expectForeignAsync(engine.render(foreignQuill(quill), doc), m)
    await expectForeignAsync(engine.render(quill, foreignDoc(doc)), m)
    await expectForeignAsync(engine.open(foreignQuill(quill), doc), 'engine.open(quill, doc)')
    await expectForeignAsync(engine.open(quill, foreignDoc(doc)), 'engine.open(quill, doc)')
    // The free probes check too, though they never touch the handle's memory:
    // the rule is about the handle, not about what a given verb happens to read.
    await expectForeignAsync(
      engine.supportedFormats(foreignQuill(quill)),
      'engine.supportedFormats(quill)'
    )
    await expectForeignAsync(
      engine.supportsCanvas(foreignQuill(quill)),
      'engine.supportsCanvas(quill)'
    )
  })

  // "Every seam checks" is enforced by hand-placed calls, so the set of seams
  // is pinned like the backend manifest above: a fifth Engine verb fails here
  // until it is classified, rather than silently accepting a foreign handle.
  // Inside the class the check is structural (`#backendOf` is the only route to
  // `backendId`, and `#withClones` checks the doc); this guards the boundary.
  it('has no Engine verb outside the checked set (drift guard)', () => {
    const TAKES_A_QUILL = ['render', 'open', 'supportedFormats', 'supportsCanvas']
    const TAKES_NO_HANDLE = ['constructor']
    expect(Object.getOwnPropertyNames(Engine.prototype).sort()).toEqual(
      [...TAKES_A_QUILL, ...TAKES_NO_HANDLE].sort()
    )
  })

  it('refuses a foreign Document on session.update', async () => {
    const engine = new Engine()
    const quill = makeRuntimeQuill()
    const session = await engine.open(quill, Document.fromMarkdown(TEST_MARKDOWN))
    try {
      const next = Document.fromMarkdown(TEST_MARKDOWN.replace('Hello World', 'Next'))
      expectForeign(() => session.update(foreignDoc(next)), 'session.update(doc)')
      // The session is untouched by the refusal and still applies a local doc.
      expect(session.update(next).pageCount).toBe(session.pageCount)
    } finally {
      session.free()
    }
  }, 120000)

  // The real shape: a SECOND COPY of the built core artifact on disk, which is
  // what npm produces. A query suffix is not enough, since `wasm.js?x`
  // re-evaluates but still imports the cached `./wasm_bg.js`, leaving the
  // classes identical. Copying the directory forks the module graph and the
  // linear memory.
  describe('against a second copy of the core build on disk', () => {
    let copyB
    beforeAll(async () => {
      const src = path.join(PKG_DIR, 'core')
      const dst = path.join(PKG_DIR, 'dup-core')
      fs.rmSync(dst, { recursive: true, force: true })
      fs.cpSync(src, dst, { recursive: true })
      copyB = await import(/* @vite-ignore */ path.join(dst, 'wasm.js'))
    })

    it('is genuinely a different class over a different memory', () => {
      expect(copyB.Document).not.toBe(Document)
      expect(copyB.Quill).not.toBe(Quill)
    })

    it('refuses copy B handles everywhere, in contract', async () => {
      const quillA = makeRuntimeQuill()
      const docA = Document.fromMarkdown(TEST_MARKDOWN)
      const docB = copyB.Document.fromMarkdown(TEST_MARKDOWN)
      const quillB = copyB.Quill.fromTree(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))

      expectForeign(() => docA.equals(docB), 'Document.equals')
      expectForeign(() => quillA.validate(docB), 'Quill.validate')
      expectForeign(() => quillA.resolve(docB), 'Quill.resolve')
      expectForeign(() => quillA.writer(docB), 'quill.writer(doc)')
      expectForeign(() => quillA.reader(docB), 'quill.reader(doc)')
      expectForeign(() => new DocumentWriter(quillB, docA), 'quill.writer(doc)')

      const engine = new Engine()
      await expectForeignAsync(engine.render(quillA, docB), 'engine.render(quill, doc)')
      await expectForeignAsync(
        engine.supportedFormats(quillB),
        'engine.supportedFormats(quill)'
      )

      // Nothing we did freed or mutated the caller's handles.
      expect(docB.quillRef).toBe('test_quill')
      expect(quillB.backendId).toBe('typst')
    })

    it('reads copy B handles fine through copy B, which is the point', () => {
      // The rule is about CROSSING, not about copy B being defective. Copy B's
      // own classes work together; only mixing them throws.
      const docB = copyB.Document.fromMarkdown(TEST_MARKDOWN)
      const quillB = copyB.Quill.fromTree(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
      expect(quillB.validate(docB)).toBeDefined()
    })
  })

  // Re-evaluating the runtime module against a cached, and so already patched,
  // core build must not wrap the wrappers: the Vite HMR / shared Vitest worker
  // case the `Symbol.for` marker exists for. A copy of runtime.js beside the
  // original re-evaluates while its relative `../core/wasm.js` import still
  // resolves to the cached module.
  it('patches once across a re-evaluation of the runtime module', async () => {
    const before = Document.prototype.equals
    expect(before[Symbol.for('@quillmark/wasm:handle-checked')]).toBe(true)

    // The twin has to sit BESIDE the original for its `../core/wasm.js` import
    // to resolve to the same cached module. `pkg/runtime/` is a published
    // directory, so remove it as soon as it is loaded rather than leave a stray
    // file that a publish from an unrebuilt pkg/ would ship.
    const twin = path.join(PKG_DIR, 'runtime', 'runtime.hmr.js')
    fs.copyFileSync(path.join(PKG_DIR, 'runtime', 'runtime.js'), twin)
    let reevaluated
    try {
      reevaluated = await import(/* @vite-ignore */ twin)
    } finally {
      fs.rmSync(twin, { force: true })
    }

    expect(reevaluated.Document).toBe(Document)
    expect(Document.prototype.equals).toBe(before)
    expect(Quill.prototype.validate[Symbol.for('@quillmark/wasm:handle-checked')]).toBe(true)
  })
})
