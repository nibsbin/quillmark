//! Holds `docs/reference/diagnostic-codes.md` and the minted codes to each
//! other: a rename edits the page in the same commit, where a reviewer sees it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn is_code(s: &str) -> bool {
    let Some((ns, name)) = s.split_once("::") else {
        return false;
    };
    let lead = |t: &str| t.starts_with(|c: char| c.is_ascii_lowercase());
    lead(ns)
        && lead(name)
        && ns.chars().all(|c| c.is_ascii_lowercase() || c == '_')
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Every `"ns::name"` or `'ns::name'` literal in `text`.
fn quoted_codes(text: &str, into: &mut BTreeSet<String>) {
    let bytes = text.as_bytes();
    for (i, &q) in bytes.iter().enumerate() {
        if q != b'"' && q != b'\'' {
            continue;
        }
        let rest = &text[i + 1..];
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':'))
            .unwrap_or(rest.len());
        if rest.as_bytes().get(end) == Some(&q) && is_code(&rest[..end]) {
            into.insert(rest[..end].to_string());
        }
    }
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("source dir reads") {
        let path = entry.expect("dir entry").path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if !matches!(name, "target" | "node_modules" | "pkg" | ".venv") {
                walk(&path, files);
            }
        } else if name.ends_with(".rs") && !path.ends_with(file!()) {
            files.push(path);
        }
    }
}

fn minted() -> BTreeSet<String> {
    let mut files = Vec::new();
    walk(&Path::new(ROOT).join("crates"), &mut files);
    // The WASM runtime mints `runtime::*` in JavaScript.
    for name in ["runtime.js", "env-node.js", "env-web.js"] {
        files.push(Path::new(ROOT).join("crates/bindings/wasm/runtime").join(name));
    }
    let mut codes = BTreeSet::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("source reads");
        quoted_codes(&text, &mut codes);
    }
    codes
}

fn listed() -> BTreeSet<String> {
    let page = std::fs::read_to_string(Path::new(ROOT).join("docs/reference/diagnostic-codes.md"))
        .expect("diagnostic-codes.md reads");
    page.lines()
        .filter_map(|line| line.strip_prefix("- `")?.split_once('`'))
        .map(|(code, _)| code)
        .filter(|code| is_code(code))
        .map(str::to_string)
        .collect()
}

/// The page's two derived families: `conform::<edit class>` and
/// `quill::seed_<validation class>`.
fn derived(code: &str, listed: &BTreeSet<String>) -> bool {
    if let Some(class) = code.strip_prefix("conform::") {
        return listed.contains(&format!("edit::{class}"));
    }
    if let Some(class) = code.strip_prefix("quill::seed_") {
        return listed.contains(&format!("validation::{class}"))
            || listed.contains(&format!("validation::seed_{class}"));
    }
    false
}

#[test]
fn every_minted_code_is_listed_and_every_listed_code_is_minted() {
    let minted = minted();
    let listed = listed();

    let unlisted: Vec<_> = minted
        .iter()
        .filter(|c| !listed.contains(*c) && !derived(c, &listed))
        .collect();
    assert!(
        unlisted.is_empty(),
        "minted but missing from docs/reference/diagnostic-codes.md: {unlisted:?}"
    );

    let stale: Vec<_> = listed.difference(&minted).collect();
    assert!(
        stale.is_empty(),
        "listed in docs/reference/diagnostic-codes.md but minted nowhere: {stale:?}; \
         a removed or renamed code leaves the page in the release whose migration guide names it"
    );
}
