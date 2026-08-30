use crate::errors::{CliError, Result};
use clap::Parser;
use quillmark_core::quill::{CardSchema, FieldSchema, QuillConfig};
use quillmark_core::{Diagnostic, Severity};
use indexmap::IndexMap;
use std::path::{Path, PathBuf};

#[derive(Parser)]
pub struct ValidateArgs {
    /// Path to quill directory
    #[arg(value_name = "QUILL_PATH")]
    quill_path: PathBuf,

    /// Show verbose output with all validation details
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Default)]
struct ValidationResult {
    issues: Vec<Diagnostic>,
}

impl ValidationResult {
    fn new() -> Self {
        Self { issues: Vec::new() }
    }

    fn add_error(&mut self, message: impl Into<String>, code: &str) {
        self.issues
            .push(Diagnostic::new(Severity::Error, message.into()).with_code(code.to_string()));
    }

    fn add_warning(&mut self, message: impl Into<String>, code: &str) {
        self.issues
            .push(Diagnostic::new(Severity::Warning, message.into()).with_code(code.to_string()));
    }

    fn count(&self, severity: Severity) -> usize {
        self.issues.iter().filter(|d| d.severity == severity).count()
    }

    fn has_errors(&self) -> bool {
        self.count(Severity::Error) > 0
    }
}

pub fn execute(args: ValidateArgs) -> Result<()> {
    // Gated on the directory: the loader below owns the missing-path message,
    // and an ungated pre-check answers a typo'd path with the bundle's contents
    // again. What it adds is the path on a real directory, which the tree
    // loader cannot name.
    let quill_yaml_path = args.quill_path.join("Quill.yaml");
    if args.quill_path.is_dir() && !quill_yaml_path.exists() {
        return Err(CliError::InvalidArgument(format!(
            "Quill.yaml not found in: {}",
            args.quill_path.display()
        )));
    }

    if args.verbose {
        println!("Validating quill at: {}", args.quill_path.display());
    }

    let mut result = ValidationResult::new();

    // `_with_warnings` keeps the config warnings the plain loader drops.
    let (quill, config_warnings) = match quillmark::quill_from_path_with_warnings(&args.quill_path) {
        Ok(pair) => pair,
        Err(e) => {
            for diag in e.diagnostics() {
                eprintln!("{}", diag.fmt_pretty());
            }
            // The branch covers every load failure, a missing directory and an
            // unreadable `Quill.yaml` among them, so it names neither.
            eprintln!("\nValidation failed: {} error(s)", e.diagnostics().len());
            return Err(CliError::InvalidArgument(
                "Quill could not be loaded".to_string(),
            ));
        }
    };
    let config = quill.config();

    result.issues.extend(config_warnings);

    if args.verbose {
        println!("  Quill name: {}", config.name);
        println!("  Backend: {}", config.backend);
        println!("  Fields: {}", config.main.fields.len());
        println!("  Cards: {}", config.card_kinds.len());
        println!("  Schema generated successfully");
        println!("  Defaults extracted: {}", config.main.defaults().len());
    }

    validate_file_references(&args.quill_path, config, &mut result);

    validate_field_schemas(&config.main.fields, &mut result, "field");

    for card_schema in &config.card_kinds {
        validate_card_schema(&card_schema.name, card_schema, &mut result);
    }

    print_validation_result(&result, args.verbose);

    if result.has_errors() {
        Err(CliError::InvalidArgument(format!(
            "Validation failed with {} error(s)",
            result.count(Severity::Error)
        )))
    } else {
        Ok(())
    }
}

fn validate_file_references(
    quill_path: &Path,
    config: &QuillConfig,
    result: &mut ValidationResult,
) {
    // `plate_file` comes from the untrusted Quill.yaml, so reject anything but
    // a simple relative filename before touching the filesystem: an absolute
    // `Path::join` replaces the base and `..` escapes the quill root, either of
    // which turns `plate_path.exists()` into a host path-probing oracle.
    if let Some(plate_file) = config
        .backend_config
        .get("plate_file")
        .and_then(|v| v.as_str())
    {
        let rel = Path::new(plate_file);
        if rel
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            result.add_error(
                format!(
                    "plate_file '{}' must be a relative path within the quill (no '..' or absolute components)",
                    plate_file
                ),
                "cli::plate_file_escapes_quill",
            );
        } else {
            let plate_path = quill_path.join(rel);
            if !plate_path.exists() {
                result.add_error(
                    format!("Referenced plate_file '{}' does not exist", plate_file),
                    "cli::plate_file_missing",
                );
            }
        }
    }
}

/// The one advisory check config parsing does not already make: `example:` and
/// `default:` literal errors are caught authoritatively at load time.
fn validate_field_schemas(
    fields: &IndexMap<String, FieldSchema>,
    result: &mut ValidationResult,
    context: &str,
) {
    for (field_name, field_schema) in fields {
        if field_schema
            .description
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            result.add_warning(
                format!("{context} '{field_name}': missing or empty description"),
                "cli::missing_description",
            );
        }
    }
}

fn validate_card_schema(card_name: &str, card_schema: &CardSchema, result: &mut ValidationResult) {
    if card_schema
        .description
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        result.add_warning(
            format!("card '{}': missing or empty description", card_name),
            "cli::missing_description",
        );
    }

    let context = format!("card '{}' field", card_name);
    validate_field_schemas(&card_schema.fields, result, &context);
}

fn print_validation_result(result: &ValidationResult, verbose: bool) {
    let error_count = result.count(Severity::Error);
    let warning_count = result.count(Severity::Warning);

    // `-v` adds warnings; errors always print.
    for diag in &result.issues {
        if diag.severity == Severity::Error || verbose {
            eprintln!("{}", diag.fmt_pretty());
        }
    }

    if error_count == 0 && warning_count == 0 {
        println!("Validation passed: quill configuration is valid");
    } else if error_count == 0 {
        println!("Validation passed with {} warning(s)", warning_count);
    } else {
        eprintln!(
            "Validation failed: {} error(s), {} warning(s)",
            error_count, warning_count
        );
    }
}
