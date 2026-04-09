# Proposal: Documentation Overhaul

## Problem

The current documentation is organized by topic (`guides/`, `advanced/`, `python/`, `javascript/`) but serves fundamentally different audiences with conflicting needs. The `guides/` section is a catch-all that mixes template authoring tutorials, Markdown syntax reference, developer integration how-tos, and CLI documentation. A template designer looking for `Quill.yaml` field types has to navigate past dynamic asset injection. A Python developer looking for validation patterns lands next to Typst backend internals.

The result: no reader has a clear path through the docs. Everyone wades through material that isn't for them.

### Current structure

```
docs/
├── index.md                        # Landing page
├── getting-started/
│   ├── quickstart.md               # Multi-language tabs (Python, Rust, JS)
│   └── concepts.md                 # Core philosophy and pipeline
├── guides/
│   ├── creating-quills.md          # Tutorial: building templates
│   ├── quill-markdown.md           # Reference: extended Markdown syntax
│   ├── dynamic-assets.md           # How-to: runtime asset injection
│   ├── validation.md               # How-to: dry_run validation
│   ├── cli.md                      # Reference: CLI commands
│   ├── typst-backend.md            # Tutorial + reference: Typst
│   └── quill-yaml-reference.md     # Reference: complete YAML spec
├── advanced/
│   └── architecture.md             # System internals
├── python/
│   └── api.md                      # Python API reference
└── javascript/
    └── api.md                      # JavaScript/WASM API reference
```

### Specific problems

1. **`guides/` conflates four doc types.** Tutorials (creating-quills), reference (quill-yaml-reference, cli), how-to guides (dynamic-assets, validation), and conceptual explanation (typst-backend) are all peers in the same nav section.

2. **Audience mismatch.** A content author writing Markdown doesn't need Typst backend internals. A developer integrating via Python doesn't need `Quill.yaml` field schema details. Everyone sees everything.

3. **Rust documentation on the docs site is wasted effort.** The quickstart has a full Rust tab and Rust code examples throughout, but Rust users expect and prefer docs.rs. Maintaining Rust examples in the user docs is duplicated work that will drift.

4. **No content authoring section.** The largest audience — people writing Markdown documents against existing templates — has no dedicated section. Markdown syntax is buried in `guides/`.

5. **Architecture doc is misplaced.** `advanced/architecture.md` is contributor documentation living in the user-facing docs site.

6. **Landing page doesn't route by persona.** `index.md` lists features and project structure. It should help readers self-select into the right path.

## Audiences

Quillmark has four distinct consumer personas. Each needs different content in a different order.

### 1. Content Authors

People who write Markdown documents using existing Quill templates. Least technical audience. May be non-developers (writers, analysts, subject matter experts).

**Need:** Markdown syntax, YAML frontmatter rules, how CARD blocks work, field types available in their template, example documents.

**Don't need:** How to build templates, API references, architecture.

### 2. Template Designers

People who create and maintain Quill templates. They author `Quill.yaml` configs and Typst plate files. May or may not write application code.

**Need:** `Quill.yaml` reference, Typst backend guide, field schema types, versioning, asset bundling, example quills.

**Don't need:** Python/JS API details, CLI scripting patterns.

### 3. Application Developers

People integrating Quillmark into applications via Python or JavaScript APIs. They consume templates — they don't build them.

**Need:** Installation, API reference for their language, error handling, validation, dynamic assets, performance guidance.

**Don't need:** Markdown syntax details, `Quill.yaml` authoring, Typst internals.

### 4. CLI / DevOps Users

People using the CLI for batch rendering, CI/CD validation, or scripting.

**Need:** CLI command reference, exit codes, environment variables, scripting patterns.

**Don't need:** API references, template authoring.

### Non-audience: Rust developers

Rust users have docs.rs with auto-generated API documentation, inline examples, and type signatures. The docs.rs experience is superior to anything we'd maintain by hand. We should link to docs.rs from the landing page and remove Rust content from the user docs site entirely — no Rust tab in quickstart, no Rust examples in guides.

### Non-audience (for this site): Contributors

Architecture and internals documentation belongs in `prose/designs/`, not in the user-facing docs. The architecture page should be removed from the docs site. Contributors read the repo directly.

## Decisions

### 1. Reorganize by persona, not by topic

The top-level navigation becomes persona-oriented sections. Each section contains a coherent reading path for one audience.

### 2. Drop Rust from the docs site

Remove the Rust tab from quickstart. Remove Rust examples from all guides. Add a single prominent link to docs.rs on the landing page. Rust users are well-served by the ecosystem's native tooling.

### 3. Remove architecture from user docs

Move `advanced/architecture.md` out of the docs site. It already has a natural home in `prose/designs/`. The `advanced/` section is eliminated entirely.

### 4. Persona-routing landing page

Rewrite `index.md` to route readers by role: "I'm writing documents", "I'm building templates", "I'm integrating into my app", "I'm using the CLI". Each path links directly into the relevant section.

### 5. Single site, not per-language sites

Python and JavaScript stay in the same docs site under a shared `integration/` section. Rationale:

- ~80% of content is language-agnostic (concepts, template authoring, Markdown syntax). Splitting duplicates or fragments this.
- The API surfaces mirror each other (parse → register → render). Shared how-to guides with tabbed code samples serve both audiences.
- Users integrating both (Python backend + JS frontend) need one site, not two.

### 6. Quill Markdown guide splits into authoring section

`quill-markdown.md` currently covers both basic Markdown syntax and extended YAML metadata (CARD blocks, QUILL key, version selectors). Split it:

- Basic Markdown syntax and YAML frontmatter → `authoring/` section for content authors.
- CARD block definitions and schema interaction → `templates/` section for template designers.

## Target Structure

```
docs/
├── index.md                            # Persona-routing landing page
│
├── getting-started/
│   ├── quickstart.md                   # Python + JS tabs only. No Rust.
│   └── concepts.md                     # Core philosophy, pipeline, mental model
│
├── authoring/                          # For Content Authors
│   ├── markdown-syntax.md              # Standard + extended Markdown
│   ├── yaml-frontmatter.md             # Frontmatter structure, field types, data types
│   └── cards.md                        # CARD blocks: syntax, usage, examples
│
├── templates/                          # For Template Designers
│   ├── creating-quills.md              # Tutorial: building a quill from scratch
│   ├── quill-yaml-reference.md         # Complete Quill.yaml specification
│   ├── typst-backend.md                # Typst plates, data access, packages, fonts
│   └── versioning.md                   # Template versioning and compatibility
│
├── integration/                        # For Application Developers
│   ├── overview.md                     # Shared workflow (parse → register → render),
│   │                                   #   error handling patterns, output formats
│   ├── python/
│   │   └── api.md                      # Python API reference (from current python/api.md)
│   ├── javascript/
│   │   └── api.md                      # JS/WASM API reference (from current javascript/api.md)
│   ├── dynamic-assets.md               # Runtime asset/font injection (tabbed: Python + JS)
│   └── validation.md                   # Dry-run validation, LLM loops (tabbed: Python + JS)
│
├── cli/                                # For CLI / DevOps Users
│   └── reference.md                    # Commands, options, exit codes, env vars, scripting
│
└── migration.md                        # Breaking changes, upgrade paths
```

### Navigation (mkdocs.yml)

```yaml
nav:
  - Home: index.md
  - Getting Started:
      - Quickstart: getting-started/quickstart.md
      - Concepts: getting-started/concepts.md
  - Writing Documents:
      - Markdown Syntax: authoring/markdown-syntax.md
      - YAML Frontmatter: authoring/yaml-frontmatter.md
      - Card Blocks: authoring/cards.md
  - Building Templates:
      - Creating Quills: templates/creating-quills.md
      - Quill.yaml Reference: templates/quill-yaml-reference.md
      - Typst Backend: templates/typst-backend.md
      - Versioning: templates/versioning.md
  - Integration:
      - Overview: integration/overview.md
      - Python API: integration/python/api.md
      - JavaScript API: integration/javascript/api.md
      - Dynamic Assets: integration/dynamic-assets.md
      - Validation: integration/validation.md
  - CLI:
      - Reference: cli/reference.md
  - Migration: migration.md
```

## Execution Plan

We are pre-release with no backwards compatibility concerns. Execute aggressively.

### Phase 1: Restructure and move files

Create the new directory structure. Move existing files to their new locations. This is purely mechanical — no content changes yet.

| Source | Destination | Notes |
|--------|------------|-------|
| `guides/quill-markdown.md` | Split → `authoring/markdown-syntax.md`, `authoring/yaml-frontmatter.md`, `authoring/cards.md` | Decompose by audience concern |
| `guides/creating-quills.md` | `templates/creating-quills.md` | Direct move |
| `guides/quill-yaml-reference.md` | `templates/quill-yaml-reference.md` | Direct move |
| `guides/typst-backend.md` | `templates/typst-backend.md` | Direct move |
| `guides/dynamic-assets.md` | `integration/dynamic-assets.md` | Direct move |
| `guides/validation.md` | `integration/validation.md` | Direct move |
| `guides/cli.md` | `cli/reference.md` | Direct move |
| `python/api.md` | `integration/python/api.md` | Direct move |
| `javascript/api.md` | `integration/javascript/api.md` | Direct move |
| `advanced/architecture.md` | Remove from docs site | Already covered in `prose/designs/` |

Delete the now-empty `guides/`, `python/`, `javascript/`, and `advanced/` directories.

### Phase 2: Remove Rust content

- Remove the `=== "Rust"` tab from `getting-started/quickstart.md`.
- Audit all docs files for Rust-specific code examples and remove them.
- Add a docs.rs link to the landing page under a "Rust Developers" callout.

### Phase 3: Split `quill-markdown.md`

The current `quill-markdown.md` covers three separable concerns. Decompose it:

- **`authoring/markdown-syntax.md`** — Standard CommonMark elements (headings, lists, links, code blocks, emphasis, blockquotes, horizontal rules). Target audience: content authors.
- **`authoring/yaml-frontmatter.md`** — Frontmatter structure, QUILL key, data types, version selectors, body content field. Target audience: content authors.
- **`authoring/cards.md`** — CARD block syntax (`~~~cardname` fences), inline YAML in card blocks, card ordering and nesting rules. Target audience: content authors who use templates with cards.

### Phase 4: Create new pages

- **`authoring/` section pages** — Written from the decomposition above, with a content-author voice. No code examples in Python/JS/Rust. Focus on Markdown and YAML.
- **`integration/overview.md`** — Extract the shared workflow pattern (parse → register → render) from the two API reference pages. Cover output formats, error handling philosophy, and common patterns. Tabbed code samples for Python + JS.
- **`templates/versioning.md`** — Extract versioning content currently scattered across `creating-quills.md` and `concepts.md` into a dedicated page.

### Phase 5: Rewrite landing page

Replace the current `index.md` (feature list + project structure) with persona routing:

```markdown
# Quillmark

A template-first Markdown rendering system.

## Choose your path

- **Writing documents?** You author Markdown content using existing templates.
  → [Markdown Syntax](authoring/markdown-syntax.md)

- **Building templates?** You create Quill templates that control rendering.
  → [Creating Quills](templates/creating-quills.md)

- **Integrating into an app?** You use Quillmark via Python or JavaScript.
  → [Integration Overview](integration/overview.md)

- **Using the CLI?** You render and validate from the command line.
  → [CLI Reference](cli/reference.md)

- **Using Rust?** API documentation is on [docs.rs](https://docs.rs/quillmark).
```

Keep the badges and project description above the routing. Remove the project structure listing (that's contributor info, not user info).

### Phase 6: Update mkdocs.yml and cross-references

- Replace the `nav:` section with the new structure.
- Find and fix all internal cross-references (`../guides/creating-quills.md` → `../templates/creating-quills.md`, etc.).
- Verify the build with `mkdocs build --strict` to catch broken links.

### Phase 7: Content pass

With the structure in place, do a content quality pass on each section:

- **Authoring section:** Ensure examples use realistic Markdown, not developer-oriented samples. Add a "your first document" mini-tutorial.
- **Templates section:** Ensure creating-quills.md is a true tutorial (step-by-step, builds up) rather than reference. The reference role belongs to quill-yaml-reference.md.
- **Integration section:** Add Python/JS tabbed examples to dynamic-assets.md and validation.md where currently only one language is shown. Remove any Rust examples.
- **CLI section:** Add CI/CD recipe examples (GitHub Actions, shell scripting patterns).

## Files Affected

| File | Change |
|------|--------|
| `mkdocs.yml` | Full nav rewrite, new directories |
| `docs/index.md` | Rewrite as persona-routing page |
| `docs/getting-started/quickstart.md` | Remove Rust tab |
| `docs/getting-started/concepts.md` | Remove Rust references, update cross-links |
| `docs/guides/quill-markdown.md` | Delete (split into 3 authoring pages) |
| `docs/guides/creating-quills.md` | Move to `templates/` |
| `docs/guides/quill-yaml-reference.md` | Move to `templates/` |
| `docs/guides/typst-backend.md` | Move to `templates/` |
| `docs/guides/dynamic-assets.md` | Move to `integration/`, remove Rust examples |
| `docs/guides/validation.md` | Move to `integration/`, remove Rust examples |
| `docs/guides/cli.md` | Move to `cli/reference.md` |
| `docs/python/api.md` | Move to `integration/python/api.md` |
| `docs/javascript/api.md` | Move to `integration/javascript/api.md` |
| `docs/advanced/architecture.md` | Remove from docs site |
| `docs/authoring/markdown-syntax.md` | **New** — extracted from quill-markdown.md |
| `docs/authoring/yaml-frontmatter.md` | **New** — extracted from quill-markdown.md |
| `docs/authoring/cards.md` | **New** — extracted from quill-markdown.md |
| `docs/integration/overview.md` | **New** — shared workflow patterns |
| `docs/templates/versioning.md` | **New** — extracted from scattered sources |
| `MIGRATION.md` | Move to `docs/migration.md` |

## Out of Scope

- Rust API documentation (handled by docs.rs)
- Contributor/architecture documentation (stays in `prose/designs/`)
- Framework-specific integration guides (FastAPI, Next.js, etc.) — future work
- Troubleshooting / FAQ page — future work
- API reference auto-generation from source — future work
