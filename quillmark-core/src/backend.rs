use crate::{Artifact, OutputFormat, Quill, RenderOptions};
use crate::error::RenderError;
use crate::templating::Glue;

/// Backend trait for rendering different output formats
pub trait Backend: Send + Sync {
    /// Get the backend identifier (e.g., "typst", "latex")
    fn id(&self) -> &'static str;
    
    /// Get supported output formats
    fn supported_formats(&self) -> &'static [OutputFormat];
    
    /// Get the glue file extension (e.g., ".typ", ".tex")
    fn glue_type(&self) -> &'static str;
    
    /// Register backend-specific filters with the glue environment
    fn register_filters(&self, glue: &mut Glue);
    
    /// Compile the glue content into final artifacts
    fn compile(
        &self, 
        glue_content: &str, 
        quill: &Quill, 
        opts: &RenderOptions
    ) -> Result<Vec<Artifact>, RenderError>;
}