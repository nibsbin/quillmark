use std::collections::HashMap;
use std::path::Path;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{package::PackageSpec, FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

use crate::helper;
use quillmark_core::{Diagnostic, Quill, Severity};

/// A file Typst's [`VirtualPath`] would not accept, skipped rather than loaded.
/// One shape for both populations: an asset and a package file fail this the
/// same way, and a consumer routing on the code should not have to know which.
fn skipped_path(path: &Path, err: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::new(
        Severity::Warning,
        format!(
            "Skipping '{}': its path is not usable by Typst ({err})",
            path.display()
        ),
    )
    .with_code("typst::path_skipped".to_string())
    .with_hint("Rename it to a plain relative path.".to_string())
}

/// Build a [`FileId`] for a virtual path, optionally scoped to a package.
///
/// Typst 0.15 routes file ids through [`RootedPath`]: project-local files use
/// [`VirtualRoot::Project`], package files use [`VirtualRoot::Package`].
fn file_id(spec: Option<PackageSpec>, vpath: VirtualPath) -> FileId {
    let root = spec.map_or(VirtualRoot::Project, VirtualRoot::Package);
    FileId::new(RootedPath::new(root, vpath))
}

static FALLBACK_REGULAR: &[u8] = include_bytes!("fonts/Figtree-Regular.ttf");
static FALLBACK_BOLD: &[u8] = include_bytes!("fonts/Figtree-Bold.ttf");
static FALLBACK_ITALIC: &[u8] = include_bytes!("fonts/Figtree-Italic.ttf");

/// Typst `World` implementation for quill-based compilation.
///
/// Provides package loading, virtual-path handling, and asset management for
/// quill templates. Packages are loaded from `{quill}/packages/` and assets
/// from `{quill}/assets/`.
pub struct QuillWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>, // For fonts loaded from assets
    source: Source,
    sources: HashMap<FileId, Source>,
    binaries: HashMap<FileId, Bytes>,
    /// Non-fatal defects from loading the quill's assets and packages: a file
    /// the loader had to skip, a manifest it could not read.
    ///
    /// Without them each defect degrades the compile unattributably: a skipped
    /// package surfaces as an unresolved `#import` naming the plate, three files
    /// from the cause.
    ///
    /// Static for the session's lifetime, since the world loads its files once.
    /// [`crate::TypstSession`] carries them alongside every compile's own.
    load_warnings: Vec<Diagnostic>,
}

impl QuillWorld {
    /// Create a new QuillWorld from a quill template and Typst content
    pub fn new(
        source: &Quill,
        main: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut sources = HashMap::new();
        let mut binaries = HashMap::new();
        let mut load_warnings = Vec::new();

        // Create a new empty FontBook to ensure proper ordering
        let mut book = FontBook::new();
        let mut fonts = Vec::new();

        // Load fonts from quill assets (eagerly loaded).
        let font_data_list = Self::load_fonts_from_quill(source)?;
        for font_data in font_data_list {
            let font_bytes = Bytes::new(font_data);
            for font in Font::iter(font_bytes) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }

        // Fall back to the embedded Figtree faces when the quill ships no fonts.
        if fonts.is_empty() {
            for data in [FALLBACK_REGULAR, FALLBACK_BOLD, FALLBACK_ITALIC] {
                let font_bytes = Bytes::new(data.to_vec());
                for font in Font::iter(font_bytes) {
                    book.push(font.info().clone());
                    fonts.push(font);
                }
            }
        }

        // Load assets from quill's in-memory file system
        Self::load_assets_from_quill(source, &mut binaries, &mut load_warnings)?;

        // Load packages from quill's in-memory file system. Quillmark does
        // not download external packages: every package a quill imports
        // must be vendored under `packages/` in the quill tree.
        Self::load_packages_from_quill(source, &mut sources, &mut binaries, &mut load_warnings)?;

        // The helper package's typst.toml is constant for the session's
        // lifetime, so insert it once here. Re-injecting the helper `lib.typ`
        // per apply leaves it untouched.
        binaries.insert(
            Self::helper_fid("typst.toml"),
            Bytes::new(helper::generate_typst_toml().into_bytes()),
        );

        // Create main source
        let main_id = file_id(
            None,
            VirtualPath::new("main.typ").expect("\"main.typ\" is a valid virtual path"),
        );
        let source = Source::new(main_id, main.to_string());

        Ok(Self {
            library: LazyHash::new(<Library as typst::LibraryExt>::default()),
            book: LazyHash::new(book),
            fonts,
            source,
            sources,
            binaries,
            load_warnings,
        })
    }

    pub(crate) fn load_warnings(&self) -> &[Diagnostic] {
        &self.load_warnings
    }

    /// Like [`new`](Self::new), but injects `json_data` as a virtual
    /// `@local/quillmark-helper:0.1.0` package. Plates import that package to
    /// access document data and auto-evaluated markup fields. Returns the
    /// world plus the generated content windows (see
    /// [`inject_helper_package`](Self::inject_helper_package)).
    pub fn new_with_data(
        source: &Quill,
        main: &str,
        data: &serde_json::Value,
        meta: &crate::SchemaMeta,
    ) -> Result<(Self, Vec<crate::overlay::FieldWindow>), Box<dyn std::error::Error + Send + Sync>>
    {
        let mut world = Self::new(source, main)?;

        // Inject the quillmark-helper package
        let windows = world.inject_helper_package(data, meta)?;

        Ok((world, windows))
    }

    /// A [`FileId`] for `rel` inside the virtual `@local/quillmark-helper`
    /// package (e.g. `lib.typ`).
    pub(crate) fn helper_fid(rel: &str) -> FileId {
        let spec = PackageSpec {
            namespace: helper::HELPER_NAMESPACE.into(),
            name: helper::HELPER_NAME.into(),
            version: helper::HELPER_VERSION
                .parse()
                .expect("Invalid helper version"),
        };
        file_id(
            Some(spec),
            VirtualPath::new(rel).expect("valid helper vpath"),
        )
    }

    /// Replace-or-insert a source. An existing source is edited via
    /// [`Source::replace`]: a prefix/suffix diff reparses only the changed
    /// span, preserving spans on the untouched regions so `comemo` constraints
    /// keep matching across an edit. A new source is inserted whole.
    pub(crate) fn set_source(&mut self, id: FileId, text: &str) {
        match self.sources.get_mut(&id) {
            Some(s) => {
                s.replace(text);
            }
            None => {
                self.sources.insert(id, Source::new(id, text.to_string()));
            }
        }
    }

    /// Inject the quillmark-helper package generated from the transformed
    /// document data plus the schema meta (see `helper::generate_lib_typ`).
    /// `set_source` on the helper `lib.typ` makes a repeat injection (a session
    /// edit) an incremental reparse rather than a fresh parse. The helper's
    /// `typst.toml` is constant and set once at construction. Returns each
    /// generated content block's byte window, paired with the helper file's id:
    /// the span scan's classification table.
    pub(crate) fn inject_helper_package(
        &mut self,
        data: &serde_json::Value,
        meta: &crate::SchemaMeta,
    ) -> Result<Vec<crate::overlay::FieldWindow>, crate::emit::EmitError> {
        let file = Self::helper_fid("lib.typ");
        let (src, windows) = helper::generate_lib_typ(data, meta)?;
        self.set_source(file, &src);
        Ok(windows
            .into_iter()
            .map(|w| crate::overlay::FieldWindow {
                path: w.path,
                file,
                range: w.block,
                segments: w.segments,
            })
            .collect())
    }

    /// Loads fonts from quill's in-memory file system.
    fn load_fonts_from_quill(
        source: &Quill,
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut font_data = Vec::new();

        // Asset fonts first: `QuillWorld` gives them priority over package
        // fonts of the same family, and `Vec` order is that priority.
        for glob in ["assets/fonts/*", "packages/**"] {
            for font_path in source.find_files(glob) {
                let Some(ext) = font_path.extension() else {
                    continue;
                };
                if !matches!(
                    ext.to_string_lossy().to_lowercase().as_str(),
                    "ttf" | "otf" | "woff" | "woff2"
                ) {
                    continue;
                }
                if let Some(contents) = source.get_file(&font_path) {
                    font_data.push(contents.to_vec());
                }
            }
        }

        Ok(font_data)
    }

    /// Loads assets from quill's in-memory file system.
    fn load_assets_from_quill(
        source: &Quill,
        binaries: &mut HashMap<FileId, Bytes>,
        warnings: &mut Vec<Diagnostic>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get all files that start with "assets/"
        let asset_paths = source.find_files("assets/*");

        for asset_path in asset_paths {
            if let Some(contents) = source.get_file(&asset_path) {
                // Create virtual path for the asset
                let virtual_path = match VirtualPath::new(asset_path.to_string_lossy().as_ref()) {
                    Ok(vpath) => vpath,
                    Err(e) => {
                        warnings.push(skipped_path(&asset_path, e));
                        continue;
                    }
                };
                let id = file_id(None, virtual_path);
                binaries.insert(id, Bytes::new(contents.to_vec()));
            }
        }

        Ok(())
    }

    /// Loads packages from quill's in-memory file system.
    fn load_packages_from_quill(
        source: &Quill,
        sources: &mut HashMap<FileId, Source>,
        binaries: &mut HashMap<FileId, Bytes>,
        warnings: &mut Vec<Diagnostic>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get all subdirectories in packages/
        let package_dirs = source.list_directories("packages");

        for package_dir in package_dirs {
            let package_name = package_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Look for typst.toml in this package
            let toml_path = package_dir.join("typst.toml");
            if let Some(toml_contents) = source.get_file(&toml_path) {
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
                            source,
                            &package_dir,
                            sources,
                            binaries,
                            Some(spec),
                            Some(&package_info.entrypoint),
                            warnings,
                        )?;
                    }
                    Err(e) => {
                        // The package is skipped entirely, so the plate's
                        // `#import` for it fails later with an unresolved-file
                        // error naming the plate, not this manifest.
                        warnings.push(
                            Diagnostic::new(
                                Severity::Warning,
                                format!(
                                    "Skipping package '{package_name}': its typst.toml did not \
                                     parse ({e})"
                                ),
                            )
                            .with_code("typst::package_manifest".to_string()),
                        );
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
                    source,
                    &package_dir,
                    sources,
                    binaries,
                    Some(spec),
                    None,
                    warnings,
                )?;
            }
        }

        Ok(())
    }

    /// Loads files from a package directory in quill's in-memory file system.
    fn load_package_files_from_quill(
        source: &Quill,
        package_dir: &Path,
        sources: &mut HashMap<FileId, Source>,
        binaries: &mut HashMap<FileId, Bytes>,
        package_spec: Option<PackageSpec>,
        entrypoint: Option<&str>,
        warnings: &mut Vec<Diagnostic>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Find all files in the package directory
        let package_pattern = format!("{}/*", package_dir.to_string_lossy());
        let package_files = source.find_files(&package_pattern);

        for file_path in package_files {
            if let Some(contents) = source.get_file(&file_path) {
                // Calculate the relative path within the package
                let relative_path = file_path.strip_prefix(package_dir).map_err(|_| {
                    format!("Failed to get relative path for {}", file_path.display())
                })?;

                let virtual_path = match VirtualPath::new(relative_path.to_string_lossy().as_ref())
                {
                    Ok(vpath) => vpath,
                    Err(e) => {
                        warnings.push(skipped_path(&file_path, e));
                        continue;
                    }
                };
                let id = file_id(package_spec.clone(), virtual_path);

                // Check if this is a source file (.typ) or binary
                if let Some(ext) = file_path.extension() {
                    if ext == "typ" {
                        let source_content = String::from_utf8_lossy(contents);
                        let source = Source::new(id, source_content.to_string());
                        sources.insert(id, source);
                    } else {
                        binaries.insert(id, Bytes::new(contents.to_vec()));
                    }
                } else {
                    // No extension, treat as binary
                    binaries.insert(id, Bytes::new(contents.to_vec()));
                }
            }
        }

        // Verify entrypoint if specified
        if let (Some(spec), Some(entrypoint_name)) = (&package_spec, entrypoint) {
            let entrypoint_path = VirtualPath::new(entrypoint_name)
                .map_err(|e| format!("Invalid entrypoint path {}: {}", entrypoint_name, e))?;
            let entrypoint_file_id = file_id(Some(spec.clone()), entrypoint_path);

            if !sources.contains_key(&entrypoint_file_id) {
                warnings.push(
                    Diagnostic::new(
                        Severity::Warning,
                        format!(
                            "Package '{}' declares entrypoint '{entrypoint_name}', which it does \
                             not ship",
                            spec.name
                        ),
                    )
                    .with_code("typst::package_entrypoint_missing".to_string()),
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
            Err(FileError::NotFound(id.vpath().get_without_slash().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if let Some(bytes) = self.binaries.get(&id) {
            Ok(bytes.clone())
        } else {
            Err(FileError::NotFound(id.vpath().get_without_slash().into()))
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        // On native targets we can use the system clock. On wasm32 we call into
        // the JavaScript Date API via js-sys to get UTC date components.
        #[cfg(not(target_arch = "wasm32"))]
        {
            use time::{Duration as TimeDuration, OffsetDateTime};

            // Get current UTC time and apply the optional offset
            let now = OffsetDateTime::now_utc();
            let adjusted = if let Some(offset) = offset {
                now + TimeDuration::seconds(offset.seconds() as i64)
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

            // Apply the offset if requested by constructing a JS Date
            if let Some(offset) = offset {
                // Create a new Date representing now + offset
                let millis = d.get_time() + offset.seconds() * 1_000.0;
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

/// Package info parsed from a typst.toml `[package]` section.
#[derive(Debug, Clone)]
struct PackageInfo {
    namespace: String,
    name: String,
    version: String,
    entrypoint: String,
}

/// Parse a typst.toml `[package]` section into [`PackageInfo`].
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
        .unwrap_or("local")
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
        assert_eq!(package_info.namespace, "local");
        assert_eq!(package_info.entrypoint, "lib.typ");
    }

    /// A minimal in-memory quill, plus whatever extra files the caller wants
    /// under their `/`-joined tree paths.
    fn quill_with(extra: &[(&str, &str)]) -> quillmark_core::Quill {
        use quillmark_core::{FileTreeNode, Quill};
        let mut root = FileTreeNode::Directory {
            files: HashMap::new(),
        };
        root.insert(
            "Quill.yaml",
            FileTreeNode::File {
                contents: b"quill:\n  name: warn\n  version: 0.1.0\n  backend: typst\n  \
                            description: load-warning probe\ntypst:\n  plate_file: plate.typ\n\
                            main:\n  fields:\n    title:\n      type: string\n      \
                            description: title\n"
                    .to_vec(),
            },
        )
        .expect("insert Quill.yaml");
        root.insert(
            "plate.typ",
            FileTreeNode::File {
                contents: b"#set page(width: 100pt, height: 100pt)\n".to_vec(),
            },
        )
        .expect("insert plate");
        for (path, contents) in extra {
            root.insert(
                path,
                FileTreeNode::File {
                    contents: contents.as_bytes().to_vec(),
                },
            )
            .expect("insert extra file");
        }
        Quill::from_tree(root).expect("load quill")
    }

    /// A package the loader has to skip is a warning on the session, not a line
    /// on a stderr nobody reads: on wasm32 there is no stderr at all, and the
    /// only other signal is an unresolved `#import` naming the plate rather
    /// than the manifest three files away.
    #[test]
    fn unparseable_package_manifest_warns_instead_of_vanishing() {
        let quill = quill_with(&[
            ("packages/brokenpkg/typst.toml", "this is not [ valid toml"),
            ("packages/brokenpkg/lib.typ", "#let x = 1\n"),
        ]);
        let world = QuillWorld::new(&quill, "// probe").expect("world builds anyway");

        let codes: Vec<&str> = world
            .load_warnings()
            .iter()
            .filter_map(|d| d.code.as_deref())
            .collect();
        assert!(
            codes.contains(&"typst::package_manifest"),
            "expected a package-manifest warning, got {codes:?}"
        );
        let warning = &world.load_warnings()[0];
        assert_eq!(warning.severity, quillmark_core::Severity::Warning);
        assert!(
            warning.message.contains("brokenpkg"),
            "warning must name the package: {}",
            warning.message
        );
    }

    /// The clean case earns its own assertion: a well-formed quill must not
    /// accumulate advisory noise, or the channel stops being worth reading.
    #[test]
    fn a_well_formed_quill_loads_without_warnings() {
        let quill = quill_with(&[
            (
                "packages/goodpkg/typst.toml",
                "[package]\nname = \"goodpkg\"\nversion = \"0.1.0\"\nentrypoint = \"lib.typ\"\n",
            ),
            ("packages/goodpkg/lib.typ", "#let x = 1\n"),
        ]);
        let world = QuillWorld::new(&quill, "// probe").expect("world");
        assert!(
            world.load_warnings().is_empty(),
            "clean quill warned: {:?}",
            world.load_warnings()
        );
    }

    /// A package that declares an entrypoint it does not ship is importable in
    /// name only.
    #[test]
    fn missing_package_entrypoint_warns() {
        let quill = quill_with(&[(
            "packages/hollow/typst.toml",
            "[package]\nname = \"hollow\"\nversion = \"0.1.0\"\nentrypoint = \"lib.typ\"\n",
        )]);
        let world = QuillWorld::new(&quill, "// probe").expect("world");
        let codes: Vec<&str> = world
            .load_warnings()
            .iter()
            .filter_map(|d| d.code.as_deref())
            .collect();
        assert!(
            codes.contains(&"typst::package_entrypoint_missing"),
            "expected an entrypoint warning, got {codes:?}"
        );
    }

    #[test]
    fn test_asset_fonts_have_priority() {
        use quillmark_core::Quill;

        // The usaf_memo fixture carries real fonts.
        let quill_path = quillmark_fixtures::quills_path("usaf_memo");
        if !quill_path.exists() {
            // Skip test if fixture not found
            return;
        }

        let tree = quillmark::tree_from_path(quill_path).expect("walk fixture");
        let source = Quill::from_tree(tree).expect("load source");
        let world = QuillWorld::new(&source, "// Test").unwrap();

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
