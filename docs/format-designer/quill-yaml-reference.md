# Quill.yaml Reference

Complete reference for authoring `Quill.yaml` configuration files. For a hands-on introduction, see [Creating Quills](creating-quills.md).

## File Structure

A `Quill.yaml` has these top-level sections:

```yaml
Quill:        # Required — format metadata
  ...

main:         # Optional — document main card: field schemas and optional ui
  fields:
    ...
  ui:         # optional container hints (e.g. hide_body)

cards:        # Optional — composable content block types
  ...

typst:        # Optional — backend-specific configuration
  ...
```

Root-level `fields:` is not supported; define the main document’s field schemas under `main.fields`.

---

## `Quill` Section

Every Quill.yaml must have a `Quill` section with format metadata.

| Key              | Type   | Required | Description |
|------------------|--------|----------|-------------|
| `name`           | string | yes      | Unique identifier for the Quill |
| `backend`        | string | yes      | Rendering backend (e.g. `typst`) |
| `description`    | string | yes      | Human-readable description (non-empty) |
| `version`        | string | yes      | Semantic version (`MAJOR.MINOR` or `MAJOR.MINOR.PATCH`) |
| `author`         | string | no       | Creator of the Quill (defaults to `"Unknown"`) |
| `plate_file`     | string | no       | Path to the plate file |
| `example_file`   | string | no       | Path to an example Markdown document |
| `ui`             | object | no       | Document-level UI metadata |

```yaml
Quill:
  name: usaf_memo
  version: "0.1"
  backend: typst
  description: Typesetted USAF Official Memorandum
  author: TongueToQuill
  plate_file: plate.typ
  example_file: example.md
```

### Document-level `ui`

Controls UI behavior for the document root:

```yaml
Quill:
  name: metadata-only-doc
  # ...
  ui:
    hide_body: true    # Suppress the body/content editor in form UIs
```

---

## `main` Section

The main document card holds **frontmatter field schemas** under `main.fields`. Optional `main.ui` sets container-level UI for that card (for example `hide_body`). `Quill.ui` is merged with `main.ui` when building the main card.

Field order under `main.fields` determines display order in UIs — the first field gets `order: 0`, the second gets `order: 1`, and so on.

```yaml
main:
  fields:
    subject:          # Field name (used as the YAML frontmatter key)
      title: Subject of the memo
      type: string
      required: true
      description: Be brief and clear.
```

### Field Properties

| Property      | Type              | Required | Description |
|---------------|-------------------|----------|-------------|
| `type`        | string            | yes      | Data type (see [Field Types](#field-types)) |
| `title`       | string            | no       | Short label shown in UIs |
| `description` | string            | no       | Detailed help text |
| `default`     | any               | no       | Default value when not provided |
| `examples`    | array             | no       | Example values for documentation and LLMs |
| `required`    | boolean           | no       | Whether the field must be present (default: `false`) |
| `enum`        | array of strings  | no       | Restrict to specific values |
| `ui`          | object            | no       | UI rendering hints (see [UI Properties](#ui-properties)) |
| `items`       | object            | no       | Item schema (for `array` type; use `type: object` with `properties` for structured rows) |

### Field Types

| Type       | JSON Schema Output | Notes |
|------------|-------------------|-------|
| `string`   | `"type": "string"` | Also accepts `str` as alias |
| `number`   | `"type": "number"` | |
| `boolean`  | `"type": "boolean"` | |
| `array`    | `"type": "array"` | Use `items` for element schema |
| `date`     | `"type": "string", "format": "date"` | YYYY-MM-DD |
| `datetime`  | `"type": "string", "format": "date-time"` | ISO 8601 |
| `markdown` | `"type": "string", "contentMediaType": "text/markdown"` | Rich text; backends convert to target format |
| `object` or `dict` | `"type": "object"` with `properties` | Supported for typed table rows inside `array.items` |

Use `type: array` with `items: { type: object, properties: {...} }` when you need a **list** of structured rows. Top-level `type: object` fields are not supported.

### Enum Constraints

Restrict a string field to specific values:

```yaml
main:
  fields:
    format:
      type: string
      enum:
        - standard
        - informal
        - separate_page
      default: standard
      description: "Format style for the endorsement."
```

### Typed Arrays

Define array element schemas with `items`:

```yaml
main:
  fields:
    recipients:
      type: array
      items:
        type: string
      examples:
        - ["ORG1/SYMBOL", "ORG2/SYMBOL"]
```

Use `type: object` inside `items` to define structured rows. Coercion recurses into each element and converts property values to their declared types:

```yaml
main:
  fields:
    cells:
      type: array
      items:
        type: object
        properties:
          category:
            type: string
            required: true
          score:
            type: number
```

---

## UI Properties

The `ui` property on fields controls how form builders and wizards render the field. These are emitted as `x-ui` in the generated JSON Schema — they are hints for UI consumers, not validation constraints.

### `group`

Organizes fields into visual sections:

```yaml
main:
  fields:
    memo_for:
      type: array
      ui:
        group: Addressing

    memo_from:
      type: array
      ui:
        group: Addressing

    letterhead_title:
      type: string
      ui:
        group: Letterhead
```

Fields with the same `group` value are rendered together. The group name becomes the section heading.

### `order`

Auto-assigned based on field position in the YAML file. You rarely need to set this manually — just put fields in the order you want them displayed.

If you do need to override:

```yaml
main:
  fields:
    # Will get order: 0 from position, but we force it to 5
    special_field:
      type: string
      ui:
        order: 5
```

### `multiline`

Controls the initial size of the text input for `markdown` fields. When `true`, the UI starts with a larger text box instead of a single-line input:

```yaml
main:
  fields:
    summary:
      type: markdown
      description: Executive summary
      ui:
        multiline: true   # start as a larger text box

    tagline:
      type: markdown
      description: One-sentence tagline
      # no multiline — single-line input that expands on demand
```

`multiline` is a UI hint only — it has no effect on validation or backend processing. It is only meaningful on `markdown` fields and is ignored on other types.

---

## `cards` Section

Cards define composable, repeatable content blocks. A document can have zero or more instances of each card type, interleaved with body content.

```yaml
cards:
  indorsement:                    # Card type name
    title: Routing indorsement    # Display label
    description: Chain of routing endorsements.
    fields:
      from:
        type: string
        ui:
          group: Addressing
      format:
        type: string
        enum: [standard, informal, separate_page]
        default: standard
```

### Card Properties

| Property      | Type   | Required | Description |
|---------------|--------|----------|-------------|
| `title`       | string | no       | Display label for the card type |
| `description` | string | no       | Help text describing the card's purpose |
| `fields`      | object | no       | Field schemas (same structure as top-level fields) |
| `ui`          | object | no       | Container-level UI hints |

### Card-level `ui`

| Property       | Type   | Description |
|----------------|--------|-------------|
| `hide_body`    | bool   | Suppress the body/content editor for this card type |
| `default_title` | string | Template for per-instance titles in UI consumers |

#### `hide_body`

```yaml
cards:
  metadata_block:
    title: Metadata
    ui:
      hide_body: true    # Card has fields only, no body/content editor
    fields:
      category:
        type: string
```

#### `default_title`

A template string that UI consumers interpolate with field values to produce a human-readable title for each card instance. Uses `{field_name}` tokens referencing fields in the same card.

```yaml
cards:
  entry:
    title: Card Title
    ui:
      default_title: "{name}"
    fields:
      name:
        type: string
        title: Name
```

With the above, a UI rendering a list of `entry` cards can title each instance (e.g. `"Project Alpha"`) instead of falling back to a generic `"Card Title (2)"`.

**Interpolation rules (for UI consumers):**
- `{field_name}` is replaced with the current value of that field.
- If a field is absent or empty, the token resolves to an empty string.
- UI consumers are responsible for trimming degenerate separators (e.g. `" — "` with one empty side).

`default_title` is a UI hint only — it has no effect on validation or rendering.

### Using Cards in Markdown

Cards appear as fenced sections in the document body:

```markdown
---
QUILL: usaf_memo
subject: Example
# ... other fields ...
---

Main memo body text here.

~~~indorsement
from: ORG/SYMBOL
for: RECIPIENT/SYMBOL
signature_block:
  - JANE A. DOE, Colonel, USAF
  - Commander
~~~

Body of the first endorsement.

~~~indorsement
from: ANOTHER/ORG
for: FINAL/RECIPIENT
format: informal
signature_block:
  - JOHN B. SMITH, Lt Col, USAF
  - Deputy Commander
~~~

Body of the second endorsement.
```

---

## `typst` Section

Backend-specific configuration for the Typst renderer.

```yaml
typst:
  packages:
    - "@preview/appreciated-letter:0.1.0"
```

See the [Typst Backend Guide](typst-backend.md) for details.

---

## Generated JSON Schema

Quillmark generates a JSON Schema from your `Quill.yaml`. This schema is used for validation and consumed by UIs.

### How YAML maps to JSON Schema

| Quill.yaml | JSON Schema |
|------------|-------------|
| `type: string` | `"type": "string"` |
| `type: markdown` | `"type": "string", "contentMediaType": "text/markdown"` |
| `required: true` | Field name in `"required"` array |
| `default: value` | `"default": value` |
| `enum: [a, b]` | `"enum": ["a", "b"]` |
| `ui: { group: X }` | `"x-ui": { "group": "X" }` |
| `ui: { multiline: true }` | `"x-ui": { "multiline": true }` |
| `ui: { default_title: "{f}" }` | `"x-ui": { "default_title": "{f}" }` |
| `cards: { name: ... }` | `"$defs": { "name_card": ... }` |

### Stripped Schema

The `getStrippedSchema()` API returns the schema with `x-ui` removed — useful for LLM prompts and external validators that don't need UI hints.

---

## Complete Example

```yaml
Quill:
  name: project_report
  version: "1.0"
  backend: typst
  description: Monthly project status report
  author: Engineering Team
  plate_file: plate.typ
  example_file: example.md

main:
  fields:
    project_name:
      title: Project name
      type: string
      required: true
      ui:
        group: Header

    status:
      title: Overall status
      type: string
      required: true
      enum: [on_track, at_risk, blocked]
      ui:
        group: Header

    risk_description:
      title: Risk description
      type: string
      ui:
        group: Header
      description: Describe the risk or blocker. Only needed when status is not on_track.

    date:
      title: Report date
      type: date
      ui:
        group: Header

    team_members:
      title: Team members
      type: array
      items:
        type: string
      ui:
        group: Team

    budget:
      title: Budget amount
      type: number
      default: 0
      ui:
        group: Financials

cards:
  milestone:
    title: Milestone
    description: A project milestone with target date and status.
    fields:
      name:
        type: string
        required: true
      target_date:
        type: date
      completed:
        type: boolean
        default: false
```

---

## Next Steps

- [Creating Quills](creating-quills.md) — hands-on tutorial
- [Markdown Syntax](../authoring/markdown-syntax.md) — document authoring syntax
- [Validation](../integration/validation.md) — validating documents against schemas
