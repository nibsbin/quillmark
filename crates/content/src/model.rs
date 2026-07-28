//! The `Content` content model — one text sequence per field carrying line
//! attributes, anchored marks, and embedded islands, over a single coordinate
//! space of Unicode scalar values (Rust `char`).
//!
//! This is the freeze (issue #831): the mark set, the three
//! normalization rules, and the invariants are what canonical serialization
//! commits to. Everything an editor disagrees on (edge-expand,
//! adjacent-merge-at-insertion) is *not* encoded — the model only ever stores
//! the resulting range, so the stored form is identical whatever the editor
//! did.

use crate::normalize::is_bidi_char;
use serde_json::Value as JsonValue;

/// A position in a [`Content`], counted in Unicode scalar values (USV) — never
/// bytes, never UTF-16 units. One astral char is 1 USV / 4 UTF-8 bytes / 2
/// UTF-16 units. Conversions to/from the JS (UTF-16) and Rust (UTF-8)
/// boundaries live in [`crate::usv`].
pub type Usv = usize;

/// U+FFFC OBJECT REPLACEMENT CHARACTER — the single-USV slot an island occupies
/// in the content. One slot per island; every slot has a backing island. A stray
/// slot (or a slot with no island) is an invariant violation.
pub const ISLAND_SLOT: char = '\u{FFFC}';

/// One content field as a content: the text plus the structure that rides on it.
///
/// Invariants (established once by import normalization, checked by
/// [`Content::validate`]): the text holds no `\r` and no bidi controls; the
/// count of [`ISLAND_SLOT`] equals `islands.len()`; `lines.len()` equals the
/// number of `\n`-separated segments; marks are normalized (sorted, unioned).
#[derive(Debug, Clone, PartialEq)]
pub struct Content {
    /// The content. `\n` is a line boundary; [`ISLAND_SLOT`] is an island slot.
    pub text: String,
    /// One entry per `\n`-separated segment of `text`, in order. The line tree
    /// is *derived* from this flat list plus each line's `containers` path — it
    /// is never stored, so a split/join is a single-char edit with no identity
    /// crisis (there are no paragraph IDs).
    pub lines: Vec<Line>,
    /// Marks over char ranges, kept normalized: sorted by
    /// `(start, end, kind-ord, attrs)`, same-kind formatting marks unioned.
    pub marks: Vec<Mark>,
    /// One entry per [`ISLAND_SLOT`], in slot order (ascending char position).
    pub islands: Vec<Island>,
}

/// A line's attributes: its block role plus the container path it sits in.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub kind: LineKind,
    /// Ancestor containers, outermost first. A multi-paragraph list item is two
    /// `Para` lines sharing one `[ListItem]` path; a paragraph in a quote in a
    /// list item is `[ListItem, Quote]`.
    pub containers: Vec<Container>,
    /// Whether this line continues the previous line's *block* across a hard
    /// line break (no paragraph break between), rather than starting a new
    /// block. `false` = a new block (paragraph spacing on either side); `true` =
    /// a within-block line break (markdown hard break; consecutive lines of one
    /// code fence). The first line is always `false`. This is what keeps a hard
    /// break (backend `#linebreak()`) distinct from a paragraph boundary through
    /// the freeze, and what groups a code fence's lines without an
    /// adjacency heuristic.
    pub continues: bool,
}

/// The block role of a line. The tree between lines is inferred: two adjacent
/// lines with equal `kind`+`containers` are two blocks of that role (e.g. two
/// paragraphs), never one.
///
/// **Open**, on the same terms as [`MarkKind`]: an unrecognized role round-trips
/// as [`LineKind::Unknown`] and *projects* as [`LineKind::Para`], so adding a
/// block construct (a callout, a footnote, a task item) is not a document schema
/// event — an older reader renders the future construct as a plain paragraph
/// instead of refusing the whole document, and the opaque tag+attrs still reach a
/// reader that understands them (`DOCUMENT_STORAGE.md` § Open vocabularies).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LineKind {
    Para,
    /// ATX/Setext heading, level 1..=6.
    Heading {
        level: u8,
    },
    /// A line of a code block. `lang` is the (sanitized) info string, shared by
    /// every line of the same block.
    Code {
        lang: Option<String>,
    },
    /// A block-level island: the line's sole content is one [`ISLAND_SLOT`].
    Island,
    /// A thematic break (`---`/`***`/`___`). The line carries no text — the
    /// break is the line itself, parallel to how an island's content is its
    /// one slot char.
    Rule,
    /// Open-set escape hatch — a block role this build does not know,
    /// round-tripped opaque and projected as [`LineKind::Para`]. Carries
    /// arbitrary text, like the `Para` it projects as, so no
    /// [`LineKindMismatch`] constrains it.
    Unknown {
        tag: String,
        attrs: JsonValue,
    },
}

impl LineKind {
    /// Whether the line projects as a paragraph: [`LineKind::Para`] itself, or an
    /// unknown role, which every projection renders as one. For the tests a
    /// `match` cannot serve — a `matches!(kind, Para)` that special-cases the
    /// paragraph (suppressing an empty block, say) reads as complete but drops
    /// the open arm, and the two emitters then drift on the construct neither
    /// knows.
    ///
    /// An exhaustive `match` keeps listing both arms, and in this crate the
    /// compiler still polices that — `#[non_exhaustive]` does not apply within
    /// the defining crate. It does apply to the typst backend, whose emitter
    /// folds both arms into one wildcard; there the ladder, not the compiler,
    /// is what keeps a new role rendering.
    pub fn projects_as_para(&self) -> bool {
        matches!(self, LineKind::Para | LineKind::Unknown { .. })
    }
}

/// A container a line nests inside. The ancestor path is a `Vec<Container>`.
///
/// **Open**, on [`LineKind`]'s terms: an unrecognized container round-trips as
/// [`Container::Unknown`] and projects *transparently* — its lines render at the
/// enclosing level, with no prefix, no wrapper, and no grouping of their own.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Container {
    /// A list item. `ordered` distinguishes `1.` from `-`; `start` is the list's
    /// first number (1 by default); `ordinal` is this item's 0-based index in
    /// its list. Two *adjacent* lines belong to the same item iff their whole
    /// container path (ordinals included) is equal — so a multi-paragraph item
    /// is two lines sharing one `ListItem`, while the next item differs by
    /// `ordinal`. (Identity is path **plus contiguity**: two sibling inner lists
    /// under one outer item can produce equal first-item paths, distinguished
    /// only by the non-adjacency of their runs.) Positional and deterministic —
    /// no minted ids.
    ListItem {
        ordered: bool,
        start: u64,
        ordinal: u64,
    },
    /// A block quote. Adjacent lines sharing `[Quote]` are one multi-paragraph
    /// quote; two adjacent separate quotes are not distinguished (they merge on
    /// round-trip — a documented canonicalization).
    Quote,
    /// Open-set escape hatch — a container this build does not know, kept in the
    /// path so it round-trips, transparent to both projections. Two adjacent
    /// lines sit in the same one iff their whole `(tag, attrs)` is equal, the
    /// path-plus-contiguity rule the known containers use.
    Unknown {
        tag: String,
        attrs: JsonValue,
    },
}

/// A mark over a char range `[start, end)`. `start == end` (zero-width) is legal
/// only for [`MarkKind::Anchor`]; normalization drops zero-width formatting.
#[derive(Debug, Clone, PartialEq)]
pub struct Mark {
    pub start: Usv,
    pub end: Usv,
    pub kind: MarkKind,
}

/// The mark set — **open**: an unknown kind round-trips as [`MarkKind::Unknown`],
/// absorbed as a new *type*, never a changed semantics of a known one. Two
/// algebra classes: formatting is a property of a range (two coincident are
/// redundant); identity is a handle (two over the same range are two things).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum MarkKind {
    // Formatting — round-trippable projection marks. `is_formatting()`.
    Strong,
    Emph,
    Underline,
    Strike,
    Code,
    Link {
        url: String,
    },
    // Identity — a handle, not a property. Never merged, may be zero-width.
    /// A comment thread or stable anchor, carried by id and rebased across
    /// edits like any position. The id is caller-supplied, unique per `Content`,
    /// opaque and invariant while the mark lives; positions rebase, the id never
    /// does, and moved-and-rewritten text drops the mark whole
    /// (`DOCUMENT_STORAGE.md` § Anchor-id identity). No markdown projection
    /// (omitted on export; survives via diff-rebase).
    Anchor {
        id: String,
    },
    // Open-set escape hatch — an unknown mark type, round-tripped opaque.
    Unknown {
        tag: String,
        attrs: JsonValue,
    },
}

/// A structured object with no honest text encoding — a table, figure, or future
/// embed — occupying one [`ISLAND_SLOT`] in the content.
#[derive(Debug, Clone, PartialEq)]
pub struct Island {
    /// Deterministically minted, session-stable id — `isl-{n}` by import
    /// position (`import::mint_island`). Part of the canonical form and thus
    /// hash input; deterministic by contract, never ambient, so equal content
    /// hashes equal (`DOCUMENT_STORAGE.md` § Island-id determinism). Edits keep
    /// it stable rather than re-deriving it, so [`Content::validate`] enforces
    /// uniqueness, not positional equality.
    pub id: String,
    /// Island type discriminator (`"table"`, `"image"`, …). Unknown types
    /// round-trip opaque.
    pub island_type: String,
    /// Typed payload. Recursively key-sorted by normalization so it hashes
    /// deterministically despite `serde_json`'s `preserve_order`.
    pub props: JsonValue,
    /// How faithfully the markdown projection can carry this island.
    pub loss: Loss,
}

/// The markdown-projection loss class of an island — a **description** of how
/// faithfully the projection carries it, for a consumer to surface (a caller
/// warned that a form field silently dropped a table, issue #1043). It is not a
/// switch: [`crate::export::to_markdown`] dispatches on
/// [`Island::island_type`], never on this.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Loss {
    /// Markdown carries it faithfully (round-trips identically).
    Lossless,
    /// Markdown carries an approximation (round-trips visibly, not identically).
    Degraded,
    /// No markdown encoding — what an island type with no projection carries.
    Unrepresentable,
    /// A class a future writer stamped that this build lacks, carried verbatim.
    ///
    /// `loss` is open on the terms the other four vocabularies use — mark
    /// `type`, line `kind`, container, island `type` all round-trip an
    /// unrecognized member opaque. Carrying the raw tag rather than rewriting it
    /// is what keeps a reader that merely opens a document from moving that
    /// document's content hash (`DOCUMENT_STORAGE.md` § Byte-stability).
    ///
    /// Read fidelity through [`Loss::fidelity`], never by matching this arm: an
    /// unrecognized class degrades to [`Unrepresentable`](Loss::Unrepresentable),
    /// so nothing is claimed to carry faithfully on the strength of a name this
    /// build cannot interpret.
    Unknown(String),
}

impl Loss {
    /// The fidelity this class describes, with an unrecognized class degraded to
    /// the safe end. Never returns [`Unknown`](Loss::Unknown).
    ///
    /// Carrying the raw tag preserves the byte round-trip. Reading it through
    /// here keeps that carriage from being mistaken for a claim about the
    /// projection.
    pub fn fidelity(&self) -> Loss {
        match self {
            Loss::Lossless => Loss::Lossless,
            Loss::Degraded => Loss::Degraded,
            Loss::Unrepresentable | Loss::Unknown(_) => Loss::Unrepresentable,
        }
    }
}

impl MarkKind {
    /// Formatting marks are a property of a range and union when coincident;
    /// identity/unknown marks are handles and never merge (Spike-A rules).
    ///
    /// Class membership is stored meaning, not presentation: promoting an
    /// open-set tag *into* this class starts unioning adjacent runs that
    /// round-tripped as two marks, so the promotion moves the canonical bytes of
    /// documents nobody edited (`DOCUMENT_STORAGE.md` § Promoting a vocabulary
    /// member).
    pub fn is_formatting(&self) -> bool {
        matches!(
            self,
            MarkKind::Strong
                | MarkKind::Emph
                | MarkKind::Underline
                | MarkKind::Strike
                | MarkKind::Code
                | MarkKind::Link { .. }
        )
    }

    /// Total order over kinds for the canonical sort tie-break, after
    /// `(start, end)`. Stable across releases — part of the freeze.
    ///
    /// A new variant takes the slot immediately **before** [`MarkKind::Unknown`],
    /// pushing `Unknown` up by one. That is the only placement where a build that
    /// knows the type and a build that reads it as `Unknown` order it identically
    /// against every built-in; anywhere else is two canonical forms for one
    /// document, one per reader (`DOCUMENT_STORAGE.md` § Promoting a vocabulary
    /// member). The block axes sort by nothing, so the rule is the mark axis'
    /// alone.
    pub fn ord(&self) -> u8 {
        match self {
            MarkKind::Strong => 0,
            MarkKind::Emph => 1,
            MarkKind::Underline => 2,
            MarkKind::Strike => 3,
            MarkKind::Code => 4,
            MarkKind::Link { .. } => 5,
            MarkKind::Anchor { .. } => 6,
            MarkKind::Unknown { .. } => 7,
        }
    }

    /// Attribute tie-break string, appended after `ord` in the canonical sort so
    /// two marks that differ only in attrs order deterministically. Also the
    /// grouping key for same-kind union (two formatting marks union only when
    /// this matches — e.g. two `link`s union only at the same url).
    pub fn attrs_key(&self) -> String {
        match self {
            MarkKind::Link { url } => url.clone(),
            MarkKind::Anchor { id } => id.clone(),
            MarkKind::Unknown { tag, attrs } => {
                // Attrs sorted so the key is order-insensitive.
                format!("{}\u{0}{}", tag, canonical_json_string(attrs))
            }
            _ => String::new(),
        }
    }
}

/// A `serde_json::Value` rendered to a string with object keys recursively
/// sorted — order-insensitive, so it is a stable comparison/grouping key.
fn canonical_json_string(v: &JsonValue) -> String {
    serde_json::to_string(&sort_keys_owned(v.clone())).unwrap_or_default()
}

/// Whether every object in `v` already has its keys in ascending order,
/// recursively — the cheap allocation-free check that lets a re-normalize skip
/// rebuilding an already-canonical `props`/`attrs` tree.
/// Once normalized, an untouched tree stays sorted, so a per-keystroke
/// re-normalize pays a scan instead of a full clone.
pub(crate) fn is_value_key_sorted(v: &JsonValue) -> bool {
    match v {
        JsonValue::Array(items) => items.iter().all(is_value_key_sorted),
        JsonValue::Object(map) => {
            map.keys().zip(map.keys().skip(1)).all(|(a, b)| a <= b)
                && map.values().all(is_value_key_sorted)
        }
        _ => true,
    }
}

/// `true` when `v` nests deeper than `max` container levels — the guard that
/// keeps the recursive walkers above ([`is_value_key_sorted`], [`sort_keys_owned`])
/// and `Value`'s own `Drop` inside a bounded frame count.
///
/// The walk is iterative (explicit stack), so the check itself cannot overflow on
/// the adversarially deep input it exists to detect. The unit is **container
/// levels**, not nodes: only arrays and objects are charged a level and the
/// scalar leaf at the bottom of a chain is never checked, so `max` nested
/// containers are accepted whether the deepest holds a scalar, is empty, or holds
/// another container, and `max + 1` is rejected in every case. Level-charging
/// matches [`quillmark_core::json_depth_exceeds`], the document-side twin, so the
/// two boundaries reject the identical shape.
pub(crate) fn json_depth_exceeds(v: &JsonValue, max: usize) -> bool {
    // (value, depth) pairs; depth counts container levels entered.
    let mut stack: Vec<(&JsonValue, usize)> = vec![(v, 0)];
    while let Some((v, depth)) = stack.pop() {
        match v {
            JsonValue::Array(items) => {
                if depth + 1 > max {
                    return true;
                }
                stack.extend(items.iter().map(|c| (c, depth + 1)));
            }
            JsonValue::Object(map) => {
                if depth + 1 > max {
                    return true;
                }
                stack.extend(map.values().map(|c| (c, depth + 1)));
            }
            _ => {}
        }
    }
    false
}

/// [`json_depth_exceeds`] against [`MAX_JSON_DEPTH`](crate::MAX_JSON_DEPTH) as an
/// [`Invariant`] result, `what` naming the bag. The one spelling shared by
/// [`Content::validate`] and the decoders in [`crate::serial`], so the wire and
/// in-process readings of the limit cannot drift.
pub(crate) fn check_json_depth(v: &JsonValue, what: &'static str) -> Result<(), Invariant> {
    if json_depth_exceeds(v, crate::MAX_JSON_DEPTH) {
        return Err(Invariant::JsonTooDeep {
            what,
            max: crate::MAX_JSON_DEPTH,
        });
    }
    Ok(())
}

/// Put `v` in canonical key order, rebuilding it only when a key is actually out
/// of order — an untouched tree (a pure text splice) stays sorted, so the
/// per-keystroke path pays the scan and skips the deep clone.
pub(crate) fn canonicalize_keys(v: &mut JsonValue) {
    if !is_value_key_sorted(v) {
        *v = sort_keys_owned(std::mem::take(v));
    }
}

/// The crate's one key sorter: reorder every object's keys by **moving** each
/// entry into a freshly key-sorted map, recursively. Pins island `props` (and
/// unknown line/container/mark `attrs`) against `preserve_order` leaking
/// insertion order into the canonical bytes / content hash. Re-sorting into a
/// new `serde_json::Map` (not sorting in place) keeps that independent of
/// whether `serde_json`'s `preserve_order` feature is on in the crate graph.
///
/// The fixed struct keys land alphabetically and an already-sorted
/// `props`/`attrs` re-sorts to itself. The leaves (the `text` string, mark
/// attrs, arrays) move rather than deep-clone, so a tree built once by
/// `to_value` is canonicalized without a second full clone — and the encoders
/// emit their payload bags verbatim, one pass over the finished tree rather
/// than one per bag.
pub(crate) fn sort_keys_owned(v: JsonValue) -> JsonValue {
    match v {
        JsonValue::Array(items) => {
            JsonValue::Array(items.into_iter().map(sort_keys_owned).collect())
        }
        JsonValue::Object(map) => {
            let mut entries: Vec<(String, JsonValue)> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut out = serde_json::Map::with_capacity(entries.len());
            for (k, child) in entries {
                out.insert(k, sort_keys_owned(child));
            }
            JsonValue::Object(out)
        }
        other => other,
    }
}

/// Ways a [`Content`] can violate its invariants. Returned by
/// [`Content::validate`]; import normalization guarantees none of these.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Invariant {
    /// `\r` in the text (line endings must be normalized to `\n`).
    CarriageReturn,
    /// A bidi formatting control in the text.
    BidiControl(char),
    /// `island_slot_count != islands.len()`.
    IslandSlotMismatch { slots: usize, islands: usize },
    /// `lines.len() != newline_segment_count`.
    LineCountMismatch { lines: usize, segments: usize },
    /// A mark range runs past the content or is inverted (`start > end`).
    MarkOutOfRange { start: Usv, end: Usv, len: Usv },
    /// A zero-width formatting mark survived normalization.
    ZeroWidthFormatting { at: Usv },
    /// A heading level outside 1..=6.
    BadHeadingLevel(u8),
    /// The first line has `continues: true` (nothing precedes it to continue).
    FirstLineContinues,
    /// An [`MarkKind::Unknown`] reused a reserved built-in `type` name.
    ReservedUnknownTag(String),
    /// A [`LineKind::Unknown`] reused a reserved built-in `kind` name — its
    /// serialization would parse back as the built-in, dropping its attrs.
    ReservedUnknownLineKind(String),
    /// A [`Container::Unknown`] reused a reserved built-in `container` name, the
    /// same non-injectivity as [`Invariant::ReservedUnknownLineKind`].
    ReservedUnknownContainer(String),
    /// A formatting mark edge sits on a `\n` (normalization should have trimmed
    /// it) — a hand-built content that skipped `normalize`.
    MarkEdgeOnNewline { at: Usv },
    /// A table island's `aligns` length differs from its column count (the
    /// header width). `normalize` syncs `aligns` to the column count.
    TableAlignsMismatch { aligns: usize, cols: usize },
    /// A table island body row's width differs from the column count (the header
    /// width). `normalize` pads short rows (and the header) to the widest.
    TableRaggedRow { row: usize, width: usize, cols: usize },
    /// A table cell's text carries a `\n` — cells are single-line (a newline
    /// would break the exported table). `cell` is the flat header-then-rows
    /// index; `normalize` rewrites the newline to a space.
    TableCellNewline { cell: usize },
    /// Two islands share an `id`. Ids are deterministic, session-stable
    /// identities (hash input, so never ambient); import mints them by index so
    /// they never collide, but a hand-built or round-tripped content can.
    /// Downstream code that keys islands by id would otherwise silently pick the
    /// wrong one. Uniqueness is the id invariant `validate` enforces — positional
    /// equality is not, since edits keep an island's id stable across renumbers.
    IslandIdCollision { id: String },
    /// Two prose anchors share an `id`, or one carries the empty id. An anchor
    /// id is a caller-supplied, opaque handle, unique per `Content` (hash input,
    /// never ambient in the twin's sense; `DOCUMENT_STORAGE.md` § Anchor-id
    /// identity). `RemoveAnchor { id }` retains-out *every* match, so a shared id
    /// makes removing one destroy both; the empty id is a degenerate handle.
    /// Scope is prose marks — cell anchors are outside the op surface.
    AnchorIdCollision { id: String },
    /// A table island's `header` prop is present but not a JSON array — it
    /// cannot carry column cells. `normalize` rewrites a non-array header to an
    /// empty array (a zero-column, content-free table).
    TableHeaderNotArray,
    /// A line's [`LineKind`] contradicts its text. Export trusts the kind and
    /// never re-reads the segment, so an unchecked mismatch is silent text loss
    /// (an `Island`-tagged prose line projects to its resolved island alone).
    LineKindMismatch { line: usize, mismatch: LineKindMismatch },
    /// A line's container path is nested deeper than [`MAX_NESTING_DEPTH`]. Both
    /// emitters recurse one frame per container, so an unbounded path overflows
    /// the stack; import caps it, and this is the same cap for the content that
    /// never went through import (a decoded blob, a hand-built value).
    NestingTooDeep {
        line: usize,
        depth: usize,
        max: usize,
    },
    /// An opaque JSON payload — an island's `props`, an unknown line/container/
    /// mark's `attrs` — nests deeper than [`MAX_JSON_DEPTH`](crate::MAX_JSON_DEPTH):
    /// [`Invariant::NestingTooDeep`] on the payload axis. `what` names the bag.
    /// No true depth — the check bails at the first over-deep container rather
    /// than measuring past the limit.
    JsonTooDeep { what: &'static str, max: usize },
}

/// The way a line's text can contradict its [`LineKind`]. `Para` and `Heading`
/// carry arbitrary text including slots (an inline image is a slot in a `Para`),
/// so only the three kinds whose contract *names* their content constrain it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LineKindMismatch {
    /// [`LineKind::Island`] whose text is not exactly one [`ISLAND_SLOT`].
    IslandNotOneSlot,
    /// [`LineKind::Rule`] carrying text — the break is the line itself.
    RuleNotEmpty,
    /// [`LineKind::Code`] carrying an [`ISLAND_SLOT`]. A fence emits its text
    /// verbatim, so the slot lands raw in the output and re-imports as nothing:
    /// the island and its slot both vanish.
    CodeHasSlot,
}

/// How a line's text contradicts `kind`, if it does — the single reading behind
/// the [`Invariant::LineKindMismatch`] and
/// [`ApplyError::LineKindMismatch`](crate::ops::ApplyError::LineKindMismatch)
/// twins, so the validate-time and op-time checks cannot drift.
pub fn line_kind_mismatch(kind: &LineKind, seg: &str) -> Option<LineKindMismatch> {
    match kind {
        LineKind::Island => {
            let mut chars = seg.chars();
            match (chars.next(), chars.next()) {
                (Some(ISLAND_SLOT), None) => None,
                _ => Some(LineKindMismatch::IslandNotOneSlot),
            }
        }
        LineKind::Rule if !seg.is_empty() => Some(LineKindMismatch::RuleNotEmpty),
        LineKind::Code { .. } if seg.contains(ISLAND_SLOT) => Some(LineKindMismatch::CodeHasSlot),
        _ => None,
    }
}

impl Content {
    /// An empty content: one empty `Para` line, no marks, no islands.
    pub fn empty() -> Self {
        Content {
            text: String::new(),
            lines: vec![Line {
                kind: LineKind::Para,
                containers: Vec::new(),
                continues: false,
            }],
            marks: Vec::new(),
            islands: Vec::new(),
        }
    }

    /// Total length in USV.
    pub fn len_usv(&self) -> Usv {
        self.text.chars().count()
    }

    /// Whether this content satisfies the `richtext(inline)` constraint: exactly
    /// one `Para` line, sitting in no container, with no islands. A single line
    /// can never `continues` (line 0 is always `false`), so that dimension is
    /// implied. [`Content::empty`] is inline (one empty `Para`), so a blank or
    /// zero-filled inline field passes.
    pub fn is_inline(&self) -> bool {
        self.islands.is_empty()
            && self.lines.len() == 1
            && self.lines[0].kind == LineKind::Para
            && self.lines[0].containers.is_empty()
    }

    /// Whether this content satisfies the `plaintext` constraint: no marks, no
    /// islands, and every line is a plain `Para` sitting in no container. It is
    /// the multi-line generalization of [`is_inline`](Self::is_inline) (which
    /// additionally pins the content to one line) with the mark/island exclusion
    /// made explicit — a plaintext value carries prose the author navigates but
    /// no formatting. `continues` is unconstrained: a lone `\n` may be a
    /// within-paragraph break. [`Content::empty`] is plain.
    ///
    /// This is the plaintext analogue of `is_inline`, enforced at coercion and
    /// validation with the `NotPlain` error; the distinguishing property of
    /// plaintext over `richtext { marks: [] }` is the *literal* codec
    /// ([`crate::import::from_plaintext`]), not this predicate.
    pub fn is_plain(&self) -> bool {
        self.marks.is_empty()
            && self.islands.is_empty()
            && self
                .lines
                .iter()
                .all(|l| l.kind == LineKind::Para && l.containers.is_empty())
    }

    /// Whether the content carries no renderable content: the text is empty or
    /// whitespace-only. An island slot ([`ISLAND_SLOT`], U+FFFC) is not
    /// whitespace, so an island-bearing content is never blank. Body-disabled
    /// validation and round-trip emit key on it.
    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// Number of `\n`-separated segments — the required `lines.len()`.
    pub fn segment_count(&self) -> usize {
        self.text.chars().filter(|c| *c == '\n').count() + 1
    }

    /// Normalize marks in place: drop zero-width formatting, union same-kind
    /// formatting that is adjacent or overlapping, recursively key-sort island
    /// props and unknown-mark attrs, then sort marks canonically. Idempotent —
    /// the fixed point the canonical serialization commits to.
    pub fn normalize(&mut self) {
        // Line kinds whose contract names their content, against the content
        // they now hold. A splice writes text, never kinds: typing into a table
        // line leaves it `Island` over prose, joining a fence to an image line
        // leaves it `Code` over a slot, and export reads the kind and not the
        // text — so the un-repaired line projects its content away. Demote to
        // `Para`, the kind that carries anything (an inline island is a slot in a
        // `Para`), which is what re-importing the line's own markdown yields.
        // The repair-side twin of the [`Invariant::LineKindMismatch`] check; the
        // deliberate mis-tag is refused up front instead
        // ([`ApplyError::LineKindMismatch`](crate::ops::ApplyError::LineKindMismatch)),
        // since silently undoing an op the caller asked for is worse than an error.
        // The same pass canonicalizes the open block vocabulary's opaque `attrs`:
        // it is hash input like every other field, so two equal contents whose
        // unknown lines were built key-reversed must not serialize to different
        // bytes. A demoted line is `Para`, never `Unknown`, so the two are
        // independent.
        for (line, seg) in self.lines.iter_mut().zip(self.text.split('\n')) {
            if line_kind_mismatch(&line.kind, seg).is_some() {
                line.kind = LineKind::Para;
            }
            if let LineKind::Unknown { attrs, .. } = &mut line.kind {
                canonicalize_keys(attrs);
            }
            for c in &mut line.containers {
                if let Container::Unknown { attrs, .. } = c {
                    canonicalize_keys(attrs);
                }
            }
        }
        // Islands: canonicalize props key order. A table island's cells carry
        // inline `{text, marks}`; repair its shape (pad the header/rows/aligns to
        // one column count, rewrite any cell `\n` to a space) and canonicalize
        // each cell's marks (sort, union, drop zero-width) first so equal cells
        // serialize to equal bytes and `validate` holds — the props are
        // otherwise opaque here.
        for island in &mut self.islands {
            crate::island::normalize_island_structure(island);
            canonicalize_keys(&mut island.props);
        }
        for mark in &mut self.marks {
            if let MarkKind::Unknown { attrs, .. } = &mut mark.kind {
                canonicalize_keys(attrs);
            }
        }
        // A formatting mark's edges never sit on a line boundary: markdown can't
        // bold a `\n`, so two producers that disagree only about whether the
        // boundary is "inside" the mark must canonicalize to the same bounds.
        // Trim leading/trailing `\n` (interior boundaries are kept — a mark may
        // legitimately span lines). Zero-width results are dropped below.
        // Skip the full-text char collection when nothing needs trimming.
        if self.marks.iter().any(|m| m.kind.is_formatting()) {
            let chars: Vec<char> = self.text.chars().collect();
            for m in &mut self.marks {
                if m.kind.is_formatting() {
                    while m.start < m.end && chars.get(m.start) == Some(&'\n') {
                        m.start += 1;
                    }
                    while m.end > m.start && chars.get(m.end - 1) == Some(&'\n') {
                        m.end -= 1;
                    }
                }
            }
        }
        self.marks = normalize_marks(std::mem::take(&mut self.marks));
    }

    /// Mark `type` names the projection reserves; an [`MarkKind::Unknown`] may
    /// not reuse one (its serialization would parse back as the built-in,
    /// silently dropping its attrs — non-injective).
    ///
    /// Two enforcement points, on two lanes. [`Content::validate`] catches an
    /// in-process Rust construction. The wire never reaches it: a decoder resolves
    /// the built-in name before the `Unknown` fallthrough, so a reserved tag
    /// *becomes* the built-in rather than arriving as an `Unknown`. The authored
    /// lane therefore rejects the shape up front
    /// ([`serial::mark_from_op_value`](crate::serial::mark_from_op_value), its two
    /// block-axis twins, and
    /// [`serial::from_authored_value`](crate::serial::from_authored_value)), while
    /// storage decode stays lenient by design — see there.
    ///
    /// This list and its two siblings are re-spelled by hand on the TypeScript
    /// surface: the unions in `crates/bindings/wasm/src/engine.rs` and the
    /// `isUnknown*` guards' tables in
    /// `crates/bindings/wasm/runtime/runtime.js`. Both are pinned to these
    /// constants by `crates/bindings/wasm/tests/known_names_drift.rs`.
    pub const RESERVED_MARK_TYPES: [&'static str; 7] = [
        "strong",
        "emph",
        "underline",
        "strike",
        "code",
        "link",
        "anchor",
    ];

    /// Line `kind` names the projection reserves — the [`LineKind`] twin of
    /// [`RESERVED_MARK_TYPES`](Self::RESERVED_MARK_TYPES), for the same
    /// injectivity reason.
    pub const RESERVED_LINE_KINDS: [&'static str; 5] =
        ["para", "heading", "code", "island", "rule"];

    /// Container names the projection reserves — the [`Container`] twin of
    /// [`RESERVED_MARK_TYPES`](Self::RESERVED_MARK_TYPES).
    pub const RESERVED_CONTAINERS: [&'static str; 2] = ["list_item", "quote"];

    /// Check every invariant. `Ok(())` on a well-formed content. Import
    /// guarantees this; a hand-built content should be run through it in tests.
    pub fn validate(&self) -> Result<(), Invariant> {
        let mut slots = 0usize;
        for c in self.text.chars() {
            if c == '\r' {
                return Err(Invariant::CarriageReturn);
            }
            if is_bidi_char(c) {
                return Err(Invariant::BidiControl(c));
            }
            if c == ISLAND_SLOT {
                slots += 1;
            }
        }
        if slots != self.islands.len() {
            return Err(Invariant::IslandSlotMismatch {
                slots,
                islands: self.islands.len(),
            });
        }
        let segments = self.segment_count();
        if self.lines.len() != segments {
            return Err(Invariant::LineCountMismatch {
                lines: self.lines.len(),
                segments,
            });
        }
        if self.lines.first().is_some_and(|l| l.continues) {
            return Err(Invariant::FirstLineContinues);
        }
        let len = self.len_usv();
        let chars: Vec<char> = self.text.chars().collect();
        // Prose anchor ids: unique, non-empty, caller-supplied opaque handles
        // (`DOCUMENT_STORAGE.md` § Anchor-id identity). Uniqueness — the same
        // invariant the island loop enforces below — is what `RemoveAnchor`
        // presumes; scope is prose marks, cell anchors excluded by construction.
        let mut seen_anchor_ids = std::collections::HashSet::new();
        for m in &self.marks {
            if m.start > m.end || m.end > len {
                return Err(Invariant::MarkOutOfRange {
                    start: m.start,
                    end: m.end,
                    len,
                });
            }
            if m.start == m.end && m.kind.is_formatting() {
                return Err(Invariant::ZeroWidthFormatting { at: m.start });
            }
            if m.kind.is_formatting() {
                if chars.get(m.start) == Some(&'\n') {
                    return Err(Invariant::MarkEdgeOnNewline { at: m.start });
                }
                if m.end > m.start && chars.get(m.end - 1) == Some(&'\n') {
                    return Err(Invariant::MarkEdgeOnNewline { at: m.end - 1 });
                }
            }
            match &m.kind {
                MarkKind::Unknown { tag, attrs } => {
                    if Self::RESERVED_MARK_TYPES.contains(&tag.as_str()) {
                        return Err(Invariant::ReservedUnknownTag(tag.clone()));
                    }
                    check_json_depth(attrs, "mark attrs")?;
                }
                MarkKind::Anchor { id } => {
                    if id.is_empty() || !seen_anchor_ids.insert(id.as_str()) {
                        return Err(Invariant::AnchorIdCollision { id: id.clone() });
                    }
                }
                _ => {}
            }
        }
        // One pass over the lines against their own text segment. `lines.len()`
        // already equals the segment count, so the zip is total.
        for (i, (line, seg)) in self.lines.iter().zip(self.text.split('\n')).enumerate() {
            match &line.kind {
                LineKind::Heading { level } if !(1..=6).contains(level) => {
                    return Err(Invariant::BadHeadingLevel(*level));
                }
                // An unknown role may not reuse a built-in `kind` name: it would
                // serialize as the built-in and parse back as one, dropping its
                // attrs — the mark-side rule, one axis over.
                LineKind::Unknown { tag, attrs } => {
                    if Self::RESERVED_LINE_KINDS.contains(&tag.as_str()) {
                        return Err(Invariant::ReservedUnknownLineKind(tag.clone()));
                    }
                    check_json_depth(attrs, "line attrs")?;
                }
                _ => {}
            }
            for c in &line.containers {
                if let Container::Unknown { tag, attrs } = c {
                    if Self::RESERVED_CONTAINERS.contains(&tag.as_str()) {
                        return Err(Invariant::ReservedUnknownContainer(tag.clone()));
                    }
                    check_json_depth(attrs, "container attrs")?;
                }
            }
            if let Some(mismatch) = line_kind_mismatch(&line.kind, seg) {
                return Err(Invariant::LineKindMismatch { line: i, mismatch });
            }
            if line.containers.len() > crate::MAX_NESTING_DEPTH {
                return Err(Invariant::NestingTooDeep {
                    line: i,
                    depth: line.containers.len(),
                    max: crate::MAX_NESTING_DEPTH,
                });
            }
        }
        // Table-cell marks: the prose range/zero-width/reserved-tag rules again,
        // but each mark is bounded by its own cell's text length (in USV). Cells
        // hold no `\n`, so the edge-on-newline rule does not apply.
        let mut seen_ids = std::collections::HashSet::with_capacity(self.islands.len());
        for island in &self.islands {
            // Ids are deterministic, session-stable identities (hash input), so
            // two islands may not share one. Uniqueness — not `id == isl-{i}` —
            // is the invariant: edits keep an island's id across renumbers.
            if !seen_ids.insert(island.id.as_str()) {
                return Err(Invariant::IslandIdCollision {
                    id: island.id.clone(),
                });
            }
            // Payload depth before any pass that walks `props`. A cell's own
            // `attrs` is a subtree of `props`, so one check bounds the cell marks
            // the loop below reads as well.
            check_json_depth(&island.props, "island props")?;
            // Structural shape (table column/row/aligns consistency, `\n`-free
            // cells) before the per-cell mark ranges — a ragged island is
            // ill-formed regardless of its marks.
            if let Some(e) = crate::island::island_shape_error(island) {
                return Err(e);
            }
            for (text, marks) in crate::island::island_cell_marks(island) {
                let clen = text.chars().count();
                for m in &marks {
                    if m.start > m.end || m.end > clen {
                        return Err(Invariant::MarkOutOfRange {
                            start: m.start,
                            end: m.end,
                            len: clen,
                        });
                    }
                    if m.start == m.end && m.kind.is_formatting() {
                        return Err(Invariant::ZeroWidthFormatting { at: m.start });
                    }
                    if let MarkKind::Unknown { tag, .. } = &m.kind {
                        if Self::RESERVED_MARK_TYPES.contains(&tag.as_str()) {
                            return Err(Invariant::ReservedUnknownTag(tag.clone()));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Apply the three Spike-A rules and the canonical sort to a flat mark list.
///
/// 1. Same-kind formatting marks union when adjacent *or* overlapping.
/// 2. Different-kind marks overlap freely (never split into runs).
/// 3. Identity (and unknown) marks never merge.
///
/// Zero-width formatting marks are dropped (no-ops); zero-width anchors survive.
pub(crate) fn normalize_marks(marks: Vec<Mark>) -> Vec<Mark> {
    use std::collections::BTreeMap;

    // Partition: formatting marks group by (ord, attrs_key) for union; identity
    // and unknown pass through untouched (but zero-width formatting is dropped).
    let mut groups: BTreeMap<(u8, String), Vec<(Usv, Usv)>> = BTreeMap::new();
    let mut kind_of: BTreeMap<(u8, String), MarkKind> = BTreeMap::new();
    let mut passthrough: Vec<Mark> = Vec::new();

    for m in marks {
        if m.kind.is_formatting() {
            if m.start >= m.end {
                continue; // drop zero-width / inverted formatting
            }
            let key = (m.kind.ord(), m.kind.attrs_key());
            kind_of.entry(key.clone()).or_insert_with(|| m.kind.clone());
            groups.entry(key).or_default().push((m.start, m.end));
        } else {
            passthrough.push(m);
        }
    }

    let mut out: Vec<Mark> = Vec::new();
    for (key, mut ranges) in groups {
        ranges.sort_unstable();
        let kind = kind_of.remove(&key).expect("kind recorded with group");
        let mut cur = ranges[0];
        for &(s, e) in &ranges[1..] {
            if s <= cur.1 {
                // adjacent (s == cur.1) or overlapping — union
                cur.1 = cur.1.max(e);
            } else {
                out.push(Mark {
                    start: cur.0,
                    end: cur.1,
                    kind: kind.clone(),
                });
                cur = (s, e);
            }
        }
        out.push(Mark {
            start: cur.0,
            end: cur.1,
            kind,
        });
    }
    out.extend(passthrough);

    // Canonical sort: (start, end, kind-ord, attrs). Key cached per mark so
    // `attrs_key`'s allocation runs once each, not once per comparison.
    out.sort_by_cached_key(|m| (m.start, m.end, m.kind.ord(), m.kind.attrs_key()));
    // Drop byte-identical duplicates. Identity/unknown handles never *merge*
    // (Spike-A rule 3), but two marks equal in range, kind, and attrs are the
    // same handle recorded twice — redundant bytes, not two handles. The sort
    // above makes any such pair adjacent, so `dedup` (structural `PartialEq`,
    // order-independent for `Unknown` attrs under `preserve_order`) removes it.
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(start: Usv, end: Usv, kind: MarkKind) -> Mark {
        Mark { start, end, kind }
    }


    #[test]
    fn is_blank_tracks_whitespace_and_islands() {
        assert!(Content::empty().is_blank());
        let mut ws = Content::empty();
        ws.text = "  \n\t ".to_string();
        ws.lines = vec![
            Line {
                kind: LineKind::Para,
                containers: Vec::new(),
                continues: false,
            },
            Line {
                kind: LineKind::Para,
                containers: Vec::new(),
                continues: false,
            },
        ];
        assert!(ws.is_blank(), "whitespace-only text is blank");

        let mut has_text = Content::empty();
        has_text.text = "x".to_string();
        assert!(!has_text.is_blank());

        // An island slot is not whitespace, so an island-bearing content is
        // never blank even with no other text.
        let mut island_only = Content::empty();
        island_only.text = ISLAND_SLOT.to_string();
        assert!(!island_only.is_blank());
    }

    /// A single-line content over `text` tagged `kind` — the shape a `SetKind`
    /// or a splice can leave behind.
    fn tagged(text: &str, kind: LineKind) -> Content {
        Content {
            text: text.to_string(),
            lines: vec![Line {
                kind,
                containers: Vec::new(),
                continues: false,
            }],
            marks: Vec::new(),
            islands: Vec::new(),
        }
    }

    /// Issue #1050: a line kind that contradicts the line's text is refused.
    /// Export trusts the kind and never re-reads the segment, so `Island` over
    /// prose projects to the island alone and `Rule` over prose to `---` — the
    /// text silently gone.
    #[test]
    fn line_kind_must_agree_with_line_text() {
        assert_eq!(
            tagged("hello world", LineKind::Island).validate(),
            Err(Invariant::LineKindMismatch {
                line: 0,
                mismatch: LineKindMismatch::IslandNotOneSlot
            })
        );
        assert_eq!(
            tagged("", LineKind::Island).validate(),
            Err(Invariant::LineKindMismatch {
                line: 0,
                mismatch: LineKindMismatch::IslandNotOneSlot
            })
        );
        assert_eq!(
            tagged("important text", LineKind::Rule).validate(),
            Err(Invariant::LineKindMismatch {
                line: 0,
                mismatch: LineKindMismatch::RuleNotEmpty
            })
        );
        // `Para`/`Heading` carry slots — an inline image is a slot in prose —
        // so only a fence, whose text is emitted verbatim, refuses one.
        let mut code = tagged(&format!("a{ISLAND_SLOT}b"), LineKind::Code { lang: None });
        code.islands = vec![Island {
            id: "isl-0".into(),
            island_type: "image".into(),
            props: serde_json::json!({"alt": "x", "url": "y.png"}),
            loss: Loss::Lossless,
        }];
        assert_eq!(
            code.validate(),
            Err(Invariant::LineKindMismatch {
                line: 0,
                mismatch: LineKindMismatch::CodeHasSlot
            })
        );
        let mut para = code.clone();
        para.lines[0].kind = LineKind::Para;
        assert_eq!(para.validate(), Ok(()));
        let mut heading = code.clone();
        heading.lines[0].kind = LineKind::Heading { level: 1 };
        assert_eq!(heading.validate(), Ok(()));
        // The kinds that name their content, holding it.
        assert_eq!(tagged("", LineKind::Rule).validate(), Ok(()));
    }

    /// Issue #1050: a splice writes text, never kinds, so it can strand an
    /// `Island` line over prose. `normalize` demotes the stranded kind to `Para`
    /// rather than let export drop the text — the repair-side twin of the
    /// invariant, and what keeps every op path's terminal normalize total.
    #[test]
    fn normalize_demotes_a_stranded_line_kind() {
        let mut rt = tagged("typed into a table line", LineKind::Island);
        rt.normalize();
        assert_eq!(rt.lines[0].kind, LineKind::Para);
        assert_eq!(rt.validate(), Ok(()));
        let mut rt = tagged("text on a rule line", LineKind::Rule);
        rt.normalize();
        assert_eq!(rt.lines[0].kind, LineKind::Para);
        // A well-formed island line is left alone.
        let mut rt = tagged(&ISLAND_SLOT.to_string(), LineKind::Island);
        rt.islands = vec![Island {
            id: "isl-0".into(),
            island_type: "image".into(),
            props: serde_json::json!({"alt": "x", "url": "y.png"}),
            loss: Loss::Lossless,
        }];
        rt.normalize();
        assert_eq!(rt.lines[0].kind, LineKind::Island);
        assert_eq!(rt.validate(), Ok(()));
    }

    /// Issue #1051: container nesting is capped at the same depth import
    /// enforces, so a content that never went through import cannot reach the
    /// emitters — which recurse one frame per container — with an unbounded path.
    #[test]
    fn container_nesting_is_capped() {
        let mut rt = tagged("hi", LineKind::Para);
        rt.lines[0].containers = vec![Container::Quote; crate::MAX_NESTING_DEPTH];
        assert_eq!(rt.validate(), Ok(()));
        rt.lines[0].containers.push(Container::Quote);
        assert_eq!(
            rt.validate(),
            Err(Invariant::NestingTooDeep {
                line: 0,
                depth: crate::MAX_NESTING_DEPTH + 1,
                max: crate::MAX_NESTING_DEPTH,
            })
        );
    }

    /// Issue #1093: [`container_nesting_is_capped`] on the payload axis. The
    /// decoders refuse an over-deep bag at the wire; this is the same cap for
    /// content that never went through one, since the recursive consumers spend
    /// a frame per level whatever built the tree.
    #[test]
    fn json_payload_depth_is_capped() {
        let nested = |depth: usize| {
            let mut v = JsonValue::Null;
            for _ in 0..depth {
                v = JsonValue::Array(vec![v]);
            }
            v
        };
        let too_deep = |what: &'static str| {
            Err(Invariant::JsonTooDeep {
                what,
                max: crate::MAX_JSON_DEPTH,
            })
        };

        let mut rt = tagged("hi", LineKind::Para);
        rt.lines[0].kind = LineKind::Unknown {
            tag: "callout".into(),
            attrs: nested(crate::MAX_JSON_DEPTH),
        };
        assert_eq!(rt.validate(), Ok(()));
        rt.lines[0].kind = LineKind::Unknown {
            tag: "callout".into(),
            attrs: nested(crate::MAX_JSON_DEPTH + 1),
        };
        assert_eq!(rt.validate(), too_deep("line attrs"));

        let mut rt = tagged("hi", LineKind::Para);
        rt.lines[0].containers = vec![Container::Unknown {
            tag: "indent".into(),
            attrs: nested(crate::MAX_JSON_DEPTH + 1),
        }];
        assert_eq!(rt.validate(), too_deep("container attrs"));

        let mut rt = tagged("hi", LineKind::Para);
        rt.marks = vec![Mark {
            start: 0,
            end: 2,
            kind: MarkKind::Unknown {
                tag: "sparkle".into(),
                attrs: nested(crate::MAX_JSON_DEPTH + 1),
            },
        }];
        assert_eq!(rt.validate(), too_deep("mark attrs"));

        let mut rt = tagged("\u{fffc}", LineKind::Island);
        rt.islands = vec![Island {
            id: "i1".into(),
            island_type: "widget".into(),
            props: nested(crate::MAX_JSON_DEPTH + 1),
            loss: Loss::Lossless,
        }];
        assert_eq!(rt.validate(), too_deep("island props"));
    }

    #[test]
    fn same_kind_adjacent_unions() {
        // [0,3) strong + [3,6) strong -> [0,6) strong (rule 1, adjacency).
        let got = normalize_marks(vec![f(3, 6, MarkKind::Strong), f(0, 3, MarkKind::Strong)]);
        assert_eq!(got, vec![f(0, 6, MarkKind::Strong)]);
    }

    #[test]
    fn same_kind_overlapping_unions() {
        let got = normalize_marks(vec![f(0, 4, MarkKind::Emph), f(2, 7, MarkKind::Emph)]);
        assert_eq!(got, vec![f(0, 7, MarkKind::Emph)]);
    }

    #[test]
    fn different_kinds_overlap_freely() {
        // Strong and emph over overlapping ranges stay two marks (rule 2).
        let got = normalize_marks(vec![f(0, 5, MarkKind::Strong), f(2, 7, MarkKind::Emph)]);
        assert_eq!(
            got,
            vec![f(0, 5, MarkKind::Strong), f(2, 7, MarkKind::Emph)]
        );
    }

    #[test]
    fn links_union_only_at_same_url() {
        let a = MarkKind::Link { url: "a".into() };
        let b = MarkKind::Link { url: "b".into() };
        // Same url adjacent -> union; different url -> distinct.
        let got = normalize_marks(vec![
            f(0, 2, a.clone()),
            f(2, 4, a.clone()),
            f(4, 6, b.clone()),
        ]);
        assert_eq!(got, vec![f(0, 4, a), f(4, 6, b)]);
    }

    #[test]
    fn identity_never_merges() {
        // Two anchors over the same range are two distinct things (rule 3).
        let a = MarkKind::Anchor { id: "c1".into() };
        let b = MarkKind::Anchor { id: "c2".into() };
        let got = normalize_marks(vec![f(3, 3, a.clone()), f(3, 3, b.clone())]);
        assert_eq!(got.len(), 2);
        assert!(got.contains(&f(3, 3, a)));
        assert!(got.contains(&f(3, 3, b)));
    }

    #[test]
    fn zero_width_formatting_dropped_zero_width_anchor_kept() {
        let got = normalize_marks(vec![
            f(2, 2, MarkKind::Strong),
            f(2, 2, MarkKind::Anchor { id: "x".into() }),
        ]);
        assert_eq!(got, vec![f(2, 2, MarkKind::Anchor { id: "x".into() })]);
    }

    #[test]
    fn empty_is_valid() {
        assert_eq!(Content::empty().validate(), Ok(()));
    }

    #[test]
    fn is_inline_accepts_empty_and_single_para() {
        assert!(Content::empty().is_inline());
        assert!(crate::import::from_markdown("just one line")
            .unwrap()
            .is_inline());
        assert!(crate::import::from_markdown("a *bold* run")
            .unwrap()
            .is_inline());
    }

    #[test]
    fn is_inline_rejects_blocks_containers_and_islands() {
        // Two paragraphs → two Para lines.
        assert!(!crate::import::from_markdown("one\n\ntwo")
            .unwrap()
            .is_inline());
        // A heading is a non-Para line kind.
        assert!(!crate::import::from_markdown("# heading")
            .unwrap()
            .is_inline());
        // A list item sits in a container.
        assert!(!crate::import::from_markdown("- item").unwrap().is_inline());
    }

    #[test]
    fn validate_catches_slot_mismatch() {
        let mut rt = Content::empty();
        rt.text = "\u{FFFC}".into();
        rt.lines = vec![Line {
            kind: LineKind::Island,
            containers: vec![],
            continues: false,
        }];
        assert_eq!(
            rt.validate(),
            Err(Invariant::IslandSlotMismatch {
                slots: 1,
                islands: 0
            })
        );
    }

    #[test]
    fn validate_catches_line_count() {
        let mut rt = Content::empty();
        rt.text = "a\nb".into(); // 2 segments, but 1 line
        assert_eq!(
            rt.validate(),
            Err(Invariant::LineCountMismatch {
                lines: 1,
                segments: 2
            })
        );
    }

    #[test]
    fn normalize_is_idempotent() {
        let mut rt = Content::empty();
        rt.text = "hello world".into();
        rt.marks = vec![
            f(6, 11, MarkKind::Strong),
            f(0, 5, MarkKind::Strong),
            f(0, 5, MarkKind::Emph),
        ];
        rt.normalize();
        let once = rt.marks.clone();
        rt.normalize();
        assert_eq!(rt.marks, once);
        assert_eq!(rt.validate(), Ok(()));
    }

    /// A table cell built with un-normalized marks (reversed order, an adjacent
    /// same-kind pair, a zero-width formatting mark) canonicalizes to the same
    /// marks whatever the input order — the live-model determinism invariant.
    #[test]
    fn table_cell_marks_normalize_and_are_idempotent() {
        fn table(cell_marks: serde_json::Value) -> Content {
            let mut rt = Content::empty();
            rt.text = ISLAND_SLOT.to_string();
            rt.lines = vec![Line {
                kind: LineKind::Island,
                containers: vec![],
                continues: false,
            }];
            rt.islands = vec![Island {
                id: "i".into(),
                island_type: "table".into(),
                props: serde_json::json!({
                    "aligns": ["none"],
                    "header": [{"text": "abcd", "marks": cell_marks}],
                    "rows": [],
                }),
                loss: Loss::Lossless,
            }];
            rt
        }
        // Reversed order + adjacent same-kind pair (0..2)+(2..4) → unioned 0..4;
        // a zero-width strong at 1 → dropped.
        let mut a = table(serde_json::json!([
            {"start": 2, "end": 4, "type": "strong"},
            {"start": 1, "end": 1, "type": "strong"},
            {"start": 0, "end": 2, "type": "strong"}
        ]));
        a.normalize();
        assert_eq!(a.validate(), Ok(()));
        let cell = &a.islands[0].props["header"][0];
        assert_eq!(cell["marks"].as_array().unwrap().len(), 1);
        assert_eq!(cell["marks"][0]["start"], 0);
        assert_eq!(cell["marks"][0]["end"], 4);
        // Same content, different input order → identical canonical bytes.
        let mut b = table(serde_json::json!([
            {"start": 0, "end": 2, "type": "strong"},
            {"start": 2, "end": 4, "type": "strong"}
        ]));
        b.normalize();
        assert_eq!(a.to_canonical_json(), b.to_canonical_json());
        // Idempotent.
        let once = a.to_canonical_json();
        a.normalize();
        assert_eq!(a.to_canonical_json(), once);
    }

    /// Issue #1092: a cell is canonicalized in place, so a key this build does
    /// not recognize survives. The rule has no slack — `normalize` runs on
    /// decode, on every op apply, and on every serialize, so a cell rebuilt
    /// whole drops the key on first contact and every contact after.
    #[test]
    fn unrecognized_cell_key_survives_normalize() {
        // Two columns, one body cell: `pad_row` mints the second, so the same
        // pass exercises both a carried cell and a synthesized one.
        let mut rt = table_rt(serde_json::json!({
            "aligns": ["none", "none"],
            "header": [{"text": "h", "marks": [], "colspan": 2}, cell("h2")],
            "rows": [[cell("a")]],
        }));
        rt.normalize();
        assert_eq!(rt.islands[0].props["header"][0]["colspan"], 2);
        // A minted cell has nothing to carry.
        assert!(rt.islands[0].props["rows"][0][1].get("colspan").is_none());
        // Hash input like any other key.
        assert!(rt.to_canonical_json().contains(r#""colspan":2"#));
    }

    /// `validate` bounds a cell mark by its own cell's text length (in USV).
    #[test]
    fn validate_catches_cell_mark_out_of_range() {
        let mut rt = Content::empty();
        rt.text = ISLAND_SLOT.to_string();
        rt.lines = vec![Line {
            kind: LineKind::Island,
            containers: vec![],
            continues: false,
        }];
        rt.islands = vec![Island {
            id: "i".into(),
            island_type: "table".into(),
            props: serde_json::json!({
                "aligns": ["none"],
                // "ab" is 2 USV; a mark ending at 5 runs past the cell.
                "header": [{"text": "ab", "marks": [{"start": 0, "end": 5, "type": "strong"}]}],
                "rows": [],
            }),
            loss: Loss::Lossless,
        }];
        assert_eq!(
            rt.validate(),
            Err(Invariant::MarkOutOfRange {
                start: 0,
                end: 5,
                len: 2
            })
        );
    }

    /// A `Content` holding a single table island with the given props — the
    /// shared fixture for the table-shape invariant tests.
    fn table_rt(props: serde_json::Value) -> Content {
        let mut rt = Content::empty();
        rt.text = ISLAND_SLOT.to_string();
        rt.lines = vec![Line {
            kind: LineKind::Island,
            containers: vec![],
            continues: false,
        }];
        rt.islands = vec![Island {
            id: "i".into(),
            island_type: "table".into(),
            props,
            loss: Loss::Lossless,
        }];
        rt
    }

    fn cell(t: &str) -> serde_json::Value {
        serde_json::json!({ "text": t, "marks": [] })
    }

    /// `validate` rejects a ragged body row, an `aligns`/column mismatch, and a
    /// cell carrying a `\n` — the three table-shape invariants.
    #[test]
    fn validate_catches_table_shape() {
        // Ragged row: header has 2 columns, the row has 3.
        let rt = table_rt(serde_json::json!({
            "aligns": ["none", "none"],
            "header": [cell("a"), cell("b")],
            "rows": [[cell("1"), cell("2"), cell("3")]],
        }));
        assert_eq!(
            rt.validate(),
            Err(Invariant::TableRaggedRow {
                row: 0,
                width: 3,
                cols: 2
            })
        );

        // aligns length differs from the column count.
        let rt = table_rt(serde_json::json!({
            "aligns": ["none"],
            "header": [cell("a"), cell("b")],
            "rows": [],
        }));
        assert_eq!(
            rt.validate(),
            Err(Invariant::TableAlignsMismatch { aligns: 1, cols: 2 })
        );

        // A `\n` in a cell (flat header-then-rows index 1 = the second header cell).
        let rt = table_rt(serde_json::json!({
            "aligns": ["none", "none"],
            "header": [cell("a"), cell("b\nc")],
            "rows": [],
        }));
        assert_eq!(rt.validate(), Err(Invariant::TableCellNewline { cell: 1 }));
    }

    /// `normalize` repairs every table-shape violation — pads the header and
    /// short rows to the widest column count, syncs `aligns`, and rewrites a
    /// cell `\n` to a space — so the result validates and is idempotent. This is
    /// also the one-column-count unification: the widest row (3) drives the
    /// header width, so the markdown (header-derived) and Typst (widest-row)
    /// projections agree.
    #[test]
    fn normalize_repairs_table_shape() {
        let mut rt = table_rt(serde_json::json!({
            "aligns": ["none"],
            "header": [cell("h")],
            "rows": [
                [cell("a"), cell("b"), cell("c")],
                [cell("d\ne")],
            ],
        }));
        rt.normalize();
        assert_eq!(rt.validate(), Ok(()));

        let props = &rt.islands[0].props;
        assert_eq!(props["header"].as_array().unwrap().len(), 3);
        assert_eq!(props["aligns"].as_array().unwrap().len(), 3);
        for row in props["rows"].as_array().unwrap() {
            assert_eq!(row.as_array().unwrap().len(), 3);
        }
        // Padded aligns default to "none"; the padded cells are empty.
        assert_eq!(props["aligns"][2], serde_json::json!("none"));
        assert_eq!(props["header"][1]["text"], serde_json::json!(""));
        // The `\n` in "d\ne" became a space, preserving char count.
        assert_eq!(props["rows"][1][0]["text"], serde_json::json!("d e"));

        // Idempotent on canonical bytes.
        let once = rt.to_canonical_json();
        rt.normalize();
        assert_eq!(rt.to_canonical_json(), once);
    }

    /// An empty table (no header, no rows) is trivially well-formed: every width
    /// is zero, so no shape invariant fires and `normalize` leaves it alone.
    #[test]
    fn empty_table_is_valid() {
        let mut rt = table_rt(serde_json::json!({
            "aligns": [],
            "header": [],
            "rows": [],
        }));
        assert_eq!(rt.validate(), Ok(()));
        rt.normalize();
        assert_eq!(rt.validate(), Ok(()));
    }

    /// A non-array `header` carries no cells: `validate` rejects it and
    /// `normalize` repairs it to an empty array (a zero-column table that then
    /// validates). Issue #904.
    #[test]
    fn non_array_table_header_is_rejected_then_repaired() {
        let mut rt = table_rt(serde_json::json!({
            "header": "oops",
            "aligns": [],
            "rows": [],
        }));
        assert_eq!(rt.validate(), Err(Invariant::TableHeaderNotArray));
        rt.normalize();
        assert_eq!(rt.validate(), Ok(()));
        assert_eq!(rt.islands[0].props["header"], serde_json::json!([]));
    }

    /// Two islands sharing an `id` violate the minted-identity invariant.
    /// Import mints ids by index so never collides; a hand-built content can.
    /// Issue #903.
    #[test]
    fn duplicate_island_id_is_rejected() {
        let mut rt = Content::empty();
        rt.text = format!("{ISLAND_SLOT}\n{ISLAND_SLOT}");
        rt.lines = vec![
            Line {
                kind: LineKind::Island,
                containers: vec![],
                continues: false,
            },
            Line {
                kind: LineKind::Island,
                containers: vec![],
                continues: false,
            },
        ];
        let table = |id: &str| Island {
            id: id.into(),
            island_type: "table".into(),
            props: serde_json::json!({ "header": [cell("h")], "aligns": ["none"], "rows": [] }),
            loss: Loss::Lossless,
        };
        rt.islands = vec![table("dup"), table("dup")];
        assert_eq!(
            rt.validate(),
            Err(Invariant::IslandIdCollision { id: "dup".into() })
        );
        // Distinct ids validate.
        rt.islands = vec![table("a"), table("b")];
        assert_eq!(rt.validate(), Ok(()));
    }

    /// Two prose anchors sharing an `id` at different ranges violate the
    /// anchor-id uniqueness invariant, as does the empty id. (Byte-identical
    /// anchors `normalize` already dedupes; this is the surviving collision.)
    /// Issue #1039.
    #[test]
    fn duplicate_or_empty_anchor_id_is_rejected() {
        let mut rt = Content::empty();
        rt.text = "abcd".into();
        let anchor = |start, end, id: &str| Mark {
            start,
            end,
            kind: MarkKind::Anchor { id: id.into() },
        };
        // Same id at two ranges — `RemoveAnchor` can't disambiguate them.
        rt.marks = vec![anchor(0, 2, "x"), anchor(2, 4, "x")];
        assert_eq!(
            rt.validate(),
            Err(Invariant::AnchorIdCollision { id: "x".into() })
        );
        // Distinct ids over distinct ranges validate.
        rt.marks = vec![anchor(0, 2, "x"), anchor(2, 4, "y")];
        assert_eq!(rt.validate(), Ok(()));
        // The empty id is a degenerate handle.
        rt.marks = vec![anchor(0, 2, "")];
        assert_eq!(
            rt.validate(),
            Err(Invariant::AnchorIdCollision { id: String::new() })
        );
    }

    /// `normalize` drops a byte-identical duplicate identity mark (same range,
    /// same id) — the same handle recorded twice is redundant, not two handles.
    /// Distinct-id anchors over the same range are kept. Issue #906.
    #[test]
    fn normalize_dedupes_identical_identity_marks() {
        let mut rt = Content::empty();
        rt.text = "abcd".into();
        let anchor = |id: &str| Mark {
            start: 0,
            end: 4,
            kind: MarkKind::Anchor { id: id.into() },
        };
        rt.marks = vec![anchor("x"), anchor("x")];
        rt.normalize();
        assert_eq!(rt.marks, vec![anchor("x")]);
        // Different ids over the same range are distinct handles — both survive.
        rt.marks = vec![anchor("x"), anchor("y")];
        rt.normalize();
        assert_eq!(rt.marks.len(), 2);
    }
}
