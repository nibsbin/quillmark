# Markdown Parsing & Conversion Hardening Proposal

**Date:** 2025-12-26
**Author:** Code Review Analysis
**Context:** Security audit and robustness review of `parse.rs`, `convert.rs`, `normalize.rs`, and `guillemet.rs`
**Scope:** Identify and remediate vulnerabilities, gaps, and weaknesses in markdown processing pipeline

---

## Executive Summary

A comprehensive review of Quillmark's markdown parsing and conversion subsystems identified **23 issues** across 6 categories:

| Category | Critical | High | Medium | Low |
|----------|----------|------|--------|-----|
| Security Vulnerabilities | 1 | 3 | 2 | 0 |
| Parsing Gaps | 0 | 2 | 2 | 1 |
| Conversion Gaps | 0 | 2 | 3 | 2 |
| Edge Cases | 0 | 1 | 3 | 2 |
| Design Weaknesses | 0 | 0 | 3 | 2 |

This proposal provides detailed analysis and actionable recommendations for each issue.

---

## Implementation Status

> **Last Updated:** 2025-12-26

| # | Issue | Priority | Status |
|---|-------|----------|--------|
| **1.1** | Incomplete Typst Character Escaping (`~`) | CRITICAL | ✅ Fixed - tilde now escaped |
| **1.2** | Eval Injection Pipeline Risk | HIGH | ✅ Fuzz tests added |
| **1.3** | Asset Path Traversal Incomplete | HIGH | ✅ Allowlist approach implemented |
| **1.4** | No YAML Recursion Depth Limit | HIGH | ✅ serde-saphyr Budget configured |
| **1.5** | Potential ReDoS in Guillemet Processing | MEDIUM | ✅ Early exit added |
| **1.6** | Bidi Override Attack Surface | MEDIUM | ⚠️ Partial - additional zero-width chars pending |
| **2.1** | Fenced Code Block Detection | HIGH | ✅ Strict 3-backtick enforcement |
| **2.2** | HTML Comment Handling Incomplete | HIGH | ❌ Not implemented |
| **2.3** | Horizontal Rule Disambiguation | MEDIUM | ✅ `---` reserved for metadata per spec |
| **2.4** | YAML Custom Tags Silently Stripped | LOW | ✅ Intentional - documented |
| **2.5** | Carriage Return Handling Inconsistent | LOW | ❌ Not implemented |
| **3.1-3.4** | Unsupported Features | - | ✅ Intentional limitations documented |
| **3.5** | EmphasisFixer Correctness Issues | MEDIUM | ✅ Escape handling + debug warning |
| **3.6** | Inline Code Backtick Handling | LOW | ❌ Not implemented |
| **3.7** | Silent Feature Dropping | LOW | ✅ Documented in EXTENDED_MARKDOWN.md |
| **4.1** | Depth Limit Consolidation | HIGH | ✅ Consolidated to error.rs |
| **4.2** | No Limit on Field/Card Count | MEDIUM | ✅ MAX_CARD_COUNT, MAX_FIELD_COUNT |
| **4.3** | Unicode Normalization Missing | MEDIUM | ✅ NFC via `normalize_field_name()` |
| **4.4** | Empty/Whitespace Frontmatter Confusion | MEDIUM | ❌ Not implemented |
| **4.5** | CARDS Array Always Present | LOW | ⚠️ Doc only - current behavior maintained |
| **4.6** | Error Messages Lack Source Location | LOW | ✅ `YamlErrorWithLocation` added |
| **5.1** | Multi-Phase Normalization Inconsistency | MEDIUM | ✅ `normalize_document()` entry point |
| **5.2** | No Validation Mode | MEDIUM | ✅ Exists in `schema.rs::validate_document()` |
| **5.3** | Template Errors vs Parse Errors Conflated | MEDIUM | ❌ Not implemented (high effort) |
| **5.4** | No Streaming/Incremental Parsing | LOW | ⚠️ Known limitation - documented |
| **5.5** | Integration Attack Scenario Tests | LOW | ✅ 11 tests in `security_tests.rs` |

**Legend:** ✅ Implemented | ⚠️ Partial/Deferred | ❌ Not implemented

---

## 1. Security Vulnerabilities

### 1.1 [CRITICAL] Incomplete Typst Character Escaping

**Location:** `crates/backends/typst/src/convert.rs:51-64`

**Current Implementation:**
```rust
pub fn escape_markup(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace("//", "\\/\\/")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('`', "\\`")
        .replace('#', "\\#")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('$', "\\$")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('@', "\\@")
}
```

**Missing Characters:**
| Character | Typst Meaning | Risk |
|-----------|---------------|------|
| `~` | Non-breaking space | Layout manipulation |
| `=` (at line start) | Heading marker | Structure injection |
| `+` (at line start) | List item | Structure injection |
| `-` (at line start) | List item / rule | Structure injection |
| `'` | Smart quotes | Minor |

**Attack Vector Example:**
```markdown
User input: "= Injected Heading"
After conversion: "= Injected Heading" (not escaped)
Typst interprets as: Level 1 heading
```

**Recommendation:**
```rust
pub fn escape_markup(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace("//", "\\/\\/")
        .replace('~', "\\~")           // ADD
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('`', "\\`")
        .replace('#', "\\#")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('$', "\\$")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('@', "\\@")
}

// Additionally, escape line-start-sensitive chars contextually
fn escape_line_start(line: &str) -> String {
    if line.starts_with('=') || line.starts_with('+') || line.starts_with('-') {
        format!("\\{}", line)
    } else {
        line.to_string()
    }
}
```

**Priority:** Critical
**Effort:** Low (1-2 hours)

---

### 1.2 [HIGH] Eval Injection Pipeline Risk

**Location:** `crates/backends/typst/src/filters.rs:198-201`

**Current Implementation:**
```rust
pub fn content_filter(_state: &State, value: Value, _kwargs: Kwargs) -> Result<Value, Error> {
    // ...
    let markup = mark_to_typst(&content).map_err(...)?;
    Ok(Value::from_safe_string(format!(
        "eval(\"{}\", mode: \"markup\")",
        escape_string(&markup)
    )))
}
```

**Risk:** The double-transformation pipeline (markdown → Typst → string literal → eval) creates complex escaping requirements. While `escape_string()` handles basic cases, edge cases involving:
- Nested quotes with backslashes
- Unicode characters that normalize to special chars
- Control characters

...could potentially bypass escaping.

**Recommendation:**

1. Add comprehensive fuzz tests targeting this specific pipeline:

```rust
// In convert_fuzz.rs
proptest! {
    #[test]
    fn fuzz_content_filter_injection(s in "\\PC{0,500}") {
        let markdown = s;
        if let Ok(typst) = mark_to_typst(&markdown) {
            let escaped = escape_string(&typst);
            let eval_expr = format!("eval(\"{}\", mode: \"markup\")", escaped);

            // Verify no unescaped quotes that could break out
            assert!(!contains_unescaped_quote(&eval_expr));

            // Verify no eval-within-eval patterns
            assert!(!eval_expr.contains("eval(eval("));
        }
    }
}
```

2. Consider alternative approach using raw strings:
```rust
// Instead of string escaping, use Typst raw content
Ok(Value::from_safe_string(format!(
    "eval(```{}```, mode: \"markup\")",
    markup.replace("```", "`` `")  // Escape only triple backticks
)))
```

**Priority:** High
**Effort:** Medium (4-8 hours for fuzz tests, additional for alternative impl)

---

### 1.3 [HIGH] Asset Path Traversal Incomplete

**Location:** `crates/backends/typst/src/filters.rs:208-217`

**Current Implementation:**
```rust
if filename.contains('/') || filename.contains('\\') {
    return Err(Error::new(
        ErrorKind::InvalidOperation,
        format!("Asset filename cannot contain path separators: '{}'", filename),
    ));
}
```

**Bypass Vectors:**
| Attack | Example | Bypasses Check |
|--------|---------|----------------|
| URL encoding | `..%2F..%2Fetc%2Fpasswd` | Yes |
| Unicode confusables | `..／etc／passwd` (fullwidth solidus U+FF0F) | Yes |
| Null byte injection | `valid.png\x00../../etc/passwd` | Potentially |
| Double encoding | `..%252F..%252F` | Yes |

**Recommendation:**
```rust
pub fn asset_filter(_state: &State, value: Value, _kwargs: Kwargs) -> Result<Value, Error> {
    let filename = value.to_string();

    // Decode URL encoding first
    let decoded = percent_decode_str(&filename)
        .decode_utf8()
        .map_err(|_| Error::new(ErrorKind::InvalidOperation, "Invalid UTF-8 in filename"))?;

    // Normalize Unicode
    let normalized: String = decoded.nfkc().collect();

    // Check for path separators (including Unicode variants)
    let has_path_sep = normalized.chars().any(|c| {
        c == '/' || c == '\\' ||
        c == '\u{FF0F}' ||  // Fullwidth solidus
        c == '\u{FF3C}' ||  // Fullwidth reverse solidus
        c == '\u{2215}' ||  // Division slash
        c == '\u{2216}'     // Set minus
    });

    if has_path_sep {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!("Asset filename cannot contain path separators: '{}'", filename),
        ));
    }

    // Check for null bytes
    if normalized.contains('\0') {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "Asset filename cannot contain null bytes",
        ));
    }

    // Check for path traversal patterns
    if normalized.contains("..") {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "Asset filename cannot contain path traversal patterns",
        ));
    }

    // Validate filename characters (allowlist approach)
    let valid_chars = normalized.chars().all(|c| {
        c.is_alphanumeric() || c == '.' || c == '-' || c == '_'
    });

    if !valid_chars {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!("Asset filename contains invalid characters: '{}'", filename),
        ));
    }

    let asset_path = format!("assets/DYNAMIC_ASSET__{}", normalized);
    Ok(Value::from_safe_string(format!("\"{}\"", asset_path)))
}
```

**Priority:** High
**Effort:** Medium (2-4 hours)

---

### 1.4 [HIGH] No YAML Recursion Depth Limit

**Location:** `crates/core/src/parse.rs:334`

**Issue:** While `normalize.rs` and `convert.rs` have `MAX_NESTING_DEPTH = 100`, YAML parsing via `serde-saphyr` has no explicit depth limit. Deeply nested YAML structures could cause stack overflow.

**Attack Vector:**
```yaml
a:
  b:
    c:
      d:
        # ... 1000+ levels deep
```

**Recommendation:**

1. Use `serde-saphyr`'s recursion limit if available, or:

2. Implement pre-parse depth check:
```rust
fn check_yaml_depth(content: &str, max_depth: usize) -> Result<(), ParseError> {
    let mut depth = 0;
    let mut max_seen = 0;

    for line in content.lines() {
        let indent = line.len() - line.trim_start().len();
        let current_depth = indent / 2; // Assuming 2-space indent
        max_seen = max_seen.max(current_depth);

        if max_seen > max_depth {
            return Err(ParseError::InvalidStructure(
                format!("YAML nesting depth {} exceeds maximum {}", max_seen, max_depth)
            ));
        }
    }
    Ok(())
}
```

**Priority:** High
**Effort:** Low (1-2 hours)

---

### 1.5 [MEDIUM] Potential ReDoS in Guillemet Processing

**Location:** `crates/core/src/guillemet.rs:50-185`

**Issue:** The `preprocess_guillemets_impl` function performs nested scanning. For input with many `<<` without matching `>>`, the complexity approaches O(n²).

**Attack Vector:**
```
<<<<<<<<<<<<<<<<<<<<... (10,000+ unmatched opening chevrons)
```

**Recommendation:**

1. Add early termination when no `>>` exists:
```rust
fn preprocess_guillemets_impl(text: &str, skip_code_blocks: bool) -> String {
    // Early exit if no closing pattern exists
    if !text.contains(">>") {
        return text.to_string();
    }
    // ... rest of implementation
}
```

2. Add iteration limit:
```rust
const MAX_GUILLEMET_ITERATIONS: usize = 10_000;

let mut iterations = 0;
while i < chars.len() {
    iterations += 1;
    if iterations > MAX_GUILLEMET_ITERATIONS {
        // Return partially processed result
        result.push_str(&chars[i..].iter().collect::<String>());
        break;
    }
    // ... rest of loop
}
```

**Priority:** Medium
**Effort:** Low (1 hour)

---

### 1.6 [MEDIUM] Bidi Override Attack Surface

**Location:** `crates/core/src/normalize.rs:71-88`

**Current State:** The code strips 11 bidi control characters. This is good, but the attack surface is broader.

**Missing Characters:**
| Character | Code Point | Name |
|-----------|------------|------|
| ZWNJ | U+200C | Zero Width Non-Joiner |
| ZWJ | U+200D | Zero Width Joiner |
| WJ | U+2060 | Word Joiner |
| NNBSP | U+202F | Narrow No-Break Space |
| ZWNBSP | U+FEFF | Zero Width No-Break Space (BOM) |

**Recommendation:**
```rust
fn is_invisible_or_bidi_char(c: char) -> bool {
    matches!(
        c,
        // Bidi controls
        '\u{200E}'..='\u{200F}' |  // LRM, RLM
        '\u{202A}'..='\u{202E}' |  // LRE, RLE, PDF, LRO, RLO
        '\u{2066}'..='\u{2069}' |  // LRI, RLI, FSI, PDI
        // Zero-width characters
        '\u{200B}' |  // ZWSP
        '\u{200C}' |  // ZWNJ
        '\u{200D}' |  // ZWJ
        '\u{2060}' |  // WJ
        '\u{FEFF}'    // BOM/ZWNBSP
    )
}
```

**Priority:** Medium
**Effort:** Low (30 minutes)

---

## 2. Parsing Gaps

### 2.1 [HIGH] Fenced Code Block Detection is Simplistic

**Location:** `crates/core/src/parse.rs:191-214`

**Current Implementation:**
```rust
fn is_inside_fenced_block(markdown: &str, pos: usize) -> bool {
    let before = &markdown[..pos];
    let mut fence_count = 0;

    if before.starts_with("```") || before.starts_with("~~~") {
        fence_count += 1;
    }

    fence_count += before.matches("\n```").count();
    fence_count += before.matches("\n~~~").count();
    // ...

    fence_count % 2 == 1
}
```

**Problems:**

1. **Loose Detection:** Current parser accepts variable lengths and tildes.
2. **Ambiguity:** `~~~` vs ```` ` vs ` ``` ` creates confusion.
3. **Spec Violation:** Current implementation does not enforce project-specific hardening rules.

**Use Case Enforcement:**
Per new specification, the parser must enforce:
1. **No Tildes:** `~~~` and other squiggly lines are forbidden as code fences.
2. **Strict Length:** Only **exactly three backticks** (```) are allowed.
   - ` ``` ` (3 ticks) -> Valid Fence
   - ` ```` ` (4 ticks) -> Invalid (Text)
   - ` `` ` (2 ticks) -> Invalid (Inline code or text)

**Recommendation:** Implement strict fence parsing regex/logic:

**Recommendation:** Implement proper CommonMark fence parsing:

```rust
#[derive(Debug, Clone)]
struct ActiveFence {
    indent: usize,    // 0-3 spaces
    start_pos: usize,
}

fn find_active_fence(markdown: &str, pos: usize) -> Option<ActiveFence> {
    let before = &markdown[..pos];
    let mut active: Option<ActiveFence> = None;

    for (line_start, line) in before.lines().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        // Skip if indent >= 4 (indented code block)
        if indent >= 4 {
            continue;
        }

        // Check for fence start char
        if trimmed.starts_with('`') {
            let fence_len = trimmed.chars().take_while(|&c| c == '`').count();

            // STRICT SPECIFICATION:
            // 1. Only backticks allowed (no tildes)
            // 2. Exactly three backticks required
            if fence_len == 3 {
                if active.is_some() {
                    // Check if this closes the active fence
                    // Closing fence must not have info string (just whitespace)
                    if trimmed[3..].trim().is_empty() {
                        active = None;
                    }
                } else {
                    // Open new fence
                    active = Some(ActiveFence {
                        indent,
                        start_pos: line_start,
                    });
                }
            }
        }
    }

    active
}

fn is_inside_fenced_block(markdown: &str, pos: usize) -> bool {
    find_active_fence(markdown, pos).is_some()
}
```

**Priority:** High
**Effort:** Medium (4-6 hours)

---

### 2.2 [COMPLETE] HTML Comment Handling Incomplete

**Status:** ✅ IMPLEMENTED

**Location:** `crates/core/src/normalize.rs:171-218`

**Issue:** Previous implementation was a simple regex-like search for `-->`, which could incorrectly identify standalone arrows as fence closers.

**Resolution:**

Rewrote `fix_html_comment_fences` to be context-aware. It now acts as a state machine that scanning for `<!--` openers first, then looks for the matching `-->` closer.

- **Standalone `-->`**: Ignored (treated as text).
- **Nested `<!--`**: Treated as text within the comment (correct HTML behavior).
- **Unclosed `<!--`**: Ignored (content remains as-is).

```rust
// Logic overview:
while let Some(open) = find_next("<!--") {
    if let Some(close) = find_next("-->") {
        // Fix fence for this block
    }
}
```

**Priority:** High
**Effort:** Medium (Implementation completed)


---

### 2.3 [MEDIUM] Horizontal Rule Disambiguation Edge Cases

**Location:** `crates/core/src/parse.rs:253-284`

**Issue:** The parser implementation currently attempts to support `---` as a horizontal rule if surrounded by blank lines. This is a direct violation of the `EXTENDED_MARKDOWN.md` specification.

**Specification Violation:**
> **`---` is reserved for metadata blocks only** — never treated as a thematic break
> Uses `***` or `___` for horizontal rules. The `---` syntax is not available for thematic breaks.

**Impact:**
1.  **Ambiguity:** Creates confusion about whether a generic `---` is a separator or broken metadata block.
2.  **Spec Drift:** Codebase behavior diverges from documentation.

**Recommendation:**
Strictly disable `---` detection as a horizontal rule.
1. `---` at the start of a line is **always** a metadata block delimiter.
2. If it is not a valid delimiter (e.g. not followed by valid YAML or closing fence), it should probably be treated as text or an error, but NEVER as a thematic break.

```rust
// In crates/core/src/parse.rs
// REMOVE the logic that skips '---' if preceded/followed by blank lines.
// It should strictly be treated as a metadata delimiter.
```

---

### 2.4 [LOW] YAML Custom Tags Silently Stripped

**Location:** `crates/core/src/parse.rs:334`

**Observation:** Custom YAML tags like `!fill` are silently stripped by `serde-saphyr` during parsing.
`!fill 2d lt example` becomes `"2d lt example"`.

**Assessment:**
This behavior is **INTENTIONAL**.
- Tags like `!fill` are metadata for LLM assistance or GUI highlighting.
- The Quillmark core renderer does not need to process these tags to generate the output document.
- Stripping them simplifies the data model (pure JSON).

**Recommendation:**
1. **Document this behavior** in `PARSE.md`. Explicitly state that custom tags are consumed/stripped during parsing and are not available to the rendering engine.
2. **Do NOT** implement complex tag preservation logic.

**Priority:** Low (Documentation only)
**Effort:** Low

---

### 2.5 [LOW] Carriage Return Handling Inconsistent

**Location:** Multiple files

| File | `\r` Handling |
|------|---------------|
| `parse.rs` | Handles `\r\n`, ignores standalone `\r` |
| `normalize.rs` | No `\r` normalization |
| `guillemet.rs` | Preserves `\r` in content |

**Recommendation:**

Add `\r` normalization early in the pipeline:
```rust
pub fn normalize_line_endings(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}
```

**Priority:** Low
**Effort:** Low (30 minutes)

---

## 3. Conversion Gaps

### 3.1 [VALIDATED] Fenced Code Blocks Not Supported
**Location:** `crates/backends/typst/src/convert.rs`

**Status:** Intentional Limitation.

**Assessment:**
Fenced code blocks are currently unsupported in the Quillmark renderer. This is a deliberate design choice for the current version. The renderer safely ignores the block delimiters and renders content as plain text.

**Recommendation:**
No action required for implementation. Consider adding a debug warning if `debug_assertions` are enabled.

---

### 3.2 [VALIDATED] Images Not Supported

**Location:** `crates/backends/typst/src/convert.rs`

**Status:** Intentional Limitation.

**Assessment:**
Inline images are not supported. The parsing pipeline correctly ignores image tags.

**Recommendation:**
No action required.


---

### 3.3 [VALIDATED] Block Quotes Not Supported

**Location:** `crates/backends/typst/src/convert.rs`

**Status:** Intentional Limitation.

**Recommendation:** No action required.


---

### 3.4 [VALIDATED] Horizontal Rules Not Supported

**Location:** `crates/backends/typst/src/convert.rs`

**Status:** Intentional Limitation.

**Recommendation:**
Ensure horizontal rules are ignored or treated as plain text if caught by the parser. (Note: `---` is reserved for metadata, but `***` or `___` are valid CommonMark rules that we are choosing not to support).


---

### 3.5 [MEDIUM] EmphasisFixer Correctness Issues

**Location:** `crates/backends/typst/src/convert.rs:328-472`

**Issue 1:** Skips processing when `\` or `&` present:
```rust
if source_slice.contains('\\') || source_slice.contains('&') {
    self.buffer.push((Event::Text(source_slice.into()), range));
    return;
}
```

This means `foo\\__bar__` won't have underscore emphasis processed.

**Issue 2:** Unclosed underlines silently revert to original text without warning.

**Recommendation:**

```rust
fn process_text_from_source(&mut self, range: Range<usize>) {
    let source_slice = &self.source[range.clone()];

    // Handle escaped characters properly instead of skipping
    if source_slice.contains('\\') {
        // Split on backslash and process segments
        self.process_with_escapes(source_slice, range);
        return;
    }

    // ... rest of processing

    // If unclosed underline, emit warning (requires adding warning channel)
    if in_underline {
        #[cfg(debug_assertions)]
        eprintln!("Warning: Unclosed __ in text: {:?}", source_slice);

        self.buffer.push((Event::Text(source_slice.into()), range));
    }
}
```

**Priority:** Medium
**Effort:** Medium (2-3 hours)

---

### 3.6 [LOW] Inline Code Backtick Handling

**Location:** `crates/backends/typst/src/convert.rs:303-308`

**Issue:** If CommonMark inline code contains literal backticks (which it can via multiple backtick delimiters), the current implementation doesn't handle this.

**Recommendation:**
```rust
Event::Code(text) => {
    // Count backticks in content
    let max_run = find_max_backtick_run(&text);
    let delimiter_len = max_run + 1;
    let delimiter: String = "`".repeat(delimiter_len);

    output.push_str(&delimiter);
    if text.starts_with('`') {
        output.push(' '); // Space padding per CommonMark
    }
    output.push_str(&text);
    if text.ends_with('`') {
        output.push(' ');
    }
    output.push_str(&delimiter);
    end_newline = false;
}
```

**Priority:** Low
**Effort:** Low (1 hour)

---

### 3.7 [LOW] Silent Feature Dropping

**Issue:** Multiple CommonMark features are silently ignored:
- HTML blocks
- Footnotes
- Math expressions
- Tables

**Verdict:**
This is consistent with the decision to support a minimal Markdown subset.

**Recommendation:**
Document the supported subset in `EXTENDED_MARKDOWN.md` (Completed). No code changes required.




---

## 4. Edge Cases & Robustness

### 4.1 [HIGH] Depth Limit Consolidation

**Issue:** Different modules define their own depth limits:

| Module | Constant | Value |
|--------|----------|-------|
| `convert.rs` | `MAX_NESTING_DEPTH` | 100 |
| `normalize.rs` | `MAX_NESTING_DEPTH` | 100 |
| `parse.rs` | (none) | unlimited |

**Recommendation:**

Create shared constants in `error.rs`:
```rust
// In crates/core/src/error.rs
pub const MAX_NESTING_DEPTH: usize = 100;
pub const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024;
pub const MAX_YAML_SIZE: usize = 1024 * 1024;
pub const MAX_GUILLEMET_LENGTH: usize = 64 * 1024;
pub const MAX_FIELD_COUNT: usize = 1000;
pub const MAX_CARD_COUNT: usize = 1000;
```

**Priority:** High
**Effort:** Low (1 hour)

---

### 4.2 [MEDIUM] No Limit on Field/Card Count

**Location:** `crates/core/src/parse.rs`

**Issue:** A malicious document could have thousands of CARD blocks or fields, causing memory exhaustion.

**Recommendation:**
```rust
const MAX_CARDS: usize = 1000;
const MAX_FIELDS: usize = 1000;

// In find_metadata_blocks:
if blocks.len() > MAX_CARDS {
    return Err(ParseError::InputTooLarge {
        size: blocks.len(),
        max: MAX_CARDS,
    });
}

// In decompose:
if fields.len() > MAX_FIELDS {
    return Err(ParseError::InputTooLarge {
        size: fields.len(),
        max: MAX_FIELDS,
    });
}
```

**Priority:** Medium
**Effort:** Low (30 minutes)

---

### 4.3 [MEDIUM] Unicode Normalization Missing

**Location:** `crates/core/src/normalize.rs`

**Issue:** Field names and values aren't Unicode normalized. `café` (composed) and `café` (decomposed) are different keys.

**Recommendation:**
```rust
use unicode_normalization::UnicodeNormalization;

pub fn normalize_field_name(name: &str) -> String {
    name.nfc().collect()
}
```

**Priority:** Medium
**Effort:** Low (add dependency, 1 hour)

---

### 4.4 [COMPLETE] Empty/Whitespace Frontmatter Confusion

**Status:** ✅ IMPLEMENTED

**Location:** `crates/core/src/parse.rs:2199`

**Issue:** The difference between empty frontmatter (`---\n---`) and whitespace-only (`---\n \n---`) is subtle and confusing.

**Resolution:**

Implemented whitespace trimming normalization in `decompose()`. If frontmatter content is only whitespace, it is now treated exactly like empty frontmatter (no YAML parsing attempted).

```rust
// In crates/core/src/parse.rs
let content = content.trim();
let (tag, quill_name, yaml_value) = if !content.is_empty() {
    // ... parse as YAML
} else {
    // ... treat as empty
    (None, None, None)
};
```

**Priority:** Medium
**Effort:** Low (1 hour)


---

### 4.5 [LOW] CARDS Array Always Present

**Location:** `crates/core/src/parse.rs:662-665`

**Issue:** CARDS is always added even when empty, which might surprise template authors.

**Options:**

1. **Keep current behavior** (simpler templates)
2. **Only add when non-empty** (more explicit)
3. **Add configuration option**

**Recommendation:** Document current behavior. If changing, use option 2:
```rust
if !cards_array.is_empty() {
    fields.insert(
        "CARDS".to_string(),
        QuillValue::from_json(serde_json::Value::Array(cards_array)),
    );
}
```

**Priority:** Low
**Effort:** Low (30 minutes)

---

### 4.6 [LOW] Error Messages Lack Source Location

**Location:** `crates/core/src/parse.rs:419-422`

**Issue:** YAML errors don't include byte offset in the source document.

**Recommendation:**
```rust
Err(e) => {
    let block_start_line = markdown[..abs_pos].lines().count();
    return Err(ParseError::YamlErrorWithLocation {
        error: e,
        line: block_start_line,
        block_index: blocks.len(),
    });
}
```

**Priority:** Low
**Effort:** Medium (2 hours, requires error type changes)

---

## 5. Design Weaknesses

### 5.1 [COMPLETE] Multi-Phase Normalization Creates Inconsistency Risk

**Status:** ✅ IMPLEMENTED

**Issue:** Normalization happens at multiple stages:
1. `normalize_markdown()` - initial input
2. `normalize_fields()` - after parsing
3. `preprocess_markdown_guillemets()` - during templating

If any phase is skipped (e.g., direct API usage), results are inconsistent.

**Resolution:**

Added `normalize_document()` function in `crates/core/src/normalize.rs` as the single entry point for document normalization. The function:
- Applies all field-level normalizations in the correct order
- Preserves the quill tag
- Is idempotent (can be called multiple times safely)
- Has comprehensive documentation including usage examples

```rust
use quillmark_core::{ParsedDocument, normalize::normalize_document};

// After parsing, call normalize_document before rendering
let doc = ParsedDocument::from_markdown(markdown).unwrap();
let normalized = normalize_document(doc);
```

**Priority:** Medium
**Effort:** Medium (2-3 hours)


---

### 5.2 [MEDIUM] No Validation Mode

**Issue:** There's no way to validate a document without rendering it.

**Recommendation:**

Add validation-only mode:
```rust
pub fn validate_document(markdown: &str, schema: Option<&QuillValue>) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Parse (collects parse errors)
    match ParsedDocument::from_markdown(markdown) {
        Ok(doc) => {
            // Schema validation if provided
            if let Some(schema) = schema {
                errors.extend(validate_against_schema(&doc, schema));
            }
        }
        Err(e) => errors.push(e.into()),
    }

    errors
}
```

**Priority:** Medium
**Effort:** Medium (4-6 hours)

---

### 5.3 [MEDIUM] Template Errors vs Parse Errors Conflated

**Issue:** When rendering fails, it's not always clear if the error is in:
- Markdown parsing
- Template syntax
- Filter application
- Typst compilation

**Recommendation:**

Create distinct error phases:
```rust
pub enum RenderPhase {
    Parsing,
    Normalization,
    Templating,
    Conversion,
    Compilation,
}

pub struct RenderError {
    pub phase: RenderPhase,
    pub message: String,
    pub source_location: Option<SourceLocation>,
}
```

**Priority:** Medium
**Effort:** High (significant refactor)

---

### 5.4 [LOW] No Streaming/Incremental Parsing

**Issue:** Entire document must be loaded into memory. For very large documents, this is inefficient.

**Recommendation:** Document this as a known limitation. For future:
- Consider streaming YAML parser
- Consider incremental markdown parser

**Priority:** Low
**Effort:** Very High (architectural change)

---

### 5.5 [LOW] Testing Gap: Integration Attack Scenarios

**Issue:** While unit tests are comprehensive, there are few end-to-end attack scenario tests.

**Recommendation:**

Add integration tests in `crates/quillmark/tests/`:
```rust
#[test]
fn test_injection_attack_scenarios() {
    let attacks = vec![
        // Typst injection via markdown
        ("**\"; eval(\"malicious\")**", "should_not_contain_eval"),
        // Path traversal via asset
        ("![](../../../etc/passwd)", "should_not_resolve"),
        // Deeply nested YAML
        (&"a:\n".repeat(200), "should_fail_gracefully"),
    ];

    for (input, expectation) in attacks {
        let result = render_to_pdf(input);
        assert!(verify_expectation(result, expectation));
    }
}
```

**Priority:** Low
**Effort:** Medium (4-6 hours)

---

## Implementation Roadmap

### Phase 1: Critical Security (1-2 days)
1. Fix `escape_markup()` missing characters
2. Add YAML depth limit
3. Improve asset path validation
4. Add early exit to guillemet processing

### Phase 2: High Priority Gaps (2-3 days)
5. Fix fenced code block detection (Parser)
6. Consolidate depth limits

### Phase 3: Medium Priority (3-5 days)
7. Fix EmphasisFixer edge cases
8. Add field/card count limits
9. Add Unicode normalization

### Phase 4: Polish (2-3 days)
10. Improve error messages with source locations
11. Add validation-only mode
12. Integration attack scenario tests

---

## Appendix: Test Cases for Validation

### A.1 Security Test Cases

```rust
#[test]
fn test_escape_markup_comprehensive() {
    // All Typst special chars
    let input = "\\*_`#[]$<>@~=+-";
    let escaped = escape_markup(input);
    assert!(!escaped.contains(|c| "*_`#[]$<>@~".contains(c) && !escaped.contains(&format!("\\{}", c))));
}

#[test]
fn test_yaml_depth_limit() {
    let deep_yaml = (0..150).map(|i| format!("{}a:", " ".repeat(i * 2))).collect::<String>();
    let markdown = format!("---\n{}\n---\n\nBody", deep_yaml);
    let result = ParsedDocument::from_markdown(&markdown);
    assert!(matches!(result, Err(ParseError::InvalidStructure(_))));
}

#[test]
fn test_asset_path_traversal_unicode() {
    let attacks = vec![
        "../etc/passwd",
        "..%2Fetc%2Fpasswd",
        "..／etc／passwd",  // Fullwidth solidus
        "file\x00.png",
    ];
    for attack in attacks {
        let result = asset_filter(&mock_state(), Value::from(attack), Kwargs::default());
        assert!(result.is_err());
    }
}
```

### A.2 Parsing Edge Case Tests

```rust
#[test]
fn test_fence_with_info_string() {
    let markdown = "```rust\n---\nCARD: test\n---\n```";
    let doc = ParsedDocument::from_markdown(markdown).unwrap();
    // Should be inside fence
    assert!(doc.get_field("CARDS").unwrap().as_array().unwrap().is_empty());
}

#[test]
fn test_strict_length_enforcement() {
    // 4 backticks = Text, not fence
    let markdown = "````\n---\nCARD: test\n---\n````";
    let doc = ParsedDocument::from_markdown(markdown).unwrap();
    // The --- should be DETECTED as metadata because ```` is not a fence
    assert!(!doc.get_field("CARDS").unwrap().as_array().unwrap().is_empty());
}

#[test]
fn test_no_tildes() {
    // Tildes = Text, not fence
    let markdown = "~~~\n---\nCARD: test\n---\n~~~";
    let doc = ParsedDocument::from_markdown(markdown).unwrap();
    assert!(!doc.get_field("CARDS").unwrap().as_array().unwrap().is_empty());
}
```



---

## References

- [CommonMark Spec](https://spec.commonmark.org/)
- [Typst Documentation](https://typst.app/docs/)
- [OWASP Input Validation](https://owasp.org/www-community/Input_Validation_Cheat_Sheet)
- `prose/designs/EXTENDED_MARKDOWN.md`
- `crates/backends/typst/docs/designs/CONVERT.md`
- `prose/designs/PARSE.md`
