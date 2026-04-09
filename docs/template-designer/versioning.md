# Quill Versioning

Versioning helps template designers evolve Quills safely while keeping document rendering predictable.

## Version Field in `Quill.yaml`

Each Quill should declare a semantic version:

```yaml
Quill:
  name: my-quill
  backend: typst
  description: A professional document template
  version: "1.2.0"
```

Use semantic versioning (`MAJOR.MINOR.PATCH`) to communicate compatibility:

- **MAJOR**: breaking changes to fields, cards, or expected document shape
- **MINOR**: backward-compatible additions (new optional fields, non-breaking behavior)
- **PATCH**: fixes and small improvements without shape changes

## How Authors Select Versions

Authors can target versions through the `QUILL` key in frontmatter:

```markdown
---
QUILL: my-quill@1.2
title: Quarterly Report
---
```

Supported selectors:

| Selector | Meaning |
|---|---|
| `my-quill` | Latest available version |
| `my-quill@latest` | Latest available version (explicit) |
| `my-quill@1` | Latest 1.x.x |
| `my-quill@1.2` | Latest 1.2.x |
| `my-quill@1.2.0` | Exact version |

## Practical Guidelines

1. Start at `1.0.0` for your first stable internal template release.
2. Increase versions on every template change, even if small.
3. Treat field renames/removals as breaking (`MAJOR`) changes.
4. Prefer additive changes (new optional fields/cards) to reduce migration work.
5. Keep example documents updated for the latest major/minor versions.

## Related Pages

- [Creating Quills](creating-quills.md)
- [Quill.yaml Reference](quill-yaml-reference.md)
- [YAML Frontmatter](../authoring/yaml-frontmatter.md)
