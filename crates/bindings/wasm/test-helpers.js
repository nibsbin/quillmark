const enc = new TextEncoder()

export function makeQuill({
  name = 'test_quill',
  version = '1.0.0',
  plate = '#import "@local/quillmark-helper:0.1.0": data\n= Test',
} = {}) {
  return new Map([
    ['Quill.yaml', enc.encode(`Quill:
  name: ${name}
  version: "${version}"
  backend: typst
  plate_file: plate.typ
  description: Test quill for smoke tests
`)],
    ['plate.typ', enc.encode(plate)],
  ])
}
