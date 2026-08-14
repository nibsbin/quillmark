# Enum Variants

> **Issue**: borb-sh/quillmark#1268
> **Touches**: `crates/core/src/quill/`, `crates/fixtures/resources/quills/usaf_memo/`, `prose/canon/SCHEMAS.md`, `prose/canon/BLUEPRINT.md`

## TL;DR

An enum field may declare `variants:`: per-value field sets that exist only when the discriminant holds that value. The document stays a flat map, the render floor stays total, and out-of-variant authored values warn rather than gate. Implementation is a load-time hoist: variant fields become ordinary entries in the card's field map carrying a `variant_of` back-reference, so coercion, compose, conform, resolve, and both backends need zero changes; only load, validation, blueprint, and seeding read the reference.

## The model

The syntax borrows the tagged-union shape; the semantics are deliberately weaker, and the weakening is the design:

- **Flat document.** Variant fields are authored at the card's top level like any field (`cui_poc: …`, never nested under `classification:`). Existing documents parse unchanged.
- **Union-typed.** Every variant field has one unconditional type. Coercion never consults another field's value. This is enforced by a load-time name-uniqueness rule, not by runtime dispatch.
- **Total floor.** Variant fields are always present in the plate projection, blank-filled by the ordinary ladder. `plate.typ` reads `data.cui_poc` unconditionally today and continues to; the empty-document render contract holds untouched.
- **Warning, never a gate.** An authored non-blank value in an inactive variant's field draws `validation::out_of_variant` at `Severity::Warning`. The value still coerces (malformed stays fatal), still flows to the plate per the ladder, and the document still renders. Flipping the discriminant in an editor strands data as a warning, not an invalid document.

So "conditional existence" is a fact of the authoring, validation, and UI surfaces, and deliberately not a fact of the wire. What the DSL gains: the schema names which fields belong to which world, `must_fill` becomes "required in this world", a UI gets a machine-readable hide/show signal, and the LLM authoring loop gets told "not done" when a discriminant flip brings obliged cells into play.

## Syntax and load rules

```yaml
classification:
  type: enum
  values: [UNCLASSIFIED, CUI, CONFIDENTIAL, SECRET, TOP SECRET]
  variants:
    CUI:
      cui_controlled_by: { type: string }
      cui_poc:           { type: string }
      cui_category:      { type: string, default: "" }
```

`variants:` is a map from enum value to a `fields:`-shaped map; each value schema uses the full existing field grammar (`ui:`, `default:`, `example:`, `must_fill:`, nested `items:`/`properties:`).

Load errors, per-cause codes following the `quill::enum_blank_member` precedent:

| Condition | Code |
|---|---|
| `variants:` on a non-enum field | `quill::variants_on_non_enum` |
| a `variants:` key not in `values:` (the blank `""` falls out here: it is never a member) | `quill::variant_unknown_value` |
| a variant field name colliding with any other field in the card — flat fields and every variant of every enum, one namespace | `quill::variant_field_collision` |
| `variants:` below card level (inside `items:` or `properties:`) or on a variant field itself (no recursion) | `quill::variant_placement` |

The one-namespace rule is what keeps coercion value-independent: the effective type map is the flat union, and the collision that would make a field's type depend on the discriminant is unrepresentable.

## Implementation: hoist at load

The whole cut rides one load-time pass, and its cheapness is the argument for this shape.

**Parse.** `FieldSchemaDef` gains `variants: Option<serde_json::Map<String, serde_json::Value>>`; `FieldSchema::from_quill_value` parses it into a transient `variants: Option<IndexMap<String, IndexMap<String, Box<FieldSchema>>>>` on `FieldSchema` (never serialized; order preserved).

**Hoist.** The loader post-pass (`QuillConfig::from_yaml_with_warnings`, beside the content-cache import) drains each card's `variants` maps: after the membership, placement, and collision checks, every variant field is spliced into `CardSchema.fields` immediately after its discriminant, variants in declaration order, fields in declaration order within each. Each hoisted `FieldSchema` is stamped `variant_of: Option<VariantOf { field: String, value: String }>`.

**Everything downstream is free.** Coercion, `compile_data` and the ladder, `conform`, `resolve()`, the pdfform widget walk, and the Typst `_qm-meta` address tables all iterate `CardSchema.fields` and see ordinary fields. `resolve()` keeps its byte-for-byte-with-the-plate contract with no edit. Declaration order stays the one ordering carrier: hoist position *is* display and blueprint position.

**Serialization.** The hand-written `FieldSchema` serializer emits `variant_of: { field: …, value: … }` as the last key (after `inline`). `QuillConfig::schema()` therefore emits the union flat with per-field annotations rather than mirroring the authored nesting. That is the better consumer contract: an editor renders one ordered field list and keys hide/show on the annotation, instead of re-implementing the hoist and inventing an ordering for variant fields. `FieldSchemaDef` rejects a wire `variant_of:` with a directive message (the `ui.order` / `enum:` parse-to-reject precedent): the authoring spelling stays nested-only.

## Validation

One shared helper: resolve each discriminant per card as authored-non-null › `default:` › blank (the render ladder minus `example`, which never enters it), and derive an active-variant predicate `field is in play ⇔ variant_of is None, or its (field, value) matches the resolved discriminant`. The blank discriminant activates nothing.

Two consumers in `validation.rs`:

- **`unauthored` must-fill trigger**: skips out-of-play fields. In-play variant fields keep the ordinary derivation (`must_fill:` unset derives from `default:` presence), which is the whole conditional-obligation story — no new axis. The `marker` trigger stays document-sovereign and fires regardless: a human's `!must_fill` is not the schema's to suppress.
- **`validation::out_of_variant`** (new, `Severity::Warning`): an authored, non-blank value in an out-of-play field. Blank-ness compares against the field's `blank` producer (empty content for prose types); the `0`/`false` seam from the blank table applies here too — a numeric variant field's authored `0` reads as blank and never warns, accepted as the same permanent seam. Message per the ERROR.md contract: field path, the discriminant's name and resolved value, the owning variant value, and both exits (set the discriminant, or clear the field).

## Projections

**Transform schema** (`build_transform_schema`): emit `quillmark:variant_of: { field, value }` in the prelude beside `quillmark:must_fill` (before the enum early-return). `quillmark:must_fill` keeps its unconditional derived answer; `variant_of` scopes it — the pair reads "must fill, in this world". No `if`/`then` emission: an out-of-variant value is wire-valid by design (it coerces and renders), so a standard validator accepting it is correct, and the engine's `validate` remains the authority on the warning.

**Blueprint**: the blueprint-active variant per discriminant is derived from its cell value (`default:` › `example:` › bare blank). In-play variant fields emit as live YAML with the ordinary per-field treatment; out-of-play variant fields are skipped entirely — no commented-out fields, per the existing rule. Each discriminant gains one leading `# when <VALUE>: <field>, <field>, …` line per skipped variant, after the description line and before `# e.g.`. Field names are snake_case, so the line cannot collide with the reserved-character grammar. Discovery of the skipped fields' types and descriptions is the validate loop's job: an agent that writes `classification: CUI` and re-validates receives `unauthored` warnings naming the newly obliged cells. The parse/round-trip/render guarantee holds trivially (fewer fields, plus comments, which round-trip).

**Seeding**: the seed-active variant per discriminant is `example:` (committed) › `default:` › blank. Example-bearing in-play variant fields commit as usual, marker stamping unchanged; out-of-play variant fields never seed.

**Plate contract addendum** (BLUEPRINT.md guarantees section): a plate keys a variant's block on the discriminant, never on the variant fields' non-emptiness — an out-of-variant value reaches the plate and must stay inert there.

## Fixture migration

`usaf_memo` 0.2.0, edited in place: no type changes, no stored-value reinterpretation, and plate JSON stays byte-identical, so the VERSIONING ref-immutability rule (which targets type changes that rewrite stored values) is not tripped; the only behavioral delta is new warnings.

- Move `cui_controlled_by`, `cui_poc`, `cui_category`, `cui_limited_dissemination` under `classification.variants.CUI`, keeping each field's `ui.group: classification`.
- Drop `default: ""` from `cui_controlled_by` and `cui_poc`: the derivation then obliges them when CUI is active, matching DoDM 5200.48. `cui_category` and `cui_limited_dissemination` keep `default: ""` (skippable).
- Trim the "Required … when classification is CUI" clauses from descriptions: the scoping is structural now.
- Hoist order places the `cui_*` fields between `classification` and `dissemination`; regenerate `__golden__/schema.yaml`. An omitted `cui_controlled_by`/`cui_poc` moves from the `default` rung to the `blank` rung in `resolve()` output (same `""` bytes); regenerate any golden that captures source tags.

## Canon and docs

- `SCHEMAS.md`: a "Variants" subsection in the DSL section (syntax, load errors, hoist/`variant_of`, the flat-document/total-floor/warning triad), plus the `out_of_variant` entry and the `unauthored`-trigger scoping under Native validation.
- `BLUEPRINT.md`: the `# when` leading line and the in-play emission rule; the plate contract addendum.
- `ERROR.md`: register the new codes.
- `docs/quills/quill-yaml-reference.md`: author-facing `variants:` entry.

## Tests

- Load: parse happy path; each error code; hoist order and `variant_of` stamps; two variant-bearing enums in one card.
- Emission: `schema()` flat-plus-annotation (golden); transform schema `quillmark:variant_of`.
- Validation: `out_of_variant` fires on authored non-blank out-of-play, not on blank/null/in-play, and resolves the discriminant through `default:`; `unauthored` skips out-of-play and fires in-play; marker sovereignty on an out-of-play field.
- Blueprint: skip plus `# when` line with a blank-defaulted discriminant; live variant fields when the default names the variant; existing quiver round-trip/render tests cover the migrated fixture.
- Seeding: out-of-play examples not committed.
- Render: migrated `usaf_memo` plate JSON byte-identical to pre-migration.

## Deferred

Out of the initial cut, each a compatible later extension: multi-value variant keys (the CONFIDENTIAL/SECRET/TOP SECRET shared block — the accepted ceiling; YAML anchors are the interim spelling), floor suppression of out-of-variant values (would demote the authored rung; needs its own case), in-play tagging on `resolve()` rows (consumers already hold `variant_of` and the discriminant row), `variants:` below card level, JSON-Schema `if`/`then` on the transform wire, and non-enum discriminants (#1202's `SEE DISTRIBUTION` shape remains open there).
