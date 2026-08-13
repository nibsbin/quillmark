//! Recover schema-field regions from *glyph spans*: the origin every drawn
//! frame item already carries. Content fields are codegen'd as markup block
//! bindings (`#let _qm_cN = [ .. ]`) in the generated helper `lib.typ`, so
//! every glyph's span nests inside its field's byte window; the backend
//! records that window plus a per-segment source map ([`FieldWindow`]) and
//! classifies each frame item by containment, two-tier: which window, then
//! which segment. Regions key on `(window, segment)`. A scalar the plate
//! interpolates directly (`#data.subject`) has no segments; its window comes
//! from [`scalar_windows`]. Spans survive any `show`-rule content rebuild
//! because they are a property of the glyph, not a sibling element.
//!
//! **Resolution goes through the compile's own helper source.** The session
//! serves reads from its last-good compile, but a failed `apply` has already
//! written the *next* injection's helper text into the world; resolving the
//! served document's spans against that text would shift every range. Only
//! non-helper spans (plate, vendored packages: stable within a session)
//! resolve through the live world.
//!
//! **First placement only.** Each key's region is its first maximal run of
//! consecutive matching frame items. Span data cannot distinguish "package
//! chrome between two placements" from "a second placement" (both are a gap of
//! foreign spans), so later runs are not enumerated. One tolerance keeps
//! continuation pages covered: page marginals walk between one page's body and
//! the next's, so a run may resume on the immediately following page; a
//! same-page gap still ends it.
//!
//! Geometry composes the group-transform stack exactly like
//! `typst_layout::introspect::discover_frame`, transforming all four corners of
//! each item box (the stack may rotate or scale).

use std::collections::HashMap;
use std::ops::Range;

use typst::layout::{Frame, FrameItem, Point, Transform};
use typst::syntax::ast::{self, AstNode};
use typst::syntax::{DiagSpan, DiagSpanKind, FileId, LinkedNode, Source, Span, SyntaxKind};
use typst::World;
use typst_layout::PagedDocument;

use quillmark_core::{ContentHit, HitGranularity, RenderedRegion};

use crate::emit::SegmentMap;
use crate::world::QuillWorld;

/// A tracked byte window in a compiled source: the schema field whose content
/// resolves into `range` of `file`. Content fields point at their generated
/// markup block in the helper `lib.typ`; scalars at their plate expression.
#[derive(Debug, Clone)]
pub(crate) struct FieldWindow {
    pub path: String,
    pub file: FileId,
    pub range: Range<usize>,
    /// `generated` ranges index the helper `lib.typ`. Empty for a scalar/widget
    /// site, whose whole first placement is one span-less region.
    pub segments: Vec<SegmentMap>,
}

/// An axis-aligned box accumulated in page-space (top-left origin) pt.
#[derive(Clone, Copy)]
struct Aabb {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Aabb {
    fn of(corners: [Point; 4], ts: Transform) -> Self {
        let mut b = Self {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        for c in corners {
            let p = c.transform(ts);
            let (x, y) = (p.x.to_pt(), p.y.to_pt());
            b.min_x = b.min_x.min(x);
            b.min_y = b.min_y.min(y);
            b.max_x = b.max_x.max(x);
            b.max_y = b.max_y.max(y);
        }
        b
    }

    fn union(&mut self, o: Aabb) {
        self.min_x = self.min_x.min(o.min_x);
        self.min_y = self.min_y.min(o.min_y);
        self.max_x = self.max_x.max(o.max_x);
        self.max_y = self.max_y.max(o.max_y);
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        self.min_x <= x && x <= self.max_x && self.min_y <= y && y <= self.max_y
    }
}

/// Page-space (top-left origin) box → PDF-space (bottom-left origin) rect.
fn pdf_rect(b: &Aabb, page_h: f64) -> [f32; 4] {
    [
        b.min_x as f32,
        (page_h - b.max_y) as f32,
        b.max_x as f32,
        (page_h - b.min_y) as f32,
    ]
}

struct Hit {
    page: usize,
    class: HitClass,
    /// `Some` for any window-classified ink, `None` for foreign ink.
    rect: Option<Aabb>,
}

/// Where a hit falls in the flattened key space `(window, Option<segment>)`.
#[derive(Clone, Copy)]
enum HitClass {
    Boxable { key: (usize, Option<usize>) },
    /// A content window's ink between its segments (brackets, container-open
    /// syntax). Suspends a *different* window's run like foreign ink, but is a
    /// no-op while its own window's segment is the run.
    Transparent { window: usize },
    /// Ink from a detached span: Typst's synthesized text decorations (the
    /// `underline`/`strike` line, drawn as a `Shape` mid-run) and list markers.
    /// Attributable to no field, and never a run-breaker: a decoration drawn
    /// between a field's own glyphs would otherwise orphan the rest of the line.
    Anonymous,
    /// No window, but a *resolvable* span: page chrome, another field's text,
    /// vendored-package output. Breaks a run.
    Foreign,
}

impl HitClass {
    fn window(self) -> Option<usize> {
        match self {
            HitClass::Boxable { key } => Some(key.0),
            HitClass::Transparent { window } => Some(window),
            HitClass::Anonymous | HitClass::Foreign => None,
        }
    }
}

/// `(w, None)` splits by window kind: on a segment-less window it is the whole
/// placement's boxable key; on a content window it is inter-segment ink.
fn hit_class(
    resolved: Option<(usize, Option<usize>)>,
    detached: bool,
    windows: &[FieldWindow],
) -> HitClass {
    match resolved {
        None if detached => HitClass::Anonymous,
        None => HitClass::Foreign,
        Some((w, Some(s))) => HitClass::Boxable { key: (w, Some(s)) },
        Some((w, None)) if windows[w].segments.is_empty() => HitClass::Boxable { key: (w, None) },
        Some((w, None)) => HitClass::Transparent { window: w },
    }
}

/// Memoizing span → two-tier classifier: the resolve + segment search runs once
/// per distinct span, not once per glyph.
struct Classifier<'a> {
    world: &'a QuillWorld,
    helper: &'a Source,
    windows: &'a [FieldWindow],
    memo: HashMap<Span, Option<(usize, Option<usize>)>>,
}

impl<'a> Classifier<'a> {
    fn new(world: &'a QuillWorld, helper: &'a Source, windows: &'a [FieldWindow]) -> Self {
        Self {
            world,
            helper,
            windows,
            memo: HashMap::new(),
        }
    }

    /// The same unpack `WorldExt::range` performs, with the helper file routed
    /// to the served compile's snapshot instead of the world.
    fn resolve_range(&self, span: Span) -> Option<(FileId, Range<usize>)> {
        match DiagSpan::from(span).get() {
            DiagSpanKind::Detached => None,
            DiagSpanKind::Number { id, num, sub_range } => {
                let range = if id == self.helper.id() {
                    self.helper.range(num, sub_range)
                } else {
                    self.world
                        .source(id)
                        .ok()
                        .and_then(|s| s.range(num, sub_range))
                };
                range.map(|r| (id, r))
            }
            DiagSpanKind::Range { id, range } => Some((id, range)),
        }
    }

    /// Resolves to the innermost segment whose `generated` range contains the
    /// span, or `None` for inter-segment / segment-less ink.
    fn classify_seg(&mut self, span: Span) -> Option<(usize, Option<usize>)> {
        if let Some(&c) = self.memo.get(&span) {
            return c;
        }
        let c = self.resolve_range(span).and_then(|(file, range)| {
            self.windows
                .iter()
                .position(|win| {
                    win.file == file && win.range.start <= range.start && range.end <= win.range.end
                })
                .map(|i| (i, self.seg_of(i, &range)))
        });
        self.memo.insert(span, c);
        c
    }

    /// Segments are `generated`-ordered and disjoint, so the sole candidate is
    /// the last one starting at or before `range.start`.
    fn seg_of(&self, window: usize, range: &Range<usize>) -> Option<usize> {
        let segs = &self.windows[window].segments;
        let i = segs.partition_point(|s| s.generated.start <= range.start);
        (i > 0 && segs[i - 1].generated.end >= range.end).then(|| i - 1)
    }
}

fn collect_page_hits(frame: &Frame, page: usize, cls: &mut Classifier, out: &mut Vec<Hit>) {
    walk_items(frame, Transform::identity(), page, &mut |page, span, _offset, aabb| {
        let class = hit_class(cls.classify_seg(span), span.is_detached(), cls.windows);
        // Foreign ink still emits a rect-less Hit so the run machine sees the
        // full ink sequence.
        let rect = class.window().is_some().then(aabb);
        out.push(Hit { page, class, rect });
    });
}

/// Per-item callback for [`walk_items`]: `(page, span, intra-node byte offset,
/// thunk computing the page-space box on demand)`. The box is a thunk so a
/// consumer that discards foreign ink pays no box arithmetic.
type ItemVisitor<'a> = dyn FnMut(usize, Span, u16, &dyn Fn() -> Aabb) + 'a;

fn walk_items(frame: &Frame, ts: Transform, page: usize, visit: &mut ItemVisitor) {
    for (pos, item) in frame.items() {
        match item {
            FrameItem::Group(group) => {
                let ts = ts
                    .pre_concat(Transform::translate(pos.x, pos.y))
                    .pre_concat(group.transform);
                walk_items(&group.frame, ts, page, visit);
            }
            FrameItem::Text(text) => {
                let bb = text.bbox();
                let mut cursor = Point::zero();
                for glyph in &text.glyphs {
                    let advance =
                        Point::new(glyph.x_advance.at(text.size), glyph.y_advance.at(text.size));
                    let offset =
                        Point::new(glyph.x_offset.at(text.size), glyph.y_offset.at(text.size));
                    let lo = Point::new(cursor.x + offset.x, cursor.y + bb.min.y);
                    let hi = Point::new(cursor.x + offset.x + advance.x, cursor.y + bb.max.y);
                    let p = *pos;
                    visit(page, glyph.span.0, glyph.span.1, &|| item_aabb(p, lo, hi, ts));
                    cursor += advance;
                }
            }
            FrameItem::Shape(shape, span) => {
                let bb = shape.geometry.bbox(shape.stroke.as_ref());
                let p = *pos;
                visit(page, *span, 0, &|| item_aabb(p, bb.min, bb.max, ts));
            }
            FrameItem::Image(_, size, span) => {
                let sz = size.to_point();
                let p = *pos;
                visit(page, *span, 0, &|| item_aabb(p, Point::zero(), sz, ts));
            }
            _ => {}
        }
    }
}

/// All four corners transform: `ts` may rotate or scale.
fn item_aabb(pos: Point, lo: Point, hi: Point, ts: Transform) -> Aabb {
    Aabb::of(
        [
            Point::new(pos.x + lo.x, pos.y + lo.y),
            Point::new(pos.x + hi.x, pos.y + lo.y),
            Point::new(pos.x + lo.x, pos.y + hi.y),
            Point::new(pos.x + hi.x, pos.y + hi.y),
        ],
        ts,
    )
}

/// Out-of-run states only: at most one window accrues at a time, tracked by
/// [`run_scan_machine`] as a single cursor.
#[derive(Clone, Copy, PartialEq)]
enum Run {
    NotSeen,
    /// Interrupted by foreign ink; may resume on page `last_page + 1` only.
    Suspended {
        last_page: usize,
    },
    Done,
}

/// Each window's **first placement**: one [`RenderedRegion`] per page the run
/// touches, PDF bottom-left rects, sorted (page, field, window order).
pub(crate) fn scan_content_regions(
    doc: &PagedDocument,
    world: &QuillWorld,
    helper: &Source,
    windows: &[FieldWindow],
) -> Vec<RenderedRegion> {
    if windows.is_empty() {
        return Vec::new();
    }
    let mut cls = Classifier::new(world, helper, windows);
    let mut hits = Vec::new();
    for (page, p) in doc.pages().iter().enumerate() {
        collect_page_hits(&p.frame, page, &mut cls, &mut hits);
    }

    let keys = flatten_keys(windows);
    let boxes = run_scan_machine(&keys, &hits);

    let mut out: Vec<(RenderedRegion, usize)> = Vec::new();
    for (ki, &(wi, seg)) in keys.iter().enumerate() {
        let window = &windows[wi];
        let span = seg.map(|s| {
            let c = &window.segments[s].content;
            [c.start, c.end]
        });
        for (page, b) in &boxes[ki] {
            let Some(page_h) = doc.pages().get(*page).map(|p| p.frame.size().y.to_pt()) else {
                continue;
            };
            let mut region =
                RenderedRegion::new(window.path.clone(), *page, pdf_rect(b, page_h));
            region.span = span;
            out.push((region, ki));
        }
    }
    // `ki` orders window-major then segment-ascending, a stable tiebreak.
    out.sort_by(|(a, ai), (b, bi)| (a.page, &a.field, *ai).cmp(&(b.page, &b.field, *bi)));
    out.into_iter().map(|(r, _)| r).collect()
}

/// Window-major, segment-ascending: the order regions sort by.
fn flatten_keys(windows: &[FieldWindow]) -> Vec<(usize, Option<usize>)> {
    let mut keys = Vec::new();
    for (wi, w) in windows.iter().enumerate() {
        if w.segments.is_empty() {
            keys.push((wi, None));
        } else {
            keys.extend((0..w.segments.len()).map(|s| (wi, Some(s))));
        }
    }
    keys
}

/// Each key's first placement as per-page boxes, indexed parallel to `keys`.
/// `current` is the one key whose run is accruing; a boxable hit for a different
/// key (or foreign ink) suspends it, and a suspended run resumes only on the
/// immediately following page.
fn run_scan_machine(keys: &[(usize, Option<usize>)], hits: &[Hit]) -> Vec<Vec<(usize, Aabb)>> {
    let key_index: HashMap<(usize, Option<usize>), usize> =
        keys.iter().enumerate().map(|(i, k)| (*k, i)).collect();
    let mut state = vec![Run::NotSeen; keys.len()];
    let mut boxes: Vec<Vec<(usize, Aabb)>> = vec![Vec::new(); keys.len()];
    let mut current: Option<(usize, usize)> = None; // (key index, last_page)

    for hit in hits {
        match hit.class {
            HitClass::Boxable { key } => {
                let ki = key_index[&key];
                if current.map(|(c, _)| c) == Some(ki) {
                    accrue(&mut boxes[ki], hit);
                    current = Some((ki, hit.page));
                } else {
                    if let Some((c, last_page)) = current.take() {
                        state[c] = Run::Suspended { last_page };
                    }
                    match state[ki] {
                        Run::NotSeen => {
                            accrue(&mut boxes[ki], hit);
                            current = Some((ki, hit.page));
                        }
                        Run::Suspended { last_page } if hit.page == last_page + 1 => {
                            accrue(&mut boxes[ki], hit);
                            current = Some((ki, hit.page));
                        }
                        Run::Suspended { .. } => state[ki] = Run::Done,
                        Run::Done => {}
                    }
                }
            }
            HitClass::Transparent { window } => match current {
                // Transparent only while this field's own segment is the run,
                // else interleaved placements merge into one lying box.
                Some((c, _)) if keys[c].0 == window => {}
                _ => {
                    if let Some((c, last_page)) = current.take() {
                        state[c] = Run::Suspended { last_page };
                    }
                }
            },
            HitClass::Anonymous => {}
            HitClass::Foreign => {
                if let Some((c, last_page)) = current.take() {
                    state[c] = Run::Suspended { last_page };
                }
            }
        }
    }
    boxes
}

/// Pages are nondecreasing in walk order, so a page transition opens a new box.
fn accrue(boxes: &mut Vec<(usize, Aabb)>, hit: &Hit) {
    let rect = hit.rect.expect("classified hits carry a box");
    match boxes.last_mut() {
        Some((page, b)) if *page == hit.page => b.union(rect),
        _ => boxes.push((hit.page, rect)),
    }
}

/// The schema field under a point (`x`/`y` in PDF bottom-left points). Unlike
/// [`scan_content_regions`] every placement answers, not just the first. Among
/// tracked ink the later-painted item wins; untracked ink never occludes.
pub(crate) fn field_at(
    doc: &PagedDocument,
    world: &QuillWorld,
    helper: &Source,
    windows: &[FieldWindow],
    page: usize,
    x: f32,
    y: f32,
) -> Option<String> {
    if windows.is_empty() {
        return None;
    }
    let frame = &doc.pages().get(page)?.frame;
    let page_h = frame.size().y.to_pt();
    let (x, y) = (x as f64, page_h - y as f64);

    let mut cls = Classifier::new(world, helper, windows);
    let mut hits = Vec::new();
    collect_page_hits(frame, page, &mut cls, &mut hits);

    hits.iter()
        .rev()
        .find(|h| h.rect.is_some_and(|r| r.contains(x, y)))
        .and_then(|h| h.class.window())
        .map(|w| windows[w].path.clone())
}

/// The finer counterpart to [`Hit`], carrying the node range and intra-node
/// offset that [`position_at`]/[`locate`] need and a region scan discards.
struct GlyphHit {
    page: usize,
    rect: Aabb,
    node: Range<usize>,
    /// Typst types the intra-node span offset as `u16`, so a text node wider
    /// than 64 KiB saturates it: the caret floors to the cluster boundary
    /// instead of the exact glyph. No emitted node approaches that bound.
    offset: u16,
    window: usize,
    seg: Option<usize>,
}

/// The content-position twin of [`collect_page_hits`]. Foreign and unresolvable
/// ink is skipped: it has no content address.
///
/// `only` restricts emission to a single `(window, seg)`: the caret path knows
/// its target segment up front, so it skips the box arithmetic and the
/// `GlyphHit` allocation for every other glyph in the document. `None` emits all
/// classified+resolved ink (the point-hit path needs the full set).
fn walk_glyphs(
    frame: &Frame,
    page: usize,
    cls: &mut Classifier,
    only: Option<(usize, Option<usize>)>,
    out: &mut Vec<GlyphHit>,
) {
    walk_items(frame, Transform::identity(), page, &mut |page, span, offset, aabb| {
        let Some((w, seg)) = cls.classify_seg(span) else {
            return;
        };
        if only.is_some_and(|t| t != (w, seg)) {
            return;
        }
        if let Some((_, node)) = cls.resolve_range(span) {
            out.push(GlyphHit {
                page,
                rect: aabb(),
                node,
                offset,
                window: w,
                seg,
            });
        }
    });
}

/// A point (PDF bottom-left points) → content position in a content field.
/// Degrades to the segment's content start when the resolved node nests inside
/// no single run (a multi-line `#raw` block, or structural ink).
pub(crate) fn position_at(
    doc: &PagedDocument,
    world: &QuillWorld,
    helper: &Source,
    windows: &[FieldWindow],
    page: usize,
    x: f32,
    y: f32,
) -> Option<ContentHit> {
    if windows.is_empty() {
        return None;
    }
    let frame = &doc.pages().get(page)?.frame;
    let page_h = frame.size().y.to_pt();
    let (px, py) = (x as f64, page_h - y as f64);

    let mut cls = Classifier::new(world, helper, windows);
    let mut hits = Vec::new();
    walk_glyphs(frame, page, &mut cls, None, &mut hits);

    // Later-painted wins; ink with no segment has no content position.
    let hit = hits
        .iter()
        .rev()
        .find(|g| g.seg.is_some() && g.rect.contains(px, py))?;
    let window = &windows[hit.window];
    let segmap = &window.segments[hit.seg?];
    let (pos, granularity) = invert_hit(helper, segmap, &hit.node, hit.offset);
    Some(ContentHit::new(window.path.clone(), pos).with_granularity(granularity))
}

/// The content USV offset a glyph's generated position maps to. With no owning
/// run (the `#raw` multi-line case, where every line shares one node wider than
/// any run) the safe degrade is the segment's content start, not a node-start
/// computation that could land outside every line's own text.
fn invert_hit(
    helper: &Source,
    segmap: &SegmentMap,
    node: &Range<usize>,
    offset: u16,
) -> (usize, HitGranularity) {
    let Some((content_r, gen_r, ctx)) = segmap
        .runs
        .iter()
        .find(|(_, g, _)| g.start <= node.start && node.end <= g.end)
    else {
        return (segmap.content.start, HitGranularity::Segment);
    };
    // `node.start + glyph.span.1` is the exact generated byte for markup text;
    // clamp inside the run against a boundary-hugging offset.
    let abs = (node.start + offset as usize)
        .min(gen_r.end.saturating_sub(1))
        .max(gen_r.start);
    let gen_text = &helper.text()[gen_r.clone()];
    let pos = content_r.start + crate::emit::invert_gen_offset(gen_text, *ctx, abs - gen_r.start);
    (pos, HitGranularity::Cluster)
}

/// A content position → caret rect: the box of the frame glyph whose resolved
/// node covers `pos`, with `span` collapsed to `[pos, pos]`.
pub(crate) fn locate(
    doc: &PagedDocument,
    world: &QuillWorld,
    helper: &Source,
    windows: &[FieldWindow],
    field: &str,
    pos: usize,
) -> Option<RenderedRegion> {
    if windows.is_empty() {
        return None;
    }
    let (wi, window) = windows
        .iter()
        .enumerate()
        .find(|(_, w)| w.path == field && !w.segments.is_empty())?;
    let seg_idx = window
        .segments
        .iter()
        .position(|s| s.content.start <= pos && pos <= s.content.end)?;
    let target_gen = forward_pos(helper, &window.segments[seg_idx], pos);

    let mut cls = Classifier::new(world, helper, windows);
    let mut hits = Vec::new();
    for (page, p) in doc.pages().iter().enumerate() {
        walk_glyphs(&p.frame, page, &mut cls, Some((wi, Some(seg_idx))), &mut hits);
    }

    // A covering glyph always beats a non-covering one, so a caret near a run
    // edge still resolves; `min_by_key` keeps the first on ties.
    let g = hits
        .iter()
        .min_by_key(|g| {
            let covers = g.node.start <= target_gen && target_gen < g.node.end;
            let caret = g.node.start + g.offset as usize;
            (
                !covers,
                caret > target_gen,
                (caret as isize - target_gen as isize).unsigned_abs(),
            )
        })?;
    let page_h = doc.pages().get(g.page)?.frame.size().y.to_pt();
    Some(
        RenderedRegion::new(field.to_string(), g.page, pdf_rect(&g.rect, page_h))
            .with_span([pos, pos]),
    )
}

/// A position in a structural gap (or at the segment edge) falls back to the
/// segment's generated window start.
fn forward_pos(helper: &Source, segmap: &SegmentMap, pos: usize) -> usize {
    match segmap
        .runs
        .iter()
        .find(|(c, _, _)| c.start <= pos && pos < c.end)
    {
        Some((content_r, gen_r, ctx)) => {
            let gen_text = &helper.text()[gen_r.clone()];
            gen_r.start + crate::emit::forward_content_offset(gen_text, *ctx, pos - content_r.start)
        }
        None => segmap.generated.start,
    }
}

/// Byte windows for the plate's direct scalar references. Two windows per
/// reference site where they differ:
///
/// - the **chain** window: the `data.<field>` access widened to the outermost
///   postfix chain it heads (`data.refs.at(0)`), matching ink whose span is the
///   reference expression itself; and
/// - the **enclosing-expression** window, widened through surrounding call
///   arguments and operators (`#upper(data.subject)`). Emitted only when exactly
///   one reference sits inside it: `data.a + data.b` has no single owner.
///
/// Chain windows sort first, so ink resolving to the reference itself is never
/// claimed by a wider window. A value laundered through `#let s = data.x` is not
/// chased: it carries the binding's span.
pub(crate) fn scalar_windows(source: &Source, fields: &[String]) -> Vec<(String, Range<usize>)> {
    let mut anchors: Vec<(String, Range<usize>, Range<usize>)> = Vec::new();
    collect_anchors(&LinkedNode::new(source.root()), fields, &mut anchors);

    let mut out: Vec<(String, Range<usize>)> = anchors
        .iter()
        .map(|(path, chain, _)| (path.clone(), chain.clone()))
        .collect();
    for (path, chain, wide) in &anchors {
        if wide == chain {
            continue;
        }
        let inside = anchors
            .iter()
            .filter(|(_, c, _)| wide.start <= c.start && c.end <= wide.end)
            .count();
        if inside == 1 {
            out.push((path.clone(), wide.clone()));
        }
    }
    out
}

/// Recursion continues into matched subtrees: a reference nested in another
/// chain's arguments is its own site.
fn collect_anchors(
    node: &LinkedNode,
    fields: &[String],
    out: &mut Vec<(String, Range<usize>, Range<usize>)>,
) {
    if let Some((path, anchor)) = data_access(node, fields) {
        // Chain: the outermost postfix chain headed by this access.
        let mut chain = anchor.clone();
        while let Some(parent) = chain.parent() {
            match parent.kind() {
                SyntaxKind::FieldAccess | SyntaxKind::FuncCall => chain = parent.clone(),
                _ => break,
            }
        }
        // Enclosing expression: widened through argument and operator
        // context, stopping at any statement/markup boundary.
        let mut wide = chain.clone();
        while let Some(parent) = wide.parent() {
            match parent.kind() {
                SyntaxKind::FieldAccess
                | SyntaxKind::FuncCall
                | SyntaxKind::Args
                | SyntaxKind::Named
                | SyntaxKind::Spread
                | SyntaxKind::Parenthesized
                | SyntaxKind::Unary
                | SyntaxKind::Binary => wide = parent.clone(),
                _ => break,
            }
        }
        out.push((path, chain.range(), wide.range()));
    }
    for child in node.children() {
        collect_anchors(&child, fields, out);
    }
}

/// If `node` is a `data.<field>` access or a `data.at("<field>")` call head
/// with a declared field, its schema path and the node to widen from.
fn data_access<'a>(node: &LinkedNode<'a>, fields: &[String]) -> Option<(String, LinkedNode<'a>)> {
    if node.kind() != SyntaxKind::FieldAccess {
        return None;
    }
    let access = node.cast::<ast::FieldAccess>()?;
    let ast::Expr::Ident(target) = access.target() else {
        return None;
    };
    if target.as_str() != "data" {
        return None;
    }
    let field = access.field();
    if fields.iter().any(|f| f == field.as_str()) {
        return Some((field.as_str().to_string(), node.clone()));
    }
    // `data.at("field")`: the parent call carries the field name as its first
    // positional string argument.
    if field.as_str() == "at" {
        let parent = node.parent()?;
        let call = parent.cast::<ast::FuncCall>()?;
        let ast::Expr::FieldAccess(callee) = call.callee() else {
            return None;
        };
        if callee.to_untyped() != node.get() {
            return None;
        }
        let first = call.args().items().find_map(|arg| match arg {
            ast::Arg::Pos(ast::Expr::Str(s)) => Some(s.get().to_string()),
            _ => None,
        })?;
        if fields.contains(&first) {
            return Some((first, parent.clone()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::compile_document;
    use crate::world::QuillWorld;
    use quillmark_core::{FileTreeNode, Quill};
    use std::collections::HashMap as Map;
    use typst::World;

    fn quill(yaml: &str, plate: &str) -> Quill {
        let mut files = Map::new();
        files.insert(
            "Quill.yaml".to_string(),
            FileTreeNode::File {
                contents: yaml.as_bytes().to_vec(),
            },
        );
        files.insert(
            "plate.typ".to_string(),
            FileTreeNode::File {
                contents: plate.as_bytes().to_vec(),
            },
        );
        Quill::from_tree(FileTreeNode::Directory { files }).expect("load quill")
    }

    /// The premise the whole mechanism stands on.
    #[test]
    fn block_output_spans_resolve_into_the_helper_file() {
        const YAML: &str = r#"
quill:
  name: span_probe
  version: 0.1.0
  backend: typst
  description: helper-file span resolution probe
typst:
  plate_file: plate.typ
main:
  fields:
    intro:
      type: richtext
      description: a probe field
"#;
        const PLATE: &str = r#"
#import "@local/quillmark-helper:0.1.0": data
#set page(width: 400pt, height: 400pt, margin: 40pt)
#data.intro
"#;
        let q = quill(YAML, PLATE);
        let plate = crate::read_plate(&q).expect("plate");
        let schema = quillmark_core::quill::build_transform_schema(q.config());
        let meta = crate::SchemaMeta::from_schema_json(schema.as_json());
        let rt = quillmark_content::import::from_markdown("A probe paragraph, PROBETOKEN.")
            .expect("import");
        let data =
            serde_json::json!({ "intro": quillmark_content::serial::to_canonical_value(&rt) });
        let transformed = crate::transformed_data(&meta, &data).expect("transform");
        let mut world = QuillWorld::new(&q, &plate).expect("world");
        let windows = world
            .inject_helper_package(transformed.as_ref(), &meta)
            .expect("inject");
        let (doc, _) = compile_document(&world).expect("compile");
        let helper = world
            .source(QuillWorld::helper_fid("lib.typ"))
            .expect("helper source");

        let intro_idx = windows
            .iter()
            .position(|w| w.path == "intro")
            .expect("intro window");
        let mut cls = Classifier::new(&world, &helper, &windows);
        let mut hits = Vec::new();
        for (page, p) in doc.pages().iter().enumerate() {
            collect_page_hits(&p.frame, page, &mut cls, &mut hits);
        }
        assert!(
            hits.iter().any(|h| h.class.window() == Some(intro_idx)),
            "block output glyphs must classify into the helper file's recorded window {:?}",
            windows[intro_idx].range
        );
    }

    /// `underline`/`strike` draw a decoration `Shape` with a detached span
    /// between the decorated glyphs and the trailing plain run; classifying it
    /// `Foreign` would truncate the region at the mark's start.
    #[test]
    fn decoration_marks_do_not_truncate_the_region() {
        use quillmark_content::model::{Line, LineKind, Mark, MarkKind, Content};
        const YAML: &str = r#"
quill:
  name: deco_probe
  version: 0.1.0
  backend: typst
  description: underline and strike region truncation probe
typst:
  plate_file: plate.typ
main:
  fields:
    body:
      type: richtext
      description: one paragraph with one decorated run
"#;
        const PLATE: &str = r#"
#import "@local/quillmark-helper:0.1.0": data
#set page(width: 400pt, height: 400pt, margin: 40pt)
#data.body
"#;
        const TEXT: &str = "Start uline and then a long trailing plain run of text.";
        let region_width = |kind: MarkKind| -> f32 {
            let rt = Content::new(TEXT.to_string(), vec![Line::new(LineKind::Para)])
                .with_marks(vec![Mark::new(6, 11, kind)]);
            let q = quill(YAML, PLATE);
            let plate = crate::read_plate(&q).expect("plate");
            let schema = quillmark_core::quill::build_transform_schema(q.config());
            let meta = crate::SchemaMeta::from_schema_json(schema.as_json());
            let data =
                serde_json::json!({ "body": quillmark_content::serial::to_canonical_value(&rt) });
            let transformed = crate::transformed_data(&meta, &data).expect("transform");
            let mut world = QuillWorld::new(&q, &plate).expect("world");
            let windows = world
                .inject_helper_package(transformed.as_ref(), &meta)
                .expect("inject");
            let (doc, _) = compile_document(&world).expect("compile");
            let helper = world
                .source(QuillWorld::helper_fid("lib.typ"))
                .expect("helper source");
            let regions = scan_content_regions(&doc, &world, &helper, &windows);
            regions
                .iter()
                .filter(|r| r.field == "body")
                .map(|r| r.rect[2] - r.rect[0])
                .fold(0.0f32, f32::max)
        };

        // `strong` stands in for "no decoration shape".
        let baseline = region_width(MarkKind::Strong);
        assert!(
            baseline > 150.0,
            "sanity: full-line region is wide: {baseline}"
        );
        for kind in [MarkKind::Emph, MarkKind::Underline, MarkKind::Strike] {
            let w = region_width(kind.clone());
            assert!(
                w >= baseline * 0.9,
                "{kind:?} region width {w} truncated vs {baseline} baseline"
            );
        }
    }

    #[test]
    fn scalar_windows_track_chains_and_single_owner_enclosing_expressions() {
        let src = Source::detached(
            r#"
#import "@local/quillmark-helper:0.1.0": data
#data.subject
#data.at("subject")
#data.refs.at(0)
#upper(data.subject)
#(data.subject + data.other)
#let s = data.other
"#,
        );
        let fields = vec![
            "subject".to_string(),
            "refs".to_string(),
            "other".to_string(),
        ];
        let wins = scalar_windows(&src, &fields);
        let text = src.text();
        let spans: Vec<(&str, &str)> = wins
            .iter()
            .map(|(p, r)| (p.as_str(), &text[r.clone()]))
            .collect();
        for expected in [
            ("subject", "data.subject"),
            ("subject", "data.at(\"subject\")"),
            ("refs", "data.refs.at(0)"),
            ("other", "data.other"),
            ("subject", "upper(data.subject)"),
        ] {
            assert!(spans.contains(&expected), "missing {expected:?}: {spans:?}");
        }
        assert!(
            !spans
                .iter()
                .any(|(_, t)| t.contains("data.subject + data.other")),
            "multi-reference expressions are not attributed: {spans:?}"
        );
        let chain_pos = spans
            .iter()
            .position(|s| *s == ("subject", "data.subject"))
            .unwrap();
        let wide_pos = spans
            .iter()
            .position(|s| *s == ("subject", "upper(data.subject)"))
            .unwrap();
        assert!(chain_pos < wide_pos, "chains sort before wides: {spans:?}");
    }

    fn collect_spans(frame: &Frame, out: &mut Vec<Span>) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(group) => collect_spans(&group.frame, out),
                FrameItem::Text(text) => out.extend(text.glyphs.iter().map(|g| g.span.0)),
                FrameItem::Shape(_, span) => out.push(*span),
                FrameItem::Image(_, _, span) => out.push(*span),
                _ => {}
            }
        }
    }

    /// Typst's synthesized list marker carries a detached span, so it lands in
    /// the "no window" bucket rather than producing `(window, None)` ink.
    #[test]
    fn two_tier_classification_resolves_each_segment_independently() {
        const YAML: &str = r#"
quill:
  name: two_tier_probe
  version: 0.1.0
  backend: typst
  description: PR-F Unknown-1 two-tier classification probe
typst:
  plate_file: plate.typ
main:
  fields:
    body:
      type: richtext
      description: a two-item list
"#;
        const PLATE: &str = r#"
#import "@local/quillmark-helper:0.1.0": data
#set page(width: 400pt, height: 400pt, margin: 40pt)
#data.body
"#;
        let q = quill(YAML, PLATE);
        let plate = crate::read_plate(&q).expect("plate");
        let schema = quillmark_core::quill::build_transform_schema(q.config());
        let meta = crate::SchemaMeta::from_schema_json(schema.as_json());
        let rt =
            quillmark_content::import::from_markdown("- Item ONE\n- Item TWO").expect("import");
        let data =
            serde_json::json!({ "body": quillmark_content::serial::to_canonical_value(&rt) });
        let transformed = crate::transformed_data(&meta, &data).expect("transform");
        let mut world = QuillWorld::new(&q, &plate).expect("world");
        let windows = world
            .inject_helper_package(transformed.as_ref(), &meta)
            .expect("inject");
        let (doc, _) = compile_document(&world).expect("compile");
        let helper = world
            .source(QuillWorld::helper_fid("lib.typ"))
            .expect("helper source");

        let win_idx = windows
            .iter()
            .position(|w| w.path == "body")
            .expect("body window");
        assert_eq!(
            windows[win_idx].segments.len(),
            2,
            "one segment per list item"
        );

        let mut cls = Classifier::new(&world, &helper, &windows);
        let mut spans = Vec::new();
        for p in doc.pages().iter() {
            collect_spans(&p.frame, &mut spans);
        }

        let mut seg_hits = [0usize; 2];
        let mut field_only_hits = 0usize;
        let mut untracked_hits = 0usize;
        for span in spans {
            match cls.classify_seg(span) {
                Some((w, Some(s))) if w == win_idx => seg_hits[s] += 1,
                Some((w, None)) if w == win_idx => field_only_hits += 1,
                _ => untracked_hits += 1,
            }
        }
        assert!(
            seg_hits[0] > 0 && seg_hits[1] > 0,
            "each list item's own ink resolves to its own segment: {seg_hits:?}"
        );
        assert_eq!(field_only_hits, 0, "list markers produce no (window, None) ink");
        assert!(
            untracked_hits > 0,
            "the two markers are hit but resolve to no window (detached span)"
        );
    }

    fn aabb(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Aabb {
        Aabb {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    fn boxable_hit(page: usize, key: (usize, Option<usize>), rect: Aabb) -> Hit {
        Hit {
            page,
            class: HitClass::Boxable { key },
            rect: Some(rect),
        }
    }

    fn transparent_hit(page: usize, window: usize) -> Hit {
        Hit {
            page,
            class: HitClass::Transparent { window },
            rect: None,
        }
    }

    fn foreign_hit(page: usize) -> Hit {
        Hit {
            page,
            class: HitClass::Foreign,
            rect: None,
        }
    }

    fn anonymous_hit(page: usize) -> Hit {
        Hit {
            page,
            class: HitClass::Anonymous,
            rect: None,
        }
    }

    /// [`run_scan_machine`] read as per-key page sequences, never-accrued keys
    /// dropped.
    fn key_pages(
        keys: &[(usize, Option<usize>)],
        hits: &[Hit],
    ) -> HashMap<(usize, Option<usize>), Vec<usize>> {
        run_scan_machine(keys, hits)
            .into_iter()
            .enumerate()
            .filter(|(_, b)| !b.is_empty())
            .map(|(i, b)| (keys[i], b.into_iter().map(|(p, _)| p).collect()))
            .collect()
    }

    /// `interrupt` between two boxable hits of one segment keeps the run, while
    /// identically-placed foreign ink ends it.
    fn assert_transparent_but_foreign_ink_is_not(interrupt: Hit) {
        let keys = vec![(0usize, Some(0usize))];
        let (a, b) = (aabb(0.0, 0.0, 1.0, 1.0), aabb(10.0, 10.0, 11.0, 11.0));

        let transparent = vec![
            boxable_hit(0, (0, Some(0)), a),
            interrupt,
            boxable_hit(0, (0, Some(0)), b),
        ];
        let boxes = run_scan_machine(&keys, &transparent);
        assert_eq!(boxes[0].len(), 1, "one page-0 box");
        let (_, bx) = boxes[0][0];
        assert!(
            bx.min_x <= 0.0 && bx.max_x >= 11.0,
            "the box unions both hits: the run stayed unbroken"
        );

        let foreign = vec![
            boxable_hit(0, (0, Some(0)), a),
            foreign_hit(0),
            boxable_hit(0, (0, Some(0)), b),
        ];
        let boxes = run_scan_machine(&keys, &foreign);
        assert_eq!(boxes[0].len(), 1, "one page-0 box, unresumed");
        let (_, bx) = boxes[0][0];
        assert!(
            bx.max_x < 11.0,
            "only the first hit accrued: same-page foreign ink ends the run"
        );
    }

    #[test]
    fn field_only_ink_is_transparent_but_foreign_ink_is_not() {
        assert_transparent_but_foreign_ink_is_not(transparent_hit(0, 0));
    }

    #[test]
    fn anonymous_ink_is_transparent_but_foreign_ink_is_not() {
        assert_transparent_but_foreign_ink_is_not(anonymous_hit(0));
    }

    #[test]
    fn adjacent_segments_of_one_field_run_independently() {
        let keys = vec![(0, Some(0)), (0, Some(1))];
        let r = aabb(0.0, 0.0, 1.0, 1.0);
        let hits = vec![
            boxable_hit(0, (0, Some(0)), r),
            transparent_hit(0, 0),
            boxable_hit(0, (0, Some(1)), r),
        ];
        let pages = key_pages(&keys, &hits);
        assert_eq!(pages[&(0, Some(0))], vec![0]);
        assert_eq!(pages[&(0, Some(1))], vec![0]);
    }

    /// Transparency is relative to a *same-window* current run only.
    #[test]
    fn field_only_ink_still_suspends_a_different_fields_current_run() {
        let keys = vec![(1, Some(0))];
        let r = aabb(0.0, 0.0, 1.0, 1.0);

        let cross_page = vec![
            boxable_hit(0, (1, Some(0)), r), // field 1's segment 0, page 0
            transparent_hit(0, 0),           // field 0's own structural ink
            boxable_hit(1, (1, Some(0)), r), // field 1's segment 0 resumes, page 1
        ];
        assert_eq!(
            key_pages(&keys, &cross_page)[&(1, Some(0))],
            vec![0, 1],
            "a foreign field's field-only ink suspends the running field \
             (the page-turn tolerance still lets it resume)"
        );

        let same_page = vec![
            boxable_hit(0, (1, Some(0)), r),
            transparent_hit(0, 0),
            boxable_hit(0, (1, Some(0)), r),
        ];
        assert_eq!(
            key_pages(&keys, &same_page)[&(1, Some(0))],
            vec![0],
            "no same-page resume: field-only ink is not a wildcard exception \
             to the foreign-ink suspension rule"
        );
    }
}
