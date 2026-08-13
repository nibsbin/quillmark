//! Canonical JSON serialization: the freeze.
//!
//! Byte-deterministic within this schema: equal [`Content`] values (by
//! `PartialEq` after [`Content::normalize`]) serialize to byte-equal JSON,
//! insensitive to the order marks/islands were discovered in. Three order
//! sources are closed here and in `normalize`: mark order (canonical sort),
//! island order (slot position), and object-key order inside island `props` /
//! unknown-mark `attrs` (recursively sorted).
//!
//! Two fixed points, and they are not the same promise. **Bytes**:
//! `to_canonical_json(from_canonical_json(b)) == b` for canonical `b`, what a
//! consumer hashing stored documents spends. **Values**:
//! `from_canonical_json(to_canonical_json(rt)) == rt` for a normalized `rt`,
//! which holds only while every discriminator's encoding is injective. An axis
//! can keep the first and lose the second: a value that encodes to some *other*
//! value's bytes moves nothing on disk and still fails its own round trip.
//!
//! The seam encoding and the storage encoding are the *same* canonical form.

use crate::model::{
    sort_keys_owned, Container, Invariant, Island, Line, LineKind, Loss, Mark,
    MarkKind, Content, Usv,
};
use serde_json::{Map, Value};
use std::borrow::Cow;

/// Why canonical-JSON parsing failed. Structural only: a well-formed producer
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
    /// is canonical regardless of the caller's mark/island order, and sorts every
    /// object key recursively, so the bytes do **not** depend on `serde_json`'s
    /// `preserve_order` feature in the consumer's crate graph.
    pub fn to_canonical_json(&self) -> String {
        to_canonical_value(self).to_string()
    }

    /// Parse canonical JSON, normalize, and validate. Returns
    /// [`ParseError::Invalid`] for a content that violates its invariants, so
    /// storage cannot silently round-trip a malformed value.
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

/// The canonical content form as a structural [`Value`]: the recursively
/// key-sorted tree [`Content::to_canonical_json`] renders to bytes. A storage
/// layer embeds this as a nested object rather than an escaped string;
/// serializing it with `serde_json` is byte-identical to that JSON, independent
/// of the consumer's `preserve_order` feature.
pub fn to_canonical_value(rt: &Content) -> Value {
    let mut rt = rt.clone();
    rt.normalize();
    sort_keys_owned(rt.to_value())
}

/// Parse the canonical content form from a structural [`Value`], normalize, and
/// validate: the [`Value`]-input counterpart to
/// [`Content::from_canonical_json`].
pub fn from_canonical_value(v: &Value) -> Result<Content, ParseError> {
    let mut rt = Content::from_value(v)?;
    rt.normalize();
    rt.validate().map_err(ParseError::Invalid)?;
    Ok(rt)
}

/// Read an opaque payload bag (`attrs`, `props`) off the wire, absent → `Null`.
///
/// **Depth-checked before the clone.** `Value::clone` spends a frame per level,
/// so an over-deep bag has to be refused while it is still borrowed from the
/// caller's `Value`: once owned the frames are already spent, and dropping it
/// spends them again.
fn bag_from_wire(
    o: &Map<String, Value>,
    key: &'static str,
    what: &'static str,
) -> Result<Value, ParseError> {
    let Some(v) = o.get(key) else {
        return Ok(Value::Null);
    };
    crate::model::check_json_depth(v, what).map_err(ParseError::Invalid)?;
    Ok(v.clone())
}

/// Read a wire position as a [`Usv`] index. **Checked**, not `as usize`: the
/// deployment target is wasm32, where the truncating cast turns `2^32 + 5` into
/// an in-range `5`, landing a mark at the wrong position instead of rejecting
/// the document.
pub(crate) fn usv_from(v: Option<&Value>, what: &'static str) -> Result<Usv, ParseError> {
    let n = v.and_then(Value::as_u64).ok_or(ParseError::Shape(what))?;
    Usv::try_from(n).map_err(|_| ParseError::Shape(what))
}

fn arr<'a>(obj: &'a Map<String, Value>, key: &'static str) -> Result<&'a Vec<Value>, ParseError> {
    obj.get(key)
        .and_then(Value::as_array)
        .ok_or(ParseError::Shape(key))
}

/// `v` as a slice, empty when it is not an array: the lenient counterpart to
/// [`arr`], since [`from_canonical_value`] owns the shape errors.
fn as_slice(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or_default()
}

fn arr_or_empty<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key).map(as_slice).unwrap_or_default()
}

/// Fold a legacy `attrs` bag into the object when `tag` names a **built-in**:
/// the storage lane's promotion path. A blob written while `callout` was outside
/// this build's vocabulary carries `{"kind":"callout","attrs":{…}}`, and the
/// release that promotes `callout` reads named siblings that blob never had,
/// dropping its payload unread. Folding the bag in before the built-in arms run
/// makes each promotion carry its own legacy form structurally.
///
/// Three bounds. A named sibling wins over an `attrs` entry. Only a reserved
/// name folds, so an unknown's bag stays its opaque payload. The discriminator
/// is read from the *original* object, so a bag holding a
/// `kind`/`type`/`container` key cannot re-target the match.
///
/// The authored lane rejects `attrs` beside a built-in up front, so it never
/// arrives here with this shape.
///
/// Depth-checked like [`bag_from_wire`]: the fold deep-clones the object it
/// folds into.
fn fold_legacy_attrs<'a>(
    o: &'a Map<String, Value>,
    tag: &str,
    reserved: &[&str],
    what: &'static str,
) -> Result<Cow<'a, Map<String, Value>>, ParseError> {
    let Some(bag @ Value::Object(attrs)) = o.get("attrs") else {
        return Ok(Cow::Borrowed(o));
    };
    if attrs.is_empty() || !reserved.contains(&tag) {
        return Ok(Cow::Borrowed(o));
    }
    crate::model::check_json_depth(bag, what).map_err(ParseError::Invalid)?;
    let mut folded = o.clone();
    for (k, v) in attrs {
        folded.entry(k.clone()).or_insert_with(|| v.clone());
    }
    Ok(Cow::Owned(folded))
}

/// Encode a [`LineKind`] into its canonical `kind` fields (`"para"`,
/// `{"kind":"heading","level":n}`, …). Public so the op wire ([`crate::ops`])
/// reuses the exact discriminant a `ContentLine` carries.
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
        // The tag *is* the discriminator and the payload rides one opaque
        // `attrs` bag, so a reader lacking the role still carries it whole.
        LineKind::Unknown { tag, attrs } => {
            m.insert("kind".into(), Value::String(tag.clone()));
            m.insert("attrs".into(), attrs.clone());
        }
    }
    Value::Object(m)
}

/// Decode a [`LineKind`] from an object carrying the canonical `kind` fields.
pub fn line_kind_from_value(v: &Value) -> Result<LineKind, ParseError> {
    let o = v.as_object().ok_or(ParseError::Shape("line"))?;
    // A missing/non-string `kind` is the one shape error here: the open set
    // absorbs unknown *names*, not malformed objects.
    let tag = o
        .get("kind")
        .and_then(Value::as_str)
        .ok_or(ParseError::Shape("line kind"))?;
    let o = fold_legacy_attrs(o, tag, Content::RESERVED_LINE_KINDS, "line attrs")?;
    match tag {
        "para" => Ok(LineKind::Para),
        "heading" => {
            let level = o
                .get("level")
                .and_then(Value::as_u64)
                .ok_or(ParseError::Shape("heading level"))?;
            if !(1..=6).contains(&level) {
                return Err(ParseError::Shape("heading level"));
            }
            Ok(LineKind::Heading { level: level as u8 })
        }
        "code" => Ok(LineKind::Code {
            lang: o.get("lang").and_then(Value::as_str).map(str::to_string),
        }),
        "island" => Ok(LineKind::Island),
        "rule" => Ok(LineKind::Rule),
        // Any other name is a block role this build lacks, kept opaque and
        // projected as `Para`, so the document still opens.
        other => Ok(LineKind::Unknown {
            tag: other.to_string(),
            attrs: bag_from_wire(&o, "attrs", "line attrs")?,
        }),
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
    // Omitted when false: presence is a pure function of the value, so the
    // encoding stays deterministic.
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

/// Encode a [`Container`] into its canonical wire object. Public so the op wire
/// ([`crate::ops`]) reuses the same container shape a `ContentLine` carries.
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
            m.insert("attrs".into(), attrs.clone());
        }
    }
    Value::Object(m)
}

/// Decode a [`Container`] from its canonical wire object.
pub fn container_from_value(v: &Value) -> Result<Container, ParseError> {
    let o = v.as_object().ok_or(ParseError::Shape("container"))?;
    let tag = o
        .get("container")
        .and_then(Value::as_str)
        .ok_or(ParseError::Shape("container kind"))?;
    let o = fold_legacy_attrs(o, tag, Content::RESERVED_CONTAINERS, "container attrs")?;
    match tag {
        "list_item" => Ok(Container::ListItem {
            ordered: o.get("ordered").and_then(Value::as_bool).unwrap_or(false),
            start: o.get("start").and_then(Value::as_u64).unwrap_or(1),
            ordinal: o.get("ordinal").and_then(Value::as_u64).unwrap_or(0),
        }),
        "quote" => Ok(Container::Quote),
        // An unrecognized container round-trips opaque and projects
        // transparently.
        other => Ok(Container::Unknown {
            tag: other.to_string(),
            attrs: bag_from_wire(&o, "attrs", "container attrs")?,
        }),
    }
}

/// Encode a [`Mark`] (`{start, end, type, …}`) into its canonical wire object.
/// Public so the op wire ([`crate::ops`]) reuses the exact `type` discriminant a
/// `ContentMark` carries.
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
            m.insert("attrs".into(), attrs.clone());
        }
    }
    Value::Object(m)
}

/// What every mark carries whatever its type.
struct MarkShape<'a> {
    fields: &'a Map<String, Value>,
    start: Usv,
    end: Usv,
    ty: &'a str,
}

/// A mark's fallible half: the prologue of [`mark_from_value`], and the whole of
/// what a caller wanting only the verdict needs. Building the [`MarkKind`]
/// cannot fail, and for an unknown tag it deep-clones the opaque `attrs` bag,
/// which a validity check has no reason to pay for.
fn mark_shape(v: &Value) -> Result<MarkShape<'_>, ParseError> {
    let fields = v.as_object().ok_or(ParseError::Shape("mark"))?;
    let start = usv_from(fields.get("start"), "mark start")?;
    let end = usv_from(fields.get("end"), "mark end")?;
    let ty = fields
        .get("type")
        .and_then(Value::as_str)
        .ok_or(ParseError::Shape("mark type"))?;
    Ok(MarkShape {
        fields,
        start,
        end,
        ty,
    })
}

/// Decode a [`Mark`] from its canonical wire object.
pub fn mark_from_value(v: &Value) -> Result<Mark, ParseError> {
    let MarkShape {
        fields: o,
        start,
        end,
        ty,
    } = mark_shape(v)?;
    // After the shape read, not inside it: the fold's clone is exactly the cost
    // `mark_shape` exists to let a verdict-only caller skip.
    let o = fold_legacy_attrs(o, ty, Content::RESERVED_MARK_TYPES, "mark attrs")?;
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
        // Any other type name is an unknown mark, round-tripped opaque with
        // whatever `attrs` it carried.
        other => MarkKind::Unknown {
            tag: other.to_string(),
            attrs: bag_from_wire(&o, "attrs", "mark attrs")?,
        },
    };
    Ok(Mark { start, end, kind })
}

// Authored-lane readers, strict about reserved-name reuse.
//
// The readers above resolve a built-in discriminator before the `Unknown`
// fallthrough, so `{"kind": "para", "attrs": {…}}` decodes to `Para` and the
// `attrs` are dropped unread. The two wire lanes want opposite answers to that
// drop, and the seam is authored-now vs read-back:
//
// - **Storage** (`Content::from_canonical_json`) stays lenient. A blob written
//   when `callout` was unknown carries `{"kind": "callout", "attrs": {…}}`, and
//   the release that makes `callout` a built-in must still open it.
// - **Authored** (the `crate::ops` wire, and `install` through
//   [`from_authored_value`]) rejects it: the host is writing now, so the shape
//   means a stale copy of the built-in list and the drop is silent corruption.
//
// The rule is narrow on purpose: `attrs` beside a *reserved* name, nothing else.

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

/// The authored lane's verdict on a mark, without building it. Table cells are
/// the only caller: [`parse_cell`] reads their marks leniently, so a cell mark
/// reaches no strict decode that would raise the error on its own.
pub(crate) fn reject_unreadable_mark(v: &Value) -> Result<(), ParseError> {
    reject_mark_attrs(v)?;
    mark_shape(v)?;
    Ok(())
}

/// [`from_canonical_value`] for a content the **host authored just now**: the
/// `install` input, not a blob read back from storage. Same decode, plus the
/// reserved-name rule on every axis [`Content::validate`] checks: line kinds,
/// containers, prose marks, and table-cell marks.
pub fn from_authored_value(v: &Value) -> Result<Content, ParseError> {
    reject_reserved_attrs_deep(v)?;
    from_canonical_value(v)
}

/// The authored-lane scan [`from_authored_value`] runs. Structural rather than a
/// blind recursive walk: an unknown's `attrs` is opaque host payload that may
/// legitimately contain an object spelled `{"type": "link", "attrs": …}`, and
/// rejecting that would make the carrier unable to carry.
fn reject_reserved_attrs_deep(v: &Value) -> Result<(), ParseError> {
    for line in arr_or_empty(v, "lines") {
        reject_line_kind_attrs(line)?;
        for c in arr_or_empty(line, "containers") {
            reject_container_attrs(c)?;
        }
    }
    // Only the reserved-name half here: a prose mark that will not parse is
    // rejected by the strict decode `from_canonical_value` runs next.
    for m in arr_or_empty(v, "marks") {
        reject_mark_attrs(m)?;
    }
    // Cell marks ride the prose mark shape, so the rule follows them in, plus
    // the readability check, since no strict decode reaches them. Dispatch goes
    // through `KnownIslandType`, so a new mark-carrying type is a compile error
    // here rather than a silent skip.
    for island in arr_or_empty(v, "islands") {
        let ty = island.get("type").and_then(Value::as_str).unwrap_or_default();
        match crate::island::KnownIslandType::parse(ty) {
            Some(crate::island::KnownIslandType::Table) => {
                let Some(props) = island.get("props") else {
                    continue;
                };
                for cell in table_cell_values(props) {
                    for m in arr_or_empty(cell, "marks") {
                        reject_unreadable_mark(m)?;
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
        Content::RESERVED_LINE_KINDS,
        "attrs beside built-in kind",
    )
}

fn reject_container_attrs(v: &Value) -> Result<(), ParseError> {
    reject_reserved_attrs(
        v,
        "container",
        Content::RESERVED_CONTAINERS,
        "attrs beside built-in container",
    )
}

fn reject_mark_attrs(v: &Value) -> Result<(), ParseError> {
    reject_reserved_attrs(
        v,
        "type",
        Content::RESERVED_MARK_TYPES,
        "attrs beside built-in mark type",
    )
}

/// Error when `v` carries an `attrs` bag alongside a `discriminant` naming a
/// built-in: the producer meant an unknown and named a known. A non-object or a
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

// A pipe-table cell is inline-only: its own plain `text` plus `marks` whose
// ranges are USV offsets into that text. The marks ride the same wire shape
// prose marks use, so nothing forks the encoding.

/// Parse a table-cell object `{text, marks}` leniently: its plain text plus the
/// marks over it. A malformed mark is skipped rather than failing. Public so the
/// typst emitter renders a cell through the same parse the codecs use.
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

/// Build a table-cell object `{text, marks}`. Key order is fixed by the
/// recursive key-sort in [`Content::normalize`], not here.
pub(crate) fn cell_to_value(text: &str, marks: &[Mark]) -> Value {
    let mut m = Map::new();
    m.insert("text".into(), Value::String(text.to_string()));
    m.insert(
        "marks".into(),
        Value::Array(marks.iter().map(mark_to_value).collect()),
    );
    Value::Object(m)
}

/// Every cell object in a table island's props, header then each body row: the
/// undecoded half of [`table_cells`], walking the same cells in the same order.
pub(crate) fn table_cell_values(props: &Value) -> impl Iterator<Item = &Value> {
    let header = arr_or_empty(props, "header").iter();
    let rows = arr_or_empty(props, "rows")
        .iter()
        .flat_map(|row| as_slice(row).iter());
    header.chain(rows)
}

/// Every cell's `(text, marks)` in a table island's props: header then each
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
///   only grows: no cell is ever truncated). Materializing the count into the
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
/// shrinks: `cols` is the widest, so a shorter array only grows.
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
/// minting a fresh one, so a key this build does not recognize survives: a
/// cell is an opaque carrier, not an envelope (`DOCUMENT_STORAGE.md` § Open
/// vocabularies).
fn canon_cell(cell: &mut Value) {
    let (text, marks) = parse_cell(cell);
    let text = if text.contains(['\n', '\r']) {
        text.replace(['\n', '\r'], " ")
    } else {
        text
    };
    let canon = cell_to_value(&text, &crate::model::normalize_marks(marks));
    match (cell.as_object_mut(), canon) {
        // Overwrite the canonical keys, leave the rest: the merge
        // [`crate::ops`] does for a mark's fields on an op object.
        (Some(o), Value::Object(fields)) => o.extend(fields),
        // A non-object cell (a bare string, a null) holds no keys to preserve.
        (_, canon) => *cell = canon,
    }
}

/// A table island's shape violation, if any: the widths the header, `aligns`,
/// and each body row must share (the header width), plus the `\n`-free-cell rule.
/// The validate-side twin of [`normalize_table_props`].
pub(crate) fn table_shape_error(props: &Value) -> Option<Invariant> {
    // A present-but-non-array header can't carry column cells: `normalize`
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

pub(crate) fn island_to_value(island: &Island) -> Value {
    let mut m = Map::new();
    m.insert("id".into(), Value::String(island.id.clone()));
    m.insert("type".into(), Value::String(island.island_type.clone()));
    m.insert("props".into(), island.props.clone());
    m.insert("loss".into(), island.loss.as_str().into());
    Value::Object(m)
}

pub(crate) fn island_from_value(v: &Value) -> Result<Island, ParseError> {
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
        props: bag_from_wire(o, "props", "island props")?,
        // The class is carried whether or not this build interprets it, and
        // reads through `Loss::fidelity` at the *safe* end: never claim a class
        // the reader cannot interpret "carries faithfully". A missing key is the
        // faithful class, which is what an island with no loss recorded means.
        loss: o
            .get("loss")
            .and_then(Value::as_str)
            .map_or(Loss::LOSSLESS, Loss::new),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Fidelity, Line, LineKind};

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

    /// The decoder is the entry point for stored and
    /// caller-supplied content, and export recurses one frame per container. A
    /// 20 000-deep path that decoded clean would abort the process on
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

    /// Build a `Value` nesting `depth` array levels: iteratively, so *building*
    /// the fixture cannot overflow. Handling it still can: `Value`'s `Clone` and
    /// `Drop` both recurse, so the tests below probe just past the cap at 1 000
    /// (deep enough to be refused, shallow enough to pass around) rather than at
    /// a depth that overflows the test itself. A depth the guard must reject is a
    /// depth the test cannot hold either, which is the reason the limit exists.
    fn nested_arrays(depth: usize) -> Value {
        let mut v = Value::Null;
        for _ in 0..depth {
            v = Value::Array(vec![v]);
        }
        v
    }

    /// [`deep_container_nesting_is_rejected_at_decode`] on the
    /// payload axis, through the `Value` lane. The string lane is bounded by its
    /// parser (`serde_json::from_str` refuses past 128); the `Value` lane is the
    /// host-authored one (`install` reaches it) and has to refuse the same
    /// shape, since an unguarded deep `props` aborts the process rather than
    /// erroring.
    #[test]
    fn deep_json_payload_is_rejected_at_decode_on_the_value_lane() {
        let deep = nested_arrays(1_000);
        let cases: [(Value, &'static str); 4] = [
            (
                serde_json::json!({"text":"\u{fffc}","lines":[{"kind":"island","containers":[]}],
                  "marks":[],"islands":[{"id":"i1","type":"widget","loss":"lossless","props":deep}]}),
                "island props",
            ),
            (
                serde_json::json!({"text":"x","lines":[{"kind":"para","containers":[]}],
                  "marks":[{"start":0,"end":1,"type":"sparkle","attrs":deep}],"islands":[]}),
                "mark attrs",
            ),
            (
                serde_json::json!({"text":"x","lines":[{"kind":"callout","containers":[],"attrs":deep}],
                  "marks":[],"islands":[]}),
                "line attrs",
            ),
            (
                serde_json::json!({"text":"x","lines":[{"kind":"para",
                  "containers":[{"container":"indent","attrs":deep}]}],"marks":[],"islands":[]}),
                "container attrs",
            ),
        ];
        for (v, what) in cases {
            assert_eq!(
                from_canonical_value(&v),
                Err(ParseError::Invalid(Invariant::JsonTooDeep {
                    what,
                    max: crate::MAX_JSON_DEPTH,
                })),
                "{what} accepted a 1 000-deep payload"
            );
            // The authored lane funnels through the same decode, so it refuses
            // the same shape rather than trapping on the reserved-name scan.
            assert!(matches!(
                from_authored_value(&v),
                Err(ParseError::Invalid(Invariant::JsonTooDeep { .. }))
            ));
        }
    }

    /// The cap admits every payload a stored blob can carry, so closing the
    /// `Value` lane costs no stored population. Stated as the implication rather
    /// than an offset: `serde_json::from_str`'s own limit counts from the document
    /// root, not from the bag, so the wrapper levels it also charges are its
    /// business, what must hold is that anything the string lane delivers, the
    /// per-bag cap accepts.
    #[test]
    fn json_depth_cap_admits_every_storable_payload() {
        let content = |props: Value| {
            serde_json::json!({"text":"\u{fffc}","lines":[{"kind":"island","containers":[]}],
              "marks":[],"islands":[{"id":"i1","type":"widget","loss":"lossless","props":props}]})
        };
        assert!(from_canonical_value(&content(nested_arrays(crate::MAX_JSON_DEPTH))).is_ok());
        assert!(from_canonical_value(&content(nested_arrays(crate::MAX_JSON_DEPTH + 1))).is_err());

        // Across the whole boundary region, string-lane-accepted implies
        // `Value`-lane-accepted. The converse does not hold and need not: the
        // string lane's root-relative count refuses a few depths the bag cap
        // allows.
        let mut storable = 0;
        for d in 1..=crate::MAX_JSON_DEPTH + 8 {
            let v = content(nested_arrays(d));
            if Content::from_canonical_json(&v.to_string()).is_ok() {
                storable = d;
                assert!(
                    from_canonical_value(&v).is_ok(),
                    "the bag cap refused a {d}-deep props the string lane accepts"
                );
            }
        }
        assert!(
            storable > 0 && storable <= crate::MAX_JSON_DEPTH,
            "string lane's deepest storable props was {storable}"
        );
    }

    /// The legacy-attrs fold is the one frame that spends the depth
    /// without retaining the bag: it deep-clones the object it folds into, and it
    /// runs for a *built-in* name, where no `Unknown` arm reads the bag at all. A
    /// nested-object bag, since only an object folds.
    #[test]
    fn deep_json_payload_is_rejected_before_the_legacy_attrs_fold() {
        let mut deep = Value::Null;
        for _ in 0..1_000 {
            deep = serde_json::json!({"a": deep});
        }
        // `para` is reserved, `attrs` is a non-empty object: the fold path.
        let v = serde_json::json!({"text":"x","lines":[{"kind":"para","containers":[],"attrs":deep}],
          "marks":[],"islands":[]});
        assert_eq!(
            from_canonical_value(&v),
            Err(ParseError::Invalid(Invariant::JsonTooDeep {
                what: "line attrs",
                max: crate::MAX_JSON_DEPTH,
            }))
        );
    }

    /// An over-deep bag is refused whichever door it arrives at, so
    /// the op wire cannot install one either.
    #[test]
    fn deep_json_payload_is_rejected_on_the_op_wire() {
        let deep = nested_arrays(1_000);
        let op = serde_json::json!({"op":"add","start":0,"end":1,"type":"sparkle","attrs":deep});
        assert!(matches!(
            crate::ops::mark_op_from_value(&op),
            Err(ParseError::Invalid(Invariant::JsonTooDeep { .. }))
        ));
        let op = serde_json::json!({"op":"setKind","line":0,"kind":"callout","attrs":deep});
        assert!(matches!(
            crate::ops::line_op_from_value(&op),
            Err(ParseError::Invalid(Invariant::JsonTooDeep { .. }))
        ));
    }

    /// A wire position past `usize` is refused, not truncated. On
    /// wasm32 (the deployment target) `as usize` turned `2^32 + 5` into an
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
            loss: Loss::LOSSLESS,
        }];
        let mut two = one.clone();
        two.islands[0].props = serde_json::json!({"a": 2, "b": 1}); // keys reversed
        assert_eq!(one.to_canonical_json(), two.to_canonical_json());
    }

    #[test]
    fn golden_bytes_are_feature_independent() {
        // Pins the exact canonical form. Every object key is sorted, so the
        // bytes do not depend on serde_json's preserve_order feature. If this
        // string changes, the freeze changed: bump the schema version.
        let rt = sample();
        assert_eq!(
            rt.to_canonical_json(),
            r#"{"islands":[],"lines":[{"containers":[],"kind":"para"}],"marks":[{"end":5,"start":0,"type":"emph"},{"end":11,"start":6,"type":"strong"}],"text":"hello world"}"#
        );
    }

    #[test]
    fn from_canonical_json_rejects_invalid() {
        // lines.len() != segment count: must not silently round-trip.
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
        // as the built-in, dropping attrs: non-injective).
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

    /// `loss` is an open vocabulary on [`Island::island_type`]'s terms, not the
    /// block axes'. A class this build lacks is **carried**, not rewritten, so a
    /// reader that merely opens the document neither destroys the class nor
    /// moves the content hash. Reading it degrades to the safe end.
    #[test]
    fn unknown_loss_class_round_trips_and_reads_unrepresentable() {
        let json = concat!(
            r#"{"islands":[{"id":"i1","loss":"partial","props":{},"type":"widget"}],"#,
            r#""lines":[{"containers":[],"kind":"island"}],"marks":[],"text":"￼"}"#
        );
        let rt = Content::from_canonical_json(json).unwrap();
        assert_eq!(rt.islands[0].loss, Loss::new("partial"));
        assert_eq!(rt.islands[0].loss.fidelity(), Fidelity::Unrepresentable);
        assert_eq!(rt.to_canonical_json(), json);
    }

    /// The class is the stored value, so a built-in's name has one spelling and
    /// the reserved-name rule the block axes need has nothing to guard: what a
    /// caller hand-builds from that name **is** the built-in, and survives the
    /// round trip as itself.
    #[test]
    fn a_built_in_class_name_has_one_spelling() {
        assert_eq!(Loss::new("lossless"), Loss::LOSSLESS);
        let mut rt = Content::empty();
        rt.text = "\u{FFFC}".into();
        rt.lines = vec![Line {
            kind: LineKind::Island,
            containers: vec![],
            continues: false,
        }];
        rt.islands = vec![Island {
            id: "i1".into(),
            island_type: "widget".into(),
            props: serde_json::json!({}),
            loss: Loss::new("lossless"),
        }];
        assert_eq!(rt.validate(), Ok(()));
        let back = Content::from_canonical_json(&rt.to_canonical_json()).unwrap();
        assert_eq!(back.islands[0].loss, rt.islands[0].loss);
        assert_eq!(back.islands[0].loss.fidelity(), Fidelity::Lossless);
    }

    /// Every class `Fidelity` names round-trips to its own level, so the closed
    /// view and the wire spellings cannot drift apart.
    #[test]
    fn every_fidelity_level_round_trips_through_its_class() {
        for &f in Fidelity::ALL {
            assert_eq!(Loss::new(f.as_str()).fidelity(), f);
        }
    }

    /// The block vocabulary is open on the mark axis' terms. A
    /// `kind`/`container` this build lacks decodes to `Unknown` (the document
    /// **opens**) and re-encodes byte-identically, so a construct a future
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
        // A missing/non-string discriminator is still a shape error: the open
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

    /// An unknown line kind / container may not reuse a built-in
    /// name: it would serialize as the built-in and parse back as one, dropping
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

    /// The authored lane (`install`) applies the reserved-name rule
    /// the decoders cannot: by the time a lenient reader has resolved `"para"`
    /// to `Para`, the `attrs` are gone and `validate` has nothing to object to.
    /// Every axis `validate` checks, including cell marks.
    ///
    /// The last case: a cell mark that will not parse at all.
    /// It is the one axis with no strict decode behind it (`parse_cell` skips
    /// what it cannot read and `canon_cell` makes the skip permanent) so
    /// without this the host's mark vanishes with no signal.
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
            // table cell mark with no `type` at all
            concat!(
                r#"{"islands":[{"id":"i1","loss":"lossless","props":{"aligns":["none"],"#,
                r#""header":[{"marks":[{"end":1,"start":0}],"text":"h"}],"#,
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
            // The storage lane opens all five: a document written before the
            // name was built in must keep loading.
            assert!(
                Content::from_canonical_json(json).is_ok(),
                "storage lane rejected: {json}"
            );
        }
        // …and what storage does with the unreadable one: skips it, keeping the
        // document openable.
        let rt = Content::from_canonical_json(bad[4]).unwrap();
        assert!(rt.islands[0].props["header"][0]["marks"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    /// The authored lane's scan is structural, not a blind walk. An
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

    /// Opaque block attrs are hash input, so their key order must
    /// not leak into the canonical bytes: the unknown-mark rule, one axis over.
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

    /// Promotion moves a construct's payload from the opaque bag to
    /// named siblings, and every blob written before it still spells the payload
    /// the old way. The storage lane folds the bag in, so the promoted decoder
    /// reads what the unknown wrote instead of dropping it. Pinned on the
    /// built-ins carrying payload today: the fold keys off `RESERVED_*`, so a
    /// promoted name joins it by the same edit that promotes it.
    #[test]
    fn built_in_decoders_read_the_legacy_attrs_form() {
        let cases: [(Value, LineKind); 2] = [
            (
                serde_json::json!({"kind": "heading", "attrs": {"level": 2}}),
                LineKind::Heading { level: 2 },
            ),
            (
                serde_json::json!({"kind": "code", "attrs": {"lang": "rust"}}),
                LineKind::Code {
                    lang: Some("rust".into()),
                },
            ),
        ];
        for (v, want) in cases {
            assert_eq!(line_kind_from_value(&v).unwrap(), want);
        }
        let item = serde_json::json!({
            "container": "list_item",
            "attrs": {"ordered": true, "start": 3, "ordinal": 1}
        });
        assert_eq!(
            container_from_value(&item).unwrap(),
            Container::ListItem {
                ordered: true,
                start: 3,
                ordinal: 1,
            }
        );
        let link = serde_json::json!({"start": 0, "end": 1, "type": "link", "attrs": {"url": "u"}});
        assert_eq!(
            mark_from_value(&link).unwrap().kind,
            MarkKind::Link { url: "u".into() }
        );
        // Both spellings present: the named sibling is the canonical one.
        let both = serde_json::json!({"kind": "heading", "level": 3, "attrs": {"level": 2}});
        assert_eq!(
            line_kind_from_value(&both).unwrap(),
            LineKind::Heading { level: 3 }
        );
        // An unknown's bag is payload, not a source of named fields; including a
        // key that would re-target the match if the fold read the discriminator
        // back out of it.
        let unknown = serde_json::json!({"kind": "callout", "attrs": {"kind": "heading", "level": 2}});
        assert_eq!(
            line_kind_from_value(&unknown).unwrap(),
            LineKind::Unknown {
                tag: "callout".into(),
                attrs: serde_json::json!({"kind": "heading", "level": 2}),
            }
        );
        // Re-encode is the promoted spelling, so opening a legacy blob under the
        // release that promotes its tag moves the document's canonical bytes:
        // the read-repair / accepted-movement case § Byte-stability governs.
        let legacy = r#"{"islands":[],"lines":[{"attrs":{"level":2},"containers":[],"kind":"heading"}],"marks":[],"text":"hi"}"#;
        assert_eq!(
            Content::from_canonical_json(legacy)
                .unwrap()
                .to_canonical_json(),
            r#"{"islands":[],"lines":[{"containers":[],"kind":"heading","level":2}],"marks":[],"text":"hi"}"#
        );
    }

    /// `ord` is part of the freeze, and a promoted type takes the
    /// slot `Unknown` held. Anywhere else and a build that knows the type orders
    /// it against the built-ins differently from a build that reads it as
    /// `Unknown`: one document, two canonical forms.
    #[test]
    fn unknown_holds_the_last_mark_ordinal() {
        let all = [
            MarkKind::Strong,
            MarkKind::Emph,
            MarkKind::Underline,
            MarkKind::Strike,
            MarkKind::Code,
            MarkKind::Link { url: "u".into() },
            MarkKind::Anchor { id: "a".into() },
            MarkKind::Unknown {
                tag: "kbd".into(),
                attrs: Value::Null,
            },
        ];
        // Exhaustive on purpose: a new variant is a compile error here, which is
        // where the placement rule gets read, rather than a slot silently taken
        // after `Unknown`.
        for k in &all {
            match k {
                MarkKind::Strong
                | MarkKind::Emph
                | MarkKind::Underline
                | MarkKind::Strike
                | MarkKind::Code
                | MarkKind::Link { .. }
                | MarkKind::Anchor { .. }
                | MarkKind::Unknown { .. } => {}
            }
        }
        let ords: Vec<u8> = all.iter().map(MarkKind::ord).collect();
        assert_eq!(ords, (0..all.len() as u8).collect::<Vec<_>>());
        assert!(matches!(all.last(), Some(MarkKind::Unknown { .. })));
    }

    /// Formatting-class membership is stored meaning. Two adjacent
    /// unknowns are two marks; two adjacent formatting marks are one. Promoting a
    /// tag into the class therefore rewrites documents nobody edited, which makes
    /// it a canonical-byte event rather than a silent widening.
    #[test]
    fn formatting_class_membership_decides_adjacent_union() {
        let mut rt = Content::empty();
        rt.text = "abcd".into();
        let unknown = |start, end| Mark {
            start,
            end,
            kind: MarkKind::Unknown {
                tag: "kbd".into(),
                attrs: serde_json::json!({}),
            },
        };
        rt.marks = vec![unknown(0, 2), unknown(2, 4)];
        rt.normalize();
        assert_eq!(rt.marks.len(), 2);
        rt.marks = vec![
            Mark {
                start: 0,
                end: 2,
                kind: MarkKind::Strong,
            },
            Mark {
                start: 2,
                end: 4,
                kind: MarkKind::Strong,
            },
        ];
        rt.normalize();
        assert_eq!(rt.marks.len(), 1);
    }

    /// Promotion grows `RESERVED_*`, and the authored lane then
    /// refuses a shape it accepted the release before. By design: a host still
    /// authoring the unknown spelling of a name that now means the built-in is
    /// writing the silent drop the rule exists to catch, but from the host's
    /// side it reads as a release breaking its writes.
    #[test]
    fn reserved_growth_flips_authored_acceptance() {
        let doc = |kind: &str| {
            serde_json::json!({
                "islands": [],
                "lines": [{"attrs": {"level": 2}, "containers": [], "kind": kind}],
                "marks": [],
                "text": "hi",
            })
        };
        // Outside `RESERVED_LINE_KINDS` today: an unknown carrying its payload.
        assert!(from_authored_value(&doc("callout")).is_ok());
        // Inside it: what `"callout"` becomes the release it is promoted.
        assert!(matches!(
            from_authored_value(&doc("heading")),
            Err(ParseError::Shape(_))
        ));
    }
}
