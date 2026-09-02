//! `set_values` against `reader.values()`: the per-axis rule, the guard, and
//! the invariants the pair holds.

use indexmap::IndexMap;

use crate::document::EditError;
use crate::quill::quill_from_yaml;
use crate::quill::values::tests::{cards, fields, parse, values_of, DOC, QUILL};
use crate::quill::{CardValues, DocumentValues};
use crate::{Document, Quill};

fn bytes(doc: &Document) -> String {
    serde_json::to_string(doc).unwrap()
}

fn paths(errors: &[(crate::DocPath, EditError)]) -> Vec<String> {
    errors.iter().map(|(p, _)| p.to_string()).collect()
}

fn one(name: &str, value: serde_json::Value) -> IndexMap<String, serde_json::Value> {
    IndexMap::from([(name.to_string(), value)])
}

/// I1: the cycle writes nothing at all, on every document the bound door
/// admits.
#[track_caller]
fn assert_cycle_is_a_no_op(quill: &Quill, md: &str) {
    let mut doc = parse(quill, md);
    let before = bytes(&doc);
    let values = values_of(quill, &doc);
    quill
        .writer(&mut doc)
        .set_values(&values)
        .unwrap_or_else(|e| panic!("set_values refused its own read: {e:?}"));
    assert_eq!(bytes(&doc), before, "set_values(values()) moved bytes on:\n{md}");
}

#[test]
fn the_untouched_cycle_is_a_byte_no_op() {
    let quill = quill_from_yaml(QUILL);
    assert_cycle_is_a_no_op(&quill, DOC);
}

#[test]
fn the_cycle_preserves_what_a_re_import_cannot_reproduce() {
    let quill = quill_from_yaml(QUILL);
    for md in [
        // a must-fill marker
        "~~~card-yaml\n$quill: memo@1.0\n$kind: main\nsubject: !must_fill Draft\n~~~\n",
        // a YAML comment
        "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n# keep me\nsubject: Hi\n~~~\n",
        // an undeclared field
        "~~~card-yaml\n$quill: memo@1.0\n$kind: main\nstray: whatever\n~~~\n",
        // a scalar shorthand
        "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n\n\
         ~~~card-yaml\n$kind: line_item\nqty: \"3\"\n~~~\n",
        // a present-null
        "~~~card-yaml\n$quill: memo@1.0\n$kind: main\nsubject:\nqty:\n~~~\n",
        // an explicit empty $ext
        "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n$ext: {}\nsubject: Hi\n~~~\n",
        // a kindless card
        "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n\n~~~card-yaml\nfoo: bar\n~~~\n",
        // an unknown-kind card
        "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n\n\
         ~~~card-yaml\n$kind: mystery\nfoo: bar\n~~~\nStray.\n",
        // a dirty leaf
        "~~~card-yaml\n$quill: memo@1.0\n$kind: main\nparagraphs:\n  - not: content\n~~~\n",
        // a bare variant discriminant
        "~~~card-yaml\n$quill: memo@1.0\n$kind: main\nclassification: CUI\n~~~\n",
    ] {
        assert_cycle_is_a_no_op(&quill, md);
    }
}

/// I2: one write canonicalises, and the canonical form is a fixed point.
#[test]
fn a_write_canonicalises_once_then_holds_still() {
    let quill = quill_from_yaml(QUILL);
    let blank = || parse(&quill, "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n");
    let mut values = DocumentValues::new(one("subject", serde_json::json!("Widget __A__")));
    values
        .fields
        .as_mut()
        .unwrap()
        .insert("qty".into(), serde_json::json!("3"));

    let mut d1 = blank();
    quill.writer(&mut d1).set_values(&values).unwrap();
    let p = values_of(&quill, &d1);
    assert_eq!(
        fields(&p)["subject"],
        serde_json::json!("Widget **A**"),
        "the write canonicalises markdown"
    );
    assert_eq!(fields(&p)["qty"], serde_json::json!(3), "and the scalar");

    let mut d2 = blank();
    quill.writer(&mut d2).set_values(&p).unwrap();
    assert_eq!(bytes(&d1), bytes(&d2));
    assert_eq!(values_of(&quill, &d2), p, "the canonical form is a fixed point");
}

#[test]
fn an_absent_axis_is_untouched() {
    let quill = quill_from_yaml(QUILL);
    let mut doc = parse(&quill, DOC);
    let before = values_of(&quill, &doc);
    quill
        .writer(&mut doc)
        .set_values(&DocumentValues::new(one("subject", serde_json::json!("Bye"))))
        .unwrap();
    let after = values_of(&quill, &doc);
    assert_eq!(fields(&after)["subject"], serde_json::json!("Bye"));
    assert_eq!(after.body, before.body, "body untouched");
    assert_eq!(after.cards, before.cards, "cards untouched");
    assert_eq!(after.ext, before.ext, "$ext untouched");
    assert!(
        !fields(&after).contains_key("note"),
        "within the present `fields` axis, an unnamed declared field is removed"
    );

    let mut doc = parse(&quill, DOC);
    quill
        .writer(&mut doc)
        .set_values(&DocumentValues::default())
        .unwrap();
    assert_eq!(values_of(&quill, &doc), before, "the empty patch touches nothing");
}

#[test]
fn a_present_cards_axis_is_the_card_list() {
    let quill = quill_from_yaml(QUILL);
    let md = "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n\n\
              ~~~card-yaml\n$kind: line_item\nqty: 1\n~~~\n\n\
              ~~~card-yaml\n$kind: line_item\nqty: 2\n~~~\n";

    let mut doc = parse(&quill, md);
    let mut values = values_of(&quill, &doc);
    values.cards.as_mut().unwrap().truncate(1);
    quill.writer(&mut doc).set_values(&values).unwrap();
    assert_eq!(doc.cards().len(), 1, "cards past the list are removed");
    assert_eq!(
        quill.reader(&doc).card(0).unwrap().get("qty").unwrap(),
        Some(crate::QuillValue::from_json(serde_json::json!(1)))
    );

    let mut doc = parse(&quill, md);
    quill
        .writer(&mut doc)
        .set_values(&DocumentValues::default().with_cards(Vec::new()))
        .unwrap();
    assert_eq!(doc.cards().len(), 0, "an empty list empties the document");

    let mut doc = parse(&quill, md);
    let mut values = values_of(&quill, &doc);
    values.cards.as_mut().unwrap().push(
        CardValues::new("line_item", one("desc", serde_json::json!("Widget **A**")))
            .with_body("Note."),
    );
    quill.writer(&mut doc).set_values(&values).unwrap();
    assert_eq!(doc.cards().len(), 3, "an entry past the end appends");
    assert_eq!(cards(&values_of(&quill, &doc))[2].body.as_deref(), Some("Note."));
}

#[test]
fn a_card_entry_without_a_kind_keeps_the_cards_and_a_differing_kind_rebuilds() {
    let quill = quill_from_yaml(QUILL);
    let md = "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n\n\
              ~~~card-yaml\n$kind: line_item\n$ext:\n  app:\n    k: 1\nqty: 1\n~~~\nKept.\n";

    let mut doc = parse(&quill, md);
    let patch = DocumentValues::default().with_cards(vec![CardValues {
        fields: Some(one("qty", serde_json::json!(5))),
        ..CardValues::default()
    }]);
    quill.writer(&mut doc).set_values(&patch).unwrap();
    let patched = values_of(&quill, &doc);
    let card = &cards(&patched)[0];
    assert_eq!(card.kind, Some(Some("line_item".into())), "the kind is inherited");
    assert_eq!(card.fields.as_ref().unwrap()["qty"], serde_json::json!(5));
    assert_eq!(card.body.as_deref(), Some("Kept."), "absent axes on a patched card stay");
    assert!(card.ext.as_ref().unwrap().is_some());

    let mut doc = parse(&quill, md);
    let rebuild = DocumentValues::default().with_cards(vec![CardValues::new(
        "mystery",
        IndexMap::new(),
    )]);
    quill.writer(&mut doc).set_values(&rebuild).unwrap();
    let rebuilt = values_of(&quill, &doc);
    let card = &cards(&rebuilt)[0];
    assert_eq!(card.kind, Some(Some("mystery".into())));
    assert_eq!(card.body.as_deref(), Some(""), "a rebuilt card starts empty");
    assert_eq!(card.ext, Some(None));

    let mut doc = parse(&quill, md);
    let errors = quill
        .writer(&mut doc)
        .set_values(&DocumentValues::default().with_cards(vec![
            CardValues::default(),
            CardValues::default(),
        ]))
        .unwrap_err();
    assert_eq!(
        paths(&errors),
        ["cards[1]"],
        "a position with no card to inherit from is refused as a kindless build is"
    );
}

#[test]
fn a_kindless_card_is_kept_in_place_and_refused_as_a_build() {
    let quill = quill_from_yaml(QUILL);
    let md = "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n\n~~~card-yaml\nfoo: bar\n~~~\n";
    let mut doc = parse(&quill, md);
    let mut values = values_of(&quill, &doc);
    assert_eq!(cards(&values)[0].kind, Some(None));
    values.cards.as_mut().unwrap().push(CardValues {
        kind: Some(None),
        ..CardValues::default()
    });
    let errors = quill.writer(&mut doc).set_values(&values).unwrap_err();
    assert_eq!(paths(&errors), ["cards[1]"]);
    assert!(matches!(errors[0].1, EditError::InvalidKindName(_)));
    assert_eq!(doc.cards().len(), 1);
}

#[test]
fn an_undeclared_name_is_carried_and_immutable() {
    let quill = quill_from_yaml(QUILL);
    let md = "~~~card-yaml\n$quill: memo@1.0\n$kind: main\nsubject: Hi\nstray: whatever\n~~~\n";

    let mut doc = parse(&quill, md);
    let mut values = values_of(&quill, &doc);
    values
        .fields
        .as_mut()
        .unwrap()
        .insert("stray".into(), serde_json::json!("changed"));
    let errors = quill.writer(&mut doc).set_values(&values).unwrap_err();
    assert_eq!(paths(&errors), ["main.stray"]);
    assert!(matches!(errors[0].1, EditError::UnknownField { .. }));

    let mut doc = parse(&quill, md);
    let mut values = values_of(&quill, &doc);
    values.fields.as_mut().unwrap().shift_remove("stray");
    quill.writer(&mut doc).set_values(&values).unwrap();
    assert!(
        doc.main().payload().get("stray").is_some(),
        "unnamed, an undeclared field is left alone: it is outside the vocabulary, not absent from it"
    );

    let mut doc = parse(&quill, md);
    let errors = quill
        .writer(&mut doc)
        .set_values(&DocumentValues::new(one("nope", serde_json::json!("x"))))
        .unwrap_err();
    assert_eq!(paths(&errors), ["main.nope"]);
}

#[test]
fn every_refusal_arrives_at_once_and_nothing_is_written() {
    let quill = quill_from_yaml(QUILL);
    let mut doc = parse(&quill, "~~~card-yaml\n$quill: memo@1.0\n$kind: main\nsubject: Hi\n~~~\n");
    let before = bytes(&doc);
    let mut values = DocumentValues::new(one("nope", serde_json::json!("x")));
    values
        .fields
        .as_mut()
        .unwrap()
        .insert("alsonope".into(), serde_json::json!("y"));
    values.cards = Some(vec![CardValues::new(
        "line_item",
        one("bad", serde_json::json!(1)),
    )]);
    let errors = quill.writer(&mut doc).set_values(&values).unwrap_err();
    assert_eq!(
        paths(&errors),
        ["main.nope", "main.alsonope", "cards.line_item[0].bad"]
    );
    assert_eq!(bytes(&doc), before, "an all-or-nothing batch leaves the document untouched");
}

#[test]
fn ext_is_three_valued() {
    let quill = quill_from_yaml(QUILL);
    let md = "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n$ext:\n  app:\n    k: 1\n~~~\n";

    let mut doc = parse(&quill, md);
    quill
        .writer(&mut doc)
        .set_values(&DocumentValues::default().with_ext(None))
        .unwrap();
    assert!(doc.main().ext().is_none(), "null removes $ext");

    let mut doc = parse(&quill, md);
    quill
        .writer(&mut doc)
        .set_values(&DocumentValues::default().with_ext(Some(serde_json::Map::new())))
        .unwrap();
    assert_eq!(doc.main().ext(), Some(&serde_json::Map::new()), "{{}} is an explicit empty $ext");
    assert!(doc.to_markdown().contains("$ext: {}"));

    let mut doc = parse(&quill, md);
    let replacement = serde_json::json!({"other": true}).as_object().cloned().unwrap();
    quill
        .writer(&mut doc)
        .set_values(&DocumentValues::default().with_ext(Some(replacement.clone())))
        .unwrap();
    assert_eq!(doc.main().ext(), Some(&replacement), "a map replaces, never merges");
}

#[test]
fn body_is_replaced_from_markdown() {
    let quill = quill_from_yaml(QUILL);
    let mut doc = parse(&quill, DOC);
    quill
        .writer(&mut doc)
        .set_values(&DocumentValues::default().with_body("New __body__."))
        .unwrap();
    assert_eq!(doc.main().body_markdown(), "New **body**.");
    assert_eq!(
        fields(&values_of(&quill, &doc))["subject"],
        serde_json::json!("Hello **world**"),
        "the fields axis was absent and is untouched"
    );
}

#[test]
fn null_writes_a_present_null_and_omission_removes() {
    let quill = quill_from_yaml(QUILL);
    let mut doc = parse(&quill, "~~~card-yaml\n$quill: memo@1.0\n$kind: main\nsubject: Hi\nnote: n\n~~~\n");
    quill
        .writer(&mut doc)
        .set_values(&DocumentValues::new(one("subject", serde_json::Value::Null)))
        .unwrap();
    let v = values_of(&quill, &doc);
    assert_eq!(fields(&v).get("subject"), Some(&serde_json::Value::Null));
    assert!(!fields(&v).contains_key("note"));
}

#[test]
fn the_card_scope_is_the_document_scope_restricted_to_one_slot() {
    let quill = quill_from_yaml(QUILL);
    let mut doc = parse(&quill, DOC);
    let before = values_of(&quill, &doc);

    quill
        .writer(&mut doc)
        .card(0)
        .unwrap()
        .set_values(&CardValues {
            fields: Some(one("desc", serde_json::json!("Gadget"))),
            ..CardValues::default()
        })
        .unwrap();
    let after = values_of(&quill, &doc);
    let card = &cards(&after)[0];
    assert_eq!(card.fields.as_ref().unwrap()["desc"], serde_json::json!("Gadget"));
    assert!(
        !card.fields.as_ref().unwrap().contains_key("qty"),
        "within the present axis, the unnamed declared field is removed"
    );
    assert_eq!(card.body, before.cards.as_ref().unwrap()[0].body);
    assert_eq!(after.fields, before.fields, "the main card is out of scope");

    let mut doc = parse(&quill, DOC);
    let errors = quill
        .writer(&mut doc)
        .card(0)
        .unwrap()
        .set_values(&CardValues {
            fields: Some(one("bad", serde_json::json!(1))),
            ..CardValues::default()
        })
        .unwrap_err();
    assert_eq!(paths(&errors), ["cards.line_item[0].bad"]);

    let mut doc = parse(&quill, DOC);
    let before = bytes(&doc);
    let read = quill.reader(&doc).card(0).unwrap().values();
    quill.writer(&mut doc).card(0).unwrap().set_values(&read).unwrap();
    assert_eq!(bytes(&doc), before, "an unedited card read written back is a no-op");
}
