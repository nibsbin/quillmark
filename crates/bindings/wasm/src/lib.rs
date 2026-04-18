//! # Quillmark WASM
//!
//! WebAssembly bindings for the Quillmark markdown rendering engine.
//!
//! This crate provides a JavaScript/TypeScript API for using Quillmark in web browsers,
//! Node.js, and other JavaScript environments.
//!
//! ## API
//!
//! The WASM API provides a single class for all operations:
//!
//! - [`Quill`] - Opaque quill handle created by factory methods
//! - [`Quillmark`] - Engine for registering Quills and rendering markdown
//!
//! ## Workflow
//!
//! The typical workflow consists of five steps:
//!
//! 1. **Parse Markdown** - Use `Quillmark.parseMarkdown()` to parse markdown with YAML frontmatter
//! 2. **Build Quill** - Use `Quill.fromJson()` or `Quill.fromTree()` to parse and validate a quill
//! 3. **Register Quill** - Use `registerQuill()` with a `Quill` handle
//! 4. **Get Quill Info** - Use `getQuillInfo()` to retrieve metadata and configuration options
//! 5. **Render** - Use `render()` with the ParsedDocument and render options
//!
//! ## Example (JavaScript/TypeScript)
//!
//! ```javascript
//! import { Quill, Quillmark } from '@quillmark-test/wasm';
//!
//! // Step 1: Parse markdown
//! const markdown = `---
//! title: My Document
//! author: Alice
//! QUILL: letter-quill
//! ---
//!
//! # Hello World
//!
//! This is my document.
//! `;
//!
//! const parsed = Quillmark.parseMarkdown(markdown);
//!
//! // Step 2: Load and register Quill (from JSON)
//! const engine = new Quillmark();
//! const quillJson = { /* Quill file tree in JSON format */ };
//! const quill = Quill.fromJson(quillJson);
//! engine.registerQuill(quill);
//!
//! // Step 3: Get Quill info to inspect available options
//! const info = engine.getQuillInfo('letter-quill');
//! console.log('Supported formats:', info.supportedFormats);
//! console.log('Schema YAML:', info.schema);
//!
//! // Step 4: Render
//! const result = engine.render(parsed, { format: 'pdf' });
//! const pdfBytes = result.artifacts[0].bytes;
//! ```
//!
//! ## Error Handling
//!
//! All errors are represented as [`JsValue`] containing serialized [`WasmError`] objects
//! with diagnostic information from the core error types.

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
