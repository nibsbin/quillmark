//! # Error Handling
//!
//! Structured error handling with diagnostics and source location tracking.
//!
//! ## Overview
//!
//! The `error` module provides error types and diagnostic types for actionable
//! error reporting with source location tracking.
//!
//! ## Key Types
//!
//! - [`RenderError`]: Main error enum for rendering operations
//! - [`crate::TemplateError`]: Template-specific errors
//! - [`Diagnostic`]: Structured diagnostic information
//! - [`Location`]: Source file location (file, line, column)
//! - [`Severity`]: Error severity levels (Error, Warning, Note)
//! - [`RenderResult`]: Result type with artifacts and warnings
//!
//! ## Error Hierarchy
//!
//! ### RenderError Variants
//!
//! - [`RenderError::EngineCreation`]: Failed to create rendering engine
//! - [`RenderError::InvalidFrontmatter`]: Malformed YAML frontmatter
//! - [`RenderError::TemplateFailed`]: Template rendering error
//! - [`RenderError::CompilationFailed`]: Backend compilation errors
//! - [`RenderError::FormatNotSupported`]: Requested format not supported
//! - [`RenderError::UnsupportedBackend`]: Backend not registered
//! - [`RenderError::DynamicAssetCollision`]: Asset filename collision
//! - [`RenderError::DynamicFontCollision`]: Font filename collision
//! - [`RenderError::InputTooLarge`]: Input size limits exceeded
//! - [`RenderError::YamlTooLarge`]: YAML size exceeded maximum
//! - [`RenderError::NestingTooDeep`]: Nesting depth exceeded maximum
//! - [`RenderError::OutputTooLarge`]: Template output exceeded maximum size
//!
//! ## Examples
//!
//! ### Error Handling
//!
//! ```no_run
//! use quillmark_core::{RenderError, error::print_errors};
//! # use quillmark_core::{RenderResult, OutputFormat};
//! # struct Workflow;
//! # impl Workflow {
//! #     fn render(&self, _: &str, _: Option<()>) -> Result<RenderResult, RenderError> {
//! #         Ok(RenderResult::new(vec![], OutputFormat::Pdf))
//! #     }
//! # }
//! # let workflow = Workflow;
//! # let markdown = "";
//!
//! match workflow.render(markdown, None) {
//!     Ok(result) => {
//!         // Process artifacts
//!         for artifact in result.artifacts {
//!             std::fs::write(
//!                 format!("output.{:?}", artifact.output_format),
//!                 &artifact.bytes
//!             )?;
//!         }
//!     }
//!     Err(e) => {
//!         // Print structured diagnostics
//!         print_errors(&e);
//!         
//!         // Match specific error types
//!         match e {
//!             RenderError::CompilationFailed { diags } => {
//!                 eprintln!("Compilation failed with {} errors:", diags.len());
//!                 for diag in diags {
//!                     eprintln!("{}", diag.fmt_pretty());
//!                 }
//!             }
//!             RenderError::InvalidFrontmatter { diag } => {
//!                 eprintln!("Frontmatter error: {}", diag.message);
//!             }
//!             _ => eprintln!("Error: {}", e),
//!         }
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Creating Diagnostics
//!
//! ```
//! use quillmark_core::{Diagnostic, Location, Severity};
//!
//! let diag = Diagnostic::new(Severity::Error, "Undefined variable".to_string())
//!     .with_code("E001".to_string())
//!     .with_location(Location {
//!         file: "template.typ".to_string(),
//!         line: 10,
//!         col: 5,
//!     })
//!     .with_hint("Check variable spelling".to_string());
//!
//! println!("{}", diag.fmt_pretty());
//! ```
//!
//! Example output:
//! ```text
//! [ERROR] Undefined variable (E001) at template.typ:10:5
//!   hint: Check variable spelling
//! ```
//!
//! ### Result with Warnings
//!
//! ```no_run
//! # use quillmark_core::{RenderResult, Diagnostic, Severity, OutputFormat};
//! # let artifacts = vec![];
//! let result = RenderResult::new(artifacts, OutputFormat::Pdf)
//!     .with_warning(Diagnostic::new(
//!         Severity::Warning,
//!         "Deprecated field used".to_string(),
//!     ));
//! ```
//!
//! ## Pretty Printing
//!
//! The [`Diagnostic`] type provides [`Diagnostic::fmt_pretty()`] for human-readable output with error code, location, and hints.
//!
//! ## Machine-Readable Output
//!
//! All diagnostic types implement `serde::Serialize` for JSON export:
//!
//! ```no_run
//! # use quillmark_core::{Diagnostic, Severity};
//! # let diagnostic = Diagnostic::new(Severity::Error, "Test".to_string());
//! let json = serde_json::to_string(&diagnostic).unwrap();
//! ```

use crate::OutputFormat;

/// Maximum input size for markdown (10 MB)
pub const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024;

/// Maximum YAML size (1 MB)
pub const MAX_YAML_SIZE: usize = 1024 * 1024;

/// Maximum nesting depth for markdown structures (100 levels)
pub const MAX_NESTING_DEPTH: usize = 100;

/// Maximum template output size (50 MB)
pub const MAX_TEMPLATE_OUTPUT: usize = 50 * 1024 * 1024;

/// Maximum YAML nesting depth (100 levels)
/// Prevents stack overflow from deeply nested YAML structures
pub const MAX_YAML_DEPTH: usize = 100;

/// Maximum number of CARD blocks allowed per document
/// Prevents memory exhaustion from documents with excessive card blocks
pub const MAX_CARD_COUNT: usize = 1000;

/// Maximum number of fields allowed per document
/// Prevents memory exhaustion from documents with excessive fields
pub const MAX_FIELD_COUNT: usize = 1000;

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    /// Fatal error that prevents completion
    Error,
    /// Non-fatal issue that may need attention
    Warning,
    /// Informational message
    Note,
}

/// Location information for diagnostics
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Location {
    /// Source file name (e.g., "plate.typ", "template.typ", "input.md")
    pub file: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub col: u32,
}

/// Structured diagnostic information
#[derive(Debug, serde::Serialize)]
pub struct Diagnostic {
    /// Error severity level
    pub severity: Severity,
    /// Optional error code (e.g., "E001", "typst::syntax")
    pub code: Option<String>,
    /// Human-readable error message
    pub message: String,
    /// Primary source location
    pub primary: Option<Location>,
    /// Optional hint for fixing the error
    pub hint: Option<String>,
    /// Source error that caused this diagnostic (for error chaining)
    /// Note: This field is excluded from serialization as Error trait
    /// objects cannot be serialized
    #[serde(skip)]
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Diagnostic {
    /// Create a new diagnostic
    pub fn new(severity: Severity, message: String) -> Self {
        Self {
            severity,
            code: None,
            message,
            primary: None,
            hint: None,
            source: None,
        }
    }

    /// Set the error code
    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }

    /// Set the primary location
    pub fn with_location(mut self, location: Location) -> Self {
        self.primary = Some(location);
        self
    }

    /// Set a hint
    pub fn with_hint(mut self, hint: String) -> Self {
        self.hint = Some(hint);
        self
    }

    /// Set error source (chainable)
    pub fn with_source(mut self, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        self.source = Some(source);
        self
    }

    /// Get the source chain as a list of error messages
    pub fn source_chain(&self) -> Vec<String> {
        let mut chain = Vec::new();
        let mut current_source = self
            .source
            .as_ref()
            .map(|b| b.as_ref() as &dyn std::error::Error);
        while let Some(err) = current_source {
            chain.push(err.to_string());
            current_source = err.source();
        }
        chain
    }

    /// Format diagnostic for pretty printing
    pub fn fmt_pretty(&self) -> String {
        let mut result = format!(
            "[{}] {}",
            match self.severity {
                Severity::Error => "ERROR",
                Severity::Warning => "WARN",
                Severity::Note => "NOTE",
            },
            self.message
        );

        if let Some(ref code) = self.code {
            result.push_str(&format!(" ({})", code));
        }

        if let Some(ref loc) = self.primary {
            result.push_str(&format!("\n  --> {}:{}:{}", loc.file, loc.line, loc.col));
        }

        if let Some(ref hint) = self.hint {
            result.push_str(&format!("\n  hint: {}", hint));
        }

        result
    }

    /// Format diagnostic with source chain for debugging
    pub fn fmt_pretty_with_source(&self) -> String {
        let mut result = self.fmt_pretty();

        for (i, cause) in self.source_chain().iter().enumerate() {
            result.push_str(&format!("\n  cause {}: {}", i + 1, cause));
        }

        result
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Serializable diagnostic for cross-language boundaries
///
/// This type is used when diagnostics need to be serialized and sent across
/// FFI boundaries (e.g., Python, WASM). Unlike `Diagnostic`, it does not
/// contain the non-serializable `source` field, but instead includes a
/// flattened `source_chain` for display purposes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SerializableDiagnostic {
    /// Error severity level
    pub severity: Severity,
    /// Optional error code (e.g., "E001", "typst::syntax")
    pub code: Option<String>,
    /// Human-readable error message
    pub message: String,
    /// Primary source location
    pub primary: Option<Location>,
    /// Optional hint for fixing the error
    pub hint: Option<String>,
    /// Source chain as list of strings (for display purposes)
    pub source_chain: Vec<String>,
}

impl From<Diagnostic> for SerializableDiagnostic {
    fn from(diag: Diagnostic) -> Self {
        let source_chain = diag.source_chain();
        Self {
            severity: diag.severity,
            code: diag.code,
            message: diag.message,
            primary: diag.primary,
            hint: diag.hint,
            source_chain,
        }
    }
}

impl From<&Diagnostic> for SerializableDiagnostic {
    fn from(diag: &Diagnostic) -> Self {
        Self {
            severity: diag.severity,
            code: diag.code.clone(),
            message: diag.message.clone(),
            primary: diag.primary.clone(),
            hint: diag.hint.clone(),
            source_chain: diag.source_chain(),
        }
    }
}

/// Error type for parsing operations
#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    /// Input too large
    #[error("Input too large: {size} bytes (max: {max} bytes)")]
    InputTooLarge {
        /// Actual size
        size: usize,
        /// Maximum allowed size
        max: usize,
    },

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    YamlError(#[from] serde_saphyr::Error),

    /// JSON parsing/conversion error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Invalid YAML structure
    #[error("Invalid YAML structure: {0}")]
    InvalidStructure(String),

    /// Missing CARD directive in inline metadata block
    #[error("{}", .diag.message)]
    MissingCardDirective {
        /// Diagnostic information with hint
        diag: Box<Diagnostic>,
    },

    /// YAML parsing error with location context
    #[error("YAML error at line {line}: {message}")]
    YamlErrorWithLocation {
        /// Error message
        message: String,
        /// Line number in the source document (1-indexed)
        line: usize,
        /// Index of the metadata block (0-indexed)
        block_index: usize,
    },

    /// Other parsing errors
    #[error("{0}")]
    Other(String),
}

impl ParseError {
    /// Create a MissingCardDirective error with helpful hint
    pub fn missing_card_directive() -> Self {
        let diag = Diagnostic::new(
            Severity::Error,
            "Inline metadata block missing CARD directive".to_string(),
        )
        .with_code("parse::missing_card".to_string())
        .with_hint(
            "Add 'CARD: <card_type>' to specify which card this block belongs to. \
            Example:\n---\nCARD: my_card_type\nfield: value\n---"
                .to_string(),
        );
        ParseError::MissingCardDirective {
            diag: Box::new(diag),
        }
    }

    /// Convert the parse error into a structured diagnostic
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            ParseError::MissingCardDirective { diag } => Diagnostic {
                severity: diag.severity,
                code: diag.code.clone(),
                message: diag.message.clone(),
                primary: diag.primary.clone(),
                hint: diag.hint.clone(),
                source: None, // Cannot clone trait object, but it's empty in this case usually
            },
            ParseError::InputTooLarge { size, max } => Diagnostic::new(
                Severity::Error,
                format!("Input too large: {} bytes (max: {} bytes)", size, max),
            )
            .with_code("parse::input_too_large".to_string()),
            ParseError::YamlError(e) => {
                Diagnostic::new(Severity::Error, format!("YAML parsing error: {}", e))
                    .with_code("parse::yaml_error".to_string())
            } // serde_saphyr::Error implements Error+Clone? No, usually Error is not Clone.
            ParseError::JsonError(e) => {
                Diagnostic::new(Severity::Error, format!("JSON conversion error: {}", e))
                    .with_code("parse::json_error".to_string())
            }
            ParseError::InvalidStructure(msg) => Diagnostic::new(Severity::Error, msg.clone())
                .with_code("parse::invalid_structure".to_string()),
            ParseError::YamlErrorWithLocation {
                message,
                line,
                block_index,
            } => Diagnostic::new(
                Severity::Error,
                format!(
                    "YAML error at line {} (block {}): {}",
                    line, block_index, message
                ),
            )
            .with_code("parse::yaml_error_with_location".to_string()),
            ParseError::Other(msg) => Diagnostic::new(Severity::Error, msg.clone()),
        }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for ParseError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        ParseError::Other(err.to_string())
    }
}

impl From<String> for ParseError {
    fn from(msg: String) -> Self {
        ParseError::Other(msg)
    }
}

impl From<&str> for ParseError {
    fn from(msg: &str) -> Self {
        ParseError::Other(msg.to_string())
    }
}

/// Main error type for rendering operations
#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    /// Failed to create rendering engine
    #[error("{diag}")]
    EngineCreation {
        /// Diagnostic information
        diag: Box<Diagnostic>,
    },

    /// Invalid YAML frontmatter in markdown document
    #[error("{diag}")]
    InvalidFrontmatter {
        /// Diagnostic information
        diag: Box<Diagnostic>,
    },

    /// Template rendering failed
    #[error("{diag}")]
    TemplateFailed {
        /// Diagnostic information
        diag: Box<Diagnostic>,
    },

    /// Backend compilation failed with one or more errors
    #[error("Backend compilation failed with {} error(s)", diags.len())]
    CompilationFailed {
        /// List of diagnostics
        diags: Vec<Diagnostic>,
    },

    /// Requested output format not supported by backend
    #[error("{diag}")]
    FormatNotSupported {
        /// Diagnostic information
        diag: Box<Diagnostic>,
    },

    /// Backend not registered with engine
    #[error("{diag}")]
    UnsupportedBackend {
        /// Diagnostic information
        diag: Box<Diagnostic>,
    },

    /// Dynamic asset filename collision
    #[error("{diag}")]
    DynamicAssetCollision {
        /// Diagnostic information
        diag: Box<Diagnostic>,
    },

    /// Dynamic font filename collision
    #[error("{diag}")]
    DynamicFontCollision {
        /// Diagnostic information
        diag: Box<Diagnostic>,
    },

    /// Input size limits exceeded
    #[error("{diag}")]
    InputTooLarge {
        /// Diagnostic information
        diag: Box<Diagnostic>,
    },

    /// YAML size exceeded maximum allowed
    #[error("{diag}")]
    YamlTooLarge {
        /// Diagnostic information
        diag: Box<Diagnostic>,
    },

    /// Nesting depth exceeded maximum allowed
    #[error("{diag}")]
    NestingTooDeep {
        /// Diagnostic information
        diag: Box<Diagnostic>,
    },

    /// Template output exceeded maximum size
    #[error("{diag}")]
    OutputTooLarge {
        /// Diagnostic information
        diag: Box<Diagnostic>,
    },

    /// Validation failed for parsed document
    #[error("{diag}")]
    ValidationFailed {
        /// Diagnostic information
        diag: Box<Diagnostic>,
    },

    /// Invalid schema definition
    #[error("{diag}")]
    InvalidSchema {
        /// Diagnostic information
        diag: Box<Diagnostic>,
    },

    /// Quill configuration error
    #[error("{diag}")]
    QuillConfig {
        /// Diagnostic information
        diag: Box<Diagnostic>,
    },
}

impl RenderError {
    /// Extract all diagnostics from this error
    pub fn diagnostics(&self) -> Vec<&Diagnostic> {
        match self {
            RenderError::CompilationFailed { diags } => diags.iter().collect(),
            RenderError::EngineCreation { diag }
            | RenderError::InvalidFrontmatter { diag }
            | RenderError::TemplateFailed { diag }
            | RenderError::FormatNotSupported { diag }
            | RenderError::UnsupportedBackend { diag }
            | RenderError::DynamicAssetCollision { diag }
            | RenderError::DynamicFontCollision { diag }
            | RenderError::InputTooLarge { diag }
            | RenderError::YamlTooLarge { diag }
            | RenderError::NestingTooDeep { diag }
            | RenderError::OutputTooLarge { diag }
            | RenderError::ValidationFailed { diag }
            | RenderError::InvalidSchema { diag }
            | RenderError::QuillConfig { diag } => vec![diag.as_ref()],
        }
    }
}

/// Result type containing artifacts and warnings
#[derive(Debug)]
pub struct RenderResult {
    /// Generated output artifacts
    pub artifacts: Vec<crate::Artifact>,
    /// Non-fatal diagnostic messages
    pub warnings: Vec<Diagnostic>,
    /// Output format that was produced
    pub output_format: OutputFormat,
}

impl RenderResult {
    /// Create a new result with artifacts and output format
    pub fn new(artifacts: Vec<crate::Artifact>, output_format: OutputFormat) -> Self {
        Self {
            artifacts,
            warnings: Vec::new(),
            output_format,
        }
    }

    /// Add a warning to the result
    pub fn with_warning(mut self, warning: Diagnostic) -> Self {
        self.warnings.push(warning);
        self
    }
}

/// Convert minijinja errors to RenderError
impl From<minijinja::Error> for RenderError {
    fn from(e: minijinja::Error) -> Self {
        // Extract location with proper range information
        let loc = e.line().map(|line| Location {
            file: e.name().unwrap_or("template").to_string(),
            line: line as u32,
            // MiniJinja provides range, extract approximate column
            col: e.range().map(|r| r.start as u32).unwrap_or(0),
        });

        // Generate helpful hints based on error kind
        let hint = generate_minijinja_hint(&e);

        // Create diagnostic with source preservation
        let mut diag = Diagnostic::new(Severity::Error, e.to_string())
            .with_code(format!("minijinja::{:?}", e.kind()));

        if let Some(loc) = loc {
            diag = diag.with_location(loc);
        }

        if let Some(hint) = hint {
            diag = diag.with_hint(hint);
        }

        // Preserve the original error as source
        diag = diag.with_source(Box::new(e));

        RenderError::TemplateFailed {
            diag: Box::new(diag),
        }
    }
}

/// Generate helpful hints for common MiniJinja errors
fn generate_minijinja_hint(e: &minijinja::Error) -> Option<String> {
    use minijinja::ErrorKind;

    match e.kind() {
        ErrorKind::UndefinedError => {
            Some("Check variable spelling and ensure it's defined in frontmatter".to_string())
        }
        ErrorKind::InvalidOperation => {
            Some("Check that you're using the correct filter or operator for this type".to_string())
        }
        ErrorKind::SyntaxError => Some(
            "Check template syntax - look for unclosed tags or invalid expressions".to_string(),
        ),
        _ => e.detail().map(|d| d.to_string()),
    }
}

/// Helper to print structured errors
pub fn print_errors(err: &RenderError) {
    match err {
        RenderError::CompilationFailed { diags } => {
            for d in diags {
                eprintln!("{}", d.fmt_pretty());
            }
        }
        RenderError::TemplateFailed { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::InvalidFrontmatter { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::EngineCreation { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::FormatNotSupported { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::UnsupportedBackend { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::DynamicAssetCollision { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::DynamicFontCollision { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::InputTooLarge { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::YamlTooLarge { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::NestingTooDeep { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::OutputTooLarge { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::ValidationFailed { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::InvalidSchema { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::QuillConfig { diag } => eprintln!("{}", diag.fmt_pretty()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_with_source_chain() {
        let root_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let diag = Diagnostic::new(Severity::Error, "Rendering failed".to_string())
            .with_source(Box::new(root_err));

        let chain = diag.source_chain();
        assert_eq!(chain.len(), 1);
        assert!(chain[0].contains("File not found"));
    }

    #[test]
    fn test_diagnostic_serialization() {
        let diag = Diagnostic::new(Severity::Error, "Test error".to_string())
            .with_code("E001".to_string())
            .with_location(Location {
                file: "test.typ".to_string(),
                line: 10,
                col: 5,
            });

        let serializable: SerializableDiagnostic = diag.into();
        let json = serde_json::to_string(&serializable).unwrap();
        assert!(json.contains("Test error"));
        assert!(json.contains("E001"));
    }

    #[test]
    fn test_render_error_diagnostics_extraction() {
        let diag1 = Diagnostic::new(Severity::Error, "Error 1".to_string());
        let diag2 = Diagnostic::new(Severity::Error, "Error 2".to_string());

        let err = RenderError::CompilationFailed {
            diags: vec![diag1, diag2],
        };

        let diags = err.diagnostics();
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn test_diagnostic_fmt_pretty() {
        let diag = Diagnostic::new(Severity::Warning, "Deprecated field used".to_string())
            .with_code("W001".to_string())
            .with_location(Location {
                file: "input.md".to_string(),
                line: 5,
                col: 10,
            })
            .with_hint("Use the new field name instead".to_string());

        let output = diag.fmt_pretty();
        assert!(output.contains("[WARN]"));
        assert!(output.contains("Deprecated field used"));
        assert!(output.contains("W001"));
        assert!(output.contains("input.md:5:10"));
        assert!(output.contains("hint:"));
    }

    #[test]
    fn test_diagnostic_fmt_pretty_with_source() {
        let root_err = std::io::Error::other("Underlying error");
        let diag = Diagnostic::new(Severity::Error, "Top-level error".to_string())
            .with_code("E002".to_string())
            .with_source(Box::new(root_err));

        let output = diag.fmt_pretty_with_source();
        assert!(output.contains("[ERROR]"));
        assert!(output.contains("Top-level error"));
        assert!(output.contains("cause 1:"));
        assert!(output.contains("Underlying error"));
    }

    #[test]
    fn test_render_result_with_warnings() {
        let artifacts = vec![];
        let warning = Diagnostic::new(Severity::Warning, "Test warning".to_string());

        let result = RenderResult::new(artifacts, OutputFormat::Pdf).with_warning(warning);

        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].message, "Test warning");
    }

    #[test]
    fn test_minijinja_error_conversion() {
        // Use undefined variable with strict mode to trigger an error
        let template_str = "{{ undefined_var }}";
        let mut env = minijinja::Environment::new();
        env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);

        let result = env.render_str(template_str, minijinja::context! {});
        assert!(
            result.is_err(),
            "Expected rendering to fail with undefined variable"
        );

        let minijinja_err = result.unwrap_err();
        let render_err: RenderError = minijinja_err.into();

        match render_err {
            RenderError::TemplateFailed { diag } => {
                assert_eq!(diag.severity, Severity::Error);
                assert!(diag.code.is_some());
                assert!(diag.hint.is_some());
                assert!(diag.source.is_some());
            }
            _ => panic!("Expected TemplateFailed error"),
        }
    }
}
