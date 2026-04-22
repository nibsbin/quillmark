//! Assembly of fences and sentinels into a [`Document`].
//!
//! This module contains the top-level parsing glue: it calls the fence scanner,
//! extracts sentinels, and assembles a typed [`Document`] from the pieces.

use std::str::FromStr;

use indexmap::IndexMap;

use crate::error::ParseError;
use crate::value::QuillValue;
use crate::version::QuillReference;
use crate::Diagnostic;

use super::fences::{fence_opener_len, find_metadata_blocks};
use super::sentinel::extract_sentinels;
use super::{Card, Document};

/// An intermediate representation of one `---…---` metadata block.
#[derive(Debug)]
pub(super) struct MetadataBlock {
    pub(super) start: usize,                          // Position of opening "---"
    pub(super) end: usize,                            // Position after closing "---\n"
    pub(super) yaml_value: Option<serde_json::Value>, // Parsed YAML as JSON (None if empty or parse failed)
    pub(super) tag: Option<String>,                   // Field name from CARD key
    pub(super) quill_ref: Option<String>,             // Quill reference from QUILL key
}

/// Creates serde_saphyr Options with security budgets configured.
///
/// Uses MAX_YAML_DEPTH from limits.rs to limit nesting depth at the parser level,
/// which is more robust than heuristic-based pre-parse checks.
fn yaml_parse_options() -> serde_saphyr::Options {
    let budget = serde_saphyr::Budget {
        max_depth: super::limits::MAX_YAML_DEPTH,
        ..Default::default()
    };
    serde_saphyr::Options {
        budget: Some(budget),
        ..Default::default()
    }
}

/// Process YAML content for a recognized metadata fence and build a
/// `MetadataBlock`. The `is_first_block` flag governs whether `QUILL` is
/// expected (vs. `CARD`). Returns errors per spec §9.
pub(super) fn build_block(
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

/// Decompose markdown, discarding warnings. Test- and `from_markdown`-facing.
pub(super) fn decompose(markdown: &str) -> Result<Document, crate::error::ParseError> {
    decompose_with_warnings(markdown).map(|(doc, _)| doc)
}

/// Decompose markdown into a typed [`Document`], returning any non-fatal warnings
/// collected during fence scanning.
pub(super) fn decompose_with_warnings(
    markdown: &str,
) -> Result<(Document, Vec<Diagnostic>), crate::error::ParseError> {
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

    // Find all metadata blocks. F1/F2 already guarantee that block 0 carries
    // QUILL and that every subsequent block carries CARD.
    let (blocks, warnings, first_fence_issue) = find_metadata_blocks(markdown)?;

    if blocks.is_empty() {
        return Err(crate::error::ParseError::InvalidStructure(
            missing_quill_message(first_fence_issue),
        ));
    }

    // Block 0 is always the QUILL frontmatter (F1 guarantee).
    let frontmatter_block = &blocks[0];
    let quill_tag = frontmatter_block.quill_ref.clone().ok_or_else(|| {
        ParseError::InvalidStructure(
            "Missing required QUILL field. Add `QUILL: <name>` to the frontmatter.".to_string(),
        )
    })?;

    // Build frontmatter IndexMap (YAML content with QUILL stripped).
    let mut frontmatter: IndexMap<String, QuillValue> = IndexMap::new();
    match &frontmatter_block.yaml_value {
        Some(serde_json::Value::Object(mapping)) => {
            for (key, value) in mapping {
                frontmatter.insert(key.clone(), QuillValue::from_json(value.clone()));
            }
        }
        Some(serde_json::Value::Null) | None => {}
        Some(_) => {
            return Err(ParseError::InvalidStructure(
                "Invalid YAML frontmatter: expected a mapping".to_string(),
            ));
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
    let global_body = markdown[body_start..body_end].to_string();

    // Parse tagged blocks (CARD blocks) into typed Cards.
    let mut cards: Vec<Card> = Vec::new();
    for (idx, block) in blocks.iter().enumerate() {
        if let Some(ref tag_name) = block.tag {
            // Build the card's typed fields.
            let mut card_fields: IndexMap<String, QuillValue> = IndexMap::new();
            match &block.yaml_value {
                Some(serde_json::Value::Object(mapping)) => {
                    for (key, value) in mapping {
                        card_fields.insert(key.clone(), QuillValue::from_json(value.clone()));
                    }
                }
                Some(serde_json::Value::Null) | None => {}
                Some(_) => {
                    return Err(crate::error::ParseError::InvalidStructure(format!(
                        "Invalid YAML in card block '{}': expected a mapping",
                        tag_name
                    )));
                }
            }

            // Card body: between this block's end and the next block's start (or EOF).
            let card_body_start = block.end;
            let card_body_end = if idx + 1 < blocks.len() {
                blocks[idx + 1].start
            } else {
                markdown.len()
            };
            let card_body = markdown[card_body_start..card_body_end].to_string();

            cards.push(Card::new_internal(tag_name.clone(), card_fields, card_body));
        }
    }

    let quill_ref = QuillReference::from_str(&quill_tag).map_err(|e| {
        ParseError::InvalidStructure(format!("Invalid QUILL tag '{}': {}", quill_tag, e))
    })?;

    let doc = Document::new_internal(quill_ref, frontmatter, global_body, cards, warnings.clone());

    Ok((doc, warnings))
}
