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

## Intended consumers

- LLM generation/repair loops
- form/UI builders
- third-party integrations that need field contracts without internal runtime details

## Shape

Top-level keys: `name`, `description` (optional), `example` (optional), `fields`, `cards` (omitted when empty).

Each field includes: `type`, `title` (optional), `description` (optional), `required` (omitted when false), `default` (optional), `examples` (optional list), `enum` (optional), `properties` (optional, for `object` fields), `items` (optional, for `array` fields), `ui` (optional).

Each card includes: `title` (optional), `description` (optional), `fields`, `ui` (optional).

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
  status:
    type: string
    enum: [draft, final]
    default: draft
card_types:
  indorsement:
    title: Routing Indorsement
    description: Routing chain metadata.
    fields:
      from:
        type: string
        required: true
    ui:
      hide_body: false
      default_title: Indorsement
```

## Relationship to `Quill.yaml`

Projection is by exclusion:

- Keeps field/card contracts and author-facing hints
- Drops internal metadata used for loading/runtime internals

`QuillConfig` remains the source of truth for both runtime and emitted contract.

## Who exposes it

- **Python** (`crates/bindings/python/`): `quill.schema` property returns YAML string. See PYTHON.md.
- **CLI** (`crates/bindings/cli/`): `quillmark schema <path>` subcommand prints or writes the YAML.
- **WASM** (`crates/bindings/wasm/`): the WASM `Quill` class does **not** currently expose a schema getter. See WASM.md.

## Why YAML text (not JSON object)

- Matches authoring format (`Quill.yaml`) and docs/examples
- Avoids maintaining parallel object schemas/projections in bindings
- Keeps binding contracts simple — callers receive a plain YAML string

## Contract ownership

The emitted output shape is the contract. Fixtures and snapshot tests define the canonical expected output.
