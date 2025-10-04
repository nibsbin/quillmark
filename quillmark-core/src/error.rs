#![doc = include_str!("../docs/errors.md")]

use crate::OutputFormat;

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Severity {
    /// Fatal error that prevents completion
    Error,
    /// Non-fatal issue that may need attention
    Warning,
    /// Informational message
    Note,
}

/// Location information for diagnostics
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Location {
    /// Source file name (e.g., "glue.typ", "template.typ", "input.md")
    pub file: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub col: u32,
}

/// Structured diagnostic information
#[derive(Debug, Clone, serde::Serialize)]
pub struct Diagnostic {
    /// Error severity level
    pub severity: Severity,
    /// Optional error code (e.g., "E001", "typst::syntax")
    pub code: Option<String>,
    /// Human-readable error message
    pub message: String,
    /// Primary source location
    pub primary: Option<Location>,
    /// Related source locations for context
    pub related: Vec<Location>,
    /// Optional hint for fixing the error
    pub hint: Option<String>,
}

impl Diagnostic {
    /// Create a new diagnostic
    pub fn new(severity: Severity, message: String) -> Self {
        Self {
            severity,
            code: None,
            message,
            primary: None,
            related: Vec::new(),
            hint: None,
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

    /// Add a related location
    pub fn with_related(mut self, location: Location) -> Self {
        self.related.push(location);
        self
    }

    /// Set a hint
    pub fn with_hint(mut self, hint: String) -> Self {
        self.hint = Some(hint);
        self
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

        // Add related locations (trace)
        for (i, related) in self.related.iter().enumerate() {
            result.push_str(&format!(
                "\n  {} {}:{}:{}",
                if i == 0 { "trace:" } else { "      " },
                related.file,
                related.line,
                related.col
            ));
        }

        if let Some(ref hint) = self.hint {
            result.push_str(&format!("\n  hint: {}", hint));
        }

        result
    }
}

/// Main error type for rendering operations
#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    /// Failed to create rendering engine
    #[error("Engine creation failed")]
    EngineCreation {
        /// Diagnostic information
        diag: Diagnostic,
        #[source]
        /// Optional source error
        source: Option<anyhow::Error>,
    },

    /// Invalid YAML frontmatter in markdown document
    #[error("Invalid YAML frontmatter")]
    InvalidFrontmatter {
        /// Diagnostic information
        diag: Diagnostic,
        #[source]
        /// Optional source error
        source: Option<anyhow::Error>,
    },

    /// Template rendering failed
    #[error("Template rendering failed")]
    TemplateFailed {
        #[source]
        /// MiniJinja error
        source: minijinja::Error,
        /// Diagnostic information
        diag: Diagnostic,
    },

    /// Backend compilation failed with one or more errors
    #[error("Backend compilation failed with {0} error(s)")]
    CompilationFailed(
        /// Number of errors
        usize,
        /// List of diagnostics
        Vec<Diagnostic>,
    ),

    /// Requested output format not supported by backend
    #[error("{format:?} not supported by {backend}")]
    FormatNotSupported {
        /// Backend identifier
        backend: String,
        /// Requested format
        format: OutputFormat,
    },

    /// Backend not registered with engine
    #[error("Unsupported backend: {0}")]
    UnsupportedBackend(String),

    /// Dynamic asset filename collision
    #[error("Dynamic asset collision: {filename}")]
    DynamicAssetCollision {
        /// Filename that collided
        filename: String,
        /// Error message
        message: String,
    },

    /// Internal error (wraps anyhow::Error)
    #[error(transparent)]
    Internal(#[from] anyhow::Error),

    /// Other errors (boxed trait object)
    #[error("{0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),

    /// Template-related error
    #[error("Template error: {0}")]
    Template(#[from] crate::templating::TemplateError),
}

/// Result type containing artifacts and warnings
#[derive(Debug)]
pub struct RenderResult {
    /// Generated output artifacts
    pub artifacts: Vec<crate::Artifact>,
    /// Non-fatal diagnostic messages
    pub warnings: Vec<Diagnostic>,
}

impl RenderResult {
    /// Create a new result with artifacts
    pub fn new(artifacts: Vec<crate::Artifact>) -> Self {
        Self {
            artifacts,
            warnings: Vec::new(),
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
        let loc = e.line().map(|line| {
            Location {
                file: e.name().unwrap_or("template").to_string(),
                line: line as u32,
                // MiniJinja provides range, extract approximate column
                col: e.range().map(|r| r.start as u32).unwrap_or(0),
            }
        });

        // Generate helpful hints based on error kind
        let hint = generate_minijinja_hint(&e);

        let diag = Diagnostic {
            severity: Severity::Error,
            code: Some(format!("minijinja::{:?}", e.kind())),
            message: e.to_string(),
            primary: loc,
            related: vec![],
            hint,
        };

        RenderError::TemplateFailed { source: e, diag }
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
        ErrorKind::SyntaxError => {
            Some("Check template syntax - look for unclosed tags or invalid expressions".to_string())
        }
        _ => e.detail().map(|d| d.to_string()),
    }
}


/// Helper to print structured errors
pub fn print_errors(err: &RenderError) {
    match err {
        RenderError::CompilationFailed(_, diags) => {
            for d in diags {
                eprintln!("{}", d.fmt_pretty());
            }
        }
        RenderError::TemplateFailed { diag, .. } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::InvalidFrontmatter { diag, .. } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::EngineCreation { diag, .. } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::FormatNotSupported { backend, format } => {
            eprintln!("[ERROR] Format {:?} not supported by {} backend", format, backend);
        }
        RenderError::UnsupportedBackend(name) => {
            eprintln!("[ERROR] Unsupported backend: {}", name);
        }
        RenderError::DynamicAssetCollision { filename, message } => {
            eprintln!("[ERROR] Dynamic asset collision: {}\n  {}", filename, message);
        }
        RenderError::Internal(e) => {
            eprintln!("[ERROR] Internal error: {}", e);
        }
        RenderError::Template(e) => {
            eprintln!("[ERROR] Template error: {}", e);
        }
        RenderError::Other(e) => {
            eprintln!("[ERROR] {}", e);
        }
    }
}
