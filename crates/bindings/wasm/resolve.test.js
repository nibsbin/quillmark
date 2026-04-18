import { describe, it, expect } from 'vitest'
import { Quill, Quillmark } from '@quillmark-wasm'

const enc = new TextEncoder()

function makeVersionedQuill(version) {
  return Quill.fromTree(new Map([
    ['Quill.yaml', enc.encode(`Quill:
  name: usaf_memo
  version: "${version}"
  backend: typst
  plate_file: plate.typ
  description: Version ${version}
`)],
    ['plate.typ', enc.encode(`hello ${version}`)],
  ]))
}

describe('resolveQuill bug', () => {
  it('should resolve correct version', () => {
    const engine = new Quillmark()

    engine.registerQuill(makeVersionedQuill('0.1.0'))
    engine.registerQuill(makeVersionedQuill('0.2.0'))

    // Verify resolveQuill returns the correct info
    const info2 = engine.resolveQuill("usaf_memo@0.2.0")
    expect(info2).toBeDefined()
    expect(info2.name).toBe("usaf_memo")
    expect(info2.metadata.version).toBe("0.2.0")

    const info1 = engine.resolveQuill("usaf_memo@0.1.0")
    expect(info1).toBeDefined()
    expect(info1.name).toBe("usaf_memo")
    expect(info1.metadata.version).toBe("0.1.0")
  })
})
