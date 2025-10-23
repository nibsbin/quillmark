//! Type definitions for the WASM API

use serde::{Deserialize, Serialize};

/// Output formats supported by backends
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Pdf,
    Svg,
    Txt,
}

impl From<OutputFormat> for quillmark_core::OutputFormat {
    fn from(format: OutputFormat) -> Self {
        match format {
            OutputFormat::Pdf => quillmark_core::OutputFormat::Pdf,
            OutputFormat::Svg => quillmark_core::OutputFormat::Svg,
            OutputFormat::Txt => quillmark_core::OutputFormat::Txt,
        }
    }
}

impl From<quillmark_core::OutputFormat> for OutputFormat {
    fn from(format: quillmark_core::OutputFormat) -> Self {
        match format {
            quillmark_core::OutputFormat::Pdf => OutputFormat::Pdf,
            quillmark_core::OutputFormat::Svg => OutputFormat::Svg,
            quillmark_core::OutputFormat::Txt => OutputFormat::Txt,
        }
    }
}

/// Severity levels for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl From<quillmark_core::Severity> for Severity {
    fn from(severity: quillmark_core::Severity) -> Self {
        match severity {
            quillmark_core::Severity::Error => Severity::Error,
            quillmark_core::Severity::Warning => Severity::Warning,
            quillmark_core::Severity::Note => Severity::Note,
        }
    }
}

/// Source location for errors and warnings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl From<quillmark_core::Location> for Location {
    fn from(loc: quillmark_core::Location) -> Self {
        Location {
            file: loc.file,
            line: loc.line as usize,
            column: loc.col as usize,
        }
    }
}

/// Diagnostic message (error, warning, or note)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: Severity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub source_chain: Vec<String>,
}

impl From<quillmark_core::Diagnostic> for Diagnostic {
    fn from(diag: quillmark_core::Diagnostic) -> Self {
        let source_chain = diag.source_chain();
        Diagnostic {
            severity: diag.severity.into(),
            code: diag.code,
            message: diag.message,
            location: diag.primary.map(|loc| loc.into()),
            hint: diag.hint,
            source_chain,
        }
    }
}

/// Rendered artifact (PDF, SVG, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub format: OutputFormat,
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

impl Artifact {
    fn mime_type_for_format(format: OutputFormat) -> String {
        match format {
            OutputFormat::Pdf => "application/pdf".to_string(),
            OutputFormat::Svg => "image/svg+xml".to_string(),
            OutputFormat::Txt => "text/plain".to_string(),
        }
    }
}

impl From<quillmark_core::Artifact> for Artifact {
    fn from(artifact: quillmark_core::Artifact) -> Self {
        let format = artifact.output_format.into();
        Artifact {
            format,
            mime_type: Self::mime_type_for_format(format),
            bytes: artifact.bytes,
        }
    }
}

/// Result of a render operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderResult {
    pub artifacts: Vec<Artifact>,
    pub warnings: Vec<Diagnostic>,
    pub output_format: OutputFormat,
    pub render_time_ms: f64,
}

/// Quill metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuillMetadata {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
}

/// Shallow information about a registered Quill
///
/// This provides consumers with the necessary information to configure render options
/// without exposing the entire Quill file tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuillInfo {
    /// Quill name
    pub name: String,
    /// Backend ID (e.g., "typst")
    pub backend: String,
    /// Quill metadata (plain JavaScript object)
    pub metadata: serde_json::Value,
    /// Loaded example markdown (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
    /// Field schemas (plain JavaScript object)
    pub schema: serde_json::Value,
    /// Supported output formats for this quill's backend
    pub supported_formats: Vec<OutputFormat>,
}

/// Parsed markdown document
///
/// Returned by `Quillmark.parseMarkdown()`. Contains the parsed YAML frontmatter
/// fields and the optional quill tag from the QUILL field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedDocument {
    /// YAML frontmatter fields
    pub fields: serde_json::Value,
    /// The quill tag from QUILL field (if present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quill_tag: Option<String>,
}

/// Options for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<OutputFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<serde_json::Value>,
    /// Optional quill name that overrides or fills in for the markdown's QUILL frontmatter field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quill_name: Option<String>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            format: Some(OutputFormat::Pdf),
            assets: None,
            quill_name: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_serialization() {
        // Test that OutputFormat serializes to lowercase strings
        let pdf = OutputFormat::Pdf;
        let json_pdf = serde_json::to_string(&pdf).unwrap();
        assert_eq!(json_pdf, "\"pdf\"");

        let svg = OutputFormat::Svg;
        let json_svg = serde_json::to_string(&svg).unwrap();
        assert_eq!(json_svg, "\"svg\"");

        let txt = OutputFormat::Txt;
        let json_txt = serde_json::to_string(&txt).unwrap();
        assert_eq!(json_txt, "\"txt\"");
    }

    #[test]
    fn test_output_format_deserialization() {
        // Test that lowercase strings deserialize to OutputFormat
        let pdf: OutputFormat = serde_json::from_str("\"pdf\"").unwrap();
        assert_eq!(pdf, OutputFormat::Pdf);

        let svg: OutputFormat = serde_json::from_str("\"svg\"").unwrap();
        assert_eq!(svg, OutputFormat::Svg);

        let txt: OutputFormat = serde_json::from_str("\"txt\"").unwrap();
        assert_eq!(txt, OutputFormat::Txt);
    }

    #[test]
    fn test_severity_serialization() {
        // Test that Severity serializes to lowercase strings
        let error = Severity::Error;
        let json_error = serde_json::to_string(&error).unwrap();
        assert_eq!(json_error, "\"error\"");

        let warning = Severity::Warning;
        let json_warning = serde_json::to_string(&warning).unwrap();
        assert_eq!(json_warning, "\"warning\"");

        let note = Severity::Note;
        let json_note = serde_json::to_string(&note).unwrap();
        assert_eq!(json_note, "\"note\"");
    }

    #[test]
    fn test_severity_deserialization() {
        // Test that lowercase strings deserialize to Severity
        let error: Severity = serde_json::from_str("\"error\"").unwrap();
        assert_eq!(error, Severity::Error);

        let warning: Severity = serde_json::from_str("\"warning\"").unwrap();
        assert_eq!(warning, Severity::Warning);

        let note: Severity = serde_json::from_str("\"note\"").unwrap();
        assert_eq!(note, Severity::Note);
    }

    #[test]
    fn test_diagnostic_serialization() {
        // Test that diagnostics with all fields serialize correctly
        let diag = quillmark_core::Diagnostic::new(
            quillmark_core::Severity::Error,
            "Test error message".to_string(),
        )
        .with_code("E001".to_string())
        .with_location(quillmark_core::Location {
            file: "test.typ".to_string(),
            line: 10,
            col: 5,
        })
        .with_hint("This is a hint".to_string());

        let wasm_diag: Diagnostic = diag.into();
        let json = serde_json::to_string(&wasm_diag).unwrap();

        assert!(json.contains("\"severity\":\"error\""));
        assert!(json.contains("\"code\":\"E001\""));
        assert!(json.contains("\"message\":\"Test error message\""));
        assert!(json.contains("\"hint\":\"This is a hint\""));
        assert!(json.contains("\"file\":\"test.typ\""));
        assert!(json.contains("\"line\":10"));
        assert!(json.contains("\"column\":5"));
    }

    #[test]
    fn test_diagnostic_with_source_chain() {
        // Test that diagnostics with source chains serialize correctly
        let root_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let diag = quillmark_core::Diagnostic::new(
            quillmark_core::Severity::Error,
            "Failed to load template".to_string(),
        )
        .with_code("E002".to_string())
        .with_source(Box::new(root_error));

        let wasm_diag: Diagnostic = diag.into();
        let json = serde_json::to_string(&wasm_diag).unwrap();

        assert!(json.contains("\"severity\":\"error\""));
        assert!(json.contains("\"code\":\"E002\""));
        assert!(json.contains("\"message\":\"Failed to load template\""));
        assert!(json.contains("\"sourceChain\""));
        assert!(json.contains("File not found"));
    }

    #[test]
    fn test_render_options_with_format() {
        // Test that RenderOptions with format works correctly
        let options = RenderOptions {
            format: Some(OutputFormat::Pdf),
            assets: None,
            quill_name: None,
        };
        let json = serde_json::to_string(&options).unwrap();
        assert!(json.contains("\"format\":\"pdf\""));

        // Test deserialization
        let options_from_json: RenderOptions = serde_json::from_str(r#"{"format":"svg"}"#).unwrap();
        assert_eq!(options_from_json.format, Some(OutputFormat::Svg));

        // Test with quill_name
        let options_with_quill = RenderOptions {
            format: Some(OutputFormat::Pdf),
            assets: None,
            quill_name: Some("test_quill".to_string()),
        };
        let json_with_quill = serde_json::to_string(&options_with_quill).unwrap();
        assert!(json_with_quill.contains("\"quillName\":\"test_quill\""));

        // Test deserialization with quill_name
        let options_from_json_with_quill: RenderOptions =
            serde_json::from_str(r#"{"format":"pdf","quillName":"my_quill"}"#).unwrap();
        assert_eq!(
            options_from_json_with_quill.quill_name,
            Some("my_quill".to_string())
        );
    }

    #[test]
    fn test_render_options_with_assets() {
        // Test that assets field can be deserialized from JSON object
        let json = r#"{
            "format": "pdf",
            "assets": {
                "logo.png": [137, 80, 78, 71],
                "font.ttf": [0, 1, 2, 3]
            }
        }"#;

        let options: RenderOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.format, Some(OutputFormat::Pdf));
        assert!(options.assets.is_some());

        // Verify assets is a JSON object
        let assets = options.assets.unwrap();
        assert!(assets.is_object());
        let assets_obj = assets.as_object().unwrap();
        assert_eq!(assets_obj.len(), 2);
        assert!(assets_obj.contains_key("logo.png"));
        assert!(assets_obj.contains_key("font.ttf"));

        // Verify asset values are arrays
        let logo_bytes = assets_obj.get("logo.png").unwrap().as_array().unwrap();
        assert_eq!(logo_bytes.len(), 4);
        assert_eq!(logo_bytes[0].as_u64().unwrap(), 137);
    }

    #[test]
    fn test_quill_info_plain_objects() {
        // Test that QuillInfo metadata and field_schemas are serde_json::Value objects
        let mut metadata_obj = serde_json::Map::new();
        metadata_obj.insert("key1".to_string(), serde_json::json!("value1"));
        metadata_obj.insert("key2".to_string(), serde_json::json!(42));

        let mut schema_obj = serde_json::Map::new();
        schema_obj.insert(
            "title".to_string(),
            serde_json::json!({
                "type": "string",
                "required": true,
                "description": "Document title"
            }),
        );

        let quill_info = QuillInfo {
            name: "test-quill".to_string(),
            backend: "typst".to_string(),
            metadata: serde_json::Value::Object(metadata_obj),
            example: None,
            schema: serde_json::Value::Object(schema_obj),
            supported_formats: vec![OutputFormat::Pdf, OutputFormat::Svg],
        };

        // Serialize to JSON and verify structure
        let json = serde_json::to_value(&quill_info).unwrap();
        assert!(json.is_object());

        let obj = json.as_object().unwrap();
        assert_eq!(obj.get("name").unwrap().as_str().unwrap(), "test-quill");
        assert_eq!(obj.get("backend").unwrap().as_str().unwrap(), "typst");

        // Verify metadata is an object (not a Map)
        let metadata = obj.get("metadata").unwrap();
        assert!(metadata.is_object());
        let metadata_obj = metadata.as_object().unwrap();
        assert_eq!(
            metadata_obj.get("key1").unwrap().as_str().unwrap(),
            "value1"
        );
        assert_eq!(metadata_obj.get("key2").unwrap().as_u64().unwrap(), 42);

        // Verify field_schemas is an object
        let schema = obj.get("schema").unwrap();
        assert!(schema.is_object());
    }

    #[test]
    fn test_parsed_document_fields_is_object() {
        // Test that ParsedDocument fields is a plain JSON object
        let mut fields_obj = serde_json::Map::new();
        fields_obj.insert("title".to_string(), serde_json::json!("My Document"));
        fields_obj.insert("author".to_string(), serde_json::json!("Alice"));

        let parsed_doc = ParsedDocument {
            fields: serde_json::Value::Object(fields_obj),
            quill_tag: Some("test-quill".to_string()),
        };

        // Serialize and verify structure
        let json = serde_json::to_value(&parsed_doc).unwrap();
        assert!(json.is_object());

        let obj = json.as_object().unwrap();
        assert_eq!(obj.get("quillTag").unwrap().as_str().unwrap(), "test-quill");

        // Verify fields is an object (not a Map)
        let fields = obj.get("fields").unwrap();
        assert!(fields.is_object());
        let fields_obj = fields.as_object().unwrap();
        assert_eq!(
            fields_obj.get("title").unwrap().as_str().unwrap(),
            "My Document"
        );
        assert_eq!(fields_obj.get("author").unwrap().as_str().unwrap(), "Alice");
    }
}
