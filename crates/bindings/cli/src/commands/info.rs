use crate::errors::{CliError, Result};
use clap::Parser;
use quillmark::Quill;
use std::path::PathBuf;

#[derive(Parser)]
pub struct InfoArgs {
    /// Path to quill directory
    #[arg(value_name = "QUILL_PATH")]
    quill_path: PathBuf,

    /// Output as JSON instead of human-readable format
    #[arg(long)]
    json: bool,
}

pub fn execute(args: InfoArgs) -> Result<()> {
    // Validate quill path exists
    if !args.quill_path.exists() {
        return Err(CliError::InvalidArgument(format!(
            "Quill directory not found: {}",
            args.quill_path.display()
        )));
    }

    // Load Quill
    let quill = Quill::from_path(&args.quill_path)?;

    if args.json {
        print_json(&quill)?;
    } else {
        print_human_readable(&quill);
    }

    Ok(())
}

fn print_json(quill: &Quill) -> Result<()> {
    // Build a JSON object with the metadata
    let mut info = serde_json::Map::new();
    info.insert(
        "name".to_string(),
        serde_json::Value::String(quill.name.clone()),
    );
    info.insert(
        "backend".to_string(),
        serde_json::Value::String(quill.backend.clone()),
    );

    // Extract metadata fields: version, author, description
    if let Some(version) = quill.metadata.get("version") {
        info.insert("version".to_string(), version.as_json().clone());
    }
    if let Some(author) = quill.metadata.get("author") {
        info.insert("author".to_string(), author.as_json().clone());
    }
    if let Some(description) = quill.metadata.get("description") {
        info.insert("description".to_string(), description.as_json().clone());
    }

    // Add counts
    info.insert(
        "field_count".to_string(),
        serde_json::Value::Number(count_schema_fields(&quill.schema).into()),
    );
    let card_count = count_schema_cards(&quill.schema);
    if card_count > 0 {
        info.insert(
            "card_count".to_string(),
            serde_json::Value::Number(card_count.into()),
        );
    }
    info.insert(
        "has_plate".to_string(),
        serde_json::Value::Bool(quill.plate.is_some()),
    );
    info.insert(
        "has_example".to_string(),
        serde_json::Value::Bool(quill.example.is_some()),
    );

    // Add any additional metadata (excluding the standard fields already included)
    let mut extra_metadata = serde_json::Map::new();
    for (key, value) in &quill.metadata {
        if key != "backend" && key != "version" && key != "author" && key != "description" {
            extra_metadata.insert(key.clone(), value.as_json().clone());
        }
    }
    if !extra_metadata.is_empty() {
        info.insert(
            "metadata".to_string(),
            serde_json::Value::Object(extra_metadata),
        );
    }

    let json_str = serde_json::to_string_pretty(&serde_json::Value::Object(info))
        .map_err(|e| CliError::InvalidArgument(format!("Failed to serialize info: {}", e)))?;
    println!("{}", json_str);

    Ok(())
}

fn print_human_readable(quill: &Quill) {
    println!("Quill: {}", quill.name);

    if let Some(description) = quill.metadata.get("description") {
        if let Some(desc_str) = description.as_str() {
            if !desc_str.is_empty() {
                println!("  Description: {}", desc_str);
            }
        }
    }

    if let Some(version) = quill.metadata.get("version") {
        if let Some(ver_str) = version.as_str() {
            println!("  Version:     {}", ver_str);
        }
    }

    if let Some(author) = quill.metadata.get("author") {
        if let Some(auth_str) = author.as_str() {
            println!("  Author:      {}", auth_str);
        }
    }

    println!("  Backend:     {}", quill.backend);

    // Field count from schema properties
    let field_count = count_schema_fields(&quill.schema);
    println!("  Fields:      {}", field_count);

    // Card count from schema $defs
    let card_count = count_schema_cards(&quill.schema);
    if card_count > 0 {
        println!("  Cards:       {}", card_count);
    }

    // Defaults and examples
    let defaults_count = quill.extract_defaults().len();
    if defaults_count > 0 {
        println!("  Defaults:    {}", defaults_count);
    }

    let examples_count = quill.extract_examples().len();
    if examples_count > 0 {
        println!("  Examples:    {}", examples_count);
    }

    // Plate and example
    println!(
        "  Has plate:   {}",
        if quill.plate.is_some() { "yes" } else { "no" }
    );
    println!(
        "  Has example: {}",
        if quill.example.is_some() { "yes" } else { "no" }
    );

    // Additional metadata
    let extra_keys: Vec<&String> = quill
        .metadata
        .keys()
        .filter(|k| *k != "backend" && *k != "version" && *k != "author" && *k != "description")
        .collect();
    if !extra_keys.is_empty() {
        println!("  Metadata:");
        for key in extra_keys {
            if let Some(value) = quill.metadata.get(key) {
                println!("    {}: {}", key, format_metadata_value(value));
            }
        }
    }
}

/// Count top-level fields from schema properties (excluding BODY)
fn count_schema_fields(schema: &quillmark_core::QuillValue) -> usize {
    schema
        .as_json()
        .get("properties")
        .and_then(|p| p.as_object())
        .map(|props| {
            props
                .keys()
                .filter(|k| *k != "BODY" && *k != "CARDS")
                .count()
        })
        .unwrap_or(0)
}

/// Count card types from schema $defs
fn count_schema_cards(schema: &quillmark_core::QuillValue) -> usize {
    schema
        .as_json()
        .get("$defs")
        .and_then(|d| d.as_object())
        .map(|defs| defs.len())
        .unwrap_or(0)
}

fn format_metadata_value(value: &quillmark_core::QuillValue) -> String {
    let json = value.as_json();
    match json {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr
                .iter()
                .map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .collect();
            items.join(", ")
        }
        other => other.to_string(),
    }
}
