# Quillmark Architectural Ergonomics Assessment

**Date:** 2026-01-20
**Focus:** Architectural pain points (excluding documentation and tooling)
**Scope:** Quill configuration (Quill.toml) and markdown block/card parsing

---

## Executive Summary

Quillmark's architecture demonstrates **sound engineering principles** with clear separation of concerns, type safety, and composability. However, several **architectural constraints** create ergonomic friction that cannot be solved through better documentation or tooling alone.

**Key Issues:**
1. TOML's verbosity for nested structures (inherent format limitation)
2. UPPERCASE reserved keywords causing namespace pollution
3. Asymmetric first-block semantics creating complexity
4. Overly restrictive tag naming conventions
5. Missing reusability mechanisms (field schemas, card defaults)

**Impact:** Moderate friction for power users, significant onboarding barrier for newcomers.

---

## 1. Architectural Pain Points: Quill Configuration

### 1.1 TOML Nested Structure Verbosity ⚠️ **High Impact**

**Issue:** Defining nested structures in TOML requires verbose dot-notation syntax that obscures the actual data structure.

**Example from classic_resume/Quill.toml:**
```toml
# To define: cells: array of objects with {category: string, skills: string}
# Requires 8 lines of TOML:

cells.title = "Skill Categories"
cells.type = "array"
cells.required = true
cells.examples = [[
    { category = "Languages", skills = "Python, Rust, C++" },
    { category = "DevOps", skills = "Docker, Kubernetes" },
]]
cells.items.type = "object"
cells.items.properties.category = { type = "string", title = "Category", required = true }
cells.items.properties.skills = { type = "string", title = "Skills", required = true }
```

**Desired structure (conceptual):**
```toml
[fields.cells]
type = "array"
title = "Skill Categories"
required = true
examples = [[...]]

  [fields.cells.items]
  type = "object"

    [fields.cells.items.properties.category]
    type = "string"
    title = "Category"
    required = true

    [fields.cells.items.properties.skills]
    type = "string"
    title = "Skills"
    required = true
```

**Why this is architectural:**
- TOML's table/dotted-key semantics force this verbosity
- The parser (`QuillConfig::from_toml()`) expects this structure
- Cannot be fixed with documentation - it's a TOML format constraint

**Workarounds considered:**
1. Switch to YAML (loses TOML's advantages: comments, section clarity, type clarity)
2. Inline JSON for complex fields (breaks consistency)
3. External JSON Schema files (adds complexity, multiple file coordination)

**Impact:**
- **Users:** Frustrating for complex schemas (resumes, forms, grids)
- **Template authors:** Copy-paste errors common, hard to visualize structure
- **LLM generation:** Works but generates verbose TOML

**Recommendation:**
- Accept TOML limitation OR provide alternative schema format for complex cases
- Consider allowing inline JSON for specific field definitions:
  ```toml
  [fields.cells]
  schema = '''
  {
    "type": "array",
    "items": {
      "type": "object",
      "properties": {
        "category": {"type": "string", "title": "Category"},
        "skills": {"type": "string", "title": "Skills"}
      }
    }
  }
  '''
  ```

---

### 1.2 Reserved Keyword Namespace Pollution ⚠️ **Medium Impact**

**Issue:** Four UPPERCASE keywords (`QUILL`, `CARD`, `BODY`, `CARDS`) are reserved and hardcoded in the parser, creating potential conflicts with domain-specific terminology.

**Source:** `crates/core/src/parse.rs:385-395`
```rust
const RESERVED_FIELDS: &[&str] = &["BODY", "CARDS"];
for reserved in RESERVED_FIELDS {
    if mapping.contains_key(*reserved) {
        return Err(ParseError::InvalidStructure(
            format!("Reserved field name '{}' cannot be used", reserved)
        ));
    }
}
```

**Conflict scenarios:**
1. **Military documents:** Alert levels might use `BODY` (e.g., "BODY: Main Unit")
2. **Card games:** Documentation might use `CARDS` as a field
3. **Medical documents:** `BODY` as anatomical reference
4. **Publishing:** `CARD` for index cards or card stock type

**Why this is architectural:**
- Keywords are hardcoded in parsing logic
- `BODY` and `CARDS` are automatically injected into every parsed document
- No escape mechanism exists for domain conflicts

**Current workaround:** Users must rename their domain fields (e.g., `BODY` → `body_part`)

**Recommendation:**
1. **Namespace reserved keywords** (breaking change):
   ```
   __QUILL__, __CARD__, __BODY__, __CARDS__
   ```
   Reduces collision probability significantly.

2. **Document all reserved keywords** in a single authoritative location (not architectural, but critical)

3. **Allow opt-out** via configuration:
   ```toml
   [Quill]
   reserved_prefix = "__"  # Reserved fields become __BODY__, __CARDS__
   ```

**Impact:**
- **Current:** Low frequency but high frustration when encountered
- **Future:** Increases as domain coverage expands

---

### 1.3 No Field Schema Reusability ⚠️ **Medium Impact**

**Issue:** Cannot define reusable field schemas or create inheritance hierarchies. Forces copy-paste for similar structures.

**Example use case:**
```toml
# Want to reuse "address" schema across multiple fields
# Currently requires copy-paste:

[fields.shipping_address]
type = "object"
properties.street = { type = "string", required = true }
properties.city = { type = "string", required = true }
properties.state = { type = "string" }
properties.zip = { type = "string" }

[fields.billing_address]  # Copy-paste entire structure
type = "object"
properties.street = { type = "string", required = true }
properties.city = { type = "string", required = true }
properties.state = { type = "string" }
properties.zip = { type = "string" }
```

**Desired syntax:**
```toml
# Define reusable schemas
[schemas.address]
type = "object"
properties.street = { type = "string", required = true }
properties.city = { type = "string", required = true }
properties.state = { type = "string" }
properties.zip = { type = "string" }

# Reference in fields
[fields.shipping_address]
$ref = "address"

[fields.billing_address]
$ref = "address"
extends.properties.delivery_instructions = { type = "string" }
```

**Why this is architectural:**
- No `$ref` or `extends` mechanism in the schema builder (`schema.rs:build_field_property`)
- No schema registry or lookup mechanism
- Current design is flat - each field is independent

**Impact:**
- Large templates (50+ fields) have significant duplication
- Changes to common structures require multi-file edits
- Error-prone copy-paste maintenance

**Recommendation:**
Add JSON Schema `$defs` support:
```toml
[Quill.schema_defs.address]
type = "object"
properties.street = { type = "string" }
# ...

[fields.shipping_address]
type = "$ref:address"
```

Implementation: Resolve `$ref:name` to `#/$defs/name` in `build_field_property()`.

---

### 1.4 Strict Tag Naming Restrictions ⚠️ **Low-Medium Impact**

**Issue:** Card and quill names must match `[a-z_][a-z0-9_]*` - lowercase letters, digits, underscores only. No uppercase, hyphens, or Unicode.

**Source:** `crates/core/src/parse.rs:169-189`
```rust
fn is_valid_tag_name(name: &str) -> bool {
    // Must start with lowercase letter or underscore
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() && first != '_' { return false; }

    // Rest must be lowercase, digits, or underscores
    for ch in chars {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '_' {
            return false;
        }
    }
    true
}
```

**Rejected valid names:**
- `experienceSection` (camelCase)
- `experience-section` (kebab-case)
- `Experience` (PascalCase)
- `résumé` (Unicode)
- `proj_v2.1` (period)

**Why this restriction exists:**
- Ensures safe variable names in template languages (Typst, Jinja)
- Prevents parsing ambiguities
- Consistent with Rust/Python identifier rules

**Why this is architectural:**
- Validation is hardcoded in parser
- Template rendering assumes these naming rules
- Changing would require coordination with all backends

**Impact:**
- Newcomers from JavaScript/TypeScript expect camelCase
- kebab-case is idiomatic in YAML/TOML communities
- Creates naming convention friction

**Recommendation:**
1. **Accept restriction** but make it explicit in error messages:
   ```
   Error: Invalid card name 'experienceSection'
   Card names must match pattern: [a-z_][a-z0-9_]*
   Did you mean: experience_section?
   ```

2. **Allow backend-specific relaxation:**
   ```toml
   [Quill]
   backend = "typst"
   naming_style = "snake_case"  # or "camel_case" if backend supports it
   ```

**Priority:** Low - restriction is defensible, mainly needs better communication.

---

## 2. Architectural Pain Points: Markdown Block/Card System

### 2.1 Asymmetric First-Block Semantics ⚠️ **High Impact**

**Issue:** The first metadata block has different parsing rules than subsequent blocks, creating complexity and confusion.

**Rules:**
1. **First block** (index 0):
   - CAN have `QUILL:` directive
   - CAN be global frontmatter (no `QUILL:`, no `CARD:`)
   - CAN be a card (has `CARD:`)

2. **Subsequent blocks** (index > 0):
   - MUST have `CARD:` directive
   - CANNOT have `QUILL:` directive (error)
   - CANNOT be global frontmatter (error)

**Source:** `crates/core/src/parse.rs:532-553`
```rust
for (idx, block) in blocks.iter().enumerate() {
    if idx == 0 {
        // Top-level frontmatter: can have QUILL or neither
        if block.tag.is_none() && block.quill_name.is_none() {
            global_frontmatter_index = Some(idx);
        }
    } else {
        // Inline blocks: MUST have CARD
        if block.quill_name.is_some() {
            return Err("QUILL directive can only appear in top-level frontmatter");
        }
        if block.tag.is_none() {
            return Err(missing_card_directive());
        }
    }
}
```

**Example confusion:**
```markdown
---
title: My Document
---

Some content.

---
author: John Doe    # ❌ ERROR: Missing CARD directive
---
```

User expectation: "Second block is also global frontmatter, just like the first."
Reality: "Second block MUST be a card or error."

**Why this is architectural:**
- Design decision: global frontmatter lives in a single top block
- Parsing logic enforces this rule
- Cannot be changed without redefining the document model

**Impact:**
- **Newcomers:** Frequently make this mistake
- **Migrations:** Documents from Jekyll/Hugo have multiple frontmatter blocks
- **Authoring flow:** Adds cognitive load - "Is this the first block?"

**Recommendation:**

**Option A:** Allow multiple global frontmatter blocks (merge them):
```rust
// Parse ALL blocks without CARD: as global frontmatter
// Merge into single fields map
// Error on field name collisions
```

**Option B:** Require explicit `FRONTMATTER:` directive:
```markdown
---
FRONTMATTER:
title: My Document
---

---
CARD: section
---
```
Makes the special first-block behavior explicit.

**Option C:** Document-level vs inline distinction:
```markdown
===
DOCUMENT:
title: Global metadata
===

---
CARD: section
---
```
Use different delimiters (`===` vs `---`) for document vs. cards.

**Preferred:** Option A - least breaking, most intuitive.

---

### 2.2 Horizontal Rule Ambiguity ⚠️ **Medium Impact**

**Issue:** The delimiter `---` serves dual purpose (metadata block AND horizontal rule), requiring complex heuristics to disambiguate.

**Current logic:** `crates/core/src/parse.rs:283-304`
```rust
// Horizontal rule: blank lines both above AND below
if preceded_by_blank && followed_by_blank {
    // This is <hr>, skip it
    continue;
}

// Otherwise, it's a metadata block opening
```

**Ambiguous cases:**
```markdown
First paragraph.
---                  # ← Not preceded by blank line
                     # ← Followed by blank line
Second paragraph.
```

Is this a horizontal rule? A failed metadata block? Body content?

**Current behavior:** `---` followed by blank but NOT preceded by blank is **skipped** (treated as invalid metadata block opening, becomes body content).

**Why this is architectural:**
- Markdown already uses `---` for horizontal rules
- Quillmark uses `---` for metadata delimiters
- Parser must choose between interpretations

**Impact:**
- Users familiar with Markdown expect `---` = horizontal rule
- Edge case behavior is surprising: "Why was my `---` ignored?"
- Documentation can explain, but doesn't remove ambiguity

**Alternatives considered:**

1. **Different delimiter for metadata:**
   ```markdown
   +++
   CARD: section
   +++
   ```
   Hugo uses `+++` for TOML frontmatter.

2. **Require keyword in metadata blocks:**
   ```markdown
   ---metadata
   CARD: section
   ---
   ```

3. **Use fenced code block syntax:**
   ~~~markdown
   ```metadata
   CARD: section
   ```
   ~~~

4. **Current approach:** Context-dependent parsing (blank line detection)

**Recommendation:**
- **Accept current behavior** - it works for 95% of cases
- **Document edge cases** prominently with examples
- **Improve error messages** when ambiguous `---` is detected:
  ```
  Warning: Found '---' that could be metadata or horizontal rule.
  If you want a horizontal rule, ensure blank lines both above AND below.
  If you want metadata, ensure content immediately follows the opening '---'.
  ```

**Priority:** Medium - Works but requires user awareness.

---

### 2.3 No Card-Level Default Values ⚠️ **Medium Impact**

**Issue:** When multiple cards share common field values, users must repeat them in every card block. No mechanism for card-level defaults.

**Example:** Resume with 10 work experiences all under "Work Experience" section:

```markdown
---
CARD: experience_section
title: Work Experience          # Repeated
headingLeft: Company A
headingRight: 2020-2021
---
Content A

---
CARD: experience_section
title: Work Experience          # Repeated
headingLeft: Company B
headingRight: 2021-2022
---
Content B

# ... 8 more with repeated "title: Work Experience"
```

**Desired syntax (hypothetical):**
```markdown
---
CARD_DEFAULTS: experience_section
title: Work Experience
---

---
CARD: experience_section
headingLeft: Company A          # title inherited
---
Content A

---
CARD: experience_section
headingLeft: Company B          # title inherited
---
Content B
```

**Why this isn't implemented:**
- No `CARD_DEFAULTS` parsing logic exists
- Card blocks are independent - no inheritance mechanism
- Would require stateful parsing (tracking active defaults)

**Why this is architectural:**
- Current design: cards are self-contained objects
- Adding defaults requires new parsing rules and data structures
- Potential for confusion: "Where did this field value come from?"

**Impact:**
- Verbose for documents with many similar cards (resumes, catalogs)
- Copy-paste errors common (wrong title on a card)
- Tedious authoring experience

**Recommendation:**

**Option A:** Add `DEFAULTS:` metadata in card blocks:
```markdown
---
CARD: experience_section
DEFAULTS: { title: "Work Experience" }
headingLeft: Company A
---
```

**Option B:** Schema-level defaults in Quill.toml:
```toml
[cards.experience_section.fields.title]
default = "Work Experience"
```
Then omit from markdown - applied automatically.

**Option C:** Template-level grouping:
```markdown
---
SECTION: work_experience
DEFAULTS: { title: "Work Experience" }
---

---
CARD: experience_section
headingLeft: Company A
---

---
CARD: experience_section
headingLeft: Company B
---

---
END_SECTION:
---
```

**Preferred:** Option B - defaults in schema, not markdown. Cleaner separation.

**Priority:** Medium - Quality of life for power users.

---

### 2.4 Strict Fence Detection (Only ```exactly 3 backticks```) ⚠️ **Low Impact**

**Issue:** Only exactly 3 backticks (` ``` `) are recognized as code fence delimiters. Tildes (`~~~`) and 4+ backticks are NOT treated as fences.

**Source:** `crates/core/src/parse.rs:216-229`
```rust
fn is_exact_fence_at(text: &str, pos: usize) -> bool {
    if !remaining.starts_with("```") { return false; }
    // Ensure it's exactly 3 backticks (4th char is not a backtick)
    remaining.len() == 3 || remaining.as_bytes().get(3) != Some(&b'`')
}
```

**Consequences:**

1. **Tildes not recognized:**
   ```markdown
   ~~~yaml
   ---
   CARD: example    # ← Parsed as real card, not code example!
   ---
   ~~~
   ```

2. **4+ backticks not recognized:**
   ````markdown
   ````yaml
   ---
   CARD: example    # ← Parsed as real card!
   ---
   ````
   ````

**Why this is restrictive:**
- CommonMark spec allows both ` ``` ` and `~~~` as fence markers
- CommonMark allows 3+ backticks/tildes
- Quillmark intentionally restricts to avoid ambiguity

**Rationale (from design):**
- Simplifies parser - only one fence pattern
- Avoids nested fence confusion
- Users can escape by using 4+ backticks (since they're not treated as fences)

**Impact:**
- Users coming from GitHub/GitLab expect `~~~` to work
- Copy-pasting code examples can create parsing bugs
- Surprise when metadata is parsed inside what looks like code blocks

**Recommendation:**
1. **Accept restriction** - simplifies parser significantly
2. **Improve error detection:**
   ```
   Warning: Found CARD directive inside ~~~ block.
   Note: Quillmark only treats ``` (exactly 3 backticks) as code fences.
   Use ``` instead of ~~~ to prevent this CARD from being parsed.
   ```

3. **Alternative:** Loosen restriction to support `~~~`:
   - Add tilde detection to `is_exact_fence_at()`
   - Track fence type (backtick vs tilde) and require matching close
   - Minimal complexity increase

**Priority:** Low - Restriction is defensible, mainly needs awareness.

---

## 3. Data Model Constraints

### 3.1 Card vs. Field Conceptual Distinction ⚠️ **High Impact**

**Issue:** The distinction between "fields" (document-level metadata) and "cards" (repeatable in-body structures) is a core architectural decision that isn't obvious to new users.

**Model:**
```
Document
├─ Fields (global metadata)
│  ├─ title: string
│  ├─ author: string
│  └─ date: date
├─ BODY (global markdown content)
└─ CARDS[] (ordered array of typed objects)
   ├─ Card { CARD: "section", title: "Intro", BODY: "..." }
   ├─ Card { CARD: "section", title: "Methods", BODY: "..." }
   └─ Card { CARD: "review", rating: 5, BODY: "..." }
```

**Decision tree (when to use which):**

**Use a FIELD when:**
- Single value per document (title, author, date)
- Applies to entire document (classification, version)
- Order doesn't matter
- Not repeatable

**Use a CARD when:**
- Multiple instances needed (sections, items, reviews)
- Order matters
- Includes markdown body content
- Type varies within document

**Why this is confusing:**
- Not obvious from syntax alone
- Both use YAML frontmatter blocks
- Distinction is conceptual, not syntactic

**Example confusion:**
```toml
# Should "sections" be a field or card?

# Option A: Field (array of objects)
[fields.sections]
type = "array"
items.type = "object"
items.properties.title = { type = "string" }

# Option B: Card (repeatable blocks)
[cards.section]
title.type = "string"
```

**When does it matter?**
- **Fields:** No body content, pure data
- **Cards:** Have body content, composable

**Why this is architectural:**
- Core data model separation
- Cannot be changed without redesigning the system
- Affects how templates are written

**Recommendation:**
1. **Accept model** - it's sound and necessary
2. **Provide clear decision tree** in documentation
3. **Consider naming:**
   - "Fields" → "Metadata" (clearer: document metadata)
   - "Cards" → "Blocks" or "Sections" (clearer: content blocks)

**Priority:** High conceptual barrier, but model is correct. Mainly needs communication.

---

### 3.2 Implicit Schema Injections ⚠️ **Low Impact**

**Issue:** The schema builder automatically injects fields that aren't explicitly defined in Quill.toml:

1. **BODY field:** Always added if not present (`schema.rs:218-227`)
2. **CARDS array:** Always added if cards exist (`schema.rs:253-263`)

**Example:**
```toml
# Quill.toml defines NO fields
[Quill]
name = "minimal"
backend = "typst"
description = "Minimal quill"

# Generated schema includes:
{
  "properties": {
    "BODY": { "type": "string", "contentMediaType": "text/markdown" }
    # ← Injected automatically
  }
}
```

**Why this happens:**
- `BODY` is fundamental - every document has content
- `CARDS` is required if card schemas exist
- Schema generation assumes these exist

**Impact:**
- Magical behavior - not obvious from configuration
- Can't opt out of `BODY` field
- No way to make `CARDS` optional even if cards are defined

**Recommendation:**
1. **Document automatic injection** clearly
2. **Allow opt-out via explicit configuration:**
   ```toml
   [Quill]
   include_body = false  # Don't inject BODY field
   ```

**Priority:** Low - current behavior is sensible, just implicit.

---

## 4. Missing Architectural Capabilities

### 4.1 No Field-Level Validation Constraints

**Missing:** Conditional required fields, cross-field validation, min/max constraints.

**Examples:**
```toml
# Can't express:
[fields.classification_guide]
required_if = { classification: "!= ''" }

[fields.password]
minLength = 8
pattern = "^(?=.*[A-Z])(?=.*[0-9]).*$"

[fields.end_date]
after = "start_date"
```

**Impact:** Medium - validation pushed to template layer or application code.

**Recommendation:** Add JSON Schema validation keywords:
- `minLength`, `maxLength`, `pattern` for strings
- `minimum`, `maximum` for numbers
- `minItems`, `maxItems` for arrays

Implementation: Pass through to JSON Schema generation.

---

### 4.2 No Card Ordering Metadata

**Missing:** Cannot specify display order or grouping for cards independently from document order.

**Example:**
```markdown
---
CARD: introduction
ORDER: 1
---

---
CARD: conclusion
ORDER: 3
---

---
CARD: methods
ORDER: 2
---
```

**Current behavior:** Order is implicit based on markdown position.

**Impact:** Low - current implicit ordering works for most cases.

**Recommendation:** Add optional `ORDER` field to card schema:
```toml
[cards.section.fields.order]
type = "number"
ui.hidden = true  # Don't show in UI, template use only
```

---

## 5. Quantified Impact Analysis

### Pain Point Severity Matrix

| Issue | Frequency | Severity | Workaround | Total Impact |
|-------|-----------|----------|------------|--------------|
| TOML nested verbosity | High | Medium | Inline JSON | **High** |
| Asymmetric first-block | Very High | High | None | **Critical** |
| Reserved keywords | Low | High | Rename fields | **Medium** |
| Tag naming restrictions | Medium | Low | Accept convention | **Low** |
| No field reusability | Medium | Medium | Copy-paste | **Medium** |
| No card defaults | High | Medium | Repeat values | **Medium** |
| Horizontal rule ambiguity | Low | Medium | Use blank lines | **Low** |
| Fence detection strictness | Low | Low | Use ``` only | **Low** |
| Card vs. field distinction | Very High | High | Learn model | **High** |

**Critical issues to address:**
1. Asymmetric first-block semantics
2. TOML nested structure verbosity
3. Card vs. field conceptual model (communication)

---

## 6. Recommendations Summary

### High Priority (Architectural Changes)

1. **Allow multiple global frontmatter blocks** (fixes asymmetry):
   - Merge all blocks without `CARD:` into global fields
   - Error on name collisions
   - Simplifies mental model significantly

2. **Schema-level card defaults** (reduces repetition):
   ```toml
   [cards.experience_section.fields.title]
   default = "Work Experience"
   ```

3. **Field schema reusability** (reduces duplication):
   ```toml
   [Quill.schema_defs.address]
   type = "object"
   properties.street = { type = "string" }

   [fields.shipping_address]
   type = "$ref:address"
   ```

### Medium Priority (Incremental Improvements)

4. **Namespace reserved keywords** (reduces collisions):
   ```
   __QUILL__, __CARD__, __BODY__, __CARDS__
   ```

5. **Inline JSON for complex field schemas** (TOML escape hatch):
   ```toml
   [fields.complex]
   schema = '''{"type": "array", "items": {...}}'''
   ```

6. **Relaxed tag naming** (backend-specific):
   ```toml
   [Quill]
   naming_style = "snake_case"  # or "camel_case"
   ```

### Low Priority (Accept Current Design)

7. **Fence detection:** Document clearly, accept restriction
8. **Horizontal rule ambiguity:** Document edge cases, improve errors
9. **Implicit schema injections:** Document behavior

---

## 7. Architectural Strengths (For Context)

**Well-designed aspects:**
1. **Type-safe schema system** - JSON Schema generation is clean and correct
2. **Unified CARDS array** - Simpler than separate arrays per type
3. **Automatic BODY injection** - Every card gets body content naturally
4. **OpenAPI discriminator pattern** - LLM-friendly schema generation
5. **Separation of metadata and schema** - Clean boundaries
6. **Backend-agnostic core** - Quill/Card system doesn't assume Typst

---

## 8. Architecture vs. Communication

**Architectural issues** (require code changes):
- TOML verbosity for nested structures
- Asymmetric first-block parsing
- No field schema reusability
- No card-level defaults
- Reserved keyword collisions

**Communication issues** (require documentation):
- Card vs. field mental model
- Tag naming restrictions
- Horizontal rule edge cases
- Fence detection strictness
- Implicit schema injections

This assessment focuses on the former. The communication issues are equally important but solvable without architectural changes.

---

## Conclusion

Quillmark's architecture is **fundamentally sound** with a coherent data model and clean separation of concerns. The primary ergonomic friction comes from:

1. **Format constraints** (TOML verbosity) - inherent to choice of config language
2. **Asymmetric parsing rules** (first-block special case) - fixable architectural decision
3. **Missing reusability mechanisms** - addressable with incremental features

The system is not ergonomically broken, but **could benefit from targeted architectural enhancements** to reduce repetition (field reusability, card defaults) and simplify mental models (symmetric frontmatter blocks).

**Overall architecture rating:** 8/10 - Strong foundations, moderate room for ergonomic improvement.
