import { Quill, Quillmark } from './pkg/quillmark_wasm.js'

const engine = new Quillmark()
const enc = new TextEncoder()

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
engine.registerQuill(Quill.fromTree(new Map(
  Object.entries(quill1.files).map(([path, { contents }]) => [path, enc.encode(contents)])
)))

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
engine.registerQuill(Quill.fromTree(new Map(
  Object.entries(quill2.files).map(([path, { contents }]) => [path, enc.encode(contents)])
)))

const resolved = engine.resolveQuill("usaf_memo@0.2.0")
console.log("Resolved version:", resolved.metadata.version)
