//! # Quillmark WASM
//!
//! WebAssembly bindings for the Quillmark markdown rendering engine.
//!
//! This crate provides a JavaScript/TypeScript API for using Quillmark in web browsers,
//! Node.js, and other JavaScript environments.
//!
//! ## API Structure
//!
//! The WASM API provides three main classes:
//!
//! - [`QuillmarkEngine`] - Engine for managing backends and Quills
//! - [`Quill`] - Represents a Quill template bundle
//! - [`Workflow`] - Rendering workflow for a specific Quill
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

pub use engine::QuillmarkEngine;
pub use error::QuillmarkError;
pub use quill::Quill;
pub use types::*;
pub use workflow::Workflow;

/// Initialize the WASM module with panic hooks for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}
