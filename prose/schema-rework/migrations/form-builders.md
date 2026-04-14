# Migration: Form Builders

Audience: consumers that render a UI form from a Quill's schema (our own
form designer, any third-party integrator).

## What changed

| | Before | After |
|---|---|---|
| Format | JSON Schema (`application/schema+json`) | Quill.yaml subset (YAML text) |
| Delivery | Parsed JSON object | YAML string — consumer parses once |
| UI hints | `x-ui` extension keyword | Native `ui:` key (no rename, no namespace) |
| Cards | Top-level `CARDS` array property | Top-level `cards:` map keyed by card name |
| Views | `SchemaProjection::UI` vs `SchemaProjection::AI` | Single unified schema — no projection |

The new contract is a strict YAML subset of the author's own `Quill.yaml`,
so the shape the designer sees is the shape the author wrote.

## New shape

```yaml
name: <string>
description: <string>           # optional
example: <markdown string>      # optional; full end-to-end document example

fields:
  <field_name>:
    type: string | number | boolean | array | object | date | datetime | markdown
    title: <string>
    description: <string>
    required: <bool>            # omitted when false
    default: <value>
    examples: [<value>, ...]
    enum: [<value>, ...]
    properties: { ... }         # object types
    items: { ... }              # array types
    ui:
      group: <string>
      order: <int>
      compact: <bool>
      multiline: <bool>

cards:                          # omitted when empty
  <card_name>:
    title: <string>
    description: <string>
    fields: { <same shape as above> }
    ui:
      hide_body: <bool>
      default_title: <string>
```

Golden reference: `crates/fixtures/resources/quills/usaf_memo/0.1.0/__golden__/public_schema.yaml`.

## Fetching the schema

**WASM:**

```js
// before
const info = engine.getQuillInfo('usaf_memo');
const schema = info.schema;                // JSON Schema object
const stripped = engine.getStrippedSchema('usaf_memo');  // UI projection
```

```js
// after
const info = engine.getQuillInfo('usaf_memo');
const schemaYaml = info.schema;            // YAML text (string)
// or, stand-alone:
const schemaYaml = engine.getQuillSchema('usaf_memo');
// parse with js-yaml / yaml / @std/yaml
import yaml from 'js-yaml';
const schema = yaml.load(schemaYaml);
```

`getStrippedSchema()` is removed. There is no separate UI projection —
parse the YAML once and read `ui:` keys where you need them.

**Python:**

```python
# before
info = quill.info                  # dict with JSON Schema
schema = info["schema"]
```

```python
# after
schema_yaml = quill.schema         # str (YAML text)
import yaml
schema = yaml.safe_load(schema_yaml)
```

## Key-by-key rewrites

### `x-ui.*` → `ui.*`

```yaml
# before
properties:
  name:
    type: string
    x-ui:
      group: identity
      order: 1
```

```yaml
# after
fields:
  name:
    type: string
    ui:
      group: identity
      order: 1
```

Rename wherever your form code reads the hint. The key names underneath
(`group`, `order`, `compact`, `multiline` for fields; `hide_body`,
`default_title` for cards) are unchanged.

### `properties:` → `fields:` at top level

```yaml
# before (JSON Schema)
type: object
properties:
  subject: { type: string }
  date: { type: string, format: date }
required: [subject]
```

```yaml
# after
fields:
  subject:
    type: string
    required: true
  date:
    type: date
```

`required` moves from a top-level array to a per-field boolean. The
top-level object shell is gone — start at `fields:` directly.

### `CARDS` array → `cards:` map

```yaml
# before
properties:
  CARDS:
    type: array
    items:
      oneOf:
        - $ref: '#/$defs/indorsement'
        - $ref: '#/$defs/attachment'
$defs:
  indorsement: { type: object, properties: { ... } }
```

```yaml
# after
cards:
  indorsement:
    title: Indorsement
    fields: { ... }
  attachment:
    title: Attachment
    fields: { ... }
```

Card types are keys, not entries in a `oneOf`. Cards live alongside
`fields:` at the top level, not nested under a property.

### Type names

The seven canonical types are `string`, `number`, `boolean`, `array`,
`object`, `date`, `datetime`, `markdown`. JSON Schema's `format: date`
etc. collapses into the type itself (`type: date` not `type: string,
format: date`).

### Removed JSON Schema keywords

None of the following appear in the new contract. If your form code
branches on them, delete those branches:

- `$schema`, `$id`, `$ref`, `$defs`, `definitions`
- `allOf`, `anyOf`, `oneOf`, `not`
- `additionalProperties`, `patternProperties`
- `minLength`, `maxLength`, `minimum`, `maximum`, `pattern`, `format`
- `dependencies`, `if`/`then`/`else`

The Quill schema DSL is intentionally small. Constraints beyond type +
enum are out of scope — validation runs in the engine via
`QuillConfig::validate()`, not in the form.

## Validation

Client-side form validation no longer runs through a JSON Schema
validator (`ajv`, `jsonschema`, etc.). Options:

1. **Trust the engine.** Submit the field payload; on error, parse the
   engine's structured `ValidationError` list (see
   `crates/core/src/quill/validation.rs:12`). Errors use field-path
   strings like `cards.indorsement[0].signature_block`, directly
   addressable in form state.

2. **Lightweight client-side checks.** Implement your own type + enum +
   required checks from the parsed YAML. No regex/format validation —
   that's the engine's job.

There is no supported client-side JSON-Schema-equivalent validator.

## Quick checklist

- [ ] Replace `JSON.parse(info.schema)` with `yaml.load(info.schema)`
      (or equivalent)
- [ ] Remove `getStrippedSchema()` calls
- [ ] Rename every `x-ui` read to `ui`
- [ ] Walk `schema.fields` at the top level (not `schema.properties`)
- [ ] Walk `schema.cards[name]` instead of `schema.$defs[name]` or a
      `CARDS` array
- [ ] Drop any JSON-Schema-keyword branches (`$ref`, `oneOf`, `format`, …)
- [ ] Switch client-side validation to engine-driven (or a minimal
      type/enum/required checker)
- [ ] Map `required` from a top-level array to a per-field `required: true`
