//! Drift guard for the open sets' known halves. `runtime/runtime.js`'s
//! `isUnknown*` guards decide known-vs-unknown from hand-spelled name tables; a
//! table that lags a new built-in reports it as unknown, and a read-modify-write
//! consumer then round-trips it through its unknown carrier, silently dropping
//! the payload. So the tables are checked against the Rust constants they mirror.

use quillmark_content::island::KnownIslandType;
use quillmark_content::{Content, Fidelity};

const RUNTIME_JS: &str = include_str!("../runtime/runtime.js");

fn js_set(name: &str) -> Vec<String> {
    let decl = format!("const {name} = new Set([");
    let start = RUNTIME_JS
        .find(&decl)
        .unwrap_or_else(|| panic!("runtime.js has no `{decl}…`"))
        + decl.len();
    let body = &RUNTIME_JS[start..];
    let end = body.find("])").expect("unterminated Set literal");
    body[..end]
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_matches('\'').trim_matches('"').to_string())
        .collect()
}

#[test]
fn js_known_name_tables_match_the_rust_open_sets() {
    assert_eq!(js_set("KNOWN_LINE_KINDS"), Content::RESERVED_LINE_KINDS);
    assert_eq!(js_set("KNOWN_CONTAINERS"), Content::RESERVED_CONTAINERS);
    assert_eq!(js_set("KNOWN_MARK_TYPES"), Content::RESERVED_MARK_TYPES);
    // The island axis has no reserved-name rule (its `type` is a bare `String`,
    // never an `Unknown` variant) so `KnownIslandType` is the known half.
    let island_types: Vec<_> = KnownIslandType::ALL.iter().map(|k| k.as_str()).collect();
    assert_eq!(js_set("KNOWN_ISLAND_TYPES"), island_types);
}

/// The same names are spelled a third time as TypeScript unions in
/// `src/engine.rs`; a union that lags a new built-in is a consumer that cannot
/// narrow the new arm. Arm *shape* varies, so this asserts only that the name
/// appears in the union.
#[test]
fn ts_unions_name_every_built_in() {
    const ENGINE_RS: &str = include_str!("../src/engine.rs");

    /// Bounded by the blank line before the next declaration, since an arm's own
    /// `;` separates its members and cannot terminate the search.
    fn ts_union(name: &str) -> &'static str {
        let decl = format!("export type {name} =");
        let start = ENGINE_RS
            .find(&decl)
            .unwrap_or_else(|| panic!("engine.rs has no `{decl}`"))
            + decl.len();
        let body = &ENGINE_RS[start..];
        &body[..body.find("\n\n").expect("unterminated type alias")]
    }

    // The loss axis has no reserved list to mirror, being injective (one `Loss`
    // per wire string), so what it pins is its closed view's spellings.
    let loss_names: Vec<_> = Fidelity::ALL.iter().map(|f| f.as_str()).collect();

    for (union, names) in [
        (ts_union("ContentLineKind"), Content::RESERVED_LINE_KINDS),
        (ts_union("ContentContainer"), Content::RESERVED_CONTAINERS),
        (ts_union("ContentMark"), Content::RESERVED_MARK_TYPES),
        (ts_union("ContentLossClass"), loss_names.as_slice()),
    ] {
        for name in names {
            assert!(
                union.contains(&format!("\"{name}\"")),
                "a TS union in engine.rs is missing the `{name}` arm"
            );
        }
    }
}
