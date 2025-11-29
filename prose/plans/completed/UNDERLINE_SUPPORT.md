# Underline Support via Double Underscore Disambiguation

> **Status**: Design Phase  
> **Design Reference**: `prose/designs/ARCHITECTURE.md` (Backend Architecture)  
> **Target**: `crates/backends/typst/src/convert.rs`

---

## Problem Statement

`pulldown-cmark` parses both `__text__` and `**text**` as `Tag::Strong` events. Currently, both render as bold (`*content*`) in our Typst output. We need to distinguish these styles to support underlining.

**Current behavior:**
- `__text__` → `*text*` (bold)
- `**text**` → `*text*` (bold)

**Desired behavior:**
- `__text__` → `#underline[text]`
- `**text**` → `*text*`

---

## Technical Approach

The `pulldown-cmark` parser does not provide semantic information about which delimiter style (`__` vs `**`) produced a `Tag::Strong` event. We must implement a **stateful interceptor** using source-text peeking.

### Key Insight

While `pulldown-cmark` does not distinguish between `__` and `**` in its event stream, it does provide **offset iteration** that gives us the byte range in the source text for each event. By peeking at the source text at these ranges, we can determine the original delimiter style.

---

## Architecture

### 1. Switch to Offset Iteration

The current implementation uses simple iteration:

```rust
// Current
let parser = Parser::new_ext(markdown, options);
for event in parser { ... }
```

Change to offset iteration to access source byte ranges:

```rust
// Proposed
let parser = Parser::new_ext(markdown, options);
for (event, range) in parser.into_offset_iter() { ... }
```

The `range: Range<usize>` gives the byte offsets into the original markdown source where this event originated.

### 2. State Management (LIFO Stack)

A stack tracks the semantic intent of nested `Strong` tags, ensuring correct closing tags.

```rust
enum StrongKind {
    Bold,      // Source was **...**
    Underline, // Source was __...__
}

let mut strong_stack: Vec<StrongKind> = Vec::new();
```

**Why a stack?**
- Markdown supports nested formatting: `__A **B** A__`
- Each `Start(Strong)` pushes, each `End(Strong)` pops
- LIFO order ensures correct pairing of open/close delimiters

### 3. Event Processing Logic

#### On `Start(Tag::Strong)`:

1. Peek at `&source[range.start..]`
2. Check first two characters:
   - If `"__"`: Push `StrongKind::Underline`, emit `#underline[`
   - If `"**"`: Push `StrongKind::Bold`, emit `*`
3. Handle edge case: Ensure range has at least 2 bytes to avoid panic

#### On `End(TagEnd::Strong)`:

1. Pop from `strong_stack`
2. Match popped value:
   - `StrongKind::Underline`: Emit `]`
   - `StrongKind::Bold`: Emit `*` (plus word-boundary handling for adjacent alphanumerics)
3. Handle edge case: Empty stack indicates malformed input (should not happen with valid markdown)

---

## Implementation Details

### Function Signature Change

The `push_typst` function signature changes to accept source text:

```rust
// Current
fn push_typst<'a, I>(output: &mut String, iter: I) -> Result<(), ConversionError>
where
    I: Iterator<Item = Event<'a>>,

// Proposed
fn push_typst<'a, I>(
    output: &mut String,
    source: &str,
    iter: I,
) -> Result<(), ConversionError>
where
    I: Iterator<Item = (Event<'a>, Range<usize>)>,
```

### mark_to_typst Changes

```rust
pub fn mark_to_typst(markdown: &str) -> Result<String, ConversionError> {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(markdown, options);
    let mut typst_output = String::new();

    // Pass source text for delimiter peeking
    push_typst(&mut typst_output, markdown, parser.into_offset_iter())?;
    Ok(typst_output)
}
```

### Delimiter Detection Logic

```rust
// At Start(Tag::Strong):
let kind = if range.start + 2 <= source.len() {
    match &source[range.start..range.start + 2] {
        "__" => StrongKind::Underline,
        _ => StrongKind::Bold, // Default to bold for ** or edge cases
    }
} else {
    StrongKind::Bold // Fallback for very short ranges
};
strong_stack.push(kind);

match kind {
    StrongKind::Underline => output.push_str("#underline["),
    StrongKind::Bold => output.push('*'),
}
```

### Zero-Allocation Guarantee

The delimiter detection uses string slicing (`&source[range.start..range.start + 2]`) which:
- Does not allocate new memory
- Returns a borrowed view into the source string
- Is O(1) operation (just pointer arithmetic)

---

## Typst Escaping

Text content inside `#underline[...]` must be properly escaped. The existing `escape_markup()` function already handles Typst control characters:

- `#` → `\#`
- `$` → `\$`
- `*` → `\*`
- `[` → `\[`
- `]` → `\]`
- `@` → `\@`

This function is already applied to all `Event::Text` content, so underlined text is automatically escaped.

**Example:**
- Input: `__#1__`
- Text event: `#1` → escaped to `\#1`
- Output: `#underline[\#1]`

---

## Edge Cases

### Deep Nesting

**Input:** `__A **B** A__`

**Event sequence:**
1. `Start(Strong)` at `__` → push Underline, emit `#underline[`
2. `Text("A ")` → emit `A `
3. `Start(Strong)` at `**` → push Bold, emit `*`
4. `Text("B")` → emit `B`
5. `End(Strong)` → pop Bold, emit `*`
6. `Text(" A")` → emit ` A`
7. `End(Strong)` → pop Underline, emit `]`

**Output:** `#underline[A *B* A]`

### Adjacent Styles

**Input:** `__A__**B**`

**Event sequence:**
1. `Start(Strong)` at `__` → push Underline, emit `#underline[`
2. `Text("A")` → emit `A`
3. `End(Strong)` → pop Underline, emit `]`
4. `Start(Strong)` at `**` → push Bold, emit `*`
5. `Text("B")` → emit `B`
6. `End(Strong)` → pop Bold, emit `*`

**Output:** `#underline[A]*B*`

### Underline Followed by Alphanumeric

**Input:** `__text__more`

The existing word-boundary logic (`#{}` insertion) may need adjustment. Currently:

```rust
// In TagEnd::Strong handling:
if let Some(Event::Text(text)) = iter.peek() {
    if text.chars().next().map_or(false, |c| c.is_alphanumeric()) {
        output.push_str("#{}");
    }
}
```

For underlines, this is not needed because `]` followed by alphanumeric is valid Typst. Only bold (`*`) needs this treatment.

**Modified logic:**
```rust
TagEnd::Strong => {
    match strong_stack.pop() {
        Some(StrongKind::Bold) => {
            output.push('*');
            // Word-boundary handling only for bold
            if let Some((Event::Text(text), _)) = iter.peek() {
                if text.chars().next().map_or(false, |c| c.is_alphanumeric()) {
                    output.push_str("#{}");
                }
            }
        }
        Some(StrongKind::Underline) => {
            output.push(']');
            // No word-boundary handling needed for function syntax
        }
        None => {
            // Malformed: more end tags than start tags
            // Default to bold behavior for robustness
            output.push('*');
        }
    }
    end_newline = false;
}
```

### Empty Strong Tags

**Input:** `____` or `****`

Both produce empty strong tags. The output should be:
- `____` → `#underline[]`
- `****` → `**`

This is handled naturally by the stack-based approach.

### Triple/Quadruple Underscores

**Input:** `___text___` (3 underscores) or `____text____` (4 underscores)

`pulldown-cmark` handles these as combinations of emphasis and strong. For example, `___text___` is parsed as `Emphasis(Strong(text))`.

Our implementation only affects `Tag::Strong` events, so:
- The emphasis layer uses `_` delimiter
- The strong layer uses `__` delimiter inside
- Detection at the `Tag::Strong` event will see `__` and correctly identify it

**Expected behavior preserved.**

---

## New State Variables Summary

| Variable | Type | Purpose |
|----------|------|---------|
| `strong_stack` | `Vec<StrongKind>` | Track open Strong tags and their delimiter type |

**Memory overhead:** One stack element per nested Strong tag. Typical documents have 0-3 nested levels, so this is negligible.

---

## Files to Modify

### Primary: `crates/backends/typst/src/convert.rs`

1. **Add enum definition** (near line 86, after `ListType`):
   ```rust
   #[derive(Debug, Clone, Copy)]
   enum StrongKind {
       Bold,
       Underline,
   }
   ```

2. **Add import** (line 31):
   ```rust
   use pulldown_cmark::{Event, Parser, Tag, TagEnd};
   ```
   Change to:
   ```rust
   use pulldown_cmark::{Event, Parser, Tag, TagEnd};
   use std::ops::Range;
   ```

3. **Update `push_typst` signature** (lines 93-96)

4. **Add `strong_stack` initialization** (near line 98)

5. **Update `Start(Tag::Strong)` handler** (lines 161-164)

6. **Update `End(TagEnd::Strong)` handler** (lines 230-238)

7. **Update `mark_to_typst`** (lines 291-299):
   - Change `parser` iteration to `parser.into_offset_iter()`
   - Pass `markdown` source to `push_typst`

8. **Update all match patterns** to extract `(event, range)` from iterator

---

## Tests to Add

Add these tests to the existing test module in `convert.rs`:

### Basic Underline Tests

```rust
#[test]
fn test_underline_basic() {
    assert_eq!(mark_to_typst("__underlined__").unwrap(), "#underline[underlined]\n\n");
}

#[test]
fn test_underline_with_text() {
    assert_eq!(
        mark_to_typst("This is __underlined__ text").unwrap(),
        "This is #underline[underlined] text\n\n"
    );
}

#[test]
fn test_bold_unchanged() {
    // Verify ** still works as bold
    assert_eq!(mark_to_typst("**bold**").unwrap(), "*bold*\n\n");
}
```

### Nesting Tests

```rust
#[test]
fn test_underline_containing_bold() {
    assert_eq!(
        mark_to_typst("__A **B** A__").unwrap(),
        "#underline[A *B* A]\n\n"
    );
}

#[test]
fn test_bold_containing_underline() {
    assert_eq!(
        mark_to_typst("**A __B__ A**").unwrap(),
        "*A #underline[B] A*\n\n"
    );
}

#[test]
fn test_deep_nesting() {
    assert_eq!(
        mark_to_typst("__A **B __C__ B** A__").unwrap(),
        "#underline[A *B #underline[C] B* A]\n\n"
    );
}
```

### Adjacent Styles Tests

```rust
#[test]
fn test_adjacent_underline_bold() {
    assert_eq!(
        mark_to_typst("__A__**B**").unwrap(),
        "#underline[A]*B*\n\n"
    );
}

#[test]
fn test_adjacent_bold_underline() {
    assert_eq!(
        mark_to_typst("**A**__B__").unwrap(),
        "*A*#underline[B]\n\n"
    );
}
```

### Escaping Tests

```rust
#[test]
fn test_underline_special_chars() {
    // Special characters inside underline should be escaped
    assert_eq!(
        mark_to_typst("__#1__").unwrap(),
        "#underline[\\#1]\n\n"
    );
}

#[test]
fn test_underline_with_brackets() {
    assert_eq!(
        mark_to_typst("__[text]__").unwrap(),
        "#underline[\\[text\\]]\n\n"
    );
}

#[test]
fn test_underline_with_asterisk() {
    assert_eq!(
        mark_to_typst("__a * b__").unwrap(),
        "#underline[a \\* b]\n\n"
    );
}
```

### Edge Case Tests

```rust
#[test]
fn test_empty_underline() {
    // Empty underline
    assert_eq!(mark_to_typst("____").unwrap(), "#underline[]\n\n");
}

#[test]
fn test_underline_in_list() {
    assert_eq!(
        mark_to_typst("- __underlined__ item").unwrap(),
        "- #underline[underlined] item\n\n"
    );
}

#[test]
fn test_underline_in_heading() {
    assert_eq!(
        mark_to_typst("# Heading with __underline__").unwrap(),
        "= Heading with #underline[underline]\n\n"
    );
}

#[test]
fn test_underline_followed_by_alphanumeric() {
    // Unlike bold, underline closing doesn't need word boundary
    assert_eq!(
        mark_to_typst("__under__line").unwrap(),
        "#underline[under]line\n\n"
    );
}
```

### Mixed Formatting Tests

```rust
#[test]
fn test_underline_with_italic() {
    assert_eq!(
        mark_to_typst("__underline *italic*__").unwrap(),
        "#underline[underline _italic_]\n\n"
    );
}

#[test]
fn test_underline_with_code() {
    assert_eq!(
        mark_to_typst("__underline `code`__").unwrap(),
        "#underline[underline `code`]\n\n"
    );
}

#[test]
fn test_underline_with_strikethrough() {
    assert_eq!(
        mark_to_typst("__underline ~~strike~~__").unwrap(),
        "#underline[underline #strike[strike]]\n\n"
    );
}
```

---

## Acceptance Criteria

- [ ] `__text__` renders as `#underline[text]`
- [ ] `**text**` renders as `*text*`
- [ ] Mixed nesting renders valid Typst syntax
- [ ] Special characters inside underlines are escaped correctly
- [ ] All existing tests continue to pass
- [ ] New tests cover all edge cases listed above

---

## Verification Steps

1. Run existing test suite: `cargo test -p quillmark-typst`
2. Verify no regressions in bold formatting
3. Manual verification with Typst compiler:
   - Compile sample document with underlines
   - Verify PDF output shows underlined text
4. Test edge cases manually with various nesting combinations

---

## Cross-References

- **Architecture**: `prose/designs/ARCHITECTURE.md` - Backend Architecture section
- **Typst Backend**: `crates/backends/typst/` - Implementation location
- **pulldown-cmark docs**: `Parser::into_offset_iter()` API reference

---

## Notes

### Why Not Configure pulldown-cmark?

`pulldown-cmark` does not provide configuration to distinguish `__` vs `**` in the event stream. This is by design—both are semantically "strong emphasis" in CommonMark.

The offset iteration approach is the cleanest solution that:
- Requires no changes to the parser
- Works with any pulldown-cmark version
- Is efficient (O(1) string slicing)
- Is maintainable (clear stack-based state)

### Alternative Considered: Pre-processing

An alternative would be to pre-process the markdown, replacing `__` with a custom syntax before parsing. This was rejected because:
- Adds complexity and allocation
- Risk of replacing `__` in code blocks or other contexts
- Harder to maintain and test

### Alternative Considered: Custom Parser

Building a custom parser or forking pulldown-cmark was rejected because:
- pulldown-cmark is well-tested and maintained
- Offset iteration provides sufficient information
- Would significantly increase maintenance burden
