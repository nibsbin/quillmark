//! Bulk document generation: a [`MergeSpec`] interpreted over input rows, or
//! over documents already in the values form, into `Document`s plus a per-row
//! report. Engine-free: the plan hands documents to whatever render loop the
//! surface runs, so a batch validates before any compilation is paid.
//!
//! ```ignore
//! let spec = MergeSpec::from_yaml(yaml)?;
//! let plan = plan(&quill, &spec, &Input::Rows(rows));
//! for d in plan.errors() {
//!     eprintln!("row {:?} column {:?}: {}", d.row, d.column, d.diagnostic.message);
//! }
//! for planned in plan.clean_documents() {
//!     engine.render(&quill, &planned.document, &opts)?;
//! }
//! ```
//!
//! The contract is `prose/canon/MERGE.md`.

mod address;
mod spec;

pub use spec::{CardMapping, Mapping, MergeSpec, Mode};

use indexmap::IndexMap;
use quillmark_core::{
    CardValues, Diagnostic, DocPath, DocSeg, Document, DocumentValues, Quill, QuillReference,
    Severity,
};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use spec::{hex, spec_error, Token};
use std::collections::HashMap;
use std::str::FromStr;

/// One input row keyed by header. Cells arrive as strings from a spreadsheet
/// edge and as native JSON from an API caller; the typed writer judges both.
pub type Row = IndexMap<String, Value>;

/// The two input lanes. Rows are lowered through the spec's mappings;
/// documents are the values form already and skip the mapping.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Input {
    Rows(Vec<Row>),
    Documents(Vec<DocumentValues>),
}

/// A diagnostic anchored to its source row and, where its path reverse-maps
/// to a mapping, the column that fed it. `row` is the 0-based data row, or
/// the index in the documents lane; `None` is spec-level.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct RowDiagnostic {
    pub row: Option<usize>,
    pub column: Option<String>,
    pub diagnostic: Diagnostic,
}

#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct PlannedDocument {
    /// The `key` column's value or the output name in `document` mode, the
    /// `group_by` value in `cards` mode, the output name in the documents
    /// lane. Stamped on the document as `$ext.merge.row_key`.
    pub key: String,
    /// The source rows, in card order for `cards` mode.
    pub rows: Vec<usize>,
    /// The output file stem; the surface appends the format's extension.
    pub filename: String,
    /// SHA-256 over the spec hash, the quill reference and the values that
    /// built the document: what an incremental re-run compares.
    pub input_hash: String,
    #[serde(skip)]
    pub document: Document,
}

impl RowDiagnostic {
    pub fn new(row: Option<usize>, column: Option<String>, diagnostic: Diagnostic) -> Self {
        Self {
            row,
            column,
            diagnostic,
        }
    }
}

#[derive(Debug, Default, Serialize)]
#[non_exhaustive]
pub struct MergePlan {
    pub documents: Vec<PlannedDocument>,
    pub report: Vec<RowDiagnostic>,
    /// Rows every cell of which was empty.
    pub skipped_empty: usize,
}

impl MergePlan {
    /// No error in the report. Warnings never block.
    pub fn is_clean(&self) -> bool {
        self.errors().next().is_none()
    }

    pub fn errors(&self) -> impl Iterator<Item = &RowDiagnostic> {
        self.report
            .iter()
            .filter(|d| d.diagnostic.severity == Severity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &RowDiagnostic> {
        self.report
            .iter()
            .filter(|d| d.diagnostic.severity == Severity::Warning)
    }

    /// The documents no error anchors to: what a forced render renders.
    pub fn clean_documents(&self) -> impl Iterator<Item = &PlannedDocument> {
        let dirty: Vec<usize> = self.errors().filter_map(|d| d.row).collect();
        self.documents
            .iter()
            .filter(move |doc| !doc.rows.iter().any(|r| dirty.contains(r)))
    }
}

/// Interpret `spec` over `input` against `quill`. Spec-level refusals end the
/// plan with no documents; otherwise every row is planned and every refusal,
/// validation diagnostic, naming collision and grouping conflict lands in the
/// report under its row.
pub fn plan(quill: &Quill, spec: &MergeSpec, input: &Input) -> MergePlan {
    let mut plan = MergePlan::default();
    push_spec_level(&mut plan.report, spec.check(quill));
    if !plan.is_clean() {
        return plan;
    }
    let ctx = Context {
        quill,
        spec,
        quill_ref: spec.quill_ref().expect("checked"),
        spec_hash: spec.hash(),
        tokens: spec::output_tokens(&spec.output).expect("checked"),
    };
    let mut naming = Naming::default();
    match input {
        Input::Documents(documents) => plan_documents(&ctx, documents, &mut naming, &mut plan),
        Input::Rows(rows) => plan_rows(&ctx, rows, &mut naming, &mut plan),
    }
    plan
}

struct Context<'a> {
    quill: &'a Quill,
    spec: &'a MergeSpec,
    quill_ref: QuillReference,
    spec_hash: String,
    tokens: Vec<Token>,
}

/// Output names and keys already claimed, each by the key that claimed it.
#[derive(Default)]
struct Naming {
    filenames: HashMap<String, String>,
    keys: HashMap<String, usize>,
}

fn push_spec_level(report: &mut Vec<RowDiagnostic>, diagnostics: Vec<Diagnostic>) {
    report.extend(diagnostics.into_iter().map(|diagnostic| RowDiagnostic {
        row: None,
        column: None,
        diagnostic,
    }));
}

fn row_error(row: Option<usize>, column: Option<&str>, code: &str, message: String) -> RowDiagnostic {
    RowDiagnostic {
        row,
        column: column.map(str::to_owned),
        diagnostic: spec_error(code, message),
    }
}

fn plan_documents(
    ctx: &Context,
    documents: &[DocumentValues],
    naming: &mut Naming,
    plan: &mut MergePlan,
) {
    for (i, values) in documents.iter().enumerate() {
        let fields = values.fields.clone().unwrap_or_default();
        let Some(filename) = ctx.output_name(&fields, i, &mut plan.report) else {
            continue;
        };
        if naming.claim_filename(&filename, &filename, i, &mut plan.report).is_none() {
            continue;
        }
        let locate = |_: &str| (Some(i), None);
        let Some(document) = ctx.build(values, &filename, &locate, &mut plan.report) else {
            continue;
        };
        plan.documents.push(PlannedDocument {
            key: filename.clone(),
            rows: vec![i],
            filename,
            input_hash: ctx.input_hash(values),
            document,
        });
    }
}

fn plan_rows(ctx: &Context, rows: &[Row], naming: &mut Naming, plan: &mut MergePlan) {
    let header = header_of(rows);
    let Some(tables) = ctx.tables(&header, &mut plan.report) else {
        return;
    };
    match ctx.spec.mode {
        Mode::Document => plan_document_rows(ctx, &tables.main, rows, naming, plan),
        Mode::Cards => {
            let (kind, card_map) = tables.card.as_ref().expect("checked");
            plan_card_rows(ctx, &tables.main, kind, card_map, rows, naming, plan)
        }
    }
}

fn plan_document_rows(
    ctx: &Context,
    map: &IndexMap<String, Mapping>,
    rows: &[Row],
    naming: &mut Naming,
    plan: &mut MergePlan,
) {
    for (i, row) in rows.iter().enumerate() {
        if is_empty_row(row) {
            plan.skipped_empty += 1;
            continue;
        }
        let lowered = lower(map, row, i, &DocPath::main(), &mut plan.report);
        let Some(filename) = ctx.output_name(&lowered.fields, i, &mut plan.report) else {
            continue;
        };
        let key = match &ctx.spec.key {
            None => filename.clone(),
            Some(column) => match row.get(column).and_then(cell_text) {
                Some(key) => key,
                None => {
                    plan.report.push(row_error(
                        Some(i),
                        Some(column),
                        "missing_key",
                        format!("row {i} has no value in the `key` column '{column}'"),
                    ));
                    continue;
                }
            },
        };
        if naming.claim_filename(&filename, &key, i, &mut plan.report).is_none() {
            continue;
        }
        if ctx.spec.key.is_some() {
            if let Some(earlier) = naming.keys.insert(key.clone(), i) {
                plan.report.push(row_error(
                    Some(i),
                    ctx.spec.key.as_deref(),
                    "duplicate_key",
                    format!("key '{key}' is already taken by row {earlier}"),
                ));
                continue;
            }
        }
        let values = lowered.into_document_values();
        let locate = |path: &str| (Some(i), reverse_main(map, path));
        let Some(document) = ctx.build(&values, &key, &locate, &mut plan.report) else {
            continue;
        };
        plan.documents.push(PlannedDocument {
            key,
            rows: vec![i],
            filename,
            input_hash: ctx.input_hash(&values),
            document,
        });
    }
}

fn plan_card_rows(
    ctx: &Context,
    main_map: &IndexMap<String, Mapping>,
    kind: &str,
    card_map: &IndexMap<String, Mapping>,
    rows: &[Row],
    naming: &mut Naming,
    plan: &mut MergePlan,
) {
    let group_col = ctx.spec.group_by.as_deref().expect("checked");
    let mut groups: IndexMap<String, Vec<usize>> = IndexMap::new();
    for (i, row) in rows.iter().enumerate() {
        if is_empty_row(row) {
            plan.skipped_empty += 1;
            continue;
        }
        match row.get(group_col).and_then(cell_text) {
            Some(key) => groups.entry(key).or_default().push(i),
            None => plan.report.push(row_error(
                Some(i),
                Some(group_col),
                "missing_group_key",
                format!("row {i} has no value in the `group_by` column '{group_col}'"),
            )),
        }
    }
    for (key, members) in groups {
        let first = members[0];
        let main = lower(main_map, &rows[first], first, &DocPath::main(), &mut plan.report);
        let mut conflict = false;
        for &i in &members[1..] {
            let other = lower(main_map, &rows[i], i, &DocPath::main(), &mut plan.report);
            for (target, mapping) in main_map {
                if main.at(target) != other.at(target) {
                    conflict = true;
                    plan.report.push(RowDiagnostic {
                        row: Some(i),
                        column: mapping.column.clone(),
                        diagnostic: spec_error(
                            "group_conflict",
                            format!(
                                "main target '{target}' differs from row {first} within group '{key}'; a main-mapped column is constant across its group"
                            ),
                        )
                        .with_path(target_path(&DocPath::main(), target).to_string()),
                    });
                }
            }
        }
        let cards: Vec<CardValues> = members
            .iter()
            .enumerate()
            .map(|(n, &i)| {
                let base = DocPath::card(Some(kind), n);
                lower(card_map, &rows[i], i, &base, &mut plan.report).into_card_values(kind)
            })
            .collect();
        if conflict {
            continue;
        }
        let Some(filename) = ctx.output_name(&main.fields, first, &mut plan.report) else {
            continue;
        };
        if naming.claim_filename(&filename, &key, first, &mut plan.report).is_none() {
            continue;
        }
        let values = main.into_document_values().with_cards(cards);
        let locate = |path: &str| match DocPath::from_str(path).ok().as_ref().map(|p| p.segs()) {
            Some([DocSeg::Card { index, .. }, rest @ ..]) => {
                (members.get(*index).copied(), reverse(card_map, rest))
            }
            Some([DocSeg::Main, rest @ ..]) => (Some(first), reverse(main_map, rest)),
            _ => (None, None),
        };
        let Some(document) = ctx.build(&values, &key, &locate, &mut plan.report) else {
            continue;
        };
        plan.documents.push(PlannedDocument {
            key,
            rows: members,
            filename,
            input_hash: ctx.input_hash(&values),
            document,
        });
    }
}

/// The mapping tables a row is lowered through: the spec's, plus an identity
/// entry for every header that is itself a target and that no entry claims.
struct Tables {
    main: IndexMap<String, Mapping>,
    card: Option<(String, IndexMap<String, Mapping>)>,
}

impl Context<'_> {
    fn tables(&self, header: &[String], report: &mut Vec<RowDiagnostic>) -> Option<Tables> {
        let config = self.quill.config();
        let mut main = self.spec.map.clone();
        let mut card = self
            .spec
            .cards
            .iter()
            .next()
            .map(|(kind, c)| (kind.clone(), c.map.clone()));
        let card_fields = card
            .as_ref()
            .and_then(|(kind, _)| config.card_kind(kind))
            .map(|s| &s.fields);
        let mut fatal = false;
        let referenced: Vec<&str> = self.spec.columns().collect();
        for column in &referenced {
            if !header.iter().any(|h| h == column) {
                report.push(row_error(
                    None,
                    Some(column),
                    "unknown_column",
                    format!("spec reads column '{column}', which the input has no header for"),
                ));
                fatal = true;
            }
        }
        for h in header {
            if referenced.contains(&h.as_str()) {
                continue;
            }
            let in_main =
                !main.contains_key(h) && address::resolve(&config.main.fields, h).is_ok();
            let in_card = card_fields.is_some_and(|f| address::resolve(f, h).is_ok())
                && card.as_ref().is_some_and(|(_, m)| !m.contains_key(h));
            match (in_main, in_card) {
                (true, true) => {
                    report.push(row_error(
                        None,
                        Some(h),
                        "ambiguous_column",
                        format!(
                            "column '{h}' is a field of both the main card and the card kind; map it explicitly"
                        ),
                    ));
                    fatal = true;
                }
                (true, false) => {
                    main.insert(h.clone(), Mapping::column(h.clone()));
                }
                (false, true) => {
                    card.as_mut().expect("in_card").1.insert(h.clone(), Mapping::column(h.clone()));
                }
                (false, false) => report.push(RowDiagnostic {
                    row: None,
                    column: Some(h.clone()),
                    diagnostic: Diagnostic::new(
                        Severity::Warning,
                        format!("column '{h}' is not mapped and will be ignored"),
                    )
                    .with_code("merge::unmapped_column".to_string()),
                }),
            }
        }
        if fatal {
            return None;
        }
        Some(Tables { main, card })
    }

    /// Construct through the typed writer, then validate. Every refusal and
    /// every validation diagnostic lands in `report` under its row; a document
    /// an error touches is not returned.
    fn build(
        &self,
        values: &DocumentValues,
        key: &str,
        locate: &dyn Fn(&str) -> (Option<usize>, Option<String>),
        report: &mut Vec<RowDiagnostic>,
    ) -> Option<Document> {
        let mut doc = Document::new(self.quill_ref.clone());
        if let Err(refusals) = self.quill.writer(&mut doc).set_values(values) {
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
        for diagnostic in self.quill.validate(&doc) {
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
            .store_ext_namespace(
                "merge",
                serde_json::json!({ "row_key": key, "spec_hash": self.spec_hash }),
            )
            .expect("two levels deep");
        Some(doc)
    }

    fn input_hash(&self, values: &DocumentValues) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.spec_hash.as_bytes());
        hasher.update(b"\n");
        hasher.update(self.quill_ref.to_string().as_bytes());
        hasher.update(b"\n");
        hasher.update(serde_json::to_vec(values).expect("values serialize"));
        hex(&hasher.finalize())
    }

    /// The output stem for one document: `{field}` tokens against the lowered
    /// main fields, scalars only, and no path separator.
    fn output_name(
        &self,
        fields: &IndexMap<String, Value>,
        row: usize,
        report: &mut Vec<RowDiagnostic>,
    ) -> Option<String> {
        let mut out = String::new();
        for token in &self.tokens {
            match token {
                Token::Literal(text) => out.push_str(text),
                Token::Field(name) => match fields.get(name) {
                    Some(Value::String(s)) => out.push_str(s),
                    Some(Value::Number(n)) => out.push_str(&n.to_string()),
                    Some(Value::Bool(b)) => out.push_str(&b.to_string()),
                    _ => {
                        report.push(row_error(
                            Some(row),
                            None,
                            "output_unresolvable",
                            format!(
                                "output pattern names `{{{name}}}`, which row {row} carries no scalar for"
                            ),
                        ));
                        return None;
                    }
                },
            }
        }
        if out.is_empty() || out == "." || out == ".." || out.contains(['/', '\\', '\0']) {
            report.push(row_error(
                Some(row),
                None,
                "output_unsafe",
                format!("'{out}' is not a safe file name"),
            ));
            return None;
        }
        Some(out)
    }
}

impl Naming {
    fn claim_filename(
        &mut self,
        filename: &str,
        key: &str,
        row: usize,
        report: &mut Vec<RowDiagnostic>,
    ) -> Option<()> {
        if let Some(earlier) = self.filenames.get(filename) {
            report.push(row_error(
                Some(row),
                None,
                "output_collision",
                format!(
                    "output '{filename}' is already taken by document '{earlier}'; the output pattern does not key the input"
                ),
            ));
            return None;
        }
        self.filenames.insert(filename.to_string(), key.to_string());
        Some(())
    }
}

/// One row lowered through one mapping table.
#[derive(Default)]
struct Lowered {
    fields: IndexMap<String, Value>,
    body: Option<String>,
}

impl Lowered {
    fn at(&self, target: &str) -> Option<Value> {
        if target == address::BODY {
            return self.body.clone().map(Value::String);
        }
        value_at(&self.fields, target).cloned()
    }

    fn into_document_values(self) -> DocumentValues {
        let values = DocumentValues::new(self.fields);
        match self.body {
            Some(body) => values.with_body(body),
            None => values,
        }
    }

    fn into_card_values(self, kind: &str) -> CardValues {
        let values = CardValues::new(kind, self.fields);
        match self.body {
            Some(body) => values.with_body(body),
            None => values,
        }
    }
}

/// Lower one row through a mapping table. A column cell that is null, empty
/// or whitespace is absent, so the field falls to the schema's ladder; a
/// constant is taken verbatim, `value: ""` included. Cells are trimmed.
fn lower(
    map: &IndexMap<String, Mapping>,
    row: &Row,
    row_index: usize,
    base: &DocPath,
    report: &mut Vec<RowDiagnostic>,
) -> Lowered {
    let mut lowered = Lowered::default();
    for (target, mapping) in map {
        let cell = match (&mapping.value, &mapping.column) {
            (Some(Value::Null), _) => continue,
            (Some(constant), _) => constant.clone(),
            (None, Some(column)) => match row.get(column) {
                None | Some(Value::Null) => continue,
                Some(Value::String(s)) if s.trim().is_empty() => continue,
                Some(Value::String(s)) => Value::String(s.trim().to_string()),
                Some(other) => other.clone(),
            },
            (None, None) => unreachable!("spec check requires one of column / value"),
        };
        let mut cell = cell;
        if let Some(separator) = &mapping.split {
            if let Value::String(s) = cell {
                cell = Value::Array(
                    s.split(separator.as_str())
                        .map(str::trim)
                        .filter(|piece| !piece.is_empty())
                        .map(|piece| Value::String(piece.to_string()))
                        .collect(),
                );
            }
        }
        if let Some(format) = &mapping.format {
            if let Value::String(s) = &cell {
                match parse_date(s, format) {
                    Ok(iso) => cell = Value::String(iso),
                    Err(reason) => {
                        report.push(RowDiagnostic {
                            row: Some(row_index),
                            column: mapping.column.clone(),
                            diagnostic: spec_error(
                                "date_format",
                                format!("'{s}' does not parse as `{format}`: {reason}"),
                            )
                            .with_path(target_path(base, target).to_string()),
                        });
                        continue;
                    }
                }
            }
        }
        if target == address::BODY {
            match cell {
                Value::String(text) => lowered.body = Some(text),
                other => report.push(RowDiagnostic {
                    row: Some(row_index),
                    column: mapping.column.clone(),
                    diagnostic: spec_error(
                        "body_not_text",
                        format!("a body is markdown text, not {other}"),
                    )
                    .with_path(base.body().to_string()),
                }),
            }
        } else {
            place(&mut lowered.fields, target, cell);
        }
    }
    lowered
}

/// The document-model path a target anchors at under `base`.
fn target_path(base: &DocPath, target: &str) -> DocPath {
    if target == address::BODY {
        return base.body();
    }
    target
        .split('.')
        .fold(base.clone(), |path, seg| path.field(seg))
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

/// Assemble a dotted target into nested objects. A later entry at a prefix of
/// an earlier one replaces it.
fn place(fields: &mut IndexMap<String, Value>, target: &str, cell: Value) {
    let mut segments = target.split('.');
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

fn value_at<'a>(fields: &'a IndexMap<String, Value>, target: &str) -> Option<&'a Value> {
    let mut segments = target.split('.');
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

/// Every header the rows carry, in first appearance order.
fn header_of(rows: &[Row]) -> Vec<String> {
    let mut seen: IndexMap<&str, ()> = IndexMap::new();
    for row in rows {
        for key in row.keys() {
            seen.entry(key.as_str()).or_insert(());
        }
    }
    seen.keys().map(|k| k.to_string()).collect()
}

fn reverse_main(map: &IndexMap<String, Mapping>, path: &str) -> Option<String> {
    match DocPath::from_str(path).ok().as_ref().map(|p| p.segs()) {
        Some([DocSeg::Main, rest @ ..]) => reverse(map, rest),
        _ => None,
    }
}

/// The column behind the mapping whose target the field segments of `segs`
/// fall under. An index step ends the target: it refines a target the mapping
/// already names. A lone body step is the `$body` mapping.
fn reverse(map: &IndexMap<String, Mapping>, segs: &[DocSeg]) -> Option<String> {
    if let [DocSeg::Body] = segs {
        return map.get(address::BODY).and_then(|m| m.column.clone());
    }
    let target: Vec<&str> = segs
        .iter()
        .map_while(|seg| match seg {
            DocSeg::Field { name } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let target = target.join(".");
    map.iter()
        .find(|(t, _)| **t == target || target.starts_with(&format!("{t}.")))
        .and_then(|(_, mapping)| mapping.column.clone())
}

#[cfg(test)]
mod tests;
