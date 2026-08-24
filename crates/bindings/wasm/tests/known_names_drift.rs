//! Drift guard for the names `runtime/runtime.js` spells for itself instead of
//! reading off a type. Each is checked against the Rust constant it mirrors,
//! because nothing else observes the two diverging: an `isUnknown*` table that
//! lags a new built-in reports it as unknown, and a read-modify-write consumer
//! then round-trips it through its unknown carrier, dropping the payload.

use quillmark_content::island::KnownIslandType;
use quillmark_content::{Container, Content, Fidelity};
use quillmark_core::quill::VARIANT_DISCRIMINANT_KEY;

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

/// The `.d.ts` is pinned as a string *literal* type: widened to `string` it
/// would stop narrowing an index into the container.
#[test]
fn js_variant_discriminant_matches_the_rust_key() {
    const RUNTIME_DTS: &str = include_str!("../runtime/runtime.d.ts");
    let key = VARIANT_DISCRIMINANT_KEY;

    for (file, decl) in [
        (RUNTIME_JS, format!("export const VARIANT_DISCRIMINANT_KEY = '{key}'")),
        (
            RUNTIME_DTS,
            format!("export declare const VARIANT_DISCRIMINANT_KEY: '{key}'"),
        ),
    ] {
        assert!(file.contains(&decl), "the JS layer has no `{decl}`");
    }
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

/// `weldsWith` in `runtime.js` re-spells the rule `Container::same_weld` owns:
/// which fields two adjacent runs must share for the Markdown projection to
/// read them as one, and therefore for a writer to owe a discriminator. The
/// Rust half here is read off the predicate rather than restated, so a change to
/// the rule fails here instead of welding two runs at every JS consumer.
#[test]
fn js_weld_keys_match_the_rust_weld_rule() {
    fn body() -> &'static str {
        const DECL: &str = "const WELD_KEYS = {";
        let start = RUNTIME_JS
            .find(DECL)
            .expect("runtime.js has no `const WELD_KEYS = {…`")
            + DECL.len();
        let rest = &RUNTIME_JS[start..];
        &rest[..rest.find('}').expect("unterminated WELD_KEYS literal")]
    }

    fn keys(tag: &str) -> Vec<String> {
        let decl = format!("{tag}: [");
        let start = body()
            .find(&decl)
            .unwrap_or_else(|| panic!("WELD_KEYS has no `{tag}`"))
            + decl.len();
        let rest = &body()[start..];
        rest[..rest.find(']').expect("unterminated key list")]
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_matches('\'').to_string())
            .collect()
    }

    let tags: Vec<String> = body()
        .split(']')
        .filter_map(|chunk| chunk.split_once(':'))
        .map(|(tag, _)| tag.trim().trim_start_matches(',').trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    // A built-in arriving without an entry would fall through to the unknown
    // branch, which compares an `attrs` a known arm does not carry.
    assert_eq!(tags, Content::RESERVED_CONTAINERS);

    let li = |ordered, start, ordinal, instance| Container::ListItem {
        ordered,
        start,
        ordinal,
        instance,
    };
    let base = li(false, 1, 0, 0);
    let reacts: Vec<&str> = [
        ("ordered", li(true, 1, 0, 0)),
        ("start", li(false, 3, 0, 0)),
        ("ordinal", li(false, 1, 1, 0)),
        ("instance", li(false, 1, 0, 1)),
    ]
    .into_iter()
    .filter(|(_, other)| !base.same_weld(other))
    .map(|(name, _)| name)
    .collect();
    assert_eq!(keys("list_item"), reacts);

    assert!(keys("quote").is_empty());
    assert!(Container::Quote { instance: 0 }.same_weld(&Container::Quote { instance: 1 }));
}
