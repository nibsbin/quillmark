# Quill.toml Configuration Enhancement Proposal
## Support for Scoped Collection Schema Annotations

**Date:** 2025-12-15
**Context:** Enable schema annotation for SCOPE collections (e.g., `indorsements`) in USAF memo quill
**Design Focus:** Ergonomic, scalable, implementation-agnostic

---

## Problem Statement

Currently, Quill.toml supports schema annotation only for main document fields via `[fields.*]` sections. However, many quills use the SCOPE feature to create collections (arrays of structured objects), such as:

- **Endorsements** (`indorsements`) in USAF/USSF memos
- **Products** in catalogs
- **Authors** in multi-author documents
- **Sections** in structured reports

These scoped collections have their own field schemas (e.g., endorsement fields: `from`, `to`, `signature_block`, `new_page`, `date`, `attachments`, `cc`, `informal`), but there's currently no way to:

1. ✗ Define validation rules for collection item fields
2. ✗ Specify default values for collection item fields
3. ✗ Provide UI metadata (groups, ordering) for collection wizards
4. ✗ Generate JSON Schema for collections
5. ✗ Auto-document available fields for collection items

---

## Design Principles

1. **Ergonomic** - Easy to read, write, and maintain
2. **Scalable** - Works for any SCOPE collection type
3. **Consistent** - Follows existing `[fields.*]` patterns
4. **Declarative** - Schema-first, not code-first
5. **Extensible** - Room for future enhancements (nested collections, constraints)
6. **Type-safe** - Full JSON Schema generation support

---

## Design Options

### Option A: Dedicated `[collections]` Section (RECOMMENDED)

**Syntax:**
```toml
[Quill]
name = "usaf_memo"
backend = "typst"
description = "Typesetted USAF Official Memorandum"

# Main document fields (existing)
[fields.subject]
title = "Subject of the memo"
type = "string"
examples = ["Subject of the Memorandum"]
ui.group = "Essentials"
description = "Be brief and clear."

[fields.memo_for]
title = "List of recipient organization(s)"
type = "array"
# ...

# NEW: Collection definitions
[collections.indorsements]
description = "Endorsements (forwarding/response) appended to the memo per AFH 33-337"
ui.group = "Endorsements"
ui.icon = "mail-forward"  # Optional: UI icon hint
ui.add_button_text = "Add Endorsement"  # Optional: Custom button text
ui.item_label = "{{ordinal}} Endorsement"  # Optional: Template for item labels (uses {{ordinal}}, {{index}}, or field values)

# Collection item fields
[collections.indorsements.fields.from]
title = "Sender organization"
type = "string"
examples = ["ORG/SYMBOL"]
ui.group = "Header"
description = "Organization/office symbol of the endorsing authority. Use UPPERCASE per AFH 33-337."

[collections.indorsements.fields.to]
title = "Recipient organization"
type = "string"
examples = ["ORG/SYMBOL"]
ui.group = "Header"
description = "Organization receiving this endorsement (typically the previous endorser or originator)."

[collections.indorsements.fields.signature_block]
title = "Signature block lines"
type = "array"
examples = [["FIRST M. LAST, Rank, USAF", "Duty Title"]]
ui.group = "Signature"
description = "Line 1: Name in UPPERCASE, grade, service. Line 2: Duty title."

[collections.indorsements.fields.new_page]
title = "Start on new page"
type = "boolean"
default = false
ui.group = "Formatting"
description = "Whether to start this endorsement on a new page."

[collections.indorsements.fields.date]
title = "Endorsement date (YYYY-MM-DD)"
type = "date"
default = ""  # Empty string means "use today's date"
ui.group = "Header"
description = "Date of this endorsement. Leave blank to use today's date."

[collections.indorsements.fields.attachments]
title = "List of attachments"
type = "array"
default = []
examples = [["Attachment description, YYYY MMM DD"]]
ui.group = "Routing"
description = "Attachments specific to this endorsement (not the original memo)."

[collections.indorsements.fields.cc]
title = "Carbon copy recipients"
type = "array"
default = []
examples = [["Rank and Name, ORG/SYMBOL"]]
ui.group = "Routing"
description = "Additional recipients to receive copies of this endorsement."

[collections.indorsements.fields.informal]
title = "Informal endorsement"
type = "boolean"
default = false
ui.group = "Formatting"
description = "Use informal format (omits from/to headers). Rarely used."
```

**JSON Schema Output:**
```json
{
  "$schema": "https://json-schema.org/draft/2019-09/schema",
  "type": "object",
  "properties": {
    "subject": {
      "type": "string",
      "title": "Subject of the memo",
      "description": "Be brief and clear.",
      "x-ui": { "group": "Essentials", "order": 1 }
    },
    "indorsements": {
      "type": "array",
      "description": "Endorsements (forwarding/response) appended to the memo per AFH 33-337",
      "x-ui": {
        "group": "Endorsements",
        "icon": "mail-forward",
        "add_button_text": "Add Endorsement",
        "item_label": "{{ordinal}} Endorsement"
      },
      "items": {
        "type": "object",
        "properties": {
          "from": {
            "type": "string",
            "title": "Sender organization",
            "description": "Organization/office symbol of the endorsing authority...",
            "examples": ["ORG/SYMBOL"],
            "x-ui": { "group": "Header", "order": 1 }
          },
          "to": {
            "type": "string",
            "title": "Recipient organization",
            "examples": ["ORG/SYMBOL"],
            "x-ui": { "group": "Header", "order": 2 }
          },
          "signature_block": {
            "type": "array",
            "title": "Signature block lines",
            "examples": [["FIRST M. LAST, Rank, USAF", "Duty Title"]],
            "x-ui": { "group": "Signature", "order": 3 }
          },
          "new_page": {
            "type": "boolean",
            "title": "Start on new page",
            "default": false,
            "x-ui": { "group": "Formatting", "order": 4 }
          },
          "date": {
            "type": "string",
            "format": "date",
            "title": "Endorsement date (YYYY-MM-DD)",
            "default": "",
            "x-ui": { "group": "Header", "order": 5 }
          },
          "attachments": {
            "type": "array",
            "title": "List of attachments",
            "default": [],
            "x-ui": { "group": "Routing", "order": 6 }
          },
          "cc": {
            "type": "array",
            "title": "Carbon copy recipients",
            "default": [],
            "x-ui": { "group": "Routing", "order": 7 }
          },
          "informal": {
            "type": "boolean",
            "title": "Informal endorsement",
            "default": false,
            "x-ui": { "group": "Formatting", "order": 8 }
          }
        },
        "required": ["from", "to", "signature_block"]
      }
    }
  },
  "required": ["subject", "memo_for"],
  "additionalProperties": true
}
```

**Markdown Usage (Unchanged):**
```markdown
---
QUILL: usaf_memo
subject: Request for Equipment Authorization
memo_for: ["INSTALLATION/CC"]
memo_from: ["SQUADRON/CC", "123 Squadron", "123 Main St", "City ST 12345"]
signature_block: ["JOHN DOE, Lt Col, USAF", "Commander"]
---

Request body...

---
SCOPE: indorsements
from: INSTALLATION/CC
to: SQUADRON/CC
signature_block: ["JANE SMITH, Col, USAF", "Installation Commander"]
new_page: true
---

I approve this request and forward to MAJCOM/CC for final approval.

---
SCOPE: indorsements
from: MAJCOM/CC
to: INSTALLATION/CC
signature_block: ["ROBERT JONES, Brig Gen, USAF", "Commander"]
---

Request approved.
```

**Advantages:**
- ✅ Clear separation between main fields and collections
- ✅ Self-documenting: `[collections.*]` signals "this is an array"
- ✅ Supports collection-level metadata (UI hints, description)
- ✅ Mirrors existing `[fields.*]` pattern for consistency
- ✅ Easy to extend with collection-level constraints (min/max items, etc.)
- ✅ Natural mapping to JSON Schema `items.properties`

**Disadvantages:**
- Requires new top-level `[collections]` section
- Slightly more verbose than inline approach

---

### Option B: Inline Array Schema (Alternative)

**Syntax:**
```toml
[fields.indorsements]
type = "array"
description = "Endorsements (forwarding/response) appended to the memo"
ui.group = "Endorsements"

# Define item schema inline
[fields.indorsements.items.from]
title = "Sender organization"
type = "string"
examples = ["ORG/SYMBOL"]

[fields.indorsements.items.to]
title = "Recipient organization"
type = "string"
examples = ["ORG/SYMBOL"]

[fields.indorsements.items.signature_block]
title = "Signature block lines"
type = "array"
examples = [["FIRST M. LAST, Rank, USAF", "Duty Title"]]

[fields.indorsements.items.new_page]
title = "Start on new page"
type = "boolean"
default = false

# ... etc
```

**Advantages:**
- ✅ Everything in `[fields]` section (no new top-level section)
- ✅ Follows JSON Schema `.items.properties` naming convention
- ✅ Compact for simple collections

**Disadvantages:**
- ⚠️ Less discoverable (`[fields.indorsements.items.*]` is verbose)
- ⚠️ Confusing nesting: `fields.X.items.Y` mixes field and schema concepts
- ⚠️ Hard to distinguish "indorsements is an array field" from "indorsements has nested items"
- ⚠️ No clear place for collection-level UI metadata

---

### Option C: Hybrid Approach (Maximum Flexibility)

Allow BOTH patterns:
- Use `[collections.*]` for SCOPE collections (most common case)
- Use `[fields.*.items.*]` for inline array fields (rare case: a main field that's an array of objects)

**Example:**
```toml
# Regular array field (not a SCOPE)
[fields.authors]
type = "array"
description = "Document authors"

[fields.authors.items.name]
type = "string"
title = "Author name"

[fields.authors.items.affiliation]
type = "string"
title = "Institution"

# SCOPE collection
[collections.indorsements]
description = "Endorsements"

[collections.indorsements.fields.from]
type = "string"
# ...
```

**Advantages:**
- ✅ Supports both use cases: inline arrays and SCOPE collections
- ✅ Clearest intent: `[collections]` = SCOPE, `[fields.*.items]` = inline array

**Disadvantages:**
- Two ways to define array schemas (confusing?)
- More complex implementation

---

### Option D: Unified `[schemas]` Section (Most General)

**Syntax:**
```toml
[schemas.Document]
[schemas.Document.fields.subject]
type = "string"
# ...

[schemas.Indorsement]
[schemas.Indorsement.fields.from]
type = "string"
# ...

[schemas.Indorsement.fields.to]
type = "string"
# ...

# Bind schemas to document structure
[fields.indorsements]
type = "array"
items_schema = "Indorsement"  # Reference to [schemas.Indorsement]
```

**Advantages:**
- ✅ Reusable schemas (define once, reference many times)
- ✅ Supports complex nested structures
- ✅ Closest to JSON Schema philosophy

**Disadvantages:**
- ⚠️ Over-engineered for most use cases
- ⚠️ Indirection makes simple cases harder to read
- ⚠️ Requires schema reference resolution

---

## Recommended Design: Option A + Enhancements

**Core:**
- Use `[collections.*]` for SCOPE-based collections
- Keep `[fields.*]` for main document fields
- Mirror field definition syntax for consistency

**Enhancements:**

### 1. Collection-Level Metadata
```toml
[collections.indorsements]
description = "Endorsements per AFH 33-337"
min_items = 0  # Optional: minimum number of items
max_items = 10  # Optional: maximum number of items
ui.group = "Endorsements"
ui.icon = "mail-forward"
ui.add_button_text = "Add Endorsement"
ui.item_label = "{{ordinal}} Endorsement"  # "1st Endorsement", "2nd Endorsement", etc.
ui.collapsible = true  # Optional: UI hint for collapsible items
ui.sortable = false  # Optional: whether items can be reordered in UI
```

### 2. Field Inheritance
```toml
[collections.indorsements]
# Endorsements inherit some fields from main document
inherit_fields = ["classification", "font_size"]  # Optional: fields to inherit from parent

[collections.indorsements.fields.from]
# ...
```

### 3. Conditional Fields
```toml
[collections.indorsements.fields.from]
type = "string"
# ...

[collections.indorsements.fields.to]
type = "string"
# ...

[collections.indorsements.fields.informal]
type = "boolean"
default = false

# This field only appears if informal = false
[collections.indorsements.fields.from]
ui.visible_when = "informal == false"

[collections.indorsements.fields.to]
ui.visible_when = "informal == false"
```

### 4. Validation Constraints
```toml
[collections.indorsements.fields.from]
type = "string"
pattern = "^[A-Z0-9/]+$"  # Regex validation
min_length = 1
max_length = 100

[collections.indorsements.fields.date]
type = "date"
minimum = "2020-01-01"  # Can't be before this date
maximum = "2030-12-31"
```

### 5. Nested Collections (Future)
```toml
[collections.sections]
description = "Document sections"

[collections.sections.fields.title]
type = "string"

# Nested collection: each section can have subsections
[collections.sections.collections.subsections]
description = "Subsections within a section"

[collections.sections.collections.subsections.fields.title]
type = "string"
```

---

## Implementation Considerations

(Note: User requested we don't worry about implementation difficulty, but documenting for completeness)

### Parsing & Validation
1. Parse `[collections.*]` sections from Quill.toml
2. Build JSON Schema with `items` property for each collection
3. Validate markdown SCOPE blocks against collection schemas
4. Apply defaults to collection items
5. Generate UI metadata for collection wizards

### Backward Compatibility
- Existing `[fields.*]` sections unchanged
- New `[collections.*]` sections are additive
- SCOPE feature continues to work without schema annotations

### Migration Path
1. **Phase 1:** Add `[collections]` support, keep existing behavior
2. **Phase 2:** Update existing quills to use new syntax (optional)
3. **Phase 3:** Add advanced features (inheritance, conditionals, etc.)

---

## Example: Complete USAF Memo Quill.toml

```toml
[Quill]
name = "usaf_memo"
backend = "typst"
plate_file = "plate.typ"
example_file = "usaf_memo.md"
description = "Typesetted USAF Official Memorandum per AFH 33-337"
version = "2.0.0"
author = "nibsbin"

# ===========================================
# MAIN DOCUMENT FIELDS
# ===========================================

[fields.memo_for]
title = "Recipient organization(s)"
type = "array"
examples = [["ORG1/SYMBOL", "ORG2/SYMBOL"]]
ui.group = "Essentials"
description = "Organization/office symbol in UPPERCASE. To address a specific person, add rank and name in parentheses."

[fields.memo_from]
title = "Sender information"
type = "array"
examples = [["ORG/SYMBOL", "Organization Name", "123 Street", "City ST 12345"]]
ui.group = "Essentials"
description = "Office symbol and optional mailing address."

[fields.subject]
title = "Subject line"
type = "string"
examples = ["Subject of the Memorandum"]
ui.group = "Essentials"
description = "Brief, clear subject. Capitalize first letter of each major word."

[fields.signature_block]
title = "Signature block"
type = "array"
examples = [["FIRST M. LAST, Rank, USAF", "Duty Title"]]
ui.group = "Essentials"
description = "Line 1: Name (UPPERCASE), grade, service. Line 2: Duty title."

[fields.date]
title = "Memo date (YYYY-MM-DD)"
type = "date"
default = ""
ui.group = "Advanced"
description = "Date of memo. Leave blank for today's date."

[fields.classification]
title = "Classification marking"
type = "string"
default = ""
examples = ["CONFIDENTIAL", "SECRET"]
ui.group = "Advanced"
description = "Classification level displayed in banner. Leave blank for unclassified."

# ... (other fields: letterhead_title, letterhead_caption, tag_line, references, cc, distribution, attachments, font_size)

# ===========================================
# COLLECTIONS (SCOPE-based)
# ===========================================

[collections.indorsements]
description = "Endorsements (1st Ind, 2d Ind, etc.) per AFH 33-337 guidance on forwarding and responding to official memorandums"
ui.group = "Endorsements"
ui.icon = "mail-forward"
ui.add_button_text = "Add Endorsement"
ui.item_label = "{{ordinal}} Indorsement"
ui.help_text = "Add endorsements to forward or respond to the original memo. Each endorsement is numbered sequentially (1st Ind, 2d Ind, 3d Ind, etc.)."

[collections.indorsements.fields.from]
title = "Endorsing organization"
type = "string"
examples = ["HQ USAF/A1", "MAJCOM/CC"]
ui.group = "Header"
description = "Organization/office symbol of the endorsing authority. Use UPPERCASE per AFH 33-337."

[collections.indorsements.fields.to]
title = "Recipient organization"
type = "string"
examples = ["SQUADRON/CC", "INSTALLATION/CV"]
ui.group = "Header"
description = "Organization receiving this endorsement (typically the previous endorser or memo originator)."

[collections.indorsements.fields.signature_block]
title = "Signature block"
type = "array"
examples = [["JANE DOE, Col, USAF", "Installation Commander"]]
ui.group = "Signature"
description = "Line 1: Name (UPPERCASE), grade, service. Line 2: Duty title. Spell out 'Colonel' and general officer ranks."

[collections.indorsements.fields.date]
title = "Endorsement date (YYYY-MM-DD)"
type = "date"
default = ""
ui.group = "Header"
description = "Date of this endorsement. Leave blank to use today's date."

[collections.indorsements.fields.new_page]
title = "Start on new page"
type = "boolean"
default = false
ui.group = "Formatting"
description = "Whether to start this endorsement on a new page (recommended for lengthy endorsements)."

[collections.indorsements.fields.attachments]
title = "Attachments"
type = "array"
default = []
examples = [["Analysis Report, 2024 Jan 15"]]
ui.group = "Routing"
description = "Attachments specific to this endorsement (separate from original memo attachments)."

[collections.indorsements.fields.cc]
title = "Carbon copy recipients"
type = "array"
default = []
examples = [["Col John Smith, ORG/CC"]]
ui.group = "Routing"
description = "Additional recipients to receive copies of this endorsement."

[collections.indorsements.fields.informal]
title = "Use informal format"
type = "boolean"
default = false
ui.group = "Formatting"
description = "Informal endorsements omit from/to headers. Rarely used; consult AFH 33-337 before enabling."

# ===========================================
# BACKEND CONFIGURATION
# ===========================================

[typst]
packages = ["@preview/tonguetoquill-usaf-memo:2.0.0"]
```

---

## Comparison Matrix

| Feature | Current | Option A (Collections) | Option B (Inline) | Option C (Hybrid) | Option D (Schemas) |
|---------|---------|----------------------|-------------------|-------------------|-------------------|
| **Ergonomics** | N/A | ⭐⭐⭐⭐⭐ Excellent | ⭐⭐⭐ Good | ⭐⭐⭐ Good | ⭐⭐ Fair |
| **Clarity** | N/A | ⭐⭐⭐⭐⭐ Very clear | ⭐⭐⭐ Moderate | ⭐⭐⭐⭐ Clear | ⭐⭐ Confusing |
| **Scalability** | N/A | ⭐⭐⭐⭐⭐ Excellent | ⭐⭐⭐ Good | ⭐⭐⭐⭐⭐ Excellent | ⭐⭐⭐⭐⭐ Excellent |
| **Consistency** | N/A | ⭐⭐⭐⭐⭐ Mirrors fields | ⭐⭐⭐⭐ Extends fields | ⭐⭐⭐ Two patterns | ⭐⭐ New pattern |
| **UI Support** | N/A | ⭐⭐⭐⭐⭐ Rich metadata | ⭐⭐⭐ Basic | ⭐⭐⭐⭐⭐ Rich | ⭐⭐⭐⭐ Good |
| **Extensibility** | N/A | ⭐⭐⭐⭐⭐ Easy to extend | ⭐⭐⭐ Limited | ⭐⭐⭐⭐ Flexible | ⭐⭐⭐⭐⭐ Very flexible |
| **Learning Curve** | N/A | ⭐⭐⭐⭐⭐ Easy | ⭐⭐⭐⭐ Easy | ⭐⭐⭐ Moderate | ⭐⭐ Steep |
| **Implementation** | N/A | ⭐⭐⭐ Moderate | ⭐⭐⭐⭐ Easy | ⭐⭐ Complex | ⭐ Very complex |

---

## Recommendation

**Use Option A (`[collections.*]` section) with enhancements.**

**Rationale:**
1. ✅ **Most ergonomic** - Clear, readable, self-documenting
2. ✅ **Consistent** - Mirrors existing `[fields.*]` pattern
3. ✅ **Scalable** - Handles simple and complex collections
4. ✅ **UI-friendly** - Rich metadata for wizard generation
5. ✅ **Extensible** - Easy to add constraints, validation, inheritance
6. ✅ **Separation of concerns** - Main fields vs. collections clearly distinguished

**Next Steps:**
1. Validate design with stakeholders
2. Update SCHEMAS.md with new specification
3. Implement parser for `[collections]` section
4. Update JSON Schema generation
5. Migrate USAF memo quill as reference implementation
6. Add documentation and examples

---

## Appendix: Real-World Examples

### Example 1: Product Catalog
```toml
[collections.products]
description = "Product listings"
ui.group = "Products"
ui.add_button_text = "Add Product"
ui.item_label = "{{name}}"

[collections.products.fields.name]
type = "string"
title = "Product name"

[collections.products.fields.price]
type = "number"
title = "Price (USD)"

[collections.products.fields.in_stock]
type = "boolean"
default = true
```

### Example 2: Multi-Author Paper
```toml
[collections.authors]
description = "Document authors"
min_items = 1
ui.group = "Authors"
ui.item_label = "Author {{index}}: {{name}}"

[collections.authors.fields.name]
type = "string"
title = "Full name"

[collections.authors.fields.affiliation]
type = "string"
title = "Institution"

[collections.authors.fields.email]
type = "string"
title = "Email"
pattern = "^[^@]+@[^@]+\\.[^@]+$"
```

### Example 3: Meeting Minutes with Action Items
```toml
[collections.action_items]
description = "Action items from the meeting"
ui.group = "Action Items"
ui.add_button_text = "Add Action Item"
ui.sortable = true

[collections.action_items.fields.task]
type = "string"
title = "Task description"

[collections.action_items.fields.owner]
type = "string"
title = "Responsible person"

[collections.action_items.fields.due_date]
type = "date"
title = "Due date"

[collections.action_items.fields.status]
type = "string"
title = "Status"
default = "pending"
examples = ["pending", "in_progress", "completed"]
```

---

## Questions & Feedback

Please review and provide feedback on:
1. Is `[collections.*]` the right naming? Alternatives: `[scopes.*]`, `[arrays.*]`, `[lists.*]`
2. Should collection-level metadata be extensible (custom `x-*` properties)?
3. Are conditional fields (`ui.visible_when`) necessary for v1?
4. Should we support nested collections in v1 or defer to v2?
5. Any additional UI metadata needed for collection wizards?
