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
//! 1. Build a render-ready quill with `engine.quillFromTree(...)`
//! 2. Parse markdown via `ParsedDocument.fromMarkdown(...)` (or pass markdown directly)
//! 3. Render with `quill.render(...)`
//!
//! ## Example
//!
//! ```javascript
//! import { ParsedDocument, Quillmark } from '@quillmark-test/wasm';
//!
//! const engine = new Quillmark();
//! const quill = engine.quillFromTree(tree);
//!
//! const parsed = ParsedDocument.fromMarkdown(markdown);
//! const result = quill.render(parsed, { format: 'pdf' });
//! const pdfBytes = result.artifacts[0].bytes;
//! ```
//!
//! `Quillmark.parseMarkdown(...)` is kept as a deprecated wrapper around
//! `ParsedDocument.fromMarkdown(...)`.

use wasm_bindgen::prelude::*;

mod engine;
mod error;
mod types;

pub use engine::{CompiledDocument, Quill, Quillmark};
pub use error::WasmError;
pub use types::*;

/// Initialize the WASM module with panic hooks for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}
