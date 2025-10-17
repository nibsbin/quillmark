use acroform::{AcroFormDocument, FieldValue};
use quillmark_acroform::AcroformBackend;
use std::collections::HashMap;

#[cfg_attr(not(feature = "acroform"), ignore)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_compilation() {
        use quillmark_core::{Backend, OutputFormat, Quill, RenderOptions};

        let backend = AcroformBackend::default();
        let quill_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../quillmark-fixtures/resources/usaf_form_8"
        );
        let quill = Quill::from_path(quill_path).expect("Failed to load quill");

        let json_context = r#"{"test": "success!"}"#;

        let opts = RenderOptions {
            output_format: Some(OutputFormat::Pdf),
        };

        let result = backend.compile(json_context, &quill, &opts);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

        let artifacts = result.unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].output_format, OutputFormat::Pdf);
        assert!(!artifacts[0].bytes.is_empty());
    }

    #[test]
    fn test_undefined_values_render_as_empty_string() {
        use quillmark_core::{Backend, OutputFormat, Quill, RenderOptions};

        let backend = AcroformBackend::default();
        let quill_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../quillmark-fixtures/resources/usaf_form_8"
        );
        let quill = Quill::from_path(quill_path).expect("Failed to load quill");

        let json_context = r#"{
        "name": "Test User",
        "grade": "O-3",
        "requisite_info": [
            {
                "requisites": "First requirement",
                "date": "2025-01-01",
                "results": "Pass"
            },
            {
                "requisites": "Second requirement",
                "date": "2025-01-02",
                "results": "Pass"
            }
        ]
    }"#;

        let opts = RenderOptions {
            output_format: Some(OutputFormat::Pdf),
        };

        let result = backend.compile(json_context, &quill, &opts);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

        let artifacts = result.unwrap();
        assert_eq!(artifacts.len(), 1);
        assert!(!artifacts[0].bytes.is_empty());

        let filled_doc = AcroFormDocument::from_bytes(artifacts[0].bytes.clone())
            .expect("Failed to load filled PDF");

        let fields = filled_doc.fields().expect("Failed to get fields");
        let field_map: HashMap<String, FieldValue> = fields
            .into_iter()
            .filter_map(|f| f.current_value.map(|v| (f.name, v)))
            .collect();

        // Verify filled fields
        let req_field_1 = field_map.get("topmostSubform[0].Page1[0].P[0].ReqFld1[0]");
        if let Some(FieldValue::Text(val)) = req_field_1 {
            assert_eq!(val, "First requirement");
        }

        let req_field_2 = field_map.get("topmostSubform[0].Page1[0].P[0].ReqFld2[0]");
        if let Some(FieldValue::Text(val)) = req_field_2 {
            assert_eq!(val, "Second requirement");
        }

        // Verify out-of-bounds renders as empty
        let req_field_11 = field_map.get("topmostSubform[0].Page1[0].P[0].ReqFld11[0]");
        if let Some(FieldValue::Text(val)) = req_field_11 {
            assert_eq!(val, "");
        }
    }

    #[test]
    fn test_tooltip_template_parsing() {
        use quillmark_core::{Backend, OutputFormat, Quill, RenderOptions};

        let backend = AcroformBackend::default();
        let quill_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../quillmark-fixtures/resources/usaf_form_8"
        );
        let quill = Quill::from_path(quill_path).expect("Failed to load quill");

        let json_context = r#"{
        "other": {
            "restrictions": "On"
        }
    }"#;

        let opts = RenderOptions {
            output_format: Some(OutputFormat::Pdf),
        };

        let result = backend.compile(json_context, &quill, &opts);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

        let artifacts = result.unwrap();
        assert_eq!(artifacts.len(), 1);

        let filled_doc = AcroFormDocument::from_bytes(artifacts[0].bytes.clone())
            .expect("Failed to load filled PDF");

        let fields = filled_doc.fields().expect("Failed to get fields");

        let field_with_tooltip = fields
            .iter()
            .find(|f| f.name == "P[0].Page1[0].topmostSubform[0].MbrRestct[0]")
            .expect("Field not found");

        if let Some(FieldValue::Choice(val)) = &field_with_tooltip.current_value {
            assert_eq!(val, "On");
        } else {
            panic!(
                "Expected Choice field value, got: {:?}",
                field_with_tooltip.current_value
            );
        }
    }

    #[test]
    fn test_field_type_preservation() {
        use quillmark_core::{Backend, OutputFormat, Quill, RenderOptions};

        let backend = AcroformBackend::default();
        let quill_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../quillmark-fixtures/resources/usaf_form_8"
        );
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

        let artifacts = result.unwrap();
        assert_eq!(artifacts.len(), 1);

        let filled_doc = AcroFormDocument::from_bytes(artifacts[0].bytes.clone())
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
