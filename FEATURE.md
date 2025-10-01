# Dynamic Asset Integration Feature

## Overview

This document describes the design for dynamically adding files to Workflow renders using a builder pattern. This feature enables users to add runtime assets (images, data files, etc.) that will be accessible within templates via a new `Asset` filter.

## Motivation

Currently, Quill templates can only access assets that are physically present in the `assets/` directory of a quill template when it's loaded. This limitation prevents:

- **Runtime-generated content**: Charts, graphs, or images generated during rendering
- **User-provided files**: Documents that users upload or provide at render time
- **Dynamic data**: Files that vary based on context or user input

The dynamic asset feature addresses these limitations by allowing programmatic injection of files into the rendering context.

## High-Level Design

### User-Facing API

Users will be able to add files dynamically through a builder pattern on the `Workflow` struct:

```rust
use quillmark::{Quillmark, OutputFormat};

// Create workflow from a quill
let mut engine = Quillmark::new();
let quill = Quill::from_path("path/to/quill")?;
engine.register_quill(quill);
let workflow = engine.load("my-quill")?;

// Add dynamic assets using builder pattern
let result = workflow
    .with_asset("chart.png", chart_bytes)?
    .with_asset("data.csv", csv_bytes)?
    .render(markdown, Some(OutputFormat::Pdf))?;
```

### Template Usage

In Typst glue templates, dynamic assets are referenced via the `Asset` filter:

```typst
// Reference a dynamic asset
#image("{{ 'chart.png' | Asset }}")

// Reference a static asset (existing behavior)
#image("assets/logo.png")
```

The `Asset` filter transforms a filename into the full virtual path with the required prefix to access the dynamic asset.

## Technical Design

### 1. Workflow Builder Pattern

#### Modified Workflow Structure

```rust
pub struct Workflow {
    backend: Box<dyn Backend>,
    quill: Quill,
    dynamic_assets: HashMap<String, Vec<u8>>, // NEW: stores dynamic assets
}
```

#### Builder Methods

```rust
impl Workflow {
    /// Add a dynamic asset to the workflow
    /// 
    /// # Arguments
    /// * `filename` - The filename to use (e.g., "chart.png")
    /// * `contents` - The file contents as bytes
    /// 
    /// # Errors
    /// Returns error if a dynamic asset with the same filename already exists (collision detection)
    pub fn with_asset(
        mut self,
        filename: impl Into<String>,
        contents: impl Into<Vec<u8>>
    ) -> Result<Self, RenderError> {
        let filename = filename.into();
        
        // Check for collision
        if self.dynamic_assets.contains_key(&filename) {
            return Err(RenderError::DynamicAssetCollision {
                filename,
                message: format!(
                    "Dynamic asset '{}' already exists. Each asset filename must be unique.",
                    filename
                ),
            });
        }
        
        self.dynamic_assets.insert(filename, contents.into());
        Ok(self)
    }
    
    /// Add multiple dynamic assets at once
    pub fn with_assets(
        mut self,
        assets: impl IntoIterator<Item = (String, Vec<u8>)>
    ) -> Result<Self, RenderError> {
        for (filename, contents) in assets {
            self = self.with_asset(filename, contents)?;
        }
        Ok(self)
    }
    
    /// Clear all dynamic assets from the workflow
    /// 
    /// This method removes all previously added dynamic assets, allowing you to
    /// start fresh or conditionally reset the asset state in a builder chain.
    /// 
    /// # Example
    /// ```rust
    /// let workflow = engine.load("report")?
    ///     .with_asset("temp.png", temp_bytes)?
    ///     .clear_assets()  // Remove temp.png
    ///     .with_asset("final.png", final_bytes)?
    ///     .render(markdown, Some(OutputFormat::Pdf))?;
    /// ```
    pub fn clear_assets(mut self) -> Self {
        self.dynamic_assets.clear();
        self
    }
}
```

### 2. Asset Prefix and Integration

#### Prefix Convention

Dynamic assets will be stored with the `DYNAMIC_ASSET__` prefix to avoid collisions with static assets:

- Static asset: `assets/logo.png` → virtual path `assets/logo.png`
- Dynamic asset: `chart.png` → virtual path `assets/DYNAMIC_ASSET__chart.png`

The prefix ensures that:
1. Dynamic assets don't conflict with static assets
2. The asset system can distinguish between static and dynamic assets
3. Virtual path organization remains clean and predictable

#### Quill Cloning and Modification

Before passing the Quill to the backend, the Workflow will clone it and inject dynamic assets:

```rust
impl Workflow {
    /// Internal method to prepare a quill with dynamic assets
    fn prepare_quill_with_assets(&self) -> Quill {
        let mut quill = self.quill.clone();
        
        // Add dynamic assets to the cloned quill's file system
        for (filename, contents) in &self.dynamic_assets {
            let prefixed_path = PathBuf::from(format!("assets/DYNAMIC_ASSET__{}", filename));
            let entry = FileEntry {
                contents: contents.clone(),
                path: prefixed_path.clone(),
                is_dir: false,
            };
            quill.files.insert(prefixed_path, entry);
        }
        
        quill
    }
    
    /// Modified render method to use prepared quill
    pub fn render(
        &self,
        markdown: &str,
        format: Option<OutputFormat>,
    ) -> Result<RenderResult, RenderError> {
        let glue_output = self.process_glue(markdown)?;
        
        // Prepare quill with dynamic assets
        let prepared_quill = self.prepare_quill_with_assets();
        
        // Pass prepared quill to backend
        self.render_content_with_quill(&glue_output, format, &prepared_quill)
    }
}
```

### 3. Asset Filter Implementation

#### Filter Registration

The `Asset` filter will be registered in the Typst backend alongside existing filters:

```rust
impl Backend for TypstBackend {
    fn register_filters(&self, glue: &mut Glue) {
        glue.register_filter("String", string_filter);
        glue.register_filter("Lines", lines_filter);
        glue.register_filter("Date", date_filter);
        glue.register_filter("Dict", dict_filter);
        glue.register_filter("Body", body_filter);
        glue.register_filter("Asset", asset_filter); // NEW
    }
}
```

#### Filter Implementation

```rust
/// Asset filter - converts a filename to a dynamic asset path
/// 
/// Usage in templates:
///   {{ 'chart.png' | Asset }}
///   {{ filename_variable | Asset }}
/// 
/// Output:
///   "assets/DYNAMIC_ASSET__chart.png"
pub fn asset_filter(
    _state: &State,
    value: Value,
    _kwargs: Kwargs
) -> Result<Value, Error> {
    // Get the filename from the value
    let filename = value.to_string();
    
    // Validate filename (no path separators allowed for security)
    if filename.contains('/') || filename.contains('\\') {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!(
                "Asset filename cannot contain path separators: '{}'",
                filename
            ),
        ));
    }
    
    // Build the prefixed path
    let asset_path = format!("assets/DYNAMIC_ASSET__{}", filename);
    
    // Return as a Typst string literal
    Ok(Value::from_safe_string(format!("\"{}\"", asset_path)))
}
```

### 4. Error Handling

#### New Error Variant

Add a new error variant to `RenderError` in `quillmark-core/src/error.rs`:

```rust
pub enum RenderError {
    // ... existing variants ...
    
    /// Error when a dynamic asset collision occurs
    #[error("Dynamic asset collision: {filename}")]
    DynamicAssetCollision {
        filename: String,
        message: String,
    },
}
```

#### Collision Detection

Collisions are detected at two levels:

1. **During asset addition** (via `with_asset`): Prevents adding the same dynamic asset twice
2. **Implicit validation**: The prefix system prevents dynamic assets from colliding with static assets

### 5. Backend Integration

#### QuillWorld Asset Loading

The existing `load_assets_from_quill` method in `QuillWorld` will automatically load dynamic assets because they're added to the Quill's file system:

```rust
impl QuillWorld {
    fn load_assets_from_quill(
        quill: &Quill,
        binaries: &mut HashMap<FileId, Bytes>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get all files that start with "assets/"
        // This will include both static assets and DYNAMIC_ASSET__ prefixed files
        let asset_paths = quill.find_files("assets/*");

        for asset_path in asset_paths {
            if let Some(contents) = quill.get_file(&asset_path) {
                // Create virtual path for the asset
                let virtual_path = VirtualPath::new(asset_path.to_string_lossy().as_ref());
                let file_id = FileId::new(None, virtual_path);
                binaries.insert(file_id, Bytes::new(contents.to_vec()));
            }
        }

        Ok(())
    }
}
```

No changes needed to this method—it already handles the prefixed paths correctly.

## Implementation Roadmap

### Phase 1: Core Infrastructure

1. **Extend Workflow struct**
   - Add `dynamic_assets` field to `Workflow`
   - Implement `with_asset` and `with_assets` methods
   - Add `prepare_quill_with_assets` helper

2. **Add error handling**
   - Add `DynamicAssetCollision` variant to `RenderError`
   - Implement collision detection logic

3. **Update render flow**
   - Modify `render` method to use prepared quill
   - Add `render_content_with_quill` internal method
   - Ensure `render_content` still works for backward compatibility

### Phase 2: Filter Implementation

1. **Implement Asset filter**
   - Add `asset_filter` function in `quillmark-typst/src/filters.rs`
   - Implement path validation
   - Implement prefix transformation

2. **Register filter**
   - Update `register_filters` in `TypstBackend`
   - Add filter to backend initialization

### Phase 3: Testing & Documentation

1. **Unit tests**
   - Test collision detection
   - Test asset filter path transformation
   - Test filename validation (reject paths with separators)

2. **Integration tests**
   - Test end-to-end workflow with dynamic assets
   - Test rendering with both static and dynamic assets
   - Test error cases (collisions, invalid filenames)

3. **Documentation**
   - Update API documentation
   - Add examples to README
   - Update DESIGN.md with dynamic asset section

## Usage Examples

### Example 1: Adding a Generated Chart

```rust
use quillmark::{Quillmark, Quill, OutputFormat};

// Generate a chart as PNG bytes
let chart_data = generate_chart(&data)?;

// Create workflow and add dynamic asset
let mut engine = Quillmark::new();
let quill = Quill::from_path("./quills/report")?;
engine.register_quill(quill);

let result = engine
    .load("report")?
    .with_asset("monthly_chart.png", chart_data)?
    .render(markdown_content, Some(OutputFormat::Pdf))?;
```

Template (`glue.typ`):
```typst
= Monthly Report

#image({{ 'monthly_chart.png' | Asset }}, width: 100%)
```

### Example 2: Multiple Dynamic Assets

```rust
let assets = vec![
    ("chart1.png".to_string(), chart1_bytes),
    ("chart2.png".to_string(), chart2_bytes),
    ("data.csv".to_string(), csv_bytes),
];

let result = engine
    .load("report")?
    .with_assets(assets)?
    .render(markdown_content, Some(OutputFormat::Pdf))?;
```

Template (`glue.typ`):
```typst
#image({{ 'chart1.png' | Asset }})
#image({{ 'chart2.png' | Asset }})
#import csv: csv
#let data = csv({{ 'data.csv' | Asset }})
```

### Example 3: Conditional Asset Usage

```typst
// In frontmatter:
---
include_chart: true
chart_name: "sales_chart.png"
---

// In template:
#if {{ include_chart | String }} == "true" [
  #image({{ chart_name | Asset }})
]
```

### Example 4: Clearing and Replacing Assets

```rust
// Start with draft assets
let mut workflow = engine.load("report")?
    .with_asset("draft_chart.png", draft_chart_bytes)?
    .with_asset("draft_data.csv", draft_data_bytes)?;

// Conditionally clear and replace with final assets
if use_final_version {
    workflow = workflow
        .clear_assets()  // Remove all draft assets
        .with_asset("final_chart.png", final_chart_bytes)?
        .with_asset("final_data.csv", final_data_bytes)?;
}

let result = workflow.render(markdown, Some(OutputFormat::Pdf))?;
```

Template (`glue.typ`):
```typst
// References work for both draft and final assets
#image({{ chart_filename | Asset }})
#import csv: csv
#let data = csv({{ data_filename | Asset }})
```

## Security Considerations

### Path Traversal Prevention

The `Asset` filter validates filenames to prevent path traversal attacks:

```rust
// REJECT: "../../../etc/passwd"
// REJECT: "subdir/file.png"
// ACCEPT: "chart.png"
// ACCEPT: "my-data.csv"
```

Only simple filenames without path separators are allowed.

### Prefix Isolation

The `DYNAMIC_ASSET__` prefix ensures dynamic assets are isolated from static assets, preventing:
- Overwriting static template assets
- Accessing unintended template files
- Namespace confusion

## Backward Compatibility

This feature is fully backward compatible:

1. **Existing workflows** continue to work without changes
2. **Static assets** are unaffected and continue to load normally
3. **New builder methods** are optional; workflows can be used without them
4. **Filter is additive**: The `Asset` filter is a new addition that doesn't affect existing filters

## Alternative Designs Considered

### 1. Direct Path in Filter

**Rejected approach**: Allow the Asset filter to accept full paths like `assets/DYNAMIC_ASSET__chart.png`

**Reason**: This exposes implementation details to users and is error-prone. The filter should abstract away the prefix.

### 2. Separate Dynamic Asset Directory

**Rejected approach**: Store dynamic assets in `dynamic_assets/` instead of prefixed in `assets/`

**Reason**: This would require changes to the virtual file system structure and backend loading logic. The prefix approach is simpler and more contained.

### 3. Asset Registry Pattern

**Rejected approach**: Maintain a separate registry mapping logical names to dynamic assets

**Reason**: Adds complexity without clear benefits. The in-memory file system approach is more aligned with existing Quill architecture.

## Future Enhancements

### 1. Asset Metadata

Support for asset metadata like MIME types, descriptions:

```rust
pub struct DynamicAsset {
    filename: String,
    contents: Vec<u8>,
    mime_type: Option<String>,
    description: Option<String>,
}
```

### 2. Asset Validation

Validate assets based on type (e.g., ensure PNG files have valid PNG headers):

```rust
impl Workflow {
    pub fn with_validated_image(
        self,
        filename: impl Into<String>,
        contents: impl Into<Vec<u8>>
    ) -> Result<Self, RenderError> {
        let contents = contents.into();
        validate_image_format(&contents)?;
        self.with_asset(filename, contents)
    }
}
```

### 3. Asset Preprocessing

Support for automatic asset transformations:

```rust
impl Workflow {
    pub fn with_auto_optimized_image(
        self,
        filename: impl Into<String>,
        contents: impl Into<Vec<u8>>
    ) -> Result<Self, RenderError> {
        let optimized = optimize_image(contents.into())?;
        self.with_asset(filename, optimized)
    }
}
```

### 4. Streaming Assets

For large assets, support streaming instead of loading entirely into memory:

```rust
pub fn with_asset_from_reader(
    self,
    filename: impl Into<String>,
    reader: impl Read
) -> Result<Self, RenderError>
```

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_asset_collision_detection() {
    let workflow = Workflow::new(backend, quill)?;
    let workflow = workflow.with_asset("chart.png", vec![1, 2, 3])?;
    
    // Should fail - asset already exists
    let result = workflow.with_asset("chart.png", vec![4, 5, 6]);
    assert!(matches!(result, Err(RenderError::DynamicAssetCollision { .. })));
}

#[test]
fn test_asset_filter_rejects_paths() {
    // Should reject filenames with path separators
    let result = asset_filter(&state, Value::from("../hack.png"), Kwargs::new());
    assert!(result.is_err());
}

#[test]
fn test_asset_filter_transforms_filename() {
    let result = asset_filter(&state, Value::from("chart.png"), Kwargs::new())?;
    assert_eq!(result.to_string(), "\"assets/DYNAMIC_ASSET__chart.png\"");
}

#[test]
fn test_clear_assets() {
    let workflow = Workflow::new(backend, quill)?;
    let workflow = workflow
        .with_asset("chart1.png", vec![1, 2, 3])?
        .with_asset("chart2.png", vec![4, 5, 6])?
        .clear_assets();
    
    // After clearing, should be able to add the same filenames again
    let workflow = workflow.with_asset("chart1.png", vec![7, 8, 9])?;
    assert!(workflow.is_ok());
}
```

### Integration Tests

```rust
#[test]
fn test_dynamic_asset_in_render() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_path("tests/fixtures/simple-quill")?;
    engine.register_quill(quill);
    
    let image_bytes = vec![0xFF, 0xD8, 0xFF]; // JPEG header
    let markdown = r#"
---
title: Test
---
Image: {{ 'test.jpg' | Asset }}
"#;
    
    let result = engine
        .load("simple-quill")?
        .with_asset("test.jpg", image_bytes)?
        .render(markdown, Some(OutputFormat::Pdf))?;
    
    assert!(!result.artifacts.is_empty());
}
```

## Open Questions

1. **Should we support individual asset removal?**
   - Add a `without_asset` method to remove specific previously added assets?
   - Current design includes `clear_assets()` to remove all assets
   - Consider if granular removal is needed for specific use cases

2. **Should we limit the number or size of dynamic assets?**
   - Prevent memory exhaustion from too many/large assets?
   - Add configuration for limits?

3. **Should the Asset filter be backend-specific or generic?**
   - Current design: Typst-specific
   - Alternative: Make it a core filter available to all backends
   - Decision: Start backend-specific, generalize if other backends need it

4. **Should we support subdirectories in dynamic assets?**
   - Current design: Flat namespace only (no separators)
   - Alternative: Allow "charts/Q1.png" → "assets/DYNAMIC_ASSET__charts/Q1.png"
   - Decision: Start with flat namespace for simplicity and security

## References

- [DESIGN.md](./DESIGN.md) - Architecture overview and asset loading
- [quillmark-core/src/lib.rs](./quillmark-core/src/lib.rs) - Quill and FileEntry structures
- [quillmark-typst/src/world.rs](./quillmark-typst/src/world.rs) - Asset loading implementation
- [quillmark-typst/src/filters.rs](./quillmark-typst/src/filters.rs) - Filter implementations
- [quillmark/src/lib.rs](./quillmark/src/lib.rs) - Workflow API
