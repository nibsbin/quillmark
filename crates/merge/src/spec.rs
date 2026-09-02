//! The declarative merge spec: one serde model, YAML and JSON both deserialize.

use indexmap::IndexMap;
use quillmark_core::{quill_ref_hint, Diagnostic, Quill, QuillReference, Severity, Version};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MergeSpec {
    #[serde(rename = "$quill")]
    pub quill: String,
    #[serde(default)]
    pub mode: Mode,
    /// Main-card mappings keyed by schema address.
    #[serde(default)]
    pub map: IndexMap<String, Mapping>,
    /// `cards` mode: the column whose value groups rows into one document.
    #[serde(default)]
    pub group_by: Option<String>,
    /// `cards` mode: one card kind and its mappings.
    #[serde(default)]
    pub cards: IndexMap<String, CardMapping>,
    /// Filename pattern; `{field}` interpolates a main field.
    pub output: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Document,
    Cards,
}

/// Exactly one of `column` / `value`. `split` turns a cell into an array on the
/// separator; `format` is a strftime pattern a date cell is parsed with.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    pub column: Option<String>,
    pub value: Option<serde_json::Value>,
    pub split: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardMapping {
    pub map: IndexMap<String, Mapping>,
}

fn spec_error(code: &str, message: String) -> Diagnostic {
    Diagnostic::new(Severity::Error, message).with_code(format!("merge::{code}"))
}

impl MergeSpec {
    pub fn from_yaml(text: &str) -> Result<Self, Diagnostic> {
        serde_saphyr::from_str(text)
            .map_err(|e| spec_error("spec_parse", format!("merge spec does not parse: {e}")))
    }

    pub fn quill_ref(&self) -> Result<QuillReference, Diagnostic> {
        QuillReference::from_str(&self.quill).map_err(|e| {
            spec_error("spec_quill_ref", format!("invalid $quill '{}': {e}", self.quill))
                .with_hint(quill_ref_hint().to_string())
        })
    }

    /// Every column the spec reads.
    pub fn columns(&self) -> impl Iterator<Item = &str> {
        self.map
            .values()
            .chain(self.cards.values().flat_map(|c| c.map.values()))
            .filter_map(|m| m.column.as_deref())
            .chain(self.group_by.as_deref())
    }

    /// Spec-level checks against the quill: reference pairing, mode shape,
    /// mapping shape, and that every top-level target is a declared field.
    pub fn check(&self, quill: &Quill) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        match self.quill_ref() {
            Err(d) => out.push(d),
            Ok(r) => {
                if r.name != quill.name() {
                    out.push(spec_error(
                        "quill_mismatch",
                        format!("spec pins quill '{}', loaded quill is '{}'", r.name, quill.name()),
                    ));
                } else {
                    let version = quill
                        .metadata()
                        .get("version")
                        .and_then(|v| v.as_json().as_str())
                        .and_then(|s| Version::from_str(s).ok());
                    if let Some(v) = version {
                        if !r.selector.matches(v) {
                            out.push(spec_error(
                                "quill_mismatch",
                                format!("spec pins '{}', loaded quill is version {v}", self.quill),
                            ));
                        }
                    }
                }
            }
        }
        match self.mode {
            Mode::Document => {
                if self.group_by.is_some() || !self.cards.is_empty() {
                    out.push(spec_error(
                        "spec_mode",
                        "`group_by` and `cards` belong to `mode: cards`".to_string(),
                    ));
                }
            }
            Mode::Cards => {
                if self.group_by.is_none() {
                    out.push(spec_error("spec_mode", "`mode: cards` needs `group_by`".to_string()));
                }
                if self.cards.len() != 1 {
                    out.push(spec_error(
                        "spec_mode",
                        format!("`mode: cards` takes exactly one card kind, got {}", self.cards.len()),
                    ));
                }
                for kind in self.cards.keys() {
                    if quill.config().card_kind(kind).is_none() {
                        out.push(spec_error(
                            "spec_unknown_kind",
                            format!("quill '{}' declares no card kind '{kind}'", quill.name()),
                        ));
                    }
                }
            }
        }
        let main = &quill.config().main.fields;
        out.extend(check_mappings(&self.map, |top| main.contains_key(top), "main"));
        for (kind, card) in &self.cards {
            let fields = quill.config().card_kind(kind).map(|s| &s.fields);
            out.extend(check_mappings(
                &card.map,
                |top| fields.is_some_and(|f| f.contains_key(top)),
                kind,
            ));
        }
        out
    }

    /// Header checks: every named column exists; every unreferenced column
    /// warns once.
    pub fn check_header<'a>(&self, header: impl Iterator<Item = &'a str>) -> Vec<Diagnostic> {
        let header: Vec<&str> = header.collect();
        let mut out = Vec::new();
        let mut used: Vec<&str> = Vec::new();
        for column in self.columns() {
            used.push(column);
            if !header.contains(&column) {
                out.push(spec_error(
                    "unknown_column",
                    format!("spec reads column '{column}', which the input has no header for"),
                ));
            }
        }
        for column in header {
            if !used.contains(&column) {
                out.push(
                    Diagnostic::new(
                        Severity::Warning,
                        format!("column '{column}' is not mapped and will be ignored"),
                    )
                    .with_code("merge::unmapped_column".to_string()),
                );
            }
        }
        out
    }
}

fn check_mappings(
    map: &IndexMap<String, Mapping>,
    declared: impl Fn(&str) -> bool,
    scope: &str,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (address, mapping) in map {
        if mapping.column.is_some() == mapping.value.is_some() {
            out.push(spec_error(
                "spec_mapping",
                format!("mapping '{address}' needs exactly one of `column` / `value`"),
            ));
        }
        let top = address.split('.').next().unwrap_or_default();
        if top.is_empty() || !declared(top) {
            out.push(spec_error(
                "spec_unknown_target",
                format!("'{scope}' declares no field '{top}' for target '{address}'"),
            ));
        }
    }
    out
}
