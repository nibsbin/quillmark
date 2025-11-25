#[path = "../tests/common.rs"]
mod common;
use common::demo;

fn main() {
    // Use the fixtures demo helper which centralizes file IO and printing.
    demo(
        "appreciated_letter",
        "appreciated_letter_glue.typ",
        "appreciated_letter_output.pdf",
        true,
    )
    .expect("Demo failed");
}
