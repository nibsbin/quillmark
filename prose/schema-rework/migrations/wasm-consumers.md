# Migration: WASM / JavaScript / TypeScript Consumers

Audience: apps that load `@quillmark/wasm` (or the unpublished
`crates/bindings/wasm` build) and call `Quillmark` methods.

## Summary of breaks

| Surface | Before | After |
|---|---|---|
| `QuillInfo.schema` | `Record<string, any>` (JSON Schema object) | `string` (YAML text) |
| `engine.getStrippedSchema(name)` | returned JSON object | **removed** |
| `engine.getQuillSchema(name)` | — | new, returns YAML `string` |
| `QuillInfo.defaults` | unchanged — still `Record<string, any>` | unchanged |
| `QuillInfo.examples` | unchanged — still `Record<string, any[]>` | unchanged |
| `QuillInfo.example` | — | new optional `string` (full markdown example) |

Everything else on `Quillmark` (`registerQuill`, `parseMarkdown`, `render`,
`compile`, `dryRun`, `compileData`, `resolveQuill`, `listQuills`,
`unregisterQuill`, `renderPages`) is unchanged in signature.

## Type-level break

Any TypeScript code that treats `info.schema` as an object no longer
compiles. The regenerated `.d.ts` now declares:

```ts
export interface QuillInfo {
  name: string;
  backend: string;
  metadata: Record<string, any>;
  example?: string;
  schema: string;                   // was: Record<string, any>
  defaults: Record<string, any>;
  examples: Record<string, any[]>;
  supportedFormats: OutputFormat[];
}
```

## Code changes

### Reading the schema

```ts
// before
const info = engine.getQuillInfo('usaf_memo');
const jsonSchema = info.schema;
const title = jsonSchema.properties?.subject?.title;
```

```ts
// after
import yaml from 'js-yaml';
const info = engine.getQuillInfo('usaf_memo');
const schema = yaml.load(info.schema) as {
  name: string;
  description?: string;
  example?: string;
  fields: Record<string, FieldSchema>;
  cards?: Record<string, CardSchema>;
};
const title = schema.fields.subject?.title;
```

See `prose/schema-rework/migrations/form-builders.md` for the full YAML
subset shape.

### `getStrippedSchema()` removal

```ts
// before
const uiSchema = engine.getStrippedSchema('usaf_memo');
```

```ts
// after — the public schema is already the UI-shaped view.
const info = engine.getQuillInfo('usaf_memo');
const schema = yaml.load(info.schema);

// or stand-alone, without the full QuillInfo:
const schemaYaml = engine.getQuillSchema('usaf_memo');
```

The old method returned a JSON Schema with `x-ui` hints preserved. The
new schema has native `ui:` keys already — there is no separate "AI
view" and "UI view" anymore.

### `x-ui` → `ui` rename

If you were reading UI hints off the old JSON Schema:

```ts
// before
const group = schema.properties?.subject?.['x-ui']?.group;
```

```ts
// after
const group = schema.fields?.subject?.ui?.group;
```

Underlying hint keys (`group`, `order`, `compact`, `multiline`,
`hide_body`, `default_title`) are unchanged.

### `CARDS` projection

The JSON Schema used to expose cards as a top-level `CARDS` array property
with `$ref`/`oneOf` branching. That is replaced by a top-level `cards:`
map:

```ts
// before
const cardTypes = schema.properties?.CARDS?.items?.oneOf
  ?.map(b => b.$ref.split('/').pop()) ?? [];
```

```ts
// after
const cardTypes = Object.keys(schema.cards ?? {});
```

Rendering rows for card instances stays the same — the instances still
travel through `ParsedDocument.fields.CARDS` at *runtime* (that's a
backend data-injection shape, see `04-cutover.md` non-goals). Only the
*schema* representation of cards changed.

## Validation errors

`dryRun()` still throws on validation failure, but the error payload
shape is different. The old payload mirrored the `jsonschema` crate:
pointer-style paths, schema-keyword codes.

The new payload is driven by `ValidationError` in
`crates/core/src/quill/validation.rs:12`, with six variants:

- `MissingRequired { path }`
- `TypeMismatch { path, expected, actual }`
- `EnumViolation { path, value, allowed }`
- `FormatViolation { path, format }`
- `UnknownCard { path, card }`
- `MissingCardDiscriminator { path }`

Paths are field paths, not JSON Pointers:
`cards.indorsement[0].signature_block`, not
`/properties/CARDS/items/oneOf/0/properties/signature_block`.

If you were pattern-matching on old error codes or message substrings,
update to match the new enum / message shape. The wire format is still
whatever `dryRun` / `compileData` threw before — check
`crates/bindings/wasm/src/engine.rs` for the current shape.

## Package consumption note

No new runtime dependencies. If you want to parse the YAML, pull a YAML
library (`js-yaml`, `yaml`, `@std/yaml` on Deno). We don't bundle one —
keeping the binding surface small was explicit.

## Quick checklist

- [ ] `info.schema` is now a `string`; add a YAML parse step where you
      used to index into the object
- [ ] Delete `engine.getStrippedSchema(...)` calls; use `info.schema`
      directly or `engine.getQuillSchema(name)`
- [ ] Rename TypeScript types referring to `QuillInfo.schema` (was
      `object`-like, now `string`)
- [ ] Replace `x-ui` reads with `ui`
- [ ] Replace `properties.CARDS.items.oneOf` walks with
      `cards[name]` lookups
- [ ] Replace `properties.X` walks with `fields.X`
- [ ] Update `catch` blocks if you pattern-matched on old validation
      error shape
