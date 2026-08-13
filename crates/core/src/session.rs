use crate::quill::QuillConfig;
use crate::{
    ContentHit, Diagnostic, Document, RenderError, RenderOptions, RenderResult, RenderedRegion,
    Severity,
};
pub use quillmark_content::{ApplyError, Assoc, ChangeBundle, Delta, IslandOp, LineOp, MarkOp, Op};

/// What a committed [`LiveSession::update`] changed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ChangeSet {
    /// Page count after the edit.
    pub page_count: usize,
    /// Pages whose rendered content differs from the previous compile,
    /// including pages the edit added. Pages the edit removed are implied by
    /// `page_count`. A preview repaints `dirty ∩ visible` and nothing else.
    pub dirty_pages: Vec<usize>,
}

impl ChangeSet {
    pub fn new(page_count: usize, dirty_pages: Vec<usize>) -> Self {
        Self {
            page_count,
            dirty_pages,
        }
    }
}

/// Backend-specific session implementation. The `'static` bound prevents
/// borrowing source data: own anything the session must keep alive.
#[doc(hidden)]
pub trait SessionHandle: Send + Sync + 'static {
    fn render(&self, opts: &RenderOptions) -> Result<RenderResult, RenderError>;
    fn page_count(&self) -> usize;

    /// Recompile the session against new document data.
    ///
    /// Transactional: on `Err` the previous compile stays live and every read
    /// keeps serving it. The returned [`ChangeSet`] reports the pages the edit
    /// visibly changed. Default: update is unsupported.
    fn update(&mut self, _json_data: &serde_json::Value) -> Result<ChangeSet, RenderError> {
        Err(RenderError::from_diag(
            Diagnostic::new(
                Severity::Error,
                "this backend's session does not support update".to_string(),
            )
            .with_code("backend::update_unsupported".to_string()),
        ))
    }

    /// Page dimensions in points (1 pt = 1/72"), or `None` if `page` is out of
    /// range. The canvas-preview seam: a backend that can rasterize pages
    /// overrides this and [`render_rgba`](Self::render_rgba). Default `None`
    /// marks the session as having no canvas painter.
    fn page_size_pt(&self, _page: usize) -> Option<(f32, f32)> {
        None
    }

    /// Render `page` to a non-premultiplied RGBA8 buffer at `scale`× the natural
    /// 72-ppi size, returning `(width_px, height_px, rgba)` (row-major, `w*h*4`
    /// bytes), or `None` if `page` is out of range or the backend has no canvas
    /// painter. The other half of the seam paired with
    /// [`page_size_pt`](Self::page_size_pt).
    ///
    /// A backend that returns `Some` here guarantees a **complete** raster:
    /// every piece of page content is already in the returned pixels and the
    /// caller composites nothing. [`regions`](Self::regions) is for overlay and
    /// cross-navigation UIs, never required to complete the raster.
    ///
    /// A backend with no painter overrides neither this nor
    /// [`page_size_pt`](Self::page_size_pt), and
    /// [`LiveSession::supports_canvas`] derives the capability from that half
    /// of the seam rather than a separate flag.
    fn render_rgba(&self, _page: usize, _scale: f32) -> Option<(u32, u32, Vec<u8>)> {
        None
    }

    /// Schema-field geometry for the compiled session: [`RenderedRegion`]s
    /// keyed on the quill schema address each field carries.
    ///
    /// A session-level query, not a render output: computed from resolved
    /// field placements with no rasterization and no byte artifact. Default
    /// empty; a backend that places schema fields overrides this.
    ///
    /// Emit each content field's **first placement** (one region per page it
    /// touches) plus one region per widget and per scalar reference site, so
    /// `field` is not unique in the result. Order deterministically: widget
    /// regions first, then content regions in (page, field, site) order.
    fn regions(&self) -> Vec<RenderedRegion> {
        Vec::new()
    }

    /// The schema field whose content is under a point: the forward
    /// (click → field) direction of the region system. `x`/`y` are PDF points
    /// with a **bottom-left** origin on `page`, the same convention as
    /// [`RenderedRegion::rect`]. Unlike [`regions`](Self::regions), *every*
    /// placement should answer: one concrete point identifies one drawn item.
    ///
    /// The default hit-tests [`regions`](Self::regions), which is complete only
    /// for a backend whose regions enumerate every placement. A backend that
    /// emits first-placement-only content must override this with a real
    /// document hit-test, or clicks on unenumerated placements dead-end.
    fn field_at(&self, page: usize, x: f32, y: f32) -> Option<String> {
        self.regions()
            .into_iter()
            .find(|r| r.contains(page, x, y))
            .map(|r| r.field)
    }

    /// A point → **content position** in a content field: the fine-grained
    /// twin of [`field_at`](Self::field_at) (which answers with the field
    /// alone). `x`/`y` are PDF points, bottom-left origin on `page`. Returns
    /// the field plus a USV offset into its `Content`, cluster-exact and
    /// degrading to the containing segment's start on origin-less ink (see
    /// [`ContentHit`]). `None` off all content ink, on a scalar/widget (no
    /// content address), or when the backend maps no content. Default `None`:
    /// a backend that carries a per-segment source map overrides this.
    fn position_at(&self, _page: usize, _x: f32, _y: f32) -> Option<ContentHit> {
        None
    }

    /// A content position → **caret rect** in a content field: the reverse of
    /// [`position_at`](Self::position_at). `pos` is a USV offset into `field`'s
    /// `Content`; the returned [`RenderedRegion`] is the box of the glyph the
    /// caret sits at, page-indexed, with `span` collapsed to `[pos, pos]`.
    /// `None` when `field` places no tracked content or `pos` maps to no drawn
    /// glyph. Default `None`: overridden by a backend with a source map.
    fn locate(&self, _field: &str, _pos: usize) -> Option<RenderedRegion> {
        None
    }

    /// Non-fatal diagnostics of the **current compile**. They swap with the
    /// compile on each committed [`update`](Self::update), so a failed update
    /// keeps the last-good compile's warnings alongside its document.
    fn warnings(&self) -> &[Diagnostic] {
        &[]
    }
}

/// Opaque, backend-backed live render session: a persistent compiler that
/// serves reads (`render`, `paint` seams, `regions`) from its current compile
/// and takes edits via [`update`](LiveSession::update). Reads between edits see
/// a stable document (`update` is transactional, swapping the compile only on
/// success) so immutability is an invariant between commits, not a type.
///
/// Geometry reads (`regions`, `position_at`, `locate`) resolve against the
/// current compile. Anchoring a caret or selection across edits is the editor's
/// job (its own transaction mapping): the session holds no change log and maps
/// no positions forward; a consumer re-reads geometry after each committed
/// [`update`](Self::update).
pub struct LiveSession {
    inner: Box<dyn SessionHandle>,
    /// Held as the config rather than the whole [`Quill`](crate::Quill)
    /// because the compile is a pure config read; the font and package bytes
    /// stay with the backend that needed them.
    config: QuillConfig,
}

impl LiveSession {
    /// Born bound: a session cannot exist without the schema it renders. The
    /// backend has the [`Quill`](crate::Quill) in hand inside
    /// [`Backend::open`](crate::Backend::open), so binding costs it a
    /// `source.config().clone()` and buys [`update`](Self::update) a document
    /// verb whose plate is always compiled by *this* config: the pairing is
    /// structural, not an obligation on the caller.
    #[doc(hidden)]
    pub fn new(inner: Box<dyn SessionHandle>, config: QuillConfig) -> Self {
        Self { inner, config }
    }

    pub fn page_count(&self) -> usize {
        self.inner.page_count()
    }

    /// Whether this session can paint pages to a canvas: the authoritative,
    /// session-level capability. Derived directly from the canvas seam (a
    /// painter exposes [`page_size_pt`](SessionHandle::page_size_pt) for its
    /// pages), so there is no separate capability flag to keep in sync: a
    /// canvas backend pairs [`render_rgba`](Self::render_rgba) with
    /// `page_size_pt`, so this reflects what `paint` will do. A canvas-capable
    /// backend with zero pages reports `false` (nothing to paint).
    ///
    /// For a pre-session estimate (no open session yet), see
    /// [`formats_support_canvas`](crate::formats_support_canvas).
    pub fn supports_canvas(&self) -> bool {
        self.inner.page_count() > 0 && self.inner.page_size_pt(0).is_some()
    }

    /// Page dimensions in points, or `None` if `page` is out of range or the
    /// backend has no canvas painter. Generalized canvas-preview seam; see
    /// [`SessionHandle::page_size_pt`].
    pub fn page_size_pt(&self, page: usize) -> Option<(f32, f32)> {
        self.inner.page_size_pt(page)
    }

    /// Rasterize `page` to non-premultiplied RGBA8 at `scale`× 72 ppi, or `None`
    /// if `page` is out of range or the backend has no canvas painter. A `Some`
    /// result is a **complete** raster of the page (all content visible, no
    /// caller-side compositing) per the per-backend contract on
    /// [`SessionHandle::render_rgba`].
    pub fn render_rgba(&self, page: usize, scale: f32) -> Option<(u32, u32, Vec<u8>)> {
        self.inner.render_rgba(page, scale)
    }

    /// Schema-field geometry for the compiled session: each content field's
    /// **first placement** (one [`RenderedRegion`] per page it touches), plus
    /// one region per `field:`-bound widget and per direct scalar reference
    /// site, keyed on the quill schema field path. A session-level query
    /// computed without rendering bytes; an interactive preview reads it to
    /// scroll to / highlight the focused field over a `paint`-ed canvas.
    /// Empty for backends that place no schema fields.
    ///
    /// `field` is still not unique in the result: a placement breaking across
    /// pages surfaces one fragment per page (a highlight covers continuation
    /// pages), a scalar referenced at several plate sites surfaces each site,
    /// and a field arising from both tracked content and a bound widget
    /// surfaces both (overlapping rects that route to the same field). Group
    /// by `field`; every entry routes to that field in the editor. Later
    /// placements of one content value are **not** enumerated: for
    /// point-driven lookup over any placement, use
    /// [`field_at`](Self::field_at).
    ///
    /// Reflects the current compile; re-read after each committed
    /// [`update`](Self::update) to pair a highlight box with the edit it shows.
    pub fn regions(&self) -> Vec<RenderedRegion> {
        self.inner.regions()
    }

    /// The whole-field highlight boxes for `field`: one union rect per page,
    /// over the field's `span`-bearing content segments (the "highlight the
    /// focused field" quantity). The convenience that owns the union
    /// [`regions`](Self::regions) leaves derived: it keeps `regions()` as the
    /// low-level disjoint truth and folds the span-filter + per-page
    /// union here so no consumer reimplements it. Content only: a field placed
    /// solely as a scalar reference or a bound widget carries no `span` and
    /// yields nothing here; its box is a single [`regions`](Self::regions) rect.
    /// Reflects the current compile, like `regions`. See [`crate::field_boxes`].
    pub fn field_boxes(&self, field: &str) -> Vec<RenderedRegion> {
        crate::field_boxes(&self.regions(), field)
    }

    /// The schema field whose content is under a point on `page`, the
    /// forward (click → field) direction: hit-test a click against the
    /// compiled document and get back the field address to focus in the
    /// editor. `x`/`y` are PDF points with a **bottom-left** origin, the same
    /// convention as [`RenderedRegion::rect`] (a canvas consumer applies the
    /// inverse of the overlay transform it already uses for regions). Every
    /// placement answers, not just the first surfaced by
    /// [`regions`](Self::regions). `None` off any field's ink, out of range,
    /// or for backends that place no schema fields.
    pub fn field_at(&self, page: usize, x: f32, y: f32) -> Option<String> {
        self.inner.field_at(page, x, y)
    }

    /// A point → **content position**, the fine-grained click direction:
    /// hit-test a point and get back the field *and* a USV offset into its
    /// `Content`, for placing a caret or mapping a selection into the content
    /// model. `x`/`y` are PDF points, bottom-left origin, the same convention
    /// as [`field_at`](Self::field_at). The offset is cluster-exact and
    /// degrades to the containing segment's start on origin-less ink (list
    /// markers, a code fence's interior). `None` off all content ink, on a
    /// scalar/widget, or for backends with no content map. See [`ContentHit`].
    ///
    /// Resolves against the current compile; the editor owns the caret it
    /// places and anchors it across later edits itself.
    pub fn position_at(&self, page: usize, x: f32, y: f32) -> Option<ContentHit> {
        self.inner.position_at(page, x, y)
    }

    /// A content position → **caret rect**, the reverse of
    /// [`position_at`](Self::position_at): given a field and a USV offset into
    /// its `Content`, return the box (page-indexed) to draw a caret at. `None`
    /// when the field places no tracked content or the offset maps to no drawn
    /// glyph. Resolves against the current compile.
    pub fn locate(&self, field: &str, pos: usize) -> Option<RenderedRegion> {
        self.inner.locate(field, pos)
    }

    /// Non-fatal diagnostics of the session's **current compile**: set at
    /// `Backend::open` and refreshed by each committed [`update`](Self::update);
    /// a failed update keeps the last-good compile *and* its warnings. Also
    /// appended to [`RenderResult::warnings`] on each
    /// [`render`](Self::render) call. Exposed for consumers (e.g. canvas
    /// previews) that never call `render()`.
    pub fn warnings(&self) -> &[Diagnostic] {
        self.inner.warnings()
    }

    pub fn render(&self, opts: &RenderOptions) -> Result<RenderResult, RenderError> {
        let mut result = self.inner.render(opts)?;
        result
            .warnings
            .extend(self.inner.warnings().iter().cloned());
        // The regions sidecar is attached here, at the wrapper, so every
        // backend's one-shot render carries it without implementing anything
        // beyond the `regions` accessor it already has.
        if opts.regions {
            result.regions = self.inner.regions();
        }
        Ok(result)
    }

    /// Recompile the session against new document data: the edit verb of a
    /// live preview. Transactional: on `Err` the previous compile stays live,
    /// so every read keeps serving the last-good document and its
    /// [`warnings`](Self::warnings); on `Ok` the session serves the new
    /// compile (warnings included) and the [`ChangeSet`] reports what
    /// changed.
    ///
    /// `doc` is checked against the session's quill and compiled through the
    /// same pipeline as the first compile ([`QuillConfig::compile_checked`]),
    /// so an edit cannot reach the backend under a schema the session was not
    /// opened against; a mismatch errors before anything is applied and leaves
    /// the compile live like any other failed update.
    pub fn update(&mut self, doc: &Document) -> Result<ChangeSet, RenderError> {
        let json_data = self.config.compile_checked(doc)?;
        self.inner.update(&json_data)
    }

    /// [`update`](Self::update) with the schema layer cut away: plate data
    /// straight to the backend, no `$quill` check and no compile.
    ///
    /// For a backend's own acceptance tests, which drive a session against
    /// synthetic plate data to exercise recompile and dirty-page behavior,
    /// including data a schema would reject: the only lever that makes a
    /// backend's compile fail on demand.
    ///
    /// Behind the `internal-test-seam` feature rather than `#[doc(hidden)]`
    /// alone: the attribute hides a method from rustdoc and leaves it one
    /// identifier away in every consumer build, and this one carries the
    /// obligation [`update`](Self::update) exists to discharge. Off by default,
    /// so the seam is absent unless a crate asks for it.
    #[cfg(feature = "internal-test-seam")]
    #[doc(hidden)]
    pub fn update_data(&mut self, json_data: &serde_json::Value) -> Result<ChangeSet, RenderError> {
        self.inner.update(json_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::QuillReference;
    use std::str::FromStr;

    const QUILL_YAML: &str = "\
quill:
  name: memo
  backend: typst
  version: 1.0.0
  description: Session test quill
main:
  fields:
    subject:
      type: plaintext
";

    /// The schema every session in these tests is born bound to.
    fn config() -> QuillConfig {
        QuillConfig::from_yaml(QUILL_YAML).expect("valid quill")
    }

    /// A document that pairs with [`config`], so `update` reaches the handle.
    fn doc() -> Document {
        Document::new(QuillReference::from_str("memo@1.0.0").unwrap())
    }

    /// A canvas-capable session: overrides the seam for `pages` pages.
    struct CanvasHandle {
        pages: usize,
    }
    impl SessionHandle for CanvasHandle {
        fn render(&self, _: &RenderOptions) -> Result<RenderResult, RenderError> {
            unimplemented!("render is not exercised by capability tests")
        }
        fn page_count(&self) -> usize {
            self.pages
        }
        fn page_size_pt(&self, page: usize) -> Option<(f32, f32)> {
            (page < self.pages).then_some((612.0, 792.0))
        }
    }

    /// A non-canvas session: leaves the seam at its `None` defaults.
    struct PlainHandle;
    impl SessionHandle for PlainHandle {
        fn render(&self, _: &RenderOptions) -> Result<RenderResult, RenderError> {
            unimplemented!("render is not exercised by capability tests")
        }
        fn page_count(&self) -> usize {
            1
        }
    }

    /// A warning-emitting session: `warnings` reflects the current compile
    /// (one warning per committed update), and `render` succeeds empty.
    struct WarningHandle {
        current: Vec<Diagnostic>,
        applies: usize,
    }
    impl SessionHandle for WarningHandle {
        fn render(&self, _: &RenderOptions) -> Result<RenderResult, RenderError> {
            Ok(RenderResult::new(Vec::new(), crate::OutputFormat::Pdf))
        }
        fn page_count(&self) -> usize {
            1
        }
        fn update(&mut self, _: &serde_json::Value) -> Result<ChangeSet, RenderError> {
            self.applies += 1;
            self.current = vec![Diagnostic::new(
                Severity::Warning,
                format!("warning of compile {}", self.applies),
            )];
            Ok(ChangeSet {
                page_count: 1,
                dirty_pages: vec![],
            })
        }
        fn warnings(&self) -> &[Diagnostic] {
            &self.current
        }
    }

    /// `LiveSession::warnings` reflects the handle's current compile
    /// (refreshed by a committed update) and `render` appends the same set to
    /// `RenderResult::warnings`.
    #[test]
    fn warnings_track_current_compile() {
        let open_warning = vec![Diagnostic::new(Severity::Warning, "open-time".to_string())];
        let mut session = LiveSession::new(
            Box::new(WarningHandle {
                current: open_warning,
                applies: 0,
            }),
            config(),
        );
        assert_eq!(session.warnings()[0].message, "open-time");

        session.update(&doc()).unwrap();
        assert_eq!(session.warnings()[0].message, "warning of compile 1");

        let result = session.render(&RenderOptions::default()).unwrap();
        assert_eq!(result.warnings[0].message, "warning of compile 1");
    }

    /// A handle that surfaces one content region, one hit, and one caret rect:
    /// the geometry the wrapper passes straight through.
    struct RegionHandle;
    impl SessionHandle for RegionHandle {
        fn render(&self, _: &RenderOptions) -> Result<RenderResult, RenderError> {
            unimplemented!("render is not exercised by geometry tests")
        }
        fn page_count(&self) -> usize {
            1
        }
        fn regions(&self) -> Vec<RenderedRegion> {
            vec![RenderedRegion {
                field: "subject".to_string(),
                page: 0,
                rect: [1.0, 2.0, 3.0, 4.0],
                span: Some([0, 3]),
            }]
        }
        fn position_at(&self, _: usize, _: f32, _: f32) -> Option<ContentHit> {
            Some(ContentHit {
                field: "subject".to_string(),
                pos: 2,
                granularity: Some(crate::HitGranularity::Cluster),
            })
        }
        fn locate(&self, field: &str, pos: usize) -> Option<RenderedRegion> {
            Some(RenderedRegion {
                field: field.to_string(),
                page: 0,
                rect: [1.0, 2.0, 1.0, 4.0],
                span: Some([pos, pos]),
            })
        }
    }

    /// `field_boxes` derives the whole-field box off the session's own
    /// `regions()`.
    #[test]
    fn field_boxes_derives_off_regions() {
        let session = LiveSession::new(Box::new(RegionHandle), config());
        let boxes = session.field_boxes("subject");
        assert_eq!(boxes.len(), 1, "one span-bearing region → one box");
        assert_eq!(boxes[0].field, "subject");
        // A field with no span-bearing region has no derived content box.
        assert!(session.field_boxes("nope").is_empty());
    }

    #[test]
    fn supports_canvas_derives_from_seam() {
        // A session that exposes page geometry is canvas-capable…
        let canvas = LiveSession::new(Box::new(CanvasHandle { pages: 2 }), config());
        assert!(canvas.supports_canvas());
        // …one that leaves the seam at its defaults is not…
        let plain = LiveSession::new(Box::new(PlainHandle), config());
        assert!(!plain.supports_canvas());
        // …and a canvas backend with no pages has nothing to paint.
        let empty = LiveSession::new(Box::new(CanvasHandle { pages: 0 }), config());
        assert!(!empty.supports_canvas());
    }
}
