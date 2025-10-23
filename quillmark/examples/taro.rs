#[path = "../tests/common.rs"]
mod common;
use common::demo;

fn main() {
    demo("taro", "taro.typ", "taro.pdf", false).expect("Demo failed");
}
