//! Regression coverage for the "string index N is not a character boundary"
//! panic class on the YAML scanner paths that run *after* the prescan.
//!
//! The bullet-marker path is pinned in [`crate::document::prescan`]
//! (`sequence_with_multibyte_after_dash_does_not_panic`) and the caret renderer
//! in [`crate::document::yaml_hints`]; what lives here is the quoted-scalar
//! reader, which has no unit owner, and the wiring that hands the renderer a
//! char-bounded slice. Each test's crucial assertion is "did not panic"; whether
//! the input parses is secondary.

use crate::document::assemble::decompose;

#[test]
fn multibyte_in_quoted_scalar_parses() {
    // Quoted scalars with multibyte content land in a different scanner path
    // than plain scalars. Cover both styles.
    let single = "~~~card-yaml\n$quill: q@0.1\n$kind: main\nbluf: '\u{2014}leading em-dash'\n~~~\n";
    let double =
        "~~~card-yaml\n$quill: q@0.1\n$kind: main\nbluf: \"\u{201C}smart-quoted\u{201D}\"\n~~~\n";
    for input in [single, double] {
        decompose(input).unwrap_or_else(|e| panic!("expected parse to succeed, got: {e}"));
    }
}

#[test]
fn multibyte_yaml_errors_reach_the_renderer_char_bounded() {
    // The renderer's own multibyte guarantee is pinned at its altitude
    // (`yaml_hints::does_not_panic_on_multibyte_content`); what this covers is
    // the wiring — that `assemble` hands it a char-bounded slice, whether the
    // multibyte chars sit in a key, before a structural bug on the same line, or
    // in the value the caret has to scan past. A parse error is the expected
    // outcome; a panic is not.
    let inputs = [
        "~~~card-yaml\n$quill: q@0.1\n$kind: main\nf\u{2014}o: 1\nf\u{2014}o: 2\n~~~\n",
        "~~~card-yaml\n$quill: q@0.1\n$kind: main\nx: hello \u{2014} world\nbluf: *bad-alias\n~~~\n",
        "~~~card-yaml\n$quill: q@0.1\n$kind: main\nsystem_name: \u{201C}Service\u{201D}: Order API\n~~~\n",
    ];
    for input in inputs {
        let _ = decompose(input);
    }
}
