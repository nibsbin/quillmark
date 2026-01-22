# Quill Versioning System Proposal
## Enabling Reproducible Rendering with Version Pinning

**Date:** 2026-01-21
**Status:** Pre-1.0 addition (breaking for Quill.toml, backward-compatible for documents)
**Context:** Enable documents to specify which version of a Quill template they require, ensuring reproducible rendering across time as templates evolve.
**Design Focus:** Simplicity for users, two-segment versioning for compatibility, extensibility for future distribution systems.

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

Extend the QUILL tag syntax to support version specification using two-segment versioning (`MAJOR.MINOR`). Allow partial version constraints (major-only) that resolve to the latest matching version. This provides both exact pinning for stability and flexible constraints for compatibility.

### Core Design Principles

1. **Default to latest, pin for stability.** Documents without version specification use the latest available version. Documents that need reproducibility specify an explicit version.

2. **Simple for non-technical users.** Basic version syntax is easy to understand: `@2` means "latest version 2", `@2.1` means exactly "version 2.1".

3. **Two-segment versioning.** Major versions signal breaking changes, minor versions signal all compatible changes (bug fixes, new features, improvements). This is simpler than three-segment semver and matches how templates actually evolve.

4. **No complex syntax required.** Unlike full semver range syntax (`^1.2.0`, `~1.2.0`, `>=1.0.0`), which is confusing for non-technical users, we support only simple partial specifications that are self-explanatory.

5. **Extensible for future distribution.** The design accommodates future features like remote repositories, dependency resolution, and lockfiles without requiring changes to the document format.

---

## Version Specification Syntax

Documents specify versions in the QUILL tag using `@` separator:

```yaml
---
QUILL: "resume-template@2.1"      # Exact version
QUILL: "resume-template@2"        # Latest 2.x
QUILL: "resume-template@latest"   # Latest overall (explicit)
QUILL: "resume-template"          # Latest overall (default)
---
```

### Parsing Rules

The parser splits on `@` to extract the template name and optional version constraint.

The version constraint is parsed as:

- **Two components (2.1)** — Exact version match required.
- **One component (2)** — Match latest minor version in the 2.x series.
- **Keyword "latest"** — Match the highest version available (explicit).
- **No version specified** — Default to latest available version.

Version numbers follow two-segment format: `MAJOR.MINOR`. Template authors increment major for breaking changes, minor for all backward-compatible changes (bug fixes, features, improvements).

### Resolution Semantics

Version resolution selects the highest version that satisfies the constraint. Given available versions `[1.0, 1.1, 2.0, 2.1, 2.2, 3.0]`:

- `@3` resolves to `3.0` (latest 3.x)
- `@2` resolves to `2.2` (latest 2.x)
- `@2.1` resolves to `2.1` (exact match)
- `@latest` resolves to `3.0` (highest overall)

If no matching version exists, rendering fails with an error listing available versions.

### Version Bumping Guidelines

**Increment MAJOR when:**
- Layout changes that reflow content
- Removing or renaming required fields
- Changing field types incompatibly
- Switching backends
- Any change that might break existing documents

**Increment MINOR when:**
- Bug fixes (spacing, styling, margins)
- Adding new optional fields or features
- Performance improvements
- Compatible enhancements
- Documentation updates

There is no distinction between patch and minor updates—all non-breaking changes increment the minor version.

---

## Engine Architecture Changes

The engine maintains a **version registry** that maps template names to sets of versioned Quills. When a document is rendered, the engine parses the version constraint and resolves it against the available versions.

### Core Concepts

**Version Registry:**
- Maps template names to multiple version instances
- Each template can have many versions registered simultaneously
- Versions are stored in sorted order for efficient "latest" lookup

**Version Structure:**
- Two components: `major.minor`
- Stored as structured data (not strings) for proper semantic comparison
- Must implement total ordering: `1.0 < 1.1 < 2.0 < 2.1`

**Version Selectors:**
- **Exact:** `2.1` → Match exactly version 2.1
- **Major:** `2` → Match latest version in 2.x series
- **Latest:** `latest` or unspecified → Match highest version available

**Parsing:**
- Split QUILL tag on `@` separator
- Parse version string into selector type
- Handle missing version as "latest"

**Resolution Algorithm:**
- For exact matches: Direct lookup
- For major-only: Find highest minor version with matching major
- For latest: Return highest version overall
- Fail with helpful error if no match found

**Registration:**
- Read version from Quill.toml `version` field
- Validate version format (must be two segments)
- Add to version registry under template name
- Multiple versions coexist without conflict

**Render Flow:**
```
Parse QUILL tag → Extract name and version selector
    ↓
Resolve version selector → Get specific version
    ↓
Retrieve Quill instance → Load template
    ↓
Create Workflow → Render document
```

---

## Template Metadata Requirements

The Quill.toml `version` field becomes mandatory for all templates. Without it, the template cannot be registered in the versioned system. Existing templates will need migration.

```toml
[Quill]
name = "resume-template"
version = "2.1"              # Required: two-segment version
backend = "typst"
description = "Professional resume template"
```

Template authors should follow version bumping guidelines. The system does not enforce semantic correctness—it trusts authors to signal compatibility appropriately—but documentation and tooling should guide proper version management.

### Version Evolution Example

```
1.0 → Initial release
1.1 → Add optional skills section
1.2 → Fix education section spacing, improve typography
1.3 → Add references section, fix margins
2.0 → Complete redesign with new layout (breaking)
2.1 → Add customization options, fix header alignment
2.2 → Improve PDF metadata, add theme variants
3.0 → Switch to new backend or major layout change (breaking)
```

---

## CLI Enhancements

The CLI gains new commands for version management:

### Version Listing

```bash
$ quillmark versions resume-template
Available versions for resume-template:
  3.0
  2.2
  2.1
  2.0
  1.3
  1.2
  1.1
  1.0
```

### Version Resolution

```bash
$ quillmark resolve resume-template@2
resume-template@2 → 2.2

$ quillmark resolve resume-template@2.1
resume-template@2.1 → 2.1
```

### Document Pinning

```bash
# Pin to exact current version
$ quillmark pin document.md
Updated document.md: QUILL "resume-template@2.1"

# Pin to major version (allows minor updates)
$ quillmark pin document.md --major
Updated document.md: QUILL "resume-template@2"

# Show what version is currently being used
$ quillmark pin document.md --show
document.md uses: resume-template@2.1
```

The pin command adds or updates the version constraint in the document's QUILL tag. This allows users to lock documents to specific versions after verifying they render correctly.

### Upgrade Assistant

```bash
$ quillmark upgrade document.md
Current: resume-template@2
Latest:  resume-template@3.0

Warning: Major version change (2 → 3) may contain breaking changes.
Proceed? [y/N]

# Upgrade within same major version
$ quillmark upgrade document.md --minor
Current: resume-template@2.1
Latest:  resume-template@2.2 (within major version 2)
Proceed? [Y/n]
```

The upgrade command helps users transition documents to newer template versions with appropriate warnings about compatibility.

---

## Error Messages and Diagnostics

When version resolution fails, the system provides actionable error messages with context:

```
Error: Version not found
  Template: resume-template
  Requested: @2.3
  Available: 3.0, 2.2, 2.1, 2.0, 1.3, 1.2, 1.1, 1.0

  Suggestion: Use @2 for latest 2.x (currently 2.2), or specify @2.2
```

When a major version upgrade is attempted:

```
Warning: Major version change detected
  Current: resume-template@2.2
  Upgrading to: resume-template@3.0

  Major version changes may include breaking changes that affect rendering.
  Review the changelog before proceeding.
```

---

## Migration Strategy

This is a breaking change for the Quill.toml format. Pre-1.0 software, pre-1.0 rules.

### All Quills Must Declare Versions

Every Quill.toml must include a `version` field. Add `version = "1.0"` to all existing templates as a starting point.

### Documents Work Without Changes

Existing documents without version specifications continue to work, automatically using the latest available version. This provides zero-friction migration for document authors.

### Optional: Pin Documents to Current Version

Provide a migration tool to bulk-pin documents to their current rendering version:

```bash
# Pin all documents to exact version currently in use
$ quillmark pin --all ./docs

# Pin documents to major version (allow minor updates)
$ quillmark pin --all --major ./docs

# Interactive pinning with preview
$ quillmark pin --all --interactive ./docs
```

This is optional—documents continue working without pinning, but pinning ensures reproducibility.

---

## Future Extensions

The core versioning system enables several future enhancements:

### Remote Repositories

Version specification works naturally with remote template repositories. The syntax could be extended to include repository URLs or aliases:

```yaml
QUILL: "https://quills.example.com/resume-template@2.1"
QUILL: "github:user/repo/resume-template@2.1"
```

The engine would fetch and cache the specified version on demand. The resolution logic remains unchanged—only the source of available versions differs.

### Dependency Resolution

Templates could declare dependencies on other templates or on specific Quillmark versions:

```toml
[Quill]
name = "resume-template"
version = "2.1"
min_quillmark = "0.30"

[dependencies]
common-styles = "1.2"  # Requires exactly 1.2
utility-functions = "2"  # Any 2.x version
```

The engine would resolve the dependency tree and ensure all constraints are satisfied before rendering.

### Lockfiles

For reproducibility across machines, projects could use lockfiles that record exact versions:

```toml
# quillmark.lock
[documents."resume.md"]
quill = "resume-template"
resolved_version = "2.1"
rendered_at = "2026-01-21T10:30:00Z"

[documents."cover-letter.md"]
quill = "letter-template"
resolved_version = "1.3"
rendered_at = "2026-01-21T10:31:15Z"
```

The lockfile pins versions without modifying document source files, similar to package-lock.json in Node.js or Cargo.lock in Rust.

### Version Ranges (Advanced)

If future use cases require more complex constraints beyond major-only resolution, the system could be extended to support range syntax:

```yaml
QUILL: "resume-template@>=2.1"    # Any version >= 2.1
QUILL: "resume-template@2.1..2.5" # Between 2.1 and 2.5
```

This would enable more sophisticated compatibility specifications but is not necessary for the initial implementation. The simple major/exact syntax handles the vast majority of real-world needs.

---

## Implementation Checklist

1. **Version data structures.** Define Version type (major.minor), VersionSelector enum, QuillReference, and VersionedQuillSet.

2. **Version parsing.** Parse QUILL tags to extract name and version selector. Handle all selector types (exact, major, latest, unspecified).

3. **Resolution algorithm.** Implement version resolution logic with proper semantic ordering and error cases.

4. **Engine integration.** Add version registry to Quillmark engine. Update Quill registration to read and store versions.

5. **Workflow creation.** Modify document rendering to parse QUILL tags and resolve versions before creating workflows.

6. **CLI commands.** Implement `versions` (list), `resolve` (show resolution), `pin` (add version to document), `upgrade` (update version).

7. **Error handling.** Clear, actionable error messages for version not found and invalid version formats.

8. **Template migration.** Add `version = "1.0"` to all existing Quills in the repository.

---

## Open Questions

### Version Validation

Should the system validate that template authors follow versioning guidelines correctly? This is challenging—determining whether a change is "breaking" requires semantic understanding of template changes. The system cannot enforce this automatically. Rely on documentation, conventions, and community norms.

### Backend Compatibility

Different versions of a template might target different backend versions. Could add `min_backend_version` or `min_quillmark` fields to Quill.toml for validation.

### Multi-Repository Coordination

When templates are distributed across multiple repositories with overlapping names, use namespacing like `@org/template@version`. Defer until remote distribution is implemented.

---

## Why Two-Segment Versioning?

Traditional semantic versioning uses three segments (`MAJOR.MINOR.PATCH`), but this adds unnecessary complexity for Quill templates:

**Two segments are sufficient because:**

1. **Templates evolve differently than libraries.** Software libraries ship frequent patch releases for security fixes and bugs. Templates change less frequently and the distinction between "bug fix" and "feature" is less meaningful—a spacing fix and a new section are both just "improvements that don't break documents."

2. **Simpler mental model.** Users only need to understand one question: "Will this break my document?" If yes → major bump. If no → minor bump. No need to distinguish patch vs minor.

3. **Fewer version proliferation.** Avoids accumulating `2.1.0`, `2.1.1`, `2.1.2`, `2.1.3` for trivial changes. Each release is intentional.

4. **Precedent exists.** Go modules, many Python packages (Django uses `4.2`, `4.3`), and other systems successfully use two-segment versioning.

5. **Cleaner version strings.** `2.1` is easier to read and communicate than `2.1.3`.

The system can always be extended to support three segments in the future if needed, but starting simple reduces implementation complexity and user cognitive load.

---

## Benefits

1. **Opt-in reproducibility.** Documents that need stability can pin to specific versions. Documents that prefer convenience automatically get the latest.

2. **Zero-friction migration.** Existing documents continue working without modification—they simply use the latest version.

3. **Safe template evolution.** Authors can iterate aggressively knowing users can pin versions if needed.

4. **Clear compatibility signaling.** Two-segment versioning communicates breaking vs. compatible changes.

5. **User control.** Document authors choose their stability level: bleeding edge (unversioned/`@latest`), stable (major version like `@2`), frozen (exact version like `@2.1`).

6. **Foundation for distribution.** The versioning system enables template repositories, dependency management, and publishing workflows.

7. **Simple and understandable.** Two-segment versions are easy to explain to non-technical users while still being semantically meaningful.

8. **No legacy baggage.** Pre-1.0 means we can design it right the first time without compromise.
