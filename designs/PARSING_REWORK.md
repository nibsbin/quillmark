# Quillmark Parsing Rework

This rework introduces the ability to tag markdown documents with the Quill to be used for rendering using reserved YAML keys instead of YAML tag directives.

## Quill Tagging

Users should be able to tag documents with `QUILL: {quill_name}`. The `quillmark` crate consumer should be able to load workflows with `quillmark::Quillmark::workflow_from_parsed()`. This pipes into `workflow_from_quill_name` internally.

For example, for the following document:

```md
---
QUILL: usaf_memo
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

Subdocuments are specified with the `SCOPE:` reserved key. This uses standard YAML syntax instead of custom tag directives.

### Format

```md
---
SCOPE: indorsements
name: "FIRST M. LAST, Rank, USSF"
title: "Title"
organization: "ORG/SYMBOL"
---
```

## Reserved Keys

- `QUILL`: Specifies the quill (template/workflow) to use for rendering
- `SCOPE`: Specifies the field name for scoped/tagged blocks that become arrays
- `body`: Reserved field for document body content (cannot be used as SCOPE value)

## References

See `designs/PARSE.md` for current parsing design.