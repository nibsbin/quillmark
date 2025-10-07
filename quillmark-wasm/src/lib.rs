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
//! ## Error Handling
//!
//! All errors are represented as [`JsValue`] containing serialized [`QuillmarkError`] objects
//! with diagnostic information.

use wasm_bindgen::prelude::*;

mod engine;
mod error;
mod quill;
mod types;
mod workflow;

pub use engine::Quillmark;
pub use error::QuillmarkError;
pub use types::*;

// Legacy exports for backward compatibility (not part of public API)
#[doc(hidden)]
pub use quill::Quill;
#[doc(hidden)]
pub use workflow::Workflow;

/// Initialize the WASM module with panic hooks for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}
