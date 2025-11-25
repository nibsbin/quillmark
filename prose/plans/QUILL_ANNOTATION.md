# Quill Annotation Revamp

## Goals

- Create metadata foundation for defining dynamic UI forms
- Migrate away from jsonschema? Use custom validation system?

## Out of scope

## Structure

**Current**

- key (name)
- description
- type (array | string | bool | number)
- default 
- example

**New**

- Section
- Tooltip

## API

We need to expose a way for WASM consumers to retrieve the annotations.

**Potential frontend flow:**
parse markdown -> extract Quill tag -> 
if tag changed, retrieve Quill info ->
update wizard

## Cleanup
- Split up `quillmark/src/orchestration.rs` into clean, simple organization of files for maintainability.

**Consolidate workflow creation functions to stay opinionated**
Currently there are:
- workflow_from_quill_name
- workflow_from_quill
- workflow_from_parsed

`workflow_from_quill_name` and `workflow_from_quill` are redundant. We could remove `workflow_from_quill_name` and `workflow_from_parsed` to force all renders to flow through `workflow_from_quill`. We could rename this consolidated function to just `new_workflow()`.

## Impl
- In the quillmark crate, expose a function to retrieve the Quill from bindings
- Wasm bindings engine should not maintain its own map of registered Quills. Rely on core engine's registry to avoid drift and memory overhead.
- Changes to QuillInfo in `bindings/quillmark-wasm/src/types.rs`


## IMPORTANT DECISION!!!
- Migrate away from jsonschema?
- Use Quill config fields as single source of truth?
    - Still flow through jsonschema? Probably not
- Use another schema library/language?

**jsonschema (current)**

+ Widely supported (especially in js environment)

- Can it support tooltip and section attributes?
    - If no,Requires fragmentation of metadata for UI support

**another schema language?**




**custom validation**

+ Supports all metadata (schema, tooltip, section) with no fragmentation

- We have to write it