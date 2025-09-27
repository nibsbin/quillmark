use quillmark_core::{Backend, OutputFormat, Options, RenderError, Artifact};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use serde::Deserialize;
pub use convert::mark_to_typst;

mod compiler;
mod convert;

/// Configuration for a quill template
#[derive(Debug, Clone, Deserialize)]
pub struct Quill {
    /// Path to the quill template directory
    pub path: PathBuf,
    /// Name of the quill template
    pub name: String,
    /// Main Typst file (usually "main.typ")
    pub main_file: String,
}

impl Quill {
    /// Create a new Quill from a template directory
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.as_ref();
        let name = path.file_name()
            .ok_or("Invalid quill path")?
            .to_string_lossy()
            .to_string();
        
        // Check if main.typ exists
        let main_file = "main.typ".to_string();
        let main_path = path.join(&main_file);
        if !main_path.exists() {
            return Err(format!("main.typ not found in quill: {}", path.display()).into());
        }

        Ok(Quill {
            path: path.to_path_buf(),
            name,
            main_file,
        })
    }

    /// Get the path to the main Typst file
    pub fn main_path(&self) -> PathBuf {
        self.path.join(&self.main_file)
    }

    /// Get the path to the packages directory
    pub fn packages_path(&self) -> PathBuf {
        self.path.join("packages")
    }

    /// Get the path to the assets directory
    pub fn assets_path(&self) -> PathBuf {
        self.path.join("assets")
    }

    /// Check if the quill template is valid
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.path.exists() {
            return Err(format!("Quill directory does not exist: {}", self.path.display()).into());
        }

        if !self.main_path().exists() {
            return Err(format!("Main file does not exist: {}", self.main_path().display()).into());
        }

        Ok(())
    }

    /// Load all fonts from the quill's assets directory
    pub fn load_fonts(&self) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        let assets_path = self.assets_path();
        let mut fonts = Vec::new();

        if assets_path.exists() {
            for entry in fs::read_dir(&assets_path)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if matches!(ext.to_str(), Some("ttf") | Some("otf")) {
                            let font_data = fs::read(&path)?;
                            fonts.push(font_data);
                        }
                    }
                }
            }
        }

        Ok(fonts)
    }
}

/// Typst backend implementation using Typst with dynamic quill loading
pub struct TypstBackend {
    /// Available quill templates
    quills: HashMap<String, Quill>,
}

impl TypstBackend {
    /// Create a new TypstBackend
    pub fn new() -> Self {
        Self {
            quills: HashMap::new(),
        }
    }

    /// Register a quill template
    pub fn register_quill(&mut self, quill: Quill) -> Result<(), Box<dyn std::error::Error>> {
        quill.validate()?;
        self.quills.insert(quill.name.clone(), quill);
        Ok(())
    }

    /// Get a registered quill by name
    pub fn get_quill(&self, name: &str) -> Option<&Quill> {
        self.quills.get(name)
    }

    /// Create a TypstBackend with a specific quill loaded
    pub fn with_quill<P: AsRef<Path>>(quill_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut backend = Self::new();
        let quill = Quill::from_path(quill_path)?;
        backend.register_quill(quill)?;
        Ok(backend)
    }
}

impl Backend for TypstBackend {
    fn id(&self) -> &'static str {
        "typst"
    }

    fn supported_formats(&self) -> &'static [OutputFormat] {
        // Typst can output PDF and SVG
        &[OutputFormat::Pdf, OutputFormat::Svg]
    }

    fn render(&self, markdown: &str, opts: &Options) -> Result<Vec<Artifact>, RenderError> {
        // For now, we'll use a simple approach where we expect a quill to be registered
        // In a more sophisticated implementation, this could be specified in opts
        if self.quills.is_empty() {
            return Err(RenderError::Other(
                "No quill templates registered. Use TypstBackend::with_quill() or register_quill()".to_string().into()
            ));
        }

        // Use the first available quill for now
        let quill = self.quills.values().next().unwrap();
        
        // Convert markdown to Typst using the conversion logic
        let typst_content = mark_to_typst(markdown);
        
        let format = opts.format.unwrap_or(OutputFormat::Pdf);
        
        match format {
            OutputFormat::Pdf => {
                let pdf_bytes = compiler::compile_to_pdf(quill, &typst_content)
                    .map_err(|e| RenderError::Other(format!("PDF compilation failed: {}", e).into()))?;
                
                Ok(vec![Artifact {
                    bytes: pdf_bytes,
                    output_format: OutputFormat::Pdf,
                }])
            }
            OutputFormat::Svg => {
                let svg_pages = compiler::compile_to_svg(quill, &typst_content)
                    .map_err(|e| RenderError::Other(format!("SVG compilation failed: {}", e).into()))?;
                
                Ok(svg_pages.into_iter().map(|bytes| Artifact {
                    bytes,
                    output_format: OutputFormat::Svg,
                }).collect())
            }
            OutputFormat::Txt => {
                Err(RenderError::FormatNotSupported {
                    backend: self.id().to_string(),
                    format: OutputFormat::Txt,
                })
            }
        }
    }
}

impl Default for TypstBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_quill() -> Result<(TempDir, PathBuf), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let quill_path = temp_dir.path().join("test-quill");
        
        fs::create_dir_all(&quill_path)?;
        fs::create_dir_all(quill_path.join("packages"))?;
        fs::create_dir_all(quill_path.join("assets"))?;
        
        // Create a simple main.typ
        fs::write(
            quill_path.join("main.typ"),
            r#"#set page(width: 8.5in, height: 11in, margin: 1in)
#set text(font: "Times New Roman", size: 12pt)

= Test Document

This is a test document with markdown content: $content$

== Features
- Simple typography
- Basic layout
- Content placeholder
"#,
        )?;
        
        Ok((temp_dir, quill_path))
    }

    #[test]
    fn test_quill_creation() -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, quill_path) = create_test_quill()?;
        
        let quill = Quill::from_path(&quill_path)?;
        assert_eq!(quill.name, "test-quill");
        assert_eq!(quill.main_file, "main.typ");
        assert!(quill.main_path().exists());
        
        quill.validate()?;
        Ok(())
    }

    #[test]
    fn test_quill_paths() -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, quill_path) = create_test_quill()?;
        let quill = Quill::from_path(&quill_path)?;
        
        assert!(quill.main_path().ends_with("main.typ"));
        assert!(quill.packages_path().ends_with("packages"));
        assert!(quill.assets_path().ends_with("assets"));
        
        Ok(())
    }

    #[test]
    fn test_typst_backend_basic() {
        let backend = TypstBackend::default();
        assert_eq!(backend.id(), "typst");
        
        let formats = backend.supported_formats();
        assert!(formats.contains(&OutputFormat::Pdf));
        assert!(formats.contains(&OutputFormat::Svg));
        assert!(!formats.contains(&OutputFormat::Txt));
    }

    #[test] 
    fn test_typst_backend_with_quill() -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, quill_path) = create_test_quill()?;
        
        let backend = TypstBackend::with_quill(&quill_path)?;
        assert_eq!(backend.quills.len(), 1);
        assert!(backend.get_quill("test-quill").is_some());
        
        Ok(())
    }

    #[test]
    fn test_backend_render_no_quills() {
        let backend = TypstBackend::default();
        let options = Options {
            backend: Some("typst".to_string()),
            format: Some(OutputFormat::Pdf),
        };
    }
}
