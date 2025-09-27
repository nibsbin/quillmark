use std::collections::HashMap;
use std::path::Path;
use std::fs;
use typst::diag::{FileError, FileResult, Warned};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath, package::PackageSpec};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst::layout::PagedDocument;
use typst_pdf::PdfOptions;

use crate::Quill;

/// Compile a quill template with markdown content to PDF
pub fn compile_to_pdf(quill: &Quill, markdown: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let world = QuillWorld::new(quill, markdown)?;
    let document = compile_document(&world)?;
    
    let pdf = typst_pdf::pdf(&document, &PdfOptions::default())
        .map_err(|e| format!("PDF generation failed: {:?}", e))?;

    Ok(pdf)
}

/// Compile a quill template with markdown content to SVG pages
pub fn compile_to_svg(quill: &Quill, markdown: &str) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
    let world = QuillWorld::new(quill, markdown)?;
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
        format!("Compilation failed with {} error(s): {:?}", errors.len(), errors).into()
    })
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
    /// Create a new QuillWorld from a quill template and markdown content
    pub fn new(quill: &Quill, markdown: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut sources = HashMap::new();
        let mut binaries = HashMap::new();
        
        // Load fonts
        let mut book = FontBook::new();
        let mut fonts = Vec::new();
        
        // Load fonts from the quill's assets directory
        let font_data_list = quill.load_fonts()?;
        for font_data in font_data_list {
            let font_bytes = Bytes::new(font_data);
            for font in Font::iter(font_bytes) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }
        
        // If no fonts were found in the quill, use system fonts
        if fonts.is_empty() {
            // Add some basic font fallbacks - in a real implementation, 
            // you might want to embed some default fonts
            for family in &["Linux Libertine", "Times New Roman", "Arial"] {
                if let Some(font_data) = find_system_font(family) {
                    let font_bytes = Bytes::new(font_data);
                    for font in Font::iter(font_bytes) {
                        book.push(font.info().clone());
                        fonts.push(font);
                    }
                    break;
                }
            }
        }
        
        // Load assets from the quill
        Self::load_assets_recursive(&quill.assets_path(), &mut binaries, &VirtualPath::new("assets"))?;
        
        // Load packages from the quill
        Self::load_packages_recursive(&quill.packages_path(), &mut sources, &mut binaries)?;
        
        // Read and process the main Typst file
        let main_content = fs::read_to_string(quill.main_path())?;
        let processed_content = process_main_content(&main_content, markdown)?;
        
        // Create main source
        let main_id = FileId::new(None, VirtualPath::new("main.typ"));
        let source = Source::new(main_id, processed_content);
        
        Ok(Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            source,
            sources,
            binaries,
        })
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

/// Process the main content by replacing placeholders with markdown content
fn process_main_content(main_content: &str, markdown: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Simple markdown to Typst conversion
    // In a real implementation, this would use pulldown-cmark or similar
    let typst_content = markdown_to_typst_simple(markdown)?;
    
    // Replace the content placeholder
    let processed = main_content.replace("$content$", &typst_content);
    
    Ok(processed)
}

/// Simple markdown to Typst conversion
/// This is a basic implementation - a full implementation would use pulldown-cmark
fn markdown_to_typst_simple(markdown: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut result = String::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    
    for line in markdown.lines() {
        if line.starts_with("```") {
            if in_code_block {
                // End code block
                result.push_str("```\n");
                in_code_block = false;
                code_lang.clear();
            } else {
                // Start code block
                let lang = line.strip_prefix("```").unwrap_or("").trim();
                if !lang.is_empty() {
                    result.push_str(&format!("```{}\n", lang));
                } else {
                    result.push_str("```\n");
                }
                in_code_block = true;
            }
        } else if in_code_block {
            // Inside code block - pass through as-is
            result.push_str(line);
            result.push('\n');
        } else {
            // Regular markdown processing
            let converted = convert_markdown_line(line);
            result.push_str(&converted);
            result.push('\n');
        }
    }
    
    Ok(result)
}

/// Convert a single markdown line to Typst
fn convert_markdown_line(line: &str) -> String {
    let line = line.trim();
    
    if line.is_empty() {
        return String::new();
    }
    
    // Headers
    if let Some(stripped) = line.strip_prefix("#### ") {
        return format!("==== {}", stripped);
    }
    if let Some(stripped) = line.strip_prefix("### ") {
        return format!("=== {}", stripped);
    }
    if let Some(stripped) = line.strip_prefix("## ") {
        return format!("== {}", stripped);
    }
    if let Some(stripped) = line.strip_prefix("# ") {
        return format!("= {}", stripped);
    }
    
    // Blockquotes
    if let Some(stripped) = line.strip_prefix("> ") {
        return format!("#quote[{}]", convert_inline_markdown(stripped));
    }
    
    // Lists
    if let Some(stripped) = line.strip_prefix("- ") {
        return format!("- {}", convert_inline_markdown(stripped));
    }
    if let Some(stripped) = line.strip_prefix("* ") {
        return format!("- {}", convert_inline_markdown(stripped));
    }
    
    // Regular paragraph
    convert_inline_markdown(line)
}

/// Convert inline markdown elements
fn convert_inline_markdown(text: &str) -> String {
    let mut result = text.to_string();
    
    // Bold: **text** -> *text*
    while let Some(start) = result.find("**") {
        if let Some(end) = result[start + 2..].find("**") {
            let end = start + 2 + end;
            let bold_text = &result[start + 2..end];
            result.replace_range(start..end + 2, &format!("*{}*", bold_text));
        } else {
            break;
        }
    }
    
    // Italic: *text* -> _text_
    let mut chars: Vec<char> = result.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '*' {
            // Find the closing *
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '*' {
                j += 1;
            }
            if j < chars.len() {
                // Replace * with _
                chars[i] = '_';
                chars[j] = '_';
                i = j + 1;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    result = chars.into_iter().collect();
    
    // Inline code: `code` -> `code` (stays the same)
    
    result
}

/// Try to find a system font (placeholder - in a real implementation this would
/// use proper font discovery)
fn find_system_font(_family: &str) -> Option<Vec<u8>> {
    // This is a placeholder - in a real implementation you would:
    // 1. Use fontconfig on Linux
    // 2. Use system font directories on macOS/Windows
    // 3. Or embed some default fonts
    None
}