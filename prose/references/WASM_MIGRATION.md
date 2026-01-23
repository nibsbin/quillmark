
> **Version**: 0.31.0 (Versioning Rework)

This guide covers breaking changes for JavaScript/TypeScript consumers of the Quillmark WASM bindings.

## Breaking Changes

### `quillTag` → `quillName`

The `ParsedDocument` type now uses `quillName` instead of `quillTag`:

```diff
  const doc = engine.parseMarkdown(markdown);
- const tag = doc.quillTag;
+ const name = doc.quillName;
```

**TypeScript types updated:**

```typescript
interface ParsedDocument {
  fields: Record<string, any>;
- quillTag: string;
+ quillName: string;
}
```

### Version Syntax in QUILL Field

Documents can now specify version selectors in the QUILL frontmatter:

```yaml
---
QUILL: "my_template@2.1"      # Exact version
QUILL: "my_template@2"        # Latest 2.x
QUILL: "my_template@latest"   # Latest overall (explicit)
QUILL: "my_template"          # Latest overall (default, unchanged)
---
```

**The `quillName` property returns only the template name** (without version suffix):

```javascript
const doc = engine.parseMarkdown(`---
QUILL: resume_template@2.1
---`);

console.log(doc.quillName);  // "resume_template" (not "resume_template@2.1")
```

### Quill.toml Requires `version` Field

All Quill bundles must now include a `version` field:

```toml
[Quill]
name = "my_template"
version = "1.0"          # Required!
backend = "typst"
description = "..."
```

**Error if missing:**
```
Missing required field 'version' in Quill.toml
```

## New Error Types

Three new error types for version resolution:

| Error | When |
|-------|------|
| `VersionNotFound` | Requested version doesn't exist (e.g., `@2.3` when only `2.0, 2.1, 2.2` registered) |
| `QuillNotFound` | Template name not registered at all |
| `InvalidVersion` | Malformed version string (e.g., `@abc`, `@1.2.3`) |

**Error structure:**

```javascript
try {
  const workflow = engine.workflow("resume@2.3");
} catch (err) {
  // err.type === "diagnostic"
  // err.message contains available versions
  // err.diagnostic has structured info
}
```

## Migration Checklist

- [ ] Replace all `quillTag` references with `quillName`
- [ ] Update TypeScript types if using custom interfaces
- [ ] Add `version` field to all Quill.toml files
- [ ] Handle new version-related error types
- [ ] (Optional) Pin document versions for reproducibility

## Unchanged APIs

The following remain unchanged:

- `Quillmark.parseMarkdown(markdown)` - same signature
- `Quillmark.registerQuill(quillJson)` - same signature  
- `Quillmark.render(doc, options)` - same signature
- `RenderOptions.quillName` - overrides document's QUILL field
- `QuillInfo` structure - same fields

## Example: Before and After

**Before (v0.29.x):**

```javascript
const engine = new Quillmark();
await engine.registerQuill(quillJson);

const doc = engine.parseMarkdown(markdown);
const quillName = doc.quillTag;  // ❌ Old API

const workflow = engine.workflow(quillName);
const result = await workflow.render(doc);
```

**After (v0.30.0):**

```javascript
const engine = new Quillmark();
await engine.registerQuill(quillJson);  // Quill must have version in Quill.toml

const doc = engine.parseMarkdown(markdown);
const quillName = doc.quillName;  // ✅ New API

// Version resolution happens automatically from QUILL field
const workflow = engine.workflow(quillName);
const result = await workflow.render(doc);
```

## Related Documentation

- [VERSIONING.md](../../prose/designs/VERSIONING.md) - Full versioning system design
- [README.md](./README.md) - WASM API reference
