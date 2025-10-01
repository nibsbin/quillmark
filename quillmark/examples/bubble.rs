#[path = "../tests/common.rs"]
mod common;
use common::demo;

fn main() {
    demo(
        "bubble.md",
        "bubble",
        "bubble.typ",
        "bubble.pdf",
    )
    .expect("Demo failed");
}
