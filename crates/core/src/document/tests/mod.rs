mod ambiguous_strings_tests;
mod assemble_tests;
mod card_fence_tests;
mod edit_tests;
mod emit_idempotence_tests;
mod emit_stability_tests;
mod emit_tests;
mod ext_tests;
mod fence_conformance_tests;
mod lossiness_tests;
mod multibyte_tests;
mod number_edge_tests;
mod seed_tests;

/// Every `.md` file reachable from `root`, walked recursively. Picks up the
/// `README.md`/`LICENSE.md` inside bundled quill packages too; those carry no
/// root card-yaml block and are skipped at parse time.
///
/// Not what `emit_tests::fixtures_round_trip` wants: that one scans the
/// top level only, deliberately.
pub(super) fn collect_md_files(root: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
}
