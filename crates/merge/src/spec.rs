//! The declarative merge spec: one serde model, YAML and JSON both deserialize.

use indexmap::IndexMap;
use quillmark_core::{quill_ref_hint, Diagnostic, Quill, QuillReference, Severity, Version};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str::FromStr;

use crate::address;

/// A merge spec. Pins the quill the way a document does, names the mode, maps
/// columns onto schema addresses, and patterns the output name.
///
/// ```yaml
/// $quill: certificate@1.2.0
/// map:
///   recipient:  { column: Name }
///   awarded_on: { column: Date, format: "%m/%d/%Y" }
///   event:      { value: "Rustconf 2026" }
/// output: "{recipient}-certificate"
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct MergeSpec {
    /// `name@selector`, checked against the loaded quill.
    #[serde(rename = "$quill")]
    pub quill: String,
    #[serde(default)]
    pub mode: Mode,
    /// `document` mode: the column whose value keys each row. Absent, the
    /// output name is the key.
    #[serde(default)]
    pub key: Option<String>,
    /// Main-card mappings keyed by target address. A header that is itself an
    /// address maps without an entry here; an entry overrides.
    #[serde(default)]
    pub map: IndexMap<String, Mapping>,
    /// `cards` mode: the column whose value groups rows into one document.
    #[serde(default)]
    pub group_by: Option<String>,
    /// `cards` mode: the one card kind a row becomes, and its mappings.
    #[serde(default)]
    pub cards: IndexMap<String, CardMapping>,
    /// Output file stem; `{field}` interpolates a main field's lowered value.
    /// The surface appends the format's extension.
    pub output: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// One row is one document.
    #[default]
    Document,
    /// Rows sharing `group_by` are one document, one card per row.
    Cards,
}

/// Where one target's value comes from: a `column` of the input or a constant
/// `value`, exactly one of the two. `split` turns a cell into an array on the
/// separator; `format` is the strftime pattern a date cell is written in.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Mapping {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

impl Mapping {
    pub fn column(name: impl Into<String>) -> Self {
        Self {
            column: Some(name.into()),
            ..Self::default()
        }
    }

    pub fn value(value: impl Into<serde_json::Value>) -> Self {
        Self {
            value: Some(value.into()),
            ..Self::default()
        }
    }

    pub fn with_split(mut self, separator: impl Into<String>) -> Self {
        self.split = Some(separator.into());
        self
    }

    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct CardMapping {
    #[serde(default)]
    pub map: IndexMap<String, Mapping>,
}

impl CardMapping {
    pub fn new(map: IndexMap<String, Mapping>) -> Self {
        Self { map }
    }
}

pub(crate) fn spec_error(code: &str, message: String) -> Diagnostic {
    Diagnostic::new(Severity::Error, message).with_code(format!("merge::{code}"))
}

impl MergeSpec {
    /// A `document`-mode spec with no mappings: identity mapping over the
    /// input's headers.
    pub fn new(quill: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            quill: quill.into(),
            mode: Mode::Document,
            key: None,
            map: IndexMap::new(),
            group_by: None,
            cards: IndexMap::new(),
            output: output.into(),
        }
    }

    pub fn from_yaml(text: &str) -> Result<Self, Diagnostic> {
        serde_saphyr::from_str(text)
            .map_err(|e| spec_error("spec_parse", format!("merge spec does not parse: {e}")))
    }

    pub fn from_json(text: &str) -> Result<Self, Diagnostic> {
        serde_json::from_str(text)
            .map_err(|e| spec_error("spec_parse", format!("merge spec does not parse: {e}")))
    }

    pub fn quill_ref(&self) -> Result<QuillReference, Diagnostic> {
        QuillReference::from_str(&self.quill).map_err(|e| {
            spec_error("spec_quill_ref", format!("invalid $quill '{}': {e}", self.quill))
                .with_hint(quill_ref_hint().to_string())
        })
    }

    /// SHA-256 of the canonical JSON form, hex. The same spec spelled with
    /// other whitespace or comments hashes the same; a changed mapping does
    /// not.
    pub fn hash(&self) -> String {
        let bytes = serde_json::to_vec(self).expect("a spec serializes");
        hex(&Sha256::digest(bytes))
    }

    /// Every column the spec reads.
    pub fn columns(&self) -> impl Iterator<Item = &str> {
        self.map
            .values()
            .chain(self.cards.values().flat_map(|c| c.map.values()))
            .filter_map(|m| m.column.as_deref())
            .chain(self.group_by.as_deref())
            .chain(self.key.as_deref())
    }

    /// Spec-level checks against the quill: reference pairing, mode shape,
    /// mapping shape, and every target resolving through the schema.
    pub fn check(&self, quill: &Quill) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        match self.quill_ref() {
            Err(d) => out.push(d),
            Ok(r) => {
                if r.name != quill.name() {
                    out.push(spec_error(
                        "quill_mismatch",
                        format!(
                            "spec pins quill '{}', loaded quill is '{}'",
                            r.name,
                            quill.name()
                        ),
                    ));
                } else if let Some(v) = quill_version(quill) {
                    if !r.selector.matches(v) {
                        out.push(spec_error(
                            "quill_mismatch",
                            format!(
                                "spec pins '{}', loaded quill is version {v}",
                                self.quill
                            ),
                        ));
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
                    out.push(spec_error(
                        "spec_mode",
                        "`mode: cards` needs `group_by`".to_string(),
                    ));
                }
                if self.key.is_some() {
                    out.push(spec_error(
                        "spec_mode",
                        "`key` belongs to `mode: document`; `group_by` keys a cards-mode document"
                            .to_string(),
                    ));
                }
                if self.cards.len() != 1 {
                    out.push(spec_error(
                        "spec_mode",
                        format!(
                            "`mode: cards` takes exactly one card kind, got {}",
                            self.cards.len()
                        ),
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
        out.extend(check_mappings(&self.map, &quill.config().main.fields, "main"));
        for (kind, card) in &self.cards {
            if let Some(schema) = quill.config().card_kind(kind) {
                out.extend(check_mappings(&card.map, &schema.fields, kind));
            }
        }
        match output_tokens(&self.output) {
            Err(reason) => out.push(spec_error(
                "spec_output",
                format!("output pattern '{}': {reason}", self.output),
            )),
            Ok(tokens) => {
                for token in tokens {
                    if let Token::Field(name) = token {
                        if !quill.config().main.fields.contains_key(&name) {
                            out.push(spec_error(
                                "spec_output",
                                format!(
                                    "output pattern names `{{{name}}}`, which 'main' does not declare"
                                ),
                            ));
                        }
                    }
                }
            }
        }
        out
    }
}

/// One piece of an output pattern.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    Literal(String),
    Field(String),
}

/// Split an output pattern into literal runs and `{field}` tokens.
pub(crate) fn output_tokens(pattern: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut rest = pattern;
    while let Some(open) = rest.find('{') {
        if open > 0 {
            tokens.push(Token::Literal(rest[..open].to_string()));
        }
        let close = rest[open..]
            .find('}')
            .ok_or_else(|| "unclosed `{`".to_string())?;
        let name = &rest[open + 1..open + close];
        if name.is_empty() {
            return Err("empty `{}` token".to_string());
        }
        tokens.push(Token::Field(name.to_string()));
        rest = &rest[open + close + 1..];
    }
    if rest.contains('}') {
        return Err("unmatched `}`".to_string());
    }
    if !rest.is_empty() {
        tokens.push(Token::Literal(rest.to_string()));
    }
    Ok(tokens)
}

fn quill_version(quill: &Quill) -> Option<Version> {
    quill
        .metadata()
        .get("version")
        .and_then(|v| v.as_json().as_str())
        .and_then(|s| Version::from_str(s).ok())
}

fn check_mappings(
    map: &IndexMap<String, Mapping>,
    fields: &IndexMap<String, quillmark_core::FieldSchema>,
    scope: &str,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (target, mapping) in map {
        if mapping.column.is_some() == mapping.value.is_some() {
            out.push(spec_error(
                "spec_mapping",
                format!("mapping '{target}' needs exactly one of `column` / `value`"),
            ));
        }
        if let Err(reason) = address::resolve(fields, target) {
            out.push(spec_error(
                "spec_unknown_target",
                format!("target '{target}' on '{scope}': {reason}"),
            ));
        }
    }
    out
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
