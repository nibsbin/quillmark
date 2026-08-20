//! The incremental-update envelope shared by the stamp and flatten paths: open a
//! base PDF (validate the reader's input contract, read the trailer, seed the
//! object-id counter, optionally stamp `/Info` `/Producer`) and close it with one
//! incremental-update append. Each path supplies only the objects in between.

use crate::error::PdfError;
use crate::reader::{
    append_incremental_update, assert_overwrite_gen_zero, assert_unrotated_pages, err,
    find_dict_value, open_trailer, parse_indirect_ref, resolve_page_ids, UpdatedObject,
};
use crate::writer::apply_producer_stamp;
use crate::FieldSpec;

const CODE_PARSE: &str = "pdf::update_parse";

/// One incremental-update revision in progress.
#[non_exhaustive]
pub struct PdfUpdate {
    xref_offset: usize,
    /// The base PDF's catalog (`/Root`) object id.
    pub catalog_id: u32,
    /// Next free object id, seeded at the trailer `/Size`. Hand out via
    /// [`alloc_id`](crate::writer::alloc_id).
    pub next_id: u32,
    /// Objects to write in this revision; callers push their own onto it.
    pub objects: Vec<UpdatedObject>,
    /// `Some` when the producer stamp allocated a fresh `/Info`, which
    /// [`finish`](PdfUpdate::finish) threads into the new trailer.
    extra_info_ref: Option<u32>,
}

impl PdfUpdate {
    /// Open `pdf` for an incremental update. The caller then pushes its objects
    /// onto [`objects`](Self::objects) and calls [`finish`](Self::finish).
    pub fn begin(pdf: &[u8], producer: Option<&str>) -> Result<Self, PdfError> {
        let (xref_offset, trailer, catalog_id) = open_trailer(pdf, CODE_PARSE)?;
        if find_dict_value(trailer, "Encrypt").is_some() {
            return Err(err(
                "pdf::encrypted",
                "PDF is encrypted; the stamp spine does not handle encrypted PDFs",
            ));
        }
        // The new trailer re-references the catalog as `/Root <id> 0 R`, so a
        // non-zero-generation catalog would be silently corrupted even when only
        // the producer is stamped (catalog not itself overwritten).
        assert_overwrite_gen_zero(pdf, catalog_id, "catalog (/Root)")?;
        let size = find_dict_value(trailer, "Size")
            .and_then(|v| std::str::from_utf8(v.trim_ascii()).ok())
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| err(CODE_PARSE, "/Size missing or malformed in trailer"))?;
        let info_ref = find_dict_value(trailer, "Info").and_then(parse_indirect_ref);

        // One counter seeded at `/Size`, so created ids never collide with the
        // base's. `alloc_id` checks it: a malformed near-`u32::MAX` `/Size` errors
        // instead of wrapping into a colliding id.
        let mut next_id = size;
        let mut objects: Vec<UpdatedObject> = Vec::new();
        let mut extra_info_ref = None;
        if let Some(producer) = producer {
            extra_info_ref =
                apply_producer_stamp(pdf, info_ref, producer, &mut next_id, &mut objects)?;
        }

        Ok(Self {
            xref_offset,
            catalog_id,
            next_id,
            objects,
            extra_info_ref,
        })
    }

    /// Resolve the base's page object ids, bounds-checking every field's `page`
    /// so a spec targeting a non-existent page errors rather than panicking later.
    pub fn resolve_pages(&self, pdf: &[u8], fields: &[FieldSpec]) -> Result<Vec<u32>, PdfError> {
        let page_ids = resolve_page_ids(pdf, self.catalog_id)?;
        let page_count = page_ids.len();
        // Both assertions below re-scan the whole byte buffer, so validate each
        // distinct page once rather than pay O(fields × file_size).
        let mut checked = vec![false; page_count];
        let mut targeted = Vec::new();
        for spec in fields {
            if spec.page >= page_count {
                return Err(err(
                    CODE_PARSE,
                    format!(
                        "field {:?} targets page {} but the PDF has {page_count} page(s)",
                        spec.name, spec.page
                    ),
                ));
            }
            if checked[spec.page] {
                continue;
            }
            // A targeted page is overwritten and referenced as gen 0, so a
            // non-zero-generation page would be silently corrupted.
            assert_overwrite_gen_zero(pdf, page_ids[spec.page], "page")?;
            checked[spec.page] = true;
            targeted.push(page_ids[spec.page]);
        }
        // Geometry is written in unrotated user space, so a rotated target page
        // would mis-place every field.
        assert_unrotated_pages(pdf, self.catalog_id, &targeted)?;
        Ok(page_ids)
    }

    /// Serialize the accumulated objects onto `pdf` via one incremental-update
    /// append, threading in a freshly-allocated `/Info` when there is one.
    pub fn finish(self, pdf: Vec<u8>) -> Result<Vec<u8>, PdfError> {
        append_incremental_update(
            pdf,
            self.xref_offset,
            self.catalog_id,
            self.next_id,
            self.extra_info_ref,
            &self.objects,
        )
    }
}
