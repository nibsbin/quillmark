//! What a plate declines to typeset, and the warning a body draws for holding
//! it anyway.
//!
//! A quill's plate is free to reinterpret a construct the content holds —
//! absorb it into a neighbour, move its text elsewhere, typeset nothing at all
//! — and the render says nothing, because only the quill knows it did. Core
//! cannot detect it: the plate's output is ink, and the absence of ink where a
//! construct used to be is not a signal any backend reports.
//!
//! So the quill declares it instead, per body, as data
//! ([`BodyCardSchema::unsupported`]). That buys two things one diagnostic
//! producer could not:
//!
//! - **An answer before the render.** An editor reads the declaration off the
//!   schema and declines the gesture at the moment the author makes it. A
//!   warning that arrives with the rendered page is too late to be the input
//!   rule's answer, which is the question that raised this.
//! - **A walk, not a scatter.** One traversal of the content sees every
//!   occurrence at once, so a body holding forty rules draws one diagnostic
//!   carrying `count: 40` rather than forty identical ones.
//!
//! What it does not buy is verification. A declaration is a claim about the
//! plate that nothing checks, and a plate that drops a construct it never
//! declared stays as silent as before. This is documentation with a diagnostic
//! attached.
//!
//! The vocabulary is flat — one name per block kind, no `list_item > heading`
//! contextual forms. Nothing in the bundled quills needs a contextual
//! declaration, and the shape to reach for when one does is an open question
//! better answered by the case that raises it.

use std::collections::BTreeMap;

use quillmark_content::{Content, KnownIslandType, LineKind};

use crate::document::Document;
use crate::path::DocPath;
use crate::quill::types::BlockConstruct;
use crate::{Diagnostic, Quill, Severity};

use super::CardSchema;

/// The diagnostic code every declined construct rides.
pub const UNSUPPORTED_CONSTRUCT: &str = "plate::unsupported_construct";

impl Quill {
    /// One `plate::unsupported_construct` warning per (body, construct) this
    /// quill declares it does not typeset and `doc` holds anyway.
    ///
    /// Pre-render and schema-level, alongside `conform::*`: [`Quill::parse`]
    /// appends these to the `Parsed.warnings` it returns, and the walk is
    /// stateless, so a repeat call re-emits the identical set. A quill that
    /// declares nothing returns empty, which is every quill until one opts in.
    ///
    /// Non-fatal by construction. The content is representable, stores, and
    /// round-trips; it is the page that will not carry it, and that is the
    /// author's call to make once told.
    pub fn unsupported_constructs(&self, doc: &Document) -> Vec<Diagnostic> {
        let config = self.config();
        let mut diags = Vec::new();
        collect(
            &config.main,
            doc.main().body(),
            &DocPath::main_body(),
            &mut diags,
        );
        for (index, card) in doc.cards().iter().enumerate() {
            let kind = card.kind().unwrap_or("");
            // An unknown kind declares no body, so it declines nothing; the
            // render gate passes it and so does this.
            let Some(schema) = config.card_kind(kind) else {
                continue;
            };
            let path = DocPath::card(Some(kind), index).body();
            collect(schema, card.body(), &path, &mut diags);
        }
        diags
    }
}

/// Append one diagnostic per declared construct `body` actually holds, in the
/// declaration's own order so the set is stable across calls.
fn collect(schema: &CardSchema, body: &Content, path: &DocPath, out: &mut Vec<Diagnostic>) {
    let Some(declared) = schema.body.as_ref().map(|b| &b.unsupported) else {
        return;
    };
    // No `is_blank` short-circuit: blankness is about text, and a rule carries
    // none, so a body that is nothing but the construct being declined reads as
    // blank and would skip the walk that exists to catch it.
    if declared.is_empty() {
        return;
    }
    let present = census(body);
    for construct in declared {
        let Some(&count) = present.get(construct) else {
            continue;
        };
        let mut args = BTreeMap::new();
        args.insert("construct".to_string(), construct.as_str().into());
        args.insert("count".to_string(), count.into());
        out.push(
            Diagnostic::new(
                Severity::Warning,
                format!(
                    "this quill does not typeset {}: {count} in this body will not reach the page",
                    plural(*construct, count)
                ),
            )
            .with_code(UNSUPPORTED_CONSTRUCT.to_string())
            .with_path(path.to_string())
            .with_args(args),
        );
    }
}

/// English enough for the engine's own sentence; a consumer wording this
/// itself reads `construct` and `count` off `args` instead.
fn plural(construct: BlockConstruct, count: usize) -> String {
    let name = match construct {
        BlockConstruct::Heading => "heading",
        BlockConstruct::Rule => "horizontal rule",
        BlockConstruct::Code => "code block",
        BlockConstruct::List => "list",
        BlockConstruct::Quote => "block quote",
        BlockConstruct::Table => "table",
        BlockConstruct::Image => "image",
    };
    if count == 1 {
        format!("a {name}")
    } else {
        format!("{name}s")
    }
}

/// How many of each block construct `body` holds.
///
/// A container counts once per contiguous **run**, not once per line or per
/// item: a three-item list is one list, and a multi-paragraph quote is one
/// quote. A run opens where the previous line carried no like container at that
/// depth — like meaning same shape, so a sibling item (differing only by
/// `ordinal`) continues its list rather than opening another. Two adjacent
/// lists of identical shape therefore count as one, the same non-distinction
/// the model itself documents for adjacent quotes.
///
/// A leaf block counts per block: a rule and a heading are one line each, and a
/// code fence is counted at the line that opens it.
///
/// Islands are counted off `islands` rather than off the lines that hold them,
/// so an image counts wherever it sits — alone on its line as a block island,
/// or mid-sentence as an inline one. A quill that does not typeset images does
/// not typeset either.
fn census(body: &Content) -> BTreeMap<BlockConstruct, usize> {
    use quillmark_content::model::Container;

    /// A container's identity for run purposes: everything but which item.
    fn shape(container: &Container) -> Option<(BlockConstruct, bool, u64)> {
        match container {
            Container::ListItem { ordered, start, .. } => {
                Some((BlockConstruct::List, *ordered, *start))
            }
            Container::Quote => Some((BlockConstruct::Quote, false, 0)),
            // The open set: a container this build does not know names no
            // construct to decline.
            _ => None,
        }
    }

    let mut counts: BTreeMap<BlockConstruct, usize> = BTreeMap::new();
    let mut prev: Vec<Option<(BlockConstruct, bool, u64)>> = Vec::new();

    for island in &body.islands {
        match KnownIslandType::parse(&island.island_type) {
            Some(KnownIslandType::Table) => *counts.entry(BlockConstruct::Table).or_insert(0) += 1,
            Some(KnownIslandType::Image) => *counts.entry(BlockConstruct::Image).or_insert(0) += 1,
            // The open set: an island type this build does not know names no
            // construct to decline.
            None => {}
        }
    }

    for line in &body.lines {
        let here: Vec<_> = line.containers.iter().map(shape).collect();
        for (depth, shape) in here.iter().enumerate() {
            let Some((construct, _, _)) = shape else {
                continue;
            };
            if prev.get(depth) != Some(shape) {
                *counts.entry(*construct).or_insert(0) += 1;
            }
        }
        prev = here;

        // Islands are already counted; every other block kind counts here.
        let construct = match &line.kind {
            LineKind::Heading { .. } => Some(BlockConstruct::Heading),
            LineKind::Rule => Some(BlockConstruct::Rule),
            LineKind::Code { .. } if !line.continues => Some(BlockConstruct::Code),
            _ => None,
        };
        if let Some(construct) = construct {
            *counts.entry(construct).or_insert(0) += 1;
        }
    }
    counts
}
