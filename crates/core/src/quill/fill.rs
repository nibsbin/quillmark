//! The per-field blank: the field's spelling of "explicitly nothing", and the
//! render floor, shared by blueprint emission and the blank-filled render.

use serde_json::json;

use super::{FieldSchema, FieldType};
use crate::value::QuillValue;

/// The **blank** for `field`: the leanest value satisfying its declared type,
/// and the value a reader recognizes as "nobody said anything".
///
/// | Type | Blank |
/// |---|---|
/// | `string`, `date`, `datetime` | `""` (a date's `""` lowers to Typst `none`) |
/// | `enum` | `""` — reserved, and never a member of `values:` |
/// | `richtext`, `plaintext` | the empty content |
/// | `array` | `[]` |
/// | `object` | every property at its own blank, recursively |
/// | `integer`, `number` | `0` |
/// | `boolean` | `false` |
///
/// A field's blank is a property of the *field*, not a member of the type's
/// value domain: an `enum`'s blank sits outside `values:` rather than claiming
/// a variant, so no document renders a choice nobody made. The loader rejects a
/// declared `""` (`quill::enum_blank_member`), but this function does not lean
/// on that: a [`QuillConfig`](super::QuillConfig) built through serde bypasses
/// loader validation, so the `enum` blank is `""` unconditionally.
///
/// `integer`, `number` and `boolean` are the seam: their blanks (`0`, `false`)
/// are indistinguishable at the plate from an authored `0` / `false`, and so is
/// any `object` or `array` over them, since their blank is the recursive one. A
/// wire `none` would be type-*absent* rather than type-*minimal*, which Typst
/// arithmetic and comparison reject — it would cost the totality the floor
/// exists to buy. An author needing to spell "unset" for a number models it as
/// an `enum`, which has a real blank.
///
/// An `object` with `properties` is shape-valid only when every property is
/// present, so it blanks (recursively) to an object with every property at its
/// own blank, not a bare `{}` (which only a property-less object degrades to).
pub fn blank(field: &FieldSchema) -> QuillValue {
    if field.enum_values.is_some() {
        return QuillValue::from_json(json!(""));
    }
    let json = match field.r#type {
        FieldType::Array => json!([]),
        FieldType::Object => match &field.properties {
            // Recurse so each property blanks to its own leaf: the result is a
            // shape-valid object, not a bare `{}`.
            Some(properties) => serde_json::Value::Object(
                properties
                    .iter()
                    .map(|(name, schema)| (name.clone(), blank(schema).into_json()))
                    .collect(),
            ),
            // A property-less object is schema-invalid; `{}` is its only
            // type-correct blank.
            None => json!({}),
        },
        FieldType::Integer | FieldType::Number => json!(0),
        FieldType::Boolean => json!(false),
        // A content field's blank is the empty content, not `""`: the seam carries
        // canonical Content-JSON, so the render floor must fill an absent
        // richtext or plaintext field with a content the backend can lower. The
        // empty content is single-`Para`, so it satisfies `inline` and is `plain`.
        FieldType::RichText { .. } | FieldType::PlainText { .. } => {
            quillmark_content::serial::to_canonical_value(&quillmark_content::Content::empty())
        }
        // String / Date / DateTime: `""` is the schema-valid blank for all three
        // (an empty string lowers to `none`, the absent-date sentinel).
        _ => json!(""),
    };
    QuillValue::from_json(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(yaml: &str) -> FieldSchema {
        let value = QuillValue::from_yaml_str(yaml).unwrap();
        FieldSchema::from_quill_value("field".to_string(), &value).unwrap()
    }

    #[test]
    fn object_with_properties_blanks_each_scalar_leaf() {
        let schema = field(
            r#"
type: object
properties:
  street: { type: string }
  zip: { type: integer }
  active: { type: boolean }
"#,
        );

        assert_eq!(
            blank(&schema).into_json(),
            json!({ "street": "", "zip": 0, "active": false })
        );
    }

    #[test]
    fn nested_object_recurses_to_blank_leaves() {
        let schema = field(
            r#"
type: object
properties:
  name: { type: string }
  address:
    type: object
    properties:
      city: { type: string }
      tags: { type: array, items: { type: string } }
"#,
        );

        assert_eq!(
            blank(&schema).into_json(),
            json!({
                "name": "",
                "address": { "city": "", "tags": [] }
            })
        );
    }

    #[test]
    fn property_less_object_degrades_to_empty_object() {
        // A property-less object is schema-invalid in practice; `{}` is the
        // only type-correct blank it can carry.
        let schema = field("type: object\n");
        assert_eq!(blank(&schema).into_json(), json!({}));
    }
}
