# Quill Versioning System

> **Implementation**: `crates/core/src/`
> **Related**: [QUILL.md](QUILL.md), [ERROR.md](ERROR.md)

## TL;DR

Quills declare a semantic `version` in `Quill.yaml`, and documents carry an optional `$quill: name@selector` reference. The selector is parsed and stored on `QuillReference`, but never **resolved**: the engine loads exactly one Quill from a path or in-memory file tree, never picking among versions. It is **enforced**: at render time the reference's two components are checked against the loaded Quill, and either a *name* mismatch or a *version* outside the selector is a hard error. The document is valid; it was paired with the wrong Quill, which is a footgun.

## Version Format

Semantic versioning: `MAJOR.MINOR.PATCH`, with two-segment `MAJOR.MINOR` also
valid. `version` is required in the `quill:` section, and an invalid or missing
value fails at load. Always quote it: an unquoted `1.0` is read as a YAML number
and stringified to `"1"`, which fails validation.

| Increment | When |
|-----------|------|
| **MAJOR** | Breaking changes: layout changes, removed fields, incompatible types |
| **MINOR** | New optional fields, enhancements (backward-compatible) |
| **PATCH** | Bug fixes, corrections (backward-compatible) |

## Document Syntax

The version selector rides on the root block's `$quill` system-metadata line (see [markdown-spec.md](../references/markdown-spec.md) §3.3):

```
$quill: my_format@2.1.0    # exact
$quill: my_format@2.1      # 2.1.x
$quill: my_format@2        # 2.x.x
$quill: my_format@latest   # latest (explicit)
$quill: my_format          # latest (default)
```

No registry consumes the selector: there is no collection of installed versions to pick from, so it is a pin, not a resolver. *Resolution* (matching `name@selector` against a set of installed versions) belongs to a higher layer; the engine loads one Quill and *enforces* the reference against it. Detection needs no registry (the engine has the loaded Quill's name and version and the document's reference) so `render` and `dry_run` both reject a mismatch with a single-diagnostic [`RenderError`](ERROR.md). They check in order:

- **`quill::name_mismatch`**: the reference *name* differs from the loaded Quill. The name is the prerequisite (a selector belongs to a *named* Quill), so a name mismatch short-circuits and the version is left unevaluated.
- **`quill::version_mismatch`**: names agree but the Quill's `version` falls outside the selector (e.g. `name@2` against `3.0.0`). `VersionSelector::matches` decides: `Exact` the identical version, `Minor` any patch in the `MAJOR.MINOR` series, `Major` any version in the `MAJOR` series, `Latest` (the default) anything.

A quill mismatch is distinct from a validation failure (a malformed document): here the document is well-formed but paired with the wrong Quill, so the remedy is to render with the referenced Quill or amend `$quill`. A bare name or `@latest` matches any version, so correctly-targeted documents never trip either check.

`$quill` is a **pairing assertion**, not a render target or a schema declaration. The caller chooses which Quill renders; the reference only confirms the document was authored against it.

## Error Handling

Three distinct failure paths, and the parser owns one of them outright:

- **`Quill.yaml` version invalid** → `quill::invalid_version` → surfaces as
  `RenderError::QuillConfig` at Quill load.
- **Document `$quill` reference invalid** (e.g. `my_format@bad`) →
  `ParseError::InvalidQuillReference`, returned directly by the parser, never as
  `RenderError::QuillConfig`.
- **Loaded Quill does not satisfy a well-formed `$quill`** → the two mismatch
  codes above, as a `RenderError` from `render`/`dry_run`.

## Ref Immutability

A canonical ref (`name@version`) is **immutable content**, at least within the
lifespan of a runtime: once any layer has materialized a Quill for a ref, the
content behind that ref never changes for that process. Publishing different
content requires a version bump.

Every cache between a document and its rendered output keys on this invariant,
and none of them exposes an invalidation API, **by design**:

- quiver's quill cache holds one `Quill` per canonical ref for the `Quiver`
  instance's lifetime;
- app-level services cache that same instance per canonical ref;
- the wasm `Engine` caches backend-memory clones in a `WeakMap` keyed on the
  canonical `Quill` instance, so a clone's lifetime follows the instance.

"Invalidate" therefore means *replace the instance*: a new `Quill` at a new
ref, or a new `Quiver`, and the downstream caches follow automatically
(WeakMap + weak refs).
