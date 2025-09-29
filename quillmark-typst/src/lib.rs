use quillmark_core::{Backend, OutputFormat, RenderOptions, RenderError, Artifact, Quill, Glue};
pub use convert::mark_to_typst;
use filters::*;

mod compiler;
mod convert;
mod filters;

pub struct TypstBackend {}

impl TypstBackend {
    pub fn new() -> Self {
        Self {}
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

    fn glue_type(&self) -> &'static str {
        ".typ"
    }

    fn register_filters(&self, glue: &mut Glue) {
        glue.register_filter("String", string_filter);
        glue.register_filter("Lines", lines_filter);
        glue.register_filter("Date", datetime_filter);
        glue.register_filter("Dict", dict_filter);
        glue.register_filter("Body", body_filter);
    }

    fn compile(&self, glued_content: &str, quill: &Quill, opts: &RenderOptions) -> Result<Vec<Artifact>, RenderError> {
        let format = opts.output_format.unwrap_or(OutputFormat::Pdf);
        
        match format {
            OutputFormat::Pdf => {
                let pdf_bytes = compiler::compile_to_pdf(quill, glued_content)
                    .map_err(|e| RenderError::Other(format!("PDF compilation failed: {}", e).into()))?;
                
                Ok(vec![Artifact {
                    bytes: pdf_bytes,
                    output_format: OutputFormat::Pdf,
                }])
            }
            OutputFormat::Svg => {
                let svg_pages = compiler::compile_to_svg(quill, glued_content)
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
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_quill() -> Result<(TempDir, PathBuf), Box<dyn std::error::Error + Send + Sync>> {
        let temp_dir = TempDir::new()?;
        let quill_path = temp_dir.path().join("test-quill");
        
        fs::create_dir_all(&quill_path)?;
        fs::create_dir_all(quill_path.join("packages"))?;
        fs::create_dir_all(quill_path.join("assets"))?;
        
        // Copy some fonts from the hello-quill example for testing
        let hello_quill_assets = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../examples/hello-quill/assets");
        if hello_quill_assets.exists() {
            for entry in fs::read_dir(&hello_quill_assets)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if matches!(ext.to_string_lossy().to_lowercase().as_str(), "ttf" | "otf") {
                            let dest = quill_path.join("assets").join(entry.file_name());
                            fs::copy(&path, &dest)?;
                        }
                    }
                }
            }
        }
        
        // Create a simple glue.typ
        fs::write(
            quill_path.join("glue.typ"),
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
    fn test_quill_creation() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (_temp, quill_path) = create_test_quill()?;
        
        let quill = Quill::from_path(&quill_path)?;
        assert_eq!(quill.name, "test-quill");
        assert_eq!(quill.glue_file, "glue.typ");
        assert!(quill.glue_path().exists());
        
        quill.validate()?;
        Ok(())
    }

    #[test]
    fn test_quill_paths() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (_temp, quill_path) = create_test_quill()?;
        let quill = Quill::from_path(&quill_path)?;
        
        assert!(quill.glue_path().ends_with("glue.typ"));
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
    fn test_improved_error_visibility() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (_temp, quill_path) = create_test_quill()?;
        let quill = Quill::from_path(&quill_path)?;
        
        // Create invalid glue.typ that will cause a compilation error
        fs::write(
            quill_path.join("glue.typ"),
            r#"#set page(width: 8.5in, height: 11in, margin: 1in)
#set text(font: "Times New Roman", size: 12pt)

= Test Error

// This will cause an "unexpected argument" error
#text("arg1", "arg2")
"#,
        )?;
        
        let invalid_typst_content = r#"
// This will cause an "unexpected argument" error
#text("arg1", "arg2")
"#;
        
        // Test that the error contains improved visibility information
        match crate::compiler::compile_to_pdf(&quill, invalid_typst_content) {
            Ok(_) => panic!("Expected compilation to fail"),
            Err(e) => {
                let error_str = format!("{}", e);
                
                // Check that the error contains the improved formatting
                assert!(error_str.contains("unexpected argument"), 
                    "Error should contain the specific error message, got: {}", error_str);
                assert!(error_str.contains("Location:"), 
                    "Error should contain location information, got: {}", error_str);
                assert!(error_str.contains("line"), 
                    "Error should contain line number, got: {}", error_str);
                assert!(error_str.contains("main.typ"), 
                    "Error should contain file name, got: {}", error_str);
                assert!(error_str.contains("#text(\"arg1\", \"arg2\")"), 
                    "Error should contain the offending code line, got: {}", error_str);
            }
        }
        
        Ok(())
    }

    #[test]
    fn test_svg_error_visibility() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (_temp, quill_path) = create_test_quill()?;
        let quill = Quill::from_path(&quill_path)?;
        
        let invalid_typst_content = r#"
// This will cause an "unknown variable" error
#invalid_function()
"#;
        
        // Test that SVG compilation also gets improved error handling
        match crate::compiler::compile_to_svg(&quill, invalid_typst_content) {
            Ok(_) => panic!("Expected compilation to fail"),
            Err(e) => {
                let error_str = format!("{}", e);
                
                // Check that SVG errors also have improved visibility
                assert!(error_str.contains("unknown variable: invalid_function"), 
                    "Error should contain the specific error message, got: {}", error_str);
                assert!(error_str.contains("Location:"), 
                    "Error should contain location information, got: {}", error_str);
                assert!(error_str.contains("line"), 
                    "Error should contain line number, got: {}", error_str);
            }
        }
        
        Ok(())
    }

}
