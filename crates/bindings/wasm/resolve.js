import { Quill, Quillmark } from './pkg/quillmark_wasm.js'

const engine = new Quillmark()

const quill1 = {
  name: "usaf_memo",
  backend: "typst",
  metadata: { version: "0.1.0" },
  schema: {},
  plate: "hello 1"
}
engine.registerQuill(Quill.fromJson(quill1))

const quill2 = {
  name: "usaf_memo",
  backend: "typst",
  metadata: { version: "0.2.0" },
  schema: {},
  plate: "hello 2"
}
engine.registerQuill(Quill.fromJson(quill2))

const resolved = engine.resolveQuill("usaf_memo@0.2.0")
console.log("Resolved version:", resolved.metadata.version)
