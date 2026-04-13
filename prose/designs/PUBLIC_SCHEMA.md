# Public Schema Contract

## TL;DR

Public schema is YAML text emitted by `QuillConfig::public_schema_yaml()`. It is consumed directly by LLM/tooling integrations and UI/form builders.

## What it is

A YAML subset projection of `Quill.yaml`:

- `name`
- `description`
- optional `example`
- `fields`
- `cards`

## Who consumes it

- LLM generation/repair loops
- form/UI builders
- third-party integrations that need field contracts without internal runtime details

## Shape

```yaml
name: usaf_memo
description: Typesetted USAF Official Memorandum
example: |
  ---
  QUILL: usaf_memo
  ...
fields:
  memo_for:
    type: array
    title: Memorandum for
    description: Memorandum recipients.
    required: true
    items:
      type: string
    ui:
      group: Addressing
      order: 0
cards:
  indorsement:
    title: Routing Indorsement
    description: Routing chain metadata.
    fields:
      from:
        type: string
        required: true
```

## Relationship to `Quill.yaml`

Projection is by exclusion:

- Keeps field/card contracts and author-facing hints
- Drops internal metadata used for loading/runtime internals

`QuillConfig` remains the source of truth for both runtime and emitted contract.

## Why YAML text (not JSON object)

- Matches authoring format (`Quill.yaml`) and docs/examples
- Avoids maintaining parallel object schemas/projections in bindings
- Keeps binding contracts simple (`quill.schema` / `quillInfo.schema` are plain strings)

## Contract ownership

The emitted output shape is the contract. Fixtures and snapshot tests define the canonical expected output.
