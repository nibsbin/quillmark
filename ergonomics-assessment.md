# Quillmark Configuration & Markdown Block/Card System - Ergonomics Assessment

**Date:** 2026-01-20
**Scope:** Assessment of developer experience for Quill configuration and markdown block/card systems

---

## Executive Summary

Quillmark's configuration and block/card system demonstrates **strong architectural foundations** with clear separation of concerns, type safety, and extensibility. However, there are several **ergonomic friction points** that impact developer experience, particularly around:

1. **Verbose TOML syntax** for complex nested structures
2. **Cognitive overhead** in understanding the card vs. field distinction
3. **Limited validation feedback** during configuration authoring
4. **Documentation scattered** across multiple design documents
5. **Inconsistent terminology** (legacy SCOPE references vs. current CARD system)

**Overall Rating:** 7/10 for experienced developers, 5/10 for newcomers

---

## 1. Quill Configuration Ergonomics

### 1.1 Strengths

#### ✅ Clear, Explicit Structure
The `Quill.toml` format is explicit and self-documenting:
```toml
[Quill]
name = "classic_resume"
backend = "typst"
description = "A clean and modern classic resume template."
```

**Benefits:**
- No magic or implicit behavior
- Easy to understand what each section does
- Version control friendly (plain text, line-based)

#### ✅ Type Safety
Strong field type system with comprehensive type definitions:
- `string`, `number`, `boolean`, `array`, `object`
- Special types: `date`, `datetime`, `markdown`
- Validation at parse time

#### ✅ UI Metadata Integration
The `ui.group` and `ui.order` system enables frontend form generation:
```toml
[fields.memo_for]
title = "List of recipient organization(s)"
ui.group = "Addressing"
```

This bridges the gap between schema definition and user interface.

#### ✅ Default Quill System
The `__default__` quill enables zero-config rendering for simple documents, reducing friction for basic use cases.

---

### 1.2 Pain Points

#### ❌ Verbose Nested Field Syntax
Defining nested structures requires repetitive TOML syntax:

```toml
# From classic_resume/Quill.toml
cells.title = "Skill Categories"
cells.type = "array"
cells.required = true
cells.examples = [
    [
        { category = "Languages", skills = "Python, Rust, C++" },
        { category = "DevOps", skills = "Docker, Kubernetes" },
    ],
]
cells.items.type = "object"
cells.items.properties.category = { type = "string", title = "Category", required = true }
cells.items.properties.skills = { type = "string", title = "Skills", required = true }
```

**Issues:**
- 7 lines to define a simple array of objects
- Repetitive `cells.items.properties.` prefix
- Hard to visualize the resulting structure
- No syntax highlighting help for nested structure

**Severity:** Medium - Affects complex templates like resumes, grids, and structured data

#### ❌ Card vs. Field Mental Model
The distinction between `[fields.*]` and `[cards.*]` requires understanding:
- Fields = document-level metadata
- Cards = repeatable in-body content blocks

**Example confusion:**
```toml
# This is a field (document-level)
[fields.title]
type = "string"

# This is a card type definition
[cards.experience_section]
title = "Experience Entry"

# Card fields use different syntax
[cards.experience_section.fields.company]
type = "string"
```

**Issues:**
- Not immediately obvious why some things are fields vs. cards
- Documentation doesn't provide clear decision tree
- New users often try to make everything a field

**Severity:** Medium-High - Core conceptual hurdle for newcomers

#### ❌ No Schema Validation During Authoring
TOML files lack real-time validation:
- No IDE autocomplete for available properties
- Typos only discovered at runtime
- No schema file for TOML validation tools

**Example:**
```toml
[fields.contacts]
type = "arry"  # Typo - only caught when Quill loads
```

**Severity:** Low-Medium - IDE support could mitigate

#### ❌ Limited Error Messages for Malformed TOML
When TOML is invalid, errors reference line numbers but don't explain the schema:

```
Error: missing field `backend` at line 1 column 1
```

Better error:
```
Error: Missing required field `backend` in [Quill] section
Hint: Add `backend = "typst"` to the [Quill] section
See: https://docs.quillmark.dev/configuration#quill-section
```

**Severity:** Low - Affects onboarding

#### ❌ Inconsistent Field Definition Locations
Fields can be defined:
1. Inline: `author = { type = "string", required = true }`
2. Section: `[fields.author]` then properties below

No guidance on when to use which style.

**Example from appreciated_letter:**
```toml
# Inline style - terse but hard to read when complex
[fields]
sender = { description = "Sender of letter" }
recipient = { description = "Recipient of letter" }
```

**Example from usaf_memo:**
```toml
# Section style - verbose but clearer structure
[fields.memo_for]
title = "List of recipient organization(s)"
type = "array"
required = true
```

**Severity:** Low - Stylistic inconsistency

---

### 1.3 Missing Features

#### 🔶 No Field Dependencies or Conditional Logic
Cannot express "if field X is set, require field Y":

```toml
# Would be useful:
[fields.classification]
type = "string"

[fields.classification_guide]
type = "string"
required_if = { classification = "!=" "" }  # ❌ Not supported
```

**Workaround:** Handle in template logic (Typst/Jinja)

**Severity:** Medium - Common requirement for complex forms

#### 🔶 No Cross-Field Validation
Cannot validate that arrays have matching lengths or related constraints:

```toml
[fields.authors]
type = "array"

[fields.affiliations]
type = "array"
# ❌ Can't specify: must have same length as authors
```

**Severity:** Low-Medium - Edge case

#### 🔶 Limited Reusability
Cannot define reusable field schemas:

```toml
# ❌ Not possible:
[field_definitions.address]
type = "object"
properties.street = { type = "string" }
properties.city = { type = "string" }

[fields.shipping_address]
extends = "address"  # ❌ Not supported

[fields.billing_address]
extends = "address"  # ❌ Not supported
```

**Workaround:** Copy-paste field definitions

**Severity:** Low - Only affects large templates

---

## 2. Markdown Block/Card System Ergonomics

### 2.1 Strengths

#### ✅ Intuitive YAML Frontmatter
The `---` delimiter is familiar to users of Jekyll, Hugo, Obsidian:

```markdown
---
QUILL: classic_resume
name: John Doe
contacts:
  - john.doe@example.com
---
```

**Benefits:**
- Zero learning curve for users familiar with static site generators
- Standard YAML syntax (widely supported)
- Clean visual separation

#### ✅ Card Discriminator Pattern
The `CARD:` field clearly identifies block type:

```markdown
---
CARD: experience_section
headingLeft: Templar Archives Research Division
headingRight: August 2024 – Present
---

- Analyzed Khala disruption patterns...
```

**Benefits:**
- Self-documenting block types
- Supports multiple card types in single document
- LLM-friendly (clear discriminator for generation)

#### ✅ Body Capture
Each card automatically captures content after its metadata block:

```markdown
---
CARD: indorsement
from: ORG1/SYMBOL
---

This is the endorsement body content.
```

→ Parsed as: `{ CARD: "indorsement", from: "ORG1/SYMBOL", BODY: "This is..." }`

**Benefits:**
- Natural markdown authoring flow
- No special syntax for body content
- Composable sections

#### ✅ Unified CARDS Array
All cards aggregate into single `CARDS` array:

```json
{
  "CARDS": [
    { "CARD": "certifications_section", ... },
    { "CARD": "skills_section", ... },
    { "CARD": "experience_section", ... }
  ]
}
```

**Benefits:**
- Order preservation
- Simple template iteration
- Clean data model

---

### 2.2 Pain Points

#### ❌ UPPERCASE Convention Conflicts
Reserved fields use UPPERCASE (`QUILL`, `CARD`, `BODY`, `CARDS`), but this can conflict with domain terminology:

**Example conflict:**
```yaml
# Military document with acronyms
QUILL: usaf_memo
CARD: alert
LEVEL: HIGH     # Domain field (alert level)
BODY: Message   # ❌ Conflicts with reserved BODY field
```

**Issues:**
- Forces users to rename domain fields that happen to be uppercase
- Not obvious which fields are reserved
- Documentation doesn't list all reserved keywords

**Severity:** Low-Medium - Rare but frustrating when it happens

**Recommendation:** Document all reserved keywords prominently

#### ❌ Implicit Global Block Behavior
The first metadata block (without `CARD:`) becomes document-level fields, but this isn't visually distinguished:

```markdown
---
title: My Document    # ← Global field (no visual indicator)
---

---
CARD: section         # ← Card (has explicit CARD: marker)
---
```

**Issues:**
- First-time users often don't realize the first block is special
- Easy to accidentally create global fields when you meant to create a card
- No visual cue in the markdown itself

**Severity:** Medium - Common confusion point

**Recommendation:** Consider requiring `FRONTMATTER:` or similar marker for clarity

#### ❌ Card Schema Not Discoverable from Markdown
When writing markdown, there's no way to discover available card types or their fields without opening `Quill.toml`:

**Developer flow:**
1. Write markdown
2. Realize you need to know card schema
3. Switch to Quill.toml
4. Find `[cards.experience_section.fields.*]`
5. Switch back to markdown
6. Write card block
7. Repeat for each card

**Severity:** Medium-High - Friction in authoring workflow

**Mitigation:** IDE/editor plugin could provide autocomplete based on Quill schema

#### ❌ No Inline Card Field Documentation
Card field descriptions are in Quill.toml, not visible during markdown authoring:

```toml
[cards.indorsement.fields.from]
description = "Office symbol of the endorsing official."
```

↓ User writing markdown has no access to this hint

```markdown
---
CARD: indorsement
from: ???  # What goes here?
---
```

**Severity:** Medium - Requires context switching

**Mitigation:**
- Example markdown files help (already provided)
- IDE plugin could show field descriptions as hover tooltips

#### ❌ Repetitive Card Blocks
Writing many similar cards is verbose:

```markdown
---
CARD: experience_section
title: Work Experience
headingLeft: Company A
headingRight: 2020-2021
---
Details...

---
CARD: experience_section
title: Work Experience          # ← Repeated
headingLeft: Company B
headingRight: 2021-2022
---
Details...
```

**Issues:**
- Title field repeated for every card in same section
- No way to set card-level defaults
- Tedious for long lists (10+ work experiences)

**Severity:** Low-Medium - Quality of life issue

**Potential enhancement:**
```markdown
---
CARD: experience_section
DEFAULTS:
  title: Work Experience
---

---
CARD: experience_section
# title inherited from DEFAULTS
headingLeft: Company A
---
```

#### ❌ Horizontal Rule Ambiguity
The `---` delimiter conflicts with markdown horizontal rules:

**From parse.rs documentation:**
> Exception: `---` with blank lines above AND below is treated as content (horizontal rule)

```markdown
Main content.
                  # ← Blank line
---               # ← Treated as <hr>, not metadata block
                  # ← Blank line
More content.
```

**Issues:**
- Subtle parsing rule not obvious to users
- Causes confusion when horizontal rules are needed
- Documentation exists but buried in design docs

**Severity:** Low - Rare in practice, but surprising when encountered

**Recommendation:** Document prominently with examples

#### ❌ No Multi-Card Selection Helpers
No syntax for applying operations to multiple cards:

**Example use case:**
```markdown
# ❌ Want to hide these 3 experience entries in summary view
---
CARD: experience_section
headingLeft: Old Job 1
---
...

---
CARD: experience_section
headingLeft: Old Job 2
---
...

---
CARD: experience_section
headingLeft: Old Job 3
---
...
```

**Desired syntax:**
```markdown
---
SECTION: archived_experience
hidden: true
---

---
CARD: experience_section
headingLeft: Old Job 1
---
...
[... more cards ...]
---
END_SECTION: archived_experience
---
```

**Severity:** Low - Advanced use case

---

### 2.3 Missing Features

#### 🔶 No Card Templates/Snippets
Cannot define reusable card templates in markdown:

```markdown
# ❌ Not possible:
TEMPLATE: standard_experience
  headingLeft: Company Name
  headingRight: Dates
  subheadingLeft: Job Title

---
CARD: experience_section
TEMPLATE: standard_experience
headingLeft: ACME Corp  # Override template
---
```

**Workaround:** Use editor snippets or examples

**Severity:** Medium - Would significantly improve authoring experience

#### 🔶 No Card Ordering Hints
Cannot specify display order metadata:

```markdown
---
CARD: skills_section
ORDER: 1
---

---
CARD: experience_section
ORDER: 2
---
```

Currently order is implicit based on markdown position.

**Severity:** Low - Current implicit ordering works fine

#### 🔶 No Conditional Card Rendering
Cannot hide cards based on field values:

```markdown
---
CARD: certification_section
VISIBLE_IF: fields.show_certifications == true
---
```

**Workaround:** Handle in template logic

**Severity:** Low-Medium - Template logic is appropriate place

---

## 3. Cross-Cutting Concerns

### 3.1 Documentation & Discoverability

#### ❌ Scattered Documentation
Information spread across:
- `/prose/designs/CARDS.md`
- `/prose/designs/QUILL.md`
- `/prose/designs/SCHEMAS.md`
- `/prose/proposals/QUILL_TOML_ENHANCEMENT_PROPOSAL.md` (outdated)
- Backend-specific docs
- Example Quill.toml files

**Issues:**
- No single source of truth
- Proposals reference outdated concepts (SCOPE vs CARD)
- Hard to find answers to specific questions

**Severity:** High - Major onboarding barrier

**Recommendations:**
1. Create unified configuration guide
2. Mark outdated proposals clearly
3. Add "Configuration Cookbook" with common patterns
4. Generate reference docs from schema

#### ❌ No Error Code Reference
Error messages don't reference documentation:

```
Error: Field 'title' is required
```

Better:
```
Error: Field 'title' is required
Code: QUILL_MISSING_REQUIRED_FIELD
Docs: https://docs.quillmark.dev/errors#QUILL_MISSING_REQUIRED_FIELD
```

**Severity:** Medium - Affects debugging

### 3.2 Terminology Consistency

#### ❌ Legacy SCOPE vs Current CARD
The proposal document (`QUILL_TOML_ENHANCEMENT_PROPOSAL.md`) uses `SCOPE` terminology, but current implementation uses `CARD`:

**Outdated proposal:**
```markdown
## Problem Statement
Currently, Quill.toml supports schema annotation only for main document fields via `[fields.*]` sections. However, many quills use the SCOPE feature...
```

**Current design (CARDS.md):**
```markdown
> **Related Documents**:
> - ~~[SCOPES.md](SCOPES.md)~~ - **Superseded by this document**
```

**Issues:**
- Confusing for developers reading proposals
- GitHub issues/discussions may use outdated terms
- No migration guide for old SCOPE syntax

**Severity:** Medium - Creates confusion

**Recommendations:**
1. Archive outdated proposals with clear deprecation notice
2. Add terminology changelog to docs
3. Provide migration guide if SCOPE syntax was ever public

### 3.3 Developer Onboarding

#### ❌ Steep Learning Curve
To create a non-trivial Quill, developers must understand:

1. TOML syntax and nesting rules
2. Quill.toml structure ([Quill], [fields], [cards], [backend])
3. Field types and their JSON Schema mappings
4. Card vs. field mental model
5. UI metadata system
6. YAML frontmatter parsing rules
7. Card discriminator pattern
8. Template integration (backend-specific)

**Estimated time to first working Quill:**
- Simple (letter): 30 minutes
- Medium (resume): 2-3 hours
- Complex (memo with cards): 4-6 hours

**Severity:** High for newcomers, Low for experienced users

**Recommendations:**
1. Create interactive tutorial (step-by-step Quill creation)
2. Provide Quill scaffolding CLI: `quillmark new --template=resume`
3. Add more annotated examples
4. Create video walkthrough

---

## 4. Comparison to Alternative Approaches

### 4.1 vs. JSON Schema Directly

**Quillmark approach:**
```toml
[fields.author]
type = "string"
required = true
```

**Direct JSON Schema:**
```json
{
  "properties": {
    "author": { "type": "string" }
  },
  "required": ["author"]
}
```

**Quillmark advantages:**
- More readable for non-developers
- TOML comments for documentation
- Less verbose for common patterns

**JSON Schema advantages:**
- Native validation tooling
- IDE autocomplete (JSON Schema awareness)
- Standard format

### 4.2 vs. YAML Configuration

**Hypothetical YAML equivalent:**
```yaml
fields:
  author:
    type: string
    required: true
```

**YAML advantages:**
- More familiar to web developers
- Less verbose for nested structures

**TOML advantages:**
- Clearer section boundaries
- Better for deeply nested structures
- Less indentation-sensitive

### 4.3 vs. Code-Based Configuration

**Hypothetical TypeScript DSL:**
```typescript
defineQuill({
  name: "resume",
  backend: "typst",
  fields: {
    author: string().required(),
    contacts: array(string()),
  },
  cards: {
    experience: card({
      fields: {
        company: string().required(),
        date: string(),
      }
    })
  }
})
```

**Code advantages:**
- Type safety at authoring time
- IDE autocomplete
- Reusable abstractions

**TOML advantages:**
- No build step
- Accessible to non-programmers
- Simpler for LLM generation

---

## 5. Recommendations Summary

### High Priority (Address Soon)

1. **Create Unified Configuration Guide**
   - Single source of truth for Quill.toml syntax
   - Migration guide from any deprecated syntax
   - Decision tree for field vs. card

2. **Improve Error Messages**
   - Add error codes
   - Link to documentation
   - Provide fix suggestions

3. **Document Reserved Keywords**
   - List all reserved fields (QUILL, CARD, BODY, CARDS)
   - Explain collision handling
   - Show examples

4. **Archive/Update Outdated Proposals**
   - Mark SCOPE proposal as superseded
   - Add deprecation notices
   - Create terminology changelog

### Medium Priority (Nice to Have)

5. **Add TOML Schema Definition**
   - Create JSON Schema for Quill.toml validation
   - Enable IDE autocomplete
   - Catch typos at authoring time

6. **Create Quill Scaffolding Tool**
   - CLI: `quillmark new --name=my-resume --template=resume`
   - Interactive prompts for configuration
   - Generate example markdown

7. **Improve Card Authoring DX**
   - IDE/editor plugin with schema-aware autocomplete
   - Hover tooltips showing field descriptions
   - Inline validation

8. **Add Configuration Cookbook**
   - Common patterns (grids, nested arrays, dates)
   - Copy-paste examples
   - Troubleshooting guide

### Low Priority (Future Consideration)

9. **Field Dependencies**
   - Support `required_if`, `visible_if` conditions
   - Enables complex forms without template logic

10. **Reusable Field Definitions**
    - Define once, reference many times
    - Reduces duplication in large templates

11. **Card Templates in Markdown**
    - Reusable card snippets
    - Reduces repetition for similar cards

---

## 6. Conclusion

Quillmark's configuration and block/card system is **well-architected** with solid foundations in type safety, composability, and extensibility. The core abstractions (fields, cards, schemas) are sound.

However, **ergonomic improvements** in documentation, tooling, and error messages would significantly enhance developer experience, particularly for:

- **Newcomers**: Need clearer onboarding, unified docs, and better examples
- **Power users**: Want IDE support, validation, and reusability features
- **All users**: Benefit from better error messages and consistency

**Key insight:** The system is not ergonomically broken, but rather **underdocumented and undertooled**. The architecture supports great DX; the surrounding ecosystem needs enhancement.

**Next steps:**
1. Prioritize documentation consolidation
2. Improve error messages with codes and hints
3. Consider JSON Schema for Quill.toml validation
4. Build scaffolding/CLI tooling for new Quills
5. Explore IDE plugin for schema-aware markdown editing

---

## 7. Specific Examples of Excellent Design

### ✨ Default Quill System
The `__default__` quill is elegant:
- Zero-config for simple cases
- Progressive disclosure (add QUILL: tag when needed)
- Reduces barrier to entry

### ✨ OpenAPI 3.0 Discriminator
Using standard discriminator pattern makes schema LLM-friendly:
```json
"x-discriminator": {
  "propertyName": "CARD",
  "mapping": { "experience": "#/$defs/experience_card" }
}
```

This enables Claude/GPT to generate correctly-typed cards.

### ✨ Unified CARDS Array
Single array for all card types is simpler than separate arrays per type:

**Good (current):**
```json
{ "CARDS": [
  { "CARD": "experience", ... },
  { "CARD": "skills", ... }
]}
```

**Bad (alternative):**
```json
{
  "experiences": [...],
  "skills": [...],
  "projects": [...]
}
```

The current design preserves order and simplifies templates.

### ✨ BODY Auto-Capture
Automatic body field injection is intuitive:

```markdown
---
CARD: alert
level: high
---

This message is automatically captured as BODY.
```

No special syntax needed—just write markdown naturally.

---

## Appendix A: Field Type Completeness

Current types:
- ✅ string
- ✅ number
- ✅ boolean
- ✅ array
- ✅ object
- ✅ date
- ✅ datetime
- ✅ markdown

Potentially useful additions:
- 🔶 email (string with format: "email")
- 🔶 url (string with format: "uri")
- 🔶 color (string with format: "color")
- 🔶 file (reference to file in Quill bundle)
- 🔶 enum (limited to specific values - **currently supported via `enum` property**)

---

## Appendix B: Example Improvement Mockup

### Before (current):
```
Error: missing field `backend` at line 1 column 1
```

### After (improved):
```
Error: Missing required field 'backend' in [Quill] section
  ┌─ Quill.toml:1:1
  │
1 │ [Quill]
  │ ^^^^^^^ Missing required field: backend
  │
  = Note: The 'backend' field specifies which rendering engine to use
  = Hint: Add `backend = "typst"` to the [Quill] section
  = Docs: https://docs.quillmark.dev/configuration#quill-backend
```

---

**Assessment completed:** 2026-01-20
