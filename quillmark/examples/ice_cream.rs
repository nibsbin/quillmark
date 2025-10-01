#[path = "../tests/common.rs"]
mod common;
use common::demo;

fn main() {
    demo(
        "ice_cream.md",
        "ice_cream",
        "ice_cream.typ",
        "ice_cream.pdf",
    )
    .expect("Demo failed");
}
