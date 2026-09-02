//! Schema-bound typed writer: the front door for typed field writes.
//!
//! `Card::commit_field` asks the caller to fetch a [`FieldSchema`] per write.
//! [`Quill::writer`](crate::Quill::writer) binds the schema once instead, so
//! callers issue one verb (`set`) and never pass a type token or an `inline`
//! flag. An undeclared name is [`EditError::UnknownField`]: on the typed path
//! it is a typo, not a fallback. Opaque storage stays available through the raw
//! [`Card::store_field`](crate::Card::store_field) verb.
//!
//! ```ignore
//! let mut w = quill.writer(&mut doc);
//! w.set("subject", "Q3 results")?;       // richtext(inline) → strict content commit
//! w.set("qty", "3")?;                    // integer → strict coerce, stores 3
//! w.card(2)?.set("desc", content_json)?;  // card kind → CardSchema → field type
//! w.set_all([("a", "1"), ("b", "2")])?;  // batched, all-or-nothing
//! ```
//!
//! The writer holds `&mut Document` and `&QuillConfig`, so a bound `TypedWriter`
//! cannot cross a lifetime-free binding boundary; those surfaces construct one
//! per call from the quill handle.
//!
//! [`Quill::conform`](crate::Quill::conform) is this same strict commit driven
//! by the schema rather than by a caller, so what an ingestion lands and what a
//! write lands are the same bytes. Where a write refuses, conform leaves the
//! value authored under a `conform::*` warning.

use indexmap::IndexMap;

use crate::document::edit::resolve_field_write;
use crate::document::{Card, Document, EditError};
use crate::quill::{CardSchema, CardValues, DocumentValues, FieldSchema, QuillConfig};
use crate::value::QuillValue;
use crate::{Delta, DocPath};

/// A [`Document`] bound to its [`QuillConfig`] for typed writes. Construct with
/// [`Quill::writer`](crate::Quill::writer). Writes target the main card; use
/// [`card`](Self::card) for a composable card.
pub struct TypedWriter<'a> {
    config: &'a QuillConfig,
    doc: &'a mut Document,
}

impl<'a> TypedWriter<'a> {
    /// Bind `doc` to `config`. Prefer [`Quill::writer`](crate::Quill::writer).
    pub fn new(config: &'a QuillConfig, doc: &'a mut Document) -> Self {
        Self { config, doc }
    }

    /// Write a field on the main card, strict-committing it against the field's
    /// schema type. An undeclared name fails with [`EditError::UnknownField`]
    /// rather than falling to the opaque
    /// [`Card::store_field`](crate::Card::store_field). Other errors are those
    /// of `Card::commit_field`.
    pub fn set(&mut self, name: &str, value: impl Into<QuillValue>) -> Result<(), EditError> {
        let schema = Some(&self.config.main.fields);
        commit_impl(self.doc.main_mut(), schema, name, value)
    }

    /// Write several main-card fields atomically, the typed twin of
    /// [`Card::store_fields`](crate::Card::store_fields). Every field resolves
    /// before any is applied; on a violation nothing is written and every
    /// offending field comes back as a `(name, error)` pair, so a caller
    /// submitting a whole form sees every typo in one pass.
    pub fn set_all<K, V, I>(&mut self, fields: I) -> Result<(), Vec<(String, EditError)>>
    where
        K: Into<String>,
        V: Into<QuillValue>,
        I: IntoIterator<Item = (K, V)>,
    {
        let schema = Some(&self.config.main.fields);
        set_all_impl(self.doc.main_mut(), schema, fields)
    }

    /// Revise the main card's body from markdown: edit semantics, surviving
    /// anchors rebase, text [`Delta`] returned. Untyped, because a body carries
    /// no field schema to type against.
    pub fn revise_body(&mut self, markdown: &str) -> Result<Delta, EditError> {
        self.doc.main_mut().revise_body(markdown)
    }

    /// Revise a content field on the main card from authored text: typed *and*
    /// anchor-preserving. Surviving anchors rebase and the diffed result is
    /// schema-conformed (`richtext(inline)` rejects a multi-block result).
    /// An undeclared name fails with [`EditError::UnknownField`], as
    /// [`set`](Self::set).
    ///
    /// The codec comes from the declared type: `richtext` diffs markdown and
    /// rebases anchors; `plaintext` diffs the literal text and never imports
    /// markdown, so a byte-identical revise of a value carrying escapes is a
    /// byte no-op.
    pub fn revise_field(&mut self, name: &str, text: &str) -> Result<Delta, EditError> {
        let schema = Some(&self.config.main.fields);
        revise_impl(self.doc.main_mut(), schema, name, text)
    }

    /// Build a composable card of `kind`, typed-commit `fields` onto it,
    /// optionally set its body from markdown, and place it. `at` picks the
    /// position: `None` appends, `Some(i)` inserts at index `i`. The card is
    /// committed in full *before* it joins the document, so a rejected field
    /// (or an invalid kind, body, or out-of-range `at`) leaves the document
    /// untouched. Field errors use the all-or-nothing bundle of
    /// [`set_all`](Self::set_all); an invalid kind or body, or an out-of-range
    /// position, surfaces as a single-entry bundle keyed `$kind` / `$body`.
    pub fn add_card<K, V, I>(
        &mut self,
        kind: &str,
        fields: I,
        body: Option<&str>,
        at: Option<usize>,
    ) -> Result<(), Vec<(String, EditError)>>
    where
        K: Into<String>,
        V: Into<QuillValue>,
        I: IntoIterator<Item = (K, V)>,
    {
        let mut card = Card::new(kind).map_err(|e| vec![("$kind".to_string(), e)])?;
        let schema = self.config.card_kind(kind).map(|s| &s.fields);
        set_all_impl(&mut card, schema, fields)?;
        if let Some(md) = body {
            card.revise_body(md)
                .map_err(|e| vec![("$body".to_string(), e)])?;
        }
        match at {
            Some(index) => self.doc.insert_card(index, card),
            None => self.doc.push_card(card),
        }
        .map_err(|e| vec![("$kind".to_string(), e)])?;
        Ok(())
    }

    /// Remove the composable card at `index`, returning it. `None` when
    /// `index` is out of range.
    pub fn remove_card(&mut self, index: usize) -> Option<Card> {
        self.doc.remove_card(index)
    }

    /// Make the document's fields, bodies and cards exactly `values`: the typed
    /// lane widened from one field ([`set`](Self::set)) through one card's
    /// fields ([`set_all`](Self::set_all)) to the whole document, and the write
    /// twin of [`Quill::project`](crate::Quill::project).
    ///
    /// **Replace, not merge.** A declared field `values` does not name is
    /// removed, `values.cards` *is* the card list (matched to the document's by
    /// position when the kinds agree, rebuilt otherwise, truncated past its
    /// end), and `values.body` becomes the body. An undeclared name in `values`
    /// is [`EditError::UnknownField`], as on every typed write; an undeclared
    /// field the *document* carries is left alone, being outside this
    /// vocabulary rather than absent from it. An absent `ext` leaves `$ext`
    /// untouched — it is an open namespace this caller may not be the only
    /// writer of — while an empty one removes it.
    ///
    /// **A cell whose incoming value equals its projection is not written.**
    /// So `set_values(&quill.project(&doc))` is a byte no-op on any document,
    /// carrying through untouched cells the things a re-import cannot
    /// reproduce: identity anchors, content-only marks, `!must_fill` markers,
    /// YAML comments, a leaf that decodes under neither encoding, and a scalar
    /// shorthand the render floor reads as typed. Nothing is normalized that
    /// the consumer did not change.
    ///
    /// All-or-nothing: every cell resolves before any is written, and every
    /// refusal comes back with the [`DocPath`] it anchors at.
    ///
    /// A changed content cell is a **cold import**: anchors on it do not
    /// survive, as [`overwrite`](crate::Card::overwrite_field) does not carry
    /// them. Reach for [`revise_field`](Self::revise_field) per cell where they
    /// must. Cards match by position and kind, so deleting, inserting or
    /// reordering an entry rewrites every card after it; the structural verbs
    /// ([`add_card`](Self::add_card), [`remove_card`](Self::remove_card),
    /// `Document::move_card`) are the path that does not.
    pub fn set_values(
        &mut self,
        values: &DocumentValues,
    ) -> Result<(), Vec<(DocPath, EditError)>> {
        let mut errors = Vec::new();
        let main_plan = plan_card(
            self.doc.main(),
            Some(&self.config.main),
            &DocPath::main(),
            &values.fields,
            &values.body,
            values.ext.as_ref(),
            &mut errors,
        );

        let mut card_plans: Vec<(usize, CardPlan)> = Vec::new();
        let mut builds: Vec<(usize, Card)> = Vec::new();
        for (index, incoming) in values.cards.iter().enumerate() {
            let kind = (!incoming.kind.is_empty()).then_some(incoming.kind.as_str());
            let base = DocPath::card(kind, index);
            let schema = kind.and_then(|k| self.config.card_kind(k));
            match self.doc.card(index) {
                Some(current) if current.kind().unwrap_or_default() == incoming.kind => {
                    card_plans.push((
                        index,
                        plan_card(
                            current,
                            schema,
                            &base,
                            &incoming.fields,
                            &incoming.body,
                            incoming.ext.as_ref(),
                            &mut errors,
                        ),
                    ));
                }
                _ => match build_card(incoming, schema, &base, &mut errors) {
                    Some(card) => builds.push((index, card)),
                    None => {}
                },
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        main_plan.apply(self.doc.main_mut());
        for (index, plan) in card_plans {
            plan.apply(
                self.doc
                    .card_mut(index)
                    .expect("planned against a card at this index"),
            );
        }
        for (index, card) in builds {
            match self.doc.cards_mut().get_mut(index) {
                Some(slot) => *slot = card,
                // Every index past the end arrives in order, so the append
                // lands where the plan addressed it.
                None => self
                    .doc
                    .push_card(card)
                    .expect("kind validated by Card::new in the plan"),
            }
        }
        while self.doc.cards().len() > values.cards.len() {
            self.doc.remove_card(self.doc.cards().len() - 1);
        }
        Ok(())
    }

    /// A schema-bound writer for the composable card at `index`. The card's
    /// `$kind` resolves its [`CardSchema`]; an unknown kind carries no schema, so
    /// every typed write on it fails with [`EditError::UnknownField`] (write
    /// such a card opaquely through
    /// [`Card::store_field`](crate::Card::store_field)).
    /// [`EditError::IndexOutOfRange`] when `index` is out of range.
    pub fn card(&mut self, index: usize) -> Result<CardWriter<'_>, EditError> {
        let config = self.config;
        let len = self.doc.cards().len();
        let card = self
            .doc
            .card_mut(index)
            .ok_or(EditError::IndexOutOfRange { index, len })?;
        let schema = card.kind().and_then(|k| config.card_kind(k));
        Ok(CardWriter { schema, card })
    }
}

/// A single composable card bound to its [`CardSchema`], from
/// [`TypedWriter::card`]. Same `set` / `set_all` verbs as [`TypedWriter`].
pub struct CardWriter<'a> {
    schema: Option<&'a CardSchema>,
    card: &'a mut Card,
}

impl CardWriter<'_> {
    /// The card's `$kind`, if any.
    pub fn kind(&self) -> Option<&str> {
        self.card.kind()
    }

    /// Write a field on this card, strict-committed against the card's
    /// [`CardSchema`]. An undeclared field (or any field when the card kind is
    /// unknown) fails with [`EditError::UnknownField`] rather than storing
    /// opaquely.
    pub fn set(&mut self, name: &str, value: impl Into<QuillValue>) -> Result<(), EditError> {
        commit_impl(self.card, self.schema.map(|s| &s.fields), name, value)
    }

    /// Revise this card's body from markdown (edit semantics), returning the
    /// text [`Delta`]: the card twin of [`TypedWriter::revise_body`].
    pub fn revise_body(&mut self, markdown: &str) -> Result<Delta, EditError> {
        self.card.revise_body(markdown)
    }

    /// The card twin of [`TypedWriter::revise_field`], resolved against the
    /// card's [`CardSchema`].
    pub fn revise_field(&mut self, name: &str, text: &str) -> Result<Delta, EditError> {
        revise_impl(self.card, self.schema.map(|s| &s.fields), name, text)
    }

    /// Write several fields on this card atomically; see
    /// [`TypedWriter::set_all`]; an undeclared name aborts the whole batch with
    /// [`EditError::UnknownField`].
    pub fn set_all<K, V, I>(&mut self, fields: I) -> Result<(), Vec<(String, EditError)>>
    where
        K: Into<String>,
        V: Into<QuillValue>,
        I: IntoIterator<Item = (K, V)>,
    {
        set_all_impl(self.card, self.schema.map(|s| &s.fields), fields)
    }
}

/// Typed single-field commit shared by [`TypedWriter::set`] and
/// [`CardWriter::set`]. A `None` schema is an unknown card kind: every name on
/// it is undeclared.
fn commit_impl(
    card: &mut Card,
    fields_schema: Option<&IndexMap<String, FieldSchema>>,
    name: &str,
    value: impl Into<QuillValue>,
) -> Result<(), EditError> {
    match fields_schema.and_then(|m| m.get(name)) {
        Some(schema) => card.commit_field(name, value, schema),
        None => Err(EditError::unknown_field(name)),
    }
}

/// The anchor-preserving twin of [`commit_impl`], shared by
/// [`TypedWriter::revise_field`] and [`CardWriter::revise_field`].
fn revise_impl(
    card: &mut Card,
    fields_schema: Option<&IndexMap<String, FieldSchema>>,
    name: &str,
    text: &str,
) -> Result<Delta, EditError> {
    match fields_schema.and_then(|m| m.get(name)) {
        Some(schema) => card.revise_field_checked(name, text, schema),
        None => Err(EditError::unknown_field(name)),
    }
}

/// All-or-nothing batched write shared by [`TypedWriter::set_all`] and
/// [`CardWriter::set_all`]: resolve every field first, collecting every error.
/// A `None` schema is an unknown card kind: every name on it is undeclared.
/// One card's resolved writes, held until every cell in the document has
/// resolved. Applying cannot fail: each entry was produced by the same verb the
/// single-cell writes use.
#[derive(Default)]
struct CardPlan {
    fields: Vec<(String, QuillValue)>,
    removals: Vec<String>,
    body: Option<quillmark_content::Normalized>,
    ext: Option<Option<serde_json::Map<String, serde_json::Value>>>,
}

impl CardPlan {
    fn apply(self, card: &mut Card) {
        for name in self.removals {
            card.payload_mut().remove(&name);
        }
        for (name, stored) in self.fields {
            card.payload_mut().insert_unchecked(name, stored);
        }
        if let Some(content) = self.body {
            card.overwrite_body(content);
        }
        match self.ext {
            None => {}
            Some(None) => {
                card.payload_mut().take_ext();
            }
            Some(Some(map)) => card.payload_mut().set_ext(map),
        }
    }
}

/// Resolve one card's incoming values against `schema`, appending every refusal
/// to `errors` under its own [`DocPath`]. A `None` schema is a kind carrying no
/// declaration: every name on it is undeclared, as it is for
/// [`CardWriter::set`].
fn plan_card(
    card: &Card,
    schema: Option<&CardSchema>,
    base: &DocPath,
    fields: &indexmap::IndexMap<String, serde_json::Value>,
    body: &str,
    ext: Option<&serde_json::Map<String, serde_json::Value>>,
    errors: &mut Vec<(DocPath, EditError)>,
) -> CardPlan {
    let mut plan = CardPlan::default();
    let declared = schema.map(|s| &s.fields);
    // An undeclared name carrying the value the card already holds is what a
    // projection of a kind the schema does not declare reads back, so the
    // guard answers before the vocabulary does: an untouched card round-trips,
    // and only an edited or added cell is the typo the typed lane refuses.
    for (name, incoming) in fields {
        if declared.is_some_and(|m| m.contains_key(name)) {
            continue;
        }
        if card.payload().get(name).map(|v| v.as_json()) == Some(incoming) {
            continue;
        }
        errors.push((base.field(name), EditError::unknown_field(name)));
    }
    if let Some(declared) = declared {
        for (name, field_schema) in declared {
            let current = card.payload().get(name);
            let Some(incoming) = fields.get(name) else {
                if current.is_some() {
                    plan.removals.push(name.clone());
                }
                continue;
            };
            if current.is_some_and(|cur| {
                *incoming == crate::quill::project_field(name, cur, field_schema)
            }) {
                continue;
            }
            match resolve_field_write(
                name,
                QuillValue::from_json(incoming.clone()),
                field_schema,
            ) {
                Err(e) => errors.push((
                    e.doc_path(base).unwrap_or_else(|| base.field(name)),
                    e,
                )),
                Ok(stored) if current.is_some_and(|cur| cur.as_json() == stored.as_json()) => {}
                Ok(stored) => plan.fields.push((name.clone(), stored)),
            }
        }
    }
    if body != card.body_markdown() {
        match crate::document::import_body(body) {
            Ok(content) => plan.body = Some(content),
            Err(e) => errors.push((base.body(), EditError::Import(e))),
        }
    }
    match ext {
        None => {}
        Some(map) if map.is_empty() => {
            if card.ext().is_some() {
                plan.ext = Some(None);
            }
        }
        Some(map) => {
            if card.ext() != Some(map) {
                match crate::value::depth_check_meta_map(map.clone(), |max| {
                    EditError::ValueTooDeep { max }
                }) {
                    Ok(checked) => plan.ext = Some(Some(checked)),
                    Err(e) => errors.push((base.clone(), e)),
                }
            }
        }
    }
    plan
}

/// Build a whole card from incoming values, for a position whose kind does not
/// match the document's. Committed in full before it is placed, so a refusal
/// leaves the document untouched.
fn build_card(
    incoming: &CardValues,
    schema: Option<&CardSchema>,
    base: &DocPath,
    errors: &mut Vec<(DocPath, EditError)>,
) -> Option<Card> {
    let mut card = match Card::new(&incoming.kind) {
        Ok(card) => card,
        Err(e) => {
            errors.push((base.clone(), e));
            return None;
        }
    };
    let entries: Vec<(String, QuillValue)> = incoming
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), QuillValue::from_json(v.clone())))
        .collect();
    if let Err(bundle) = set_all_impl(&mut card, schema.map(|s| &s.fields), entries) {
        errors.extend(bundle.into_iter().map(|(name, e)| {
            (e.doc_path(base).unwrap_or_else(|| base.field(&name)), e)
        }));
        return None;
    }
    if !incoming.body.is_empty() {
        match crate::document::import_body(&incoming.body) {
            Ok(content) => card.overwrite_body(content),
            Err(e) => {
                errors.push((base.body(), EditError::Import(e)));
                return None;
            }
        }
    }
    if let Some(map) = incoming.ext.as_ref().filter(|m| !m.is_empty()) {
        if let Err(e) = card.store_ext(map.clone()) {
            errors.push((base.clone(), e));
            return None;
        }
    }
    Some(card)
}

fn set_all_impl<K, V, I>(
    card: &mut Card,
    fields_schema: Option<&IndexMap<String, FieldSchema>>,
    fields: I,
) -> Result<(), Vec<(String, EditError)>>
where
    K: Into<String>,
    V: Into<QuillValue>,
    I: IntoIterator<Item = (K, V)>,
{
    let fields: Vec<(String, QuillValue)> = fields
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();

    let mut resolved: Vec<(String, QuillValue)> = Vec::with_capacity(fields.len());
    let mut errors: Vec<(String, EditError)> = Vec::new();
    for (name, value) in fields {
        match fields_schema.and_then(|m| m.get(&name)) {
            Some(schema) => match resolve_field_write(&name, value, schema) {
                Ok(stored) => resolved.push((name, stored)),
                Err(e) => errors.push((name, e)),
            },
            None => errors.push((name.clone(), EditError::unknown_field(name))),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    // Every entry validated by `resolve_field_write` above; apply unchecked.
    for (name, stored) in resolved {
        card.payload_mut().insert_unchecked(name, stored);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Codec;
    use crate::document::{Card, Document};
    use crate::version::QuillReference;
    use std::str::FromStr;

    const QUILL_YAML: &str = "\
quill:
  name: memo
  backend: typst
  version: 1.0.0
  description: Editor test quill
main:
  fields:
    subject:
      type: richtext
      inline: true
    qty:
      type: integer
card_kinds:
  note:
    fields:
      body:
        type: richtext
";

    fn config() -> QuillConfig {
        QuillConfig::from_yaml(QUILL_YAML).expect("valid quill")
    }

    fn blank_doc() -> Document {
        Document::new(QuillReference::from_str("memo@1.0.0").unwrap())
    }

    #[test]
    fn set_resolves_schema_field_as_typed_commit() {
        let config = config();
        let mut doc = blank_doc();
        let mut ed = TypedWriter::new(&config, &mut doc);

        ed.set("qty", "3").unwrap();
        ed.set("subject", "Hello").unwrap();
        assert_eq!(
            doc.main().payload().get("qty").unwrap().as_json(),
            &serde_json::json!(3)
        );
        assert_eq!(doc.main().field_text("subject", Codec::Richtext).unwrap().unwrap(), "Hello");
    }

    #[test]
    fn set_rejects_unknown_field() {
        let config = config();
        let mut doc = blank_doc();
        let mut ed = TypedWriter::new(&config, &mut doc);
        let err = ed.set("notafield", "x").unwrap_err();
        assert_eq!(err.code(), "edit::unknown_field");
        assert!(doc.main().payload().get("notafield").is_none());
    }

    #[test]
    fn set_all_is_all_or_nothing() {
        let config = config();
        let mut doc = blank_doc();
        let mut ed = TypedWriter::new(&config, &mut doc);
        let errs = ed
            .set_all([("qty", "5"), ("subject", "bad\n\nblock")])
            .unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].0, "subject");
        assert!(doc.main().payload().get("qty").is_none());

        let mut ed = TypedWriter::new(&config, &mut doc);
        ed.set_all([("qty", "5"), ("subject", "ok")]).unwrap();
        assert_eq!(
            doc.main().payload().get("qty").unwrap().as_json(),
            &serde_json::json!(5)
        );
    }

    #[test]
    fn add_card_fuses_new_commit_push() {
        let config = config();
        let mut doc = blank_doc();
        let mut ed = TypedWriter::new(&config, &mut doc);
        ed.add_card("note", [("body", "**hi**")], Some("card body"), None)
            .unwrap();
        assert_eq!(doc.cards().len(), 1);
        assert_eq!(doc.cards()[0].kind(), Some("note"));
        assert_eq!(doc.cards()[0].field_text("body", Codec::Richtext).unwrap().unwrap(), "**hi**");
        assert_eq!(doc.cards()[0].body_markdown(), "card body");
    }

    #[test]
    fn add_card_at_inserts_and_remove_card_returns() {
        let config = config();
        let mut doc = blank_doc();
        {
            let mut ed = TypedWriter::new(&config, &mut doc);
            ed.add_card("note", [("body", "a")], None, None).unwrap();
            ed.add_card("note", [("body", "c")], None, None).unwrap();
            ed.add_card("note", [("body", "b")], None, Some(1)).unwrap();
        }
        let bodies: Vec<String> = doc
            .cards()
            .iter()
            .map(|c| c.field_text("body", Codec::Richtext).unwrap().unwrap())
            .collect();
        assert_eq!(bodies, ["a", "b", "c"]);

        // Out-of-range insert is transactional.
        {
            let mut ed = TypedWriter::new(&config, &mut doc);
            let errs = ed
                .add_card("note", [("body", "x")], None, Some(9))
                .unwrap_err();
            assert_eq!(errs[0].0, "$kind");
        }
        assert_eq!(doc.cards().len(), 3);

        {
            let mut ed = TypedWriter::new(&config, &mut doc);
            let removed = ed.remove_card(1).unwrap();
            assert_eq!(removed.field_text("body", Codec::Richtext).unwrap().unwrap(), "b");
            assert!(ed.remove_card(5).is_none());
        }
        assert_eq!(doc.cards().len(), 2);
    }

    #[test]
    fn add_card_is_transactional_on_bad_field() {
        let config = config();
        let mut doc = blank_doc();
        let mut ed = TypedWriter::new(&config, &mut doc);
        let errs = ed
            .add_card("note", [("stray", "x")], None, None)
            .unwrap_err();
        assert_eq!(errs[0].0, "stray");
        assert_eq!(errs[0].1.code(), "edit::unknown_field");
        assert_eq!(doc.cards().len(), 0);
    }

    #[test]
    fn card_writer_resolves_card_kind_schema() {
        let config = config();
        let mut doc = blank_doc();
        doc.push_card(Card::new("note").unwrap()).unwrap();

        let mut ed = TypedWriter::new(&config, &mut doc);
        let mut card_ed = ed.card(0).unwrap();
        card_ed.set("body", "**hi**").unwrap();
        let err = card_ed.set("stray", "v").unwrap_err();
        assert_eq!(err.code(), "edit::unknown_field");

        assert_eq!(doc.cards()[0].field_text("body", Codec::Richtext).unwrap().unwrap(), "**hi**");

        let mut ed = TypedWriter::new(&config, &mut doc);
        assert!(matches!(
            ed.card(9),
            Err(EditError::IndexOutOfRange { .. })
        ));
    }

    #[test]
    fn revise_field_is_typed_and_rejects_unknown_and_non_inline() {
        let config = config();
        let mut doc = blank_doc();
        let mut ed = TypedWriter::new(&config, &mut doc);
        let _delta = ed.revise_field("subject", "Hello").unwrap();
        assert_eq!(doc.main().field_text("subject", Codec::Richtext).unwrap().unwrap(), "Hello");

        let mut ed = TypedWriter::new(&config, &mut doc);
        assert_eq!(
            ed.revise_field("nope", "x").unwrap_err().code(),
            "edit::unknown_field"
        );
        // `richtext(inline)` rejects a multi-block result.
        let err = ed.revise_field("subject", "a\n\nb").unwrap_err();
        assert_eq!(err.code(), "edit::field_not_inline");
        assert_eq!(doc.main().field_text("subject", Codec::Richtext).unwrap().unwrap(), "Hello");
    }

    #[test]
    fn card_writer_revise_field_resolves_card_schema() {
        let config = config();
        let mut doc = blank_doc();
        doc.push_card(Card::new("note").unwrap()).unwrap();

        let mut ed = TypedWriter::new(&config, &mut doc);
        ed.card(0).unwrap().revise_field("body", "**hi**").unwrap();
        assert_eq!(doc.cards()[0].field_text("body", Codec::Richtext).unwrap().unwrap(), "**hi**");

        let mut ed = TypedWriter::new(&config, &mut doc);
        assert_eq!(
            ed.card(0)
                .unwrap()
                .revise_field("stray", "x")
                .unwrap_err()
                .code(),
            "edit::unknown_field"
        );
    }
}
