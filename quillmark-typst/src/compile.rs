use std::collections::HashMap;
use std::path::Path;
use typst::diag::{FileError, FileResult, Warned, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath, package::PackageSpec};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst::layout::PagedDocument;
use typst_pdf::PdfOptions;

use quillmark_core::Quill;

/// Compile a quill template with Typst content to PDF
pub fn compile_to_pdf(quill: &Quill, glued_content: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    println!("Using quill: {}", quill.name);
    let world = QuillWorld::new(quill, glued_content)?;
    println!("World initialized with {} sources and {} binaries", world.sources.len(), world.binaries.len());
    let document = compile_document(&world)?;
    
    let pdf = typst_pdf::pdf(&document, &PdfOptions::default())
        .map_err(|e| format!("PDF generation failed: {:?}", e))?;

    Ok(pdf)
}

/// Compile a quill template with Typst content to SVG pages
pub fn compile_to_svg(quill: &Quill, glued_content: &str) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
    let world = QuillWorld::new(quill, glued_content)?;
    let document = compile_document(&world)?;
    
    let mut pages = Vec::new();
    for page in &document.pages {
        let svg = typst_svg::svg(page);
        pages.push(svg.into_bytes());
    }
    
    Ok(pages)
}

/// Internal compilation function
fn compile_document(world: &QuillWorld) -> Result<PagedDocument, Box<dyn std::error::Error>> {
    let Warned { output, warnings: _ } = typst::compile::<PagedDocument>(world);
    output.map_err(|errors| {
        format_compilation_errors(&errors, world).into()
    })
}

/// Format compilation errors with better visibility
fn format_compilation_errors(errors: &[SourceDiagnostic], world: &QuillWorld) -> String {
    if errors.is_empty() {
        return "Compilation failed with unknown errors".to_string();
    }
    
    let mut formatted = format!("Compilation failed with {} error(s):", errors.len());
    
    for (i, error) in errors.iter().enumerate() {
        formatted.push_str(&format!("\n\nError #{}: {}", i + 1, error.message));
        
        // Try to get line information from the span
        if let Some(line_info) = get_line_info_from_span(error.span, world) {
            formatted.push_str(&format!("\n  Location: {}", line_info));
        } else {
            formatted.push_str(&format!("\n  Span: {:?}", error.span));
        }
        
        formatted.push_str(&format!("\n  Severity: {:?}", error.severity));
        
        // Add hints if available
        if !error.hints.is_empty() {
            formatted.push_str("\n  Hints:");
            for hint in &error.hints {
                formatted.push_str(&format!("\n    - {}", hint));
            }
        }
        
        // Add trace if available
        if !error.trace.is_empty() {
            formatted.push_str("\n  Trace:");
            for trace_entry in &error.trace {
                formatted.push_str(&format!("\n    - {:?}", trace_entry));
            }
        }
    }
    
    formatted
}

/// Extract line information from a span
fn get_line_info_from_span(span: typst::syntax::Span, world: &QuillWorld) -> Option<String> {
    // Try to find the source that contains this span
    let source_id = world.main();
    if let Ok(source) = world.source(source_id) {
        if let Some(range) = source.range(span) {
            let text = source.text();
            let start_line = text[..range.start].matches('\n').count() + 1;
            let start_col = range.start - text[..range.start].rfind('\n').map_or(0, |pos| pos + 1) + 1;
            
            // Get the actual line content
            let lines: Vec<&str> = text.lines().collect();
            let line_content = lines.get(start_line - 1).unwrap_or(&"<line not found>");
            
            return Some(format!("line {}, column {} in file '{}'\n    {}", 
                start_line, start_col, source.id().vpath().as_rootless_path().display(), line_content));
        }
    }
    
    // Also check other sources in the world
    for (&file_id, _) in &world.sources {
        if let Ok(source) = world.source(file_id) {
            if let Some(range) = source.range(span) {
                let text = source.text();
                let start_line = text[..range.start].matches('\n').count() + 1;
                let start_col = range.start - text[..range.start].rfind('\n').map_or(0, |pos| pos + 1) + 1;
                
                // Get the actual line content
                let lines: Vec<&str> = text.lines().collect();
                let line_content = lines.get(start_line - 1).unwrap_or(&"<line not found>");
                
                return Some(format!("line {}, column {} in file '{}'\n    {}", 
                    start_line, start_col, source.id().vpath().as_rootless_path().display(), line_content));
            }
        }
    }
    
    None
}

/// Typst World implementation for dynamic quill loading
/// 
/// This implementation provides efficient dynamic package loading for the Quill system.
/// Key improvements over previous hardcoded solutions:
/// 
/// - **Dynamic Package Discovery**: Automatically discovers packages in the quill's packages directory
/// - **Proper Virtual Path Handling**: Maintains directory structure in virtual file system (e.g., src/lib.typ)
/// - **Entrypoint Support**: Reads typst.toml files to respect package entrypoint configurations
/// - **Namespace Handling**: Supports @preview and custom namespaces for package imports
/// - **Asset Management**: Correctly loads assets with proper virtual paths (e.g., assets/image.gif)
/// - **Error Handling**: Provides clear error messages for missing packages or files
/// 
/// Usage: 
/// - Place packages in `{quill}/packages/{package-name}/` directories  
/// - Each package should have a `typst.toml` with package metadata including entrypoint
/// - Assets go in `{quill}/assets/` and are accessible as `assets/filename`
/// - Package files maintain their directory structure in the virtual file system
pub struct QuillWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    source: Source,
    sources: HashMap<FileId, Source>,
    binaries: HashMap<FileId, Bytes>,
}

impl QuillWorld {
    /// Create a new QuillWorld from a quill template and Typst content
    pub fn new(quill: &Quill, main: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut sources = HashMap::new();
        let mut binaries = HashMap::new();
        
        // Load fonts from quill's in-memory file system
        let mut book = FontBook::new();
        let mut fonts = Vec::new();
        
        // Load fonts from the quill's in-memory assets first
        let font_data_list = Self::load_fonts_from_quill(quill)?;
        for font_data in font_data_list {
            let font_bytes = Bytes::new(font_data);
            for font in Font::iter(font_bytes) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }
        
        // If no quill fonts found, try to load system fonts
        if fonts.is_empty() {
            let system_font_data_list = Self::load_system_fonts()?;
            for font_data in system_font_data_list {
                let font_bytes = Bytes::new(font_data);
                for font in Font::iter(font_bytes) {
                    book.push(font.info().clone());
                    fonts.push(font);
                }
            }
        }
        
        // Error if no fonts are available at all
        if fonts.is_empty() {
            return Err("No fonts found: neither quill assets nor system fonts are available".into());
        }
        
        // Load assets from the quill's in-memory file system
        Self::load_assets_from_quill(quill, &mut binaries)?;
        
        // Load packages from the quill's in-memory file system
        Self::load_packages_from_quill(quill, &mut sources, &mut binaries)?;
                
        // Create main source
        let main_id = FileId::new(None, VirtualPath::new("main.typ"));
        let source = Source::new(main_id, main.to_string());
        
        Ok(Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            source,
            sources,
            binaries,
        })
    }
    
    /// Load fonts from the quill's in-memory file system
    fn load_fonts_from_quill(quill: &Quill) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        let mut font_data = Vec::new();
        
        // Look for fonts in assets/fonts/ first
        let fonts_paths = quill.find_files("assets/fonts/*");
        for font_path in fonts_paths {
            if let Some(ext) = font_path.extension() {
                if matches!(ext.to_string_lossy().to_lowercase().as_str(), "ttf" | "otf" | "woff" | "woff2") {
                    if let Some(contents) = quill.get_file(&font_path) {
                        font_data.push(contents.to_vec());
                    }
                }
            }
        }

        // If no fonts in fonts subdirectory, look in assets/ root
        if font_data.is_empty() {
            let asset_paths = quill.find_files("assets/*");
            for asset_path in asset_paths {
                if let Some(ext) = asset_path.extension() {
                    if matches!(ext.to_string_lossy().to_lowercase().as_str(), "ttf" | "otf" | "woff" | "woff2") {
                        if let Some(contents) = quill.get_file(&asset_path) {
                            font_data.push(contents.to_vec());
                        }
                    }
                }
            }
        }
        
        Ok(font_data)
    }
    
    /// Load system fonts using fontdb
    fn load_system_fonts() -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        
        let mut font_data = Vec::new();
        
        // Iterate through all font faces in the database
        for id in db.faces().map(|info| info.id) {
            // Get font data using with_face_data
            if let Some(data) = db.with_face_data(id, |data, _face_index| {
                data.to_vec()
            }) {
                font_data.push(data);
            }
        }
        
        Ok(font_data)
    }
    
    /// Load assets from the quill's in-memory file system
    fn load_assets_from_quill(
        quill: &Quill,
        binaries: &mut HashMap<FileId, Bytes>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get all files that start with "assets/"
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
    
    /// Load packages from the quill's in-memory file system
    fn load_packages_from_quill(
        quill: &Quill,
        sources: &mut HashMap<FileId, Source>,
        binaries: &mut HashMap<FileId, Bytes>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Loading packages from quill's in-memory file system");
        
        // Get all subdirectories in packages/
        let package_dirs = quill.list_subdirectories("packages");
        
        for package_dir in package_dirs {
            let package_name = package_dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            println!("Processing package directory: {}", package_name);
            
            // Look for typst.toml in this package
            let toml_path = package_dir.join("typst.toml");
            if let Some(toml_contents) = quill.get_file(&toml_path) {
                let toml_content = String::from_utf8_lossy(toml_contents);
                match parse_package_toml(&toml_content) {
                    Ok(package_info) => {
                        let spec = PackageSpec {
                            namespace: package_info.namespace.clone().into(),
                            name: package_info.name.clone().into(),
                            version: package_info.version.parse()
                                .map_err(|_| format!("Invalid version format: {}", package_info.version))?,
                        };
                        
                        println!("Loading package: {}:{} (namespace: {})", 
                            package_info.name, package_info.version, package_info.namespace);
                        
                        // Load the package files with entrypoint awareness
                        Self::load_package_files_from_quill(quill, &package_dir, sources, binaries, Some(spec), Some(&package_info.entrypoint))?;
                    }
                    Err(e) => {
                        println!("Warning: Failed to parse typst.toml for {}: {}", package_name, e);
                        // Continue with other packages
                    }
                }
            } else {
                // Load as a simple package directory without typst.toml
                println!("No typst.toml found for {}, loading as local package", package_name);
                let spec = PackageSpec {
                    namespace: "local".into(),
                    name: package_name.into(),
                    version: "0.1.0".parse()
                        .map_err(|_| "Invalid version format")?,
                };
                
                Self::load_package_files_from_quill(quill, &package_dir, sources, binaries, Some(spec), None)?;
            }
        }
        
        Ok(())
    }

    /// Load files from a package directory in quill's in-memory file system
    fn load_package_files_from_quill(
        quill: &Quill,
        package_dir: &Path,
        sources: &mut HashMap<FileId, Source>,
        binaries: &mut HashMap<FileId, Bytes>,
        package_spec: Option<PackageSpec>,
        entrypoint: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Find all files in the package directory
        let package_pattern = format!("{}/*", package_dir.to_string_lossy());
        let package_files = quill.find_files(&package_pattern);
        
        for file_path in package_files {
            if let Some(contents) = quill.get_file(&file_path) {
                // Calculate the relative path within the package
                let relative_path = file_path.strip_prefix(package_dir)
                    .map_err(|_| format!("Failed to get relative path for {}", file_path.display()))?;
                
                let virtual_path = VirtualPath::new(relative_path.to_string_lossy().as_ref());
                let file_id = FileId::new(package_spec.clone(), virtual_path);
                
                // Check if this is a source file (.typ) or binary
                if let Some(ext) = file_path.extension() {
                    if ext == "typ" {
                        let source_content = String::from_utf8_lossy(contents);
                        let source = Source::new(file_id, source_content.to_string());
                        sources.insert(file_id, source);
                    } else {
                        binaries.insert(file_id, Bytes::new(contents.to_vec()));
                    }
                } else {
                    // No extension, treat as binary
                    binaries.insert(file_id, Bytes::new(contents.to_vec()));
                }
            }
        }
        
        // Verify entrypoint if specified
        if let (Some(spec), Some(entrypoint_name)) = (&package_spec, entrypoint) {
            let entrypoint_path = VirtualPath::new(entrypoint_name);
            let entrypoint_file_id = FileId::new(Some(spec.clone()), entrypoint_path);
            
            if sources.contains_key(&entrypoint_file_id) {
                println!("Package {} loaded successfully with entrypoint {}", spec.name, entrypoint_name);
            } else {
                println!("Warning: Entrypoint {} not found for package {}", entrypoint_name, spec.name);
            }
        }
        
        Ok(())
    }
}

impl World for QuillWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else if let Some(source) = self.sources.get(&id) {
            Ok(source.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rootless_path().to_owned()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if let Some(bytes) = self.binaries.get(&id) {
            Ok(bytes.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rootless_path().to_owned()))
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        use time::{OffsetDateTime, Duration};

        // Get current UTC time and apply optional hour offset
        let now = OffsetDateTime::now_utc();
        let adjusted = if let Some(hours) = offset {
            now + Duration::hours(hours)
        } else {
            now
        };

        let date = adjusted.date();
        Datetime::from_ymd(date.year(), date.month() as u8, date.day() as u8)
    }
}

/// Simplified package info structure with entrypoint support
#[derive(Debug, Clone)]
struct PackageInfo {
    namespace: String,
    name: String,
    version: String,
    entrypoint: String,
}

/// Parse a typst.toml for package information with better error handling
fn parse_package_toml(content: &str) -> Result<PackageInfo, Box<dyn std::error::Error>> {
    let value: toml::Value = toml::from_str(content)?;
    
    let package_section = value.get("package")
        .ok_or("Missing [package] section in typst.toml")?;
        
    let namespace = package_section.get("namespace")
        .and_then(|v| v.as_str())
        .unwrap_or("preview")
        .to_string();
        
    let name = package_section.get("name")
        .and_then(|v| v.as_str())
        .ok_or("Package name is required in typst.toml")?
        .to_string();
        
    let version = package_section.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.1.0")
        .to_string();
        
    let entrypoint = package_section.get("entrypoint")
        .and_then(|v| v.as_str())
        .unwrap_or("lib.typ")
        .to_string();
    
    Ok(PackageInfo {
        namespace,
        name,
        version,
        entrypoint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_toml() {
        let toml_content = r#"
[package]
name = "test-package"
version = "1.0.0"
namespace = "preview"
entrypoint = "src/lib.typ"
"#;
        
        let package_info = parse_package_toml(toml_content).unwrap();
        assert_eq!(package_info.name, "test-package");
        assert_eq!(package_info.version, "1.0.0");
        assert_eq!(package_info.namespace, "preview");
        assert_eq!(package_info.entrypoint, "src/lib.typ");
    }

    #[test]
    fn test_parse_package_toml_defaults() {
        let toml_content = r#"
[package]
name = "minimal-package"
"#;
        
        let package_info = parse_package_toml(toml_content).unwrap();
        assert_eq!(package_info.name, "minimal-package");
        assert_eq!(package_info.version, "0.1.0");
        assert_eq!(package_info.namespace, "preview");
        assert_eq!(package_info.entrypoint, "lib.typ");
    }

}