import { Quill, Quillmark } from './pkg/quillmark_wasm.js'
import { makeQuill } from './test-helpers.js'

const engine = new Quillmark()

engine.registerQuill(Quill.fromTree(makeQuill({
  name: 'usaf_memo',
  version: '0.1.0',
  plate: 'hello 0.1.0',
})))
engine.registerQuill(Quill.fromTree(makeQuill({
  name: 'usaf_memo',
  version: '0.2.0',
  plate: 'hello 0.2.0',
})))

const resolved = engine.resolveQuill("usaf_memo@0.2.0")
console.log("Resolved version:", resolved.metadata.version)
