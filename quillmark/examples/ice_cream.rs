#[path = "../tests/common.rs"]
mod common;
use common::demo;

fn main() {
    demo(
        "taro.md",
        "taro",
        "taro.typ",
        "taro.pdf",
    )
    .expect("Demo failed");
}
