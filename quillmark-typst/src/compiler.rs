use std::collections::HashMap;
use std::path::Path;
use std::fs;
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
pub fn compile_to_pdf(quill: &Quill, typst_content: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    println!("Using quill: {}", quill.name);
    let world = QuillWorld::new(quill, typst_content)?;
    println!("World initialized with {} sources and {} binaries", world.sources.len(), world.binaries.len());
    let document = compile_document(&world)?;
    
    let pdf = typst_pdf::pdf(&document, &PdfOptions::default())
        .map_err(|e| format!("PDF generation failed: {:?}", e))?;

    Ok(pdf)
}

/// Compile a quill template with Typst content to SVG pages
pub fn compile_to_svg(quill: &Quill, typst_content: &str) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
    let world = QuillWorld::new(quill, typst_content)?;
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
    pub fn new(quill: &Quill, typst_content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut sources = HashMap::new();
        let mut binaries = HashMap::new();
        
        // Load fonts - handled by compiler now, not by Quill
        let mut book = FontBook::new();
        let mut fonts = Vec::new();
        
        // Load fonts from the quill's assets directory
        let font_data_list = Self::load_fonts_from_assets(&quill.assets_path())?;
        for font_data in font_data_list {
            let font_bytes = Bytes::new(font_data);
            for font in Font::iter(font_bytes) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }
        
        if fonts.is_empty() {
            return Err("No fonts found in quill assets".into());
        }
        
        // Load assets from the quill
        Self::load_assets_recursive(&quill.assets_path(), &mut binaries, &VirtualPath::new("assets"))?;
        
        // Load packages from the quill
        Self::load_packages_recursive(&quill.packages_path(), &mut sources, &mut binaries)?;
                
        // Create main source
        let main_id = FileId::new(None, VirtualPath::new("main.typ"));
        let source = Source::new(main_id, typst_content.to_string());
        
        Ok(Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            source,
            sources,
            binaries,
        })
    }
    
    /// Load fonts from the assets directory - compiler-specific logic
    fn load_fonts_from_assets(assets_path: &Path) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        let fonts_dir = assets_path.join("fonts");
        let mut font_data = Vec::new();
        
        if !fonts_dir.exists() {
            // Look for any font files in the assets directory
            if assets_path.exists() {
                for entry in fs::read_dir(assets_path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension() {
                            if matches!(ext.to_string_lossy().to_lowercase().as_str(), "ttf" | "otf" | "woff" | "woff2") {
                                font_data.push(fs::read(&path)?);
                            }
                        }
                    }
                }
            }
            
            // If no fonts found in assets, provide system fonts or default fonts
            if font_data.is_empty() {
                // For now, we'll let typst handle system fonts
                // This might require additional handling based on the system
                return Err("No fonts found in quill assets directory and no system fonts configured".into());
            }
        } else {
            // Load fonts from fonts subdirectory
            for entry in fs::read_dir(&fonts_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if matches!(ext.to_string_lossy().to_lowercase().as_str(), "ttf" | "otf" | "woff" | "woff2") {
                            font_data.push(fs::read(&path)?);
                        }
                    }
                }
            }
        }
        
        Ok(font_data)
    }
    
    /// Recursively load assets from a directory
    fn load_assets_recursive(
        dir: &Path,
        binaries: &mut HashMap<FileId, Bytes>,
        base_path: &VirtualPath,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !dir.exists() {
            return Ok(());
        }
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            
            if path.is_file() {
                let virtual_path = base_path.join(&name);
                let file_id = FileId::new(None, virtual_path);
                let data = fs::read(&path)?;
                binaries.insert(file_id, Bytes::new(data));
            } else if path.is_dir() {
                let sub_path = base_path.join(&name);
                Self::load_assets_recursive(&path, binaries, &sub_path)?;
            }
        }
        
        Ok(())
    }
    
    /// Recursively load packages from a directory
    fn load_packages_recursive(
        dir: &Path,
        sources: &mut HashMap<FileId, Source>,
        binaries: &mut HashMap<FileId, Bytes>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !dir.exists() {
            return Ok(());
        }
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                let package_name = entry.file_name().to_string_lossy().to_string();
                
                // Look for a typst.toml to determine package info
                let toml_path = path.join("typst.toml");
                if toml_path.exists() {
                    let toml_content = fs::read_to_string(&toml_path)?;
                    if let Ok(package_info) = parse_package_toml(&toml_content) {
                        let spec = PackageSpec {
                            namespace: package_info.namespace.into(),
                            name: package_info.name.into(),
                            version: package_info.version.parse()
                                .map_err(|_| "Invalid version format")?,
                        };
                        
                        // Load the package files
                        Self::load_package_files(&path, sources, binaries, Some(spec))?;
                    }
                } else {
                    // Load as a simple package directory
                    let spec = PackageSpec {
                        namespace: "local".into(),
                        name: package_name.into(),
                        version: "0.1.0".parse()
                            .map_err(|_| "Invalid version format")?,
                    };
                    
                    Self::load_package_files(&path, sources, binaries, Some(spec))?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Load files from a package directory
    fn load_package_files(
        dir: &Path,
        sources: &mut HashMap<FileId, Source>,
        binaries: &mut HashMap<FileId, Bytes>,
        package_spec: Option<PackageSpec>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::load_package_files_recursive(dir, sources, binaries, package_spec, &VirtualPath::new(""))?;
        Ok(())
    }
    
    /// Recursively load package files
    fn load_package_files_recursive(
        dir: &Path,
        sources: &mut HashMap<FileId, Source>,
        binaries: &mut HashMap<FileId, Bytes>,
        package_spec: Option<PackageSpec>,
        base_path: &VirtualPath,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            
            if path.is_file() {
                let virtual_path = if base_path.as_rootless_path().as_os_str().is_empty() {
                    VirtualPath::new(&name)
                } else {
                    base_path.join(&name)
                };
                
                let file_id = FileId::new(package_spec.clone(), virtual_path);
                
                if name.ends_with(".typ") {
                    let content = fs::read_to_string(&path)?;
                    sources.insert(file_id, Source::new(file_id, content));
                } else {
                    let data = fs::read(&path)?;
                    binaries.insert(file_id, Bytes::new(data));
                }
            } else if path.is_dir() {
                let sub_path = if base_path.as_rootless_path().as_os_str().is_empty() {
                    VirtualPath::new(&name)
                } else {
                    base_path.join(&name)
                };
                Self::load_package_files_recursive(&path, sources, binaries, package_spec.clone(), &sub_path)?;
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
        use chrono::{Datelike, TimeZone};
        
        if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            let timestamp = now.as_secs() as i64;
            let adjusted = timestamp + offset.unwrap_or(0) * 3600;
            
            if let Some(utc) = chrono::Utc.timestamp_opt(adjusted, 0).single() {
                return Datetime::from_ymd(
                    utc.year(),
                    utc.month() as u8,
                    utc.day() as u8,
                );
            }
        }
        
        // Fallback date
        Datetime::from_ymd(2024, 1, 1)
    }
}

/// Simple package info structure
#[derive(Debug)]
struct PackageInfo {
    namespace: String,
    name: String,
    version: String,
}

/// Parse a basic typst.toml for package information
fn parse_package_toml(content: &str) -> Result<PackageInfo, Box<dyn std::error::Error>> {
    let value: toml::Value = toml::from_str(content)?;
    
    let namespace = value.get("package")
        .and_then(|p| p.get("namespace"))
        .and_then(|v| v.as_str())
        .unwrap_or("preview")
        .to_string();
        
    let name = value.get("package")
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
        .ok_or("Package name is required")?
        .to_string();
        
    let version = value.get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or("0.1.0")
        .to_string();
    
    Ok(PackageInfo {
        namespace,
        name,
        version,
    })
}