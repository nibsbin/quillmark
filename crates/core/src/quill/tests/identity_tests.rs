//! `quill.license` and `quill.homepage`: optional, and held to their shapes
//! when present.
use super::*;

fn load(keys: &str) -> Result<(QuillConfig, Vec<Diagnostic>), Vec<Diagnostic>> {
    QuillConfig::from_yaml_with_warnings(&format!(
        "quill:\n  name: memo\n  version: \"1.0\"\n  backend: typst\n  description: memo\n{keys}"
    ))
}

fn codes(diags: &[Diagnostic]) -> Vec<&str> {
    diags.iter().filter_map(|d| d.code.as_deref()).collect()
}

#[test]
fn license_and_homepage_load_as_written_and_stay_absent_when_omitted() {
    let license = "(MIT OR Apache-2.0 WITH LLVM-exception) AND LicenseRef-Seal";
    let homepage = "HTTPS://user@example.com:8443/quills?tab=memo#top";
    let (config, warnings) = load(&format!("  license: {license}\n  homepage: {homepage}\n"))
        .expect("both are well-formed");
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(config.license.as_deref(), Some(license));
    assert_eq!(config.homepage.as_deref(), Some(homepage));

    let (bare, warnings) = load("").expect("both are optional");
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!((bare.license, bare.homepage), (None, None));
}

#[test]
fn a_license_off_the_spdx_grammar_or_list_is_refused() {
    for license in [
        "\"\"",
        "Apache 2.0",
        "Proprietary",
        "MIT OR",
        "(MIT",
        "MIT/Apache-2.0",
        "MIT +",
        "GPL-3.0",
        "42",
        "[MIT]",
        "~",
    ] {
        let refused = load(&format!("  license: {license}\n")).expect_err(license);
        assert_eq!(codes(&refused), ["quill::invalid_license"], "{license}");
    }

    let refused = load("  license: MIT OR apache-2.0\n").expect_err("ids match by case");
    let hint = refused[0].hint.as_deref().unwrap_or_default();
    assert!(hint.contains("'Apache-2.0'"), "the hint names the listed id: {hint}");
}

#[test]
fn a_homepage_that_is_not_an_absolute_web_url_is_refused() {
    for homepage in [
        "github.com/org/quills",
        "//github.com/org/quills",
        "ftp://example.com/quills",
        "mailto:quills@example.com",
        "https://",
        "https:///quills",
        "https://user@:8443/quills",
        "\"https://example.com/my quills\"",
        "\"\"",
        "42",
        "~",
    ] {
        let refused = load(&format!("  homepage: {homepage}\n")).expect_err(homepage);
        assert_eq!(codes(&refused), ["quill::invalid_homepage"], "{homepage}");
    }
}
