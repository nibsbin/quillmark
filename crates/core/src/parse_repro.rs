#[cfg(test)]
mod reproduction_test {
    use super::*;

    #[test]
    fn test_card_field_collision_repro() {
        let markdown = r#"---
my_card: "some global value"
---

---
CARD: my_card
title: "My Card"
---
Body
"#;
        // This should SUCCEED according to new PARSE.md, but FAIL according to current parse.rs
        let result = decompose(markdown);
        assert!(
            result.is_err(),
            "Current implementation should error on collision"
        );
    }
}
