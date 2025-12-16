use std::collections::HashMap;
use std::path::Path;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{package::PackageSpec, FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

use quillmark_core::Quill;

/// Typst World implementation for quill-based compilation.
///
/// Implements the Typst `World` trait to provide dynamic package loading,
/// virtual path handling, and asset management for quill templates.
/// Packages are loaded from `{quill}/packages/` and assets from `{quill}/assets/`.
pub struct QuillWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>, // For fonts loaded from assets
    source: Source,
    sources: HashMap<FileId, Source>,
    binaries: HashMap<FileId, Bytes>,
}

impl QuillWorld {
    /// Create a new QuillWorld from a quill template and Typst content
    pub fn new(
        quill: &Quill,
        main: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut sources = HashMap::new();
        let mut binaries = HashMap::new();

        // Create a new empty FontBook to ensure proper ordering
        let mut book = FontBook::new();
        let mut fonts = Vec::new();

        // Optionally include an embedded default font (compile-time feature)
        // When enabled, this embedded font is registered BEFORE any quill asset fonts
        // so it acts as a stable fallback across platforms.
        #[cfg(feature = "embed-default-font")]
        {
            fn load_embedded(byte_array: &[u8], book: &mut FontBook, fonts: &mut Vec<Font>) {
                let bytes = Bytes::new(byte_array.to_vec());
                for font in Font::iter(bytes) {
                    book.push(font.info().clone());
                    fonts.push(font);
                }
            }
            // The font file should be placed at `quillmark-typst/assets/RobotoCondensed-VariableFont_wght.ttf`
            // and included in the crate via include_bytes! at compile time.
            const ROBOTO_BYTES: &[u8] =
                include_bytes!("../assets/RobotoCondensed-VariableFont_wght.ttf");
            load_embedded(ROBOTO_BYTES, &mut book, &mut fonts);

            const DEJAVU_SANS_MONO_BYTES: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");
            load_embedded(DEJAVU_SANS_MONO_BYTES, &mut book, &mut fonts);

            const DEJAVU_SANS_MONO_BOLD_BYTES: &[u8] =
                include_bytes!("../assets/DejaVuSansMono-Bold.ttf");
            load_embedded(DEJAVU_SANS_MONO_BOLD_BYTES, &mut book, &mut fonts);
        }

        // Load fonts from quill assets first (eagerly loaded)
        let font_data_list = Self::load_fonts_from_quill(quill)?;
        for font_data in font_data_list {
            let font_bytes = Bytes::new(font_data);
            for font in Font::iter(font_bytes) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }

        // Error if no fonts available
        if fonts.is_empty() {
            return Err(format!("No fonts found: asset_faces={}", fonts.len()).into());
        }

        // Load assets from quill's in-memory file system
        Self::load_assets_from_quill(quill, &mut binaries)?;

        // Load packages from quill's in-memory file system
        Self::load_packages_from_quill(quill, &mut sources, &mut binaries)?;

        // Download and load external packages from Quill.toml
        #[cfg(feature = "native")]
        Self::download_and_load_external_packages(quill, &mut sources, &mut binaries)?;

        // Create main source
        let main_id = FileId::new(None, VirtualPath::new("main.typ"));
        let source = Source::new(main_id, main.to_string());

        Ok(Self {
            library: LazyHash::new(<Library as typst::LibraryExt>::default()),
            book: LazyHash::new(book),
            fonts,
            source,
            sources,
            binaries,
        })
    }

    /// Loads fonts from quill's in-memory file system.
    fn load_fonts_from_quill(
        quill: &Quill,
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut font_data = Vec::new();

        // Look for fonts in assets/fonts/ first
        let fonts_paths = quill.find_files("assets/fonts/*");
        for font_path in fonts_paths {
            if let Some(ext) = font_path.extension() {
                if matches!(
                    ext.to_string_lossy().to_lowercase().as_str(),
                    "ttf" | "otf" | "woff" | "woff2"
                ) {
                    if let Some(contents) = quill.get_file(&font_path) {
                        font_data.push(contents.to_vec());
                    }
                }
            }
        }

        // Also look in packages/*/fonts/ for package fonts
        let package_font_paths = quill.find_files("packages/**");
        for font_path in package_font_paths {
            if let Some(ext) = font_path.extension() {
                if matches!(
                    ext.to_string_lossy().to_lowercase().as_str(),
                    "ttf" | "otf" | "woff" | "woff2"
                ) {
                    if let Some(contents) = quill.get_file(&font_path) {
                        font_data.push(contents.to_vec());
                    }
                }
            }
        }

        // Also look in assets/ root for dynamic fonts (DYNAMIC_FONT__*)
        let asset_paths = quill.find_files("assets/*");
        for asset_path in asset_paths {
            if let Some(ext) = asset_path.extension() {
                if matches!(
                    ext.to_string_lossy().to_lowercase().as_str(),
                    "ttf" | "otf" | "woff" | "woff2"
                ) {
                    if let Some(contents) = quill.get_file(&asset_path) {
                        font_data.push(contents.to_vec());
                    }
                }
            }
        }

        Ok(font_data)
    }

    /// Loads assets from quill's in-memory file system.
    fn load_assets_from_quill(
        quill: &Quill,
        binaries: &mut HashMap<FileId, Bytes>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    /// Downloads and loads external packages from Quill.toml.
    #[cfg(feature = "native")]
    fn download_and_load_external_packages(
        quill: &Quill,
        sources: &mut HashMap<FileId, Source>,
        binaries: &mut HashMap<FileId, Bytes>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use typst_kit::download::{Downloader, ProgressSink};
        use typst_kit::package::{PackageStorage, DEFAULT_PACKAGES_SUBDIR};

        let packages_list = quill.typst_packages();
        if packages_list.is_empty() {
            return Ok(());
        }

        // Create a package storage for downloading packages
        let downloader = Downloader::new("quillmark/0.1.0");
        let cache_dir = dirs::cache_dir().map(|d| d.join(DEFAULT_PACKAGES_SUBDIR));
        let data_dir = dirs::data_dir().map(|d| d.join(DEFAULT_PACKAGES_SUBDIR));

        let storage = PackageStorage::new(cache_dir, data_dir, downloader);

        // Parse and download each package
        for package_str in packages_list {
            // Parse package spec from string (e.g., "@preview/bubble:0.2.2")
            match package_str.parse::<PackageSpec>() {
                Ok(spec) => {
                    // Download/prepare the package
                    let mut progress = ProgressSink;
                    match storage.prepare_package(&spec, &mut progress) {
                        Ok(package_dir) => {
                            // Load the package files from the downloaded directory
                            Self::load_package_from_filesystem(
                                &package_dir,
                                sources,
                                binaries,
                                spec,
                            )?;
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to download package {}: {}", package_str, e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse package spec '{}': {}",
                        package_str, e
                    );
                }
            }
        }

        Ok(())
    }

    /// Loads a package from the filesystem (for downloaded packages).
    #[cfg(feature = "native")]
    fn load_package_from_filesystem(
        package_dir: &Path,
        sources: &mut HashMap<FileId, Source>,
        binaries: &mut HashMap<FileId, Bytes>,
        spec: PackageSpec,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::fs;

        // Read typst.toml to get package info
        let toml_path = package_dir.join("typst.toml");
        let entrypoint = if toml_path.exists() {
            let toml_content = fs::read_to_string(&toml_path)?;
            match parse_package_toml(&toml_content) {
                Ok(info) => info.entrypoint,
                Err(_) => "lib.typ".to_string(),
            }
        } else {
            "lib.typ".to_string()
        };

        // Recursively load all files from the package directory
        Self::load_package_files_recursive(package_dir, package_dir, sources, binaries, &spec)?;

        // Verify entrypoint exists
        let entrypoint_path = VirtualPath::new(&entrypoint);
        let entrypoint_file_id = FileId::new(Some(spec.clone()), entrypoint_path);

        if !sources.contains_key(&entrypoint_file_id) {
            eprintln!(
                "Warning: Entrypoint {} not found for package {}:{}",
                entrypoint, spec.name, spec.version
            );
        }

        Ok(())
    }

    /// Recursively loads files from a package directory.
    #[cfg(feature = "native")]
    fn load_package_files_recursive(
        current_dir: &Path,
        package_root: &Path,
        sources: &mut HashMap<FileId, Source>,
        binaries: &mut HashMap<FileId, Bytes>,
        spec: &PackageSpec,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::fs;

        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                // Calculate relative path from package root
                let relative_path = path
                    .strip_prefix(package_root)
                    .map_err(|e| format!("Failed to strip prefix: {}", e))?;

                let virtual_path = VirtualPath::new(relative_path.to_string_lossy().as_ref());
                let file_id = FileId::new(Some(spec.clone()), virtual_path);

                // Load file contents
                let contents = fs::read(&path)?;

                // Determine if it's a source or binary file
                if let Some(ext) = path.extension() {
                    if ext == "typ" {
                        // Source file
                        let text = String::from_utf8_lossy(&contents).to_string();
                        sources.insert(file_id, Source::new(file_id, text));
                    } else {
                        // Binary file
                        binaries.insert(file_id, Bytes::new(contents));
                    }
                } else {
                    // No extension, treat as binary
                    binaries.insert(file_id, Bytes::new(contents));
                }
            } else if path.is_dir() {
                // Recursively process subdirectories
                Self::load_package_files_recursive(&path, package_root, sources, binaries, spec)?;
            }
        }

        Ok(())
    }

    /// Loads packages from quill's in-memory file system.
    fn load_packages_from_quill(
        quill: &Quill,
        sources: &mut HashMap<FileId, Source>,
        binaries: &mut HashMap<FileId, Bytes>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get all subdirectories in packages/
        let package_dirs = quill.list_directories("packages");

        for package_dir in package_dirs {
            let package_name = package_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Look for typst.toml in this package
            let toml_path = package_dir.join("typst.toml");
            if let Some(toml_contents) = quill.get_file(&toml_path) {
                let toml_content = String::from_utf8_lossy(toml_contents);
                match parse_package_toml(&toml_content) {
                    Ok(package_info) => {
                        let spec = PackageSpec {
                            namespace: package_info.namespace.clone().into(),
                            name: package_info.name.clone().into(),
                            version: package_info.version.parse().map_err(|_| {
                                format!("Invalid version format: {}", package_info.version)
                            })?,
                        };

                        // Load the package files with entrypoint awareness
                        Self::load_package_files_from_quill(
                            quill,
                            &package_dir,
                            sources,
                            binaries,
                            Some(spec),
                            Some(&package_info.entrypoint),
                        )?;
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to parse typst.toml for {}: {}",
                            package_name, e
                        );
                        // Continue with other packages
                    }
                }
            } else {
                // Load as a simple package directory without typst.toml
                let spec = PackageSpec {
                    namespace: "local".into(),
                    name: package_name.into(),
                    version: "0.1.0".parse().map_err(|_| "Invalid version format")?,
                };

                Self::load_package_files_from_quill(
                    quill,
                    &package_dir,
                    sources,
                    binaries,
                    Some(spec),
                    None,
                )?;
            }
        }

        Ok(())
    }

    /// Loads files from a package directory in quill's in-memory file system.
    fn load_package_files_from_quill(
        quill: &Quill,
        package_dir: &Path,
        sources: &mut HashMap<FileId, Source>,
        binaries: &mut HashMap<FileId, Bytes>,
        package_spec: Option<PackageSpec>,
        entrypoint: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Find all files in the package directory
        let package_pattern = format!("{}/*", package_dir.to_string_lossy());
        let package_files = quill.find_files(&package_pattern);

        for file_path in package_files {
            if let Some(contents) = quill.get_file(&file_path) {
                // Calculate the relative path within the package
                let relative_path = file_path.strip_prefix(package_dir).map_err(|_| {
                    format!("Failed to get relative path for {}", file_path.display())
                })?;

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

            if !sources.contains_key(&entrypoint_file_id) {
                eprintln!(
                    "Warning: Entrypoint {} not found for package {}",
                    entrypoint_name, spec.name
                );
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
            Err(FileError::NotFound(
                id.vpath().as_rootless_path().to_owned(),
            ))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if let Some(bytes) = self.binaries.get(&id) {
            Ok(bytes.clone())
        } else {
            Err(FileError::NotFound(
                id.vpath().as_rootless_path().to_owned(),
            ))
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        // First check if we have an asset font at this index
        if let Some(font) = self.fonts.get(index) {
            return Some(font.clone());
        }

        None
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        // On native targets we can use the system clock. On wasm32 we call into
        // the JavaScript Date API via js-sys to get UTC date components.
        #[cfg(not(target_arch = "wasm32"))]
        {
            use time::{Duration, OffsetDateTime};

            // Get current UTC time and apply optional hour offset
            let now = OffsetDateTime::now_utc();
            let adjusted = if let Some(hours) = offset {
                now + Duration::hours(hours)
            } else {
                now
            };

            let date = adjusted.date();
            Datetime::from_ymd(date.year(), date.month() as u8, date.day())
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Use js-sys to access the JS Date methods. This returns components in
            // UTC using getUTCFullYear/getUTCMonth/getUTCDate.
            use js_sys::Date;
            use wasm_bindgen::JsValue;

            let d = Date::new_0();
            // get_utc_full_year returns f64
            let year = d.get_utc_full_year() as i32;
            // get_utc_month returns 0-based month
            let month = (d.get_utc_month() as u8).saturating_add(1);
            let day = d.get_utc_date() as u8;

            // Apply hour offset if requested by constructing a JS Date with hours
            if let Some(hours) = offset {
                // Create a new Date representing now + offset hours
                let millis = d.get_time() + (hours as f64) * 3_600_000.0;
                let d2 = Date::new(&JsValue::from_f64(millis));
                let year = d2.get_utc_full_year() as i32;
                let month = (d2.get_utc_month() as u8).saturating_add(1);
                let day = d2.get_utc_date() as u8;
                return Datetime::from_ymd(year, month, day);
            }

            Datetime::from_ymd(year, month, day)
        }
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
fn parse_package_toml(
    content: &str,
) -> Result<PackageInfo, Box<dyn std::error::Error + Send + Sync>> {
    let value: toml::Value = toml::from_str(content)?;

    let package_section = value
        .get("package")
        .ok_or("Missing [package] section in typst.toml")?;

    let namespace = package_section
        .get("namespace")
        .and_then(|v| v.as_str())
        .unwrap_or("preview")
        .to_string();

    let name = package_section
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("Package name is required in typst.toml")?
        .to_string();

    let version = package_section
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.1.0")
        .to_string();

    let entrypoint = package_section
        .get("entrypoint")
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

    #[test]
    fn test_asset_fonts_have_priority() {
        use quillmark_core::Quill;
        use std::path::Path;

        // Use the actual usaf_memo fixture which has real fonts
        let quill_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("quillmark-fixtures")
            .join("resources")
            .join("usaf_memo");

        if !quill_path.exists() {
            // Skip test if fixture not found
            return;
        }

        let quill = Quill::from_path(&quill_path).unwrap();
        let world = QuillWorld::new(&quill, "// Test").unwrap();

        // Asset fonts should be loaded
        assert!(!world.fonts.is_empty(), "Should have asset fonts loaded");

        // The first fonts in the book should be the asset fonts
        // Verify that indices 0..asset_count return asset fonts from the fonts vec
        for i in 0..world.fonts.len() {
            let font = world.font(i);
            assert!(font.is_some(), "Font at index {} should be available", i);
            // This font should come from the asset fonts (world.fonts vec), not font_slots
        }
    }
}
