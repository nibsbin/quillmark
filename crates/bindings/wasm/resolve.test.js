import { describe, it, expect } from 'vitest'
import { Quill, Quillmark } from '@quillmark-wasm'

describe('resolveQuill bug', () => {
  it('should resolve correct version', () => {
    const engine = new Quillmark()

    // Register 0.1.0
    const quill1 = {
      files: {
        'Quill.yaml': {
          contents: `Quill:
  name: usaf_memo
  version: "0.1.0"
  backend: typst
  plate_file: plate.typ
  description: Version 0.1.0
`
        },
        'plate.typ': { contents: 'hello 1' }
      }
    }
    engine.registerQuill(Quill.fromJson(quill1))

    // Register 0.2.0
    const quill2 = {
      files: {
        'Quill.yaml': {
          contents: `Quill:
  name: usaf_memo
  version: "0.2.0"
  backend: typst
  plate_file: plate.typ
  description: Version 0.2.0
`
        },
        'plate.typ': { contents: 'hello 2' }
      }
    }
    engine.registerQuill(Quill.fromJson(quill2))

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
