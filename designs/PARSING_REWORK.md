# Quillmark Parsing Rework

Users should be able to tag documents with `!quill {quill_name}`. `quillmark` crate consumer should be able to load workflows with `quillmark::Quillmark::workflow_from_parsed()`. This pipes into `workflow_from_quill_name` internally.

For example, for the following document:

```md
--- !quill usaf_memo
memo_for: [ORG/SYMBOL]
---
```

Consumers can call:

```rust
let parsed = ParsedDocument::from_markdown(markdown)?;          // 1. Parse
let workflow = engine.workflow_from_parsed(&parsed)?;           // 2. Load workflow
let result = workflow.render(&parsed, Some(OutputFormat::Pdf))?; // 3
```

See `designs/PARSE.md` for current parsing design.