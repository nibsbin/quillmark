//! Bulk document generation: a [`MergeSpec`] interpreted over JSON rows into
//! `Document`s plus a per-row report. Engine-free: the plan hands documents to
//! whatever render loop the surface runs.
//!
//! Spike for borb-sh/quillmark#1446.

mod spec;

pub use spec::{CardMapping, Mapping, MergeSpec, Mode};

use indexmap::IndexMap;
use quillmark_core::{
    CardValues, Diagnostic, DocPath, DocSeg, Document, DocumentValues, Quill, QuillReference,
    Severity,
};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

/// One input row, keyed by header. Cells arrive as strings from a spreadsheet
/// edge and as native JSON from an API caller; the typed writer judges both.
pub type Row = IndexMap<String, Value>;

/// A diagnostic anchored to its source row and, where the path reverse-maps to
/// a mapping, its column. `row` is the 0-based data row; `None` is spec-level.
#[derive(Debug, Clone)]
pub struct RowDiagnostic {
    pub row: Option<usize>,
    pub column: Option<String>,
    pub diagnostic: Diagnostic,
}

#[derive(Debug)]
pub struct PlannedDocument {
    /// The row index in `document` mode, the `group_by` value in `cards` mode.
    pub key: String,
    pub rows: Vec<usize>,
    pub filename: String,
    pub document: Document,
}

#[derive(Debug, Default)]
pub struct MergePlan {
    pub documents: Vec<PlannedDocument>,
    pub report: Vec<RowDiagnostic>,
    pub skipped_empty: usize,
}

impl MergePlan {
    pub fn is_clean(&self) -> bool {
        !self.report.iter().any(|d| d.diagnostic.severity == Severity::Error)
    }

    /// The documents no error touches: what `--force` renders.
    pub fn clean_documents(&self) -> impl Iterator<Item = &PlannedDocument> {
        let dirty: Vec<usize> = self
            .report
            .iter()
            .filter(|d| d.diagnostic.severity == Severity::Error)
            .filter_map(|d| d.row)
            .collect();
        self.documents
            .iter()
            .filter(move |doc| !doc.rows.iter().any(|r| dirty.contains(r)))
    }
}

pub fn plan(quill: &Quill, spec: &MergeSpec, rows: &[Row]) -> MergePlan {
    let mut plan = MergePlan::default();
    let mut spec_diags = spec.check(quill);
    if let Some(first) = rows.first() {
        spec_diags.extend(spec.check_header(first.keys().map(String::as_str)));
    }
    plan.report.extend(spec_diags.into_iter().map(|diagnostic| RowDiagnostic {
        row: None,
        column: None,
        diagnostic,
    }));
    if !plan.is_clean() {
        return plan;
    }
    let quill_ref = spec.quill_ref().expect("checked above");
    let mut filenames: HashMap<String, String> = HashMap::new();

    match spec.mode {
        Mode::Document => {
            for (i, row) in rows.iter().enumerate() {
                if is_empty_row(row) {
                    plan.skipped_empty += 1;
                    continue;
                }
                let fields = lower(&spec.map, row, i, &mut plan.report);
                let values = DocumentValues::new(fields.clone());
                let locate = |path: &str| (Some(i), reverse_main(&spec.map, path));
                let key = i.to_string();
                let Some(document) = build(quill, &quill_ref, &values, &key, &locate, &mut plan.report)
                else {
                    continue;
                };
                let Some(filename) =
                    output_name(&spec.output, &fields, &key, i, &mut filenames, &mut plan.report)
                else {
                    continue;
                };
                plan.documents.push(PlannedDocument {
                    key,
                    rows: vec![i],
                    filename,
                    document,
                });
            }
        }
        Mode::Cards => {
            let group_col = spec.group_by.as_deref().expect("checked above");
            let (kind, card_map) = spec.cards.iter().next().expect("checked above");
            let mut groups: IndexMap<String, Vec<usize>> = IndexMap::new();
            for (i, row) in rows.iter().enumerate() {
                if is_empty_row(row) {
                    plan.skipped_empty += 1;
                    continue;
                }
                match row.get(group_col).and_then(cell_text) {
                    Some(key) => groups.entry(key).or_default().push(i),
                    None => plan.report.push(RowDiagnostic {
                        row: Some(i),
                        column: Some(group_col.to_string()),
                        diagnostic: merge_error(
                            "missing_group_key",
                            format!("row {i} has no value in the `group_by` column '{group_col}'"),
                        ),
                    }),
                }
            }
            for (key, members) in groups {
                let first = members[0];
                let fields = lower(&spec.map, &rows[first], first, &mut plan.report);
                let mut conflict = false;
                for &i in &members[1..] {
                    let other = lower(&spec.map, &rows[i], i, &mut plan.report);
                    for (address, mapping) in &spec.map {
                        if value_at(&fields, address) != value_at(&other, address) {
                            conflict = true;
                            plan.report.push(RowDiagnostic {
                                row: Some(i),
                                column: mapping.column.clone(),
                                diagnostic: merge_error(
                                    "group_conflict",
                                    format!(
                                        "main field '{address}' differs from row {first} within group '{key}'; a main-mapped column must be constant across the group"
                                    ),
                                )
                                .with_path(DocPath::main().field(address).to_string()),
                            });
                        }
                    }
                }
                let cards: Vec<CardValues> = members
                    .iter()
                    .map(|&i| CardValues::new(kind.clone(), lower(&card_map.map, &rows[i], i, &mut plan.report)))
                    .collect();
                if conflict {
                    continue;
                }
                let values = DocumentValues::new(fields.clone()).with_cards(cards);
                let locate = |path: &str| match DocPath::from_str(path).ok().as_ref().map(|p| p.segs()) {
                    Some([DocSeg::Card { index, .. }, rest @ ..]) => {
                        (members.get(*index).copied(), reverse(&card_map.map, rest))
                    }
                    Some([DocSeg::Main, rest @ ..]) => (Some(first), reverse(&spec.map, rest)),
                    _ => (None, None),
                };
                let Some(document) = build(quill, &quill_ref, &values, &key, &locate, &mut plan.report)
                else {
                    continue;
                };
                let Some(filename) =
                    output_name(&spec.output, &fields, &key, first, &mut filenames, &mut plan.report)
                else {
                    continue;
                };
                plan.documents.push(PlannedDocument {
                    key,
                    rows: members,
                    filename,
                    document,
                });
            }
        }
    }
    plan
}

fn merge_error(code: &str, message: String) -> Diagnostic {
    Diagnostic::new(Severity::Error, message).with_code(format!("merge::{code}"))
}

/// Construct through the typed writer, then validate. Every refusal and every
/// validation diagnostic lands in `report` under its row and column; a
/// document any error touches is not returned.
fn build(
    quill: &Quill,
    quill_ref: &QuillReference,
    values: &DocumentValues,
    key: &str,
    locate: &dyn Fn(&str) -> (Option<usize>, Option<String>),
    report: &mut Vec<RowDiagnostic>,
) -> Option<Document> {
    let mut doc = Document::new(quill_ref.clone());
    if let Err(refusals) = quill.writer(&mut doc).set_values(values) {
        for (path, err) in refusals {
            let path = path.to_string();
            let (row, column) = locate(&path);
            report.push(RowDiagnostic {
                row,
                column,
                diagnostic: Diagnostic::new(Severity::Error, err.to_string())
                    .with_code(err.code().to_string())
                    .with_args(err.args())
                    .with_path(path),
            });
        }
        return None;
    }
    let mut fatal = false;
    for diagnostic in quill.validate(&doc) {
        fatal |= diagnostic.severity == Severity::Error;
        let (row, column) = diagnostic
            .path
            .as_deref()
            .map(locate)
            .unwrap_or((None, None));
        report.push(RowDiagnostic {
            row,
            column,
            diagnostic,
        });
    }
    if fatal {
        return None;
    }
    doc.main_mut()
        .store_ext_namespace("merge", serde_json::json!({ "row_key": key }))
        .expect("a two-level map is within depth");
    Some(doc)
}

/// Lower one row through a mapping table into main-or-card fields keyed by
/// top-level name, nested containers assembled from dotted addresses. An empty
/// cell is absent: the field falls to the schema's ladder.
fn lower(
    map: &IndexMap<String, Mapping>,
    row: &Row,
    row_index: usize,
    report: &mut Vec<RowDiagnostic>,
) -> IndexMap<String, Value> {
    let mut fields: IndexMap<String, Value> = IndexMap::new();
    for (address, mapping) in map {
        let cell = match (&mapping.value, &mapping.column) {
            (Some(constant), _) => constant.clone(),
            (None, Some(column)) => row.get(column).cloned().unwrap_or(Value::Null),
            (None, None) => unreachable!("spec check requires one of column / value"),
        };
        let mut cell = match cell {
            Value::Null => continue,
            Value::String(s) if s.trim().is_empty() => continue,
            Value::String(s) => Value::String(s.trim().to_string()),
            other => other,
        };
        if let Some(sep) = &mapping.split {
            cell = match cell {
                Value::String(s) => Value::Array(
                    s.split(sep.as_str())
                        .map(str::trim)
                        .filter(|p| !p.is_empty())
                        .map(|p| Value::String(p.to_string()))
                        .collect(),
                ),
                other => other,
            };
        }
        if let Some(format) = &mapping.format {
            cell = match cell {
                Value::String(s) => match parse_date(&s, format) {
                    Ok(iso) => Value::String(iso),
                    Err(reason) => {
                        report.push(RowDiagnostic {
                            row: Some(row_index),
                            column: mapping.column.clone(),
                            diagnostic: merge_error(
                                "date_format",
                                format!("'{s}' does not parse as `{format}`: {reason}"),
                            )
                            .with_path(DocPath::main().field(address).to_string()),
                        });
                        continue;
                    }
                },
                other => other,
            };
        }
        place(&mut fields, address, cell);
    }
    fields
}

/// Parse `text` with a strftime pattern and re-spell it `YYYY-MM-DD`. Padding
/// is lenient whatever the pattern says: a spreadsheet exports `3/1/2026` and
/// `03/01/2026` from the same column, and `%m` must read both, so every
/// unmodified numeric conversion is rewritten to its `%-` form before parsing.
fn parse_date(text: &str, format: &str) -> Result<String, String> {
    let mut lenient = String::with_capacity(format.len() + 4);
    let mut chars = format.chars().peekable();
    while let Some(c) = chars.next() {
        lenient.push(c);
        if c == '%' {
            match chars.peek() {
                Some('m' | 'd' | 'y' | 'Y' | 'e') => lenient.push('-'),
                Some(&next) => {
                    lenient.push(next);
                    chars.next();
                }
                None => {}
            }
        }
    }
    let description = time::format_description::parse_strftime_borrowed(&lenient)
        .map_err(|e| format!("bad strftime pattern: {e}"))?;
    let date = time::Date::parse(text, &description).map_err(|e| e.to_string())?;
    let iso = time::format_description::parse_borrowed::<2>("[year]-[month]-[day]")
        .expect("static");
    date.format(&iso).map_err(|e| e.to_string())
}

/// Assemble a dotted address into nested objects. A later mapping at a prefix
/// of an earlier one replaces it.
fn place(fields: &mut IndexMap<String, Value>, address: &str, cell: Value) {
    let mut segments = address.split('.');
    let top = segments.next().unwrap_or_default().to_string();
    let rest: Vec<&str> = segments.collect();
    if rest.is_empty() {
        fields.insert(top, cell);
        return;
    }
    let mut cursor = fields
        .entry(top)
        .or_insert_with(|| Value::Object(Default::default()));
    for (n, seg) in rest.iter().enumerate() {
        if !cursor.is_object() {
            *cursor = Value::Object(Default::default());
        }
        let map = cursor.as_object_mut().expect("object");
        if n + 1 == rest.len() {
            map.insert(seg.to_string(), cell);
            return;
        }
        cursor = map
            .entry(seg.to_string())
            .or_insert_with(|| Value::Object(Default::default()));
    }
}

fn value_at<'a>(fields: &'a IndexMap<String, Value>, address: &str) -> Option<&'a Value> {
    let mut segments = address.split('.');
    let mut cursor = fields.get(segments.next()?)?;
    for seg in segments {
        cursor = cursor.get(seg)?;
    }
    Some(cursor)
}

fn cell_text(cell: &Value) -> Option<String> {
    match cell {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn is_empty_row(row: &Row) -> bool {
    row.values().all(|v| match v {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        _ => false,
    })
}

/// The column a main-card path came from: `main.<address>…` against the
/// mapping table.
fn reverse_main(map: &IndexMap<String, Mapping>, path: &str) -> Option<String> {
    match DocPath::from_str(path).ok().as_ref().map(|p| p.segs()) {
        Some([DocSeg::Main, rest @ ..]) => reverse(map, rest),
        _ => None,
    }
}

/// The mapping whose address the field segments of `segs` fall under. An index
/// or body step ends the address: it refines a target the mapping already
/// names.
fn reverse(map: &IndexMap<String, Mapping>, segs: &[DocSeg]) -> Option<String> {
    let address: Vec<&str> = segs
        .iter()
        .map_while(|seg| match seg {
            DocSeg::Field { name } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let address = address.join(".");
    map.iter()
        .find(|(target, _)| *target == &address || address.starts_with(&format!("{target}.")))
        .and_then(|(_, mapping)| mapping.column.clone())
}

/// Interpolate `{field}` tokens against the main fields, refuse a path
/// separator, and refuse a name another document already took.
fn output_name(
    pattern: &str,
    fields: &IndexMap<String, Value>,
    key: &str,
    row: usize,
    taken: &mut HashMap<String, String>,
    report: &mut Vec<RowDiagnostic>,
) -> Option<String> {
    let mut out = String::new();
    let mut rest = pattern;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('}') else {
            report.push(RowDiagnostic {
                row: None,
                column: None,
                diagnostic: merge_error("output_pattern", format!("unclosed `{{` in output pattern '{pattern}'")),
            });
            return None;
        };
        let name = &rest[open + 1..open + close];
        let text = match fields.get(name) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            _ => {
                report.push(RowDiagnostic {
                    row: Some(row),
                    column: None,
                    diagnostic: merge_error(
                        "output_unresolvable",
                        format!("output pattern names `{{{name}}}`, which row {row} carries no scalar for"),
                    ),
                });
                return None;
            }
        };
        out.push_str(&text);
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    if out.is_empty() || out == "." || out == ".." || out.contains(['/', '\\', '\0']) {
        report.push(RowDiagnostic {
            row: Some(row),
            column: None,
            diagnostic: merge_error("output_unsafe", format!("'{out}' is not a safe filename")),
        });
        return None;
    }
    if let Some(earlier) = taken.get(&out) {
        report.push(RowDiagnostic {
            row: Some(row),
            column: None,
            diagnostic: merge_error(
                "output_collision",
                format!("'{out}' is already taken by document '{earlier}'; the output pattern does not key the input"),
            ),
        });
        return None;
    }
    taken.insert(out.clone(), key.to_string());
    Some(out)
}

#[cfg(test)]
mod tests;
