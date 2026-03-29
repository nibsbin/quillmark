//! Tests for quill types and loading.

use super::*;
use crate::Severity;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_quillignore_parsing() {
    let ignore_content = r#"
# This is a comment
*.tmp
target/
node_modules/
.git/
"#;
    let ignore = QuillIgnore::from_content(ignore_content);
    assert_eq!(ignore.patterns.len(), 4);
    assert!(ignore.patterns.contains(&"*.tmp".to_string()));
    assert!(ignore.patterns.contains(&"target/".to_string()));
}

#[test]
fn test_quillignore_matching() {
    let ignore = QuillIgnore::new(vec![
        "*.tmp".to_string(),
        "target/".to_string(),
        "node_modules/".to_string(),
        ".git/".to_string(),
    ]);

    // Test file patterns
    assert!(ignore.is_ignored("test.tmp"));
    assert!(ignore.is_ignored("path/to/file.tmp"));
    assert!(!ignore.is_ignored("test.txt"));

    // Test directory patterns
    assert!(ignore.is_ignored("target"));
    assert!(ignore.is_ignored("target/debug"));
    assert!(ignore.is_ignored("target/debug/deps"));
    assert!(!ignore.is_ignored("src/target.rs"));

    assert!(ignore.is_ignored("node_modules"));
    assert!(ignore.is_ignored("node_modules/package"));
    assert!(!ignore.is_ignored("my_node_modules"));
}

#[test]
fn test_in_memory_file_system() {
    let temp_dir = TempDir::new().unwrap();
    let quill_dir = temp_dir.path();

    // Create test files
    fs::write(
            quill_dir.join("Quill.yaml"),
            "Quill:\n  name: \"test\"\n  version: \"1.0\"\n  backend: \"typst\"\n  plate_file: \"plate.typ\"\n  description: \"Test quill\"",
        )
        .unwrap();
    fs::write(quill_dir.join("plate.typ"), "test plate").unwrap();

    let assets_dir = quill_dir.join("assets");
    fs::create_dir_all(&assets_dir).unwrap();
    fs::write(assets_dir.join("test.txt"), "asset content").unwrap();

    let packages_dir = quill_dir.join("packages");
    fs::create_dir_all(&packages_dir).unwrap();
    fs::write(packages_dir.join("package.typ"), "package content").unwrap();

    // Load quill
    let quill = Quill::from_path(quill_dir).unwrap();

    // Test file access
    assert!(quill.file_exists("plate.typ"));
    assert!(quill.file_exists("assets/test.txt"));
    assert!(quill.file_exists("packages/package.typ"));
    assert!(!quill.file_exists("nonexistent.txt"));

    // Test file content
    let asset_content = quill.get_file("assets/test.txt").unwrap();
    assert_eq!(asset_content, b"asset content");

    // Test directory listing
    let asset_files = quill.list_directory("assets");
    assert_eq!(asset_files.len(), 1);
    assert!(asset_files.contains(&PathBuf::from("assets/test.txt")));
}

#[test]
fn test_quillignore_integration() {
    let temp_dir = TempDir::new().unwrap();
    let quill_dir = temp_dir.path();

    // Create .quillignore
    fs::write(quill_dir.join(".quillignore"), "*.tmp\ntarget/\n").unwrap();

    // Create test files
    fs::write(
            quill_dir.join("Quill.yaml"),
            "Quill:\n  name: \"test\"\n  version: \"1.0\"\n  backend: \"typst\"\n  plate_file: \"plate.typ\"\n  description: \"Test quill\"",
        )
        .unwrap();
    fs::write(quill_dir.join("plate.typ"), "test template").unwrap();
    fs::write(quill_dir.join("should_ignore.tmp"), "ignored").unwrap();

    let target_dir = quill_dir.join("target");
    fs::create_dir_all(&target_dir).unwrap();
    fs::write(target_dir.join("debug.txt"), "also ignored").unwrap();

    // Load quill
    let quill = Quill::from_path(quill_dir).unwrap();

    // Test that ignored files are not loaded
    assert!(quill.file_exists("plate.typ"));
    assert!(!quill.file_exists("should_ignore.tmp"));
    assert!(!quill.file_exists("target/debug.txt"));
}

#[test]
fn test_find_files_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let quill_dir = temp_dir.path();

    // Create test directory structure
    fs::write(
            quill_dir.join("Quill.yaml"),
            "Quill:\n  name: \"test\"\n  version: \"1.0\"\n  backend: \"typst\"\n  plate_file: \"plate.typ\"\n  description: \"Test quill\"",
        )
        .unwrap();
    fs::write(quill_dir.join("plate.typ"), "template").unwrap();

    let assets_dir = quill_dir.join("assets");
    fs::create_dir_all(&assets_dir).unwrap();
    fs::write(assets_dir.join("image.png"), "png data").unwrap();
    fs::write(assets_dir.join("data.json"), "json data").unwrap();

    let fonts_dir = assets_dir.join("fonts");
    fs::create_dir_all(&fonts_dir).unwrap();
    fs::write(fonts_dir.join("font.ttf"), "font data").unwrap();

    // Load quill
    let quill = Quill::from_path(quill_dir).unwrap();

    // Test pattern matching
    let all_assets = quill.find_files("assets/*");
    assert!(all_assets.len() >= 3); // At least image.png, data.json, fonts/font.ttf

    let typ_files = quill.find_files("*.typ");
    assert_eq!(typ_files.len(), 1);
    assert!(typ_files.contains(&PathBuf::from("plate.typ")));
}

#[test]
fn test_new_standardized_yaml_format() {
    let temp_dir = TempDir::new().unwrap();
    let quill_dir = temp_dir.path();

    // Create test files using new standardized format
    let yaml_content = r#"
Quill:
  name: my-custom-quill
  version: "1.0"
  backend: typst
  plate_file: custom_plate.typ
  description: Test quill with new format
  author: Test Author
"#;
    fs::write(quill_dir.join("Quill.yaml"), yaml_content).unwrap();
    fs::write(
        quill_dir.join("custom_plate.typ"),
        "= Custom Template\n\nThis is a custom template.",
    )
    .unwrap();

    // Load quill
    let quill = Quill::from_path(quill_dir).unwrap();

    // Test that name comes from YAML, not directory
    assert_eq!(quill.name, "my-custom-quill");

    // Test that backend is in metadata
    assert!(quill.metadata.contains_key("backend"));
    if let Some(backend_val) = quill.metadata.get("backend") {
        if let Some(backend_str) = backend_val.as_str() {
            assert_eq!(backend_str, "typst");
        } else {
            panic!("Backend value is not a string");
        }
    }

    // Test that other fields are in metadata including version
    assert!(quill.metadata.contains_key("description"));
    assert!(quill.metadata.contains_key("author"));
    assert!(quill.metadata.contains_key("version")); // version should now be included
    if let Some(version_val) = quill.metadata.get("version") {
        if let Some(version_str) = version_val.as_str() {
            assert_eq!(version_str, "1.0");
        }
    }

    // Test that plate template content is loaded correctly
    assert!(quill.plate.unwrap().contains("Custom Template"));
}

#[test]
fn test_typst_packages_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let quill_dir = temp_dir.path();

    let yaml_content = r#"
Quill:
  name: "test-quill"
  version: "1.0"
  backend: "typst"
  plate_file: "plate.typ"
  description: "Test quill for packages"

typst:
  packages:
    - "@preview/bubble:0.2.2"
    - "@preview/example:1.0.0"
"#;

    fs::write(quill_dir.join("Quill.yaml"), yaml_content).unwrap();
    fs::write(quill_dir.join("plate.typ"), "test").unwrap();

    let quill = Quill::from_path(quill_dir).unwrap();
    let packages = quill.typst_packages();

    assert_eq!(packages.len(), 2);
    assert_eq!(packages[0], "@preview/bubble:0.2.2");
    assert_eq!(packages[1], "@preview/example:1.0.0");
}

#[test]
fn test_template_loading() {
    let temp_dir = TempDir::new().unwrap();
    let quill_dir = temp_dir.path();

    // Create test files with example specified
    let yaml_content = r#"Quill:
  name: "test-with-template"
  version: "1.0"
  backend: "typst"
  plate_file: "plate.typ"
  example_file: "example.md"
  description: "Test quill with template"
"#;
    fs::write(quill_dir.join("Quill.yaml"), yaml_content).unwrap();
    fs::write(quill_dir.join("plate.typ"), "plate content").unwrap();
    fs::write(
        quill_dir.join("example.md"),
        "---\ntitle: Test\n---\n\nThis is a test template.",
    )
    .unwrap();

    // Load quill
    let quill = Quill::from_path(quill_dir).unwrap();

    // Test that example content is loaded and includes some the text
    assert!(quill.example.is_some());
    let example = quill.example.unwrap();
    assert!(example.contains("title: Test"));
    assert!(example.contains("This is a test template"));

    // Test that plate template is still loaded
    assert_eq!(quill.plate.unwrap(), "plate content");
}

#[test]
fn test_template_smart_default() {
    let temp_dir = TempDir::new().unwrap();
    let quill_dir = temp_dir.path();

    // Create test files without example specified
    let yaml_content = r#"Quill:
  name: "test-smart-default"
  version: "1.0"
  backend: "typst"
  plate_file: "plate.typ"
  description: "Test quill with smart default"
"#;
    fs::write(quill_dir.join("Quill.yaml"), yaml_content).unwrap();
    fs::write(quill_dir.join("plate.typ"), "plate content").unwrap();
    // Create example.md which should be picked up automatically
    fs::write(
        quill_dir.join("example.md"),
        "---\ntitle: Smart Default\n---\n\nPicked up automatically.",
    )
    .unwrap();

    // Load quill
    let quill = Quill::from_path(quill_dir).unwrap();

    // Test that example content is loaded
    assert!(quill.example.is_some());
    let example = quill.example.unwrap();
    assert!(example.contains("title: Smart Default"));
    assert!(example.contains("Picked up automatically"));
}

#[test]
fn test_template_optional() {
    let temp_dir = TempDir::new().unwrap();
    let quill_dir = temp_dir.path();

    // Create test files without example specified
    let yaml_content = r#"Quill:
  name: "test-without-template"
  version: "1.0"
  backend: "typst"
  plate_file: "plate.typ"
  description: "Test quill without template"
"#;
    fs::write(quill_dir.join("Quill.yaml"), yaml_content).unwrap();
    fs::write(quill_dir.join("plate.typ"), "plate content").unwrap();

    // Load quill
    let quill = Quill::from_path(quill_dir).unwrap();

    // Test that example fields are None
    assert_eq!(quill.example, None);

    // Test that plate template is still loaded
    assert_eq!(quill.plate.unwrap(), "plate content");
}

#[test]
fn test_from_tree() {
    // Create a simple in-memory file tree
    let mut root_files = HashMap::new();

    // Add Quill.yaml
    let quill_yaml = r#"Quill:
  name: "test-from-tree"
  version: "1.0"
  backend: "typst"
  plate_file: "plate.typ"
  description: "A test quill from tree"
"#;
    root_files.insert(
        "Quill.yaml".to_string(),
        FileTreeNode::File {
            contents: quill_yaml.as_bytes().to_vec(),
        },
    );

    // Add plate file
    let plate_content = "= Test Template\n\nThis is a test.";
    root_files.insert(
        "plate.typ".to_string(),
        FileTreeNode::File {
            contents: plate_content.as_bytes().to_vec(),
        },
    );

    let root = FileTreeNode::Directory { files: root_files };

    // Create Quill from tree
    let quill = Quill::from_tree(root).unwrap();

    // Validate the quill
    assert_eq!(quill.name, "test-from-tree");
    assert_eq!(quill.plate.unwrap(), plate_content);
    assert!(quill.metadata.contains_key("backend"));
    assert!(quill.metadata.contains_key("description"));
}

#[test]
fn test_from_tree_with_template() {
    let mut root_files = HashMap::new();

    // Add Quill.yaml with example specified
    // Add Quill.yaml with example specified
    let quill_yaml = r#"
Quill:
  name: test-tree-template
  version: "1.0"
  backend: typst
  plate_file: plate.typ
  example_file: template.md
  description: Test tree with template
"#;
    root_files.insert(
        "Quill.yaml".to_string(),
        FileTreeNode::File {
            contents: quill_yaml.as_bytes().to_vec(),
        },
    );

    // Add plate file
    root_files.insert(
        "plate.typ".to_string(),
        FileTreeNode::File {
            contents: b"plate content".to_vec(),
        },
    );

    // Add template file
    let template_content = "# {{ title }}\n\n{{ body }}";
    root_files.insert(
        "template.md".to_string(),
        FileTreeNode::File {
            contents: template_content.as_bytes().to_vec(),
        },
    );

    let root = FileTreeNode::Directory { files: root_files };

    // Create Quill from tree
    let quill = Quill::from_tree(root).unwrap();

    // Validate template is loaded
    assert_eq!(quill.example, Some(template_content.to_string()));
}

#[test]
fn test_from_json() {
    // Create JSON representation of a Quill using new format
    let json_str = r#"{
            "metadata": {
                "name": "test_from_json"
            },
            "files": {
                "Quill.yaml": {
                    "contents": "Quill:\n  name: test_from_json\n  version: \"1.0\"\n  backend: typst\n  plate_file: plate.typ\n  description: Test quill from JSON\n"
                },
                "plate.typ": {
                    "contents": "= Test Plate\n\nThis is test content."
                }
            }
        }"#;

    // Create Quill from JSON
    let quill = Quill::from_json(json_str).unwrap();

    // Validate the quill
    assert_eq!(quill.name, "test_from_json");
    assert!(quill.plate.unwrap().contains("Test Plate"));
    assert!(quill.metadata.contains_key("backend"));
}

#[test]
fn test_from_json_with_byte_array() {
    // Create JSON with byte array representation using new format
    let json_str = r#"{
            "files": {
                "Quill.yaml": {
                    "contents": "Quill:\n  name: test\n  version: \"1.0\"\n  backend: typst\n  plate_file: plate.typ\n  description: Test quill\n"
                },
                "plate.typ": {
                    "contents": "test plate"
                }
            }
        }"#;

    // Create Quill from JSON
    let quill = Quill::from_json(json_str).unwrap();

    // Validate the quill was created
    assert_eq!(quill.name, "test");
    assert_eq!(quill.plate.unwrap(), "test plate");
}

#[test]
fn test_from_json_missing_files() {
    // JSON without files field should fail
    let json_str = r#"{
            "metadata": {
                "name": "test"
            }
        }"#;

    let result = Quill::from_json(json_str);
    assert!(result.is_err());
    // Should fail because there's no 'files' key
    assert!(result.unwrap_err().to_string().contains("files"));
}

#[test]
fn test_from_json_tree_structure() {
    // Test the new tree structure format
    let json_str = r#"{
            "files": {
                "Quill.yaml": {
                    "contents": "Quill:\n  name: test_tree_json\n  version: \"1.0\"\n  backend: typst\n  plate_file: plate.typ\n  description: Test tree JSON\n"
                },
                "plate.typ": {
                    "contents": "= Test Plate\n\nTree structure content."
                }
            }
        }"#;

    let quill = Quill::from_json(json_str).unwrap();

    assert_eq!(quill.name, "test_tree_json");
    assert!(quill.plate.unwrap().contains("Tree structure content"));
    assert!(quill.metadata.contains_key("backend"));
}

#[test]
fn test_from_json_nested_tree_structure() {
    // Test nested directories in tree structure
    let json_str = r#"{
            "files": {
                "Quill.yaml": {
                    "contents": "Quill:\n  name: nested_test\n  version: \"1.0\"\n  backend: typst\n  plate_file: plate.typ\n  description: Nested test\n"
                },
                "plate.typ": {
                    "contents": "plate"
                },
                "src": {
                    "main.rs": {
                        "contents": "fn main() {}"
                    },
                    "lib.rs": {
                        "contents": "// lib"
                    }
                }
            }
        }"#;

    let quill = Quill::from_json(json_str).unwrap();

    assert_eq!(quill.name, "nested_test");
    // Verify nested files are accessible
    assert!(quill.file_exists("src/main.rs"));
    assert!(quill.file_exists("src/lib.rs"));

    let main_rs = quill.get_file("src/main.rs").unwrap();
    assert_eq!(main_rs, b"fn main() {}");
}

#[test]
fn test_from_tree_structure_direct() {
    // Test using from_tree_structure directly
    let mut root_files = HashMap::new();

    root_files.insert(
            "Quill.yaml".to_string(),
            FileTreeNode::File {
                contents:
                    b"Quill:\n  name: direct_tree\n  version: \"1.0\"\n  backend: typst\n  plate_file: plate.typ\n  description: Direct tree test\n"
                        .to_vec(),
            },
        );

    root_files.insert(
        "plate.typ".to_string(),
        FileTreeNode::File {
            contents: b"plate content".to_vec(),
        },
    );

    // Add a nested directory
    let mut src_files = HashMap::new();
    src_files.insert(
        "main.rs".to_string(),
        FileTreeNode::File {
            contents: b"fn main() {}".to_vec(),
        },
    );

    root_files.insert(
        "src".to_string(),
        FileTreeNode::Directory { files: src_files },
    );

    let root = FileTreeNode::Directory { files: root_files };

    let quill = Quill::from_tree(root).unwrap();

    assert_eq!(quill.name, "direct_tree");
    assert!(quill.file_exists("src/main.rs"));
    assert!(quill.file_exists("plate.typ"));
}

#[test]
fn test_from_json_with_metadata_override() {
    // Test that metadata key overrides name from Quill.yaml
    let json_str = r#"{
            "metadata": {
                "name": "override_name"
            },
            "files": {
                "Quill.yaml": {
                    "contents": "Quill:\n  name: toml_name\n  version: \"1.0\"\n  backend: typst\n  plate_file: plate.typ\n  description: TOML name test\n"
                },
                "plate.typ": {
                    "contents": "= plate"
                }
            }
        }"#;

    let quill = Quill::from_json(json_str).unwrap();
    // Metadata name should be used as default, but Quill.yaml takes precedence
    // when from_tree is called
    assert_eq!(quill.name, "toml_name");
}

#[test]
fn test_from_json_empty_directory() {
    // Test that empty directories are supported
    let json_str = r#"{
            "files": {
                "Quill.yaml": {
                    "contents": "Quill:\n  name: empty_dir_test\n  version: \"1.0\"\n  backend: typst\n  plate_file: plate.typ\n  description: Empty directory test\n"
                },
                "plate.typ": {
                    "contents": "plate"
                },
                "empty_dir": {}
            }
        }"#;

    let quill = Quill::from_json(json_str).unwrap();
    assert_eq!(quill.name, "empty_dir_test");
    assert!(quill.dir_exists("empty_dir"));
    assert!(!quill.file_exists("empty_dir"));
}

#[test]
fn test_dir_exists_and_list_apis() {
    let mut root_files = HashMap::new();

    // Add Quill.yaml
    root_files.insert(
            "Quill.yaml".to_string(),
            FileTreeNode::File {
                contents: b"Quill:\n  name: test\n  version: \"1.0\"\n  backend: typst\n  plate_file: plate.typ\n  description: Test quill\n"
                    .to_vec(),
            },
        );

    // Add plate file
    root_files.insert(
        "plate.typ".to_string(),
        FileTreeNode::File {
            contents: b"plate content".to_vec(),
        },
    );

    // Add assets directory with files
    let mut assets_files = HashMap::new();
    assets_files.insert(
        "logo.png".to_string(),
        FileTreeNode::File {
            contents: vec![137, 80, 78, 71],
        },
    );
    assets_files.insert(
        "icon.svg".to_string(),
        FileTreeNode::File {
            contents: b"<svg></svg>".to_vec(),
        },
    );

    // Add subdirectory in assets
    let mut fonts_files = HashMap::new();
    fonts_files.insert(
        "font.ttf".to_string(),
        FileTreeNode::File {
            contents: b"font data".to_vec(),
        },
    );
    assets_files.insert(
        "fonts".to_string(),
        FileTreeNode::Directory { files: fonts_files },
    );

    root_files.insert(
        "assets".to_string(),
        FileTreeNode::Directory {
            files: assets_files,
        },
    );

    // Add empty directory
    root_files.insert(
        "empty".to_string(),
        FileTreeNode::Directory {
            files: HashMap::new(),
        },
    );

    let root = FileTreeNode::Directory { files: root_files };
    let quill = Quill::from_tree(root).unwrap();

    // Test dir_exists
    assert!(quill.dir_exists("assets"));
    assert!(quill.dir_exists("assets/fonts"));
    assert!(quill.dir_exists("empty"));
    assert!(!quill.dir_exists("nonexistent"));
    assert!(!quill.dir_exists("plate.typ")); // file, not directory

    // Test file_exists
    assert!(quill.file_exists("plate.typ"));
    assert!(quill.file_exists("assets/logo.png"));
    assert!(quill.file_exists("assets/fonts/font.ttf"));
    assert!(!quill.file_exists("assets")); // directory, not file

    // Test list_files
    let root_files_list = quill.list_files("");
    assert_eq!(root_files_list.len(), 2); // Quill.yaml and plate.typ
    assert!(root_files_list.contains(&"Quill.yaml".to_string()));
    assert!(root_files_list.contains(&"plate.typ".to_string()));

    let assets_files_list = quill.list_files("assets");
    assert_eq!(assets_files_list.len(), 2); // logo.png and icon.svg
    assert!(assets_files_list.contains(&"logo.png".to_string()));
    assert!(assets_files_list.contains(&"icon.svg".to_string()));

    // Test list_subdirectories
    let root_subdirs = quill.list_subdirectories("");
    assert_eq!(root_subdirs.len(), 2); // assets and empty
    assert!(root_subdirs.contains(&"assets".to_string()));
    assert!(root_subdirs.contains(&"empty".to_string()));

    let assets_subdirs = quill.list_subdirectories("assets");
    assert_eq!(assets_subdirs.len(), 1); // fonts
    assert!(assets_subdirs.contains(&"fonts".to_string()));

    let empty_subdirs = quill.list_subdirectories("empty");
    assert_eq!(empty_subdirs.len(), 0);
}

#[test]
fn test_field_schemas_parsing() {
    let mut root_files = HashMap::new();

    // Add Quill.yaml with field schemas
    let quill_yaml = r#"Quill:
  name: "taro"
  version: "1.0"
  backend: "typst"
  plate_file: "plate.typ"
  example_file: "taro.md"
  description: "Test template for field schemas"

fields:
  author:
    type: "string"
    description: "Author of document"
  ice_cream:
    type: "string"
    description: "favorite ice cream flavor"
  title:
    type: "string"
    description: "title of document"
"#;
    root_files.insert(
        "Quill.yaml".to_string(),
        FileTreeNode::File {
            contents: quill_yaml.as_bytes().to_vec(),
        },
    );

    // Add plate file
    let plate_content = "= Test Template\n\nThis is a test.";
    root_files.insert(
        "plate.typ".to_string(),
        FileTreeNode::File {
            contents: plate_content.as_bytes().to_vec(),
        },
    );

    // Add template file
    root_files.insert(
        "taro.md".to_string(),
        FileTreeNode::File {
            contents: b"# Template".to_vec(),
        },
    );

    let root = FileTreeNode::Directory { files: root_files };

    // Create Quill from tree
    let quill = Quill::from_tree(root).unwrap();

    // Validate field schemas were parsed (author, ice_cream, title, BODY)
    assert_eq!(quill.schema["properties"].as_object().unwrap().len(), 4);
    assert!(quill.schema["properties"]
        .as_object()
        .unwrap()
        .contains_key("author"));
    assert!(quill.schema["properties"]
        .as_object()
        .unwrap()
        .contains_key("ice_cream"));
    assert!(quill.schema["properties"]
        .as_object()
        .unwrap()
        .contains_key("title"));
    assert!(quill.schema["properties"]
        .as_object()
        .unwrap()
        .contains_key("BODY"));

    // Verify author field schema
    let author_schema = quill.schema["properties"]["author"].as_object().unwrap();
    assert_eq!(author_schema["description"], "Author of document");

    // Verify ice_cream field schema (no required field, should default to false)
    let ice_cream_schema = quill.schema["properties"]["ice_cream"].as_object().unwrap();
    assert_eq!(ice_cream_schema["description"], "favorite ice cream flavor");

    // Verify title field schema
    let title_schema = quill.schema["properties"]["title"].as_object().unwrap();
    assert_eq!(title_schema["description"], "title of document");
}

#[test]
fn test_field_schema_struct() {
    // Test creating FieldSchema with minimal fields
    let schema1 = FieldSchema::new(
        "test_name".to_string(),
        FieldType::String,
        Some("Test description".to_string()),
    );
    assert_eq!(schema1.description, Some("Test description".to_string()));
    assert_eq!(schema1.r#type, FieldType::String);
    assert_eq!(schema1.examples, None);
    assert_eq!(schema1.default, None);

    // Test parsing FieldSchema from YAML with all fields
    let yaml_str = r#"
description: "Full field schema"
type: "string"
examples:
  - "Example value"
default: "Default value"
"#;
    let quill_value = QuillValue::from_yaml_str(yaml_str).unwrap();
    let schema2 = FieldSchema::from_quill_value("test_name".to_string(), &quill_value).unwrap();
    assert_eq!(schema2.name, "test_name");
    assert_eq!(schema2.description, Some("Full field schema".to_string()));
    assert_eq!(schema2.r#type, FieldType::String);
    assert_eq!(
        schema2
            .examples
            .as_ref()
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str()),
        Some("Example value")
    );
    assert_eq!(
        schema2.default.as_ref().and_then(|v| v.as_str()),
        Some("Default value")
    );
}

#[test]
fn test_field_schema_ui_compact() {
    let yaml_str = r#"
type: "string"
description: "A compact field"
ui:
  compact: true
"#;
    let quill_value = QuillValue::from_yaml_str(yaml_str).unwrap();
    let schema = FieldSchema::from_quill_value("compact_field".to_string(), &quill_value).unwrap();
    assert_eq!(schema.ui.as_ref().unwrap().compact, Some(true));
}

#[test]
fn test_quill_without_plate_file() {
    // Test creating a Quill without specifying a plate file
    let mut root_files = HashMap::new();

    // Add Quill.yaml without plate field
    let quill_yaml = r#"Quill:
  name: "test-no-plate"
  version: "1.0"
  backend: "typst"
  description: "Test quill without plate file"
"#;
    root_files.insert(
        "Quill.yaml".to_string(),
        FileTreeNode::File {
            contents: quill_yaml.as_bytes().to_vec(),
        },
    );

    let root = FileTreeNode::Directory { files: root_files };

    // Create Quill from tree
    let quill = Quill::from_tree(root).unwrap();

    // Validate that plate is null (will use auto plate)
    assert!(quill.plate.clone().is_none());
    assert_eq!(quill.name, "test-no-plate");
}

#[test]
fn test_quill_config_from_yaml() {
    // Test parsing QuillConfig from YAML content
    let yaml_content = r#"
Quill:
  name: test_config
  version: "1.0"
  backend: typst
  description: Test configuration parsing
  author: Test Author
  plate_file: plate.typ
  example_file: example.md

typst:
  packages: 
    - "@preview/bubble:0.2.2"

fields:
  title:
    description: Document title
    type: string
  author:
    type: string
    description: Document author
"#;

    let config = QuillConfig::from_yaml(yaml_content).unwrap();

    // Verify required fields
    assert_eq!(config.name, "test_config");
    assert_eq!(config.main().name, "main");
    assert_eq!(config.backend, "typst");
    assert_eq!(
        config.main().description,
        Some("Test configuration parsing".to_string())
    );

    // Verify optional fields
    assert_eq!(config.version, "1.0");
    assert_eq!(config.author, "Test Author");
    assert_eq!(config.plate_file, Some("plate.typ".to_string()));
    assert_eq!(config.example_file, Some("example.md".to_string()));

    // Verify typst config
    assert!(config.typst_config.contains_key("packages"));

    // Verify field schemas
    assert_eq!(config.main().fields.len(), 2);
    assert!(config.main().fields.contains_key("title"));
    assert!(config.main().fields.contains_key("author"));

    let title_field = &config.main().fields["title"];
    assert_eq!(title_field.description, Some("Document title".to_string()));
    assert_eq!(title_field.r#type, FieldType::String);
}

#[test]
fn test_quill_config_missing_required_fields() {
    // Test that missing required fields result in error
    let yaml_missing_name = r#"
Quill:
  backend: typst
  description: Missing name
"#;
    let result = QuillConfig::from_yaml(yaml_missing_name);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing required 'name'"));

    let yaml_missing_backend = r#"
Quill:
  name: test
  description: Missing backend
"#;
    let result = QuillConfig::from_yaml(yaml_missing_backend);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing required 'backend'"));

    let yaml_missing_description = r#"
Quill:
  name: test
  version: "1.0"
  backend: typst
"#;
    let result = QuillConfig::from_yaml(yaml_missing_description);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing required 'description'"));
}

#[test]
fn test_quill_config_empty_description() {
    // Test that empty description results in error
    let yaml_empty_description = r#"
Quill:
  name: test
  version: "1.0"
  backend: typst
  description: "   "
"#;
    let result = QuillConfig::from_yaml(yaml_empty_description);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("description' field in 'Quill' section cannot be empty"));
}

#[test]
fn test_quill_config_missing_quill_section() {
    // Test that missing [Quill] section results in error
    let yaml_no_section = r#"
fields:
  title:
    description: Title
"#;
    let result = QuillConfig::from_yaml(yaml_no_section);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing required 'Quill' section"));
}

#[test]
fn test_quill_from_config_metadata() {
    // Test that QuillConfig metadata flows through to Quill
    let mut root_files = HashMap::new();

    let quill_yaml = r#"
Quill:
  name: metadata-test
  version: "1.0"
  backend: typst
  description: Test metadata flow
  author: Test Author
  custom_field: custom_value

typst:
  packages: 
    - "@preview/bubble:0.2.2"
"#;
    root_files.insert(
        "Quill.yaml".to_string(),
        FileTreeNode::File {
            contents: quill_yaml.as_bytes().to_vec(),
        },
    );

    let root = FileTreeNode::Directory { files: root_files };
    let quill = Quill::from_tree(root).unwrap();

    // Verify metadata includes backend and description
    assert!(quill.metadata.contains_key("backend"));
    assert!(quill.metadata.contains_key("description"));
    assert!(quill.metadata.contains_key("author"));

    // Verify custom field is in metadata
    assert!(quill.metadata.contains_key("custom_field"));
    assert_eq!(
        quill.metadata.get("custom_field").unwrap().as_str(),
        Some("custom_value")
    );

    // Verify typst config with typst_ prefix
    assert!(quill.metadata.contains_key("typst_packages"));
}

#[test]
fn test_extract_defaults_method() {
    // Test the extract_defaults method on Quill
    let mut root_files = HashMap::new();

    let quill_yaml = r#"
Quill:
  name: metadata-test-yaml
  version: "1.0"
  backend: typst
  description: Test metadata flow
  author: Test Author
  custom_field: custom_value

typst:
  packages: 
    - "@preview/bubble:0.2.2"

fields:
  author:
    type: string
    default: Anonymous
  status:
    type: string
    default: draft
  title:
    type: string
"#;
    root_files.insert(
        "Quill.yaml".to_string(),
        FileTreeNode::File {
            contents: quill_yaml.as_bytes().to_vec(),
        },
    );

    let root = FileTreeNode::Directory { files: root_files };
    let quill = Quill::from_tree(root).unwrap();

    // Extract defaults
    let defaults = quill.extract_defaults();

    // Verify only fields with defaults are returned
    assert_eq!(defaults.len(), 2);
    assert!(!defaults.contains_key("title")); // no default
    assert!(defaults.contains_key("author"));
    assert!(defaults.contains_key("status"));

    // Verify default values
    assert_eq!(defaults.get("author").unwrap().as_str(), Some("Anonymous"));
    assert_eq!(defaults.get("status").unwrap().as_str(), Some("draft"));
}

#[test]
fn test_field_order_preservation() {
    let yaml_content = r#"
Quill:
  name: order-test
  version: "1.0"
  backend: typst
  description: Test field order

fields:
  first:
    type: string
    description: First field
  second:
    type: string
    description: Second field
  third:
    type: string
    description: Third field
    ui:
      group: Test Group
  fourth:
    type: string
    description: Fourth field
"#;

    let config = QuillConfig::from_yaml(yaml_content).unwrap();

    // Check that fields have correct order based on TOML position
    // Order is automatically generated based on field position

    let first = config.main().fields.get("first").unwrap();
    assert_eq!(first.ui.as_ref().unwrap().order, Some(0));

    let second = config.main().fields.get("second").unwrap();
    assert_eq!(second.ui.as_ref().unwrap().order, Some(1));

    let third = config.main().fields.get("third").unwrap();
    assert_eq!(third.ui.as_ref().unwrap().order, Some(2));
    assert_eq!(
        third.ui.as_ref().unwrap().group,
        Some("Test Group".to_string())
    );

    let fourth = config.main().fields.get("fourth").unwrap();
    assert_eq!(fourth.ui.as_ref().unwrap().order, Some(3));
}

#[test]
fn test_quill_with_all_ui_properties() {
    let yaml_content = r#"
Quill:
  name: full-ui-test
  version: "1.0"
  backend: typst
  description: Test all UI properties

fields:
  author:
    description: The full name of the document author
    type: str
    ui:
      group: Author Info
"#;

    let config = QuillConfig::from_yaml(yaml_content).unwrap();

    let author_field = &config.main().fields["author"];
    let ui = author_field.ui.as_ref().unwrap();
    assert_eq!(ui.group, Some("Author Info".to_string()));
    assert_eq!(ui.order, Some(0)); // First field should have order 0
}
#[test]
fn test_field_schema_with_title_and_description() {
    // Test parsing field with new schema format (title + description, no tooltip)
    let yaml = r#"
title: "Field Title"
description: "Detailed field description"
type: "string"
examples:
  - "Example value"
ui:
  group: "Test Group"
"#;
    let quill_value = QuillValue::from_yaml_str(yaml).unwrap();
    let schema = FieldSchema::from_quill_value("test_field".to_string(), &quill_value).unwrap();

    assert_eq!(schema.title, Some("Field Title".to_string()));
    assert_eq!(
        schema.description,
        Some("Detailed field description".to_string())
    );

    assert_eq!(
        schema
            .examples
            .as_ref()
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str()),
        Some("Example value")
    );

    let ui = schema.ui.as_ref().unwrap();
    assert_eq!(ui.group, Some("Test Group".to_string()));
}

#[test]
fn test_parse_card_field_type() {
    // Test that FieldSchema no longer supports type = "card" (cards are in CardSchema now)
    let yaml = r#"
type: "string"
title: "Simple Field"
description: "A simple string field"
"#;
    let quill_value = QuillValue::from_yaml_str(yaml).unwrap();
    let schema = FieldSchema::from_quill_value("simple_field".to_string(), &quill_value).unwrap();

    assert_eq!(schema.name, "simple_field");
    assert_eq!(schema.r#type, FieldType::String);
    assert_eq!(schema.title, Some("Simple Field".to_string()));
    assert_eq!(
        schema.description,
        Some("A simple string field".to_string())
    );
}

#[test]
fn test_parse_card_with_fields_in_yaml() {
    // Test parsing [cards] section with [cards.X.fields.Y] syntax
    let yaml_content = r#"
Quill:
  name: cards-fields-test
  version: "1.0"
  backend: typst
  description: Test [cards.X.fields.Y] syntax

cards:
  endorsements:
    title: Endorsements
    description: Chain of endorsements
    fields:
      name:
        type: string
        title: Endorser Name
        description: Name of the endorsing official
        required: true
      org:
        type: string
        title: Organization
        description: Endorser's organization
        default: Unknown
"#;

    let config = QuillConfig::from_yaml(yaml_content).unwrap();

    // Verify the card was parsed into config.cards
    assert!(config.card_definition("endorsements").is_some());
    let card = config.card_definition("endorsements").unwrap();

    assert_eq!(card.name, "endorsements");
    assert_eq!(card.title, Some("Endorsements".to_string()));
    assert_eq!(card.description, Some("Chain of endorsements".to_string()));

    // Verify card fields
    assert_eq!(card.fields.len(), 2);

    let name_field = card.fields.get("name").unwrap();
    assert_eq!(name_field.r#type, FieldType::String);
    assert_eq!(name_field.title, Some("Endorser Name".to_string()));
    assert!(name_field.required);

    let org_field = card.fields.get("org").unwrap();
    assert_eq!(org_field.r#type, FieldType::String);
    assert!(org_field.default.is_some());
    assert_eq!(
        org_field.default.as_ref().unwrap().as_str(),
        Some("Unknown")
    );
}

#[test]
fn test_field_schema_rejects_unknown_keys() {
    // Test that unknown keys like "invalid_key" are rejected (strict mode)
    let yaml = r#"
type: "string"
description: "A string field"
invalid_key:
  sub_field:
    type: "string"
    description: "Nested field"
"#;
    let quill_value = QuillValue::from_yaml_str(yaml).unwrap();

    let result = FieldSchema::from_quill_value("author".to_string(), &quill_value);

    // The parsing should fail due to deny_unknown_fields
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("unknown field `invalid_key`"),
        "Error was: {}",
        err
    );
}

#[test]
fn test_quill_config_with_cards_section() {
    let yaml_content = r#"
Quill:
  name: cards-test
  version: "1.0"
  backend: typst
  description: Test [cards] section

fields:
  regular:
    description: Regular field
    type: string

cards:
  indorsements:
    title: Routing Indorsements
    description: Chain of endorsements
    fields:
      name:
        title: Name
        type: string
        description: Name field
"#;

    let config = QuillConfig::from_yaml(yaml_content).unwrap();

    // Check regular field
    assert!(config.main().fields.contains_key("regular"));
    let regular = config.main().fields.get("regular").unwrap();
    assert_eq!(regular.r#type, FieldType::String);

    // Check card is in config.cards (not config.main().fields)
    assert!(config.card_definition("indorsements").is_some());
    let card = config.card_definition("indorsements").unwrap();
    assert_eq!(card.title, Some("Routing Indorsements".to_string()));
    assert_eq!(card.description, Some("Chain of endorsements".to_string()));
    assert!(card.fields.contains_key("name"));
}

#[test]
fn test_quill_config_cards_empty_fields() {
    // Test that cards with no fields section are valid
    let yaml_content = r#"
Quill:
  name: cards-empty-fields-test
  version: "1.0"
  backend: typst
  description: Test cards without fields

cards:
  myscope:
    description: My scope
"#;

    let config = QuillConfig::from_yaml(yaml_content).unwrap();
    let card = config.card_definition("myscope").unwrap();
    assert_eq!(card.name, "myscope");
    assert_eq!(card.description, Some("My scope".to_string()));
    assert!(card.fields.is_empty());
}

#[test]
fn test_quill_config_allows_card_collision() {
    // Test that scope name colliding with field name is ALLOWED
    let yaml_content = r#"
Quill:
  name: collision-test
  version: "1.0"
  backend: typst
  description: Test collision

fields:
  conflict:
    description: Field
    type: string

cards:
  conflict:
    description: Card
"#;

    let result = QuillConfig::from_yaml(yaml_content);
    if let Err(e) = &result {
        panic!(
            "Card name collision should be allowed, but got error: {}",
            e
        );
    }
    assert!(result.is_ok());

    let config = result.unwrap();
    assert!(config.main().fields.contains_key("conflict"));
    assert!(config.card_definition("conflict").is_some());
}

#[test]
fn test_quill_config_ordering_with_cards() {
    // Test that fields have proper UI ordering (cards no longer have card-level ordering)
    let yaml_content = r#"
Quill:
  name: ordering-test
  version: "1.0"
  backend: typst
  description: Test ordering

fields:
  first:
    type: string
    description: First
  zero:
    type: string
    description: Zero

cards:
  second:
    description: Second
    fields:
      card_field:
        type: string
        description: A card field
"#;

    let config = QuillConfig::from_yaml(yaml_content).unwrap();

    let first = config.main().fields.get("first").unwrap();
    let zero = config.main().fields.get("zero").unwrap();
    let second = config.card_definition("second").unwrap();

    // Check field ordering
    let ord_first = first.ui.as_ref().unwrap().order.unwrap();
    let ord_zero = zero.ui.as_ref().unwrap().order.unwrap();

    // Within fields, "first" is before "zero"
    assert!(ord_first < ord_zero);
    assert_eq!(ord_first, 0);
    assert_eq!(ord_zero, 1);

    // Card fields should also have ordering
    let card_field = second.fields.get("card_field").unwrap();
    let ord_card_field = card_field.ui.as_ref().unwrap().order.unwrap();
    assert_eq!(ord_card_field, 0); // First (and only) field in this card
}
#[test]
fn test_card_field_order_preservation() {
    // Test that card fields preserve definition order (not alphabetical)
    // defined: z_first, then a_second
    // alphabetical: a_second, then z_first
    let yaml_content = r#"
Quill:
  name: card-order-test
  version: "1.0"
  backend: typst
  description: Test card field order

cards:
  mycard:
    description: Test card
    fields:
      z_first:
        type: string
        description: Defined first
      a_second:
        type: string
        description: Defined second
"#;

    let config = QuillConfig::from_yaml(yaml_content).unwrap();
    let card = config.card_definition("mycard").unwrap();

    let z_first = card.fields.get("z_first").unwrap();
    let a_second = card.fields.get("a_second").unwrap();

    // Check orders
    let z_order = z_first.ui.as_ref().unwrap().order.unwrap();
    let a_order = a_second.ui.as_ref().unwrap().order.unwrap();

    // If strict file order is preserved:
    // z_first should be 0, a_second should be 1
    assert_eq!(z_order, 0, "z_first should be 0 (defined first)");
    assert_eq!(a_order, 1, "a_second should be 1 (defined second)");
}
#[test]
fn test_nested_schema_parsing() {
    let yaml_content = r#"
Quill:
  name: nested-test
  version: "1.0"
  backend: typst
  description: Test nested elements

fields:
  my_list:
    type: array
    description: List of objects
    items:
      type: object
      properties:
        sub_a:
          type: string
          description: Subfield A
        sub_b:
          type: number
          description: Subfield B
  my_obj:
    type: object
    description: Single object
    properties:
      child:
        type: boolean
        description: Child field
"#;

    let config = QuillConfig::from_yaml(yaml_content).unwrap();

    // Check array with items
    let list_field = config.main().fields.get("my_list").unwrap();
    assert_eq!(list_field.r#type, FieldType::Array);
    assert!(list_field.items.is_some());

    let items_schema = list_field.items.as_ref().unwrap();
    assert_eq!(items_schema.r#type, FieldType::Object);
    assert!(items_schema.properties.is_some());

    let props = items_schema.properties.as_ref().unwrap();
    assert!(props.contains_key("sub_a"));
    assert!(props.contains_key("sub_b"));
    assert_eq!(props["sub_a"].r#type, FieldType::String);
    assert_eq!(props["sub_b"].r#type, FieldType::Number);

    // Check object with properties
    let obj_field = config.main().fields.get("my_obj").unwrap();
    assert_eq!(obj_field.r#type, FieldType::Object);
    assert!(obj_field.properties.is_some());

    let obj_props = obj_field.properties.as_ref().unwrap();
    assert!(obj_props.contains_key("child"));
    assert_eq!(obj_props["child"].r#type, FieldType::Boolean);
}

#[test]
fn test_quill_config_from_yaml_collects_non_fatal_field_warnings() {
    let yaml_content = r#"
Quill:
  name: warning-config
  version: "1.0"
  backend: typst
  description: Warning collection test

fields:
  valid_field:
    type: string
    description: Valid
  broken_field:
    description: Missing required type
"#;

    let (config, warnings) = QuillConfig::from_yaml_with_warnings(yaml_content).unwrap();

    assert!(config.main().fields.contains_key("valid_field"));
    assert!(!config.main().fields.contains_key("broken_field"));
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].severity, Severity::Warning);
    assert_eq!(
        warnings[0].code.as_deref(),
        Some("quill::field_parse_warning")
    );
    assert!(warnings[0]
        .message
        .contains("Failed to parse field schema 'broken_field'"));
}
