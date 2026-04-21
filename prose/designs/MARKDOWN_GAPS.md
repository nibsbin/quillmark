# Quillmark Markdown — Gap Analysis

**Reference:** [`MARKDOWN.md`](./MARKDOWN.md) (authoritative spec) and
[`PARSE.md`](./PARSE.md) (implementation notes).

**Scope:** Enumerates every observed divergence between the current parser
(`crates/core/src/parse.rs`, `crates/core/src/normalize.rs`) and renderer
(`crates/backends/typst/src/convert.rs`) and the Quillmark Markdown
specification. Items are tagged **GAP** (spec behavior absent or materially
wrong), **BUG** (behavior inverted or contradictory), or **MINOR** (wording,
naming, or semantic drift that is unlikely to affect conformance).

---

## 1. Fence detection (spec §4)

### 1.1 GAP — F2 "Leading blank" rule not enforced

Spec §4 requires a `---` line to open a metadata fence iff the opening is on
line 1 **or** the line immediately above it is blank.

`parse.rs:282-293` only verifies that the `---` sits at the start of a line
(i.e. preceded by `\n`/`\r`), never that the *preceding non-empty line above*
is blank. Consequence: `---` directly under a paragraph currently opens a
metadata fence instead of being delegated to CommonMark as a setext H2
underline (spec §4.1 example).

Observable in `test_triple_dash_with_single_surrounding_newline_is_also_metadata`
(`parse.rs:1785-1803`) — the test *locks in* the non-conforming behavior.

### 1.2 GAP — F1 "Sentinel" rule under-enforced

Spec §4: the first metadata fence is recognised only when the first non-blank
line of its content matches `QUILL: …`; subsequent fences require `CARD: …`.

`parse.rs:520-540` accepts an opening `---/---` pair with **no** sentinel as
"global frontmatter" (silently) and only errors later with
`"Missing required QUILL field"`. Per spec, such a pair is not a fence at all
— it must be delegated to CommonMark (two thematic breaks / literal YAML
between).

`test_multiple_global_frontmatter_blocks` (`parse.rs:1342-1371`) codifies the
current non-conforming behavior (the second `---/---` is treated as an inline
metadata block and errors with "missing CARD directive").

### 1.3 GAP — No "delegate to CommonMark" path

Spec §4: "A `---` line that fails either rule is delegated to CommonMark
unchanged." The parser has no delegation mode — a `---` either opens a fence
(and must close) or is ignored. Anything that *looks like* a fence but fails
the sentinel becomes a hard parse error instead of an ordinary paragraph /
setext heading / thematic break.

### 1.4 GAP — Trailing whitespace on the fence marker not accepted

Spec §3: fences are "a pair of lines each containing exactly `---` (with
optional trailing whitespace)".

`parse.rs:274-277` and `parse.rs:310-329` search for the literal strings
`---\n` and `---\r\n`. A closing marker like `---   \n` (trailing spaces) is
not recognised.

### 1.5 GAP — §4.2 near-miss CARD lint warning unimplemented

Spec §4.2: "Implementations **should** emit a lint warning when they
encounter a `---/---` pair whose content's first non-blank line matches
`[A-Za-z][A-Za-z0-9_]*:` but whose key is not the expected sentinel"
(`Card:`, `card:`, `CARDS:`, etc.).

No such diagnostic exists. There is also no warnings channel out of
`ParseError` — only hard failures. `RenderResult::warnings` exists but the
parser cannot populate it.

---

## 2. Metadata fences & YAML body (spec §3)

### 2.1 OK — Whitespace-only fence content

`parse.rs:353` trims the YAML content; whitespace-only fences yield `None`
and collapse to an empty field set. ✓

### 2.2 OK — Reserved keys `BODY`, `CARDS`

Explicitly rejected in `parse.rs:376-386`. ✓

### 2.3 OK — `QUILL` / `CARD` positional constraints

`parse.rs:520-540` rejects `QUILL` in non-first blocks and rejects a block
carrying both `QUILL` and `CARD`. ✓

### 2.4 MINOR — `QUILL` + `CARD` in same block error message

`parse.rs:368-373` emits `"Cannot specify both QUILL and CARD"` — clear, but
the spec's formal term is that the block fails F1 entirely. Message is fine.

---

## 3. Input normalization (spec §7)

### 3.1 OK — Bidi control stripping

`normalize.rs:73-133` strips the exact set enumerated in spec §7 (U+061C,
U+200E, U+200F, U+202A–U+202E, U+2066–U+2069). ✓

### 3.2 OK — HTML comment fence repair

`normalize.rs:173-247` inserts a newline after `-->` when followed by
non-whitespace on the same line. ✓ Extra handling for `<!--- … --->` style
comments (line 193-203) goes beyond the spec but is benign.

### 3.3 BUG — Normalization is applied to YAML field values

Spec §7: "Normalization ... is **not** applied to YAML field values."

`normalize.rs:389-403` (`normalize_fields`) walks every field, recursively
normalizes all nested JSON strings, and strips bidi from every scalar —
including YAML string values. Only the HTML-comment repair is gated by the
`BODY` key check. Bidi stripping on YAML values is therefore out-of-spec.

### 3.4 GAP — Normalization is opt-in and post-parse

Spec §7 positions normalization "Before CommonMark parsing" of each body
region. The implementation applies it **after** parsing, on the
`ParsedDocument.fields` map, and only when the caller explicitly invokes
`normalize::normalize_document(...)`.

Consequences:
- Callers who use `ParsedDocument::from_markdown` and skip the extra call get
  no normalization at all.
- HTML comment fence repair for body text happens after the body is already
  captured verbatim; the "fix" never reaches the CommonMark parser in
  `convert.rs` (the convert path reads the already-stored `BODY` string,
  which *has* been normalized if the caller invoked `normalize_document`, but
  the timing model is inverted from the spec).
- Card bodies are stored in `CARDS[i].BODY` as nested strings; the recursive
  `normalize_json_value` walks into them but the `is_body` flag only fires
  for the top-level key named `BODY` (`normalize.rs:330`) — **card `BODY`
  fields never get HTML-comment repair**, because they are nested under the
  `CARDS` key rather than living at a top-level `BODY` key.

### 3.5 MINOR — Field name NFC normalization

`normalize.rs:424-426` applies Unicode NFC to field names. The spec does not
require this; it is an additional implementation choice. Benign.

---

## 4. Extensions & deviations (spec §6)

### 4.1 OK — Strikethrough, pipe tables

`convert.rs:1235-1238` enables `ENABLE_STRIKETHROUGH` and `ENABLE_TABLES`
only. No other pulldown-cmark extensions are enabled. ✓

### 4.2 GAP — `<u>…</u>` allowlist not implemented

Spec §6.1 / §6.2 deviation 2: `<u>text</u>` is **the** one allowlisted HTML
tag; it must render as underline.

`convert.rs:968-975` strips `Event::Html` / `Event::InlineHtml` entirely and
converts only `<br>` variants to `HardBreak`. A literal `<u>underline</u>`
in the body vanishes from the output.

### 4.3 BUG — `<br>` rendered instead of suppressed

Spec §6.3: "`<br>`, `<br/>`, `<br />` — follow the raw-HTML rule
(non-rendering); authors use CommonMark-native hard breaks".

`convert.rs:970-971` converts every `<br>` variant to a `HardBreak` event
and emits a hard break. This is the exact inverse of the spec: `<br>` should
render nothing, `<u>` should render an underline.

### 4.4 OK — `__text__` → underline (deviation 1)

Source-peek in `convert.rs:264-279` (`Tag::Strong` branch) picks
`StrongKind::Underline` when the source slice starts with `__`, otherwise
`StrongKind::Bold`. ✓

### 4.5 GAP (or intentional extension) — Intraword `__` preprocessing

Spec §6.1 note: "Inline `__text__` *also* produces underline (§6.2, deviation
1), but follows CommonMark delimiter-run rules and is therefore
word-bounded. **Intraword underline uses `<u>…</u>`.**"

`convert.rs:1048-1221` (`preprocess_intraword_formatting`) rewrites intraword
`__…__` (and `~~…~~`) into Unicode placeholder characters before
pulldown-cmark runs, so that intraword underline works via `__` as well.
This is a silent superset of the spec; it also plasters over the missing
`<u>` allowlist (§4.2 above) by letting `__` cover both roles. Either the
spec should acknowledge this, or the preprocessing should be removed in
favor of the `<u>` path.

### 4.6 GAP — Images not handled

Spec §6.3: "Images (`![alt](src)`) — reserved for the asset-resolver
integration; **required for v1 of this spec**."

`convert.rs` has no `Tag::Image` arm. Image events from pulldown-cmark fall
into the `_ => {}` catch-all and are dropped, with their alt text leaking
through as plain text.

### 4.7 MINOR — Thematic breaks silently dropped

`Event::Rule` has no handler in `convert.rs`. This is not a spec violation
(no required output is specified), but a reader of the rendered output
cannot distinguish a thematic break from its absence. See the confirmatory
test at `convert.rs:2002-2010`.

---

## 5. Data model (spec §5)

### 5.1 MINOR — `QUILL` stored outside `fields`

Spec §5's `Document` interface lists `QUILL: string` alongside other
frontmatter fields. The implementation stores it separately as
`ParsedDocument.quill_ref: QuillReference` and does **not** mirror it into
`fields`. Downstream consumers who walk `fields()` will not see `QUILL`.
Not strictly wrong (the accessor `quill_reference()` exists), but the
data-model shape diverges from the spec's TypeScript example.

### 5.2 OK — Everything else

`BODY`, `CARDS` always present (even when empty), per-card `CARD` + `BODY`
injection, free collision between global and card field names: all ✓
(`parse.rs:611-700`, exercised by tests on lines 1154-1221).

---

## 6. Limits (spec §8)

| Limit | Spec | Code constant | Match |
|---|---|---|---|
| Document size | 10 MB | `MAX_INPUT_SIZE` = 10 MB | ✓ |
| YAML size per fence | 1 MB | `MAX_YAML_SIZE` = 1 MB | ✓ |
| YAML nesting depth | 100 | `MAX_YAML_DEPTH` = 100 | ✓ |
| Markdown block nesting depth | 100 | `MAX_NESTING_DEPTH` = 100 | ✓ |
| Field count per fence | 1000 | `MAX_FIELD_COUNT` = 1000 | ⚠ see 6.1 |
| Card count per document | 1000 | `MAX_CARD_COUNT` = 1000 | ⚠ see 6.2 |

### 6.1 GAP — Field-count limit applied to the aggregate, not per-fence

Spec §8 says the 1000-field cap is **per fence**. The check at
`parse.rs:703-708` enforces it on the final merged `fields` map (global
frontmatter + `BODY` + `CARDS`). No per-fence cap exists, so a single card
can carry arbitrarily many fields provided the aggregate stays under 1000.

### 6.2 GAP — "Card count" limit counts all fences, not cards

The guard at `parse.rs:470-475` compares `blocks.len()` (all metadata
blocks, including the top-level frontmatter and any QUILL-carrying blocks)
against `MAX_CARD_COUNT`. A document with 1000 cards plus a frontmatter
trips the limit off-by-one; the error message "Input too large" also
reports the generic shape rather than `"card count exceeded"`.

---

## 7. Errors (spec §9)

| Spec-required error | Current path | Status |
|---|---|---|
| Missing frontmatter (no opening `---` on line 1) | `InvalidStructure("Missing required QUILL field…")` | ⚠ wording; does not distinguish "no fence" from "fence without QUILL" |
| Missing closing `---` before EOF | `InvalidStructure("Metadata block … not closed")` | ✓ |
| Frontmatter missing `QUILL` | `InvalidStructure` | ✓ |
| Card fence missing `CARD` | `MissingCardDirective` | ✓ |
| `QUILL` outside frontmatter | `InvalidStructure` (`"top-level frontmatter"`) | ✓ |
| `CARD` value fails `/^[a-z_][a-z0-9_]*$/` | `InvalidStructure` | ✓ |
| Invalid YAML | `YamlErrorWithLocation` | ✓ |
| Reserved key (`BODY`, `CARDS`) as user field | `InvalidStructure` | ✓ |
| Any §8 limit exceeded | `InputTooLarge` | ✓ |

### 7.1 GAP — No "opening `---` on line 1" check

Spec §9 lists "no opening `---` on line 1" as a distinct error. The current
parser happily accepts an opening fence on any line start (see §1.1). Until
F2 is enforced, this error cannot be raised cleanly either.

### 7.2 MINOR — `ParseError::YamlError` / `JsonError` / `Other` are unreachable

`parse.rs` only produces `InputTooLarge`, `InvalidStructure`,
`MissingCardDirective`, and `YamlErrorWithLocation`. The `YamlError`,
`JsonError`, and `Other` variants in `error.rs:365-398` appear to be
dead. Low priority — but any rework should prune or repurpose them.

---

## 8. Summary — items to fix in the rework

Priority ordering (spec conformance impact first):

1. **Fence detection rewrite** (§1.1, §1.2, §1.3, §1.4): implement F1 + F2
   exactly as spec'd; route failed fences into CommonMark. This subsumes
   many of the current test-locked behaviors and will require test
   revisions.
2. **Normalization timing + scope** (§3.3, §3.4): apply normalization to
   body regions *before* CommonMark parsing; exclude YAML values; recurse
   into card bodies correctly (the `is_body` flag is keyed off a top-level
   `BODY` only).
3. **HTML allowlist inversion** (§4.2, §4.3): render `<u>` as underline,
   suppress `<br>` (and all other raw HTML).
4. **Image support** (§4.6): add `Tag::Image` handling wired to the
   asset-resolver integration.
5. **Per-fence field-count limit + accurate card count** (§6.1, §6.2).
6. **Lint warning channel for near-miss sentinels** (§1.5): requires
   plumbing `Vec<Diagnostic>` out of the parser (currently there is no such
   path).
7. **Decide on intraword `__`** (§4.5): either document it as a Quillmark
   extension on top of the spec, or remove it in favor of the `<u>` path
   once §4.2 lands.
8. **Error-message wording** (§7.1) and dead variants (§7.2): cleanup.

---

## 9. Files touched by the rework

- `crates/core/src/parse.rs` — fence detection (§1), data model (§5),
  limits (§6), errors (§7).
- `crates/core/src/normalize.rs` — normalization scope/timing (§3).
- `crates/core/src/error.rs` — warning channel (§1.5), prune dead variants
  (§7.2).
- `crates/backends/typst/src/convert.rs` — HTML allowlist (§4.2, §4.3),
  image support (§4.6), intraword `__` decision (§4.5).
- Tests under each of the above.
