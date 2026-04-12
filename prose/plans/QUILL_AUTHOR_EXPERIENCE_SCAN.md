# Quill Author Experience Scan

**Date:** 2026-04-12  
**Scope:** Author-facing behavior/standards for people creating Quill formats and writing Quill-targeted Markdown.

## Executive Summary

This scan found several places where author expectations can diverge from actual behavior. The highest-friction themes are:

1. **CommonMark expectations vs parser-specific metadata parsing rules** (especially fences and `---`).
2. **Naming-rule inconsistency for card identifiers** between document authoring and format design docs.
3. **Type validation messaging vs coercion behavior** (validation is not strictly "input must already be typed").
4. **YAML Frontmatter docs overstate broad YAML object support** relative to Quill schema capabilities.

---

## Findings

### 1) “Standard CommonMark” claim conflicts with metadata parser fence rules

**Why this is awkward:**
- Author docs state Quillmark supports standard CommonMark syntax.
- Parser logic for metadata delimiter detection intentionally treats only exactly triple-backtick fences as fences; tildes and 4+ backticks are ignored for fence-protection during metadata scanning.
- That means `---` appearing inside `~~~` or ```` fenced blocks can still be interpreted as metadata delimiters, which is surprising for users expecting CommonMark fence behavior.

**Evidence:**
- “supports standard CommonMark syntax” claim in docs. (`docs/authoring/markdown-syntax.md`)
- Parser comments and implementation that only exactly ` ``` ` counts as fence for metadata scanning. (`crates/core/src/parse.rs`)

**Risk for authors:**
- Copy-pasted examples that use `~~~` fences can parse unpredictably if they include `---`.

**Recommendation:**
- In author docs, add a callout that metadata scanning has stricter fence rules than full CommonMark and explicitly list supported/unsupported fence forms for delimiter shielding.

### 2) `---` horizontal rule guidance is stricter than runtime behavior

**Why this is awkward:**
- Docs say `---` cannot be used as horizontal rule in Quillmark documents.
- Parser currently has a heuristic that treats `---` as a horizontal rule when blank lines exist both above and below.
- Authors may get inconsistent outcomes depending on local spacing, which feels like a “sometimes works” rule.

**Evidence:**
- Prohibition in docs. (`docs/authoring/markdown-syntax.md`, `docs/authoring/cards.md`)
- Horizontal-rule heuristic in parser. (`crates/core/src/parse.rs`)

**Risk for authors:**
- Small whitespace edits can silently change parse interpretation around metadata boundaries.

**Recommendation:**
- Either (A) fully disallow `---` HR in parser for predictability, or (B) document the exact spacing-based heuristic with examples.

### 3) Card naming rules are inconsistent across authoring surfaces

**Why this is awkward:**
- Markdown card docs allow names matching `[a-z_][a-z0-9_]*` (leading underscore permitted).
- Quill.yaml reference says card type names must match `^[a-z][a-z0-9_]*$` (leading underscore forbidden).
- Parser helper for `CARD:` tags allows leading underscore.

**Evidence:**
- Markdown card regex allows leading underscore. (`docs/authoring/cards.md`)
- Quill.yaml reference forbids leading underscore. (`docs/format-designer/quill-yaml-reference.md`)
- `is_valid_tag_name` accepts leading underscore. (`crates/core/src/parse.rs`)
- Quill config validation for card definitions requires first char lowercase letter. (`crates/core/src/quill/config.rs`)

**Risk for authors:**
- A card block may parse, but no matching card schema can be defined under the stricter rule.

**Recommendation:**
- Choose one convention and enforce it everywhere (parser + docs + Quill config), then add one “invalid example” in docs to prevent ambiguity.

### 4) Frontmatter “objects supported” examples can mislead Quill format authors

**Why this is awkward:**
- Frontmatter docs present rich object/nested structure examples as generally supported YAML usage.
- Quill.yaml reference states top-level `type: object` fields are not supported for schema-driven fields (only object rows inside `array.items`).

**Evidence:**
- General object/nested examples in frontmatter docs. (`docs/authoring/yaml-frontmatter.md`)
- Explicit restriction on top-level object fields in Quill schema docs. (`docs/format-designer/quill-yaml-reference.md`)

**Risk for authors:**
- Authors may model schema fields as nested objects and then encounter validation/design mismatch.

**Recommendation:**
- Add an explicit cross-link warning in frontmatter docs: “YAML objects are syntactically valid, but Quill field schemas currently only support object typing inside array items.”

### 5) Validation wording implies strict typing, but workflow coerces first

**Why this is awkward:**
- Validation docs say parser checks “field types match schema.”
- Runtime workflow coerces fields before schema validation (e.g., strings to booleans/numbers and scalars to arrays where appropriate).
- This is useful behavior, but under-documented for format designers expecting strict rejection.

**Evidence:**
- Type-matching wording in frontmatter docs. (`docs/authoring/yaml-frontmatter.md`)
- Coercion rules in Quill config code. (`crates/core/src/quill/config.rs`)
- Coercion applied before `validate_document` in workflow. (`crates/quillmark/src/orchestration/workflow.rs`)

**Risk for authors:**
- Surprising acceptance of loosely typed input can hide upstream data quality issues.

**Recommendation:**
- Document coercion as a first-class validation stage, including a short conversion table and a note on how to enforce stricter upstream constraints.

---

## Suggested Prioritized Fix Order

1. **Unify card-name rules** (high confusion, easy to fix).
2. **Clarify fence/delimiter behavior** in Markdown authoring docs (high surprise potential).
3. **Align `---` HR docs and parser behavior** (predictability issue).
4. **Clarify frontmatter object examples vs schema limits** (format design correctness).
5. **Document coercion-before-validation pipeline** (expectation management).
