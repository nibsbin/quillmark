//! # Parsing Module
//!
//! Parsing functionality for markdown documents with YAML frontmatter.
//!
//! ## Overview
//!
//! The `parse` module provides the [`ParsedDocument::from_markdown`] function for parsing markdown documents
//!
//! ## Key Types
//!
//! - [`ParsedDocument`]: Container for parsed frontmatter fields and body content
//! - [`BODY_FIELD`]: Constant for the field name storing document body
//!
//! ## Examples
//!
//! ### Basic Parsing
//!
//! ```
//! use quillmark_core::ParsedDocument;
//!
//! let markdown = r#"---
//! QUILL: my_quill
//! title: My Document
//! author: John Doe
//! ---
//!
//! # Introduction
//!
//! Document content here.
//! "#;
//!
//! let doc = ParsedDocument::from_markdown(markdown).unwrap();
//! let title = doc.get_field("title")
//!     .and_then(|v| v.as_str())
//!     .unwrap_or("Untitled");
//! ```
//!
//! ## Error Handling
//!
//! The [`ParsedDocument::from_markdown`] function returns errors for:
//! - Malformed YAML syntax
//! - Unclosed frontmatter blocks
//! - Multiple global frontmatter blocks
//! - Both QUILL and CARD specified in the same block
//! - Reserved field name usage
//! - Name collisions
//!
//! See [PARSE.md](https://github.com/nibsbin/quillmark/blob/main/designs/PARSE.md) for comprehensive documentation of the Extended YAML Metadata Standard.

use std::collections::HashMap;
use std::str::FromStr;

use crate::error::ParseError;
use crate::value::QuillValue;
use crate::version::QuillReference;
use crate::{Diagnostic, Severity};

/// The field name used to store the document body
pub const BODY_FIELD: &str = "BODY";

/// Parse result carrying both the parsed document and any non-fatal warnings
/// (e.g. near-miss sentinel lints emitted per spec §4.2).
#[derive(Debug)]
pub struct ParseOutput {
    /// The successfully parsed document.
    pub document: ParsedDocument,
    /// Non-fatal warnings collected during parsing.
    pub warnings: Vec<Diagnostic>,
}

/// A parsed markdown document with frontmatter
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    fields: HashMap<String, QuillValue>,
    quill_ref: QuillReference,
}

impl ParsedDocument {
    /// Create a ParsedDocument from fields and quill reference
    pub fn new(fields: HashMap<String, QuillValue>, quill_ref: QuillReference) -> Self {
        Self { fields, quill_ref }
    }

    /// Create a ParsedDocument from markdown string, discarding any warnings.
    pub fn from_markdown(markdown: &str) -> Result<Self, crate::error::ParseError> {
        decompose(markdown)
    }

    /// Create a ParsedDocument from markdown string, returning warnings alongside the document.
    pub fn from_markdown_with_warnings(
        markdown: &str,
    ) -> Result<ParseOutput, crate::error::ParseError> {
        decompose_with_warnings(markdown)
            .map(|(document, warnings)| ParseOutput { document, warnings })
    }

    /// Get the quill reference (name + version selector)
    pub fn quill_reference(&self) -> &QuillReference {
        &self.quill_ref
    }

    /// Get the document body
    pub fn body(&self) -> Option<&str> {
        self.fields.get(BODY_FIELD).and_then(|v| v.as_str())
    }

    /// Get a specific field
    pub fn get_field(&self, name: &str) -> Option<&QuillValue> {
        self.fields.get(name)
    }

    /// Get all fields (including body)
    pub fn fields(&self) -> &HashMap<String, QuillValue> {
        &self.fields
    }

    /// Create a new ParsedDocument with default values applied
    ///
    /// This method creates a new ParsedDocument with default values applied for any
    /// fields that are missing from the original document but have defaults specified.
    /// Existing fields are preserved and not overwritten.
    ///
    /// # Arguments
    ///
    /// * `defaults` - A HashMap of field names to their default QuillValues
    ///
    /// # Returns
    ///
    /// A new ParsedDocument with defaults applied for missing fields
    pub fn with_defaults(&self, defaults: &HashMap<String, QuillValue>) -> Self {
        let mut fields = self.fields.clone();

        for (field_name, default_value) in defaults {
            // Only apply default if field is missing
            if !fields.contains_key(field_name) {
                fields.insert(field_name.clone(), default_value.clone());
            }
        }

        Self {
            fields,
            quill_ref: self.quill_ref.clone(),
        }
    }
}

#[derive(Debug)]
struct MetadataBlock {
    start: usize,                          // Position of opening "---"
    end: usize,                            // Position after closing "---\n"
    yaml_value: Option<serde_json::Value>, // Parsed YAML as JSON (None if empty or parse failed)
    tag: Option<String>,                   // Field name from CARD key
    quill_ref: Option<String>,             // Quill reference from QUILL key
}

/// Validate tag name follows pattern [a-z_][a-z0-9_]*
fn is_valid_tag_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();

    if !first.is_ascii_lowercase() && first != '_' {
        return false;
    }

    for ch in chars {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '_' {
            return false;
        }
    }

    true
}

/// Creates serde_saphyr Options with security budgets configured.
///
/// Uses MAX_YAML_DEPTH from error.rs to limit nesting depth at the parser level,
/// which is more robust than heuristic-based pre-parse checks.
fn yaml_parse_options() -> serde_saphyr::Options {
    let budget = serde_saphyr::Budget {
        max_depth: crate::error::MAX_YAML_DEPTH,
        ..Default::default()
    };
    serde_saphyr::Options {
        budget: Some(budget),
        ..Default::default()
    }
}

/// Returns true if `line` (without its line ending) is a `---` metadata-fence
/// marker per MARKDOWN.md §3: exactly three hyphens followed by optional
/// trailing whitespace (spaces or tabs).
fn is_fence_marker_line(line: &str) -> bool {
    let line = line.strip_suffix('\r').unwrap_or(line);
    // F3 (spec §4): the fence marker is preceded by zero to three spaces of
    // indentation. Four or more leading spaces (or any leading tab — a tab
    // counts as four columns of indentation) make the line indented code per
    // CommonMark §4.4, not a metadata fence.
    let indent = line.bytes().take_while(|&b| b == b' ').count();
    if indent > 3 {
        return false;
    }
    if line.as_bytes().first() == Some(&b'\t') {
        return false;
    }
    match line[indent..].strip_prefix("---") {
        Some(rest) => rest.chars().all(|c| c == ' ' || c == '\t'),
        None => false,
    }
}

/// Extracts the first non-blank, non-comment line of `content` and, if it
/// starts with a `[A-Za-z][A-Za-z0-9_]*` identifier followed by `:`, returns
/// that identifier. Any leading spaces/tabs on that line are ignored
/// (YAML indentation-tolerant). YAML `#` comment lines are skipped so the
/// sentinel rule is indifferent to banner-style comments above the key.
fn first_content_key(content: &str) -> Option<&str> {
    let first = content
        .lines()
        .find(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))?;
    let trimmed = first.trim_start_matches([' ', '\t']);
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b':' {
        Some(&trimmed[..i])
    } else {
        None
    }
}

/// Line-oriented view of the source, used for F1/F2 fence detection.
struct Lines<'a> {
    source: &'a str,
    starts: Vec<usize>, // byte offset of each line's first character
}

impl<'a> Lines<'a> {
    fn new(source: &'a str) -> Self {
        let mut starts = Vec::new();
        starts.push(0);
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        Self { source, starts }
    }
    fn len(&self) -> usize {
        self.starts.len()
    }
    fn line_start(&self, k: usize) -> usize {
        self.starts[k]
    }
    /// Byte position immediately after line k's trailing `\n` (or end-of-source
    /// if no newline follows).
    fn line_end_inclusive(&self, k: usize) -> usize {
        if k + 1 < self.starts.len() {
            self.starts[k + 1]
        } else {
            self.source.len()
        }
    }
    /// Line text without its trailing line ending.
    fn line_text(&self, k: usize) -> &'a str {
        let start = self.starts[k];
        let mut end = self.line_end_inclusive(k);
        if end > start && self.source.as_bytes()[end - 1] == b'\n' {
            end -= 1;
        }
        if end > start && self.source.as_bytes()[end - 1] == b'\r' {
            end -= 1;
        }
        &self.source[start..end]
    }
    fn is_blank(&self, k: usize) -> bool {
        self.line_text(k).chars().all(char::is_whitespace)
    }
}

/// Detect a CommonMark fenced code-block marker line. Returns `Some((char,
/// run_len))` if the line opens a fence, or `Some` with `is_closing=true` if
/// it closes one matching `open_fence`.
fn code_fence_on_line(line: &str, open_fence: Option<(u8, usize)>) -> Option<(u8, usize, bool)> {
    let indent = line.as_bytes().iter().take_while(|&&b| b == b' ').count();
    if indent > 3 {
        return None;
    }
    let trimmed = &line[indent..];
    let bytes = trimmed.as_bytes();
    let &first = bytes.first()?;
    if first != b'`' && first != b'~' {
        return None;
    }
    let run_len = bytes.iter().take_while(|&&b| b == first).count();
    if run_len < 3 {
        return None;
    }
    let rest = &trimmed[run_len..];
    match open_fence {
        Some((open_char, open_len)) => {
            if first == open_char
                && run_len >= open_len
                && rest.chars().all(|c| c == ' ' || c == '\t')
            {
                Some((first, run_len, true))
            } else {
                None
            }
        }
        None => Some((first, run_len, false)),
    }
}

/// Process YAML content for a recognized metadata fence and build a
/// `MetadataBlock`. The `is_first_block` flag governs whether `QUILL` is
/// expected (vs. `CARD`). Returns errors per spec §9.
fn build_block(
    markdown: &str,
    abs_pos: usize,
    abs_closing_pos: usize,
    block_end: usize,
    block_index: usize,
) -> Result<MetadataBlock, ParseError> {
    let raw_content = &markdown[abs_pos + fence_opener_len(markdown, abs_pos)..abs_closing_pos];

    // Check YAML size limit (spec §8)
    if raw_content.len() > crate::error::MAX_YAML_SIZE {
        return Err(ParseError::InputTooLarge {
            size: raw_content.len(),
            max: crate::error::MAX_YAML_SIZE,
        });
    }

    let content = raw_content.trim();
    let (tag, quill_ref, yaml_value) = if content.is_empty() {
        (None, None, None)
    } else {
        match serde_saphyr::from_str_with_options::<serde_json::Value>(
            content,
            yaml_parse_options(),
        ) {
            Ok(parsed) => extract_sentinels(parsed, markdown, abs_pos, block_index)?,
            Err(e) => {
                let line = markdown[..abs_pos].lines().count() + 1;
                return Err(ParseError::YamlErrorWithLocation {
                    message: e.to_string(),
                    line,
                    block_index,
                });
            }
        }
    };

    // Per-fence field-count check (spec §8, §6.1 of GAP analysis)
    if let Some(serde_json::Value::Object(ref map)) = yaml_value {
        // Add +1 for QUILL (stripped) or CARD (stripped) so the cap matches
        // what the user wrote, not what's left after sentinel extraction.
        let sentinel_extra = if quill_ref.is_some() || tag.is_some() {
            1
        } else {
            0
        };
        if map.len() + sentinel_extra > crate::error::MAX_FIELD_COUNT {
            return Err(ParseError::InputTooLarge {
                size: map.len() + sentinel_extra,
                max: crate::error::MAX_FIELD_COUNT,
            });
        }
    }

    Ok(MetadataBlock {
        start: abs_pos,
        end: block_end,
        yaml_value,
        tag,
        quill_ref,
    })
}

/// Number of bytes occupied by the opener `---[ \t]*\n` or `---[ \t]*\r\n`
/// starting at `abs_pos`. Only called on a line that already matched
/// `is_fence_marker_line`.
fn fence_opener_len(markdown: &str, abs_pos: usize) -> usize {
    let bytes = markdown.as_bytes();
    let mut i = abs_pos + 3; // past the "---"
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'\r' {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'\n' {
        i += 1;
    }
    i - abs_pos
}

/// Extract `QUILL` / `CARD` sentinels and remaining fields from a parsed-YAML
/// mapping. Returns `(tag, quill_ref, yaml_without_sentinel)`.
fn extract_sentinels(
    parsed: serde_json::Value,
    _markdown: &str,
    _abs_pos: usize,
    _block_index: usize,
) -> Result<(Option<String>, Option<String>, Option<serde_json::Value>), ParseError> {
    let Some(mapping) = parsed.as_object() else {
        // Non-mapping (scalar/sequence); keep as-is — upstream will reject if
        // it's a frontmatter/card mapping was expected.
        return Ok((None, None, Some(parsed)));
    };

    let has_quill = mapping.contains_key("QUILL");
    let has_card = mapping.contains_key("CARD");

    if has_quill && has_card {
        return Err(ParseError::InvalidStructure(
            "Cannot specify both QUILL and CARD in the same block".to_string(),
        ));
    }

    // Reserved keys (BODY, CARDS) — spec §3
    for reserved in ["BODY", "CARDS"] {
        if mapping.contains_key(reserved) {
            return Err(ParseError::InvalidStructure(format!(
                "Reserved field name '{}' cannot be used in YAML frontmatter",
                reserved
            )));
        }
    }

    if has_quill {
        let quill_str = mapping
            .get("QUILL")
            .unwrap()
            .as_str()
            .ok_or("QUILL value must be a string")?;
        quill_str.parse::<QuillReference>().map_err(|e| {
            ParseError::InvalidStructure(format!("Invalid QUILL reference '{}': {}", quill_str, e))
        })?;
        let mut new_map = mapping.clone();
        new_map.remove("QUILL");
        let new_val = if new_map.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(new_map))
        };
        Ok((None, Some(quill_str.to_string()), new_val))
    } else if has_card {
        let field_name = mapping
            .get("CARD")
            .unwrap()
            .as_str()
            .ok_or("CARD value must be a string")?;
        if !is_valid_tag_name(field_name) {
            return Err(ParseError::InvalidStructure(format!(
                "Invalid card field name '{}': must match pattern [a-z_][a-z0-9_]*",
                field_name
            )));
        }
        let mut new_map = mapping.clone();
        new_map.remove("CARD");
        let new_val = if new_map.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(new_map))
        };
        Ok((Some(field_name.to_string()), None, new_val))
    } else {
        Ok((None, None, Some(parsed)))
    }
}

/// Find all metadata fences in the document per MARKDOWN.md §3–§4.
///
/// Implements fence rules F1 (sentinel) and F2 (leading blank). Returns
/// successfully detected blocks plus any lint warnings emitted for
/// near-miss sentinels (§4.2).
/// Outcome of the fence-detection pass: the recognised metadata blocks, any
/// non-fatal diagnostics accumulated along the way, and (if applicable) the
/// first-fence F1 failure captured so the top-level error can be specific.
type FenceScan = (Vec<MetadataBlock>, Vec<Diagnostic>, Option<(String, usize)>);

fn find_metadata_blocks(markdown: &str) -> Result<FenceScan, ParseError> {
    let lines = Lines::new(markdown);
    let mut blocks: Vec<MetadataBlock> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    // (char, min_run_len, opener_line_index)
    let mut open_code_fence: Option<(u8, usize, usize)> = None;
    // First-fence F1 failure context, captured for a clearer top-level error
    // if no valid QUILL fence is ever found. (actual_key, 1-based line).
    let mut first_fence_issue: Option<(String, usize)> = None;

    let mut k: usize = 0;
    while k < lines.len() {
        let text = lines.line_text(k);

        // Track open CommonMark fenced-code-block state so that `---` inside
        // them is ignored (spec §3 "Fences inside fenced code blocks").
        if let Some((ch, min, _opener)) = open_code_fence {
            if let Some((_, _, true)) = code_fence_on_line(text, Some((ch, min))) {
                open_code_fence = None;
            }
            k += 1;
            continue;
        }
        if let Some((ch, run_len, _)) = code_fence_on_line(text, None) {
            open_code_fence = Some((ch, run_len, k));
            k += 1;
            continue;
        }

        // Candidate fence opener?
        if !is_fence_marker_line(text) {
            k += 1;
            continue;
        }

        // F2 — Leading blank rule
        let f2_ok = k == 0 || lines.is_blank(k - 1);
        if !f2_ok {
            k += 1;
            continue;
        }

        // Scan ahead for the closer. Inside a metadata fence, YAML content is
        // opaque — don't update code-block state.
        let mut closer_k: Option<usize> = None;
        let mut j = k + 1;
        while j < lines.len() {
            if is_fence_marker_line(lines.line_text(j)) {
                closer_k = Some(j);
                break;
            }
            j += 1;
        }

        let content_start = lines.line_end_inclusive(k);
        let (content_end, block_end) = match closer_k {
            Some(cj) => (lines.line_start(cj), lines.line_end_inclusive(cj)),
            None => (markdown.len(), markdown.len()),
        };
        let content = &markdown[content_start..content_end];

        // F1 — Sentinel rule. First non-blank line of content must match the
        // expected sentinel (`QUILL` for the first fence, `CARD` thereafter).
        let expected = if blocks.is_empty() { "QUILL" } else { "CARD" };
        let key = first_content_key(content);
        let f1_ok = key == Some(expected);

        if !f1_ok {
            // Near-miss lint (spec §4.2): first key looked like an identifier
            // but wasn't the expected sentinel.
            if let Some(actual) = key {
                if actual != expected {
                    warnings.push(
                        Diagnostic::new(
                            Severity::Warning,
                            format!(
                                "Near-miss metadata sentinel `{}:` at line {} — expected `{}:`. This `---/---` pair is treated as literal Markdown; if you intended a metadata fence, change the key to `{}`.",
                                actual, k + 1, expected, expected
                            ),
                        )
                        .with_code("parse::near_miss_sentinel".to_string()),
                    );
                    // Capture the first-fence F1 failure so the top-level
                    // "Missing required QUILL field" error can be specific
                    // about the actual key found.
                    if blocks.is_empty() && first_fence_issue.is_none() {
                        first_fence_issue = Some((actual.to_string(), k + 1));
                    }
                }
            }
            // Delegate this opener to CommonMark — advance past the opener
            // line only; the closer (if any) may become its own candidate
            // opener on a later iteration (it will fail F2 and be skipped).
            k += 1;
            continue;
        }

        // F1 passed — a legitimate fence. If the closer was missing, this is
        // a hard error (spec §9).
        let Some(cj) = closer_k else {
            return Err(ParseError::InvalidStructure(
                "Metadata block started but not closed with ---".to_string(),
            ));
        };

        let abs_pos = lines.line_start(k);
        let abs_closing_pos = lines.line_start(cj);
        let block = build_block(markdown, abs_pos, abs_closing_pos, block_end, blocks.len())?;
        blocks.push(block);

        k = cj + 1;
    }

    // Card-count check counts only blocks carrying a CARD sentinel (spec §8).
    let card_count = blocks.iter().filter(|b| b.tag.is_some()).count();
    if card_count > crate::error::MAX_CARD_COUNT {
        return Err(ParseError::InputTooLarge {
            size: card_count,
            max: crate::error::MAX_CARD_COUNT,
        });
    }

    // Unclosed fenced code block at end-of-document: any metadata fences below
    // the unclosed opener were silently shielded, which is almost never what
    // the author intended. Surface it as a non-fatal warning.
    if let Some((_, _, opener_line)) = open_code_fence {
        warnings.push(
            Diagnostic::new(
                Severity::Warning,
                format!(
                    "Unclosed fenced code block opened at line {} — end-of-document reached without a matching closing fence. Any `---/---` pairs after this line were treated as code and not parsed as metadata fences.",
                    opener_line + 1
                ),
            )
            .with_code("parse::unclosed_code_block".to_string()),
        );
    }

    Ok((blocks, warnings, first_fence_issue))
}

/// Decompose markdown, discarding warnings. Test- and `from_markdown`-facing.
fn decompose(markdown: &str) -> Result<ParsedDocument, crate::error::ParseError> {
    decompose_with_warnings(markdown).map(|(doc, _)| doc)
}

/// Construct the top-level "missing QUILL" error message. If we saw a
/// first-fence F1 failure, tailor the message to the actual key found:
/// a case-insensitive match to `QUILL` is a typo, anything else is a
/// key-ordering problem.
fn missing_quill_message(first_fence_issue: Option<(String, usize)>) -> String {
    match first_fence_issue {
        Some((actual, line)) if actual.eq_ignore_ascii_case("QUILL") => format!(
            "Missing required QUILL field. Found `{}:` at line {} — expected `QUILL:` (uppercase). Change the key to `QUILL` to register this fence as the document frontmatter.",
            actual, line
        ),
        Some((actual, line)) => format!(
            "Missing required QUILL field. The first YAML key in the frontmatter must be `QUILL:` (found `{}:` at line {}). Reorder the frontmatter so `QUILL: <name>` is the first key.",
            actual, line
        ),
        None => "Missing required QUILL field. Add `QUILL: <name>` to the frontmatter.".to_string(),
    }
}

/// Decompose markdown into frontmatter fields and body, returning any
/// non-fatal warnings collected during fence scanning.
fn decompose_with_warnings(
    markdown: &str,
) -> Result<(ParsedDocument, Vec<Diagnostic>), crate::error::ParseError> {
    // Strip a leading UTF-8 BOM if present. Editors on Windows (Notepad, some
    // Word exports) prepend `\u{FEFF}` which otherwise defeats F2 because the
    // first line no longer matches `---`.
    let markdown = markdown.strip_prefix('\u{FEFF}').unwrap_or(markdown);

    // Check input size limit
    if markdown.len() > crate::error::MAX_INPUT_SIZE {
        return Err(crate::error::ParseError::InputTooLarge {
            size: markdown.len(),
            max: crate::error::MAX_INPUT_SIZE,
        });
    }

    let mut fields = HashMap::new();

    // Find all metadata blocks. F1/F2 already guarantee that block 0 carries
    // QUILL and that every subsequent block carries CARD.
    let (blocks, warnings, first_fence_issue) = find_metadata_blocks(markdown)?;

    if blocks.is_empty() {
        return Err(crate::error::ParseError::InvalidStructure(
            missing_quill_message(first_fence_issue),
        ));
    }

    let mut cards_array: Vec<serde_json::Value> = Vec::new();

    // Block 0 is always the QUILL frontmatter (F1 guarantee).
    let frontmatter = &blocks[0];
    let quill_tag = frontmatter.quill_ref.clone().ok_or_else(|| {
        ParseError::InvalidStructure(
            "Missing required QUILL field. Add `QUILL: <name>` to the frontmatter.".to_string(),
        )
    })?;

    // Merge frontmatter fields (YAML content with QUILL stripped).
    match &frontmatter.yaml_value {
        Some(serde_json::Value::Object(mapping)) => {
            for (key, value) in mapping {
                fields.insert(key.clone(), QuillValue::from_json(value.clone()));
            }
        }
        Some(serde_json::Value::Null) | None => {}
        Some(_) => {
            return Err(ParseError::InvalidStructure(
                "Invalid YAML frontmatter: expected a mapping".to_string(),
            ));
        }
    }

    // Parse tagged blocks (CARD blocks)
    for (idx, block) in blocks.iter().enumerate() {
        if let Some(ref tag_name) = block.tag {
            // Get YAML metadata directly (already parsed in find_metadata_blocks)
            // Get JSON metadata directly (already parsed in find_metadata_blocks)
            let mut item_fields: serde_json::Map<String, serde_json::Value> =
                match &block.yaml_value {
                    Some(serde_json::Value::Object(mapping)) => mapping.clone(),
                    Some(serde_json::Value::Null) => {
                        // Null value (from whitespace-only YAML) - treat as empty mapping
                        serde_json::Map::new()
                    }
                    Some(_) => {
                        return Err(crate::error::ParseError::InvalidStructure(format!(
                            "Invalid YAML in card block '{}': expected a mapping",
                            tag_name
                        )));
                    }
                    None => serde_json::Map::new(),
                };

            // Extract body for this card block
            let body_start = block.end;
            let body_end = if idx + 1 < blocks.len() {
                blocks[idx + 1].start
            } else {
                markdown.len()
            };
            let body = &markdown[body_start..body_end];

            // Add body to item fields
            item_fields.insert(
                BODY_FIELD.to_string(),
                serde_json::Value::String(body.to_string()),
            );

            // Add CARD discriminator field
            item_fields.insert(
                "CARD".to_string(),
                serde_json::Value::String(tag_name.clone()),
            );

            // Add to CARDS array
            cards_array.push(serde_json::Value::Object(item_fields));
        }
    }

    // Global body: between end of frontmatter (block 0) and start of the
    // first CARD block (or EOF).
    let body_start = blocks[0].end;
    let body_end = blocks
        .iter()
        .skip(1)
        .find(|b| b.tag.is_some())
        .map(|b| b.start)
        .unwrap_or(markdown.len());
    let global_body = &markdown[body_start..body_end];

    fields.insert(
        BODY_FIELD.to_string(),
        QuillValue::from_json(serde_json::Value::String(global_body.to_string())),
    );

    // Always add CARDS array to fields (may be empty)
    fields.insert(
        "CARDS".to_string(),
        QuillValue::from_json(serde_json::Value::Array(cards_array)),
    );

    let quill_ref = QuillReference::from_str(&quill_tag).map_err(|e| {
        ParseError::InvalidStructure(format!("Invalid QUILL tag '{}': {}", quill_tag, e))
    })?;
    let parsed = ParsedDocument::new(fields, quill_ref);

    Ok((parsed, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_frontmatter() {
        let markdown = "# Hello World\n\nThis is a test.";
        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required QUILL field"));
    }

    #[test]
    fn test_with_frontmatter() {
        let markdown = r#"---
QUILL: test_quill
title: Test Document
author: Test Author
---

# Hello World

This is the body."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.body(), Some("\n# Hello World\n\nThis is the body."));
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Test Document"
        );
        assert_eq!(
            doc.get_field("author").unwrap().as_str().unwrap(),
            "Test Author"
        );
        assert_eq!(doc.fields().len(), 4); // title, author, body, CARDS
        assert_eq!(doc.quill_reference().name, "test_quill");
    }

    #[test]
    fn test_whitespace_frontmatter() {
        // Frontmatter with only whitespace has no QUILL → error
        let markdown = "---\n   \n---\n\n# Hello";
        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required QUILL field"));
    }

    #[test]
    fn test_complex_yaml_frontmatter() {
        let markdown = r#"---
QUILL: test_quill
title: Complex Document
tags:
  - test
  - yaml
metadata:
  version: 1.0
  nested:
    field: value
---

Content here."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.body(), Some("\nContent here."));
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Complex Document"
        );

        let tags = doc.get_field("tags").unwrap().as_array().unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].as_str().unwrap(), "test");
        assert_eq!(tags[1].as_str().unwrap(), "yaml");
    }

    #[test]
    fn test_with_defaults_empty_document() {
        use std::collections::HashMap;

        let mut defaults = HashMap::new();
        defaults.insert(
            "status".to_string(),
            QuillValue::from_json(serde_json::json!("draft")),
        );
        defaults.insert(
            "version".to_string(),
            QuillValue::from_json(serde_json::json!(1)),
        );

        // Create an empty parsed document
        let doc = ParsedDocument::new(HashMap::new(), QuillReference::latest("test".to_string()));
        let doc_with_defaults = doc.with_defaults(&defaults);

        // Check that defaults were applied
        assert_eq!(
            doc_with_defaults
                .get_field("status")
                .unwrap()
                .as_str()
                .unwrap(),
            "draft"
        );
        assert_eq!(
            doc_with_defaults
                .get_field("version")
                .unwrap()
                .as_number()
                .unwrap()
                .as_i64()
                .unwrap(),
            1
        );
    }

    #[test]
    fn test_with_defaults_preserves_existing_values() {
        use std::collections::HashMap;

        let mut defaults = HashMap::new();
        defaults.insert(
            "status".to_string(),
            QuillValue::from_json(serde_json::json!("draft")),
        );

        // Create document with existing status
        let mut fields = HashMap::new();
        fields.insert(
            "status".to_string(),
            QuillValue::from_json(serde_json::json!("published")),
        );
        let doc = ParsedDocument::new(fields, QuillReference::latest("test".to_string()));

        let doc_with_defaults = doc.with_defaults(&defaults);

        // Existing value should be preserved
        assert_eq!(
            doc_with_defaults
                .get_field("status")
                .unwrap()
                .as_str()
                .unwrap(),
            "published"
        );
    }

    #[test]
    fn test_with_defaults_partial_application() {
        use std::collections::HashMap;

        let mut defaults = HashMap::new();
        defaults.insert(
            "status".to_string(),
            QuillValue::from_json(serde_json::json!("draft")),
        );
        defaults.insert(
            "version".to_string(),
            QuillValue::from_json(serde_json::json!(1)),
        );

        // Create document with only one field
        let mut fields = HashMap::new();
        fields.insert(
            "status".to_string(),
            QuillValue::from_json(serde_json::json!("published")),
        );
        let doc = ParsedDocument::new(fields, QuillReference::latest("test".to_string()));

        let doc_with_defaults = doc.with_defaults(&defaults);

        // Existing field preserved, missing field gets default
        assert_eq!(
            doc_with_defaults
                .get_field("status")
                .unwrap()
                .as_str()
                .unwrap(),
            "published"
        );
        assert_eq!(
            doc_with_defaults
                .get_field("version")
                .unwrap()
                .as_number()
                .unwrap()
                .as_i64()
                .unwrap(),
            1
        );
    }

    #[test]
    fn test_with_defaults_no_defaults() {
        use std::collections::HashMap;

        let defaults = HashMap::new(); // Empty defaults map

        let doc = ParsedDocument::new(HashMap::new(), QuillReference::latest("test".to_string()));
        let doc_with_defaults = doc.with_defaults(&defaults);

        // No defaults should be applied
        assert!(doc_with_defaults.fields().is_empty());
    }

    #[test]
    fn test_with_defaults_complex_types() {
        use std::collections::HashMap;

        let mut defaults = HashMap::new();
        defaults.insert(
            "tags".to_string(),
            QuillValue::from_json(serde_json::json!(["default", "tag"])),
        );

        let doc = ParsedDocument::new(HashMap::new(), QuillReference::latest("test".to_string()));
        let doc_with_defaults = doc.with_defaults(&defaults);

        // Complex default value should be applied
        let tags = doc_with_defaults
            .get_field("tags")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].as_str().unwrap(), "default");
        assert_eq!(tags[1].as_str().unwrap(), "tag");
    }

    #[test]
    fn test_invalid_yaml() {
        // Real fence (QUILL first) with invalid YAML — size check happens, then YAML parse fails.
        let markdown = r#"---
QUILL: test_quill
title: [invalid yaml
author: missing close bracket
---

Content here."#;

        let result = decompose(markdown);
        assert!(result.is_err());
        // Error message now includes location context
        assert!(result.unwrap_err().to_string().contains("YAML error"));
    }

    #[test]
    fn test_unclosed_frontmatter() {
        // Real fence (QUILL first) without closer → spec §9 "not closed" error.
        let markdown = r#"---
QUILL: test_quill
title: Test
author: Test Author

Content without closing ---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not closed"));
    }

    // Extended metadata tests

    #[test]
    fn test_basic_tagged_block() {
        let markdown = r#"---
QUILL: test_quill
title: Main Document
---

Main body content.

---
CARD: items
name: Item 1
---

Body of item 1."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.body(), Some("\nMain body content.\n\n"));
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Main Document"
        );

        // Cards are now in CARDS array with CARD discriminator
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 1);

        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        assert_eq!(item.get("name").unwrap().as_str().unwrap(), "Item 1");
        assert_eq!(
            item.get(BODY_FIELD).unwrap().as_str().unwrap(),
            "\nBody of item 1."
        );
    }

    #[test]
    fn test_multiple_tagged_blocks() {
        let markdown = r#"---
QUILL: test_quill
---

---
CARD: items
name: Item 1
tags: [a, b]
---

First item body.

---
CARD: items
name: Item 2
tags: [c, d]
---

Second item body."#;

        let doc = decompose(markdown).unwrap();

        // Cards are in CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 2);

        let item1 = cards[0].as_object().unwrap();
        assert_eq!(item1.get("CARD").unwrap().as_str().unwrap(), "items");
        assert_eq!(item1.get("name").unwrap().as_str().unwrap(), "Item 1");

        let item2 = cards[1].as_object().unwrap();
        assert_eq!(item2.get("CARD").unwrap().as_str().unwrap(), "items");
        assert_eq!(item2.get("name").unwrap().as_str().unwrap(), "Item 2");
    }

    #[test]
    fn test_mixed_global_and_tagged() {
        let markdown = r#"---
QUILL: test_quill
title: Global
author: John Doe
---

Global body.

---
CARD: sections
title: Section 1
---

Section 1 content.

---
CARD: sections
title: Section 2
---

Section 2 content."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Global");
        assert_eq!(doc.body(), Some("\nGlobal body.\n\n"));

        // Cards are in unified CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(
            cards[0]
                .as_object()
                .unwrap()
                .get("CARD")
                .unwrap()
                .as_str()
                .unwrap(),
            "sections"
        );
    }

    #[test]
    fn test_empty_tagged_metadata() {
        let markdown = r#"---
QUILL: test_quill
---

---
CARD: items
---

Body without metadata."#;

        let doc = decompose(markdown).unwrap();

        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 1);

        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        assert_eq!(
            item.get(BODY_FIELD).unwrap().as_str().unwrap(),
            "\nBody without metadata."
        );
    }

    #[test]
    fn test_tagged_block_without_body() {
        let markdown = r#"---
QUILL: test_quill
---

---
CARD: items
name: Item
---"#;

        let doc = decompose(markdown).unwrap();

        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 1);

        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        assert_eq!(item.get(BODY_FIELD).unwrap().as_str().unwrap(), "");
    }

    #[test]
    fn test_name_collision_global_and_tagged() {
        let markdown = r#"---
QUILL: test_quill
items: "global value"
---

Body

---
CARD: items
name: Item
---

Item body"#;

        let result = decompose(markdown);
        assert!(result.is_ok(), "Name collision should be allowed now");
    }

    #[test]
    fn test_card_name_collision_with_array_field() {
        // CARD type names CAN now conflict with frontmatter field names
        let markdown = r#"---
QUILL: test_quill
items:
  - name: Global Item 1
    value: 100
---

Global body

---
CARD: items
name: Scope Item 1
---

Scope item 1 body"#;

        let result = decompose(markdown);
        assert!(
            result.is_ok(),
            "Collision with array field should be allowed"
        );
    }

    #[test]
    fn test_empty_global_array_with_card() {
        // CARD type names CAN now conflict with frontmatter field names
        let markdown = r#"---
QUILL: test_quill
items: []
---

Global body

---
CARD: items
name: Item 1
---

Item 1 body"#;

        let result = decompose(markdown);
        assert!(
            result.is_ok(),
            "Collision with empty array field should be allowed"
        );
    }

    #[test]
    fn test_reserved_field_body_rejected() {
        // BODY reserved inside a CARD block (requires prior QUILL fence per spec §4 F1).
        let markdown = r#"---
QUILL: test_quill
---

---
CARD: section
BODY: Test
---"#;

        let result = decompose(markdown);
        assert!(result.is_err(), "BODY is a reserved field name");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Reserved field name"));
    }

    #[test]
    fn test_reserved_field_cards_rejected() {
        // CARDS reserved inside the QUILL frontmatter.
        let markdown = r#"---
QUILL: test_quill
title: Test
CARDS: []
---"#;

        let result = decompose(markdown);
        assert!(result.is_err(), "CARDS is a reserved field name");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Reserved field name"));
    }

    #[test]
    fn test_delimiter_inside_fenced_code_block_backticks() {
        let markdown = r#"---
QUILL: test_quill
title: Test
---
Here is some code:

```yaml
---
fake: frontmatter
---
```

More content.
"#;

        let doc = decompose(markdown).unwrap();
        // The --- inside the code block should NOT be parsed as metadata
        assert!(doc.body().unwrap().contains("fake: frontmatter"));
        assert!(doc.get_field("fake").is_none());
    }

    #[test]
    fn test_tildes_are_fences() {
        // Per CommonMark: tildes (~~~) are valid fenced code block delimiters.
        // So --- inside ~~~ should NOT be parsed as a metadata block.
        let markdown = r#"---
QUILL: test_quill
title: Test
---
Here is some code:

~~~yaml
---
CARD: code_example
fake: frontmatter
---
~~~

More content.
"#;

        let doc = decompose(markdown).unwrap();
        assert!(doc.body().unwrap().contains("fake: frontmatter"));
        assert!(doc.get_field("fake").is_none());
    }

    #[test]
    fn test_four_backticks_are_fences() {
        // Per CommonMark: 4+ backticks are valid fenced code block delimiters.
        // So --- inside ```` should NOT be parsed as a metadata block.
        let markdown = r#"---
QUILL: test_quill
title: Test
---
Here is some code:

````yaml
---
CARD: code_example
fake: frontmatter
---
````

More content.
"#;

        let doc = decompose(markdown).unwrap();
        assert!(doc.body().unwrap().contains("fake: frontmatter"));
        assert!(doc.get_field("fake").is_none());
    }

    #[test]
    fn test_invalid_tag_syntax() {
        // CARD must follow a prior QUILL fence per spec §4 F1.
        let markdown = r#"---
QUILL: test_quill
---

---
CARD: Invalid-Name
title: Test
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid card field name"));
    }

    #[test]
    fn test_multiple_global_frontmatter_blocks() {
        // Two `---/---` blocks without QUILL/CARD sentinels both fail F1
        // and are delegated to CommonMark, so the document has no metadata
        // blocks and parsing fails with the missing-QUILL error.
        let markdown = r#"---
title: First
---

Body

---
author: Second
---

More body"#;

        let err = decompose(markdown).unwrap_err();
        let err_str = err.to_string();
        assert!(
            err_str.contains("QUILL"),
            "Error should mention missing QUILL: {}",
            err_str
        );
    }

    #[test]
    fn test_adjacent_blocks_different_tags() {
        let markdown = r#"---
QUILL: test_quill
---

---
CARD: items
name: Item 1
---

Item 1 body

---
CARD: sections
title: Section 1
---

Section 1 body"#;

        let doc = decompose(markdown).unwrap();

        // All cards in unified CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 2);

        // First card is "items" type
        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        assert_eq!(item.get("name").unwrap().as_str().unwrap(), "Item 1");

        // Second card is "sections" type
        let section = cards[1].as_object().unwrap();
        assert_eq!(section.get("CARD").unwrap().as_str().unwrap(), "sections");
        assert_eq!(section.get("title").unwrap().as_str().unwrap(), "Section 1");
    }

    #[test]
    fn test_order_preservation() {
        let markdown = r#"---
QUILL: test_quill
---

---
CARD: items
id: 1
---

First

---
CARD: items
id: 2
---

Second

---
CARD: items
id: 3
---

Third"#;

        let doc = decompose(markdown).unwrap();

        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 3);

        for (i, card) in cards.iter().enumerate() {
            let mapping = card.as_object().unwrap();
            assert_eq!(mapping.get("CARD").unwrap().as_str().unwrap(), "items");
            let id = mapping.get("id").unwrap().as_i64().unwrap();
            assert_eq!(id, (i + 1) as i64);
        }
    }

    #[test]
    fn test_product_catalog_integration() {
        let markdown = r#"---
QUILL: test_quill
title: Product Catalog
author: John Doe
date: 2024-01-01
---

This is the main catalog description.

---
CARD: products
name: Widget A
price: 19.99
sku: WID-001
---

The **Widget A** is our most popular product.

---
CARD: products
name: Gadget B
price: 29.99
sku: GAD-002
---

The **Gadget B** is perfect for professionals.

---
CARD: reviews
product: Widget A
rating: 5
---

"Excellent product! Highly recommended."

---
CARD: reviews
product: Gadget B
rating: 4
---

"Very good, but a bit pricey.""#;

        let doc = decompose(markdown).unwrap();

        // Verify global fields
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Product Catalog"
        );
        assert_eq!(
            doc.get_field("author").unwrap().as_str().unwrap(),
            "John Doe"
        );
        assert_eq!(
            doc.get_field("date").unwrap().as_str().unwrap(),
            "2024-01-01"
        );

        // Verify global body
        assert!(doc.body().unwrap().contains("main catalog description"));

        // All cards in unified CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 4); // 2 products + 2 reviews

        // First 2 are products
        let product1 = cards[0].as_object().unwrap();
        assert_eq!(product1.get("CARD").unwrap().as_str().unwrap(), "products");
        assert_eq!(product1.get("name").unwrap().as_str().unwrap(), "Widget A");
        assert_eq!(product1.get("price").unwrap().as_f64().unwrap(), 19.99);

        let product2 = cards[1].as_object().unwrap();
        assert_eq!(product2.get("CARD").unwrap().as_str().unwrap(), "products");
        assert_eq!(product2.get("name").unwrap().as_str().unwrap(), "Gadget B");

        // Last 2 are reviews
        let review1 = cards[2].as_object().unwrap();
        assert_eq!(review1.get("CARD").unwrap().as_str().unwrap(), "reviews");
        assert_eq!(
            review1.get("product").unwrap().as_str().unwrap(),
            "Widget A"
        );
        assert_eq!(review1.get("rating").unwrap().as_i64().unwrap(), 5);

        // Total fields: title, author, date, body, CARDS = 5
        assert_eq!(doc.fields().len(), 5);
    }

    #[test]
    fn taro_quill_directive() {
        let markdown = r#"---
QUILL: usaf_memo
memo_for: [ORG/SYMBOL]
memo_from: [ORG/SYMBOL]
---

This is the memo body."#;

        let doc = decompose(markdown).unwrap();

        // Verify quill tag is set
        assert_eq!(doc.quill_reference().name, "usaf_memo");

        // Verify fields from quill block become frontmatter
        assert_eq!(
            doc.get_field("memo_for").unwrap().as_array().unwrap()[0]
                .as_str()
                .unwrap(),
            "ORG/SYMBOL"
        );

        // Verify body
        assert_eq!(doc.body(), Some("\nThis is the memo body."));
    }

    #[test]
    fn test_quill_with_card_blocks() {
        let markdown = r#"---
QUILL: document
title: Test Document
---

Main body.

---
CARD: sections
name: Section 1
---

Section 1 body."#;

        let doc = decompose(markdown).unwrap();

        // Verify quill tag
        assert_eq!(doc.quill_reference().name, "document");

        // Verify global field from quill block
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Test Document"
        );

        // Verify card blocks work via CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(
            cards[0]
                .as_object()
                .unwrap()
                .get("CARD")
                .unwrap()
                .as_str()
                .unwrap(),
            "sections"
        );

        // Verify body
        assert_eq!(doc.body(), Some("\nMain body.\n\n"));
    }

    #[test]
    fn test_multiple_quill_directives_error() {
        // A second fence whose first key is QUILL (instead of CARD) fails F1
        // and is delegated to CommonMark. A near-miss warning is emitted per
        // spec §4.2; the document itself parses successfully with the stray
        // `---` lines preserved in the body.
        let markdown = r#"---
QUILL: first
---

---
QUILL: second
---"#;

        let output = ParsedDocument::from_markdown_with_warnings(markdown).unwrap();
        assert!(output
            .warnings
            .iter()
            .any(|w| w.code.as_deref() == Some("parse::near_miss_sentinel")
                && w.message.contains("QUILL")));
        assert!(output.document.body().unwrap().contains("QUILL: second"));
    }

    #[test]
    fn test_invalid_quill_ref() {
        let markdown = r#"---
QUILL: Invalid-Name
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid QUILL reference"));
    }

    #[test]
    fn test_quill_wrong_value_type() {
        let markdown = r#"---
QUILL: 123
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("QUILL value must be a string"));
    }

    #[test]
    fn test_card_wrong_value_type() {
        let markdown = r#"---
QUILL: test_quill
---

---
CARD: 123
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("CARD value must be a string"));
    }

    #[test]
    fn test_both_quill_and_card_error() {
        let markdown = r#"---
QUILL: test
CARD: items
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot specify both QUILL and CARD"));
    }

    #[test]
    fn test_blank_lines_in_frontmatter() {
        // New parsing standard: blank lines are allowed within YAML blocks
        let markdown = r#"---
QUILL: test_quill
title: Test Document
author: Test Author

description: This has a blank line above it
tags:
  - one
  - two
---

# Hello World

This is the body."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.body(), Some("\n# Hello World\n\nThis is the body."));
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Test Document"
        );
        assert_eq!(
            doc.get_field("author").unwrap().as_str().unwrap(),
            "Test Author"
        );
        assert_eq!(
            doc.get_field("description").unwrap().as_str().unwrap(),
            "This has a blank line above it"
        );

        let tags = doc.get_field("tags").unwrap().as_array().unwrap();
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_blank_lines_in_scope_blocks() {
        // Blank lines should be allowed in CARD blocks too
        let markdown = r#"---
QUILL: test_quill
---

---
CARD: items
name: Item 1

price: 19.99

tags:
  - electronics
  - gadgets
---

Body of item 1."#;

        let doc = decompose(markdown).unwrap();

        // Cards are in CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 1);

        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        assert_eq!(item.get("name").unwrap().as_str().unwrap(), "Item 1");
        assert_eq!(item.get("price").unwrap().as_f64().unwrap(), 19.99);

        let tags = item.get("tags").unwrap().as_array().unwrap();
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_triple_dash_in_body_without_sentinel_is_delegated() {
        // Triple-dash pairs without a QUILL or CARD sentinel fail F1 and are
        // delegated to CommonMark, so the `---` lines stay in the body.
        let markdown = r#"---
QUILL: test_quill
title: Test
---

First paragraph.

---

Second paragraph."#;

        let doc = decompose(markdown).unwrap();
        let body = doc.body().unwrap();
        assert!(body.contains("First paragraph."));
        assert!(body.contains("Second paragraph."));
        assert!(body.contains("---"));
    }

    #[test]
    fn test_lone_triple_dash_in_body_is_delegated() {
        // A single `---` line not preceded by a blank fails F2 and is left as
        // body content.
        let markdown = r#"---
QUILL: test_quill
title: Test
---

First paragraph.
---

Second paragraph."#;

        let doc = decompose(markdown).unwrap();
        let body = doc.body().unwrap();
        assert!(body.contains("First paragraph."));
        assert!(body.contains("Second paragraph."));
        assert!(body.contains("---"));
    }

    #[test]
    fn test_multiple_blank_lines_in_yaml() {
        // Multiple blank lines should also be allowed
        let markdown = r#"---
QUILL: test_quill
title: Test


author: John Doe


version: 1.0
---

Body content."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Test");
        assert_eq!(
            doc.get_field("author").unwrap().as_str().unwrap(),
            "John Doe"
        );
        assert_eq!(doc.get_field("version").unwrap().as_f64().unwrap(), 1.0);
    }

    #[test]
    fn test_html_comment_interaction() {
        let markdown = r#"<!---
---> the rest of the page content

---
QUILL: test_quill
key: value
---
"#;
        let doc = decompose(markdown).unwrap();

        // The comment should be ignored (or at least not cause a parse error)
        // The frontmatter should be parsed
        let key = doc.get_field("key").and_then(|v| v.as_str());
        assert_eq!(key, Some("value"));
    }
}
#[cfg(test)]
mod demo_file_test {
    use super::*;

    #[test]
    fn test_extended_metadata_demo_file() {
        let markdown = include_str!("../../fixtures/resources/extended_metadata_demo.md");
        let doc = decompose(markdown).unwrap();

        // Verify global fields
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Extended Metadata Demo"
        );
        assert_eq!(
            doc.get_field("author").unwrap().as_str().unwrap(),
            "Quillmark Team"
        );
        // version is parsed as a number by YAML
        assert_eq!(doc.get_field("version").unwrap().as_f64().unwrap(), 1.0);

        // Verify body
        assert!(doc
            .body()
            .unwrap()
            .contains("extended YAML metadata standard"));

        // All cards are now in unified CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 5); // 3 features + 2 use_cases

        // Count features and use_cases cards
        let features_count = cards
            .iter()
            .filter(|c| {
                c.as_object()
                    .unwrap()
                    .get("CARD")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    == "features"
            })
            .count();
        let use_cases_count = cards
            .iter()
            .filter(|c| {
                c.as_object()
                    .unwrap()
                    .get("CARD")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    == "use_cases"
            })
            .count();
        assert_eq!(features_count, 3);
        assert_eq!(use_cases_count, 2);

        // Check first card is a feature
        let feature1 = cards[0].as_object().unwrap();
        assert_eq!(feature1.get("CARD").unwrap().as_str().unwrap(), "features");
        assert_eq!(
            feature1.get("name").unwrap().as_str().unwrap(),
            "Tag Directives"
        );
    }

    #[test]
    fn test_input_size_limit() {
        // Create markdown larger than MAX_INPUT_SIZE (10 MB)
        let size = crate::error::MAX_INPUT_SIZE + 1;
        let large_markdown = "a".repeat(size);

        let result = decompose(&large_markdown);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Input too large"));
    }

    #[test]
    fn test_yaml_size_limit() {
        // Create YAML block larger than MAX_YAML_SIZE (1 MB)
        let mut markdown = String::from("---\nQUILL: test_quill\n");

        // Create a very large YAML field
        let size = crate::error::MAX_YAML_SIZE + 1;
        markdown.push_str("data: \"");
        markdown.push_str(&"x".repeat(size));
        markdown.push_str("\"\n---\n\nBody");

        let result = decompose(&markdown);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Input too large"));
    }

    #[test]
    fn test_input_within_size_limit() {
        // Create markdown just under the limit
        let size = 1000; // Much smaller than limit
        let markdown = format!(
            "---\nQUILL: test_quill\ntitle: Test\n---\n\n{}",
            "a".repeat(size)
        );

        let result = decompose(&markdown);
        assert!(result.is_ok());
    }

    #[test]
    fn test_yaml_within_size_limit() {
        // Create YAML block well within the limit
        let markdown = "---\nQUILL: test_quill\ntitle: Test\nauthor: John Doe\n---\n\nBody content";

        let result = decompose(markdown);
        assert!(result.is_ok());
    }

    #[test]
    fn test_yaml_depth_limit() {
        // Create deeply nested YAML that exceeds MAX_YAML_DEPTH (100 levels)
        // This tests serde-saphyr's Budget.max_depth enforcement
        let mut yaml_content = String::new();
        for i in 0..110 {
            yaml_content.push_str(&"  ".repeat(i));
            yaml_content.push_str(&format!("level{}: value\n", i));
        }

        let markdown = format!("---\n{}---\n\nBody", yaml_content);
        let result = decompose(&markdown);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // serde-saphyr returns "budget exceeded" or similar for depth violations
        assert!(
            err_msg.to_lowercase().contains("budget")
                || err_msg.to_lowercase().contains("depth")
                || err_msg.contains("YAML"),
            "Expected depth/budget error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_yaml_depth_within_limit() {
        // Create reasonably nested YAML (should succeed)
        let markdown = r#"---
QUILL: test_quill
level1:
  level2:
    level3:
      level4:
        value: test
---

Body content"#;

        let result = decompose(markdown);
        assert!(result.is_ok());
    }

    // Tests for guillemet preservation in parsing (guillemets are NOT converted during parsing)
    // Guillemet conversion now happens in process_plate, not during parsing
    #[test]
    fn test_chevrons_preserved_in_body_no_frontmatter() {
        let markdown = "---\nQUILL: test_quill\n---\nUse <<raw content>> here.";
        let doc = decompose(markdown).unwrap();

        // Body should preserve chevrons (conversion happens later in process_plate)
        assert_eq!(doc.body(), Some("Use <<raw content>> here."));
    }

    #[test]
    fn test_chevrons_preserved_in_body_with_frontmatter() {
        let markdown = r#"---
QUILL: test_quill
title: Test
---

Use <<raw content>> here."#;
        let doc = decompose(markdown).unwrap();

        // Body should preserve chevrons
        assert_eq!(doc.body(), Some("\nUse <<raw content>> here."));
    }

    #[test]
    fn test_chevrons_preserved_in_yaml_string() {
        let markdown = r#"---
QUILL: test_quill
title: Test <<with chevrons>>
---

Body content."#;
        let doc = decompose(markdown).unwrap();

        // YAML string values should preserve chevrons
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Test <<with chevrons>>"
        );
    }

    #[test]
    fn test_chevrons_preserved_in_yaml_array() {
        let markdown = r#"---
QUILL: test_quill
items:
  - "<<first>>"
  - "<<second>>"
---

Body."#;
        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_array().unwrap();
        assert_eq!(items[0].as_str().unwrap(), "<<first>>");
        assert_eq!(items[1].as_str().unwrap(), "<<second>>");
    }

    #[test]
    fn test_chevrons_preserved_in_yaml_nested() {
        let markdown = r#"---
QUILL: test_quill
metadata:
  description: "<<nested value>>"
---

Body."#;
        let doc = decompose(markdown).unwrap();

        let metadata = doc.get_field("metadata").unwrap().as_object().unwrap();
        assert_eq!(
            metadata.get("description").unwrap().as_str().unwrap(),
            "<<nested value>>"
        );
    }

    #[test]
    fn test_chevrons_preserved_in_code_blocks() {
        let markdown =
            "---\nQUILL: test_quill\n---\n```\n<<in code block>>\n```\n\n<<outside code block>>";
        let doc = decompose(markdown).unwrap();

        let body = doc.body().unwrap();
        // All chevrons should be preserved (no conversion during parsing)
        assert!(body.contains("<<in code block>>"));
        assert!(body.contains("<<outside code block>>"));
    }

    #[test]
    fn test_chevrons_preserved_in_inline_code() {
        let markdown =
            "---\nQUILL: test_quill\n---\n`<<in inline code>>` and <<outside inline code>>";
        let doc = decompose(markdown).unwrap();

        let body = doc.body().unwrap();
        // All chevrons should be preserved
        assert!(body.contains("`<<in inline code>>`"));
        assert!(body.contains("<<outside inline code>>"));
    }

    #[test]
    fn test_chevrons_preserved_in_tagged_block_body() {
        let markdown = r#"---
QUILL: test_quill
title: Main
---

Main body.

---
CARD: items
name: Item 1
---

Use <<raw>> here."#;
        let doc = decompose(markdown).unwrap();

        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        let item_body = item.get(BODY_FIELD).unwrap().as_str().unwrap();
        // Tagged block body should preserve chevrons
        assert!(item_body.contains("<<raw>>"));
    }

    #[test]
    fn test_chevrons_preserved_in_tagged_block_yaml() {
        let markdown = r#"---
QUILL: test_quill
title: Main
---

Main body.

---
CARD: items
description: "<<tagged yaml>>"
---

Item body."#;
        let doc = decompose(markdown).unwrap();

        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        // Tagged block YAML should preserve chevrons
        assert_eq!(
            item.get("description").unwrap().as_str().unwrap(),
            "<<tagged yaml>>"
        );
    }

    #[test]
    fn test_yaml_numbers_not_affected() {
        // Numbers should not be affected
        let markdown = r#"---
QUILL: test_quill
count: 42
---

Body."#;
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("count").unwrap().as_i64().unwrap(), 42);
    }

    #[test]
    fn test_yaml_booleans_not_affected() {
        // Booleans should not be affected
        let markdown = r#"---
QUILL: test_quill
active: true
---

Body."#;
        let doc = decompose(markdown).unwrap();
        assert!(doc.get_field("active").unwrap().as_bool().unwrap());
    }

    #[test]
    fn test_multiline_chevrons_preserved() {
        // Multiline chevrons should be preserved as-is
        let markdown = "---\nQUILL: test_quill\n---\n<<text\nacross lines>>";
        let doc = decompose(markdown).unwrap();

        let body = doc.body().unwrap();
        // Should contain the original chevrons
        assert!(body.contains("<<text"));
        assert!(body.contains("across lines>>"));
    }

    #[test]
    fn test_unmatched_chevrons_preserved() {
        let markdown = "---\nQUILL: test_quill\n---\n<<unmatched";
        let doc = decompose(markdown).unwrap();

        let body = doc.body().unwrap();
        // Unmatched should remain as-is
        assert_eq!(body, "<<unmatched");
    }
}

// Additional robustness tests
#[cfg(test)]
mod robustness_tests {
    use super::*;

    // Edge cases for delimiter handling

    #[test]
    fn test_empty_document() {
        let result = decompose("");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required QUILL field"));
    }

    #[test]
    fn test_only_whitespace() {
        let result = decompose("   \n\n   \t");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required QUILL field"));
    }

    #[test]
    fn test_only_dashes() {
        // "---" without newline is not a frontmatter delimiter → no blocks → QUILL error
        let result = decompose("---");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required QUILL field"));
    }

    #[test]
    fn test_dashes_in_middle_of_line() {
        // --- not at start of line should not be treated as delimiter
        let markdown = "---\nQUILL: test_quill\n---\nsome text --- more text";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.body(), Some("some text --- more text"));
    }

    #[test]
    fn test_four_dashes() {
        // ---- is not a valid delimiter — QUILL required
        let result = decompose("----\ntitle: Test\n----\n\nBody");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required QUILL field"));
    }

    #[test]
    fn test_crlf_line_endings() {
        // Windows-style line endings
        let markdown = "---\r\nQUILL: test_quill\r\ntitle: Test\r\n---\r\n\r\nBody content.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Test");
        assert!(doc.body().unwrap().contains("Body content."));
    }

    #[test]
    fn test_mixed_line_endings() {
        // Mix of \n and \r\n
        let markdown = "---\nQUILL: test_quill\r\ntitle: Test\r\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Test");
    }

    #[test]
    fn test_frontmatter_at_eof_no_trailing_newline() {
        // Frontmatter closed at EOF without trailing newline
        let markdown = "---\nQUILL: test_quill\ntitle: Test\n---";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Test");
        assert_eq!(doc.body(), Some(""));
    }

    #[test]
    fn test_empty_frontmatter() {
        // Empty/whitespace-only frontmatter has no QUILL → error
        let markdown = "---\n \n---\n\nBody content.";
        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required QUILL field"));
    }

    #[test]
    fn test_whitespace_only_frontmatter() {
        // Frontmatter with only whitespace → no QUILL → error
        let markdown = "---\n   \n\n   \n---\n\nBody.";
        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required QUILL field"));
    }

    // Unicode handling

    #[test]
    fn test_unicode_in_yaml_keys() {
        let markdown = "---\nQUILL: test_quill\ntitre: Bonjour\nタイトル: こんにちは\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("titre").unwrap().as_str().unwrap(), "Bonjour");
        assert_eq!(
            doc.get_field("タイトル").unwrap().as_str().unwrap(),
            "こんにちは"
        );
    }

    #[test]
    fn test_unicode_in_yaml_values() {
        let markdown = "---\nQUILL: test_quill\ntitle: 你好世界 🎉\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "你好世界 🎉"
        );
    }

    #[test]
    fn test_unicode_in_body() {
        let markdown = "---\nQUILL: test_quill\ntitle: Test\n---\n\n日本語テキスト with emoji 🚀";
        let doc = decompose(markdown).unwrap();
        assert!(doc.body().unwrap().contains("日本語テキスト"));
        assert!(doc.body().unwrap().contains("🚀"));
    }

    // YAML edge cases

    #[test]
    fn test_yaml_multiline_string() {
        let markdown = r#"---
QUILL: test_quill
description: |
  This is a
  multiline string
  with preserved newlines.
---

Body."#;
        let doc = decompose(markdown).unwrap();
        let desc = doc.get_field("description").unwrap().as_str().unwrap();
        assert!(desc.contains("multiline string"));
        assert!(desc.contains('\n'));
    }

    #[test]
    fn test_yaml_folded_string() {
        let markdown = r#"---
QUILL: test_quill
description: >
  This is a folded
  string that becomes
  a single line.
---

Body."#;
        let doc = decompose(markdown).unwrap();
        let desc = doc.get_field("description").unwrap().as_str().unwrap();
        // Folded strings join lines with spaces
        assert!(desc.contains("folded"));
    }

    #[test]
    fn test_yaml_null_value() {
        let markdown = "---\nQUILL: test_quill\noptional: null\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert!(doc.get_field("optional").unwrap().is_null());
    }

    #[test]
    fn test_yaml_empty_string_value() {
        let markdown = "---\nQUILL: test_quill\nempty: \"\"\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("empty").unwrap().as_str().unwrap(), "");
    }

    #[test]
    fn test_yaml_special_characters_in_string() {
        let markdown =
            "---\nQUILL: test_quill\nspecial: \"colon: here, and [brackets]\"\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(
            doc.get_field("special").unwrap().as_str().unwrap(),
            "colon: here, and [brackets]"
        );
    }

    #[test]
    fn test_yaml_nested_objects() {
        let markdown = r#"---
QUILL: test_quill
config:
  database:
    host: localhost
    port: 5432
  cache:
    enabled: true
---

Body."#;
        let doc = decompose(markdown).unwrap();
        let config = doc.get_field("config").unwrap().as_object().unwrap();
        let db = config.get("database").unwrap().as_object().unwrap();
        assert_eq!(db.get("host").unwrap().as_str().unwrap(), "localhost");
        assert_eq!(db.get("port").unwrap().as_i64().unwrap(), 5432);
    }

    // CARD block edge cases

    #[test]
    fn test_card_with_empty_body() {
        let markdown = r#"---
QUILL: test_quill
---

---
CARD: items
name: Item
---"#;
        let doc = decompose(markdown).unwrap();
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 1);
        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        assert_eq!(item.get(BODY_FIELD).unwrap().as_str().unwrap(), "");
    }

    #[test]
    fn test_card_consecutive_blocks() {
        // Per spec §4 F2 each metadata fence opener must be preceded by a
        // blank line (or start-of-file), so consecutive CARD blocks need a
        // blank separator.
        let markdown = r#"---
QUILL: test_quill
---

---
CARD: a
id: 1
---

---
CARD: a
id: 2
---"#;
        let doc = decompose(markdown).unwrap();
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(
            cards[0]
                .as_object()
                .unwrap()
                .get("CARD")
                .unwrap()
                .as_str()
                .unwrap(),
            "a"
        );
        assert_eq!(
            cards[1]
                .as_object()
                .unwrap()
                .get("CARD")
                .unwrap()
                .as_str()
                .unwrap(),
            "a"
        );
    }

    #[test]
    fn test_card_with_body_containing_dashes() {
        let markdown = r#"---
QUILL: test_quill
---

---
CARD: items
name: Item
---

Some text with --- dashes in it."#;
        let doc = decompose(markdown).unwrap();
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        let body = item.get(BODY_FIELD).unwrap().as_str().unwrap();
        assert!(body.contains("--- dashes"));
    }

    // QUILL directive edge cases

    #[test]
    fn test_quill_with_underscore_prefix() {
        let markdown = "---\nQUILL: _internal\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.quill_reference().name, "_internal");
    }

    #[test]
    fn test_quill_with_numbers() {
        let markdown = "---\nQUILL: form_8_v2\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.quill_reference().name, "form_8_v2");
    }

    #[test]
    fn test_quill_with_additional_fields() {
        let markdown = r#"---
QUILL: my_quill
title: Document Title
author: John Doe
---

Body content."#;
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.quill_reference().name, "my_quill");
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Document Title"
        );
        assert_eq!(
            doc.get_field("author").unwrap().as_str().unwrap(),
            "John Doe"
        );
    }

    // Error handling

    #[test]
    fn test_invalid_scope_name_uppercase() {
        let markdown = "---\nQUILL: test_quill\n---\n\n---\nCARD: ITEMS\n---\n\nBody.";
        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid card field name"));
    }

    #[test]
    fn test_invalid_scope_name_starts_with_number() {
        let markdown = "---\nCARD: 123items\n---\n\nBody.";
        let result = decompose(markdown);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_scope_name_with_hyphen() {
        let markdown = "---\nCARD: my-items\n---\n\nBody.";
        let result = decompose(markdown);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_quill_ref_uppercase() {
        let markdown = "---\nQUILL: MyQuill\n---\n\nBody.";
        let result = decompose(markdown);
        assert!(result.is_err());
    }

    #[test]
    fn test_yaml_syntax_error_missing_colon() {
        let markdown = "---\ntitle Test\n---\n\nBody.";
        let result = decompose(markdown);
        assert!(result.is_err());
    }

    #[test]
    fn test_yaml_syntax_error_bad_indentation() {
        let markdown = "---\nitems:\n- one\n - two\n---\n\nBody.";
        let result = decompose(markdown);
        // Bad indentation may or may not be an error depending on YAML parser
        // Just ensure it doesn't panic
        let _ = result;
    }

    // Body extraction edge cases

    #[test]
    fn test_body_with_leading_newlines() {
        let markdown =
            "---\nQUILL: test_quill\ntitle: Test\n---\n\n\n\nBody with leading newlines.";
        let doc = decompose(markdown).unwrap();
        // Body should preserve leading newlines after frontmatter
        assert!(doc.body().unwrap().starts_with('\n'));
    }

    #[test]
    fn test_body_with_trailing_newlines() {
        let markdown = "---\nQUILL: test_quill\ntitle: Test\n---\n\nBody.\n\n\n";
        let doc = decompose(markdown).unwrap();
        // Body should preserve trailing newlines
        assert!(doc.body().unwrap().ends_with('\n'));
    }

    #[test]
    fn test_no_body_after_frontmatter() {
        let markdown = "---\nQUILL: test_quill\ntitle: Test\n---";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.body(), Some(""));
    }

    // Tag name validation

    #[test]
    fn test_valid_tag_name_single_underscore() {
        assert!(is_valid_tag_name("_"));
    }

    #[test]
    fn test_valid_tag_name_underscore_prefix() {
        assert!(is_valid_tag_name("_private"));
    }

    #[test]
    fn test_valid_tag_name_with_numbers() {
        assert!(is_valid_tag_name("item1"));
        assert!(is_valid_tag_name("item_2"));
    }

    #[test]
    fn test_invalid_tag_name_empty() {
        assert!(!is_valid_tag_name(""));
    }

    #[test]
    fn test_invalid_tag_name_starts_with_number() {
        assert!(!is_valid_tag_name("1item"));
    }

    #[test]
    fn test_invalid_tag_name_uppercase() {
        assert!(!is_valid_tag_name("Items"));
        assert!(!is_valid_tag_name("ITEMS"));
    }

    #[test]
    fn test_invalid_tag_name_special_chars() {
        assert!(!is_valid_tag_name("my-items"));
        assert!(!is_valid_tag_name("my.items"));
        assert!(!is_valid_tag_name("my items"));
    }

    // Guillemet preprocessing in YAML

    #[test]
    fn test_guillemet_in_yaml_preserves_non_strings() {
        let markdown = r#"---
QUILL: test_quill
count: 42
price: 19.99
active: true
items:
  - first
  - 100
  - true
---

Body."#;
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("count").unwrap().as_i64().unwrap(), 42);
        assert_eq!(doc.get_field("price").unwrap().as_f64().unwrap(), 19.99);
        assert!(doc.get_field("active").unwrap().as_bool().unwrap());
    }

    #[test]
    fn test_guillemet_double_conversion_prevention() {
        // Ensure «» in input doesn't get double-processed
        let markdown = "---\nQUILL: test_quill\ntitle: Already «converted»\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        // Should remain as-is (not double-escaped)
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Already «converted»"
        );
    }

    #[test]
    fn test_allowed_card_field_collision() {
        let markdown = r#"---
QUILL: test_quill
my_card: "some global value"
---

---
CARD: my_card
title: "My Card"
---
Body
"#;
        // This should SUCCEED according to new PARSE.md
        let doc = decompose(markdown).unwrap();

        // Verify global field exists
        assert_eq!(
            doc.get_field("my_card").unwrap().as_str().unwrap(),
            "some global value"
        );

        // Verify Card exists in CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert!(!cards.is_empty());
        let card = cards
            .iter()
            .find(|v| v.get("CARD").and_then(|c| c.as_str()) == Some("my_card"))
            .expect("Card not found");
        assert_eq!(card.get("title").unwrap().as_str().unwrap(), "My Card");
    }

    #[test]
    fn test_yaml_custom_tags_in_frontmatter() {
        // User-defined YAML tags like !fill should be accepted and ignored
        let markdown = r#"---
QUILL: test_quill
memo_from: !fill 2d lt example
regular_field: normal value
---

Body content."#;
        let doc = decompose(markdown).unwrap();

        // The tag !fill should be ignored, value parsed as string "2d lt example"
        assert_eq!(
            doc.get_field("memo_from").unwrap().as_str().unwrap(),
            "2d lt example"
        );
        // Regular fields should still work
        assert_eq!(
            doc.get_field("regular_field").unwrap().as_str().unwrap(),
            "normal value"
        );
        assert_eq!(doc.body(), Some("\nBody content."));
    }

    /// Test the exact example from EXTENDED_MARKDOWN.md (lines 92-127)
    #[test]
    fn test_spec_example() {
        let markdown = r#"---
QUILL: blog_post
title: My Document
---
Main document body.

***

More content after horizontal rule.

---
CARD: section
heading: Introduction
---
Introduction content.

---
CARD: section
heading: Conclusion
---
Conclusion content.
"#;

        let doc = decompose(markdown).unwrap();

        // Verify global fields
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "My Document"
        );
        assert_eq!(doc.quill_reference().name, "blog_post");

        // Verify body contains horizontal rule (*** preserved)
        let body = doc.body().unwrap();
        assert!(body.contains("Main document body."));
        assert!(body.contains("***"));
        assert!(body.contains("More content after horizontal rule."));

        // Verify CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 2);

        // First card
        let card1 = cards[0].as_object().unwrap();
        assert_eq!(card1.get("CARD").unwrap().as_str().unwrap(), "section");
        assert_eq!(
            card1.get("heading").unwrap().as_str().unwrap(),
            "Introduction"
        );
        assert_eq!(
            card1.get("BODY").unwrap().as_str().unwrap(),
            "Introduction content.\n\n"
        );

        // Second card
        let card2 = cards[1].as_object().unwrap();
        assert_eq!(card2.get("CARD").unwrap().as_str().unwrap(), "section");
        assert_eq!(
            card2.get("heading").unwrap().as_str().unwrap(),
            "Conclusion"
        );
        assert_eq!(
            card2.get("BODY").unwrap().as_str().unwrap(),
            "Conclusion content.\n"
        );
    }

    #[test]
    fn test_missing_quill_field_errors() {
        let markdown = "---\ntitle: No quill here\n---\n# Body";
        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required QUILL field"));
    }
}
