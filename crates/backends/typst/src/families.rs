//! Family resolution at compile: a `font_family` mark naming a family the
//! session's fonts lack sets no family, and warns once per field.

use std::collections::BTreeMap;

use quillmark_core::error::{Diagnostic, Severity};
use quillmark_core::path::DocPath;
use quillmark_core::quill::QuillConfig;
use quillmark_core::Content;

/// The families a session sets running text in, lowercased.
pub(crate) type Families = std::collections::BTreeSet<String>;

const CODE: &str = "typst::unknown_font_family";

/// One `typst::unknown_font_family` per family a content field of the plate
/// JSON `data` names and `families` lacks, spelled as the field first does.
pub(crate) fn unknown(config: &QuillConfig, data: &serde_json::Value, families: &Families) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    config.each_plate_content(data, &mut |at: &DocPath, content: &Content| {
        let mut missing: BTreeMap<String, String> = BTreeMap::new();
        for name in content.font_families() {
            let key = name.to_lowercase();
            if !families.contains(&key) {
                missing.entry(key).or_insert(name);
            }
        }
        for name in missing.into_values() {
            diags.push(
                Diagnostic::new(
                    Severity::Warning,
                    format!("this field sets text in `{name}`, a family none of the session's fonts holds, so that text keeps the surrounding font"),
                )
                .with_code(CODE.to_string())
                .with_arg("family", name.clone().into())
                .with_path(at.to_string())
                .with_hint("ship the family in the quill or the host's font library, or choose a family they hold".to_string()),
            );
        }
    });
    diags
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use quillmark_core::quill::{FileTreeNode, Quill};
    use typst::layout::{Frame, FrameItem};

    use crate::{compile, read_plate, world, FontLibrary, SchemaMeta};

    const YAML: &str = "quill:\n  name: fonts\n  version: 0.1.0\n  backend: typst\n  description: font runs\ntypst:\n  plate_file: plate.typ\nmain:\n  fields:\n    prose:\n      type: richtext\n      multiline: true\n      description: body\n";

    fn figtree() -> Vec<u8> {
        std::fs::read(quillmark_fixtures::quills_path("taro").join("assets/fonts/Figtree-Regular.ttf"))
            .expect("read Figtree")
    }

    /// Each run of text `prose` places, as `(text, family, size in pt)`, and
    /// the session's warnings.
    fn runs(plate_head: &str, md: &str, library: FontLibrary) -> (Vec<(String, String, f64)>, Vec<quillmark_core::error::Diagnostic>) {
        let plate = format!(
            "#import \"@local/quillmark-helper:0.2.0\" as qm\n#set page(width: 400pt, height: 300pt, margin: 20pt)\n#set text(size: 10pt)\n{plate_head}\n#qm.data.prose\n"
        );
        let mut files = HashMap::new();
        files.insert("Quill.yaml".to_string(), FileTreeNode::File { contents: YAML.as_bytes().to_vec() });
        files.insert("plate.typ".to_string(), FileTreeNode::File { contents: plate.into_bytes() });
        let quill = Quill::from_tree(FileTreeNode::Directory { files }).expect("quill");
        let rt = quillmark_content::import::from_markdown(md).expect("import").content;
        let data = serde_json::json!({ "prose": quillmark_content::serial::to_canonical_value(&rt) });
        let meta = SchemaMeta::from_schema_json(quillmark_core::quill::build_transform_schema(quill.config()).as_json());
        let mut world = world::QuillWorld::new(&quill, &read_plate(&quill).expect("plate"), &Arc::new(library))
            .expect("world");
        world.inject_helper_package(&data, &meta).expect("inject");
        let (document, mut warnings) = compile::compile_document(&world).expect("compile");
        warnings.extend(super::unknown(quill.config(), &data, world.families()));

        fn walk(frame: &Frame, out: &mut Vec<(String, String, f64)>) {
            for (_, item) in frame.items() {
                match item {
                    FrameItem::Group(g) => walk(&g.frame, out),
                    FrameItem::Text(t) => out.push((t.text.to_string(), t.font.info().family.clone(), t.size.to_pt())),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        for page in document.pages() {
            walk(&page.frame, &mut out);
        }
        (out, warnings)
    }

    fn run<'a>(runs: &'a [(String, String, f64)], text: &str) -> (&'a str, f64) {
        let (_, family, size) = runs.iter().find(|r| r.0.contains(text)).unwrap_or_else(|| panic!("{text:?} in {runs:?}"));
        (family.as_str(), (size * 100.0).round() / 100.0)
    }

    #[test]
    fn a_run_sets_its_family_ahead_of_the_surrounding_fonts_and_its_step() {
        let library = FontLibrary::new(vec![figtree()]).expect("library");
        let md = "plain <qm-font family=\"figtree\" size=\"large\">big</qm-font> \
                  <qm-font size=\"small\">fine `mono` tail</qm-font> \
                  <qm-font family=\"Nope\">gone</qm-font> <qm-font family=\"Nope\" size=\"large\">kept</qm-font>";
        let (runs, warnings) = runs("", md, library);
        assert_eq!(run(&runs, "plain"), ("Roboto", 10.0));
        assert_eq!(run(&runs, "big"), ("Figtree", 12.0));
        assert_eq!(run(&runs, "fine"), ("Roboto", 8.5));
        assert_eq!(run(&runs, "mono").1, 8.0, "a code span keeps raw's own size: {runs:?}");
        assert_eq!(run(&runs, "gone"), ("Roboto", 10.0));
        assert_eq!(run(&runs, "kept"), ("Roboto", 12.0));
        let codes: Vec<(&str, &str)> = warnings
            .iter()
            .map(|d| (d.code.as_deref().unwrap_or(""), d.message.as_str()))
            .collect();
        assert_eq!(codes.len(), 1, "one warning for the field, none of Typst's own: {codes:?}");
        assert_eq!(codes[0].0, "typst::unknown_font_family");
        assert_eq!(warnings[0].path.as_deref(), Some("main.prose"));
        assert_eq!(warnings[0].args.get("family"), Some(&serde_json::json!("Nope")));
    }

    #[test]
    fn a_math_face_is_no_family_a_run_sets() {
        let (runs, warnings) = runs(
            "",
            "<qm-font family=\"Noto Sans Math\">abc</qm-font> <qm-font family=\"New Computer Modern Math\">def</qm-font>",
            FontLibrary::default(),
        );
        assert_eq!(run(&runs, "abc").0, "Roboto");
        assert_eq!(run(&runs, "def").0, "Roboto");
        let families: Vec<_> = warnings.iter().filter_map(|d| d.args.get("family")).collect();
        assert_eq!(families, [&serde_json::json!("New Computer Modern Math"), &serde_json::json!("Noto Sans Math")]);
    }

    #[test]
    fn a_plate_sets_the_steps_and_a_family_falls_through_for_what_it_lacks() {
        let library = FontLibrary::new(vec![figtree()]).expect("library");
        let (runs, _) = runs(
            "#qm.font-steps.update((large: 2em))\n#set text(font: (\"Roboto\", \"Nowhere\"))",
            "<qm-font size=\"large\">big</qm-font> <qm-font size=\"small\">fine</qm-font> <qm-font family=\"Figtree\">x中</qm-font>",
            library,
        );
        assert_eq!(run(&runs, "big"), ("Roboto", 20.0));
        assert_eq!(run(&runs, "fine"), ("Roboto", 8.5), "a step the plate leaves out keeps its default");
        assert_eq!(run(&runs, "x").0, "Figtree");
    }

    #[test]
    fn a_footnote_reference_takes_its_run_and_its_note_only_its_own_marks() {
        let (runs, _) = runs(
            "",
            "a <qm-font size=\"large\">b[^1] c</qm-font>\n\n[^1]: <qm-font size=\"small\">note</qm-font> rest",
            FontLibrary::default(),
        );
        let sizes: Vec<f64> = runs.iter().map(|r| (r.2 * 1000.0).round() / 1000.0).collect();
        let texts: Vec<&str> = runs.iter().map(|r| r.0.as_str()).collect();
        assert_eq!(texts, ["a ", "b", "1", " c", "1", "note", " rest"]);
        assert_eq!(sizes, [10.0, 12.0, 12.0, 12.0, 8.5, 7.225, 8.5]);
    }

    #[test]
    fn an_equation_breaks_the_run() {
        let (runs, _) = runs("", "<qm-font size=\"large\">a $x$ b</qm-font>", FontLibrary::default());
        assert_eq!(run(&runs, "a").1, 12.0);
        assert_eq!(run(&runs, "𝑥").1, 10.0, "{runs:?}");
        assert_eq!(run(&runs, "b").1, 12.0);
    }
}
