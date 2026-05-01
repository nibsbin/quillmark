//! Canvas painter for Typst preview rendering.
//!
//! Reaches through `core::RenderSession` into the Typst backend's cached
//! `PagedDocument` (via `quillmark_typst::typst_session_of`), rasterises the
//! requested page, and blits the pixels into a `CanvasRenderingContext2d`.
//!
//! No PNG/SVG encoding round-trip — pixels go straight from `typst-render`
//! into the canvas backing store.

use wasm_bindgen::{Clamped, JsValue};
use web_sys::{CanvasRenderingContext2d, ImageData};

use crate::error::WasmError;

/// Page dimensions in Typst points (1 pt = 1/72 inch).
///
/// Returns `None` if the session was not opened by the Typst backend, or if
/// `page` is out of range.
pub fn page_size_pt(
    session: &quillmark_core::RenderSession,
    page: usize,
) -> Option<(f32, f32)> {
    quillmark_typst::typst_session_of(session)?.page_size_pt(page)
}

/// Paint `page` into `ctx`, filling the canvas backing store at `scale`× the
/// natural 72 ppi.
///
/// Caller is responsible for sizing `ctx.canvas()` so that `canvas.width ==
/// round(pageWidthPt * scale)` and `canvas.height == round(pageHeightPt *
/// scale)` before invoking. The painter writes the rendered pixmap at origin
/// `(0, 0)`.
///
/// `backend_id` is the resolved backend identifier and is included in the
/// error message when the session was opened by a backend without a canvas
/// painter — keeping the failure self-explanatory in the browser console.
pub fn paint(
    session: &quillmark_core::RenderSession,
    ctx: &CanvasRenderingContext2d,
    page: usize,
    scale: f32,
    backend_id: &str,
) -> Result<(), JsValue> {
    let typst_session = quillmark_typst::typst_session_of(session).ok_or_else(|| {
        WasmError::from(format!(
            "paint: backend '{}' does not support canvas preview",
            backend_id
        ))
        .to_js_value()
    })?;

    let (width, height, mut rgba) = typst_session.render_rgba(page, scale).ok_or_else(|| {
        WasmError::from(format!(
            "paint: page index {} out of range (pageCount={}, backend='{}')",
            page,
            session.page_count(),
            backend_id,
        ))
        .to_js_value()
    })?;

    let img = ImageData::new_with_u8_clamped_array_and_sh(Clamped(rgba.as_mut_slice()), width, height)
        .map_err(|e| {
            WasmError::from(format!("paint: ImageData construction failed: {:?}", e)).to_js_value()
        })?;

    ctx.put_image_data(&img, 0.0, 0.0)
}
