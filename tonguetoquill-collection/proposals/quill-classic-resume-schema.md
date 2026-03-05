# Proposal: classic_resume Quill Schema Redesign

## Summary

Redesign the `skills` and `certifications` cards to use BODY-centric content with markdown structure (headers for categories, bullets for lists), replacing the rigid `key1/val1` field pattern.

---

## Current State

### Skills Card (Quill.toml)

```toml
[cards.skills]
title = "Skills Grid (4 categories)"
ui.metadata_only = true

[cards.skills.fields.key1]
type = "string"
required = true
# ... repeated for key2, key3, key4, val1, val2, val3, val4
```

### Skills Card (Markdown)

```yaml
---
CARD: skills
key1: Programming
val1: Python, R, JS
key2: Data Science
val2: ML/statistics, TensorFlow
key3: IT & Cybersecurity
val3: AD DS, Splunk, Metasploit
key4: Cloud
val4: AWS EC2/S3, Helm, Docker
---
```

### Problems

1. **Rigid** - Exactly 4 categories, no more, no less
2. **Repetitive** - `key1/val1` pattern violates DRY
3. **No rich content** - Values are plain strings, no links/emphasis
4. **Unnatural** - Doesn't feel like writing markdown
5. **`ui.metadata_only = true`** - BODY is wasted

---

## Proposed Design

### New `grid` Card

A unified card that uses BODY content with markdown structure to determine rendering.

#### Categorized Grid (Skills)

```yaml
---
CARD: grid
columns: 2
---
# Programming
Python, R, JS, C#, Rust, PowerShell, CI/CD

# Data Science
ML/statistics, TensorFlow, AI Engineering

# IT & Cybersecurity
AD DS, Splunk, Metasploit, Wireshark, Nessus

# Cloud
AWS EC2/S3, Helm, Docker, Serverless
```

**Parsing logic:**
- `# Header` → category key (bold in output)
- Following paragraph(s) → category value
- `columns` metadata controls grid layout

#### Flat Grid (Certifications)

```yaml
---
CARD: grid
columns: 2
---
- Offensive Security Certified Professional (OSCP)
- GIAC Cyber Threat Intelligence (GCTI)
- CompTIA CASP+, CySA+, Sec+, Net+, A+, Proj+
- GIAC Machine Learning Engineer (GMLE)
```

**Parsing logic:**
- Bullet list → flat items
- Each bullet becomes a grid cell
- `columns` metadata controls grid layout

#### Auto-Detection

The Quillmark engine inspects BODY structure:

| BODY starts with | Detected style | Rendering |
|------------------|----------------|-----------|
| `# Header` | categorized | Bold key + value below |
| `- Bullet` | flat | Simple grid cells |
| Paragraph | flat | Each paragraph = cell |

---

## Schema Changes

### File: `Quill.toml`

```toml
# ==========================================
# REMOVE: Old skills and certifications cards
# ==========================================

# [cards.skills]        # DELETED
# [cards.certifications] # DELETED

# ==========================================
# ADD: New unified grid card
# ==========================================

[cards.grid]
title = "Content Grid"
description = """
A flexible grid layout for skills, certifications, or any structured content.
Use markdown headers (# Category) for key-value pairs, or bullets (- Item) for flat lists.
"""

[cards.grid.fields.columns]
title = "Number of Columns"
type = "integer"
default = 2
examples = [2, 3, 4]
description = "How many columns to display in the grid"

[cards.grid.fields.style]
title = "Grid Style"
type = "string"
default = "auto"
examples = ["auto", "categorized", "flat"]
description = "Override auto-detection: 'categorized' for key-value, 'flat' for simple list"
```

### Backwards Compatibility (Optional)

Keep old cards as aliases during migration:

```toml
[cards.skills]
title = "Skills Grid"
deprecated = true
alias = "grid"
description = "DEPRECATED: Use 'grid' card with # headers instead"

[cards.certifications]
title = "Certifications"
deprecated = true
alias = "grid"
description = "DEPRECATED: Use 'grid' card with - bullets instead"
```

---

## Plate Changes

### File: `plate.typ`

**Before:**

```typst
{% if card.CARD == "skills" %}
  #category_grid(items: (
    (key: {{ card.key1 | String }}, value: {{ card.val1 | String }}),
    (key: {{ card.key2 | String }}, value: {{ card.val2 | String }}),
    (key: {{ card.key3 | String }}, value: {{ card.val3 | String }}),
    (key: {{ card.key4 | String }}, value: {{ card.val4 | String }}),
  ))
{% elif card.CARD == "certifications" %}
  #item_grid(items: {{ card.items | Lines }})
{% endif %}
```

**After:**

```typst
{% if card.CARD == "grid" %}
  #content_grid(
    items: {{ card.BODY | GridItems }},
    columns: {{ card.columns | Int | default: 2 }},
    style: {{ card.style | String | default: "auto" }},
  )
{% endif %}
```

### New Filter: `GridItems`

The Quillmark engine needs a new filter that:

1. Parses BODY markdown
2. Detects structure (headers vs bullets)
3. Returns Typst-compatible array

**Pseudocode:**

```python
def grid_items_filter(body_markdown: str) -> str:
    """Convert markdown BODY to Typst array literal."""

    parsed = parse_markdown(body_markdown)

    if starts_with_header(parsed):
        # Categorized: extract header/content pairs
        items = []
        for section in extract_sections(parsed):
            items.append({
                "key": section.header,
                "value": section.content
            })
        return to_typst_array(items, style="dict")

    elif starts_with_bullet(parsed):
        # Flat: extract bullet items
        items = [item.text for item in extract_bullets(parsed)]
        return to_typst_array(items, style="string")

    else:
        # Paragraphs: each paragraph is an item
        items = [p.text for p in extract_paragraphs(parsed)]
        return to_typst_array(items, style="string")
```

**Output examples:**

```typst
// Categorized (from headers)
((key: "Programming", value: [Python, R, JS]), (key: "Data Science", value: [ML, TensorFlow]))

// Flat (from bullets)
("OSCP", "GCTI", "CompTIA CASP+")
```

---

## Example Migration

### Before (Old Schema)

```markdown
---
CARD: section
title: Skills
---

---
CARD: skills
key1: Programming
val1: Python, R, JS, C#, Rust, PowerShell, CI/CD
key2: Data Science
val2: ML/statistics, TensorFlow, AI Engineering
key3: IT & Cybersecurity
val3: AD DS, Splunk, Metasploit, Wireshark, Nessus
key4: Cloud
val4: AWS EC2/S3, Helm, Docker, Serverless
---

---
CARD: section
title: Certifications
---

---
CARD: certifications
items:
  - Offensive Security Certified Professional (OSCP)
  - GIAC Cyber Threat Intelligence (GCTI)
---
```

### After (New Schema)

```markdown
---
CARD: section
title: Skills
---

---
CARD: grid
columns: 2
---
# Programming
Python, R, JS, C#, Rust, PowerShell, CI/CD

# Data Science
ML/statistics, TensorFlow, AI Engineering

# IT & Cybersecurity
AD DS, Splunk, Metasploit, Wireshark, Nessus

# Cloud
AWS EC2/S3, Helm, Docker, Serverless

---
CARD: section
title: Certifications
---

---
CARD: grid
columns: 2
---
- Offensive Security Certified Professional (OSCP)
- GIAC Cyber Threat Intelligence (GCTI)
- CompTIA CASP+, CySA+, Sec+, Net+, A+, Proj+
- GIAC Machine Learning Engineer (GMLE)
```

---

## Rich Content Examples

The new design enables rich markdown in values:

```markdown
---
CARD: grid
---
# Programming
Python, **R**, JavaScript, [Rust](https://rust-lang.org), PowerShell

# Certifications
[OSCP](https://offensive-security.com), _in progress:_ OSCE3
```

---

## Consumer Impact Analysis

### Human Authors

| Aspect | Before | After |
|--------|--------|-------|
| Cognitive load | High (remember key1/val1 pattern) | Low (use familiar markdown) |
| Flexibility | Fixed 4 categories | Unlimited categories |
| Rich content | Not supported | Full markdown support |
| Readability | Poor (YAML soup) | Good (natural prose) |

### GUI Form Builders

| Aspect | Before | After |
|--------|--------|-------|
| Field generation | Easy (8 text inputs) | Medium (markdown editor) |
| Validation | Strong (required fields) | Weak (freeform) |
| Preview | N/A | Render markdown preview |

**Recommendation:** GUI could offer structured input mode (dynamic rows) that generates markdown, or a markdown editor with syntax highlighting.

### LLM Consumers

| Aspect | Before | After |
|--------|--------|-------|
| Token efficiency | Poor (verbose YAML) | Good (concise markdown) |
| Generation quality | Medium (must follow schema) | High (natural markdown) |
| Understanding | Good (explicit fields) | Good (semantic headers) |

---

## Implementation Phases

### Phase 1: Add New Card
- Add `grid` card to Quill.toml
- Implement `GridItems` filter in Quillmark
- Add `content_grid` to ttq-classic-resume
- Update plate.typ with new card handling
- Update example.md with new syntax

### Phase 2: Deprecation
- Mark `skills` and `certifications` as deprecated
- Add migration documentation
- Log warnings when deprecated cards used

### Phase 3: Removal
- Remove deprecated cards from schema
- Remove old plate.typ conditionals
- Update all examples and documentation

---

## Open Questions

1. **Should `grid` support a `title` field?** To optionally include a section header, reducing card count:
   ```yaml
   ---
   CARD: grid
   title: Skills
   columns: 2
   ---
   ```

2. **What about 1-column "list" rendering?** Should `columns: 1` render as a styled list instead of single-column grid?

3. **Should we support `## Subheaders`?** For nested categorization:
   ```markdown
   # Programming
   ## Languages
   Python, Rust, Go
   ## Frameworks
   React, FastAPI
   ```

4. **Error handling for malformed BODY?** What if user mixes headers and bullets? Fail or best-effort?
