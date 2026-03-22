# backends/quillmark-acroform

Status: **Implemented** (2026-03-22)  
Source: `crates/backends/acroform`

## Quill Expectations
- Quill.yaml must set `backend: acroform`. No plate file is required (`plate_extension_types` is empty).
- Bundle typically contains a single `form.pdf` plus optional `example.md`.

## Compilation Flow
1. Load PDF form with `acroform` crate.
2. For each field:
   - Prefer tooltip templating using `description__{{ template }}`.
   - Otherwise treat the existing field value as a MiniJinja template.
3. Render templates with the JSON from `Workflow::compile_data()`.
4. Write rendered values back to a PDF and return as a single `Artifact` (PDF).

## Notes
- Ignores plate content entirely; all inputs come from JSON.
- Templates can reference any document field or card (`CARDS`) present in the JSON.
- Output formats: PDF only.
