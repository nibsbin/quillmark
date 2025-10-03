# Backend Trait

Backend trait for implementing output format backends.

## Overview

The `Backend` trait defines the interface that backends must implement
to support different output formats (PDF, SVG, TXT, etc.).

## Trait Definition

```rust,ignore
pub trait Backend: Send + Sync {
    fn id(&self) -> &'static str;
    fn supported_formats(&self) -> &'static [OutputFormat];
    fn glue_type(&self) -> &'static str;
    fn register_filters(&self, glue: &mut Glue);
    fn compile(
        &self,
        glue_content: &str,
        quill: &Quill,
        opts: &RenderOptions,
    ) -> Result<Vec<Artifact>, RenderError>;
}
```

## Implementation Guide

### Required Methods

#### `id()`
Return a unique backend identifier (e.g., "typst", "latex").

#### `supported_formats()`
Return a slice of `OutputFormat` variants this backend supports.

#### `glue_type()`
Return the file extension for glue files (e.g., ".typ", ".tex").

#### `register_filters()`
Register backend-specific filters with the glue environment.

```rust,no_run
# use quillmark_core::{Glue, templating::filter_api::{State, Value, Kwargs, Error}};
# fn string_filter(_: &State, v: Value, _: Kwargs) -> Result<Value, Error> { Ok(v) }
# fn content_filter(_: &State, v: Value, _: Kwargs) -> Result<Value, Error> { Ok(v) }
# fn lines_filter(_: &State, v: Value, _: Kwargs) -> Result<Value, Error> { Ok(v) }
# struct MyBackend;
# impl MyBackend {
fn register_filters(&self, glue: &mut Glue) {
    glue.register_filter("String", string_filter);
    glue.register_filter("Content", content_filter);
    glue.register_filter("Lines", lines_filter);
}
# }
```

#### `compile()`
Compile glue content into final artifacts.

```rust,no_run
# use quillmark_core::{Quill, RenderOptions, Artifact, OutputFormat, RenderError};
# struct MyBackend;
# impl MyBackend {
fn compile(
    &self,
    glue_content: &str,
    quill: &Quill,
    opts: &RenderOptions,
) -> Result<Vec<Artifact>, RenderError> {
    // 1. Create compilation environment
    // 2. Load assets from quill
    // 3. Compile glue content
    // 4. Handle errors and map to Diagnostics
    // 5. Return artifacts
    # let compiled_pdf = vec![];
    
    Ok(vec![Artifact {
        bytes: compiled_pdf,
        output_format: OutputFormat::Pdf,
    }])
}
# }
```

## Example Implementation

See `quillmark-typst` for a complete backend implementation example.

## Thread Safety

The `Backend` trait requires `Send + Sync` to enable concurrent rendering.
All backend implementations must be thread-safe.
