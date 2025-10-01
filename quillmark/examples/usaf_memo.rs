#[path = "../tests/common.rs"]
mod common;
use common::demo;

fn main() {
    // Use the fixtures demo helper which centralizes file IO and printing.
    demo(
        "usaf_memo.md",
        "usaf_memo",
        "usaf_memo_glue.typ",
        "usaf_memo_output.pdf",
    )
    .expect("Demo failed");
}
