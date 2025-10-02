#[path = "../tests/common.rs"]
mod common;
use common::demo;

fn main() {
    demo(
        "taro.md",
        "taro",
        Some(vec![
            "taro.png"
        ]),
        "taro.typ",
        "taro.pdf",
    )
    .expect("Demo failed");
}
