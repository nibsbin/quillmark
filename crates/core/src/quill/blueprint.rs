//! Auto-generated Markdown blueprint for a Quill: an annotated reference
//! document dense enough to replace the schema for LLM consumers.
//!
//! `blueprint()` builds a [`Document`] and emits it through
//! [`Document::to_markdown`], so the blueprint round-trips through
//! `Document::parse` by construction: there is no second formatter.

use indexmap::IndexMap;

use super::{blank, CardSchema, FieldSchema, FieldType, QuillConfig};
use crate::document::emit::{saphyr_emit_flow, saphyr_emit_scalar};
use crate::document::prescan::NestedComment;
use crate::document::{Card, Document, Payload, PayloadItem};
use crate::value::{PathSegment, QuillValue};
use serde_json::{Map as JsonMap, Value as JsonValue};

impl QuillConfig {
    /// Generate the canonical annotated Markdown blueprint for this quill:
    /// the authoring surface handed to LLMs and humans, with every must-fill
    /// cell carrying the `!must_fill` marker. See module docs for the annotation
    /// grammar; the function is total over any valid `QuillConfig`.
    ///
    /// The "filled-out" twin of the blueprint is **seeding**
    /// ([`Quill::seed_document`](crate::Quill::seed_document)), a committed
    /// [`Document`] rather than an annotated string. See
    /// `prose/canon/BLUEPRINT.md`.
    ///
    /// The result is guaranteed schema-valid and parseable (every key
    /// present, every value type-correct). It is *not* guaranteed to render:
    /// that is the quill authoring contract on `plate.typ`; see
    /// `prose/canon/BLUEPRINT.md` §Guarantees.
    ///
    /// [`Document`]: crate::Document
    pub fn blueprint(&self) -> String {
        let main_desc = self
            .main
            .description
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| Some(self.description.as_str()).filter(|s| !s.is_empty()));

        let main = build_main_card(
            &self.main,
            &format!("{}@{}", self.name, self.version),
            main_desc,
        );
        let cards = self.card_kinds.iter().map(build_card).collect();

        Document::from_main_and_cards(main, cards).to_markdown()
    }
}

/// Whitespace-collapse a description into a single line; `None` when it
/// collapses to empty.
fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collapse_opt(text: &Option<String>) -> Option<String> {
    text.as_deref()
        .map(collapse)
        .filter(|clean| !clean.is_empty())
}

/// The text after `# ` for a body region (`Write … here.` placeholder or the
/// configured `body.example`), wrapped so `to_markdown` emits it verbatim
/// after the closing fence. Empty when the card has no body.
fn body_text(card: &CardSchema, fallback_kind: &str) -> String {
    if !card.body_enabled() {
        return String::new();
    }
    let example = card.body.as_ref().and_then(|b| b.example.as_deref());
    let fallback = format!("Write {} body here.", fallback_kind);
    let text = example.unwrap_or(fallback.as_str());
    format!("\n{}\n", text)
}

/// Build the root card: `$quill` (with the `# keep verbatim` inline reminder),
/// `$kind: main`, the optional description own-line comment, then the fields.
fn build_main_card(card: &CardSchema, quill_ref: &str, description: Option<&str>) -> Card {
    let reference = quill_ref
        .parse()
        .expect("quill name@version is always a valid QuillReference");
    let mut items = vec![
        PayloadItem::Quill { reference },
        PayloadItem::comment_inline("keep verbatim"),
        PayloadItem::Kind {
            value: "main".into(),
        },
    ];
    if let Some(desc) = description {
        items.push(PayloadItem::comment(collapse(desc)));
    }
    append_fields(&mut items, card);
    Card::from_parts(
        Payload::from_items(items),
        // The blueprint's output *is* the markdown surface, so it imports the
        // body text (a trusted example or a generated placeholder) here and
        // re-emits it via `to_markdown`. The empty-content fallback is defensive:
        // a placeholder or a load-validated example never over-nests.
        crate::document::import_body(&body_text(card, "main"))
            .unwrap_or_else(|_| quillmark_content::Content::empty()),
    )
}

/// Build a composable card: `$kind: <kind>`, the `composable (0..N)` role
/// comment, the optional description, then the fields.
fn build_card(card: &CardSchema) -> Card {
    let mut items = vec![
        PayloadItem::Kind {
            value: card.name.clone(),
        },
        PayloadItem::comment("composable (0..N)"),
    ];
    if let Some(desc) = collapse_opt(&card.description) {
        items.push(PayloadItem::comment(desc));
    }
    append_fields(&mut items, card);
    Card::from_parts(
        Payload::from_items(items),
        crate::document::import_body(&body_text(card, &card.name))
            .unwrap_or_else(|_| quillmark_content::Content::empty()),
    )
}

/// Append every field of a card as payload items, clustered by `ui.group` and
/// ordered by declaration order. Group order follows the card's `ui.groups`
/// registry when present.
fn append_fields(items: &mut Vec<PayloadItem>, card: &CardSchema) {
    let registry: Option<Vec<&str>> = card
        .ui
        .as_ref()
        .and_then(|u| u.groups.as_ref())
        .map(|r| r.0.iter().map(|g| g.id.as_str()).collect());
    for field in group_fields(card.fields.values(), registry.as_deref()) {
        append_field(items, field);
    }
}

/// Order fields by `ui.group` (ungrouped lead, then grouped clusters),
/// preserving declaration order within each cluster (`fields` arrives in
/// declaration order and the clustering is stable). Grouped clusters follow
/// the `registry` declaration order when a registry is present, else first-
/// appearance order (the deprecated implicit-group fallback); for a migrated
/// quill the two are identical. Grouping is purely positional (no banner); the
/// clusters are flattened into a single field stream.
fn group_fields<'a, I: IntoIterator<Item = &'a FieldSchema>>(
    fields: I,
    registry: Option<&[&str]>,
) -> Vec<&'a FieldSchema> {
    let mut groups: Vec<(Option<&str>, Vec<&FieldSchema>)> = Vec::new();
    for field in fields {
        let group = field.ui.as_ref().and_then(|u| u.group.as_deref());
        match groups.iter_mut().find(|(g, _)| *g == group) {
            Some(slot) => slot.1.push(field),
            None => groups.push((group, vec![field])),
        }
    }
    // Ungrouped is the implicit leading pseudo-group (rank 0). Grouped clusters
    // sort by registry position shifted past it (`pos + 1`); with no registry
    // every group ties at `usize::MAX` and the stable sort preserves first
    // appearance.
    groups.sort_by_key(|(g, _)| match g {
        None => 0,
        Some(id) => registry
            .and_then(|order| order.iter().position(|o| o == id))
            .map(|pos| pos + 1)
            .unwrap_or(usize::MAX),
    });
    groups.into_iter().flat_map(|(_, fields)| fields).collect()
}

/// Append one top-level field. Dispatches typed tables (`array<object>`) and
/// typed dictionaries (`object` with `properties`) to their per-property
/// builders; everything else is a scalar/array cell.
fn append_field(items: &mut Vec<PayloadItem>, field: &FieldSchema) {
    // Typed table: an array whose element is an object with properties.
    if matches!(field.r#type, FieldType::Array) {
        if let Some(elem) = &field.items {
            if matches!(elem.r#type, FieldType::Object) {
                if let Some(props) = &elem.properties {
                    append_typed_table(items, field, props);
                    return;
                }
            }
        }
    }

    // Typed dictionary: standalone object with defined properties.
    if matches!(field.r#type, FieldType::Object) {
        if let Some(props) = &field.properties {
            append_typed_dict(items, field, props);
            return;
        }
    }

    append_scalar(items, field);
}

/// Push the leading prose comments for a *top-level* field: the description,
/// then the `# e.g.` hint. `eg_when` gates the hint: a scalar surfaces it only
/// when a `default:` already occupies the cell (otherwise the example *is* the
/// cell's value), while typed containers always surface it (their example never
/// inlines). The gate is a value-axis question and stays keyed on `default`:
/// `must_fill` never moves an example between the cell and the hint.
fn push_leading(items: &mut Vec<PayloadItem>, field: &FieldSchema, eg_when: bool) {
    if let Some(desc) = collapse_opt(&field.description) {
        items.push(PayloadItem::comment(desc));
    }
    if eg_when {
        if let Some(eg) = field.example.as_ref() {
            items.push(PayloadItem::comment(format!("e.g. {}", eg_hint(eg))));
        }
    }
}

/// The cell's `(value, fill)` for a scalar/array/richtext leaf: the two axes
/// read independently. The value is `default:` › `example:` › bare null; the
/// marker is the field's derived `must_fill`. A field carrying both a default
/// and `must_fill: true` therefore shows the default *and* asks for
/// confirmation, and an explicitly optional field with nothing to suggest emits
/// a bare unmarked cell (the type annotation alone tells the reader what goes
/// there).
fn scalar_cell(field: &FieldSchema) -> (JsonValue, bool) {
    (scalar_value(field), field.must_fill())
}

/// The value half. A richtext leaf never inlines an example: its `example:`
/// surfaces as a `# e.g.` hint instead (see `append_scalar`), so its
/// defaultless cell is always bare.
fn scalar_value(field: &FieldSchema) -> JsonValue {
    if let Some(default) = &field.default {
        return default.as_json().clone();
    }
    if matches!(field.r#type, FieldType::RichText { .. }) {
        return JsonValue::Null;
    }
    match field.example.as_ref() {
        Some(eg) => eg.as_json().clone(),
        None => JsonValue::Null,
    }
}

/// Append a scalar / scalar-array / richtext field as a single payload field
/// plus its trailing inline type annotation.
fn append_scalar(items: &mut Vec<PayloadItem>, field: &FieldSchema) {
    // A richtext field never inlines its `example:` as the marker value, so
    // (unlike other Unendorsed scalars) the example would vanish entirely.
    // Surface it as a `# e.g.` hint instead (no-ops when no `example:` is set).
    let eg_when = field.default.is_some() || matches!(field.r#type, FieldType::RichText { .. });
    push_leading(items, field, eg_when);
    let (json, fill) = scalar_cell(field);
    items.push(PayloadItem::Field {
        key: field.name.clone(),
        value: QuillValue::from_json(json),
        fill,
        nested_comments: Vec::new(),
    });
    items.push(PayloadItem::comment_inline(type_expression(field)));
}

/// Build the per-property body of a defaultless typed container: the value
/// mapping (each property at its own cell, in declaration order), the nested
/// comments (description + `# e.g.` + inline type annotation, addressed by
/// `container_path`/slot), and the nested fill paths. `prefix` is the container
/// path of the mapping relative to the field value (`[]` for a typed dict,
/// `[Index(0)]` for a typed table's synthetic row).
fn build_property_mapping(
    props: &IndexMap<String, Box<FieldSchema>>,
    prefix: &[PathSegment],
) -> (
    JsonMap<String, JsonValue>,
    Vec<NestedComment>,
    Vec<Vec<PathSegment>>,
) {
    let mut map = JsonMap::new();
    let mut nested = Vec::new();
    let mut fills = Vec::new();
    for (slot, prop) in props.values().map(|b| b.as_ref()).enumerate() {
        if let Some(desc) = collapse_opt(&prop.description) {
            nested.push(NestedComment {
                container_path: prefix.to_vec(),
                position: slot,
                text: desc,
                inline: false,
            });
        }
        // `# e.g.` only when a `default:` holds the cell (see `push_leading`).
        if prop.default.is_some() {
            if let Some(eg) = prop.example.as_ref() {
                nested.push(NestedComment {
                    container_path: prefix.to_vec(),
                    position: slot,
                    text: format!("e.g. {}", eg_hint(eg)),
                    inline: false,
                });
            }
        }
        let (json, fill) = scalar_cell(prop);
        map.insert(prop.name.clone(), json);
        if fill {
            let mut path = prefix.to_vec();
            path.push(PathSegment::Key(prop.name.clone()));
            fills.push(path);
        }
        nested.push(NestedComment {
            container_path: prefix.to_vec(),
            position: slot,
            text: type_expression(prop),
            inline: true,
        });
    }
    (map, nested, fills)
}

/// Append a typed-dictionary field (`object` with `properties`). With a
/// `default:`: render the default mapping (`{}` expands to the blank-filled
/// shape; a non-empty partial default is rendered verbatim, all unmarked).
/// Without one: recurse per property with leaf-level markers and annotations;
/// the container key itself is untagged, since `!must_fill` is rejected on a
/// mapping (`prose/references/markdown-spec.md` §3.4). A typed dictionary's own
/// `must_fill:` is therefore inert — the obligation lives on its leaves.
fn append_typed_dict(
    items: &mut Vec<PayloadItem>,
    field: &FieldSchema,
    props: &IndexMap<String, Box<FieldSchema>>,
) {
    push_leading(items, field, true);

    let (value, nested, fills) = match field.default.as_ref().map(|d| d.as_json()) {
        // `default: {}` → expand to the field's zero-filled shape so every key
        // is shown; all leaves are Endorsed-by-the-container, hence unmarked
        // and unannotated (uniform with a concrete default).
        Some(JsonValue::Object(map)) if map.is_empty() => {
            (blank(field).into_json(), Vec::new(), Vec::new())
        }
        // Concrete default (object or otherwise) → rendered verbatim, unmarked.
        Some(default) => (default.clone(), Vec::new(), Vec::new()),
        // Unendorsed → per-property recursion at the mapping root.
        None => {
            let (map, nested, fills) = build_property_mapping(props, &[]);
            (JsonValue::Object(map), nested, fills)
        }
    };

    push_container_field(items, &field.name, value, nested, fills, field);
}

/// Append a typed-table field (`array<object>`). With a `default:`: render the
/// default rows verbatim (including `default: []`, which stays inline `[]`:
/// arrays do not expand). Without one: emit one synthetic row carrying each
/// property's leaf-level marker and annotation; the container key itself is
/// untagged, so the field's own `must_fill:` is inert (as for a typed dict).
fn append_typed_table(
    items: &mut Vec<PayloadItem>,
    field: &FieldSchema,
    item_props: &IndexMap<String, Box<FieldSchema>>,
) {
    push_leading(items, field, true);

    let (value, nested, fills) = match field.default.as_ref().map(|d| d.as_json()) {
        // Any default (including `[]`) is shippable as-is, rendered verbatim.
        Some(default) => (default.clone(), Vec::new(), Vec::new()),
        // Row type declares no properties (schema-invalid in practice): emit a
        // type-valid empty array rather than a null synthetic row.
        None if item_props.is_empty() => (JsonValue::Array(Vec::new()), Vec::new(), Vec::new()),
        // Unendorsed → one synthetic row, per-property markers at `[Index(0)]`.
        None => {
            let (row, nested, fills) = build_property_mapping(item_props, &[PathSegment::Index(0)]);
            (
                JsonValue::Array(vec![JsonValue::Object(row)]),
                nested,
                fills,
            )
        }
    };

    push_container_field(items, &field.name, value, nested, fills, field);
}

/// Push a typed-container field (value + nested comments + nested fills) and
/// its trailing inline type annotation. The top-level `fill` flag is always
/// `false`: typed containers are tagged on their leaves, never the container.
fn push_container_field(
    items: &mut Vec<PayloadItem>,
    key: &str,
    value: JsonValue,
    nested_comments: Vec<NestedComment>,
    fills: Vec<Vec<PathSegment>>,
    field: &FieldSchema,
) {
    let mut quill_value = QuillValue::from_json(value);
    for path in &fills {
        quill_value.set_fill_at(path);
    }
    items.push(PayloadItem::Field {
        key: key.to_string(),
        value: quill_value,
        fill: false,
        nested_comments,
    });
    items.push(PayloadItem::comment_inline(type_expression(field)));
}

/// Build the inline annotation body (without the leading `# `): purely the
/// structural type expression `<type>[<format>]`. Shippability is carried by
/// the value cell alone (a concrete value is shippable as-is, a `!must_fill`
/// marker asks to be filled) so the annotation needs no cell-state tag.
fn type_expression(field: &FieldSchema) -> String {
    if let Some(values) = &field.enum_values {
        return format!("enum<{}>", values.join(" | "));
    }
    match field.r#type {
        FieldType::String => "string".into(),
        FieldType::Number => "number".into(),
        FieldType::Integer => "integer".into(),
        FieldType::Boolean => "boolean".into(),
        FieldType::Object => "object".into(),
        // The type names the role; the `<markdown>` format slot names the
        // surface encoding an author writes (and `to_markdown` re-emits).
        FieldType::RichText { inline: false } => "richtext<markdown>".into(),
        FieldType::RichText { inline: true } => "richtext(inline)<markdown>".into(),
        // The `<plain>` format slot names the literal codec (`from_plaintext`/
        // `to_plaintext`): content the author navigates but which takes no
        // markup, distinct from richtext's `<markdown>` surface.
        FieldType::PlainText { inline: false } => "plaintext<plain>".into(),
        FieldType::PlainText { inline: true } => "plaintext(inline)<plain>".into(),
        // `enum` fields always carry `enum_values`, so the early return above
        // handles them; this arm is the defensive fallback for a valueless enum.
        FieldType::Enum => "enum".into(),
        FieldType::Date => "date<YYYY-MM-DD>".into(),
        FieldType::DateTime => "datetime<YYYY-MM-DDThh:mm[:ss]>".into(),
        // The element type comes from `items`; a scalar element gives
        // `array<string>`/`array<integer>`/`array<markdown>`, an object
        // element gives `array<object>`.
        FieldType::Array => {
            let item = field
                .items
                .as_ref()
                .map(|it| type_expression(it))
                .unwrap_or_else(|| "string".into());
            format!("array<{}>", item)
        }
    }
}

/// Format an example value as a compact one-line hint. Arrays and objects
/// render as YAML flow collections (`[a, b, c]`, `{k: v}`) so multi-element
/// shape information is preserved without expanding into multiple comment
/// lines.
fn eg_hint(example: &QuillValue) -> String {
    match example.as_json() {
        v @ (serde_json::Value::Array(_) | serde_json::Value::Object(_)) => saphyr_emit_flow(v),
        val => saphyr_emit_scalar(val),
    }
}

#[cfg(test)]
mod tests {
    use crate::quill::QuillConfig;
    use crate::Document;

    fn cfg(yaml: &str) -> QuillConfig {
        QuillConfig::from_yaml(yaml).expect("valid yaml")
    }

    #[test]
    fn must_fill_markdown_example_surfaces_as_eg_hint_not_inline_value() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    bio: { type: richtext, example: "Hello world" }
"#)
        .blueprint();
        assert!(t.contains("# e.g. Hello world\nbio: !must_fill # richtext<markdown>\n"));
    }

    #[test]
    fn a_default_and_an_explicit_must_fill_show_both() {
        // The two axes are independent: the value cell carries the default (so
        // the document renders something safe) and the marker still asks a human
        // to confirm it. This cell had no spelling while `default:` carried both.
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    classification: { type: enum, values: [UNCLASSIFIED, CUI], default: UNCLASSIFIED, must_fill: true }
"#)
        .blueprint();
        assert!(
            t.contains("classification: !must_fill UNCLASSIFIED # enum<UNCLASSIFIED | CUI>\n"),
            "{t}"
        );
    }

    #[test]
    fn an_opted_out_field_with_nothing_to_suggest_emits_a_bare_unmarked_cell() {
        // The cell shape the emitter never had to produce: no default, no
        // example, no obligation. It stays in the blueprint rather than being
        // omitted — the blueprint is the document's full shape, and the type
        // annotation is exactly what a reader needs to fill it if they want to.
        // The explicit `null` is the emitter's spelling of an empty unmarked
        // cell; it parses back to the same state a missing key would.
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    note: { type: string, must_fill: false }
"#)
        .blueprint();
        assert!(t.contains("\nnote: null # string\n"), "{t}");
        assert!(!t.contains("!must_fill"), "{t}");

        let doc = Document::parse(&t).expect("the bare cell parses").document;
        assert_eq!(doc, Document::parse(&doc.to_markdown()).expect("re-emit").document);
    }

    #[test]
    fn an_opted_out_field_keeps_its_example_as_the_cell_value() {
        // The value axis is untouched by the marker axis: `example:` still
        // inlines when no `default:` holds the cell, marker or no marker.
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    note: { type: string, example: A note, must_fill: false }
"#)
        .blueprint();
        assert!(t.contains("note: A note # string\n"), "{t}");
        assert!(!t.contains("e.g."), "the example is the cell, not a hint: {t}");
    }

    #[test]
    fn endorsed_field_with_example_does_not_use_example_as_value() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    status: { type: string, default: draft, example: final }
"#)
        .blueprint();
        assert!(t.contains("# e.g. final\nstatus: draft # string\n"));
    }

    #[test]
    fn must_fill_array_example_renders_as_block_sequence_with_context_quoting() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    recipient:
      type: array
      items: { type: string }
      example:
        - Mr. John Doe
        - 123 Main St
        - "Anytown, USA"
"#)
        .blueprint();
        assert!(t.contains(
            "recipient: !must_fill # array<string>\n  - Mr. John Doe\n  - 123 Main St\n  - Anytown, USA\n"
        ));
        assert!(!t.contains("# e.g."));
    }

    #[test]
    fn enum_endorsed_uses_enum_format_slot_and_no_eg() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    format: { type: enum, values: [standard, informal], default: standard }
"#)
        .blueprint();
        assert!(t.contains("format: standard # enum<standard | informal>\n"));
        assert!(!t.contains("e.g."));
    }

    #[test]
    fn enum_must_fill_renders_bare_marker() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    severity: { type: enum, values: [low, medium, high] }
"#)
        .blueprint();
        assert!(t.contains("severity: !must_fill # enum<low | medium | high>\n"));
    }

    #[test]
    fn every_field_carries_inline_type_and_cell_signal() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    title: { type: string }
    size: { type: number, default: 11 }
    flag: { type: boolean, default: false }
    issued: { type: date }
    published: { type: datetime }
    refs: { type: array, default: [], items: { type: string } }
"#)
        .blueprint();
        assert!(t.contains("title: !must_fill # string\n"));
        assert!(t.contains("size: 11 # number\n"));
        assert!(t.contains("flag: false # boolean\n"));
        assert!(t.contains("issued: !must_fill # date<YYYY-MM-DD>\n"));
        assert!(t.contains("published: !must_fill # datetime<YYYY-MM-DDThh:mm[:ss]>\n"));
        assert!(t.contains("refs: [] # array<string>\n"));
    }

    #[test]
    fn scalar_array_annotation_reflects_element_type() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    counts:   { type: array, items: { type: integer } }
    sections: { type: array, items: { type: richtext } }
    tags:     { type: array, items: { type: string } }
"#)
        .blueprint();
        assert!(t.contains("counts: !must_fill # array<integer>\n"), "{t}");
        assert!(
            t.contains("sections: !must_fill # array<richtext<markdown>>\n"),
            "{t}"
        );
        assert!(t.contains("tags: !must_fill # array<string>\n"), "{t}");
    }

    #[test]
    fn endorsed_empty_markdown_renders_empty_string() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    bio: { type: richtext, default: "" }
"#)
        .blueprint();
        assert!(t.contains("bio: \"\" # richtext<markdown>\n"));
        assert!(!t.contains("|-"));
        assert!(!t.contains("!must_fill"));
    }

    #[test]
    fn endorsed_markdown_default_inlines_quoted() {
        let t = cfg(r###"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    bio:
      type: richtext
      default: "## About me\n\nHello."
"###)
        .blueprint();
        assert!(t.contains("bio: \"## About me\\n\\nHello.\" # richtext<markdown>\n"));
        assert!(!t.contains("|-"));
    }

    #[test]
    fn root_header_carries_quill_reminder_and_no_role_comment() {
        let t = cfg(r#"
quill: { name: taro, version: 0.1.0, backend: typst, description: x }
main:
  fields:
    flavor: { type: string, default: taro }
"#)
        .blueprint();
        assert!(t.starts_with("~~~\n$quill: taro@0.1.0 # keep verbatim\n$kind: main\n# x\n"));
        assert!(t.contains("\nWrite main body here.\n"));
    }

    #[test]
    fn card_fence_carries_composable_annotation() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    title: { type: string }
card_kinds:
  note:
    description: A short note appended to the document.
    fields:
      author: { type: string }
"#)
        .blueprint();
        assert!(t.contains(
            "~~~\n$kind: note\n# composable (0..N)\n# A short note appended to the document.\n"
        ));
    }

    #[test]
    fn body_disabled_card_omits_body_placeholder() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    title: { type: string }
card_kinds:
  skills:
    body: { enabled: false }
    fields:
      items: { type: array, items: { type: string } }
"#)
        .blueprint();
        let after = &t[t.find("$kind: skills").unwrap()..];
        assert!(!after.contains("skills body"));
    }

    #[test]
    fn main_body_example_appears_verbatim() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  body:
    example: "Dear Sir or Madam,\n\nI am writing to..."
  fields:
    to: { type: string }
"#)
        .blueprint();
        assert!(t.contains("\nDear Sir or Madam,\n\nI am writing to...\n"));
        assert!(!t.contains("Write main body here."));
    }

    #[test]
    fn card_body_placeholder_uses_card_name() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    title: { type: string }
card_kinds:
  indorsement:
    fields:
      from: { type: string }
"#)
        .blueprint();
        assert!(t.contains("\nWrite indorsement body here.\n"));
    }

    #[test]
    fn ui_groups_cluster_fields_without_emitting_banner() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    memo_for: { type: array, items: { type: string }, ui: { group: Addressing } }
    subject: { type: string, ui: { group: Addressing } }
    letterhead_title: { type: string, default: HQ, ui: { group: Letterhead } }
    notes: { type: string }
"#)
        .blueprint();
        let after_quill = &t[t.find("$quill:").unwrap()..];
        assert!(!after_quill.contains("===="));
        let notes = after_quill.find("notes:").unwrap();
        let memo_for = after_quill.find("memo_for:").unwrap();
        let letterhead = after_quill.find("letterhead_title:").unwrap();
        assert!(notes < memo_for);
        assert!(memo_for < letterhead);
    }

    #[test]
    fn typed_table_must_fill_emits_synthetic_row_with_leaf_markers() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    references:
      type: array
      description: Cited works.
      items:
        type: object
        properties:
          org: { type: string, description: Citing organization. }
          year: { type: integer, default: 0, description: Publication year. }
"#)
        .blueprint();
        assert!(t.contains(
            "# Cited works.\nreferences: # array<object>\n  # Citing organization.\n  - org: !must_fill # string\n"
        ));
        assert!(t.contains("    # Publication year.\n    year: 0 # integer\n"));
    }

    #[test]
    fn typed_table_with_example_keeps_eg_line_and_synthetic_row() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    refs:
      type: array
      example:
        - { org: ACME, year: 2020 }
      items:
        type: object
        properties:
          org: { type: string }
          year: { type: integer, default: 0 }
"#)
        .blueprint();
        assert!(t.contains("# e.g. [{org: ACME, year: 2020}]\n"));
        assert!(t.contains("refs: # array<object>\n  - org: !must_fill # string\n"));
        assert!(t.contains("    year: 0 # integer\n"));
    }

    #[test]
    fn typed_table_endorsed_renders_default_rows() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    refs:
      type: array
      default:
        - { org: ACME }
      items:
        type: object
        properties:
          org: { type: string }
"#)
        .blueprint();
        assert!(t.contains("refs: # array<object>\n  - org: ACME\n"));
        assert!(!t.contains("refs: # array<object>\n  -\n"));
    }

    #[test]
    fn typed_table_with_empty_default_renders_inline() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    refs:
      type: array
      default: []
      items:
        type: object
        properties:
          org: { type: string }
"#)
        .blueprint();
        assert!(
            t.contains("refs: [] # array<object>\n"),
            "wrong rendering: {t}"
        );
        assert!(!t.contains("!must_fill"), "no markers expected: {t}");
    }

    #[test]
    fn typed_dict_with_empty_default_expands_to_zero_filled() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    address:
      type: object
      default: {}
      properties:
        street: { type: string }
        zip:    { type: integer }
"#)
        .blueprint();
        assert!(
            t.contains("address: # object\n  street: \"\"\n  zip: 0\n"),
            "wrong rendering: {t}"
        );
        assert!(!t.contains("{}"), "no bare empty object expected: {t}");
        assert!(!t.contains("!must_fill"), "no markers expected: {t}");
        assert!(!t.contains("# string"), "no leaf annotations expected: {t}");
    }

    #[test]
    fn typed_dict_must_fill_emits_per_property_annotations() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    address:
      type: object
      description: Mailing address.
      properties:
        street: { type: string, description: Street line. }
        city:   { type: string }
        zip:    { type: string, default: "" }
"#)
        .blueprint();
        assert!(t.contains("# Mailing address.\naddress: # object\n"));
        assert!(t.contains("  # Street line.\n  street: !must_fill # string\n"));
        assert!(t.contains("  city: !must_fill # string\n"));
        assert!(t.contains("  zip: \"\" # string\n"));
    }

    #[test]
    fn typed_dict_endorsed_renders_block_mapping() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    address:
      type: object
      default: { street: "5000 Forbes Ave", city: Pittsburgh }
      properties:
        street: { type: string }
        city:   { type: string }
"#)
        .blueprint();
        assert!(t.contains("address: # object\n"));
        assert!(
            t.contains("  street: 5000 Forbes Ave\n")
                || t.contains("  street: \"5000 Forbes Ave\"\n")
        );
        assert!(t.contains("  city: Pittsburgh\n"));
        assert!(!t.contains("# string"));
    }

    #[test]
    fn typed_dict_with_example_keeps_eg_line_and_per_property() {
        let t = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    address:
      type: object
      example: { street: "1 Infinite Loop", city: Cupertino }
      properties:
        street: { type: string }
        city:   { type: string, default: "" }
"#)
        .blueprint();
        assert!(t.contains("address: # object\n"));
        assert!(
            t.contains("# e.g. {street: 1 Infinite Loop, city: Cupertino}\n")
                || t.contains("# e.g. {city: Cupertino, street: 1 Infinite Loop}\n")
        );
        assert!(t.contains("  street: !must_fill # string\n"));
        assert!(t.contains("  city: \"\" # string\n"));
    }

    const LETTER_QUILL: &str = r#"
quill: { name: letter, version: 1.0.0, backend: typst, description: A formal letter. }
main:
  fields:
    to:
      type: string
      description: Recipient name.
    subject:
      type: string
    date:
      type: datetime
    priority:
      type: enum
      values: [normal, urgent]
      default: normal
    attachments:
      type: array
      items: { type: string }
      default: []
      example:
        - report.pdf
card_kinds:
  enclosure:
    description: An enclosure attached to the letter.
    fields:
      label: { type: string }
      pages: { type: integer, default: 1 }
"#;

    #[test]
    fn typed_table_synthetic_row_blueprint_round_trips() {
        let bp = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    refs:
      type: array
      items:
        type: object
        properties:
          org: { type: string, description: Citing organization. }
          year: { type: integer, default: 0, description: Publication year. }
"#)
        .blueprint();
        let doc1 = Document::parse(&bp).expect("blueprint must parse").document;
        let doc2 = Document::parse(&doc1.to_markdown())
            .expect("re-emit must parse")
            .document;
        assert_eq!(doc1, doc2, "typed-table blueprint must round-trip");
    }

    #[test]
    fn must_fill_markers_round_trip_and_survive_as_fill() {
        let bp = cfg(r#"
quill: { name: letter, version: 1.0.0, backend: typst, description: A letter. }
main:
  fields:
    recipient:
      type: array
      items: { type: string }
      example: [Mr. John Doe, "Anytown, USA"]
    subject: { type: string }
    date: { type: datetime }
"#)
        .blueprint();

        let doc1 = Document::parse(&bp).expect("blueprint must parse").document;
        let payload = doc1.main().payload();
        for key in ["recipient", "subject", "date"] {
            assert!(
                payload.is_fill(key),
                "`{key}` must carry the fill marker:\n{bp}"
            );
        }
        assert_eq!(
            payload
                .get("recipient")
                .and_then(|v| v.as_json().as_array().map(|a| a.len())),
            Some(2),
            "recipient suggested value should survive: {bp}"
        );

        let md2 = doc1.to_markdown();
        let doc2 = Document::parse(&md2).expect("re-emitted markdown must parse").document;
        assert_eq!(doc1, doc2, "blueprint must round-trip idempotently");
    }

    #[test]
    fn blueprint_round_trips_idempotently() {
        let bp = cfg(LETTER_QUILL).blueprint();
        let doc1 = Document::parse(&bp).expect("blueprint must parse").document;
        let md2 = doc1.to_markdown();
        let doc2 = Document::parse(&md2).expect("round-tripped markdown must parse").document;
        assert_eq!(
            doc1, doc2,
            "Document must be equal after blueprint → parse → emit → parse"
        );
    }

    #[test]
    fn type_ambiguous_string_defaults_round_trip_as_strings() {
        let bp = cfg(r#"
quill: { name: x, version: 1.0.0, backend: typst, description: x }
main:
  fields:
    version:     { type: string, default: "1.0" }
    activation:  { type: string, default: "on" }
    code:        { type: string, default: "01234" }
    placeholder: { type: string, default: "null" }
    yes_flag:    { type: string, default: "yes" }
"#)
        .blueprint();

        let doc = Document::parse(&bp).expect("blueprint must parse").document;
        let payload = doc.main().payload();
        for (key, expected) in [
            ("version", "1.0"),
            ("activation", "on"),
            ("code", "01234"),
            ("placeholder", "null"),
            ("yes_flag", "yes"),
        ] {
            let v = payload.get(key).unwrap_or_else(|| panic!("missing {key}"));
            assert!(
                v.as_str().is_some(),
                "field {key} must round-trip as a string, got {:?}\nBlueprint:\n{}",
                v,
                bp
            );
            assert_eq!(v.as_str().unwrap(), expected, "field {key}: value mismatch");
        }
    }
}
