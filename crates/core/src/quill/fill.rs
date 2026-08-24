//! The per-field blank: the field's spelling of "explicitly nothing", and the
//! render floor, shared by blueprint emission and the blank-filled render.

use serde_json::json;

use super::{FieldSchema, FieldType, VARIANT_DISCRIMINANT_KEY};
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
/// | `enum` with `variants:` | `{value: ""}` — the container holding the blank |
///
/// A field's blank is a property of the *field*, not a member of the type's
/// value domain: an `enum`'s blank sits outside `values:` rather than claiming
/// a variant, so no document renders a choice nobody made. The loader rejects a
/// declared `""` (`quill::enum_blank_member`), but this function does not lean
/// on that: a [`QuillConfig`](super::QuillConfig) built through serde bypasses
/// loader validation, so the `enum` blank is `""` unconditionally.
///
/// `integer`, `number` and `boolean` are a permanent seam: their blanks (`0`,
/// `false`) are indistinguishable at the plate from an authored `0` / `false`,
/// as is any container over them. A wire `none` for those types would be
/// type-*absent* rather than type-*minimal*, and Typst arithmetic rejects it,
/// costing the render totality this floor buys.
///
/// An `object` with `properties` is shape-valid only when every property is
/// present, so it recurses rather than degrading to the bare `{}` that only a
/// property-less object carries.
pub fn blank(field: &FieldSchema) -> QuillValue {
    // A variant-bearing enum rests as a container, so its blank is the container
    // holding the blank discriminant. The blank activates no variant, so the
    // object carries nothing else: the field set a member would bring is exactly
    // what nobody has chosen.
    if field.is_variant_bearing() {
        let mut obj = serde_json::Map::new();
        obj.insert(VARIANT_DISCRIMINANT_KEY.to_string(), json!(""));
        return QuillValue::from_json(serde_json::Value::Object(obj));
    }
    // Keyed on the carrier rather than the `Enum` token, as every consumer of a
    // finite domain is (`SCHEMAS.md` § "Type coercion"), so a serde-built schema
    // whose type and carrier disagree still blanks to the reserved `""`.
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
            quillmark_content::serial::to_canonical_value(&quillmark_content::Normalized::empty())
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

    /// The carrier decides, so the answer survives a schema whose type and
    /// carrier disagree — which the loader rejects (`values:` on a non-enum
    /// type) and serde builds anyway.
    #[test]
    fn enum_values_on_a_non_enum_type_blanks_to_the_empty_string() {
        let mut schema = FieldSchema::new("clearance".to_string(), FieldType::Integer, None);
        schema.enum_values = Some(vec!["1".to_string(), "2".to_string()]);
        assert_eq!(blank(&schema).into_json(), json!(""));
    }

    #[test]
    fn property_less_object_degrades_to_empty_object() {
        // A property-less object is schema-invalid in practice; `{}` is the
        // only type-correct blank it can carry.
        let schema = field("type: object\n");
        assert_eq!(blank(&schema).into_json(), json!({}));
    }
}
