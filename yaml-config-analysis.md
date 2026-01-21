# Analysis: YAML Configuration vs. TOML vs. JSON Schema

**Question:** What if we used YAML instead of TOML? JSON Schema is powerful but not as easy to configure as tables.

---

## The Sweet Spot Hypothesis

**Observation:** YAML might offer the best balance:
- Better nested structure handling than TOML
- More readable than JSON Schema
- Familiar to web/DevOps developers
- Supports comments (unlike JSON)
- Can still represent JSON Schema concepts

---

## Side-by-Side Comparison

### Example: Resume Template with Nested Structures

#### Current TOML (Verbose Nesting)

```toml
[Quill]
name = "classic_resume"
backend = "typst"
description = "A clean and modern resume template"
plate_file = "plate.typ"
example_file = "example.md"

# Simple field - TOML excels here
[fields.name]
type = "string"
title = "Full Name"
required = true
examples = ["John Doe"]

# Nested structure - TOML gets verbose
[fields.cells]
type = "array"
title = "Skill Categories"
required = true
examples = [[
    { category = "Languages", skills = "Python, Rust" },
    { category = "DevOps", skills = "Docker, Kubernetes" }
]]
# Repetitive dot notation
cells.items.type = "object"
cells.items.properties.category = { type = "string", title = "Category", required = true }
cells.items.properties.skills = { type = "string", title = "Skills", required = true }

# Card definition
[cards.experience_section]
title = "Experience Entry"
description = "Work or education entry"

[cards.experience_section.fields.company]
type = "string"
title = "Company/School"
required = true

[cards.experience_section.fields.dates]
type = "string"
title = "Date Range"
```

**Lines:** ~35 lines for schema
**Readability:** ⭐⭐⭐⭐ (simple fields) / ⭐⭐ (nested)
**Nested structure pain:** High

---

#### YAML Alternative (Natural Nesting)

```yaml
# Quill.yaml
Quill:
  name: classic_resume
  backend: typst
  description: A clean and modern resume template
  plate_file: plate.typ
  example_file: example.md

# Simple field - similar to TOML
fields:
  name:
    type: string
    title: Full Name
    required: true
    examples:
      - John Doe

  # Nested structure - YAML natural nesting
  cells:
    type: array
    title: Skill Categories
    required: true
    examples:
      - - category: Languages
          skills: Python, Rust
        - category: DevOps
          skills: Docker, Kubernetes
    items:
      type: object
      properties:
        category:
          type: string
          title: Category
          required: true
        skills:
          type: string
          title: Skills
          required: true

# Card definition - clean nesting
cards:
  experience_section:
    title: Experience Entry
    description: Work or education entry
    fields:
      company:
        type: string
        title: Company/School
        required: true
      dates:
        type: string
        title: Date Range
```

**Lines:** ~50 lines (more verbose than TOML for simple fields, but scales better)
**Readability:** ⭐⭐⭐⭐ (consistent across simple and nested)
**Nested structure pain:** Low

---

#### JSON Schema (Most Powerful, Least Readable)

```json
{
  "$schema": "https://quillmark.dev/schema/quill-v1.json",
  "quill": {
    "name": "classic_resume",
    "backend": "typst",
    "description": "A clean and modern resume template",
    "plateFile": "plate.typ",
    "exampleFile": "example.md"
  },
  "properties": {
    "name": {
      "type": "string",
      "title": "Full Name",
      "examples": ["John Doe"]
    },
    "cells": {
      "type": "array",
      "title": "Skill Categories",
      "examples": [
        [
          {"category": "Languages", "skills": "Python, Rust"},
          {"category": "DevOps", "skills": "Docker, Kubernetes"}
        ]
      ],
      "items": {
        "type": "object",
        "properties": {
          "category": {
            "type": "string",
            "title": "Category"
          },
          "skills": {
            "type": "string",
            "title": "Skills"
          }
        },
        "required": ["category", "skills"]
      }
    }
  },
  "required": ["name"],
  "cards": {
    "$defs": {
      "experience_section": {
        "title": "Experience Entry",
        "description": "Work or education entry",
        "type": "object",
        "properties": {
          "company": {
            "type": "string",
            "title": "Company/School"
          },
          "dates": {
            "type": "string",
            "title": "Date Range"
          }
        },
        "required": ["company"]
      }
    }
  }
}
```

**Lines:** ~60 lines
**Readability:** ⭐⭐⭐ (many brackets, quotes everywhere)
**Nested structure pain:** None (but verbosity pain)

---

## Detailed Comparison Matrix

| Feature | TOML | YAML | JSON Schema |
|---------|------|------|-------------|
| **Simple flat fields** | ⭐⭐⭐⭐⭐ Excellent | ⭐⭐⭐⭐ Good | ⭐⭐⭐ Verbose |
| **Nested structures** | ⭐⭐ Verbose dots | ⭐⭐⭐⭐⭐ Natural | ⭐⭐⭐⭐⭐ Natural |
| **Comments** | ⭐⭐⭐⭐⭐ Native | ⭐⭐⭐⭐⭐ Native | ⭐ Only $comment |
| **Readability** | ⭐⭐⭐⭐⭐ Very clear | ⭐⭐⭐⭐ Clean | ⭐⭐⭐ Brackets |
| **Indentation** | ⭐⭐⭐⭐⭐ Flexible | ⭐⭐⭐ Required | ⭐⭐⭐⭐ Optional |
| **Learning curve** | ⭐⭐⭐⭐ Shallow | ⭐⭐⭐⭐ Familiar | ⭐⭐⭐ Steeper |
| **Explicitness** | ⭐⭐⭐⭐⭐ Very explicit | ⭐⭐⭐ Implicit types | ⭐⭐⭐⭐⭐ Explicit |
| **IDE support** | ⭐⭐ Limited | ⭐⭐⭐⭐ Good | ⭐⭐⭐⭐⭐ Excellent |
| **Tooling** | ⭐⭐ Custom only | ⭐⭐⭐⭐ Good | ⭐⭐⭐⭐⭐ Rich |
| **Non-dev friendly** | ⭐⭐⭐⭐⭐ Very | ⭐⭐⭐⭐ Yes | ⭐⭐ Not really |
| **Web dev familiar** | ⭐⭐⭐ Rust/Cargo | ⭐⭐⭐⭐⭐ Very | ⭐⭐⭐⭐ APIs |
| **LLM generation** | ⭐⭐⭐⭐ Good | ⭐⭐⭐⭐⭐ Excellent | ⭐⭐⭐⭐⭐ Excellent |
| **Error-prone** | ⭐⭐⭐⭐ Rarely | ⭐⭐⭐ Indentation | ⭐⭐⭐⭐ Rarely |

---

## YAML Advantages Over TOML

### 1. ✅ Natural Nested Structure Handling

**TOML verbose:**
```toml
cells.items.properties.category = { type = "string" }
cells.items.properties.skills = { type = "string" }
```

**YAML natural:**
```yaml
cells:
  items:
    properties:
      category:
        type: string
      skills:
        type: string
```

**Impact:** Much easier to read and write complex schemas

---

### 2. ✅ Better List Handling

**TOML:**
```toml
examples = [["item1", "item2"], ["item3", "item4"]]
```

**YAML:**
```yaml
examples:
  - - item1
    - item2
  - - item3
    - item4
```

Or inline: `examples: [[item1, item2], [item3, item4]]`

**Impact:** Flexibility for readability

---

### 3. ✅ Multi-line Strings

**TOML:**
```toml
description = """
This is a long
multi-line description
that spans several lines.
"""
```

**YAML (multiple styles):**
```yaml
# Literal block (preserves newlines)
description: |
  This is a long
  multi-line description
  that spans several lines.

# Folded block (joins lines)
description: >
  This is a long
  multi-line description
  that becomes one line.
```

**Impact:** More control over formatting

---

### 4. ✅ Web Developer Familiarity

**Ecosystems using YAML:**
- Docker Compose
- Kubernetes
- GitHub Actions
- Ansible
- OpenAPI/Swagger
- Jekyll/Hugo (frontmatter)

**Ecosystems using TOML:**
- Rust (Cargo)
- Python (pyproject.toml)
- Hugo (config, alternative)

**Impact:** Most web developers already know YAML

---

### 5. ✅ JSON Superset

```yaml
# Valid JSON is valid YAML
fields: {
  "name": {
    "type": "string"
  }
}
```

**Impact:** Can embed JSON directly when needed

---

## YAML Disadvantages vs TOML

### 1. ⚠️ Indentation Sensitivity

**TOML (indentation doesn't matter):**
```toml
[fields.name]
type = "string"
    title = "Name"  # Still valid
```

**YAML (indentation matters):**
```yaml
fields:
  name:
    type: string
      title: Name  # ERROR: incorrect indentation
```

**Impact:** More error-prone for hand-editing

---

### 2. ⚠️ Implicit Type Coercion

**TOML explicit:**
```toml
version = "1.0"    # String (quoted)
count = 42         # Number (unquoted)
active = true      # Boolean
```

**YAML implicit:**
```yaml
version: 1.0       # Number! (not string)
version: "1.0"     # String (quoted)
count: 42          # Number
active: true       # Boolean
yes_value: yes     # Boolean true! (surprises)
no_value: no       # Boolean false!
```

**Impact:** Need to be careful with quoting

---

### 3. ⚠️ Security Concerns (Historical)

**Issue:** Some YAML parsers (Python PyYAML) allow arbitrary code execution

**Mitigation:** Rust YAML parsers (serde_yaml, saphyr) are safe by design

**Impact:** Not a concern for Quillmark (using Rust), but worth noting

---

### 4. ⚠️ Multiple Ways to Express Same Thing

**YAML flexibility can confuse:**
```yaml
# All equivalent:
list: [a, b, c]

list:
  - a
  - b
  - c

list: [
  a,
  b,
  c
]
```

**TOML more restrictive:**
```toml
list = ["a", "b", "c"]
```

**Impact:** Consistency harder to enforce

---

## Real-World Example: Complete Resume Schema

### YAML Configuration

```yaml
# Quill.yaml
Quill:
  name: classic_resume
  backend: typst
  description: A clean and modern resume template
  version: 1.0.0
  author: John Doe
  plate_file: plate.typ
  example_file: example.md

backend:
  typst:
    packages:
      - "@preview/bubble:0.2.2"

# Reusable definitions (like JSON Schema $defs)
definitions:
  address:
    type: object
    properties:
      street: { type: string }
      city: { type: string }
      state: { type: string }
      zip: { type: string, pattern: '^[0-9]{5}$' }
    required: [street, city]

fields:
  name:
    type: string
    title: Full Name
    description: Candidate's full name
    required: true
    examples:
      - John Doe
    ui:
      group: Personal Info
      order: 1

  contacts:
    type: array
    title: Contact Information
    items:
      type: string
    minItems: 1
    examples:
      - - john@example.com
        - "(555) 123-4567"
    ui:
      group: Personal Info
      order: 2

  # Reference to reusable definition
  address:
    $ref: '#/definitions/address'
    ui:
      group: Personal Info
      order: 3

cards:
  experience_section:
    title: Experience/Education Entry
    description: An entry with heading, subheading, and bullets
    fields:
      title:
        type: string
        title: Section Title
        default: Experience

      headingLeft:
        type: string
        title: Company/School
        description: Organization name
        required: true

      headingRight:
        type: string
        title: Location

      dates:
        type: string
        title: Date Range
        pattern: '^[A-Za-z]+ [0-9]{4}( – [A-Za-z]+ [0-9]{4})?$'
        examples:
          - January 2020 – Present
          - June 2018 – December 2019

  skills_section:
    title: Skills Grid
    description: Grid of skill categories with key-value pairs
    ui:
      hideBody: true
    fields:
      title:
        type: string
        default: Skills

      cells:
        type: array
        title: Skill Categories
        minItems: 1
        items:
          type: object
          properties:
            category:
              type: string
              title: Category
              required: true
            skills:
              type: string
              title: Skills
              required: true
          required: [category, skills]
```

**Analysis:**
- **95 lines** (vs. ~120 in TOML, ~110 in JSON Schema)
- **Natural nesting** for complex structures
- **Readable** for both simple and complex cases
- **Comments** supported
- **$ref support** for reusability

---

## User Persona Impact

### 👨‍💻 Web Developers
**TOML:** ⭐⭐⭐ "It's okay but verbose"
**YAML:** ⭐⭐⭐⭐⭐ "This is what I use daily (Docker, K8s, CI/CD)"
**JSON Schema:** ⭐⭐⭐⭐ "Powerful but verbose"

### 🔬 Data Scientists
**TOML:** ⭐⭐⭐⭐ "Familiar from Python"
**YAML:** ⭐⭐⭐⭐⭐ "Use this for conda, configs"
**JSON Schema:** ⭐⭐⭐ "JSON is fine"

### ✍️ Content Creators
**TOML:** ⭐⭐⭐⭐ "Simple and clear"
**YAML:** ⭐⭐⭐⭐ "Similar to Jekyll frontmatter"
**JSON Schema:** ⭐⭐ "Too technical"

### 🚀 DevOps Engineers
**TOML:** ⭐⭐⭐ "Use occasionally"
**YAML:** ⭐⭐⭐⭐⭐ "Daily driver (K8s, Ansible, CI)"
**JSON Schema:** ⭐⭐⭐⭐ "For APIs"

---

## Implementation Strategy

### Option 1: Switch to YAML (Breaking Change)

**Migration:**
```bash
quillmark convert Quill.toml --to-yaml > Quill.yaml
```

**Pros:**
- Clean break, one format to support
- Better scalability for complex schemas
- More familiar to majority of users

**Cons:**
- Breaking change for existing templates
- Need migration tooling
- Some users prefer TOML

---

### Option 2: Support Both TOML and YAML

```rust
impl Quill {
    pub fn from_tree(files: FileTreeNode) -> Result<Self, Box<dyn Error>> {
        // Try YAML first
        if let Some(yaml_content) = files.get_file("Quill.yaml") {
            return QuillConfig::from_yaml(yaml_content);
        }

        // Fall back to TOML
        if let Some(toml_content) = files.get_file("Quill.toml") {
            return QuillConfig::from_toml(toml_content);
        }

        Err("No Quill configuration found")
    }
}
```

**Pros:**
- No breaking changes
- Users choose format based on preference
- Gradual migration path

**Cons:**
- Two parsers to maintain
- Documentation complexity

---

### Option 3: Triple Support (TOML + YAML + JSON Schema)

```rust
// Priority order: JSON Schema > YAML > TOML
if let Some(content) = files.get_file("Quill.schema.json") {
    QuillConfig::from_json_schema(content)
} else if let Some(content) = files.get_file("Quill.yaml") {
    QuillConfig::from_yaml(content)
} else if let Some(content) = files.get_file("Quill.toml") {
    QuillConfig::from_toml(content)
}
```

**Pros:**
- Maximum flexibility
- Simple → TOML
- Moderate → YAML
- Complex → JSON Schema

**Cons:**
- Three parsers to maintain
- Most complexity

---

## Recommendation: **YAML as Primary, with TOML Support**

### Strategy:

1. **Add YAML support** (non-breaking)
   - Implement `QuillConfig::from_yaml()`
   - Auto-detect `Quill.yaml` vs `Quill.toml`
   - Full parity with TOML features

2. **Document YAML as preferred** for new templates
   - Show YAML examples first
   - TOML examples second
   - Explain when to use which

3. **Keep TOML indefinitely** (no deprecation)
   - Simple templates can continue using TOML
   - No forced migration
   - Conversion tool available

4. **Add JSON Schema support later** (optional)
   - For power users with very complex schemas
   - After YAML adoption stabilizes

---

## Feature Parity Table

| Feature | TOML Support | YAML Support | JSON Schema |
|---------|--------------|--------------|-------------|
| **Reusable definitions** | ⚠️ Custom | ✅ `$ref` | ✅ `$ref/$defs` |
| **Nested objects** | ⚠️ Verbose | ✅ Natural | ✅ Natural |
| **Validation keywords** | ⚠️ Custom | ✅ Standard | ✅ Full spec |
| **Comments** | ✅ Native | ✅ Native | ⚠️ $comment |
| **Multi-line strings** | ✅ `"""` | ✅ `|` / `>` | ⚠️ Quoted |
| **Type explicitness** | ✅ Very clear | ⚠️ Implicit | ✅ Explicit |
| **Indentation** | ✅ Flexible | ⚠️ Required | ✅ Flexible |

---

## Implementation Effort

### Phase 1: YAML Parser
- **YAML parser integration:** 2-3 days (using `serde_yaml`)
- **Format auto-detection:** 1 day
- **Tests:** 2-3 days
- **Documentation:** 2-3 days
- **Converter tool:** 2 days
- **Total:** ~2 weeks

### Phase 2 (Optional): JSON Schema
- **JSON Schema parser:** 3-5 days
- **$ref resolution:** 2-3 days
- **Tests + docs:** 3-4 days
- **Total:** ~2 weeks

---

## Concrete Example: What Changes

### Before (TOML only)

```toml
# Only option - verbose for nesting
cells.items.properties.category = { type = "string" }
```

### After (YAML preferred)

```yaml
# Recommended for most users
cells:
  items:
    properties:
      category:
        type: string
```

**OR** keep using TOML (still works):

```toml
# Still supported
cells.items.properties.category = { type = "string" }
```

---

## Conclusion

**YAML solves the TOML verbosity problem** while remaining more approachable than JSON Schema:

- ✅ Natural nested structure handling
- ✅ Familiar to web developers
- ✅ Comments and multi-line strings
- ✅ Can support $ref for reusability
- ✅ Less verbose than JSON Schema
- ⚠️ Indentation-sensitive (minor tradeoff)

**Recommendation: Add YAML support, keep TOML**
- YAML becomes **recommended** for new templates
- TOML remains **supported** forever (no breaking change)
- Users choose based on template complexity
- Estimated effort: ~2 weeks

**This gives you the benefits of better nesting without forcing everyone to JSON Schema's verbosity.**
