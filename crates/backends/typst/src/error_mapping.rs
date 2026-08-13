//! Translates Typst diagnostics into Quillmark [`Diagnostic`](quillmark_core::Diagnostic) values.

use crate::world::QuillWorld;
use quillmark_core::{Diagnostic, Location, Severity};
use typst::diag::SourceDiagnostic;

pub fn map_typst_errors(errors: &[SourceDiagnostic], world: &QuillWorld) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|e| map_single_diagnostic(e, world))
        .collect()
}

fn map_single_diagnostic(error: &SourceDiagnostic, world: &QuillWorld) -> Diagnostic {
    let severity = match error.severity {
        typst::diag::Severity::Error => Severity::Error,
        typst::diag::Severity::Warning => Severity::Warning,
    };

    let location = resolve_span_to_location(error.span, world);

    let hint = error.hints.first().map(|h| h.v.to_string());

    // Typst has no error codes; the message prefix stands in.
    let code = Some(format!(
        "typst::{}",
        error.message.split(':').next().unwrap_or("error").trim()
    ));

    let mut diag = Diagnostic::new(severity, error.message.to_string());
    diag.code = code;
    diag.location = location;
    diag.hint = hint;
    diag
}

fn resolve_span_to_location(span: typst::syntax::DiagSpan, world: &QuillWorld) -> Option<Location> {
    use typst::{World, WorldExt};

    // A diagnostic from an injected helper or vendored package reports
    // coordinates in that file, not main.typ. Detached spans fall back to main.
    let source_id = span.id().unwrap_or_else(|| world.main());
    let source = world.source(source_id).ok()?;
    let range = world.range(span)?;

    let text = source.text();
    let line = text[..range.start].matches('\n').count() + 1;
    let column = range.start - text[..range.start].rfind('\n').map_or(0, |pos| pos + 1) + 1;

    Some(Location::new(
        source.id().vpath().get_without_slash().to_string(),
        line as u32,
        column as u32,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TypstBackend;
    use quillmark_core::{Backend, FileTreeNode, OutputFormat, Quill, RenderOptions};
    use typst::diag::SourceDiagnostic;
    use typst::syntax::Span;

    /// `None` when the fixture is absent (a stripped checkout).
    fn walk_fixture() -> Option<FileTreeNode> {
        let quill_path = quillmark_fixtures::quills_path("usaf_memo");
        if !quill_path.exists() {
            return None;
        }
        Some(quillmark::tree_from_path(quill_path).expect("walk fixture"))
    }

    fn fixture_world() -> Option<QuillWorld> {
        let tree = walk_fixture()?;
        let source = Quill::from_tree(tree).expect("load source");
        Some(QuillWorld::new(&source, "// Test").expect("create world"))
    }

    /// The fixture's `typst.plate_file: plate.typ` makes the backend read this.
    fn source_with_plate(plate: &str) -> Option<Quill> {
        let mut tree = walk_fixture()?;
        if let FileTreeNode::Directory { files } = &mut tree {
            files.insert(
                "plate.typ".to_string(),
                FileTreeNode::File {
                    contents: plate.as_bytes().to_vec(),
                },
            );
        }
        Some(Quill::from_tree(tree).expect("load source"))
    }

    #[test]
    fn unresolvable_span_keeps_existing_typst_hint() {
        let Some(world) = fixture_world() else {
            return;
        };

        let diag = SourceDiagnostic::error(Span::detached(), "unexpected closing bracket")
            .with_hint("try using a backslash escape: \\]");
        let mapped = map_single_diagnostic(&diag, &world);

        assert!(mapped.location.is_none());
        assert_eq!(
            mapped.hint.as_deref(),
            Some("try using a backslash escape: \\]"),
            "an existing Typst hint must not be overwritten"
        );
    }

    const EVAL_ERROR_PLATE: &str =
        "#set page(width: 400pt, height: 300pt)\n#eval(\"#general\", mode: \"markup\")\n";

    #[test]
    fn resolvable_eval_error_is_unchanged() {
        let Some(source) = source_with_plate(EVAL_ERROR_PLATE) else {
            return;
        };

        // Compilation happens during `open`, so the error may surface there.
        let diags = match TypstBackend.open(&source, &serde_json::json!({})) {
            Ok(session) => session
                .render(&RenderOptions::default().with_output_format(OutputFormat::Pdf))
                .expect_err("eval of `#general` should fail to compile")
                .into_diagnostics(),
            Err(err) => err.into_diagnostics(),
        };
        assert!(
            !diags.is_empty(),
            "compilation error must carry diagnostics"
        );

        let diag = diags
            .iter()
            .find(|d| d.message.contains("unknown variable: general"))
            .expect("expected the `unknown variable: general` diagnostic");

        assert!(
            diag.location.is_some(),
            "this eval error resolves to the call site; expected a location, got None"
        );
    }
}
