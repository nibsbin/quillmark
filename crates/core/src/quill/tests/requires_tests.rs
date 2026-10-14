use super::*;

fn load(requires: &str, extra: &str) -> Result<(QuillConfig, Vec<Diagnostic>), Vec<Diagnostic>> {
    QuillConfig::from_yaml_with_warnings(&format!(
        "quill:\n  name: memo\n  version: \"1.0\"\n  backend: typst\n  description: memo\n  requires: {requires}\n{extra}"
    ))
}

fn codes(diags: &[Diagnostic]) -> Vec<&str> {
    diags.iter().filter_map(|d| d.code.as_deref()).collect()
}

#[test]
fn a_floor_this_engine_meets_loads() {
    load("\">=0.1\"", "").expect("0.1 is below any released engine");
    load(&format!("\">={}\"", env!("CARGO_PKG_VERSION")), "").expect("the floor is inclusive");
}

/// The keys a newer engine reads would otherwise read as typos.
#[test]
fn a_floor_above_this_engine_is_the_one_error() {
    let refused = load("\">=9999.0\"", "  from_the_future: true\nnew_section: {}\n")
        .expect_err("9999.0 is above this engine");
    assert_eq!(codes(&refused), ["quill::engine_too_old"]);
    assert!(refused[0].message.contains("9999.0.0"), "{}", refused[0].message);
}

#[test]
fn a_floor_is_spelled_with_its_comparator() {
    for requires in ["\"1.0\"", "\">1.0\"", "\">=1\"", "\">=x.y\"", "1.0", "[\">=1.0\"]"] {
        let refused = load(requires, "").expect_err(requires);
        assert_eq!(codes(&refused), ["quill::invalid_requires"], "{requires}");
    }
}
