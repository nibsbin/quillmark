# Quill.toml Scopes Semantic Design
## Supporting Atomic and Collection Scopes

**Date:** 2025-12-16
**Status:** Design Workshop - Final Semantics
**Related:** QUILL_TOML_ENHANCEMENT_PROPOSAL.md

---

## Design Decisions

Based on design workshop, the following semantics are chosen:

1. ✅ **Section name**: `[scopes.*]` with `singular` flag field
2. ✅ **Default behavior**: Array (backward compatible), unknown scopes allowed
3. ✅ **Singular scopes**: Strict enforcement (error on multiple blocks)
4. ✅ **Required scopes**: Not supported (all scopes are optional)
5. ✅ **Collection minimums**: Not supported (no `min_items`)

---

## Core Semantics

### Scope Types

**Collection Scope (Default)**
- Multiple SCOPE blocks with same name → array of objects
- Each block becomes an item in the array
- Default behavior when `singular` is not specified

**Atomic Scope (Singular)**
- Single SCOPE block → one object (not wrapped in array)
- Exactly one block allowed (strict enforcement)
- Enabled with `singular = true`

### TOML Syntax

```toml
# Collection scope (array) - DEFAULT
[scopes.indorsements]
description = "Endorsements appended to memo"
# singular is false by default (or omitted)

[scopes.indorsements.fields.from]
type = "string"
# ...

# Atomic scope (object) - SINGULAR
[scopes.appendix]
singular = true  # This scope is atomic
description = "Appendix section with supplementary materials"

[scopes.appendix.fields.title]
type = "string"
title = "Appendix title"

[scopes.appendix.fields.page_break]
type = "boolean"
default = true
```

---

## Behavior Rules

### 1. Default Behavior (No Schema Defined)

**Markdown:**
```markdown
---
SCOPE: unknown_scope
foo: bar
---
Content
```

**Behavior:**
- ✅ Allowed (lenient mode)
- Defaults to **collection scope** (array)
- No validation applied
- Result: `unknown_scope = [{foo: "bar", body: "Content"}]`

**Rationale:** Backward compatible, allows experimentation without schema

---

### 2. Collection Scope (Array)

**TOML:**
```toml
[scopes.products]
# singular not specified = collection (default)
description = "Product listings"

[scopes.products.fields.name]
type = "string"

[scopes.products.fields.price]
type = "number"
```

**Markdown:**
```markdown
---
SCOPE: products
name: Widget
price: 19.99
---
Widget description

---
SCOPE: products
name: Gadget
price: 29.99
---
Gadget description
```

**Result:**
```json
{
  "products": [
    {"name": "Widget", "price": 19.99, "body": "Widget description"},
    {"name": "Gadget", "price": 29.99, "body": "Gadget description"}
  ]
}
```

**Edge Cases:**
- ✅ **Zero blocks**: `products = []` (empty array, valid)
- ✅ **One block**: `products = [{...}]` (array with one item)
- ✅ **Many blocks**: `products = [{...}, {...}, ...]` (array with N items)

---

### 3. Atomic Scope (Object)

**TOML:**
```toml
[scopes.appendix]
singular = true  # Atomic scope
description = "Appendix section"

[scopes.appendix.fields.title]
type = "string"
title = "Appendix title"

[scopes.appendix.fields.classification]
type = "string"
default = ""
```

**Markdown:**
```markdown
---
SCOPE: appendix
title: "Technical Specifications"
classification: "UNCLASSIFIED"
---

## Detailed Technical Data

Appendix content here...
```

**Result:**
```json
{
  "appendix": {
    "title": "Technical Specifications",
    "classification": "UNCLASSIFIED",
    "body": "## Detailed Technical Data\n\nAppendix content here..."
  }
}
```

**Edge Cases:**
- ✅ **Zero blocks**: `appendix` field not present in document (optional)
- ✅ **One block**: `appendix = {...}` (single object, correct)
- ❌ **Two+ blocks**: **PARSER ERROR** (strict enforcement)

**Error Message:**
```
Error: Scope 'appendix' is defined as singular but found 2 blocks at lines 45 and 67.
Only one SCOPE: appendix block is allowed.
```

---

### 4. Validation Rules

**Collection Scopes:**
- ✅ Validate each item against field schema
- ✅ Apply defaults to each item
- ❌ No `min_items` constraint (not supported)
- ⚠️ Optional: `max_items` could be added if needed (TBD)

**Atomic Scopes:**
- ✅ Validate single object against field schema
- ✅ Apply defaults to fields
- ✅ **Strict cardinality**: Error if multiple blocks found
- ❌ Not required (scope can be absent from document)

---

## Complete Example: USAF Memo

```toml
[Quill]
name = "usaf_memo"
backend = "typst"
description = "USAF Official Memorandum with endorsements and appendix"

# ===========================================
# MAIN DOCUMENT FIELDS
# ===========================================

[fields.subject]
type = "string"
title = "Subject line"
ui.group = "Essentials"
description = "Brief, clear subject"

[fields.memo_for]
type = "array"
title = "Recipient organization(s)"
ui.group = "Essentials"

# ... other main fields

# ===========================================
# SCOPES
# ===========================================

# Collection scope: Multiple endorsements
[scopes.indorsements]
# singular = false (implicit default)
description = "Endorsements (1st Ind, 2d Ind, etc.) per AFH 33-337"
ui.group = "Endorsements"
ui.icon = "mail-forward"
ui.add_button_text = "Add Endorsement"
ui.item_label = "{{ordinal}} Indorsement"

[scopes.indorsements.fields.from]
title = "Endorsing organization"
type = "string"
examples = ["HQ USAF/A1"]
ui.group = "Header"

[scopes.indorsements.fields.to]
title = "Recipient organization"
type = "string"
examples = ["INSTALLATION/CC"]
ui.group = "Header"

[scopes.indorsements.fields.signature_block]
title = "Signature block"
type = "array"
examples = [["JANE DOE, Col, USAF", "Commander"]]
ui.group = "Signature"

[scopes.indorsements.fields.new_page]
title = "Start on new page"
type = "boolean"
default = false
ui.group = "Formatting"

# Atomic scope: Single appendix
[scopes.appendix]
singular = true  # Only one appendix allowed
description = "Appendix with supplementary technical details"
ui.group = "Appendix"
ui.icon = "document-attachment"

[scopes.appendix.fields.title]
title = "Appendix title"
type = "string"
default = "APPENDIX"
ui.group = "Header"

[scopes.appendix.fields.classification]
title = "Classification marking"
type = "string"
default = ""
examples = ["CONFIDENTIAL"]
ui.group = "Header"

[scopes.appendix.fields.page_break]
title = "Start on new page"
type = "boolean"
default = true
ui.group = "Formatting"

# Atomic scope: Cover letter
[scopes.cover_letter]
singular = true
description = "Cover letter for memo transmission"
ui.group = "Cover Letter"

[scopes.cover_letter.fields.addressee]
title = "Letter addressee"
type = "string"
examples = ["The Honorable John Smith"]

[scopes.cover_letter.fields.formal_greeting]
title = "Formal greeting"
type = "string"
default = "Dear"
```

**Markdown Usage:**
```markdown
---
QUILL: usaf_memo
subject: Request for Equipment Authorization
memo_for: ["INSTALLATION/CC"]
memo_from: ["SQUADRON/CC", "123 Squadron"]
signature_block: ["JOHN DOE, Lt Col, USAF", "Commander"]
---

# Request Background

This memo requests authorization for...

---
SCOPE: indorsements
from: INSTALLATION/CC
to: SQUADRON/CC
signature_block: ["JANE SMITH, Col, USAF", "Installation Commander"]
new_page: true
---

I approve this request and forward to MAJCOM/CC.

---
SCOPE: indorsements
from: MAJCOM/CC
to: INSTALLATION/CC
signature_block: ["ROBERT JONES, Brig Gen, USAF", "Commander"]
---

Request approved.

---
SCOPE: appendix
title: "Technical Specifications"
classification: "UNCLASSIFIED"
page_break: true
---

## Equipment Details

Detailed technical specifications:
- Model: XYZ-2000
- Cost: $50,000
- Delivery: 90 days

---
SCOPE: cover_letter
addressee: "The Honorable Jane Doe, Secretary of Defense"
formal_greeting: "Dear Madam Secretary"
---

I am pleased to transmit the enclosed memorandum...
```

**Parsed Result:**
```json
{
  "subject": "Request for Equipment Authorization",
  "memo_for": ["INSTALLATION/CC"],
  "memo_from": ["SQUADRON/CC", "123 Squadron"],
  "signature_block": ["JOHN DOE, Lt Col, USAF", "Commander"],
  "body": "# Request Background\n\nThis memo requests authorization for...",

  "indorsements": [
    {
      "from": "INSTALLATION/CC",
      "to": "SQUADRON/CC",
      "signature_block": ["JANE SMITH, Col, USAF", "Installation Commander"],
      "new_page": true,
      "body": "I approve this request and forward to MAJCOM/CC."
    },
    {
      "from": "MAJCOM/CC",
      "to": "INSTALLATION/CC",
      "signature_block": ["ROBERT JONES, Brig Gen, USAF", "Commander"],
      "new_page": false,
      "body": "Request approved."
    }
  ],

  "appendix": {
    "title": "Technical Specifications",
    "classification": "UNCLASSIFIED",
    "page_break": true,
    "body": "## Equipment Details\n\nDetailed technical specifications:\n- Model: XYZ-2000\n- Cost: $50,000\n- Delivery: 90 days"
  },

  "cover_letter": {
    "addressee": "The Honorable Jane Doe, Secretary of Defense",
    "formal_greeting": "Dear Madam Secretary",
    "body": "I am pleased to transmit the enclosed memorandum..."
  }
}
```

---

## JSON Schema Generation

### Collection Scope Output
```toml
[scopes.indorsements]
description = "Endorsements"
```

→

```json
{
  "indorsements": {
    "type": "array",
    "description": "Endorsements",
    "items": {
      "type": "object",
      "properties": {
        "from": { "type": "string" },
        "to": { "type": "string" }
      }
    }
  }
}
```

### Atomic Scope Output
```toml
[scopes.appendix]
singular = true
description = "Appendix section"
```

→

```json
{
  "appendix": {
    "type": "object",
    "description": "Appendix section",
    "properties": {
      "title": { "type": "string", "default": "APPENDIX" },
      "page_break": { "type": "boolean", "default": true }
    }
  }
}
```

**Key Difference:**
- Collection: `"type": "array"` with `"items": {...}`
- Atomic: `"type": "object"` with `"properties": {...}` (no `items`)

---

## Error Handling

### Error 1: Multiple Blocks for Singular Scope

**Markdown:**
```markdown
---
SCOPE: appendix
title: "Appendix A"
---
Content A

---
SCOPE: appendix
title: "Appendix B"
---
Content B
```

**Error:**
```
Parse Error at line 8:
Scope 'appendix' is defined as singular (only one block allowed).
Found 2 blocks at lines 2 and 7.

To fix:
- Remove duplicate SCOPE: appendix blocks
- Or change Quill.toml to make 'appendix' a collection scope (remove 'singular = true')
```

### Error 2: Field Validation (Same for Both Types)

**Markdown:**
```markdown
---
SCOPE: appendix
title: 123  # Wrong type (should be string)
page_break: "yes"  # Wrong type (should be boolean)
---
Content
```

**Error:**
```
Validation Error in SCOPE 'appendix':
- Field 'title': Expected string, got number (123)
- Field 'page_break': Expected boolean, got string ("yes")
```

### Error 3: Unknown Scope (Allowed)

**Markdown:**
```markdown
---
SCOPE: mystery_section
foo: bar
---
Content
```

**Behavior:**
- ✅ No error (lenient mode)
- Treated as collection: `mystery_section = [{foo: "bar", body: "Content"}]`
- No validation applied

---

## Implementation Notes

### Parser Changes

1. **Load `[scopes.*]` from Quill.toml**
   - Track which scopes are `singular = true`
   - Build field schemas for each scope

2. **During Markdown Parsing**
   - Group SCOPE blocks by name
   - For each scope name:
     - If `singular = true`: Check count == 0 or 1, error if > 1
     - If `singular = false` (or not defined): Allow any count
   - Build result:
     - Singular scope → single object (not array)
     - Collection scope → array of objects

3. **Validation**
   - Validate collection items against `scopes.X.fields.*`
   - Validate atomic object against `scopes.X.fields.*`
   - Apply defaults

### JSON Schema Generation

```rust
fn generate_scope_schema(scope_config: &ScopeConfig) -> JsonValue {
    let item_schema = build_object_schema(&scope_config.fields);

    if scope_config.singular {
        // Atomic scope: type = object
        json!({
            "type": "object",
            "description": scope_config.description,
            "properties": item_schema["properties"],
            "x-ui": build_ui_metadata(&scope_config.ui)
        })
    } else {
        // Collection scope: type = array
        json!({
            "type": "array",
            "description": scope_config.description,
            "items": item_schema,
            "x-ui": build_ui_metadata(&scope_config.ui)
        })
    }
}
```

---

## Real-World Use Cases

### Collection Scopes (Array)
- **Endorsements** - Multiple forwarding/response endorsements
- **Products** - Product catalog listings
- **Authors** - Multi-author documents
- **References** - Bibliography entries
- **Action Items** - Meeting action items
- **Sections** - Numbered sections with metadata
- **Exhibits** - Legal exhibits
- **Signatures** - Multiple signature blocks

### Atomic Scopes (Object)
- **Appendix** - Single appendix section
- **Cover Letter** - Transmittal letter
- **Abstract** - Document abstract/summary
- **Executive Summary** - Single executive summary
- **Dedication** - Book dedication
- **Acknowledgments** - Acknowledgments section
- **Glossary** - Terms and definitions
- **Conclusion** - Final conclusions section
- **Distribution** - Distribution list metadata

---

## Migration Path

### Phase 1: Rename `[collections]` → `[scopes]`
- Update proposal to use `[scopes.*]` naming
- Default behavior: collection (array)

### Phase 2: Add `singular` flag support
- Parser recognizes `singular = true`
- Enforce strict cardinality for singular scopes
- Generate correct JSON Schema (object vs array)

### Phase 3: Update existing quills
- Migrate USAF memo to use `[scopes.indorsements]`
- Add atomic scopes (appendix, cover_letter) as examples

---

## Open Questions

1. **Should we support `max_items` for collections?**
   - Use case: Limit endorsements to reasonable number (10-20)?
   - Decision: TBD (not critical for v1)

2. **Should unknown scopes warn in strict mode?**
   - Current: Lenient (allow unknown scopes)
   - Alternative: Add `strict_scopes = true` option to error on unknown?
   - Decision: Keep lenient for v1, add strict mode later if needed

3. **Should `body` be optional for atomic scopes?**
   - Current: `body` always included
   - Use case: Pure metadata scope with no content?
   - Decision: TBD (current behavior is fine for v1)

---

## Summary

**Final Semantics:**
- ✅ Section name: `[scopes.*]`
- ✅ Default: Collection (array) when `singular` not specified
- ✅ Atomic: `singular = true` → single object, strict enforcement
- ✅ Unknown scopes: Allowed (lenient), default to collection
- ✅ No required scopes (all optional)
- ✅ No `min_items` for collections
- ✅ Strict cardinality for singular scopes (error on multiple blocks)

**Next Steps:**
1. Update QUILL_TOML_ENHANCEMENT_PROPOSAL.md with `[scopes]` naming
2. Add atomic scope examples (appendix, cover_letter)
3. Document singular flag and strict enforcement
4. Implement parser support for `singular` detection
5. Update JSON Schema generation
6. Add test cases for singular vs collection scopes
