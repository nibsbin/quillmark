use crate::errors::{CliError, Result};
use clap::Parser;
use quillmark::Quill;
use quillmark_core::quill::{CardSchema, FieldSchema, FieldType, QuillConfig};
use quillmark_core::QuillValue;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
pub struct ValidateArgs {
    /// Path to quill directory
    #[arg(value_name = "QUILL_PATH")]
    quill_path: PathBuf,

    /// Show verbose output with all validation details
    #[arg(short, long)]
    verbose: bool,
}

/// Validation issue severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Error,
    Warning,
}

/// A single validation issue
#[derive(Debug)]
struct ValidationIssue {
    severity: Severity,
    message: String,
}

impl ValidationIssue {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
        }
    }
}

/// Result of validating a Quill configuration
#[derive(Debug, Default)]
struct ValidationResult {
    issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    fn new() -> Self {
        Self { issues: Vec::new() }
    }

    fn add_error(&mut self, message: impl Into<String>) {
        self.issues.push(ValidationIssue::error(message));
    }

    fn add_warning(&mut self, message: impl Into<String>) {
        self.issues.push(ValidationIssue::warning(message));
    }

    fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }

    fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }
}

pub fn execute(args: ValidateArgs) -> Result<()> {
    // Validate quill path exists
    if !args.quill_path.exists() {
        return Err(CliError::InvalidArgument(format!(
            "Quill directory not found: {}",
            args.quill_path.display()
        )));
    }

    let quill_toml_path = args.quill_path.join("Quill.toml");
    if !quill_toml_path.exists() {
        return Err(CliError::InvalidArgument(format!(
            "Quill.toml not found in: {}",
            args.quill_path.display()
        )));
    }

    if args.verbose {
        println!("Validating quill at: {}", args.quill_path.display());
    }

    let mut result = ValidationResult::new();

    // Step 1: Parse the TOML config first (before full Quill load)
    let toml_content = fs::read_to_string(&quill_toml_path).map_err(CliError::Io)?;

    let config = match QuillConfig::from_toml(&toml_content) {
        Ok(c) => c,
        Err(e) => {
            result.add_error(format!("Failed to parse Quill.toml: {}", e));
            print_validation_result(&result, args.verbose);
            return Err(CliError::InvalidArgument(
                "Quill configuration is invalid".to_string(),
            ));
        }
    };

    if args.verbose {
        println!("  Quill name: {}", config.name);
        println!("  Backend: {}", config.backend);
        println!("  Fields: {}", config.fields.len());
        println!("  Cards: {}", config.cards.len());
    }

    // Step 2: Validate file references
    validate_file_references(&args.quill_path, &config, &mut result);

    // Step 3: Validate field schemas including defaults
    validate_field_schemas(&config.fields, &mut result, "field");

    // Step 4: Validate card schemas
    for (card_name, card_schema) in &config.cards {
        validate_card_schema(card_name, card_schema, &mut result);
    }

    // Step 5: Try to load the full Quill (this validates schema generation)
    match Quill::from_path(&args.quill_path) {
        Ok(quill) => {
            if args.verbose {
                println!("  Schema generated successfully");
                println!(
                    "  Defaults extracted: {}",
                    quill.extract_defaults().len()
                );
            }

            // Step 6: Validate extracted defaults match schema types
            validate_defaults_against_schema(&quill, &config, &mut result);
        }
        Err(e) => {
            result.add_error(format!("Failed to load Quill: {}", e));
        }
    }

    // Print results
    print_validation_result(&result, args.verbose);

    if result.has_errors() {
        Err(CliError::InvalidArgument(format!(
            "Validation failed with {} error(s)",
            result.error_count()
        )))
    } else {
        Ok(())
    }
}

fn validate_file_references(
    quill_path: &PathBuf,
    config: &QuillConfig,
    result: &mut ValidationResult,
) {
    // Check plate_file reference
    if let Some(ref plate_file) = config.plate_file {
        let plate_path = quill_path.join(plate_file);
        if !plate_path.exists() {
            result.add_error(format!(
                "Referenced plate_file '{}' does not exist",
                plate_file
            ));
        }
    }

    // Check example_file reference
    if let Some(ref example_file) = config.example_file {
        let example_path = quill_path.join(example_file);
        if !example_path.exists() {
            result.add_warning(format!(
                "Referenced example_file '{}' does not exist",
                example_file
            ));
        }
    }
}

fn validate_field_schemas(
    fields: &HashMap<String, FieldSchema>,
    result: &mut ValidationResult,
    context: &str,
) {
    for (field_name, field_schema) in fields {
        // Validate default value type matches declared type
        if let (Some(ref field_type), Some(ref default)) =
            (&field_schema.r#type, &field_schema.default)
        {
            if let Some(type_mismatch) = check_type_mismatch(field_type, default) {
                result.add_error(format!(
                    "{} '{}': default value {} but field type is '{}'",
                    context,
                    field_name,
                    type_mismatch,
                    field_type.as_str()
                ));
            }
        }

        // Validate enum values are strings if specified
        if let Some(ref enum_values) = field_schema.enum_values {
            if enum_values.is_empty() {
                result.add_warning(format!(
                    "{} '{}': enum constraint is empty",
                    context, field_name
                ));
            }

            // If there's a default, check it's in the enum
            if let Some(ref default) = field_schema.default {
                if let Some(default_str) = default.as_str() {
                    if !enum_values.contains(&default_str.to_string()) {
                        result.add_error(format!(
                            "{} '{}': default value '{}' is not in enum values {:?}",
                            context, field_name, default_str, enum_values
                        ));
                    }
                }
            }
        }

        // Warn about missing description
        if field_schema.description.trim().is_empty() {
            result.add_warning(format!(
                "{} '{}': missing or empty description",
                context, field_name
            ));
        }
    }
}

fn validate_card_schema(
    card_name: &str,
    card_schema: &CardSchema,
    result: &mut ValidationResult,
) {
    // Warn about missing description
    if card_schema.description.trim().is_empty() {
        result.add_warning(format!(
            "card '{}': missing or empty description",
            card_name
        ));
    }

    // Validate card fields
    let context = format!("card '{}' field", card_name);
    validate_field_schemas(&card_schema.fields, result, &context);
}

fn check_type_mismatch(field_type: &FieldType, value: &QuillValue) -> Option<String> {
    let json_value = value.as_json();

    match field_type {
        FieldType::Str | FieldType::String => {
            if !json_value.is_string() {
                Some(format!(
                    "is {} (not a string)",
                    describe_json_type(json_value)
                ))
            } else {
                None
            }
        }
        FieldType::Number => {
            if !json_value.is_number() {
                Some(format!(
                    "is {} (not a number)",
                    describe_json_type(json_value)
                ))
            } else {
                None
            }
        }
        FieldType::Boolean => {
            if !json_value.is_boolean() {
                Some(format!(
                    "is {} (not a boolean)",
                    describe_json_type(json_value)
                ))
            } else {
                None
            }
        }
        FieldType::Array => {
            if !json_value.is_array() {
                Some(format!(
                    "is {} (not an array)",
                    describe_json_type(json_value)
                ))
            } else {
                None
            }
        }
        FieldType::Dict => {
            if !json_value.is_object() {
                Some(format!(
                    "is {} (not an object)",
                    describe_json_type(json_value)
                ))
            } else {
                None
            }
        }
        FieldType::Date | FieldType::DateTime => {
            // Date/DateTime are stored as strings
            if !json_value.is_string() {
                Some(format!(
                    "is {} (not a string)",
                    describe_json_type(json_value)
                ))
            } else {
                None
            }
        }
    }
}

fn describe_json_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "a boolean",
        serde_json::Value::Number(_) => "a number",
        serde_json::Value::String(_) => "a string",
        serde_json::Value::Array(_) => "an array",
        serde_json::Value::Object(_) => "an object",
    }
}

fn validate_defaults_against_schema(
    quill: &Quill,
    config: &QuillConfig,
    result: &mut ValidationResult,
) {
    let defaults = quill.extract_defaults();

    for (field_name, default_value) in defaults {
        // Look up field type in config
        if let Some(field_schema) = config.fields.get(field_name) {
            if let Some(ref field_type) = field_schema.r#type {
                if let Some(type_mismatch) = check_type_mismatch(field_type, default_value) {
                    result.add_error(format!(
                        "extracted default for '{}' {}, expected '{}'",
                        field_name,
                        type_mismatch,
                        field_type.as_str()
                    ));
                }
            }
        }
    }
}

fn print_validation_result(result: &ValidationResult, verbose: bool) {
    let error_count = result.error_count();
    let warning_count = result.warning_count();

    // Print issues
    for issue in &result.issues {
        match issue.severity {
            Severity::Error => eprintln!("[ERROR] {}", issue.message),
            Severity::Warning => {
                if verbose {
                    eprintln!("[WARNING] {}", issue.message)
                }
            }
        }
    }

    // Print summary
    if error_count == 0 && warning_count == 0 {
        println!("Validation passed: quill configuration is valid");
    } else if error_count == 0 {
        println!(
            "Validation passed with {} warning(s)",
            warning_count
        );
    } else {
        eprintln!(
            "Validation failed: {} error(s), {} warning(s)",
            error_count, warning_count
        );
    }
}
