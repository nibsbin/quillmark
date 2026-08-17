//! Typst backend for Quillmark: markdown + card-YAML data → PDF, SVG, PNG.
//!
//! Richtext fields cross the seam as canonical content JSON and are lowered to
//! Typst markup by [`emit`] at codegen time; no code re-parses markdown at
//! render time. Plates read fields through the `@local/quillmark-helper`
//! virtual package. Form-field widgets become AcroForm widgets on PDF output
//! only; SVG and PNG render an invisible placeholder.

mod compile;
/// Content → Typst-markup lowering plus its per-segment source map.
///
/// **Workspace-internal; not covered by this crate's semver.** `pub` only so
/// `quillmark-fuzz` can drive the escapers and the lowering directly. The
/// supported surface is [`TypstBackend`].
#[doc(hidden)]
pub mod emit;
mod error_mapping;

mod helper;
mod overlay;
mod world;

use std::borrow::Cow;
use std::collections::BTreeMap;

use quillmark_core::{
    quill::build_transform_schema,
    session::SessionHandle,
    Backend, ChangeSet, ContentHit, Diagnostic, LiveSession, OutputFormat, Quill, RenderError,
    RenderOptions, RenderResult, RenderedRegion, Severity,
};

/// Typst backend implementation for Quillmark.
#[derive(Debug)]
pub struct TypstBackend;

const SUPPORTED_FORMATS: &[OutputFormat] =
    &[OutputFormat::Pdf, OutputFormat::Svg, OutputFormat::Png];

/// Persisting the world keeps fonts, packages, and assets parsed once per
/// session rather than once per compile: the substrate for incremental
/// recompiles.
struct TypstSession {
    world: world::QuillWorld,
    document: typst_layout::PagedDocument,
    page_count: usize,
    /// Extracted from each committed compile; converted to spine `FieldSpec`s
    /// on every render.
    field_placements: Vec<overlay::FieldPlacement>,
    /// Built once at `open`: the schema never changes for a session's lifetime,
    /// and codegen plus date validation read only these tables.
    schema_meta: SchemaMeta,
    /// The span scan's classification table for the live compile: generated
    /// content-block windows then the plate's scalar reference-site windows.
    /// Swapped transactionally with the document.
    windows: Vec<overlay::FieldWindow>,
    /// The plate is static for a session's lifetime, so these are computed once
    /// at `open` and re-appended into `windows` per apply.
    scalar_windows: Vec<overlay::FieldWindow>,
    /// Span resolution goes through this snapshot, not the world: a failed
    /// `apply` leaves the *next* injection's text in the world while every read
    /// keeps serving this compile.
    helper_source: typst::syntax::Source,
    /// Diffed against the next compile's to produce `ChangeSet::dirty_pages`.
    page_hashes: Vec<u128>,
    /// What [`session_warnings`] built for the live compile. The compile half
    /// swaps on each committed `apply`; the load half rides along unchanged.
    warnings: Vec<Diagnostic>,
    /// [`overlay::unclosed_claims`] for the live document, suppressed by every
    /// region and point query. Computed with the compile rather than per query:
    /// `field_at` cannot derive it from the page prefix it walks.
    unclosed_claims: Vec<usize>,
}

/// The quill's load warnings, then this compile's own, then one per runaway
/// `field-region`. One order, built in one place, so an `apply` that swaps only
/// what it recompiled keeps the load half.
fn session_warnings(
    world: &world::QuillWorld,
    compile: Vec<Diagnostic>,
    unclosed: &[(usize, String)],
) -> Vec<Diagnostic> {
    let mut all = world.load_warnings().to_vec();
    all.extend(compile);
    all.extend(unclosed.iter().map(|(_, field)| {
        Diagnostic::new(
            Severity::Warning,
            format!("`field-region(\"{field}\")` never closed; its claim was dropped"),
        )
        .with_code("typst::unclosed_field_region".to_string())
        .with_hint(
            "both markers must reach the frame together, so emit the call's \
             return value whole rather than in parts"
                .to_string(),
        )
    }));
    all
}

/// Per-page fingerprints of *visible* content, diffed across compiles for
/// `ChangeSet::dirty_pages`. Introspection `Tag` items and group parent
/// locations are skipped: they carry element hashes spanning content on *other*
/// pages, so hashing them dirties page 0 on an end-of-document edit.
///
/// Pixels, not spans: every hashed item drops its source-location `Span`. Only
/// render-affecting data may enter the hash, or a helper-layout shift that moves
/// no ink (a reordered `data` literal regenerating `lib.typ`) reports every
/// content page dirty.
pub(crate) fn page_hashes(document: &typst_layout::PagedDocument) -> Vec<u128> {
    use std::hash::{Hash, Hasher};
    use typst::layout::FrameItem;

    fn hash_text<H: Hasher>(text: &typst::text::TextItem, state: &mut H) {
        text.font.hash(state);
        text.size.hash(state);
        text.fill.hash(state);
        text.stroke.hash(state);
        text.lang.hash(state);
        text.region.hash(state);
        text.text.hash(state);
        for g in &text.glyphs {
            g.id.hash(state);
            g.x_advance.hash(state);
            g.x_offset.hash(state);
            g.y_advance.hash(state);
            g.y_offset.hash(state);
            g.range.hash(state);
            // g.span deliberately omitted: source location, not pixels.
        }
    }

    fn walk<H: Hasher>(frame: &typst::layout::Frame, state: &mut H) {
        frame.size().hash(state);
        for (pos, item) in frame.items() {
            match item {
                FrameItem::Tag(_) => {}
                FrameItem::Group(g) => {
                    pos.hash(state);
                    g.transform.hash(state);
                    g.clip.hash(state);
                    walk(&g.frame, state);
                }
                FrameItem::Text(text) => {
                    pos.hash(state);
                    hash_text(text, state);
                }
                // Shape/Image carry a trailing `Span` their derived `Hash` would
                // fold in; destructure to hash the visible parts and drop it, same
                // reason as glyph spans above.
                FrameItem::Shape(shape, _span) => {
                    pos.hash(state);
                    shape.hash(state);
                }
                FrameItem::Image(image, size, _span) => {
                    pos.hash(state);
                    image.hash(state);
                    size.hash(state);
                }
                FrameItem::Link(dest, size) => {
                    pos.hash(state);
                    dest.hash(state);
                    size.hash(state);
                }
            }
        }
    }

    struct VisiblePage<'a>(&'a typst_layout::Page);
    impl Hash for VisiblePage<'_> {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.0.fill.hash(state);
            self.0.numbering.hash(state);
            self.0.number.hash(state);
            walk(&self.0.frame, state);
        }
    }

    document
        .pages()
        .iter()
        .map(|page| typst::utils::hash128(&VisiblePage(page)))
        .collect()
}

/// The seam already carries the render shape, so no per-field transform happens
/// here; codegen validates each date at the site it parses it. Borrows
/// `json_data` unchanged for the object case: only a non-object input allocates.
fn transformed_data(json_data: &serde_json::Value) -> Cow<'_, serde_json::Value> {
    match json_data.is_object() {
        true => Cow::Borrowed(json_data),
        false => Cow::Owned(serde_json::Value::Object(serde_json::Map::new())),
    }
}

impl SessionHandle for TypstSession {
    fn render(&self, opts: &RenderOptions) -> Result<RenderResult, RenderError> {
        let format = opts.output_format.unwrap_or(OutputFormat::Pdf);

        if !SUPPORTED_FORMATS.contains(&format) {
            return Err(quillmark_core::unsupported_format(
                format,
                "typst",
                SUPPORTED_FORMATS,
            ));
        }

        compile::render_document_pages(
            &self.document,
            opts.pages.as_deref(),
            format,
            opts.ppi,
            &self.field_placements,
            opts.producer.as_deref(),
        )
    }

    fn page_count(&self) -> usize {
        self.page_count
    }

    /// Transactional: the live document, placements, hashes, and compile
    /// warnings swap together only after the compile *and* placement extraction
    /// succeed, so on `Err` every read keeps serving the last-good compile.
    fn update(&mut self, json_data: &serde_json::Value) -> Result<ChangeSet, RenderError> {
        let data = transformed_data(json_data);
        let mut windows = self
            .world
            .inject_helper_package(data.as_ref(), &self.schema_meta)
            .map_err(|e| engine_err(e.code(), e.to_string()))?;
        windows.extend(self.scalar_windows.iter().cloned());

        let (document, compile_warnings) = compile::compile_document(&self.world)?;
        let helper_source = helper_source(&self.world)?;
        let field_placements = overlay::extract(&document)?;
        let new_hashes = page_hashes(&document);
        let unclosed = overlay::unclosed_claims(&document);

        let dirty_pages = (0..new_hashes.len())
            .filter(|&i| self.page_hashes.get(i) != Some(&new_hashes[i]))
            .collect();

        self.document = document;
        self.field_placements = field_placements;
        self.windows = windows;
        self.helper_source = helper_source;
        self.page_count = new_hashes.len();
        self.page_hashes = new_hashes;
        self.warnings = session_warnings(&self.world, compile_warnings, &unclosed);
        self.unclosed_claims = unclosed.into_iter().map(|(claim, _)| claim).collect();

        Ok(ChangeSet::new(self.page_count, dirty_pages))
    }

    fn warnings(&self) -> &[Diagnostic] {
        &self.warnings
    }

    /// Typst points: 1 pt = 1/72 inch.
    fn page_size_pt(&self, page: usize) -> Option<(f32, f32)> {
        let frame = &self.document.pages().get(page)?.frame;
        let size = frame.size();
        Some((size.x.to_pt() as f32, size.y.to_pt() as f32))
    }

    /// Non-premultiplied RGBA8 at `scale`× the natural 72 ppi, returned as
    /// `(width_px, height_px, rgba)` with `w * h * 4` row-major bytes.
    fn render_rgba(&self, page: usize, scale: f32) -> Option<(u32, u32, Vec<u8>)> {
        let p = self.document.pages().get(page)?;
        let pixmap = typst_render::render(
            p,
            &typst_render::RenderOptions {
                pixel_per_pt: typst::utils::Scalar::new(scale as f64),
                ..Default::default()
            },
        );
        let width = pixmap.width();
        let height = pixmap.height();
        let mut rgba = Vec::with_capacity((width as usize) * (height as usize) * 4);
        for px in pixmap.pixels() {
            let c = px.demultiply();
            rgba.push(c.red());
            rgba.push(c.green());
            rgba.push(c.blue());
            rgba.push(c.alpha());
        }
        Some((width, height, rgba))
    }

    /// Widgets first (one fixed-size box each), then span-tracked content in
    /// (page, field, site) order. `field` is not unique: page fragments, several
    /// scalar reference sites, or tracked content plus a bound widget all
    /// repeat it, so consumers group by field. Widget regions are empty if the
    /// placements fail to resolve; a render surfaces the same error.
    fn regions(&self) -> Vec<quillmark_core::RenderedRegion> {
        let mut regions = self.widget_regions();
        regions.extend(overlay::scan_content_regions(
            &self.document,
            &self.world,
            &self.helper_source,
            &self.windows,
            &self.unclosed_claims,
        ));
        regions
    }

    /// Widget boxes answer first: a widget is a deliberate click target drawing
    /// no spanned ink of its own, so content ink beneath it must not swallow the
    /// click. Among overlapping widgets the later-painted one wins, matching
    /// `span_scan::field_at`.
    fn field_at(&self, page: usize, x: f32, y: f32) -> Option<String> {
        self.widget_regions()
            .into_iter()
            .rev()
            .find(|r| r.contains(page, x, y))
            .map(|r| r.field)
            .or_else(|| {
                overlay::field_at(
                    &self.document,
                    &self.world,
                    &self.helper_source,
                    &self.windows,
                    &self.unclosed_claims,
                    page,
                    x,
                    y,
                )
            })
    }

    /// The fine-grained twin of [`field_at`](Self::field_at). Widgets draw no
    /// spanned content ink, so unlike `field_at` they are not consulted.
    fn position_at(&self, page: usize, x: f32, y: f32) -> Option<ContentHit> {
        overlay::position_at(
            &self.document,
            &self.world,
            &self.helper_source,
            &self.windows,
            page,
            x,
            y,
        )
    }

    fn locate(&self, field: &str, pos: usize) -> Option<RenderedRegion> {
        overlay::locate(
            &self.document,
            &self.world,
            &self.helper_source,
            &self.windows,
            field,
            pos,
        )
    }
}

impl TypstSession {
    /// The single derivation `regions` and `field_at` both read.
    fn widget_regions(&self) -> Vec<quillmark_core::RenderedRegion> {
        overlay::build_field_specs(&self.document, &self.field_placements)
            .map(|specs| quillmark_pdf::regions_of(&specs))
            .unwrap_or_default()
    }
}

/// Snapshotted right after a successful compile: the text the served document's
/// spans resolve against.
fn helper_source(world: &world::QuillWorld) -> Result<typst::syntax::Source, RenderError> {
    use typst::World as _;
    world
        .source(world::QuillWorld::helper_fid("lib.typ"))
        .map_err(|e| {
            engine_err(
                "typst::helper_source",
                format!("helper lib.typ unreadable: {e}"),
            )
        })
}

impl quillmark_core::backend::sealed::Sealed for TypstBackend {}

impl Backend for TypstBackend {
    fn id(&self) -> &'static str {
        "typst"
    }

    fn supported_formats(&self) -> &'static [OutputFormat] {
        SUPPORTED_FORMATS
    }

    fn open(
        &self,
        source: &Quill,
        json_data: &serde_json::Value,
    ) -> Result<LiveSession, RenderError> {
        let plate_content = read_plate(source)?;

        let transform_schema = build_transform_schema(source.config());
        let schema_meta = SchemaMeta::from_schema_json(transform_schema.as_json());
        let data = transformed_data(json_data);
        // Built in two steps rather than through `new_with_data` so codegen's own
        // diagnostic code survives: boxing it into the world-creation error would
        // relabel a bad date `typst::world_creation`.
        let mut world = world::QuillWorld::new(source, &plate_content).map_err(|e| {
            RenderError::from_diag(
                Diagnostic::new(
                    Severity::Error,
                    format!("Failed to create Typst compilation environment: {}", e),
                )
                .with_code("typst::world_creation".to_string())
                .with_source(e.as_ref()),
            )
        })?;
        let mut windows = world
            .inject_helper_package(data.as_ref(), &schema_meta)
            .map_err(|e| engine_err(e.code(), e.to_string()))?;
        // The plate is static for the session: window its scalar sites once.
        let scalar_windows: Vec<overlay::FieldWindow> = {
            use typst::World as _;
            let main_id = world.main();
            world
                .source(main_id)
                .ok()
                .map(|src| {
                    overlay::scalar_windows(
                        &src,
                        &schema_meta.fields,
                        &schema_meta.object_fields,
                        &schema_meta.array_fields,
                    )
                    .into_iter()
                    .map(|(path, range)| overlay::FieldWindow {
                        path,
                        file: main_id,
                        range,
                        segments: Vec::new(),
                    })
                    .collect()
                })
                .unwrap_or_default()
        };
        windows.extend(scalar_windows.iter().cloned());
        let (document, compile_warnings) = compile::compile_document(&world)?;
        let unclosed = overlay::unclosed_claims(&document);
        let warnings = session_warnings(&world, compile_warnings, &unclosed);
        let helper_src = helper_source(&world)?;
        let page_count = document.pages().len();
        let field_placements = overlay::extract(&document)?;
        let hashes = page_hashes(&document);
        let session = TypstSession {
            world,
            document,
            page_count,
            field_placements,
            schema_meta,
            windows,
            scalar_windows,
            helper_source: helper_src,
            page_hashes: hashes,
            warnings,
            unclosed_claims: unclosed.into_iter().map(|(claim, _)| claim).collect(),
        };
        Ok(LiveSession::new(
            Box::new(session),
            source.config().clone(),
        ))
    }
}

impl Default for TypstBackend {
    fn default() -> Self {
        Self
    }
}

/// The plate is a Typst-only notion: its filename is declared under the
/// `typst:` backend-config section as `plate_file` and the source lives in the
/// quill's file bundle. A quill declaring no `plate_file` renders an empty one.
fn read_plate(source: &Quill) -> Result<String, RenderError> {
    let plate_file = source
        .config()
        .backend_config
        .get("plate_file")
        .and_then(|v| v.as_str());

    let Some(plate_file) = plate_file else {
        return Ok(String::new());
    };

    let bytes = source.files().get_file(plate_file).ok_or_else(|| {
        engine_err(
            "typst::plate_missing",
            format!("plate file '{plate_file}' not found in the quill's file tree"),
        )
    })?;

    String::from_utf8(bytes.to_vec()).map_err(|e| {
        engine_err(
            "typst::invalid_utf8",
            format!("plate file '{plate_file}' is not valid UTF-8: {e}"),
        )
    })
}

/// A single-diagnostic [`RenderError`] carrying `code`.
fn engine_err(code: &str, message: impl Into<String>) -> RenderError {
    RenderError::from_diag(
        Diagnostic::new(Severity::Error, message.into()).with_code(code.to_string()),
    )
}

/// Property names a container node declares, in declaration order.
fn property_names(node: &serde_json::Value) -> Vec<String> {
    node.get("properties")
        .and_then(|v| v.as_object())
        .map(|props| props.keys().cloned().collect())
        .unwrap_or_default()
}

/// The fields addressable by index suffix (`refs.0`), each mapped to the
/// property names its *row* offers (`refs.0.org`). A primitive-element array
/// maps to the empty list: the index step is all it admits.
///
/// The property step's twin, and the same shape, so one predicate reads both.
/// The nesting contract caps the suffix at a row property, so neither recurses.
fn array_field_names(
    properties: &serde_json::Map<String, serde_json::Value>,
) -> BTreeMap<String, Vec<String>> {
    properties
        .iter()
        .filter(|(_, fs)| fs.get("type").and_then(|v| v.as_str()) == Some("array"))
        .map(|(name, fs)| {
            let row = fs.get("items").map(property_names).unwrap_or_default();
            (name.clone(), row)
        })
        .collect()
}

/// The fields addressable by one property step (`address.city`,
/// `classification.poc`), each mapped to the property names it declares.
///
/// A typed dictionary and a variant container both project as `type: object`
/// carrying `properties` — the container's own `value` among them — so one
/// predicate spans both. A richtext field is `type: object` too but declares no
/// `properties`; unlike an array, which always offers its index step, an
/// object with no properties offers no step at all and so is left out.
fn object_field_names(
    properties: &serde_json::Map<String, serde_json::Value>,
) -> BTreeMap<String, Vec<String>> {
    properties
        .iter()
        .filter(|(_, fs)| fs.get("type").and_then(|v| v.as_str()) == Some("object"))
        .map(|(name, fs)| (name.clone(), property_names(fs)))
        .filter(|(_, names)| !names.is_empty())
        .collect()
}

/// The transform schema plus the address tables derived from it, kept apart
/// because they answer different questions at different depths. Lowering reads
/// the schema node ([`helper::lowering`]) and is depth-invariant; the tables
/// answer which *names* a plate may write, and are bounded by the address
/// grammar. A table can only ever answer the second.
#[derive(Default)]
pub(crate) struct SchemaMeta {
    /// The walk's cursor source: the same recursive projection
    /// `build_transform_schema` produced, kept whole rather than flattened.
    schema: serde_json::Value,
    pub(crate) fields: Vec<String>,
    /// Array field → its row's property names, gating `field.0` and
    /// `field.0.prop`.
    pub(crate) array_fields: BTreeMap<String, Vec<String>>,
    /// Container field → its property names, gating `field.sub`. Native rather
    /// than JSON: the span scan reads it on the AST walk, and `meta_literal`
    /// serializes it either way.
    pub(crate) object_fields: BTreeMap<String, Vec<String>>,
    pub(crate) card_fields: BTreeMap<String, Vec<String>>,
    pub(crate) card_array_fields: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    pub(crate) card_object_fields: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

impl SchemaMeta {
    /// A schema with no top-level `properties` yields empty tables and a walk
    /// that lowers every value literally: `build_transform_schema` always emits
    /// `properties`, so that only arises for hand-built schemas in tests.
    pub(crate) fn from_schema_json(schema_json: &serde_json::Value) -> Self {
        let empty = serde_json::Map::new();
        let properties_obj = schema_json
            .get("properties")
            .and_then(|v| v.as_object())
            .unwrap_or(&empty);

        let mut card_fields = BTreeMap::new();
        let mut card_array_fields = BTreeMap::new();
        let mut card_object_fields = BTreeMap::new();
        if let Some(defs) = schema_json.get("$defs").and_then(|v| v.as_object()) {
            for (def_name, def_schema) in defs {
                let Some(kind) = def_name.strip_suffix("_card") else {
                    continue;
                };
                let Some(props) = def_schema.get("properties").and_then(|v| v.as_object()) else {
                    continue;
                };
                card_fields.insert(kind.to_string(), props.keys().cloned().collect());
                let arrays = array_field_names(props);
                if !arrays.is_empty() {
                    card_array_fields.insert(kind.to_string(), arrays);
                }
                let objects = object_field_names(props);
                if !objects.is_empty() {
                    card_object_fields.insert(kind.to_string(), objects);
                }
            }
        }

        Self {
            fields: properties_obj.keys().cloned().collect(),
            array_fields: array_field_names(properties_obj),
            object_fields: object_field_names(properties_obj),
            card_fields,
            card_array_fields,
            card_object_fields,
            schema: schema_json.clone(),
        }
    }

    /// The schema node declaring a top-level field.
    pub(crate) fn field_node(&self, name: &str) -> Option<&serde_json::Value> {
        self.schema.get("properties")?.get(name)
    }

    pub(crate) fn card_props(
        &self,
        kind: &str,
    ) -> Option<&serde_json::Map<String, serde_json::Value>> {
        self.schema
            .get("$defs")?
            .get(format!("{kind}_card"))?
            .get("properties")?
            .as_object()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quillmark_core::quill::CONTENT_MEDIA_TYPE;
    use quillmark_core::QuillValue;
    use serde_json::json;
    use std::collections::HashMap;

    /// The shape the seam carries for a richtext field.
    fn content(markdown: &str) -> serde_json::Value {
        let rt = quillmark_content::import::from_markdown(markdown).expect("import");
        quillmark_content::serial::to_canonical_value(&rt)
    }

    /// The two quills differ only by an extra unused schema field, which
    /// lengthens the generated `_qm-meta` literal ahead of the content blocks
    /// and so shifts every glyph's span without moving a pixel.
    #[test]
    fn page_hashes_ignore_span_shift_when_ink_is_identical() {
        use quillmark_core::FileTreeNode;

        const PLATE: &str = r#"#import "@local/quillmark-helper:0.1.0": data
#set page(width: 300pt, height: 200pt, margin: 20pt)
#set text(size: 11pt)
#data.body
"#;
        let quill_with = |extra_field: bool| {
            let mut yaml = String::from(
                "quill:\n  name: shift\n  version: 0.1.0\n  backend: typst\n  description: span shift probe\ntypst:\n  plate_file: plate.typ\nmain:\n  fields:\n    body:\n      type: richtext\n      description: body\n",
            );
            if extra_field {
                yaml.push_str(
                    "    zz_unused:\n      type: string\n      description: never placed\n",
                );
            }
            let mut files = HashMap::new();
            files.insert(
                "Quill.yaml".to_string(),
                FileTreeNode::File {
                    contents: yaml.into_bytes(),
                },
            );
            files.insert(
                "plate.typ".to_string(),
                FileTreeNode::File {
                    contents: PLATE.as_bytes().to_vec(),
                },
            );
            Quill::from_tree(FileTreeNode::Directory { files }).expect("quill")
        };

        let json =
            serde_json::json!({ "body": content("A **markdown** body with real ink to lay out.") });
        let hashes_of = |quill: &Quill| {
            let plate_content = read_plate(quill).expect("plate");
            let transform_schema = build_transform_schema(quill.config());
            let schema_meta = SchemaMeta::from_schema_json(transform_schema.as_json());
            let data = transformed_data(&json);
            let (world, _windows) =
                world::QuillWorld::new_with_data(quill, &plate_content, data.as_ref(), &schema_meta)
                    .expect("world");
            let (document, _warnings) = compile::compile_document(&world).expect("compile");
            page_hashes(&document)
        };

        assert_eq!(
            hashes_of(&quill_with(false)),
            hashes_of(&quill_with(true)),
            "identical ink must fingerprint identically across a whole-file span shift"
        );
    }

    /// Asks Typst's introspector how many blocks the compile really produced,
    /// so a future Typst that changes line-anchoring fails loud here.
    #[test]
    fn line_anchored_paragraph_text_stays_literal() {
        use quillmark_core::FileTreeNode;
        use typst::foundations::{NativeElement, Selector};
        use typst::introspection::Introspector;
        use typst::model::{EnumElem, HeadingElem, ListElem, TermsElem};

        const PLATE: &str = r#"#import "@local/quillmark-helper:0.1.0": data
#set page(width: 300pt, height: 400pt, margin: 20pt)
#set text(size: 11pt)
#data.body
"#;
        let quill = || {
            let yaml = "quill:\n  name: anchor\n  version: 0.1.0\n  backend: typst\n  description: line-anchor guard\ntypst:\n  plate_file: plate.typ\nmain:\n  fields:\n    body:\n      type: richtext\n      description: body\n";
            let mut files = HashMap::new();
            files.insert(
                "Quill.yaml".to_string(),
                FileTreeNode::File { contents: yaml.as_bytes().to_vec() },
            );
            files.insert(
                "plate.typ".to_string(),
                FileTreeNode::File { contents: PLATE.as_bytes().to_vec() },
            );
            Quill::from_tree(FileTreeNode::Directory { files }).expect("quill")
        };
        // `Para` lines directly: markdown import would parse `- `/`+ `/`N. ` as
        // real lists, which is not the bug.
        use quillmark_content::model::{Line, LineKind, Content};
        let para = |_: usize| Line::new(LineKind::Para);
        let mut rt = Content::new(
            "= Heading\n- bullet\n+ numbered\n1. dotted\n/ term: desc".to_string(),
            (0..5).map(para).collect(),
        );
        rt.normalize();
        assert_eq!(rt.validate(), Ok(()), "content invariants");
        let q = quill();
        let json =
            serde_json::json!({ "body": quillmark_content::serial::to_canonical_value(&rt) });
        let plate_content = read_plate(&q).expect("plate");
        let transform_schema = build_transform_schema(q.config());
        let schema_meta = SchemaMeta::from_schema_json(transform_schema.as_json());
        let data = transformed_data(&json);
        let (world, _w) =
            world::QuillWorld::new_with_data(&q, &plate_content, data.as_ref(), &schema_meta)
                .expect("world");
        let (document, _warn) = compile::compile_document(&world).expect("compile");

        let intro = document.introspector();
        let count = |e| intro.query(&Selector::Elem(e, None)).len();
        assert_eq!(count(<HeadingElem as NativeElement>::ELEM), 0, "no heading");
        assert_eq!(count(<ListElem as NativeElement>::ELEM), 0, "no bullet list");
        assert_eq!(count(<EnumElem as NativeElement>::ELEM), 0, "no enum");
        assert_eq!(count(<TermsElem as NativeElement>::ELEM), 0, "no term list");
    }

    #[test]
    fn inline_field_in_par_emits_no_parbreak_warning() {
        use quillmark_core::FileTreeNode;

        const PLATE: &str = r#"#import "@local/quillmark-helper:0.1.0": data
#set page(width: 300pt, height: 200pt, margin: 20pt)
#set text(size: 11pt)
#par(data.subject)
"#;
        let quill = |inline: bool| {
            let inline_line = if inline { "      inline: true\n" } else { "" };
            let yaml = format!(
                "quill:\n  name: parbreak\n  version: 0.1.0\n  backend: typst\n  description: parbreak probe\ntypst:\n  plate_file: plate.typ\nmain:\n  fields:\n    subject:\n      type: richtext\n{inline_line}      description: subject\n",
            );
            let mut files = HashMap::new();
            files.insert(
                "Quill.yaml".to_string(),
                FileTreeNode::File { contents: yaml.into_bytes() },
            );
            files.insert(
                "plate.typ".to_string(),
                FileTreeNode::File { contents: PLATE.as_bytes().to_vec() },
            );
            Quill::from_tree(FileTreeNode::Directory { files }).expect("quill")
        };
        let warnings_for = |inline: bool| {
            let q = quill(inline);
            let json = serde_json::json!({ "subject": content("A subject line") });
            let plate_content = read_plate(&q).expect("plate");
            let transform_schema = build_transform_schema(q.config());
            let schema_meta = SchemaMeta::from_schema_json(transform_schema.as_json());
            let data = transformed_data(&json);
            let (world, _w) =
                world::QuillWorld::new_with_data(&q, &plate_content, data.as_ref(), &schema_meta)
                    .expect("world");
            let (_doc, warnings) = compile::compile_document(&world).expect("compile");
            warnings
        };
        let has_parbreak =
            |ws: &[Diagnostic]| ws.iter().any(|d| d.message.contains("parbreak"));

        // Negative control: the block lowering warns, so the probe has teeth.
        assert!(
            has_parbreak(&warnings_for(false)),
            "block richtext in par() should emit the parbreak warning"
        );
        assert!(
            !has_parbreak(&warnings_for(true)),
            "inline richtext in par() must emit no parbreak warning"
        );
    }

    #[test]
    fn schema_meta_array_fields_distinguish_scalar_from_array() {
        // Any array is element-addressable (`field.N`); only scalars are not.
        // A typed table's rows publish their properties too (`refs.0.org`).
        let schema = QuillValue::from_json(json!({
            "type": "object",
            "properties": {
                "subject": { "type": "object", "contentMediaType": CONTENT_MEDIA_TYPE },
                "sections": {
                    "type": "array",
                    "items": { "type": "object", "contentMediaType": CONTENT_MEDIA_TYPE }
                },
                "signature_block": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            },
            "$defs": {
                "indorsement_card": {
                    "type": "object",
                    "properties": {
                        "$body": { "type": "object", "contentMediaType": CONTENT_MEDIA_TYPE },
                        "refs": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": { "org": { "type": "string" } }
                            }
                        }
                    }
                }
            }
        }));

        let meta = SchemaMeta::from_schema_json(schema.as_json());

        assert!(meta.array_fields.contains_key("sections"));
        assert!(meta.array_fields.contains_key("signature_block"));
        assert!(!meta.array_fields.contains_key("subject"));
        // A richtext element declares no `properties`, so the row is empty and
        // `sections.0.anything` stays unaddressable.
        assert!(meta.array_fields["sections"].is_empty());

        let card_arrays = meta.card_array_fields.get("indorsement").unwrap();
        assert_eq!(card_arrays["refs"], vec!["org".to_string()]);
    }

    #[test]
    fn schema_meta_serves_the_declaring_node_at_every_position() {
        let schema = QuillValue::from_json(json!({
            "type": "object",
            "properties": {
                "issued": { "type": "string", "format": "date" },
                "contact": { "type": "object", "properties": {
                    "reply_by": { "type": "string", "format": "date" }
                }}
            },
            "$defs": {
                "indorsement_card": {
                    "type": "object",
                    "properties": { "signed_on": { "type": "string", "format": "date" } }
                }
            }
        }));
        let meta = SchemaMeta::from_schema_json(schema.as_json());

        let format_of = |node: Option<&serde_json::Value>| {
            node.and_then(|n| n.get("format"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
        };
        assert_eq!(format_of(meta.field_node("issued")).as_deref(), Some("date"));
        // A nested declaration is reachable, which is what the lowering walk
        // reads and what a table of top-level names cannot express.
        let contact = meta.field_node("contact").expect("contact declared");
        assert_eq!(
            format_of(contact.get("properties").and_then(|p| p.get("reply_by"))).as_deref(),
            Some("date"),
        );
        assert_eq!(
            format_of(meta.card_props("indorsement").and_then(|p| p.get("signed_on"))).as_deref(),
            Some("date"),
        );
        assert!(meta.field_node("nonexistent").is_none());
        assert!(meta.card_props("nonexistent").is_none());
    }
}
