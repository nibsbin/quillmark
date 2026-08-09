//! Every subcommand, once, against a fixture quill.
//!
//! The bin carries `test = false` (its name collides with the library crate),
//! so nothing inside `src/` is reachable from a test harness. These drive the
//! built executable instead, which is the surface a user actually gets: arg
//! parsing, exit status, and what lands on stdout/stderr.
//!
//! Depth belongs to the core tests these commands delegate to. What is asserted
//! here is the wiring: the command runs, exits the way it says it does, and the
//! bytes it emits are the shape it advertises.

use std::path::PathBuf;
use std::process::{Command, Output};

/// The `quillmark` executable cargo just built.
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_quillmark"))
}

fn taro() -> PathBuf {
    quillmark_fixtures::quills_path("taro")
}

fn run(args: &[&str]) -> Output {
    cli()
        .args(args)
        .output()
        .expect("the built binary is executable")
}

/// Exit 0, with stderr echoed on failure so a red run names its own cause.
fn ok(args: &[&str]) -> String {
    let out = run(args);
    assert!(
        out.status.success(),
        "`quillmark {}` exited {:?}\nstderr: {}",
        args.join(" "),
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout is UTF-8")
}

#[test]
fn info_prints_the_quill_identity() {
    let quill = taro();
    let stdout = ok(&["info", quill.to_str().unwrap()]);
    assert!(stdout.contains("taro"), "info omits the quill name: {stdout}");
}

#[test]
fn info_json_is_parseable() {
    let quill = taro();
    let stdout = ok(&["info", quill.to_str().unwrap(), "--json"]);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json emits one JSON document");
    assert!(value.is_object(), "--json emits an object: {value}");
}

#[test]
fn schema_emits_yaml_naming_a_declared_field() {
    let quill = taro();
    let stdout = ok(&["schema", quill.to_str().unwrap()]);
    // `ice_cream` is taro's own field, so this fails on an empty or generic dump.
    assert!(
        stdout.contains("ice_cream"),
        "schema omits a declared field: {stdout}"
    );
}

#[test]
fn blueprint_emits_a_card_yaml_fence() {
    let quill = taro();
    let stdout = ok(&["blueprint", quill.to_str().unwrap()]);
    assert!(
        stdout.contains("$quill:"),
        "blueprint carries no `$quill` line: {stdout}"
    );
}

#[test]
fn validate_accepts_a_shipped_quill() {
    let quill = taro();
    ok(&["validate", quill.to_str().unwrap()]);
    ok(&["validate", quill.to_str().unwrap(), "--verbose"]);
}

/// `-o` writes where it is told, for the two commands that take it.
#[test]
fn output_flag_writes_the_named_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let quill = taro();

    for (cmd, name) in [("schema", "schema.yaml"), ("blueprint", "blueprint.md")] {
        let path = dir.path().join(name);
        ok(&[cmd, quill.to_str().unwrap(), "-o", path.to_str().unwrap()]);
        let written = std::fs::read_to_string(&path).expect("the -o file exists");
        assert!(!written.trim().is_empty(), "{cmd} -o wrote an empty file");
    }
}

/// The one command that reaches a backend. Seeds from the blueprint (no markdown
/// argument), so it also covers the starter-document path.
#[test]
fn render_writes_a_pdf() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join("out.pdf");
    let quill = taro();

    ok(&[
        "render",
        quill.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);

    let bytes = std::fs::read(&out).expect("render wrote its output file");
    assert!(
        bytes.starts_with(b"%PDF-"),
        "output is not a PDF (first bytes: {:?})",
        &bytes[..bytes.len().min(8)]
    );
}

#[test]
fn render_stdout_emits_the_document_on_stdout() {
    let quill = taro();
    let out = run(&["render", quill.to_str().unwrap(), "--stdout"]);
    assert!(out.status.success(), "render --stdout failed");
    assert!(
        out.stdout.starts_with(b"%PDF-"),
        "--stdout did not emit PDF bytes"
    );
}

#[test]
fn render_svg_honours_the_format_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join("out.svg");
    let quill = taro();

    ok(&[
        "render",
        quill.to_str().unwrap(),
        "-f",
        "svg",
        "-o",
        out.to_str().unwrap(),
    ]);

    let svg = std::fs::read_to_string(&out).expect("render wrote its output file");
    assert!(svg.contains("<svg"), "output is not SVG: {}", &svg[..svg.len().min(80)]);
}

/// A missing quill exits non-zero and says so on stderr, rather than panicking
/// or exiting 0 quietly.
#[test]
fn absent_quill_fails_loudly() {
    let out = run(&["info", "/nonexistent/quill/path"]);
    assert!(!out.status.success(), "absent quill exited 0");
    assert!(
        !out.stderr.is_empty(),
        "absent quill wrote nothing to stderr"
    );
}

#[test]
fn unknown_format_fails_loudly() {
    let quill = taro();
    let out = run(&["render", quill.to_str().unwrap(), "-f", "docx", "--stdout"]);
    assert!(!out.status.success(), "unknown format exited 0");
    assert!(
        !out.stderr.is_empty(),
        "unknown format wrote nothing to stderr"
    );
}

/// A panic reaches the user as a signal death or an abort message, so it is
/// distinguishable from the clean exit 1 the error path takes.
#[test]
fn error_path_exits_one_rather_than_panicking() {
    let out = run(&["validate", "/nonexistent/quill/path"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected a clean exit 1, got {:?}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
}
