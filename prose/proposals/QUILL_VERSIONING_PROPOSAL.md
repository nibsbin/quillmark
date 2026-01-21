# Quill Versioning System Proposal
## Enabling Reproducible Rendering with Version Pinning

**Date:** 2026-01-21
**Context:** Enable documents to specify which version of a Quill template they require, ensuring reproducible rendering across time as templates evolve.
**Design Focus:** Simplicity for users, semantic versioning for compatibility, extensibility for future distribution systems.

---

## Problem Statement

Quillmark currently provides no mechanism for documents to specify which version of a Quill template they require. This creates stability problems as templates evolve:

1. **No reproducibility.** A document that renders correctly today may break or look different tomorrow if the template is updated. There is no way to guarantee that a document will render identically in the future.

2. **Breaking changes require caution.** Template authors must be extremely conservative about improvements because any change might break existing documents. This slows template evolution and makes bug fixes risky.

3. **No compatibility signaling.** Template authors have no way to communicate whether a change is backward-compatible (bug fix, new feature) or breaking (layout change, field removal). Users cannot distinguish safe updates from dangerous ones.

4. **Single version per template name.** The engine can only register one version of each template. If multiple versions coexist, they must be registered under different names, forcing awkward naming schemes like `resume-template-v2`.

The existing `version` field in Quill.toml is purely informational and unused by the rendering system. It provides documentation but no runtime behavior.

---

## Proposed Solution

Extend the QUILL tag syntax to support version specification using semantic versioning. Allow partial version constraints (major-only, major.minor) that resolve to the latest matching version. This provides both exact pinning for stability and flexible constraints for compatibility.

### Core Design Principles

1. **Backward compatible.** Documents without version specifications continue to work, defaulting to the latest available version.

2. **Simple for non-technical users.** Basic version syntax is easy to understand: `@2` means "version 2", `@2.1` means "version 2.1", `@2.1.3` means exactly that version.

3. **Semantic versioning semantics.** Major versions signal breaking changes, minor versions signal compatible additions, patch versions signal bug fixes. This leverages existing mental models from the broader software ecosystem.

4. **No complex syntax required.** Unlike full semver range syntax (`^1.2.0`, `~1.2.0`, `>=1.0.0`), which is confusing for non-technical users, we support only simple partial specifications that are self-explanatory.

5. **Extensible for future distribution.** The design accommodates future features like remote repositories, dependency resolution, and lockfiles without requiring changes to the document format.

---

## Version Specification Syntax

Documents specify versions in the QUILL tag using `@` separator:

```yaml
---
QUILL: "resume-template@2.1.3"    # Exact version
QUILL: "resume-template@2.1"      # Latest 2.1.x
QUILL: "resume-template@2"        # Latest 2.x.x
QUILL: "resume-template@latest"   # Latest overall (explicit)
QUILL: "resume-template"          # Unspecified (uses default, typically latest)
---
```

### Parsing Rules

The parser splits on `@` to extract the template name and version constraint. The version constraint is parsed as:

- **Three components (2.1.3)** — Exact match required.
- **Two components (2.1)** — Match latest patch version in the 2.1 series.
- **One component (2)** — Match latest minor.patch version in the 2.x series.
- **Keyword "latest"** — Match the highest version available.
- **No version** — Apply default policy (recommended: latest with optional warning).

Version numbers follow semantic versioning: `MAJOR.MINOR.PATCH`. Template authors increment major for breaking changes, minor for backward-compatible features, and patch for bug fixes.

### Resolution Semantics

Version resolution selects the highest version that satisfies the constraint. Given available versions `[1.0.0, 2.0.0, 2.1.0, 2.1.5, 2.2.0, 3.0.0]`:

- `@3` resolves to `3.0.0` (latest 3.x)
- `@2` resolves to `2.2.0` (latest 2.x)
- `@2.1` resolves to `2.1.5` (latest 2.1.x)
- `@2.1.0` resolves to `2.1.0` (exact match)
- `@latest` resolves to `3.0.0` (highest overall)

If no matching version exists, rendering fails with an error listing available versions.

---

## Engine Architecture Changes

The engine maintains a version registry that maps template names to sets of versioned Quills. When a document is rendered, the engine parses the version constraint and resolves it against the available versions.

### Data Structures

```rust
pub struct Quillmark {
    quills: HashMap<String, VersionedQuillSet>,
    backends: HashMap<String, Arc<dyn Backend>>,
}

pub struct VersionedQuillSet {
    name: String,
    versions: BTreeMap<Version, Quill>,
    default_selector: VersionSelector,
}

pub enum VersionSelector {
    Exact(Version),           // 2.1.3
    Major(u64),               // 2
    MajorMinor(u64, u64),     // 2.1
    Latest,
    Unspecified,
}

pub struct QuillReference {
    name: String,
    version: VersionSelector,
}
```

### Registration Flow

When a Quill is registered, the engine reads its version from Quill.toml and stores it under the template name. Multiple versions of the same template can coexist:

```rust
impl Quillmark {
    pub fn register_quill(&mut self, quill: Quill) -> Result<()> {
        let name = quill.name().to_string();
        let version = quill.version()
            .ok_or(Error::QuillMissingVersion(name.clone()))?;

        self.quills
            .entry(name)
            .or_insert_with(|| VersionedQuillSet::new(&name))
            .add_version(version, quill);

        Ok(())
    }
}
```

### Render Flow

When rendering a document, the engine parses the QUILL tag, resolves the version, and creates a workflow:

```
ParsedDocument
    ↓
Extract QUILL tag: "resume-template@2.1"
    ↓
Parse to QuillReference { name: "resume-template", version: MajorMinor(2, 1) }
    ↓
Resolve against available versions → Version(2.1.5)
    ↓
Retrieve Quill from registry
    ↓
Create Workflow with resolved Quill and Backend
    ↓
Render
```

The resolution logic is pure and testable. It takes a version constraint and a set of available versions and returns the best match or an error.

---

## Template Metadata Requirements

The Quill.toml `version` field becomes mandatory for all templates. Without it, the template cannot be registered in the versioned system. Existing templates will need migration.

```toml
[Quill]
name = "resume-template"
version = "2.1.5"              # Required: semantic version
backend = "typst"
description = "Professional resume template"
```

Template authors should follow semantic versioning conventions when bumping versions. The system does not enforce this—it trusts authors to signal compatibility correctly—but documentation and tooling should guide proper version management.

---

## CLI Enhancements

The CLI gains new commands for version management:

### Version Listing

```bash
$ quillmark versions resume-template
Available versions for resume-template:
  3.0.0
  2.2.0
  2.1.5
  2.1.0
  2.0.0
  1.0.0
```

### Version Resolution

```bash
$ quillmark resolve resume-template@2
resume-template@2 → 2.2.0

$ quillmark resolve resume-template@2.1
resume-template@2.1 → 2.1.5
```

### Document Pinning

```bash
$ quillmark pin document.md
Updated document.md: QUILL "resume-template@2.1.5"

$ quillmark pin document.md --major
Updated document.md: QUILL "resume-template@2"

$ quillmark pin document.md --minor
Updated document.md: QUILL "resume-template@2.1"
```

The pin command adds or updates the version constraint in the document's QUILL tag. This allows users to lock documents to specific versions after verifying they render correctly.

### Upgrade Assistant

```bash
$ quillmark upgrade document.md
Current: resume-template@2
Latest:  resume-template@3.0.0

Warning: Major version change (2 → 3) may contain breaking changes.
Proceed? [y/N]
```

The upgrade command helps users transition documents to newer template versions with appropriate warnings about compatibility.

---

## Error Messages and Diagnostics

When version resolution fails, the system provides actionable error messages with context:

```
Error: Version not found
  Template: resume-template
  Requested: @2.3
  Available: 2.2.0, 2.1.5, 2.1.0, 2.0.0, 1.0.0

  Suggestion: Use @2 for latest 2.x, or specify @2.2
```

When a document uses an unversioned QUILL tag, the system can optionally warn (configurable):

```
Warning: Unversioned template reference
  Document: document.md
  Template: resume-template
  Rendering with: resume-template@2.1.5 (latest)

  To lock this version: quillmark pin document.md
```

---

## Migration Strategy

### Phase 1: Additive Changes

Implement version parsing, resolution, and registry without breaking existing behavior. Documents without version constraints continue to use the latest available version. This phase is backward compatible.

### Phase 2: Template Migration

Update all existing Quills to include proper semantic versions in their Quill.toml files. Create tooling to scan template directories and assign initial versions. Establish guidelines for version bumping.

### Phase 3: Default Policy Adjustment

Consider making version specification recommended or required for new documents. Add CLI warnings for unversioned documents to encourage explicit pinning. Provide migration tooling to bulk-update document collections.

---

## Future Extensions

The core versioning system enables several future enhancements:

### Remote Repositories

Version specification works naturally with remote template repositories. The syntax could be extended to include repository URLs or aliases:

```yaml
QUILL: "https://quills.example.com/resume-template@2.1.3"
QUILL: "github:user/repo/resume-template@2.1"
```

The engine would fetch and cache the specified version on demand. The resolution logic remains unchanged—only the source of available versions differs.

### Dependency Resolution

Templates could declare dependencies on other templates or on specific Quillmark versions:

```toml
[Quill]
name = "resume-template"
version = "2.1.5"
min_quillmark = "0.30.0"

[dependencies]
common-styles = "1.2"
```

The engine would resolve the dependency tree and ensure all constraints are satisfied before rendering.

### Lockfiles

For reproducibility across machines, projects could use lockfiles that record exact versions:

```toml
# quillmark.lock
[documents."resume.md"]
quill = "resume-template"
resolved_version = "2.1.5"
rendered_at = "2026-01-21T10:30:00Z"
```

The lockfile pins versions without modifying document source files, similar to package-lock.json in Node.js or Cargo.lock in Rust.

### Version Ranges (Advanced)

If future use cases require more complex constraints, the system could be extended to support semver range syntax:

```yaml
QUILL: "resume-template@^2.1.0"   # >=2.1.0, <3.0.0
QUILL: "resume-template@~2.1.0"   # >=2.1.0, <2.2.0
```

This would enable more sophisticated compatibility specifications but is not necessary for the initial implementation. The simple partial version syntax handles the vast majority of real-world needs.

---

## Implementation Checklist

The feature can be implemented in discrete, testable phases:

1. **Version parsing.** Implement QuillReference::parse and VersionSelector enum with comprehensive unit tests for all syntax variants.

2. **Resolution algorithm.** Implement VersionedQuillSet::resolve with unit tests covering all resolution cases, edge cases, and error conditions.

3. **Engine integration.** Update Quillmark to use VersionedQuillSet internally. Add tests for multi-version registration and resolution.

4. **Workflow creation.** Update workflow_for_document to parse QUILL tags and resolve versions. Test with versioned and unversioned documents.

5. **CLI commands.** Add versions, resolve, pin, and upgrade commands. Test with various version constraints and edge cases.

6. **Error handling.** Implement clear, actionable error messages with available version listings and suggestions.

7. **Documentation.** Update user docs, template author guides, and migration documentation.

8. **Template migration.** Audit existing Quills and assign appropriate versions. Update all plates and examples.

---

## Open Considerations

### Default Version Policy

When a document does not specify a version, the system must choose a default. Options include:

- **Always latest.** Simple and encourages staying current, but sacrifices reproducibility.
- **Latest with warning.** Alerts users to unversioned documents while maintaining convenience.
- **Require explicit version.** Maximizes reproducibility but increases friction for casual users.

The recommended approach is "latest with warning" initially, allowing future tightening based on user feedback.

### Version Validation

Should the system validate that template authors follow semantic versioning correctly? This is challenging—determining whether a change is "breaking" requires semantic understanding. The system cannot enforce this automatically. Instead, rely on documentation, conventions, and community norms.

### Backend Compatibility

Different versions of a template might target different backend versions or require different backend capabilities. The system should consider whether version resolution needs to account for backend compatibility constraints. This could be handled through metadata or ignored initially.

### Multi-Repository Coordination

If templates are distributed across multiple repositories with overlapping names, namespacing becomes necessary. Consider prefixing syntax like `@org/template@version` or repository-qualified names. This can be deferred until remote distribution is implemented.

---

## Benefits

1. **Reproducible rendering.** Documents render identically over time, even as templates evolve.

2. **Safe template evolution.** Authors can improve templates without fear of breaking existing documents.

3. **Clear compatibility signaling.** Semantic versioning communicates the nature of changes to users.

4. **User control.** Document authors choose their stability level: bleeding edge (latest), stable (major version), frozen (exact version).

5. **Foundation for distribution.** The versioning system enables future features like template repositories, dependency management, and publishing workflows.

6. **Familiar model.** Semantic versioning is widely understood in software development, reducing learning curve.
