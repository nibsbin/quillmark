//! WebAssembly bindings for Quillmark.
//!
//! Three build variants ship from this one crate: a Typst-less **core** build
//! (`pkg/core/`) with no engine, and two engine-carrying backend binaries
//! (`pkg/backends/typst/`, `pkg/backends/pdfform/`) gated by the `typst` /
//! `pdfform` cargo features. None of them is a public npm export: the package
//! root `@quillmark/wasm` is the hand-written canonical layer (`pkg/runtime/`),
//! which re-exports the core build's `Quill` / `Document` and wraps the FFI
//! [`Quillmark`] below in an `Engine` that lazily loads a backend.

use wasm_bindgen::prelude::*;

mod engine;
mod error;
mod types;

pub use engine::{Document, Quill};
#[cfg(any(feature = "typst", feature = "pdfform"))]
pub use engine::{LiveSession, Quillmark};
pub use error::WasmError;
pub use types::*;

/// Runs at instantiation (the wasm-bindgen start section): installs the panic
/// hook, so a Rust panic reaches the console as a stack trace rather than
/// `unreachable`.
///
/// Not the package's `init`. That name belongs to the hand-written runtime,
/// which owns instantiation itself (`runtime/runtime.js`); this runs as part of
/// the instantiation it awaits.
#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
}
