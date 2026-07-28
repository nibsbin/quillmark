//! Canonical JSON serialization — the freeze.
//!
//! Byte-deterministic within this schema: equal [`Content`] values (by
//! `PartialEq` after [`Content::normalize`]) serialize to byte-equal JSON,
//! insensitive to the order marks/islands were discovered in. Three order
//! sources are closed here and in `normalize`: mark order (canonical sort),
//! island order (slot position), and object-key order inside island `props` /
//! unknown-mark `attrs` (recursively sorted). `deserialize ∘ serialize` is a
//! fixed point on canonical bytes.
//!
//! The seam encoding (Option A) and the storage encoding are the *same*
//! canonical form — one serializer, not two to keep aligned.

use crate::model::{
    sort_keys_owned, sorted_value, Container, Invariant, Island, Line, LineKind, Loss, Mark,
    MarkKind, Content, Usv,
};
use serde_json::{Map, Value};

/// Why canonical-JSON parsing failed. Structural only — a well-formed producer
/// (this crate's serializer, the seam, storage) never trips these.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// Top-level JSON was not an object, or a required key was missing/mistyped.
    Shape(&'static str),
    /// The JSON itself did not parse.
    Json(String),
    /// The value parsed but violates a content invariant.
    Invalid(crate::model::Invariant),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Shape(s) => write!(f, "content json shape: {s}"),
            ParseError::Json(s) => write!(f, "content json parse: {s}"),
            ParseError::Invalid(inv) => write!(f, "content invariant: {inv:?}"),
        }
    }
}
impl std::error::Error for ParseError {}

impl Content {
    /// Serialize to canonical JSON bytes. Normalizes a copy first, so the output
    /// is canonical regardless of the caller's mark/island order. Every object
    /// key is sorted recursively so the bytes do **not** depend on
    /// `serde_json`'s `preserve_order` feature being enabled in the consumer's
    /// crate graph — the canonical form is feature-independent.
    pub fn to_canonical_json(&self) -> String {
        to_canonical_value(self).to_string()
    }

    /// Parse canonical JSON, normalize (idempotent), and validate. Returns
    /// [`ParseError::Invalid`] for a content that violates its invariants, so
    /// storage cannot silently round-trip a malformed value.
    /// `from_canonical_json(to_canonical_json(x))` round-trips to a canonical
    /// value and re-serializes to identical bytes.
    pub fn from_canonical_json(s: &str) -> Result<Content, ParseError> {
        let v: Value = serde_json::from_str(s).map_err(|e| ParseError::Json(e.to_string()))?;
        from_canonical_value(&v)
    }

    fn to_value(&self) -> Value {
        let mut root = Map::new();
        root.insert("text".into(), Value::String(self.text.clone()));
        root.insert(
            "lines".into(),
            Value::Array(self.lines.iter().map(line_to_value).collect()),
        );
        root.insert(
            "marks".into(),
            Value::Array(self.marks.iter().map(mark_to_value).collect()),
        );
        root.insert(
            "islands".into(),
            Value::Array(self.islands.iter().map(island_to_value).collect()),
        );
        Value::Object(root)
    }

    fn from_value(v: &Value) -> Result<Content, ParseError> {
        let obj = v.as_object().ok_or(ParseError::Shape("root not object"))?;
        let text = obj
            .get("text")
            .and_then(Value::as_str)
            .ok_or(ParseError::Shape("text"))?
            .to_string();
        let lines = arr(obj, "lines")?
            .iter()
            .map(line_from_value)
            .collect::<Result<_, _>>()?;
        let marks = arr(obj, "marks")?
            .iter()
            .map(mark_from_value)
            .collect::<Result<_, _>>()?;
        let islands = arr(obj, "islands")?
            .iter()
            .map(island_from_value)
            .collect::<Result<_, _>>()?;
        Ok(Content {
            text,
            lines,
            marks,
            islands,
        })
    }
}

/// The canonical content form as a structural [`Value`] — the recursively
/// key-sorted tree [`Content::to_canonical_json`] renders to bytes. A storage
/// layer embeds this as a nested object (never an escaped string): serializing
/// the returned value with `serde_json` is byte-identical to that JSON
/// (`to_canonical_value(rt).to_string() == rt.to_canonical_json()`), independent
/// of the consumer's `preserve_order` feature. Normalizes a copy first, so the
/// value is canonical whatever the caller's mark/island order.
pub fn to_canonical_value(rt: &Content) -> Value {
    let mut rt = rt.clone();
    rt.normalize();
    sort_keys_owned(rt.to_value())
}

/// Parse the canonical content form from a structural [`Value`], normalize
/// (idempotent), and validate — the [`Value`]-input counterpart to
/// [`Content::from_canonical_json`]. Returns [`ParseError::Invalid`] for a
/// content that violates its invariants, so a storage layer parsing the embedded
/// object rejects a malformed value at load rather than round-tripping it.
pub fn from_canonical_value(v: &Value) -> Result<Content, ParseError> {
    let mut rt = Content::from_value(v)?;
    rt.normalize();
    rt.validate().map_err(ParseError::Invalid)?;
    Ok(rt)
}

/// Read a wire position as a [`Usv`] index. **Checked**, not `as usize`: the
/// deployment target is wasm32, where the truncating cast turns `2^32 + 5` into
/// an in-range `5` — a mark silently landing at the wrong position instead of a
/// rejected document. Every position the decoder reads goes through here.
pub(crate) fn usv_from(v: Option<&Value>, what: &'static str) -> Result<Usv, ParseError> {
    let n = v.and_then(Value::as_u64).ok_or(ParseError::Shape(what))?;
    Usv::try_from(n).map_err(|_| ParseError::Shape(what))
}

fn arr<'a>(obj: &'a Map<String, Value>, key: &'static str) -> Result<&'a Vec<Value>, ParseError> {
    obj.get(key)
        .and_then(Value::as_array)
        .ok_or(ParseError::Shape(key))
}

/// `v` as a slice, empty when it is not an array. The lenient counterpart to
/// [`arr`], for a reader that only inspects what is there — [`from_canonical_value`]
/// owns the shape errors.
fn as_slice(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or_default()
}

/// `v[key]` as a slice, empty when the key is absent or not an array.
fn arr_or_empty<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key).map(as_slice).unwrap_or_default()
}

// ---- Line ----

/// Encode a [`LineKind`] into its canonical `kind` fields (`"para"`,
/// `{"kind":"heading","level":n}`, …). Public so the mark/line **op** wire
/// ([`crate::ops`]) reuses the exact discriminant a `ContentLine` carries,
/// rather than forking the encoding.
pub fn line_kind_to_value(kind: &LineKind) -> Value {
    let mut m = Map::new();
    match kind {
        LineKind::Para => {
            m.insert("kind".into(), "para".into());
        }
        LineKind::Heading { level } => {
            m.insert("kind".into(), "heading".into());
            m.insert("level".into(), Value::from(*level));
        }
        LineKind::Code { lang } => {
            m.insert("kind".into(), "code".into());
            if let Some(l) = lang {
                m.insert("lang".into(), Value::String(l.clone()));
            }
        }
        LineKind::Island => {
            m.insert("kind".into(), "island".into());
        }
        LineKind::Rule => {
            m.insert("kind".into(), "rule".into());
        }
        // Open set, the mark encoding one axis over: the tag *is* the
        // discriminator and the payload rides one opaque `attrs` bag, so a
        // reader that lacks the role still carries it whole.
        LineKind::Unknown { tag, attrs } => {
            m.insert("kind".into(), Value::String(tag.clone()));
            m.insert("attrs".into(), sorted_value(attrs));
        }
    }
    Value::Object(m)
}

/// Decode a [`LineKind`] from an object carrying the canonical `kind` fields.
/// The inverse of [`line_kind_to_value`]; the shared line-kind reader for
/// [`line_from_value`] and the line-op wire.
pub fn line_kind_from_value(v: &Value) -> Result<LineKind, ParseError> {
    let o = v.as_object().ok_or(ParseError::Shape("line"))?;
    match o.get("kind").and_then(Value::as_str) {
        Some("para") => Ok(LineKind::Para),
        Some("heading") => {
            let level = o
                .get("level")
                .and_then(Value::as_u64)
                .ok_or(ParseError::Shape("heading level"))?;
            if !(1..=6).contains(&level) {
                return Err(ParseError::Shape("heading level"));
            }
            Ok(LineKind::Heading { level: level as u8 })
        }
        Some("code") => Ok(LineKind::Code {
            lang: o.get("lang").and_then(Value::as_str).map(str::to_string),
        }),
        Some("island") => Ok(LineKind::Island),
        Some("rule") => Ok(LineKind::Rule),
        // Open set: any other name is a block role this build lacks, kept opaque
        // and projected as `Para`. Only a missing/non-string `kind` is a shape
        // error — the *document* still opens when its vocabulary grows.
        Some(other) => Ok(LineKind::Unknown {
            tag: other.to_string(),
            attrs: o.get("attrs").cloned().unwrap_or(Value::Null),
        }),
        None => Err(ParseError::Shape("line kind")),
    }
}

fn line_to_value(line: &Line) -> Value {
    let Value::Object(mut m) = line_kind_to_value(&line.kind) else {
        unreachable!("line_kind_to_value always returns an object")
    };
    m.insert(
        "containers".into(),
        Value::Array(line.containers.iter().map(container_to_value).collect()),
    );
    // Omitted when false (the common case) — deterministic since presence is a
    // pure function of the value.
    if line.continues {
        m.insert("continues".into(), Value::Bool(true));
    }
    Value::Object(m)
}

fn line_from_value(v: &Value) -> Result<Line, ParseError> {
    let o = v.as_object().ok_or(ParseError::Shape("line"))?;
    let kind = line_kind_from_value(v)?;
    let containers = o
        .get("containers")
        .and_then(Value::as_array)
        .ok_or(ParseError::Shape("containers"))?
        .iter()
        .map(container_from_value)
        .collect::<Result<_, _>>()?;
    let continues = o.get("continues").and_then(Value::as_bool).unwrap_or(false);
    Ok(Line {
        kind,
        containers,
        continues,
    })
}

/// Encode a [`Container`] into its canonical wire object. Public so the line-op
/// wire ([`crate::ops`]) reuses the same container shape a `ContentLine`
/// carries.
pub fn container_to_value(c: &Container) -> Value {
    let mut m = Map::new();
    match c {
        Container::ListItem {
            ordered,
            start,
            ordinal,
        } => {
            m.insert("container".into(), "list_item".into());
            m.insert("ordered".into(), Value::Bool(*ordered));
            m.insert("start".into(), Value::from(*start));
            m.insert("ordinal".into(), Value::from(*ordinal));
        }
        Container::Quote => {
            m.insert("container".into(), "quote".into());
        }
        Container::Unknown { tag, attrs } => {
            m.insert("container".into(), Value::String(tag.clone()));
            m.insert("attrs".into(), sorted_value(attrs));
        }
    }
    Value::Object(m)
}

/// Decode a [`Container`] from its canonical wire object. The inverse of
/// [`container_to_value`].
pub fn container_from_value(v: &Value) -> Result<Container, ParseError> {
    let o = v.as_object().ok_or(ParseError::Shape("container"))?;
    match o.get("container").and_then(Value::as_str) {
        Some("list_item") => Ok(Container::ListItem {
            ordered: o.get("ordered").and_then(Value::as_bool).unwrap_or(false),
            start: o.get("start").and_then(Value::as_u64).unwrap_or(1),
            ordinal: o.get("ordinal").and_then(Value::as_u64).unwrap_or(0),
        }),
        Some("quote") => Ok(Container::Quote),
        // Open set, as for line kinds: an unrecognized container round-trips
        // opaque and projects transparently.
        Some(other) => Ok(Container::Unknown {
            tag: other.to_string(),
            attrs: o.get("attrs").cloned().unwrap_or(Value::Null),
        }),
        None => Err(ParseError::Shape("container kind")),
    }
}

// ---- Mark ----

/// Encode a [`Mark`] (`{start, end, type, …}`) into its canonical wire object.
/// Public so the mark-op wire ([`crate::ops`]) reuses the exact `type`
/// discriminant a `ContentMark` carries.
pub fn mark_to_value(mark: &Mark) -> Value {
    let mut m = Map::new();
    m.insert("start".into(), Value::from(mark.start));
    m.insert("end".into(), Value::from(mark.end));
    match &mark.kind {
        MarkKind::Strong => {
            m.insert("type".into(), "strong".into());
        }
        MarkKind::Emph => {
            m.insert("type".into(), "emph".into());
        }
        MarkKind::Underline => {
            m.insert("type".into(), "underline".into());
        }
        MarkKind::Strike => {
            m.insert("type".into(), "strike".into());
        }
        MarkKind::Code => {
            m.insert("type".into(), "code".into());
        }
        MarkKind::Link { url } => {
            m.insert("type".into(), "link".into());
            m.insert("url".into(), Value::String(url.clone()));
        }
        MarkKind::Anchor { id } => {
            m.insert("type".into(), "anchor".into());
            m.insert("id".into(), Value::String(id.clone()));
        }
        MarkKind::Unknown { tag, attrs } => {
            m.insert("type".into(), Value::String(tag.clone()));
            m.insert("attrs".into(), sorted_value(attrs));
        }
    }
    Value::Object(m)
}

/// Decode a [`Mark`] from its canonical wire object. The inverse of
/// [`mark_to_value`]; the shared mark reader for the content decoder and the
/// mark-op wire.
pub fn mark_from_value(v: &Value) -> Result<Mark, ParseError> {
    let o = v.as_object().ok_or(ParseError::Shape("mark"))?;
    let start = usv_from(o.get("start"), "mark start")?;
    let end = usv_from(o.get("end"), "mark end")?;
    let ty = o
        .get("type")
        .and_then(Value::as_str)
        .ok_or(ParseError::Shape("mark type"))?;
    let kind = match ty {
        "strong" => MarkKind::Strong,
        "emph" => MarkKind::Emph,
        "underline" => MarkKind::Underline,
        "strike" => MarkKind::Strike,
        "code" => MarkKind::Code,
        "link" => MarkKind::Link {
            url: o
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        "anchor" => MarkKind::Anchor {
            id: o
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        // Open set: any other type name is an unknown mark, round-tripped opaque
        // with whatever `attrs` it carried.
        other => MarkKind::Unknown {
            tag: other.to_string(),
            attrs: o.get("attrs").cloned().unwrap_or(Value::Null),
        },
    };
    Ok(Mark { start, end, kind })
}

// ---- Authored-lane readers (strict about reserved-name reuse) ----
//
// The readers above resolve a built-in discriminator before the `Unknown`
// fallthrough, so `{"kind": "para", "attrs": {…}}` decodes to `Para` and the
// `attrs` are dropped unread. `Content::validate`'s reserved-name rule
// (`Invariant::ReservedUnknownTag` and its two block-axis siblings) never sees
// such a value: it guards in-process Rust construction, not the wire.
//
// The two wire lanes want opposite answers to that drop, and the seam between
// them is not op-vs-content but authored-now vs read-back:
//
// - **Storage** (`Content::from_canonical_json`) stays lenient. A blob written
//   when `callout` was unknown carries `{"kind": "callout", "attrs": {…}}`, and
//   the release that makes `callout` a built-in must still open it. Rejecting
//   `attrs` beside a built-in here refuses documents at rest precisely when the
//   vocabulary grows — the failure the open set exists to prevent.
// - **Authored** (the `crate::ops` wire, and `install` through
//   [`from_authored_value`]) rejects it. The host is writing now, so the shape
//   means a stale copy of the built-in list, never a document from the past, and
//   the drop is silent corruption.
//
// The rule is narrow on purpose: `attrs` beside a *reserved* name, nothing else.
// A stray sibling key is evidence of nothing (a line object carries
// `containers`, an op carries `op`/`line`), and `attrs` is the unknown carrier's
// own spelling.

/// [`line_kind_from_value`] for the authored lane: `attrs` beside a built-in
/// `kind` is a shape error rather than a silent drop.
pub(crate) fn line_kind_from_authored_value(v: &Value) -> Result<LineKind, ParseError> {
    reject_line_kind_attrs(v)?;
    line_kind_from_value(v)
}

/// [`container_from_value`] for the authored lane. See
/// [`line_kind_from_authored_value`].
pub(crate) fn container_from_authored_value(v: &Value) -> Result<Container, ParseError> {
    reject_container_attrs(v)?;
    container_from_value(v)
}

/// [`mark_from_value`] for the authored lane. See [`line_kind_from_authored_value`].
pub(crate) fn mark_from_authored_value(v: &Value) -> Result<Mark, ParseError> {
    reject_mark_attrs(v)?;
    mark_from_value(v)
}

/// [`from_canonical_value`] for a content the **host authored just now** — the
/// `install` input, not a blob read back from storage. Same decode, plus the
/// reserved-name rule across the whole object, on every axis
/// [`Content::validate`] checks: line kinds, containers, prose marks, and
/// table-cell marks.
///
/// This is where the silent drop does its damage: an editor lowering a
/// whole-field diff writes through here on every keystroke.
pub fn from_authored_value(v: &Value) -> Result<Content, ParseError> {
    reject_reserved_attrs_deep(v)?;
    from_canonical_value(v)
}

/// The reserved-name scan [`from_authored_value`] runs, over the canonical
/// content shape. Structural on purpose rather than a blind recursive walk: an
/// unknown's `attrs` is opaque host payload that may legitimately contain an
/// object spelled `{"type": "link", "attrs": …}`, and rejecting that would make
/// the carrier unable to carry.
fn reject_reserved_attrs_deep(v: &Value) -> Result<(), ParseError> {
    for line in arr_or_empty(v, "lines") {
        reject_line_kind_attrs(line)?;
        for c in arr_or_empty(line, "containers") {
            reject_container_attrs(c)?;
        }
    }
    for m in arr_or_empty(v, "marks") {
        reject_mark_attrs(m)?;
    }
    // Cell marks ride the prose mark shape, so the rule follows them in. The
    // dispatch goes through `KnownIslandType` like every other one, so a new
    // mark-carrying type is a compile error here rather than a silent skip.
    for island in arr_or_empty(v, "islands") {
        let ty = island.get("type").and_then(Value::as_str).unwrap_or_default();
        match crate::island::KnownIslandType::parse(ty) {
            Some(crate::island::KnownIslandType::Table) => {
                let Some(props) = island.get("props") else {
                    continue;
                };
                for cell in table_cell_values(props) {
                    for m in arr_or_empty(cell, "marks") {
                        reject_mark_attrs(m)?;
                        // `parse_cell` drops a mark it cannot read, and
                        // `canon_cell` then writes back only what parsed, so the
                        // drop is permanent. Lenient is right for a blob at rest;
                        // on the authored lane it means the host's malformed mark
                        // disappears with no signal — the same reasoning that
                        // makes `attrs` beside a built-in an error here.
                        mark_from_value(m)?;
                    }
                }
            }
            // No cells: an image's props are flat, an unknown type's are opaque.
            Some(crate::island::KnownIslandType::Image) | None => {}
        }
    }
    Ok(())
}

fn reject_line_kind_attrs(v: &Value) -> Result<(), ParseError> {
    reject_reserved_attrs(
        v,
        "kind",
        &Content::RESERVED_LINE_KINDS,
        "attrs beside built-in kind",
    )
}

fn reject_container_attrs(v: &Value) -> Result<(), ParseError> {
    reject_reserved_attrs(
        v,
        "container",
        &Content::RESERVED_CONTAINERS,
        "attrs beside built-in container",
    )
}

fn reject_mark_attrs(v: &Value) -> Result<(), ParseError> {
    reject_reserved_attrs(
        v,
        "type",
        &Content::RESERVED_MARK_TYPES,
        "attrs beside built-in mark type",
    )
}

/// Error when `v` carries an `attrs` bag alongside a `discriminant` naming a
/// built-in — the producer meant an unknown and named a known. A non-object or a
/// missing/non-string discriminant is left to the reader that follows, which
/// reports the shape error in its own terms.
///
/// `attrs` is tested first: it is absent on all but the unknown arms, and
/// `serde_json` runs with `preserve_order`, so each key probe hashes.
fn reject_reserved_attrs(
    v: &Value,
    discriminant: &str,
    reserved: &[&str],
    err: &'static str,
) -> Result<(), ParseError> {
    let Some(o) = v.as_object() else {
        return Ok(());
    };
    if !o.contains_key("attrs") {
        return Ok(());
    }
    let Some(tag) = o.get(discriminant).and_then(Value::as_str) else {
        return Ok(());
    };
    if reserved.contains(&tag) {
        return Err(ParseError::Shape(err));
    }
    Ok(())
}

// ---- Table cell {text, marks} ----
//
// A pipe-table cell is inline-only: its own plain `text` plus `marks` whose
// ranges are USV offsets into that text (0..cell_len). The marks ride the SAME
// wire shape prose marks use (`mark_to_value`/`mark_from_value`), so nothing
// forks the encoding. Import builds cells, export/emit render them, and
// `Content::normalize`/`validate` canonicalize/check the marks — all through
// these helpers.

/// Parse a table-cell object `{text, marks}` leniently: its plain text plus the
/// marks over it. A malformed mark is skipped rather than failing — cells are
/// flat inline, so this never recurses. Public so the typst emitter renders a
/// cell through the same parse the codecs use.
pub fn parse_cell(v: &Value) -> (String, Vec<Mark>) {
    let text = v
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let marks = v
        .get("marks")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(|m| mark_from_value(m).ok()).collect())
        .unwrap_or_default();
    (text, marks)
}

/// Build a table-cell object `{text, marks}` — the inverse of [`parse_cell`],
/// reusing [`mark_to_value`]. Key order is fixed by the recursive
/// [`sorted_value`] pass in [`Content::normalize`], not here.
pub(crate) fn cell_to_value(text: &str, marks: &[Mark]) -> Value {
    let mut m = Map::new();
    m.insert("text".into(), Value::String(text.to_string()));
    m.insert(
        "marks".into(),
        Value::Array(marks.iter().map(mark_to_value).collect()),
    );
    Value::Object(m)
}

/// Every cell object in a table island's props — header then each body row, in
/// order. The undecoded half of [`table_cells`], so a reader that needs the raw
/// `Value` walks the same cells in the same order.
pub(crate) fn table_cell_values(props: &Value) -> impl Iterator<Item = &Value> {
    let header = arr_or_empty(props, "header").iter();
    let rows = arr_or_empty(props, "rows")
        .iter()
        .flat_map(|row| as_slice(row).iter());
    header.chain(rows)
}

/// Every cell's `(text, marks)` in a table island's props — header then each
/// body row, in order. For [`Content::validate`]'s cell-mark invariant checks.
pub(crate) fn table_cells(props: &Value) -> Vec<(String, Vec<Mark>)> {
    table_cell_values(props).map(parse_cell).collect()
}

// The `table` codec below (props normalize, shape-validate, cell extraction) is
// the primitive `crate::island` dispatches into for `KnownIslandType::Table`;
// island-type dispatch itself lives there, not here.

/// Repair a table island's props in place to the canonical shape:
///
/// - **One column count.** `cols` is the widest of the header, any body row, and
///   `aligns`; the header, each row, and `aligns` are padded up to it (padding
///   only grows — no cell is ever truncated). Materializing the count into the
///   header means the markdown projection (header-derived) and the Typst
///   projection (widest-row) agree on one number.
/// - **Single-line cells.** Any `\n`/`\r` in a cell's text becomes a space (the
///   same rule import applies to soft/hard breaks). A 1:1 replacement keeps char
///   offsets stable, so the cell's marks stay in range.
/// - **Canonical cell marks.** Each cell's marks are re-normalized (sort,
///   same-kind union, drop zero-width) so equal cells serialize to equal bytes.
pub(crate) fn normalize_table_props(props: &mut Value) {
    let cols = table_cols(props);
    let Some(obj) = props.as_object_mut() else {
        return;
    };
    let header = obj.entry("header").or_insert_with(|| Value::Array(vec![]));
    // A non-array header (a bare string, say) carries no cells; rewrite it to an
    // empty array so it canonicalizes to a zero-column, content-free table
    // rather than retaining opaque garbage that `validate` would then reject.
    if !header.is_array() {
        *header = Value::Array(vec![]);
    }
    pad_row(header, cols);
    if let Some(h) = header.as_array_mut() {
        h.iter_mut().for_each(canon_cell);
    }
    let aligns = obj.entry("aligns").or_insert_with(|| Value::Array(vec![]));
    if let Some(a) = aligns.as_array_mut() {
        while a.len() < cols {
            a.push(Value::String("none".into()));
        }
    }
    if let Some(rows) = obj.get_mut("rows").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            pad_row(row, cols);
            if let Some(r) = row.as_array_mut() {
                r.iter_mut().for_each(canon_cell);
            }
        }
    }
}

/// A table's canonical column count: the widest of its header, any body row, and
/// its `aligns` array. Padding (never truncation) brings every part up to it.
fn table_cols(props: &Value) -> usize {
    let arr_len = |k: &str| props.get(k).and_then(Value::as_array).map(|a| a.len());
    let header = arr_len("header").unwrap_or(0);
    let aligns = arr_len("aligns").unwrap_or(0);
    let widest_row = props
        .get("rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|r| r.as_array().map(|a| a.len()).unwrap_or(0))
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    header.max(aligns).max(widest_row)
}

/// Pad a cell array (header or body row) up to `cols` with empty cells. Never
/// shrinks — `cols` is the widest, so a shorter array only grows.
fn pad_row(v: &mut Value, cols: usize) {
    if let Some(arr) = v.as_array_mut() {
        while arr.len() < cols {
            arr.push(cell_to_value("", &[]));
        }
    }
}

/// De-newline a cell's text (each `\n`/`\r` → a space, 1:1 so mark offsets hold)
/// and re-normalize its marks. Reached per-cell from [`normalize_table_props`].
///
/// Writes `text` and `marks` back into the cell's **own** object rather than
/// minting a fresh one, so a key this build does not recognize survives. A cell
/// is the sub-structure the `table` type is likeliest to grow (`colspan`,
/// `rowspan`, a per-cell alignment or style handle), and every other opaque
/// payload in the model — unknown `attrs` on all three block axes, island
/// `props`, a table's own top-level props — already round-trips untouched. A
/// cell minted whole was the one exception, which made the likeliest extension
/// the one that could not be added without a schema-version event.
fn canon_cell(cell: &mut Value) {
    let (text, marks) = parse_cell(cell);
    let text = if text.contains(['\n', '\r']) {
        text.replace(['\n', '\r'], " ")
    } else {
        text
    };
    let marks = crate::model::normalize_marks(marks);
    match cell.as_object_mut() {
        Some(o) => {
            o.insert("text".into(), Value::String(text));
            o.insert(
                "marks".into(),
                Value::Array(marks.iter().map(mark_to_value).collect()),
            );
        }
        // A non-object cell (a bare string, a null) holds no keys to preserve;
        // rewriting it whole is what gives it the canonical shape at all. Same
        // for the empty cells `pad_row` mints — synthesized, nothing to carry.
        None => *cell = cell_to_value(&text, &marks),
    }
    // Key order is restored by the recursive `canonicalize_keys` pass in
    // `Content::normalize`, which runs over the whole props tree after this.
}

/// A table island's shape violation, if any — the widths the header, `aligns`,
/// and each body row must share (the header width), plus the `\n`-free-cell rule.
/// The validate-side twin of [`normalize_table_props`].
pub(crate) fn table_shape_error(props: &Value) -> Option<Invariant> {
    // A present-but-non-array header can't carry column cells — `normalize`
    // rewrites it to an empty array, so an un-normalized one is a hand-built
    // degenerate island. (An absent header is a zero-column table, which is
    // well-formed: `empty_table_is_valid`.)
    if props.get("header").is_some_and(|h| !h.is_array()) {
        return Some(Invariant::TableHeaderNotArray);
    }
    let cols = props
        .get("header")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    let aligns = props
        .get("aligns")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    if aligns != cols {
        return Some(Invariant::TableAlignsMismatch { aligns, cols });
    }
    if let Some(rows) = props.get("rows").and_then(Value::as_array) {
        for (i, row) in rows.iter().enumerate() {
            let width = row.as_array().map(|a| a.len()).unwrap_or(0);
            if width != cols {
                return Some(Invariant::TableRaggedRow {
                    row: i,
                    width,
                    cols,
                });
            }
        }
    }
    for (i, (text, _)) in table_cells(props).iter().enumerate() {
        if text.contains('\n') || text.contains('\r') {
            return Some(Invariant::TableCellNewline { cell: i });
        }
    }
    None
}

// ---- Island ----

fn island_to_value(island: &Island) -> Value {
    let mut m = Map::new();
    m.insert("id".into(), Value::String(island.id.clone()));
    m.insert("type".into(), Value::String(island.island_type.clone()));
    m.insert("props".into(), sorted_value(&island.props));
    m.insert("loss".into(), loss_to_str(&island.loss).into());
    Value::Object(m)
}

fn island_from_value(v: &Value) -> Result<Island, ParseError> {
    let o = v.as_object().ok_or(ParseError::Shape("island"))?;
    Ok(Island {
        id: o
            .get("id")
            .and_then(Value::as_str)
            .ok_or(ParseError::Shape("island id"))?
            .to_string(),
        island_type: o
            .get("type")
            .and_then(Value::as_str)
            .ok_or(ParseError::Shape("island type"))?
            .to_string(),
        props: o.get("props").cloned().unwrap_or(Value::Null),
        loss: loss_from_str(o.get("loss").and_then(Value::as_str).unwrap_or("lossless")),
    })
}

fn loss_to_str(loss: &Loss) -> &str {
    match loss {
        Loss::Lossless => "lossless",
        Loss::Degraded => "degraded",
        Loss::Unrepresentable => "unrepresentable",
        // Verbatim, so a class this build lacks survives a reader that merely
        // opened the document.
        Loss::Unknown(raw) => raw,
    }
}

fn loss_from_str(s: &str) -> Loss {
    match s {
        "lossless" => Loss::Lossless,
        "degraded" => Loss::Degraded,
        "unrepresentable" => Loss::Unrepresentable,
        // Unknown/future loss class is carried, and reads through
        // `Loss::fidelity` at the *safe* end: never claim a value the reader
        // can't interpret "carries faithfully".
        other => Loss::Unknown(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Line, LineKind};

    fn sample() -> Content {
        Content {
            text: "hello world".into(),
            lines: vec![Line {
                kind: LineKind::Para,
                containers: vec![],
                continues: false,
            }],
            marks: vec![
                Mark {
                    start: 6,
                    end: 11,
                    kind: MarkKind::Strong,
                },
                Mark {
                    start: 0,
                    end: 5,
                    kind: MarkKind::Emph,
                },
            ],
            islands: vec![],
        }
    }

    /// Issue #1051: the decoder is the entry point for stored and
    /// caller-supplied content, and export recurses one frame per container. A
    /// 20 000-deep path used to decode clean and abort the process on
    /// `to_markdown`; the shared `validate` cap rejects it at the door.
    #[test]
    fn deep_container_nesting_is_rejected_at_decode() {
        let containers = vec![r#"{"container":"quote"}"#; 20_000].join(",");
        let json = format!(
            r#"{{"text":"hi","lines":[{{"kind":"para","containers":[{containers}]}}],"marks":[],"islands":[]}}"#
        );
        assert!(matches!(
            Content::from_canonical_json(&json),
            Err(ParseError::Invalid(Invariant::NestingTooDeep { .. }))
        ));
    }

    /// Issue #1051: a wire position past `usize` is refused, not truncated. On
    /// wasm32 — the deployment target — `as usize` turned `2^32 + 5` into an
    /// in-range `5`, landing a mark at the wrong position in a document that
    /// then validated clean. Rejected on every target, by the checked read here
    /// on 32-bit and by the range invariant on 64-bit.
    #[test]
    fn out_of_range_wire_position_is_refused() {
        let json = r#"{"text":"hello","lines":[{"kind":"para","containers":[]}],"marks":[{"start":4294967301,"end":4294967302,"type":"strong"}],"islands":[]}"#;
        assert!(Content::from_canonical_json(json).is_err());
        assert!(usv_from(Some(&Value::from(u64::MAX)), "x").is_ok() || usize::BITS < 64);
        assert!(usv_from(Some(&Value::from(-1i64)), "x").is_err());
    }

    #[test]
    fn island_props_key_order_does_not_leak() {
        let mut one = Content::empty();
        one.text = "\u{FFFC}".into();
        one.lines = vec![Line {
            kind: LineKind::Island,
            containers: vec![],
            continues: false,
        }];
        one.islands = vec![Island {
            id: "i1".into(),
            island_type: "table".into(),
            props: serde_json::json!({"b": 1, "a": 2}),
            loss: Loss::Lossless,
        }];
        let mut two = one.clone();
        two.islands[0].props = serde_json::json!({"a": 2, "b": 1}); // keys reversed
        assert_eq!(one.to_canonical_json(), two.to_canonical_json());
    }

    #[test]
    fn golden_bytes_are_feature_independent() {
        // Pins the exact canonical form. Every object key is sorted, so the
        // bytes do not depend on serde_json's preserve_order feature. If this
        // string changes, the freeze changed — bump the schema version.
        let rt = sample();
        assert_eq!(
            rt.to_canonical_json(),
            r#"{"islands":[],"lines":[{"containers":[],"kind":"para"}],"marks":[{"end":5,"start":0,"type":"emph"},{"end":11,"start":6,"type":"strong"}],"text":"hello world"}"#
        );
    }

    #[test]
    fn from_canonical_json_rejects_invalid() {
        // lines.len() != segment count — must not silently round-trip.
        let bad =
            r#"{"text":"a\nb","lines":[{"kind":"para","containers":[]}],"marks":[],"islands":[]}"#;
        assert!(matches!(
            Content::from_canonical_json(bad),
            Err(ParseError::Invalid(_))
        ));
    }

    #[test]
    fn reserved_unknown_tag_rejected() {
        // An Unknown mark may not reuse a built-in type name (would parse back
        // as the built-in, dropping attrs — non-injective).
        let mut rt = Content::empty();
        rt.text = "abcd".into();
        rt.marks = vec![Mark {
            start: 0,
            end: 4,
            kind: MarkKind::Unknown {
                tag: "strong".into(),
                attrs: serde_json::json!({}),
            },
        }];
        assert!(matches!(
            rt.validate(),
            Err(crate::model::Invariant::ReservedUnknownTag(_))
        ));
    }

    /// Issue #1091: `loss` is the fifth open vocabulary, on the same terms as the
    /// four below. A class this build lacks is **carried**, not rewritten, so a
    /// reader that merely opened the document neither destroys the class nor
    /// moves the content hash; reading it degrades to the safe end.
    #[test]
    fn unknown_loss_class_round_trips_and_reads_unrepresentable() {
        let json = concat!(
            r#"{"islands":[{"id":"i1","loss":"partial","props":{},"type":"widget"}],"#,
            r#""lines":[{"containers":[],"kind":"island"}],"marks":[],"text":"￼"}"#
        );
        let rt = Content::from_canonical_json(json).unwrap();
        assert_eq!(rt.islands[0].loss, Loss::Unknown("partial".into()));
        assert_eq!(rt.islands[0].loss.fidelity(), Loss::Unrepresentable);
        assert_eq!(rt.to_canonical_json(), json);
    }

    /// Issue #1054: the block vocabulary is open on the mark axis' terms. A
    /// `kind`/`container` this build lacks decodes to `Unknown` — the document
    /// **opens** — and re-encodes byte-identically, so a construct a future
    /// reader understands survives the trip through this one.
    #[test]
    fn unknown_line_kind_and_container_round_trip_opaque() {
        let json = concat!(
            r#"{"islands":[],"lines":[{"attrs":{"variant":"warn"},"containers":"#,
            r#"[{"attrs":{"depth":2},"container":"indent"}],"kind":"callout"}],"#,
            r#""marks":[],"text":"heads up"}"#
        );
        let rt = Content::from_canonical_json(json).unwrap();
        assert_eq!(
            rt.lines[0].kind,
            LineKind::Unknown {
                tag: "callout".into(),
                attrs: serde_json::json!({"variant": "warn"}),
            }
        );
        assert_eq!(
            rt.lines[0].containers,
            vec![Container::Unknown {
                tag: "indent".into(),
                attrs: serde_json::json!({"depth": 2}),
            }]
        );
        assert_eq!(rt.to_canonical_json(), json);
        // An attrs-free unknown decodes too (`attrs` is null, not a shape error).
        let bare = r#"{"islands":[],"lines":[{"containers":[],"kind":"footnote"}],"marks":[],"text":"x"}"#;
        let rt = Content::from_canonical_json(bare).unwrap();
        assert_eq!(
            rt.lines[0].kind,
            LineKind::Unknown {
                tag: "footnote".into(),
                attrs: Value::Null,
            }
        );
        // A missing/non-string discriminator is still a shape error — the open
        // set absorbs unknown *names*, not malformed objects.
        for bad in [
            r#"{"islands":[],"lines":[{"containers":[]}],"marks":[],"text":"x"}"#,
            r#"{"islands":[],"lines":[{"containers":[{"container":7}],"kind":"para"}],"marks":[],"text":"x"}"#,
        ] {
            assert!(matches!(
                Content::from_canonical_json(bad),
                Err(ParseError::Shape(_))
            ));
        }
    }

    /// Issue #1054: an unknown line kind / container may not reuse a built-in
    /// name — it would serialize as the built-in and parse back as one, dropping
    /// its attrs (the `ReservedUnknownTag` rule, one axis over).
    #[test]
    fn reserved_block_vocabulary_names_rejected() {
        let mut rt = Content::empty();
        rt.text = "abcd".into();
        rt.lines[0].kind = LineKind::Unknown {
            tag: "heading".into(),
            attrs: serde_json::json!({}),
        };
        assert_eq!(
            rt.validate(),
            Err(Invariant::ReservedUnknownLineKind("heading".into()))
        );
        rt.lines[0].kind = LineKind::Para;
        rt.lines[0].containers = vec![Container::Unknown {
            tag: "quote".into(),
            attrs: serde_json::json!({}),
        }];
        assert_eq!(
            rt.validate(),
            Err(Invariant::ReservedUnknownContainer("quote".into()))
        );
    }

    /// Issue #1084: the authored lane (`install`) applies the reserved-name rule
    /// the decoders cannot — by the time a lenient reader has resolved `"para"`
    /// to `Para`, the `attrs` are gone and `validate` has nothing to object to.
    /// Every axis `validate` checks, including cell marks.
    #[test]
    fn authored_lane_rejects_attrs_beside_a_built_in_name() {
        let bad = [
            // line kind
            r#"{"islands":[],"lines":[{"attrs":{"tone":"warn"},"containers":[],"kind":"para"}],"marks":[],"text":"x"}"#,
            // container
            r#"{"islands":[],"lines":[{"containers":[{"attrs":{},"container":"quote"}],"kind":"para"}],"marks":[],"text":"x"}"#,
            // prose mark
            r#"{"islands":[],"lines":[{"containers":[],"kind":"para"}],"marks":[{"attrs":{},"end":1,"start":0,"type":"strong"}],"text":"x"}"#,
            // table cell mark
            concat!(
                r#"{"islands":[{"id":"i1","loss":"lossless","props":{"aligns":["none"],"#,
                r#""header":[{"marks":[{"attrs":{},"end":1,"start":0,"type":"emph"}],"text":"h"}],"#,
                r#""rows":[[{"marks":[],"text":"r"}]]},"type":"table"}],"#,
                r#""lines":[{"containers":[],"kind":"island"}],"marks":[],"text":"￼"}"#
            ),
        ];
        for json in bad {
            let v: Value = serde_json::from_str(json).unwrap();
            assert!(
                matches!(from_authored_value(&v), Err(ParseError::Shape(_))),
                "accepted: {json}"
            );
            // The storage lane opens all four — a document written before the
            // name was built in must keep loading.
            assert!(
                Content::from_canonical_json(json).is_ok(),
                "storage lane rejected: {json}"
            );
        }
    }

    /// Issue #1092: a cell mark the reader cannot parse is dropped by
    /// `parse_cell` and the drop made permanent by `canon_cell`, so the authored
    /// lane refuses it on the same reasoning as the reserved-name rule — the
    /// host's mark would otherwise vanish with no signal. Storage stays lenient:
    /// a stored blob's unreadable mark must not make the document unopenable.
    #[test]
    fn authored_lane_rejects_an_unparseable_cell_mark() {
        let json = concat!(
            r#"{"islands":[{"id":"i1","loss":"lossless","props":{"aligns":["none"],"#,
            r#""header":[{"marks":[{"end":1,"start":0}],"text":"h"}],"#,
            r#""rows":[[{"marks":[],"text":"r"}]]},"type":"table"}],"#,
            r#""lines":[{"containers":[],"kind":"island"}],"marks":[],"text":"￼"}"#
        );
        let v: Value = serde_json::from_str(json).unwrap();
        assert!(matches!(
            from_authored_value(&v),
            Err(ParseError::Shape(_))
        ));
        let rt = Content::from_canonical_json(json).unwrap();
        assert!(rt.islands[0].props["header"][0]["marks"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    /// Issue #1084: the authored lane's scan is structural, not a blind walk. An
    /// unknown's `attrs` is opaque host payload and may contain an object spelled
    /// like a reserved mark; rejecting that would make the carrier unable to
    /// carry the thing it exists to carry.
    #[test]
    fn authored_lane_leaves_opaque_attrs_payload_alone() {
        let json = concat!(
            r#"{"islands":[],"lines":[{"attrs":{"nested":{"attrs":{},"type":"link"}},"#,
            r#""containers":[],"kind":"callout"}],"marks":[],"text":"x"}"#
        );
        let v: Value = serde_json::from_str(json).unwrap();
        let rt = from_authored_value(&v).unwrap();
        assert_eq!(rt.to_canonical_json(), json);
    }

    /// Issue #1054: opaque block attrs are hash input, so their key order must
    /// not leak into the canonical bytes — the unknown-mark rule, one axis over.
    #[test]
    fn unknown_block_attrs_key_order_does_not_leak() {
        let mut one = Content::empty();
        one.text = "hi".into();
        one.lines[0].kind = LineKind::Unknown {
            tag: "callout".into(),
            attrs: serde_json::json!({"b": 1, "a": 2}),
        };
        one.lines[0].containers = vec![Container::Unknown {
            tag: "indent".into(),
            attrs: serde_json::json!({"y": 1, "x": 2}),
        }];
        let mut two = one.clone();
        two.lines[0].kind = LineKind::Unknown {
            tag: "callout".into(),
            attrs: serde_json::json!({"a": 2, "b": 1}),
        };
        two.lines[0].containers = vec![Container::Unknown {
            tag: "indent".into(),
            attrs: serde_json::json!({"x": 2, "y": 1}),
        }];
        assert_eq!(one.to_canonical_json(), two.to_canonical_json());
        one.normalize();
        two.normalize();
        assert_eq!(one, two, "normalize canonicalizes the live model too");
    }

    #[test]
    fn unknown_mark_round_trips_opaque() {
        let mut rt = Content::empty();
        rt.text = "abcd".into();
        rt.marks = vec![Mark {
            start: 0,
            end: 4,
            kind: MarkKind::Unknown {
                tag: "highlight".into(),
                attrs: serde_json::json!({"color": "yellow"}),
            },
        }];
        let json = rt.to_canonical_json();
        let back = Content::from_canonical_json(&json).unwrap();
        assert_eq!(back.marks[0].kind, rt.marks[0].kind);
    }
}
