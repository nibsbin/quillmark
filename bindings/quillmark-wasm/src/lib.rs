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
//! - [`Quillmark`] - Engine for registering Quills and rendering markdown
//!
//! ## Workflow
//!
//! The typical workflow consists of four steps:
//!
//! 1. **Parse Markdown** - Use `Quillmark.parseMarkdown()` to parse markdown with YAML frontmatter
//! 2. **Register Quill** - Use `registerQuill()` to load and register a Quill template bundle (JSON format)
//! 3. **Get Quill Info** - Use `getQuillInfo()` to retrieve metadata and configuration options
//! 4. **Render** - Use `render()` with the ParsedDocument and render options
//!
//! ## Example (JavaScript/TypeScript)
//!
//! ```javascript
//! import { Quillmark } from '@quillmark-test/wasm';
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
//! engine.registerQuill('letter-quill', quillJson);
//!
//! // Step 3: Get Quill info to inspect available options
//! const info = engine.getQuillInfo('letter-quill');
//! console.log('Supported formats:', info.supportedFormats);
//! console.log('Field schemas:', info.fieldSchemas);
//!
//! // Step 4: Render
//! const result = engine.render(parsed, { format: 'pdf' });
//! const pdfBytes = result.artifacts[0].bytes;
//! ```
//!
//! ## Error Handling
//!
//! All errors are represented as [`JsValue`] containing serialized [`QuillmarkError`] objects
//! with diagnostic information.

use wasm_bindgen::prelude::*;

mod engine;
mod error;
mod types;

pub use engine::Quillmark;
pub use error::QuillmarkError;
pub use types::*;

/// Initialize the WASM module with panic hooks for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}
