use serde_json::Value as JsonValue;
use std::fs;
use std::path::PathBuf;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

// This test validates end-to-end rendering with the native engine.
// It's kept for regression testing but not part of the WASM API validation.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[ignore] // Ignore for now - JSON format needs updating to match new API
fn test_process_web_input_quill_from_json() {
    // Load the JSON fixture shipped with the crate
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let json_path = manifest_dir.join("tests").join("web_input_quill.json");
    let json_str = std::fs::read_to_string(&json_path)
        .expect(&format!("failed to read fixture: {}", json_path.display()));

    // Parse JSON fixture to extract a markdown file to render
    let json_val: JsonValue = serde_json::from_str(&json_str).expect("invalid JSON fixture");

    // Extract first markdown file from the JSON
    let mut markdown: Option<String> = None;
    if let JsonValue::Object(map) = &json_val {
        for (k, v) in map {
            if k.ends_with(".md") {
                if let Some(s) = v.get("contents").and_then(|c| c.as_str()) {
                    markdown = Some(s.to_string());
                    break;
                }
            }
        }
    }

    let markdown = match markdown {
        Some(m) => m,
        None => panic!("No markdown file found in JSON fixture to render"),
    };

    // Use native quillmark engine for rendering (tests run natively)
    // Obtain a core Quill by parsing the same JSON with the core API.
    let quill_core =
        quillmark_core::Quill::from_json(&json_str).expect("core from_json should succeed");

    let mut engine = quillmark::QuillmarkEngine::new();
    engine.register_quill(quill_core.clone());
    let workflow = engine.load(&quill_core).expect("failed to load workflow");
    let render_result = workflow.render(&markdown, None).expect("render failed");

    // Determine the workspace fixtures output directory (from crate manifest dir)
    let workspace_root = manifest_dir
        .parent()
        .expect("failed to locate workspace root from CARGO_MANIFEST_DIR")
        .to_path_buf();

    let fixtures_output = workspace_root.join("quillmark-fixtures").join("output");

    // Write artifacts to the fixtures output dir (matching example behavior)
    for (i, art) in render_result.artifacts.into_iter().enumerate() {
        let filename = match art.output_format {
            quillmark_core::OutputFormat::Pdf => format!("render-output-{}.pdf", i + 1),
            quillmark_core::OutputFormat::Svg => format!("render-output-{}.svg", i + 1),
            quillmark_core::OutputFormat::Txt => format!("render-output-{}.txt", i + 1),
        };
        let out_path = fixtures_output.join(filename);
        fs::create_dir_all(out_path.parent().unwrap()).ok();
        fs::write(&out_path, &art.bytes).expect("failed to write artifact");
    }
}
