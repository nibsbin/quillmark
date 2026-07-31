//! The quill schema family assembles from outside `quillmark-core`.
//!
//! `CardSchema::new` and `FieldSchema::new` exist so a caller can build a schema
//! in memory rather than through `Quill.yaml`. `#[non_exhaustive]` (0.99) forbids
//! the struct literal, so the optional halves each need a constructor of their
//! own or the entry points lead to slots nothing can fill. This crate is out of
//! `quillmark-core`, so the check is the compile.

use quillmark_core::quill::{
    BodyCardSchema, CardSchema, FieldSchema, FieldType, GroupRegistry, GroupSchema, UiCardSchema,
    UiFieldSchema,
};

#[test]
fn every_optional_half_of_a_card_schema_is_reachable() {
    let mut card = CardSchema::new("main".into(), Default::default());
    let mut title = FieldSchema::new("title".into(), FieldType::String, None);
    title.ui = Some(
        UiFieldSchema::default()
            .with_title("Title".into())
            .with_group("head".into())
            .with_multiline(true)
            .with_compact(false),
    );
    card.fields.insert("title".into(), title);
    card.ui = Some(
        UiCardSchema::default()
            .with_title("Main".into())
            .with_groups(GroupRegistry(vec![
                GroupSchema::new("head".into()).with_title("Heading".into())
            ])),
    );
    card.body = Some(
        BodyCardSchema::default()
            .with_enabled(true)
            .with_example("Write here.".into()),
    );

    let ui = card.ui.as_ref().unwrap();
    assert_eq!(ui.groups.as_ref().unwrap().0[0].id, "head");
    assert_eq!(card.fields["title"].ui.as_ref().unwrap().group.as_deref(), Some("head"));
}
