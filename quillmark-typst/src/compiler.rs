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
        format!("Compilation failed with {} error(s): {:?}", errors.len(), errors).into()
    })
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
                // Use the same manual path construction as package loading
                let virtual_path = if base_path.as_rootless_path().as_os_str().is_empty() {
                    VirtualPath::new(&name)
                } else {
                    let base_str = base_path.as_rootless_path().to_string_lossy();
                    let full_path = format!("{}/{}", base_str, name);
                    VirtualPath::new(&full_path)
                };
                let file_id = FileId::new(None, virtual_path);
                let data = fs::read(&path)?;
                binaries.insert(file_id, Bytes::new(data));
            } else if path.is_dir() {
                // Use the same manual path construction for subdirectories
                let sub_path = if base_path.as_rootless_path().as_os_str().is_empty() {
                    VirtualPath::new(&name)
                } else {
                    let base_str = base_path.as_rootless_path().to_string_lossy();
                    let full_path = format!("{}/{}", base_str, name);
                    VirtualPath::new(&full_path)
                };
                Self::load_assets_recursive(&path, binaries, &sub_path)?;
            }
        }
        
        Ok(())
    }
    
    /// Efficiently load packages from a directory with better error handling
    /// 
    /// This method replaces the previous hardcoded package loading approach with a dynamic
    /// system that:
    /// - Scans package directories for typst.toml files
    /// - Respects package entrypoints and metadata  
    /// - Preserves directory structure in virtual paths
    /// - Handles multiple namespaces (@preview, @local, etc.)
    /// - Provides clear error reporting for debugging
    fn load_packages_recursive(
        dir: &Path,
        sources: &mut HashMap<FileId, Source>,
        binaries: &mut HashMap<FileId, Bytes>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !dir.exists() {
            println!("Package directory does not exist: {}", dir.display());
            return Ok(());
        }
        
        println!("Loading packages from: {}", dir.display());
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                let package_name = entry.file_name().to_string_lossy().to_string();
                println!("Processing package directory: {}", package_name);
                
                // Look for a typst.toml to determine package info
                let toml_path = path.join("typst.toml");
                if toml_path.exists() {
                    let toml_content = fs::read_to_string(&toml_path)?;
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
                            Self::load_package_files_with_entrypoint(&path, sources, binaries, spec, &package_info.entrypoint)?;
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
                    
                    Self::load_package_files(&path, sources, binaries, Some(spec))?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Load files from a package directory with entrypoint support
    fn load_package_files_with_entrypoint(
        dir: &Path,
        sources: &mut HashMap<FileId, Source>,
        binaries: &mut HashMap<FileId, Bytes>,
        package_spec: PackageSpec,
        entrypoint: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Load all files recursively to ensure consistent directory structure
        Self::load_package_files_recursive(dir, sources, binaries, Some(package_spec.clone()), &VirtualPath::new(""))?;
        
        // Verify the entrypoint was loaded correctly
        let expected_entrypoint_path = VirtualPath::new(entrypoint);
        let entrypoint_file_id = FileId::new(Some(package_spec.clone()), expected_entrypoint_path);
        
        if sources.contains_key(&entrypoint_file_id) {
            println!("Package {} loaded successfully with {} sources", package_spec.name, sources.len());
        } else {
            println!("Warning: Entrypoint {} not found after recursive loading", entrypoint);
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
                    // Manually construct the path to ensure it works correctly
                    let base_str = base_path.as_rootless_path().to_string_lossy();
                    let full_path = format!("{}/{}", base_str, name);
                    VirtualPath::new(&full_path)
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
    use std::fs;
    use tempfile::TempDir;

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

    #[test]
    fn test_package_loading_with_virtual_paths() {
        // This test verifies that the improved package loading correctly handles
        // virtual paths and directory structures
        let temp_dir = TempDir::new().unwrap();
        let package_dir = temp_dir.path().join("test-package");
        let src_dir = package_dir.join("src");
        
        fs::create_dir_all(&src_dir).unwrap();
        
        // Create typst.toml
        fs::write(
            package_dir.join("typst.toml"),
            r#"
[package]
name = "test-package"
version = "1.0.0" 
entrypoint = "src/lib.typ"
"#
        ).unwrap();
        
        // Create package files with directory structure
        fs::write(src_dir.join("lib.typ"), "#import \"utils.typ\": *\n\n// Main package file").unwrap();
        fs::write(src_dir.join("utils.typ"), "// Utility functions").unwrap();
        
        let mut sources = HashMap::new();
        let mut binaries = HashMap::new();
        
        // Test the package loading
        let result = QuillWorld::load_packages_recursive(&temp_dir.path(), &mut sources, &mut binaries);
        assert!(result.is_ok(), "Package loading should succeed");
        
        // Verify that files are loaded with correct virtual paths
        let expected_lib_path = VirtualPath::new("src/lib.typ");
        let expected_utils_path = VirtualPath::new("src/utils.typ");
        
        let lib_id = FileId::new(
            Some(PackageSpec {
                namespace: "preview".into(),
                name: "test-package".into(),
                version: "1.0.0".parse().unwrap(),
            }),
            expected_lib_path
        );
        
        let utils_id = FileId::new(
            Some(PackageSpec {
                namespace: "preview".into(),
                name: "test-package".into(),
                version: "1.0.0".parse().unwrap(),
            }),
            expected_utils_path
        );
        
        assert!(sources.contains_key(&lib_id), "lib.typ should be loaded at src/lib.typ");
        assert!(sources.contains_key(&utils_id), "utils.typ should be loaded at src/utils.typ");
        
        // Verify content is loaded correctly
        assert!(sources[&lib_id].text().contains("Main package file"));
        assert!(sources[&utils_id].text().contains("Utility functions"));
    }
}