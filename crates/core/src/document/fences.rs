//! Line-oriented fence scanner for Quillmark Markdown.
//!
//! Implements the F1 (sentinel) and F2 (leading blank) rules from MARKDOWN.md §3–§4,
//! and detects CommonMark fenced code blocks so that `---` inside them is ignored.

use crate::error::ParseError;
use crate::{Diagnostic, Severity};

use super::assemble::MetadataBlock;
use super::sentinel::first_content_key;

/// Line-oriented view of the source, used for F1/F2 fence detection.
pub(super) struct Lines<'a> {
    pub(super) source: &'a str,
    pub(super) starts: Vec<usize>, // byte offset of each line's first character
}

impl<'a> Lines<'a> {
    pub(super) fn new(source: &'a str) -> Self {
        let mut starts = Vec::new();
        starts.push(0);
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        Self { source, starts }
    }
    pub(super) fn len(&self) -> usize {
        self.starts.len()
    }
    pub(super) fn line_start(&self, k: usize) -> usize {
        self.starts[k]
    }
    /// Byte position immediately after line k's trailing `\n` (or end-of-source
    /// if no newline follows).
    pub(super) fn line_end_inclusive(&self, k: usize) -> usize {
        if k + 1 < self.starts.len() {
            self.starts[k + 1]
        } else {
            self.source.len()
        }
    }
    /// Line text without its trailing line ending.
    pub(super) fn line_text(&self, k: usize) -> &'a str {
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
    pub(super) fn is_blank(&self, k: usize) -> bool {
        self.line_text(k).chars().all(char::is_whitespace)
    }
}

/// Returns true if `line` (without its line ending) is a `---` metadata-fence
/// marker per MARKDOWN.md §3: exactly three hyphens followed by optional
/// trailing whitespace (spaces or tabs).
pub(super) fn is_fence_marker_line(line: &str) -> bool {
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

/// Detect a CommonMark fenced code-block marker line. Returns `Some((char,
/// run_len))` if the line opens a fence, or `Some` with `is_closing=true` if
/// it closes one matching `open_fence`.
pub(super) fn code_fence_on_line(
    line: &str,
    open_fence: Option<(u8, usize)>,
) -> Option<(u8, usize, bool)> {
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

/// Number of bytes occupied by the opener `---[ \t]*\n` or `---[ \t]*\r\n`
/// starting at `abs_pos`. Only called on a line that already matched
/// `is_fence_marker_line`.
pub(super) fn fence_opener_len(markdown: &str, abs_pos: usize) -> usize {
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

/// Outcome of the fence-detection pass: the recognised metadata blocks, any
/// non-fatal diagnostics accumulated along the way, and (if applicable) the
/// first-fence F1 failure captured so the top-level error can be specific.
pub(super) type FenceScan = (Vec<MetadataBlock>, Vec<Diagnostic>, Option<(String, usize)>);

/// Find all metadata fences in the document per MARKDOWN.md §3–§4.
///
/// Implements fence rules F1 (sentinel) and F2 (leading blank). Returns
/// successfully detected blocks plus any lint warnings emitted for
/// near-miss sentinels (§4.2).
pub(super) fn find_metadata_blocks(markdown: &str) -> Result<FenceScan, ParseError> {
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
        let block = super::assemble::build_block(
            markdown,
            abs_pos,
            abs_closing_pos,
            block_end,
            blocks.len(),
        )?;
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
