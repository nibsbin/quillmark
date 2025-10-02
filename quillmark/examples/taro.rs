#[path = "../tests/common.rs"]
mod common;
use common::demo;
use quillmark_fixtures::resource_path;

fn main() {
    demo(
        "taro.md",
        "taro",
        Some(vec![
            ("taro_ice_cream.png".to_string(), std::fs::read(resource_path("taro.png")).unwrap()),
        ]),
        "taro.typ",
        "taro.pdf",
    )
    .expect("Demo failed");
}
