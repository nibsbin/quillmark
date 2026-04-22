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
//! - [`RenderError::CompilationFailed`]: Backend compilation errors
//! - [`RenderError::FormatNotSupported`]: Requested format not supported
//! - [`RenderError::UnsupportedBackend`]: Backend not registered
//! - [`RenderError::ValidationFailed`]: Field coercion/validation failure
//! - [`RenderError::QuillConfig`]: Quill configuration error
//!
//! ## Examples
//!
//! ### Error Handling
//!
//! ```no_run
//! use quillmark_core::{RenderError, error::print_errors};
//! # use quillmark_core::{RenderResult, OutputFormat};
//! # struct Quill;
//! # impl Quill {
//! #     fn render(&self, _: &str, _: Option<()>) -> Result<RenderResult, RenderError> {
//! #         Ok(RenderResult::new(vec![], OutputFormat::Pdf))
//! #     }
//! # }
//! # let quill = Quill;
//! # let markdown = "";
//!
//! match quill.render(markdown, None) {
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

/// Maximum YAML nesting depth (100 levels)
/// Prevents stack overflow from deeply nested YAML structures
///
/// Re-exported from [`crate::document::limits::MAX_YAML_DEPTH`].
pub use crate::document::limits::MAX_YAML_DEPTH;

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

    /// Clone this diagnostic while dropping any attached source chain.
    pub fn clone_without_source(&self) -> Self {
        Self {
            severity: self.severity,
            code: self.code.clone(),
            message: self.message.clone(),
            primary: self.primary.clone(),
            hint: self.hint.clone(),
            source: None,
        }
    }
}

impl Clone for Diagnostic {
    /// Clone a `Diagnostic`, dropping the source error chain.
    ///
    /// The `source` field holds a boxed `dyn Error` which is not `Clone`;
    /// the cloned value will have `source: None`. Use `clone_without_source()`
    /// explicitly if you want to be clear about this loss.
    fn clone(&self) -> Self {
        self.clone_without_source()
    }
}

impl PartialEq for Diagnostic {
    /// Two `Diagnostic`s are equal when all fields except `source` are equal.
    fn eq(&self, other: &Self) -> bool {
        self.severity == other.severity
            && self.code == other.code
            && self.message == other.message
            && self.primary == other.primary
            && self.hint == other.hint
    }
}

impl Diagnostic {
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
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

    /// Invalid YAML structure
    #[error("Invalid YAML structure: {0}")]
    InvalidStructure(String),

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
    /// Convert the parse error into a structured diagnostic
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            ParseError::InputTooLarge { size, max } => Diagnostic::new(
                Severity::Error,
                format!("Input too large: {} bytes (max: {} bytes)", size, max),
            )
            .with_code("parse::input_too_large".to_string()),
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

/// Main error type for rendering operations.
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

    /// Validation failed for parsed document
    #[error("{diag}")]
    ValidationFailed {
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
            | RenderError::FormatNotSupported { diag }
            | RenderError::UnsupportedBackend { diag }
            | RenderError::ValidationFailed { diag }
            | RenderError::QuillConfig { diag } => vec![diag.as_ref()],
        }
    }
}

/// Convert ParseError to RenderError
impl From<ParseError> for RenderError {
    fn from(err: ParseError) -> Self {
        RenderError::InvalidFrontmatter {
            diag: Box::new(
                Diagnostic::new(Severity::Error, err.to_string())
                    .with_code("parse::error".to_string()),
            ),
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

/// Helper to print structured errors
pub fn print_errors(err: &RenderError) {
    for d in err.diagnostics() {
        eprintln!("{}", d.fmt_pretty());
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
}
