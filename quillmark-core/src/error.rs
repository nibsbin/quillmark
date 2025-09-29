use crate::OutputFormat;

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Severity { 
    Error, 
    Warning, 
    Note 
}

/// Location information for diagnostics
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Location {
    pub file: String,   // e.g., "glue.typ", "template.typ", "input.md"
    pub line: u32,
    pub col: u32,
}

/// Structured diagnostic information
#[derive(Debug, Clone, serde::Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Option<String>,
    pub message: String,
    pub primary: Option<Location>,
    pub related: Vec<Location>,
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
        let mut result = format!("[{}] {}", 
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
            result.push_str(&format!(" at {}:{}:{}", loc.file, loc.line, loc.col));
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
    #[error("Engine creation failed")] 
    EngineCreation { 
        diag: Diagnostic, 
        #[source] source: Option<anyhow::Error> 
    },

    #[error("Invalid YAML frontmatter")] 
    InvalidFrontmatter { 
        diag: Diagnostic, 
        #[source] source: Option<anyhow::Error> 
    },

    #[error("Template rendering failed")] 
    TemplateFailed { 
        #[source] source: minijinja::Error, 
        diag: Diagnostic 
    },

    #[error("Backend compilation failed with {0} error(s)")]
    CompilationFailed(usize, Vec<Diagnostic>),

    #[error("{format:?} not supported by {backend}")]
    FormatNotSupported { 
        backend: String, 
        format: OutputFormat 
    },

    #[error("Unsupported backend: {0}")]
    UnsupportedBackend(String),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),

    #[error("{0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("Template error: {0}")]
    Template(#[from] crate::templating::TemplateError),
}

/// Result type containing artifacts and warnings
#[derive(Debug)]
pub struct RenderResult {
    pub artifacts: Vec<crate::Artifact>,
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
        let loc = e.line().map(|line| Location {
            file: e.name().unwrap_or("template").to_string(),
            line: line as u32,
            col: 0, // MiniJinja doesn't provide column info
        });
        
        let diag = Diagnostic {
            severity: Severity::Error,
            code: Some(format!("minijinja::{:?}", e.kind())),
            message: e.to_string(),
            primary: loc,
            related: vec![],
            hint: None,
        };
        
        RenderError::TemplateFailed { source: e, diag }
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
        _ => eprintln!("{}", err)
    }
}