//! # Quillmark WASM
//!
//! WebAssembly bindings for Quillmark.
//!
//! ## API
//!
//! - [`Quillmark`] - engine for loading render-ready quills from in-memory trees
//! - [`Quill`] - quill handle for rendering/compiling
//! - [`ParsedDocument`] - parsed markdown payload (`fromMarkdown` static)
//!
//! ## Workflow
//!
//! 1. Build a render-ready quill with `engine.quill(...)`
//! 2. Parse markdown via `ParsedDocument.fromMarkdown(...)`
//! 3. Render with `quill.render(...)`
//!
//! ## Example
//!
//! ```javascript
//! import { ParsedDocument, Quillmark } from '@quillmark-test/wasm';
//!
//! const engine = new Quillmark();
//! const quill = engine.quill(tree);
//!
//! const parsed = ParsedDocument.fromMarkdown(markdown);
//! const result = quill.render(parsed);
//! const pdfBytes = result.artifacts[0].bytes;
//! ```

use wasm_bindgen::prelude::*;

mod engine;
mod error;
mod types;

pub use engine::{Quill, Quillmark, RenderSession};
pub use error::WasmError;
pub use types::*;

/// Initialize the WASM module with panic hooks for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}
