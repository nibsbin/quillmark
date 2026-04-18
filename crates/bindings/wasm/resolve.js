import { Quill, Quillmark } from './pkg/quillmark_wasm.js'

const engine = new Quillmark()
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

engine.registerQuill(makeVersionedQuill('0.1.0'))
engine.registerQuill(makeVersionedQuill('0.2.0'))

const resolved = engine.resolveQuill("usaf_memo@0.2.0")
console.log("Resolved version:", resolved.metadata.version)
