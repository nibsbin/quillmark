use crate::commands::load_quill;
use crate::errors::{CliError, Result};
use crate::output::page_output_path;
use clap::Parser;
use quillmark::{Quill, Quillmark};
use quillmark_core::{
    Diagnostic, DocumentValues, LiveSession, OutputFormat, RenderError, RenderOptions, Severity,
};
use quillmark_merge::{plan, Input, MergePlan, MergeSpec, PlannedDocument, Row, RowDiagnostic};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
pub struct MergeArgs {
    /// Path to quill directory
    #[arg(value_name = "QUILL_PATH")]
    quill: PathBuf,

    /// Merge spec: YAML, or JSON by extension
    #[arg(value_name = "SPEC_FILE")]
    spec: PathBuf,

    /// Rows as .csv / .tsv, or .json (an array of row objects, or an object
    /// carrying a `documents` array in the values form)
    #[arg(value_name = "INPUT_FILE")]
    input: PathBuf,

    /// Output directory; every artifact and manifest.json land here
    #[arg(long, value_name = "DIR")]
    out: Option<PathBuf>,

    /// Plan and report, render nothing
    #[arg(long)]
    dry_run: bool,

    /// Emit the report and manifest as one JSON object on stdout
    #[arg(long)]
    json: bool,

    /// Render the rows no error touches and report the rest
    #[arg(long)]
    force: bool,

    /// Output format: pdf, svg, png
    #[arg(short, long, value_name = "FORMAT", default_value = "pdf")]
    format: String,

    /// Field delimiter for a tabular input (default: `,`, or a tab for .tsv)
    #[arg(long, value_name = "CHAR")]
    delimiter: Option<char>,

    /// Render threads (default: every core)
    #[arg(long, value_name = "N")]
    jobs: Option<usize>,

    /// Suppress the report and the summary line
    #[arg(long)]
    quiet: bool,
}

/// How a row index prints: a spreadsheet's numbering counts the header as
/// row 1, a JSON array's index is what it is.
#[derive(Clone, Copy)]
enum Numbering {
    Spreadsheet,
    Index,
}

impl Numbering {
    fn label(self, row: Option<usize>) -> String {
        match (self, row) {
            (_, None) => "-".to_string(),
            (Numbering::Spreadsheet, Some(i)) => (i + 2).to_string(),
            (Numbering::Index, Some(i)) => i.to_string(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Planned,
    Rendered,
    Failed,
    Skipped,
}

/// One manifest entry: a planned document and what became of it.
#[derive(Serialize)]
struct Entry<'a> {
    key: &'a str,
    rows: &'a [usize],
    filename: &'a str,
    input_hash: &'a str,
    status: Status,
    files: Vec<String>,
}

pub fn execute(args: MergeArgs) -> Result<()> {
    let out = match (&args.out, args.dry_run) {
        (Some(dir), _) => Some(dir.clone()),
        (None, true) => None,
        (None, false) => {
            return Err(CliError::InvalidArgument(
                "--out <DIR> is required unless --dry-run".to_string(),
            ))
        }
    };
    let format = args
        .format
        .parse::<OutputFormat>()
        .map_err(|e| CliError::InvalidArgument(e.to_string()))?;

    let quill = load_quill(&args.quill)?;
    let spec = read_spec(&args.spec)?;
    let (input, numbering) = read_input(&args.input, args.delimiter)?;

    let mut plan = plan(&quill, &spec, &input);
    let clean = plan.is_clean();
    let rendering = !args.dry_run && (clean || args.force);

    let mut statuses: Vec<(Status, Vec<String>)> = plan
        .documents
        .iter()
        .map(|_| (Status::Planned, Vec::new()))
        .collect();
    if rendering {
        let dir = out.as_deref().expect("--out checked above");
        fs::create_dir_all(dir)?;
        let renderable: Vec<usize> = if clean {
            (0..plan.documents.len()).collect()
        } else {
            let clean_keys: Vec<&str> = plan.clean_documents().map(|d| d.key.as_str()).collect();
            plan.documents
                .iter()
                .enumerate()
                .filter(|(_, d)| clean_keys.contains(&d.key.as_str()))
                .map(|(i, _)| i)
                .collect()
        };
        for (i, status) in statuses.iter_mut().enumerate() {
            if !renderable.contains(&i) {
                status.0 = Status::Skipped;
            }
        }
        let outcomes = render(&quill, &plan, &renderable, format, &args.format, dir, args.jobs);
        for (i, outcome) in outcomes {
            match outcome {
                Ok(files) => statuses[i] = (Status::Rendered, files),
                Err(diagnostics) => {
                    statuses[i].0 = Status::Failed;
                    let row = plan.documents[i].rows.first().copied();
                    plan.report.extend(
                        diagnostics
                            .into_iter()
                            .map(|diagnostic| RowDiagnostic::new(row, None, diagnostic)),
                    );
                }
            }
        }
    }

    let entries: Vec<Entry> = plan
        .documents
        .iter()
        .zip(statuses)
        .map(|(doc, (status, files))| Entry {
            key: &doc.key,
            rows: &doc.rows,
            filename: &doc.filename,
            input_hash: &doc.input_hash,
            status,
            files,
        })
        .collect();
    let errors = plan.errors().count();
    let warnings = plan.warnings().count();
    let failed = entries
        .iter()
        .filter(|e| matches!(e.status, Status::Failed))
        .count();
    let rendered = entries
        .iter()
        .filter(|e| matches!(e.status, Status::Rendered))
        .count();

    if rendering {
        let manifest = serde_json::to_vec_pretty(&entries).expect("entries serialize");
        fs::write(out.as_deref().expect("--out").join("manifest.json"), manifest)?;
    }

    if args.json {
        let json = serde_json::json!({
            "clean": errors == 0,
            "skipped_empty": plan.skipped_empty,
            "report": plan.report,
            "documents": entries,
        });
        println!("{}", serde_json::to_string_pretty(&json).expect("json"));
    } else if !args.quiet {
        print_report(&plan.report, numbering);
        let mut summary = format!(
            "Planned {} document(s): {errors} error(s), {warnings} warning(s), {} empty row(s) skipped.",
            plan.documents.len(),
            plan.skipped_empty
        );
        if rendering {
            summary.push_str(&format!(
                " Rendered {rendered} to {}{}.",
                out.as_deref().expect("--out").display(),
                if failed > 0 { format!(", {failed} failed") } else { String::new() }
            ));
        } else if !args.dry_run {
            summary.push_str(" Nothing rendered: fix the errors, or pass --force to render the clean rows.");
        }
        println!("{summary}");
    }

    if errors > 0 || failed > 0 {
        return Err(CliError::Reported);
    }
    Ok(())
}

fn diag_error(diagnostic: Diagnostic) -> CliError {
    CliError::Render(RenderError::from_diag(diagnostic))
}

fn read_spec(path: &Path) -> Result<MergeSpec> {
    let text = fs::read_to_string(path).map_err(|e| {
        CliError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read spec '{}': {e}", path.display()),
        ))
    })?;
    let spec = if has_extension(path, "json") {
        MergeSpec::from_json(&text)
    } else {
        MergeSpec::from_yaml(&text)
    };
    spec.map_err(diag_error)
}

fn has_extension(path: &Path, ext: &str) -> bool {
    path.extension()
        .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case(ext))
}

fn read_input(path: &Path, delimiter: Option<char>) -> Result<(Input, Numbering)> {
    let bytes = fs::read(path).map_err(|e| {
        CliError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read input '{}': {e}", path.display()),
        ))
    })?;
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    if delimiter.is_none() && has_extension(path, "json") {
        return read_json_input(bytes).map(|input| (input, Numbering::Index));
    }
    let delimiter = match delimiter {
        Some(c) => c,
        None if has_extension(path, "tsv") => '\t',
        None if has_extension(path, "csv") => ',',
        None => {
            return Err(CliError::InvalidArgument(format!(
                "'{}' is not .csv, .tsv or .json; pass --delimiter to read it as tabular",
                path.display()
            )))
        }
    };
    let mut delimiter_byte = [0u8; 4];
    let delimiter_byte = delimiter.encode_utf8(&mut delimiter_byte).as_bytes();
    if delimiter_byte.len() != 1 {
        return Err(CliError::InvalidArgument(
            "--delimiter takes one ASCII character".to_string(),
        ));
    }
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(delimiter_byte[0])
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(bytes);
    let header: Vec<String> = reader
        .headers()
        .map_err(csv_error)?
        .iter()
        .map(str::to_owned)
        .collect();
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(csv_error)?;
        let row: Row = header
            .iter()
            .zip(record.iter())
            .map(|(h, cell)| (h.clone(), serde_json::Value::String(cell.to_string())))
            .collect();
        rows.push(row);
    }
    Ok((Input::Rows(rows), Numbering::Spreadsheet))
}

fn csv_error(e: csv::Error) -> CliError {
    CliError::InvalidArgument(format!("tabular input does not parse: {e}"))
}

fn read_json_input(bytes: &[u8]) -> Result<Input> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| CliError::InvalidArgument(format!("JSON input does not parse: {e}")))?;
    match value {
        serde_json::Value::Array(items) => {
            let mut rows = Vec::with_capacity(items.len());
            for (i, item) in items.into_iter().enumerate() {
                match item {
                    serde_json::Value::Object(map) => rows.push(map.into_iter().collect()),
                    other => {
                        return Err(CliError::InvalidArgument(format!(
                            "JSON row {i} is not an object: {other}"
                        )))
                    }
                }
            }
            Ok(Input::Rows(rows))
        }
        serde_json::Value::Object(mut map) => match map.remove("documents") {
            Some(documents) => serde_json::from_value::<Vec<DocumentValues>>(documents)
                .map(Input::Documents)
                .map_err(|e| {
                    CliError::InvalidArgument(format!(
                        "`documents` is not a list of documents in the values form: {e}"
                    ))
                }),
            None => Err(CliError::InvalidArgument(
                "a JSON object input carries a `documents` array; rows are a top-level array"
                    .to_string(),
            )),
        },
        other => Err(CliError::InvalidArgument(format!(
            "JSON input is neither an array of rows nor an object with `documents`: {other}"
        ))),
    }
}

/// Render the documents at `indices` in parallel, one live session per
/// worker: the session's compile persists, so each further document is an
/// `update` rather than a fresh open. Returns each index with its written
/// files, or the diagnostics that stopped it.
fn render(
    quill: &Quill,
    plan: &MergePlan,
    indices: &[usize],
    format: OutputFormat,
    extension: &str,
    dir: &Path,
    jobs: Option<usize>,
) -> Vec<(usize, std::result::Result<Vec<String>, Vec<Diagnostic>>)> {
    let engine = Quillmark::new();
    let opts = RenderOptions::default().with_output_format(format);
    let extension = extension.to_lowercase();
    let run = || {
        indices
            .par_iter()
            .with_min_len(4)
            .map_init(
                || None::<LiveSession>,
                |session, &i| {
                    let doc: &PlannedDocument = &plan.documents[i];
                    let result = match session {
                        None => engine.open(quill, &doc.document).and_then(|opened| {
                            let result = opened.render(&opts);
                            *session = Some(opened);
                            result
                        }),
                        Some(live) => live
                            .update(&doc.document)
                            .and_then(|_| live.render(&opts)),
                    };
                    let outcome = match result {
                        Err(e) => Err(e.into_diagnostics()),
                        Ok(rendered) => write_artifacts(dir, &doc.filename, &extension, &rendered.artifacts)
                            .map_err(|e| vec![e]),
                    };
                    (i, outcome)
                },
            )
            .collect::<Vec<_>>()
    };
    match jobs {
        Some(n) => rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build()
            .expect("thread pool")
            .install(run),
        None => run(),
    }
}

fn write_artifacts(
    dir: &Path,
    stem: &str,
    extension: &str,
    artifacts: &[quillmark_core::Artifact],
) -> std::result::Result<Vec<String>, Diagnostic> {
    let base = dir.join(format!("{stem}.{extension}"));
    let paths: Vec<PathBuf> = match artifacts {
        [_] => vec![base],
        many => (1..=many.len()).map(|n| page_output_path(&base, n)).collect(),
    };
    for (path, artifact) in paths.iter().zip(artifacts) {
        fs::write(path, &artifact.bytes).map_err(|e| {
            Diagnostic::new(
                Severity::Error,
                format!("failed to write '{}': {e}", path.display()),
            )
            .with_code("merge::write_failed".to_string())
        })?;
    }
    Ok(paths
        .iter()
        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().into_owned())
        .collect())
}

/// Errors line by line; warnings collapsed per code and anchor, since a
/// column the input never fills warns once per row.
fn print_report(report: &[RowDiagnostic], numbering: Numbering) {
    if report.is_empty() {
        return;
    }
    let mut collapsed: BTreeMap<(String, String, String), (usize, Option<usize>, &RowDiagnostic)> =
        BTreeMap::new();
    for d in report {
        match d.diagnostic.severity {
            Severity::Error => eprintln!("{}", line(d, numbering, None)),
            _ => {
                let key = (
                    d.diagnostic.code.clone().unwrap_or_default(),
                    d.diagnostic.path.clone().unwrap_or_default(),
                    d.column.clone().unwrap_or_default(),
                );
                let slot = collapsed.entry(key).or_insert((0, d.row, d));
                slot.0 += 1;
                if slot.1.is_none() {
                    slot.1 = d.row;
                }
            }
        }
    }
    for (count, first, d) in collapsed.values() {
        let repeat = if *count > 1 {
            Some(format!("{count} rows, first at row {}", numbering.label(*first)))
        } else {
            None
        };
        eprintln!("{}", line(d, numbering, repeat));
    }
}

fn line(d: &RowDiagnostic, numbering: Numbering, repeat: Option<String>) -> String {
    let severity = match d.diagnostic.severity {
        Severity::Error => "error",
        _ => "warning",
    };
    let mut out = format!("[{severity}]");
    match repeat {
        Some(text) => out.push_str(&format!(" ({text})")),
        None if d.row.is_some() => out.push_str(&format!(" row {}", numbering.label(d.row))),
        None => {}
    }
    if let Some(column) = &d.column {
        out.push_str(&format!(" column '{column}'"));
    }
    if let Some(path) = &d.diagnostic.path {
        out.push_str(&format!(" {path}"));
    }
    if let Some(code) = &d.diagnostic.code {
        out.push_str(&format!(" {code}"));
    }
    out.push_str(": ");
    out.push_str(&d.diagnostic.message);
    out
}
