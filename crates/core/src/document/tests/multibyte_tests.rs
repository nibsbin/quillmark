//! Regression coverage for the "string index N is not a character boundary"
//! panic class on the YAML scanner paths that run *after* the prescan.
//!
//! The bullet-marker path is pinned at its own altitude in
//! [`crate::document::prescan`] (`sequence_with_multibyte_after_dash_does_not_panic`);
//! what lives here is the quoted-scalar reader and the caret renderer that
//! formats a YAML error — both of which index into the source line and so must
//! respect char boundaries. Each test's crucial assertion is "did not panic";
//! whether the input parses is secondary.

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
fn multibyte_keys_do_not_panic_on_duplicate() {
    // YAML error formatting can include the offending key in its caret message.
    // A multibyte key is the suspect input for the caret renderer.
    let md = "~~~card-yaml\n$quill: q@0.1\n$kind: main\nf\u{2014}o: 1\nf\u{2014}o: 2\n~~~\n";
    // A duplicate-key parse error is the expected outcome; a panic is not.
    let _ = decompose(md);
}

#[test]
fn multibyte_in_value_with_yaml_error_does_not_panic() {
    // A value carrying multibyte chars alongside a YAML structural bug on the
    // same line — caret positioning has to scan past the multibyte chars.
    let inputs = [
        "~~~card-yaml\n$quill: q@0.1\n$kind: main\nx: hello \u{2014} world\nbluf: *bad-alias\n~~~\n",
        "~~~card-yaml\n$quill: q@0.1\n$kind: main\nsystem_name: \u{201C}Service\u{201D}: Order API\n~~~\n",
    ];
    for input in inputs {
        let _ = decompose(input);
    }
}
