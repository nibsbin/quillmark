//! Document seeding from a quill schema: commit each field's `example` and
//! leave every other field absent, so the render layer still supplies
//! `default`/blank. A committed `example` on a must-fill field carries the
//! `!must_fill` marker, so seeding and the blueprint stamp the same cells and a
//! fresh seed reads as incomplete exactly where a blank document does.

use quillmark_content::Content;

use super::Quill;
use crate::quill::CardSchema;
use crate::document::PayloadItem;
use crate::{Card, Document, Payload, QuillReference, QuillValue, SeedOverlay};

/// Build the seeded `(payload, body)` for one card schema, layering an optional
/// [`SeedOverlay`] over the schema-example base. Per field the precedence is
/// `overlay › example › absent`; the overlay may also add a field the base
/// omits (a `default`-only field with no `example`). The final fields are
/// ordered by schema declaration order (matching the blueprint). Only fields declared on the
/// schema are included: an overlay key naming no schema field is ignored here
/// (the editor-surface validator flags it). Body: `overlay › body.example ›
/// empty`, honored only when the kind enables bodies. The `$quill` / `$kind`
/// system metadata is attached by the caller.
///
/// **Seed-commits-rest**: every seeded content field lands at the resting form
/// [`Quill::conform`](crate::Quill::conform) enforces (`richtext` canonical
/// content, `plaintext` its literal string), so a seeded document is at rest
/// from birth and `conform(seed_document())` is a byte no-op. The commit runs
/// through the same strict write the typed writer uses ([`seeded_rest`]), which
/// is what makes the two agree; a schema-aware writer that left non-canonical
/// rest would recreate the construction-dependent divergence internally, moving
/// a hash on a seed → store → load → conform cycle nobody edited.
fn seed_parts(schema: &CardSchema, overlay: Option<&SeedOverlay>) -> (Payload, Content) {
    // Drive by `schema.fields` (declaration order), so the result is in
    // declaration order natively: no merge-then-sort. Per field the precedence
    // is `overlay › example_content › example › absent`: a richtext-bearing field
    // seeds from its pre-validated content companion, every other from its raw
    // `example`, and the overlay overrides either (and can supply a value for a
    // `default`-only field the base omits). An overlay key naming no schema
    // field is skipped: it is never iterated here.
    let mut items: Vec<PayloadItem> = Vec::new();
    for (name, field) in &schema.fields {
        // A variant field seeds only in the world the seed itself lands in,
        // resolved from the discriminant's own seeded value (`example:` ›
        // `default:` › blank).
        if !crate::quill::variant::in_play(field, |d| crate::quill::variant::seed_value(schema, d)) {
            continue;
        }
        let overlaid = overlay.and_then(|o| o.fields.get(name));
        let Some(value) = overlaid
            .or(field.example_content.as_ref())
            .or(field.example.as_ref())
        else {
            continue;
        };
        items.push(PayloadItem::Field {
            key: name.clone(),
            value: seeded_rest(name, value, field),
            // An `example` on a must-fill field is shape documentation, not
            // the answer, so it commits *carrying the marker*: that is what
            // lands a seed on the same cells the blueprint stamps. An overlay
            // value is exempt — `$seed` is a template author deciding, which is
            // the act the marker asks for.
            fill: overlaid.is_none() && field.must_fill(),
            nested_comments: Vec::new(),
        });
    }

    // Body region as a content: an overlay body (authored markdown) is imported;
    // otherwise the `body.example` content cache is used; else empty, and only
    // when bodies are enabled for the kind.
    let body = if schema.body_enabled() {
        if let Some(overlay_body) = overlay.and_then(|o| o.body.clone()) {
            crate::document::import_body(&overlay_body).unwrap_or_else(|_| Content::empty())
        } else if let Some(content) = schema.body.as_ref().and_then(|b| b.example_content.as_ref()) {
            quillmark_content::serial::from_canonical_value(content.as_json())
                .unwrap_or_else(|_| Content::empty())
        } else if let Some(example) = schema.body.as_ref().and_then(|b| b.example.as_ref()) {
            // Fallback for a schema built outside the loader (no cached content).
            crate::document::import_body(example).unwrap_or_else(|_| Content::empty())
        } else {
            Content::empty()
        }
    } else {
        Content::empty()
    };

    (Payload::from_items(items), body)
}

/// The form a seeded value commits at: the strict write's, for a field whose
/// type tree bears a content leaf; verbatim for every other field (a scalar's
/// authored shorthand is the typed write's to canonicalize, and conform leaves
/// it alone). A value the strict write refuses (an `example` the schema's own
/// validation flagged at load) stays authored, exactly as conform leaves it.
fn seeded_rest(name: &str, value: &QuillValue, field: &crate::quill::FieldSchema) -> QuillValue {
    if !crate::quill::config::field_contains_content(field) {
        return value.clone();
    }
    crate::document::edit::resolve_field_write(name, value.clone(), field)
        .unwrap_or_else(|_| value.clone())
}

/// `$quill` reference for the main card, as `name@version`. Falls back to a
/// versionless reference if the configured version is unparseable (it is
/// validated at quill load, so the fallback is defensive only).
fn main_reference(quill: &Quill) -> QuillReference {
    let config = quill.config();
    format!("{}@{}", config.name, config.version)
        .parse()
        .unwrap_or_else(|_| QuillReference::latest(config.name.clone()))
}

pub(crate) fn seed_main(quill: &Quill) -> Card {
    // The main card is never seeded from an overlay: `$seed` keys range over
    // composable `card_kinds`, and `main` is not one of them.
    let (mut payload, body) = seed_parts(&quill.config().main, None);
    payload.set_quill(main_reference(quill));
    // The root block carries `$kind: main` alongside `$quill` (see the
    // markdown spec); set it so a seeded main card round-trips through
    // `to_markdown()` exactly as the parser and blueprint emit it.
    payload.set_kind("main");
    Card::from_parts(payload, body)
}

pub(crate) fn seed_card_for_kind(
    quill: &Quill,
    card_kind: &str,
    overlay: Option<&SeedOverlay>,
) -> Option<Card> {
    let schema = quill.config().card_kind(card_kind)?;
    Some(seed_composable(schema, overlay))
}

/// Seed a single composable card from its schema and an optional overlay (sets
/// `$kind`, never `$quill`).
fn seed_composable(schema: &CardSchema, overlay: Option<&SeedOverlay>) -> Card {
    let (mut payload, body) = seed_parts(schema, overlay);
    payload.set_kind(schema.name.clone());
    Card::from_parts(payload, body)
}

pub(crate) fn seed_document(quill: &Quill) -> Document {
    // A fresh document carries no `$seed`, so every kind seeds from its schema
    // example base (overlay = `None`).
    let main = seed_main(quill);
    let cards = quill
        .config()
        .card_kinds
        .iter()
        .map(|schema| seed_composable(schema, None))
        .collect();
    Document::from_main_and_cards(main, cards)
}

#[cfg(test)]
mod tests;
