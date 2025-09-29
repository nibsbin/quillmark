// Placeholder for typst backend
// Will be implemented after basic structure is working

pub struct TypstBackend;

impl quillmark_core::Backend for TypstBackend {
    fn id(&self) -> &'static str {
        "typst"
    }
    
    fn supported_formats(&self) -> &'static [quillmark_core::OutputFormat] {
        &[quillmark_core::OutputFormat::Pdf, quillmark_core::OutputFormat::Svg]
    }
    
    fn glue_type(&self) -> &'static str {
        ".typ"
    }
    
    fn register_filters(&self, _glue: &mut quillmark_core::Glue) {
        // TODO: Implement filters
    }
    
    fn compile(
        &self, 
        _glue_content: &str, 
        _quill: &quillmark_core::Quill, 
        _opts: &quillmark_core::RenderOptions
    ) -> Result<Vec<quillmark_core::Artifact>, quillmark_core::RenderError> {
        // TODO: Implement compilation
        Err(quillmark_core::RenderError::UnsupportedBackend("typst".to_string()))
    }
}