# Quillmark Parsing Rework

This rework introduces the ability to tag markdown documents with the Quill to be used for rendering. This rework also changes the subdocument standard for consistency.

## Quill Tagging

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

## Subdocument Tagging

Currently, subdocuments are specified with `!{parent_field}`. This should be changed to `!scope {parent_field}` for consistency with quil ltagging.

### Before

```md
---
!indorsements
name: "FIRST M. LAST, Rank, USSF"
title: "Title"
organization: "ORG/SYMBOL"
```

### After

```md
---
!scope indorsements
name: "FIRST M. LAST, Rank, USSF"
title: "Title"
organization: "ORG/SYMBOL"
---
```

## References

See `designs/PARSE.md` for current parsing design.