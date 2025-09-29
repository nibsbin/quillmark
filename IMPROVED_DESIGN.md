# QuillMark Production Architecture
*An opinionated, ergonomic markdown rendering engine with seamless modular backends*

## Philosophy: Less is More

QuillMark prioritizes developer ergonomics over customizability. The architecture follows these core principles:

- **Convention over Configuration**: Smart defaults eliminate boilerplate
- **Zero-Friction Integration**: Get started with a single function call
- **Production-Ready by Default**: Built-in error handling, caching, and performance optimizations
- **Seamless Modularity**: Backends plug in transparently without ceremony

## Simple API Design

### The One-Line Experience
```rust
use quillmark::render_pdf;

// Render markdown to PDF in one line
let pdf_bytes = render_pdf("# Hello World\nThis is markdown.")?;
```

### Template-Based Documents
```rust
use quillmark::{render_with_template, Backend};

// Use a template with smart template discovery
let artifacts = render_with_template(
    markdown,
    "report-template",  // Auto-discovers from templates/ directory
    Backend::Typst      // Enum for compile-time backend selection
)?;
```

### Advanced Configuration (When Needed)
```rust
use quillmark::{QuillEngine, EngineConfig};

let engine = QuillEngine::new(EngineConfig {
    template_dirs: vec!["./templates", "~/.quillmark/templates"],
    cache_enabled: true,
    backend: Backend::Auto,  // Smart backend selection
    ..Default::default()
});

let result = engine.render(markdown).with_template("invoice").to_pdf()?;
```

## Opinionated Architecture

### Convention-Based Structure
```
project/
├── templates/           # Auto-discovered template directory
│   ├── report/         # Template name matches directory
│   │   ├── template.typ # Convention: template.{backend_ext}
│   │   ├── style.toml  # Convention: optional styling config
│   │   └── assets/     # Convention: template assets
│   └── invoice/
│       └── template.typ
├── content/            # Markdown source files (optional convention)
└── output/            # Generated artifacts (auto-created)
```

### Smart Template Discovery
1. Look in `./templates/{name}/template.{ext}` (project-local)
2. Look in `~/.quillmark/templates/{name}/template.{ext}` (user-global)
3. Look in built-in templates (shipped with QuillMark)
4. Auto-generate minimal template if none found

### Backend as Implementation Detail
```rust
pub enum Backend {
    Auto,      // Smart selection based on output format and available backends
    Typst,     // Compile-time backend selection
    LaTeX,     // (when implemented)
    Pandoc,    // (when implemented)
}
```

## Streamlined Core Types

### Simplified Engine Interface
```rust
pub struct QuillEngine {
    config: EngineConfig,
    template_cache: Arc<TemplateCache>,
    backend_registry: BackendRegistry,
}

impl QuillEngine {
    pub fn new(config: EngineConfig) -> Self { /* */ }
    pub fn default() -> Self { Self::new(EngineConfig::default()) }
    
    pub fn render(&self, markdown: &str) -> RenderBuilder<'_> { /* */ }
    pub fn render_file<P: AsRef<Path>>(&self, path: P) -> RenderBuilder<'_> { /* */ }
}
```

### Fluent Builder API
```rust
pub struct RenderBuilder<'a> {
    engine: &'a QuillEngine,
    markdown: String,
    template: Option<&'a str>,
    variables: HashMap<String, Value>,
}

impl<'a> RenderBuilder<'a> {
    pub fn with_template(mut self, template: &'a str) -> Self { /* */ }
    pub fn with_var(mut self, key: &str, value: impl Into<Value>) -> Self { /* */ }
    pub fn with_vars(mut self, vars: HashMap<String, Value>) -> Self { /* */ }
    
    pub fn to_pdf(self) -> Result<Vec<u8>> { /* */ }
    pub fn to_svg(self) -> Result<Vec<String>> { /* */ }
    pub fn to_html(self) -> Result<String> { /* */ }
    
    pub fn build(self) -> Result<Vec<Artifact>> { /* */ }
    pub fn save_to<P: AsRef<Path>>(self, path: P) -> Result<PathBuf> { /* */ }
}
```

### Automatic Resource Management
```rust
pub struct Template {
    name: String,
    content: String,
    backend: Backend,
    assets: AssetBundle,     // Automatically loaded and cached
    metadata: TemplateInfo,  // Auto-parsed from template comments or metadata file
}

pub struct AssetBundle {
    fonts: FontCollection,   // Auto-discovered and loaded
    images: ImageCollection, // Auto-discovered and cached
    packages: PackageRegistry, // Auto-discovered and validated
}
```

## Production-Ready Features

### Built-in Caching
- Template compilation cache (in-memory and disk)
- Asset loading cache with invalidation
- Rendered output cache with smart cache keys

### Error Handling with Context
```rust
#[derive(thiserror::Error, Debug)]
pub enum QuillError {
    #[error("Template '{template}' not found. Searched in: {search_paths:?}")]
    TemplateNotFound {
        template: String,
        search_paths: Vec<PathBuf>,
    },
    
    #[error("Compilation failed in template '{template}' at line {line}")]
    CompilationFailed {
        template: String,
        line: usize,
        context: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    
    #[error("Backend '{backend}' does not support format '{format}'")]
    UnsupportedFormat { backend: String, format: String },
}
```

### Smart Defaults and Auto-Configuration
```rust
impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            template_dirs: vec![
                PathBuf::from("./templates"),
                dirs::config_dir().unwrap_or_default().join("quillmark/templates"),
            ],
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("./target/quillmark-cache")),
            cache_enabled: true,
            backend: Backend::Auto,
            output_dir: PathBuf::from("./output"),
            auto_create_dirs: true,
            concurrent_rendering: true,
            max_cache_size: 100 * 1024 * 1024, // 100MB
        }
    }
}
```

## Seamless Backend System

### Backend Trait Simplified
```rust
pub trait BackendImpl: Send + Sync {
    fn id(&self) -> &'static str;
    fn supported_formats(&self) -> &'static [OutputFormat];
    fn file_extensions(&self) -> &'static [&'static str];
    
    // Simplified: no manual filter registration
    fn render(&self, template: &CompiledTemplate, context: &RenderContext) -> Result<Vec<Artifact>>;
    
    // Built-in asset resolution
    fn resolve_asset(&self, path: &str, template: &Template) -> Result<Vec<u8>>;
}
```

### Auto-Discovery and Registration
```rust
pub struct BackendRegistry {
    backends: HashMap<&'static str, Box<dyn BackendImpl>>,
    format_map: HashMap<OutputFormat, &'static str>,
}

impl BackendRegistry {
    pub fn auto_detect() -> Self {
        let mut registry = Self::new();
        
        // Auto-register available backends at compile time
        #[cfg(feature = "typst")]
        registry.register(Box::new(TypstBackend::new()));
        
        #[cfg(feature = "latex")]
        registry.register(Box::new(LaTeXBackend::new()));
        
        registry
    }
    
    pub fn best_backend_for(&self, format: OutputFormat) -> Option<&dyn BackendImpl> {
        // Smart backend selection based on format and availability
        self.format_map.get(&format)
            .and_then(|id| self.backends.get(id))
            .map(|b| b.as_ref())
    }
}
```

### Template Compilation Pipeline
```rust
pub struct CompiledTemplate {
    source: String,
    variables: HashSet<String>,     // Auto-extracted from template
    required_assets: Vec<String>,   // Auto-discovered asset references
    backend: Backend,
    compiled_at: SystemTime,
    hash: u64,                      // For cache invalidation
}

impl CompiledTemplate {
    pub fn compile(template: &Template, backend: Backend) -> Result<Self> {
        // Auto-parse template to extract variables and asset references
        // Cache compiled templates for performance
    }
}
```

## Developer Experience Optimizations

### Intelligent Defaults
- **Backend Selection**: Automatically choose best backend for desired output format
- **Template Discovery**: Smart search in conventional locations
- **Asset Loading**: Automatic discovery and caching of fonts, images, packages
- **Output Naming**: Sensible output file names based on input and template

### Zero-Configuration Getting Started
```rust
// Create a new document - no setup required
let engine = QuillEngine::default();

// Render with built-in template
let pdf = engine
    .render("# My Document\nContent here...")
    .to_pdf()?;

// Save with auto-generated name
std::fs::write("document.pdf", pdf)?;
```

### Development-Friendly Features
- **Hot Reload**: Template changes auto-detected in development mode
- **Debug Output**: Rich error messages with template context and suggestions
- **Template Validation**: Lint templates for common issues
- **Asset Optimization**: Auto-compress and optimize images and fonts

## Performance Optimizations

### Async-First Architecture
```rust
impl QuillEngine {
    pub async fn render_async(&self, markdown: &str) -> RenderBuilder<'_> { /* */ }
    
    pub async fn render_batch<I>(&self, inputs: I) -> Result<Vec<Artifact>>
    where
        I: IntoIterator<Item = BatchInput>,
    { /* */ }
}
```

### Resource Pooling
- Connection pooling for external services
- Template compilation worker pool
- Asset loading with concurrent fetching
- Memory-mapped file access for large assets

### Smart Caching Strategy
- LRU cache for compiled templates
- Content-addressed caching for assets
- Incremental rendering for unchanged content
- Parallel cache warming

## Migration and Compatibility

### Backward Compatibility Bridge
```rust
// Legacy API support
pub mod legacy {
    pub fn render(markdown: &str, config: &RenderConfig) -> RenderResult {
        // Bridge to new engine
        let engine = QuillEngine::from_legacy_config(config);
        engine.render(markdown).build()
    }
}
```

### Migration Path
1. **Phase 1**: Add new ergonomic API alongside existing API
2. **Phase 2**: Deprecate complex configuration patterns
3. **Phase 3**: Remove deprecated APIs in next major version

## Template System Evolution

### Enhanced Template Format
```typst
// template.typ with metadata
//! quill-template: report-v1
//! description: Professional report template
//! variables: title, author, date, content
//! assets: company-logo.png, fonts/main.ttf

#import "@quill/base" as base

#show: base.document.with(
  title: "{{ title }}",
  author: "{{ author }}",
  date: {{ date }},
)

// Auto-injected content
{{ content }}
```

### Built-in Template Library
- **Minimal**: Basic document with sensible typography
- **Article**: Academic paper format
- **Report**: Business report with cover page
- **Presentation**: Slide deck template
- **Letter**: Formal correspondence

## Extension Points (When Needed)

### Custom Backend Integration
```rust
pub struct MyBackend;

impl BackendImpl for MyBackend {
    fn id(&self) -> &'static str { "my-backend" }
    // ... implement trait
}

// Register with engine
let mut engine = QuillEngine::default();
engine.register_backend(Box::new(MyBackend));
```

### Plugin Architecture
```rust
pub trait QuillPlugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn setup(&self, engine: &mut QuillEngine) -> Result<()>;
}

// Load plugins
engine.load_plugin(Box::new(MyPlugin))?;
```

## Summary

This improved architecture prioritizes:

1. **Developer Ergonomics**: One-line rendering, fluent builders, smart defaults
2. **Production Readiness**: Built-in caching, error handling, performance optimizations  
3. **Convention over Configuration**: Standard paths, auto-discovery, sensible defaults
4. **Seamless Modularity**: Backends integrate transparently with minimal ceremony
5. **Zero-Friction Onboarding**: Get started immediately without complex setup

The result is a markdown rendering engine that feels like a modern Rust library - powerful when you need it, invisible when you don't.