//! # AcroForm Backend Integration Tests
//!
//! Integration tests for the AcroForm backend PDF form filling functionality.
//!
//! ## Test Coverage
//!
//! This test suite validates:
//! - **Backend compilation** - Compiling JSON context to filled PDF forms
//! - **Field mapping** - Mapping template values to PDF form fields
//! - **Template rendering** - MiniJinja template processing in field values
//! - **PDF output** - Valid PDF generation with filled fields
//!
//! ## AcroForm Backend
//!
//! The AcroForm backend fills PDF forms by:
//! 1. Loading a PDF form from quill's `form.pdf` file
//! 2. Extracting field names and descriptions
//! 3. Rendering field values using MiniJinja templates
//! 4. Writing rendered values back to the PDF form
//!
//! ## Test Requirements
//!
//! Tests require the `acroform` feature flag to be enabled.
//!
//! ## Design Reference
//!
//! See `prose/designs/ACROFORM.md` for backend design details.

#[cfg(test)]
#[cfg(feature = "acroform")]
mod tests {
    use acroform::{AcroFormDocument, FieldValue};
    use quillmark_acroform::AcroformBackend;

    #[test]
    fn test_backend_compilation() {
        use quillmark_core::{Backend, OutputFormat, Quill, RenderOptions};

        let backend = AcroformBackend::default();
        let quill_path = quillmark_fixtures::quills_path("usaf_form_8");
        let quill = Quill::from_path(quill_path).expect("Failed to load quill");

        let json_context = r#"{"test": "success!"}"#;

        let opts = RenderOptions {
            output_format: Some(OutputFormat::Pdf),
        };

        let result = backend.compile(json_context, &quill, &opts);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

        let render_result = result.unwrap();
        assert_eq!(render_result.artifacts.len(), 1);
        assert_eq!(render_result.artifacts[0].output_format, OutputFormat::Pdf);
        assert!(!render_result.artifacts[0].bytes.is_empty());
        assert_eq!(render_result.output_format, OutputFormat::Pdf);
    }

    #[test]
    fn test_field_type_preservation() {
        use quillmark_core::{Backend, OutputFormat, Quill, RenderOptions};

        let backend = AcroformBackend::default();
        let quill_path = quillmark_fixtures::quills_path("usaf_form_8");
        let quill = Quill::from_path(quill_path).expect("Failed to load quill");

        let json_context = r#"{
        "name": "John Doe",
        "other": {
            "restrictions": "On"
        }
    }"#;

        let opts = RenderOptions {
            output_format: Some(OutputFormat::Pdf),
        };

        let result = backend.compile(json_context, &quill, &opts);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

        let render_result = result.unwrap();
        assert_eq!(render_result.artifacts.len(), 1);
        assert_eq!(render_result.output_format, OutputFormat::Pdf);

        let filled_doc = AcroFormDocument::from_bytes(render_result.artifacts[0].bytes.clone())
            .expect("Failed to load filled PDF");

        let fields = filled_doc.fields().expect("Failed to get fields");

        // Text field should remain Text
        let name_field = fields
            .iter()
            .find(|f| f.name == "P[0].Page1[0].topmostSubform[0].MbrName[1]");
        if let Some(field) = name_field {
            assert!(matches!(field.current_value, Some(FieldValue::Text(_))));
        }

        // Choice field should remain Choice
        let restrictions_field = fields
            .iter()
            .find(|f| f.name == "P[0].Page1[0].topmostSubform[0].MbrRestct[0]");
        if let Some(field) = restrictions_field {
            if let Some(FieldValue::Choice(val)) = &field.current_value {
                assert_eq!(val, "On");
            } else {
                panic!("Expected Choice field type");
            }
        }
    }
}
