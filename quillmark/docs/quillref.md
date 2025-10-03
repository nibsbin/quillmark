# QuillRef

Reference to a Quill, either by name or by borrowed object.

`QuillRef` provides an ergonomic way to reference quills when loading workflows.
It automatically converts from common string types and quill references.

## Examples

```no_run
# use quillmark::{Quillmark, Quill, QuillRef};
# let mut engine = Quillmark::new();
# let quill = Quill::from_path("path/to/quill").unwrap();
# engine.register_quill(quill.clone());
// All of these work:
let workflow = engine.load("my-quill").unwrap();           // &str
let workflow = engine.load(&String::from("my-quill")).unwrap();  // &String
let workflow = engine.load(&quill).unwrap();               // &Quill
```
