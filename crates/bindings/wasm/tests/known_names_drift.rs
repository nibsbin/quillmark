//! Drift guard for the open sets' known halves.
//!
//! `runtime/runtime.js`'s `isUnknown*` guards decide known-vs-unknown from
//! hand-spelled name tables. A table that lags a new built-in reports that
//! built-in as unknown, and a read-modify-write consumer round-trips it through
//! its unknown carrier, dropping the payload: silently, since the wire accepts
//! the resulting write. So the tables are checked against the Rust constants
//! they mirror rather than against a comment asking someone to remember.
//!
//! Native, not `wasm_bindgen_test`: this reads source text, so it needs no
//! browser and no instantiated module.

use quillmark_content::island::KnownIslandType;
use quillmark_content::{Content, Fidelity, Loss};

const RUNTIME_JS: &str = include_str!("../runtime/runtime.js");

/// The string literals of `const <name> = new Set([…]);`, in source order.
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
/// `src/engine.rs`. A union that lags a new built-in is a consumer that cannot
/// narrow the new arm, so it is pinned here too: one place to look when adding
/// a built-in. Arm *shape* varies (the payload-free mark types share one arm),
/// so this asserts the name appears in the union, not how it is spelled there.
#[test]
fn ts_unions_name_every_built_in() {
    const ENGINE_RS: &str = include_str!("../src/engine.rs");

    /// The declaration body of `export type <name> = …`. Bounded by the blank
    /// line before the next declaration, since an arm's own `;` separates its
    /// members and cannot terminate the search.
    fn ts_union(name: &str) -> &'static str {
        let decl = format!("export type {name} =");
        let start = ENGINE_RS
            .find(&decl)
            .unwrap_or_else(|| panic!("engine.rs has no `{decl}`"))
            + decl.len();
        let body = &ENGINE_RS[start..];
        &body[..body.find("\n\n").expect("unterminated type alias")]
    }

    // The loss axis names its closed view's classes, not a reserved list: it has
    // no reserved list, being injective (one `Loss` per wire string).
    let loss_classes: Vec<_> = Fidelity::ALL.iter().map(|f| f.class()).collect();
    let loss_names: Vec<_> = loss_classes.iter().map(Loss::as_str).collect();

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
