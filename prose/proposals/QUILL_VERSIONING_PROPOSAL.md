# Quill Versioning System Proposal
## Enabling Reproducible Rendering with Version Pinning

**Date:** 2026-01-21
**Status:** Pre-1.0 breaking change - no backward compatibility required
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

1. **Explicit versioning required.** All documents must specify a version. No implicit defaults, no guessing. Reproducibility is mandatory.

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
QUILL: "resume-template@latest"   # Latest overall
---
```

### Parsing Rules

The parser splits on `@` to extract the template name and version constraint. Version specification is mandatory—documents without `@version` are rejected with a clear error.

The version constraint is parsed as:

- **Two components (2.1)** — Exact version match required.
- **One component (2)** — Match latest minor version in the 2.x series.
- **Keyword "latest"** — Match the highest version available.

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
}

pub struct Version {
    major: u64,
    minor: u64,
}

pub enum VersionSelector {
    Exact(u64, u64),     // 2.1 (major, minor)
    Major(u64),          // 2 (any minor)
    Latest,
}

pub struct QuillReference {
    name: String,
    version: VersionSelector,
}
```

### Parsing Implementation

```rust
impl QuillReference {
    pub fn parse(quill_tag: &str) -> Result<Self> {
        let (name, version_str) = quill_tag.split_once('@')
            .ok_or_else(|| Error::MissingVersion {
                quill_tag: quill_tag.to_string(),
                hint: "Specify a version like 'template@2.1' or 'template@latest'",
            })?;

        let version = match version_str {
            "latest" => VersionSelector::Latest,
            v => Self::parse_version_selector(v)?,
        };

        Ok(QuillReference {
            name: name.to_string(),
            version
        })
    }

    fn parse_version_selector(s: &str) -> Result<VersionSelector> {
        let parts: Vec<&str> = s.split('.').collect();

        match parts.len() {
            1 => {
                // Major only: "2"
                let major = parts[0].parse()
                    .map_err(|_| Error::InvalidVersion(s.to_string()))?;
                Ok(VersionSelector::Major(major))
            }
            2 => {
                // Full version: "2.1"
                let major = parts[0].parse()?;
                let minor = parts[1].parse()?;
                Ok(VersionSelector::Exact(major, minor))
            }
            _ => Err(Error::InvalidVersion(s.to_string()))
        }
    }
}
```

### Resolution Implementation

```rust
impl VersionedQuillSet {
    pub fn resolve(&self, selector: &VersionSelector) -> Result<&Quill> {
        match selector {
            VersionSelector::Exact(major, minor) => {
                let version = Version { major: *major, minor: *minor };
                self.versions.get(&version)
                    .ok_or_else(|| Error::VersionNotFound {
                        name: self.name.clone(),
                        requested: format!("{}.{}", major, minor),
                        available: self.list_versions(),
                    })
            }

            VersionSelector::Latest => {
                self.versions.values().last()
                    .ok_or_else(|| Error::NoVersionsAvailable(self.name.clone()))
            }

            VersionSelector::Major(major) => {
                // Find latest minor version matching major
                self.versions.iter()
                    .rev()
                    .find(|(v, _)| v.major == *major)
                    .map(|(_, quill)| quill)
                    .ok_or_else(|| Error::NoMatchingVersion {
                        name: self.name.clone(),
                        constraint: format!("{}.x", major),
                        available: self.list_versions(),
                    })
            }
        }
    }
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
Parse to QuillReference { name: "resume-template", version: Exact(2, 1) }
    ↓
Resolve against available versions → Version { major: 2, minor: 1 }
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

When version specification is missing, the system fails immediately with a clear error:

```
Error: Missing version specification
  Document: document.md
  QUILL tag: "resume-template"

  All documents must specify a version. Update to:
    QUILL: "resume-template@2.1"  (exact version)
    QUILL: "resume-template@2"    (latest 2.x)
    QUILL: "resume-template@latest" (latest overall)
```

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

This is a breaking change. Pre-1.0 software, pre-1.0 rules.

### All Quills Must Declare Versions

Add `version = "1.0"` to every Quill.toml. Start at 1.0 for all existing templates.

### All Documents Must Specify Versions

Add version to every QUILL tag. Provide migration tool:

```bash
# Scan and fix all documents to use @latest
$ quillmark migrate fix --version latest ./docs

# Or interactively choose version for each template
$ quillmark migrate fix --interactive ./docs
```

### Timeline

Ship it. Update all templates and examples in the same release. Documents without versions fail with clear error messages pointing to the migration tool.

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

1. **Version parsing.** Implement `QuillReference::parse` and `VersionSelector` enum. Require `@version` in all QUILL tags.

2. **Resolution algorithm.** Implement `VersionedQuillSet::resolve` with tests for all resolution cases.

3. **Engine integration.** Update `Quillmark` to use `VersionedQuillSet`. Enforce version requirement in Quill.toml.

4. **Workflow creation.** Update `workflow_for_document` to parse and resolve versions. Fail hard on missing versions.

5. **CLI commands.** Add `versions`, `resolve`, `pin`, `upgrade`, and `migrate fix` commands.

6. **Error handling.** Clear error messages for missing versions and failed resolution.

7. **Migration tool.** Build `quillmark migrate fix` to bulk-update documents.

8. **Template migration.** Add `version = "1.0"` to all existing Quills. Update all example documents.

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

1. **Mandatory reproducibility.** Every document specifies its version. No surprises, no silent breakage. Documents render identically forever.

2. **Safe template evolution.** Authors can iterate aggressively knowing existing documents are protected by explicit versioning.

3. **Clear compatibility signaling.** Two-segment versioning communicates breaking vs. compatible changes.

4. **User control.** Document authors choose their stability level: bleeding edge (`@latest`), stable (major version like `@2`), frozen (exact version like `@2.1`).

5. **Foundation for distribution.** The versioning system enables template repositories, dependency management, and publishing workflows.

6. **Simple and understandable.** Two-segment versions are easy to explain to non-technical users while still being semantically meaningful.

7. **No legacy baggage.** Pre-1.0 means we can design it right the first time without compromise.
