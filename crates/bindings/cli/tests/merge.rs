//! The `merge` verb end to end against the taro fixture: tabular and JSON
//! inputs, the dry run, the forced render, and the manifest.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn taro() -> PathBuf {
    quillmark_fixtures::quills_path("taro")
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quillmark"))
        .args(args)
        .output()
        .expect("the built binary is executable")
}

fn write(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.to_string_lossy().into_owned()
}

const SPEC: &str = "$quill: taro@0.1.0\nmap:\n  author: { column: Name }\noutput: \"{author}\"\n";

#[test]
fn a_csv_batch_renders_one_pdf_per_row_and_a_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let spec = write(tmp.path(), "spec.yaml", SPEC);
    let csv = write(
        tmp.path(),
        "rows.csv",
        "\u{feff}Name,title,Notes\nAda,First,x\nBob,Second,\n,,\n",
    );
    let out = tmp.path().join("out");
    let result = run(&[
        "merge",
        taro().to_str().unwrap(),
        &spec,
        &csv,
        "--out",
        out.to_str().unwrap(),
        "--jobs",
        "2",
    ]);
    let stderr = String::from_utf8_lossy(&result.stderr);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(result.status.success(), "stdout: {stdout}\nstderr: {stderr}");
    assert!(out.join("Ada.pdf").is_file(), "{stderr}");
    assert!(out.join("Bob.pdf").is_file());
    assert!(
        stderr.contains("merge::unmapped_column") && stderr.contains("'Notes'"),
        "the unmapped column warns: {stderr}"
    );
    assert!(stdout.contains("Planned 2 document(s)") && stdout.contains("1 empty row(s) skipped"), "{stdout}");

    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("manifest.json")).unwrap()).unwrap();
    let entries = manifest.as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["key"], "Ada");
    assert_eq!(entries[0]["rows"], serde_json::json!([0]));
    assert_eq!(entries[0]["status"], "rendered");
    assert_eq!(entries[0]["files"], serde_json::json!(["Ada.pdf"]));
    assert_eq!(entries[0]["input_hash"].as_str().unwrap().len(), 64);
}

#[test]
fn a_dry_run_reports_in_json_and_exits_nonzero_on_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let spec = write(tmp.path(), "spec.yaml", SPEC);
    let csv = write(tmp.path(), "rows.csv", "Name,title\nAda,First\nAda,Second\n");
    let result = run(&[
        "merge",
        taro().to_str().unwrap(),
        &spec,
        &csv,
        "--dry-run",
        "--json",
    ]);
    assert_eq!(result.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(json["clean"], false);
    let report = json["report"].as_array().unwrap();
    let collision = report
        .iter()
        .find(|d| d["diagnostic"]["code"] == "merge::output_collision")
        .expect("the second Ada collides");
    assert_eq!(collision["row"], 1, "0-based in JSON");
    assert_eq!(json["documents"][0]["status"], "planned");
    assert!(!tmp.path().join("out").exists(), "a dry run writes nothing");
}

#[test]
fn force_renders_the_clean_rows_and_still_exits_nonzero() {
    let tmp = tempfile::tempdir().unwrap();
    let spec = write(tmp.path(), "spec.yaml", SPEC);
    let csv = write(tmp.path(), "rows.csv", "Name,title\nAda,First\nAda,Second\nBob,Third\n");
    let out = tmp.path().join("out");
    let result = run(&[
        "merge",
        taro().to_str().unwrap(),
        &spec,
        &csv,
        "--out",
        out.to_str().unwrap(),
        "--force",
    ]);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert_eq!(result.status.code(), Some(1), "errors remain: {stderr}");
    assert!(stderr.contains("[error] row 3"), "spreadsheet numbering in the table: {stderr}");
    assert!(out.join("Ada.pdf").is_file(), "the first Ada is clean and renders");
    assert!(out.join("Bob.pdf").is_file());
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("manifest.json")).unwrap()).unwrap();
    let keys: Vec<&str> = manifest
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["Ada", "Bob"], "the colliding row was never planned");
    assert!(manifest.as_array().unwrap().iter().all(|e| e["status"] == "rendered"));

    let without_force = run(&["merge", taro().to_str().unwrap(), &spec, &csv, "--out", tmp.path().join("out2").to_str().unwrap()]);
    assert_eq!(without_force.status.code(), Some(1));
    assert!(!tmp.path().join("out2").exists(), "without --force an error renders nothing");
}

#[test]
fn a_json_documents_input_skips_the_mapping() {
    let tmp = tempfile::tempdir().unwrap();
    let spec = write(tmp.path(), "spec.yaml", "$quill: taro@0.1.0\noutput: \"{author}\"\n");
    let input = write(
        tmp.path(),
        "docs.json",
        r#"{"documents": [{"fields": {"author": "Ada", "title": "First"}, "body": "Hello *taro*"}]}"#,
    );
    let out = tmp.path().join("out");
    let result = run(&[
        "merge",
        taro().to_str().unwrap(),
        &spec,
        &input,
        "--out",
        out.to_str().unwrap(),
        "-f",
        "svg",
    ]);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success(), "{stderr}");
    assert!(out.join("Ada.svg").is_file(), "{stderr}");
}

#[test]
fn a_spec_that_does_not_hold_stops_before_any_row() {
    let tmp = tempfile::tempdir().unwrap();
    let spec = write(
        tmp.path(),
        "spec.yaml",
        "$quill: taro@0.1.0\nmap:\n  nope: { column: Name }\noutput: \"{author}\"\n",
    );
    let csv = write(tmp.path(), "rows.csv", "Name\nAda\n");
    let result = run(&["merge", taro().to_str().unwrap(), &spec, &csv, "--dry-run"]);
    assert_eq!(result.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("merge::spec_unknown_target"), "{stderr}");
}
