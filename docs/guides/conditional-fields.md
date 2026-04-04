# Conditional Fields

Show or hide fields in a UI based on the value of other fields, using `visible_when` in the `ui` property.

## The Problem

Some fields are only relevant in certain contexts. In a USAF memo, the indorsement card has a `format` field with three options: `standard`, `informal`, and `separate_page`. The `from` and `for` addressing fields make sense for standard and separate-page endorsements, but informal endorsements omit them.

Without conditional visibility, every field appears all the time. The UI is cluttered with fields that don't apply, and users have to know which ones to ignore.

## The Solution: `visible_when`

Add `visible_when` to a field's `ui` block to declare when it should be shown. In real `Quill.yaml` files, define these keys under `main.fields` (see [Quill.yaml Reference](quill-yaml-reference.md)).

```yaml
main:
  fields:
    format:
      type: string
      enum: [standard, informal, separate_page]
      default: standard

    from:
      type: string
      ui:
        group: Addressing
        visible_when:
          format: [standard, separate_page]
```

The `from` field appears when `format` is `"standard"` or `"separate_page"`, and disappears when `format` is `"informal"`.

## How It Works

`visible_when` is a map of **sibling field name** to **accepted values**:

```yaml
visible_when:
  field_name: [value1, value2, ...]
```

The visibility rules:

| Condition | Meaning |
|-----------|---------|
| No `visible_when` | Always visible |
| One key with values | Visible when that field matches any value (OR) |
| Multiple keys | Visible when ALL keys match (AND across keys, OR within each) |

### Single Condition

Show `risk_description` only when status is `at_risk` or `blocked`:

```yaml
main:
  fields:
    status:
      type: string
      enum: [on_track, at_risk, blocked]

    risk_description:
      type: string
      ui:
        visible_when:
          status: [at_risk, blocked]
```

### Multiple Conditions (AND)

Show `classified_handling` only when the document is both classified AND formal:

```yaml
main:
  fields:
    classification:
      type: string
      enum: ["", secret, top_secret]

    format:
      type: string
      enum: [formal, informal]

    classified_handling:
      type: string
      ui:
        visible_when:
          classification: [secret, top_secret]
          format: [formal]
```

Both conditions must be true: classification must be `secret` or `top_secret`, **and** format must be `formal`.

## JSON Schema Output

In the YAML, you write:

```yaml
from:
  type: string
  ui:
    group: Addressing
    visible_when:
      format: [standard, separate_page]
```

Quillmark emits this in the JSON Schema as:

```json
{
  "from": {
    "type": "string",
    "x-ui": {
      "group": "Addressing",
      "visible_when": {
        "format": ["standard", "separate_page"]
      }
    }
  }
}
```

The `ui` block maps directly to `x-ui` — no compilation, no transformation. What you write is what gets emitted.

## UI Implementation Guide

For developers building form UIs that consume Quillmark schemas, the implementation is straightforward:

```
for each field in schema.properties:
  rules = field["x-ui"]["visible_when"]
  if rules is absent:
    show the field
  else:
    for each (key, accepted_values) in rules:
      current = get_current_value(key)
      if current not in accepted_values:
        hide the field
        break
    else:
      show the field
```

That's it. No JSON Schema evaluator required. Read the `x-ui` object, compare sibling values, show or hide.

### Pseudocode (JavaScript)

```javascript
function isFieldVisible(fieldSchema, siblingValues) {
  const rules = fieldSchema["x-ui"]?.visible_when;
  if (!rules) return true;

  return Object.entries(rules).every(([field, accepted]) =>
    accepted.includes(siblingValues[field])
  );
}
```

## Scope and Limitations

### Sibling Fields Only

`visible_when` references fields at the same level — within the same card or at the document root. There is no cross-card or cross-level referencing.

```yaml
cards:
  indorsement:
    fields:
      format:
        type: string
        enum: [standard, informal]
      from:
        type: string
        ui:
          visible_when:
            format: [standard]    # References sibling "format", not a root field
```

### UI Hint, Not Validation

`visible_when` controls what the UI shows. It does **not** make fields conditionally required or conditionally invalid. A hidden field can still have a value in the document — it's simply not shown to the user. This is intentional:

- Templates can safely ignore fields they don't use for a given configuration
- Switching `format` from `standard` to `informal` doesn't invalidate existing `from` data
- No JSON Schema `if/then/else` complexity in your validation pipeline

### String Matching

Values in the accepted list are matched as strings. This works naturally for `enum` fields and other string types. For boolean-like fields, use the string representation:

```yaml
visible_when:
  has_attachments: ["true"]
```

## Real-World Example: USAF Memo Indorsements

The `usaf_memo` quill uses `visible_when` on its indorsement card:

```yaml
cards:
  indorsement:
    title: Routing indorsement
    description: Chain of routing endorsements.
    fields:
      from:
        title: From office/symbol
        type: string
        default: ORG/SYMBOL
        ui:
          group: Addressing
          visible_when:
            format: [standard, separate_page]
        description: Office symbol of the endorsing official.

      for:
        title: To office/symbol
        type: string
        default: ORG/SYMBOL
        ui:
          group: Addressing
          visible_when:
            format: [standard, separate_page]
        description: Office symbol receiving the endorsed memo.

      signature_block:
        title: Signature block lines
        type: array
        required: true
        ui:
          group: Addressing
        description: "Name, grade, service, and duty title."

      format:
        title: Indorsement format
        type: string
        enum: [standard, informal, separate_page]
        default: standard
        ui:
          group: Additional
        description: "Format style for the endorsement."

      attachments:
        title: Attachments
        type: array
        ui:
          group: Additional

      cc:
        title: Carbon copy recipients
        type: array
        ui:
          group: Additional

      date:
        title: Date of endorsement
        type: string
        ui:
          group: Additional
```

When a user selects `informal` as the format, the `from` and `for` fields disappear from the form. The signature block, attachments, cc, and date remain visible regardless of format.

## Next Steps

- [Quill.yaml Reference](quill-yaml-reference.md) — complete YAML property reference
- [Creating Quills](creating-quills.md) — hands-on tutorial
- [USAF Memo](../quills/usaf-memo.md) — full quill documentation with endorsements
