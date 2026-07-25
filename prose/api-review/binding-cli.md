# Binding: CLI

## Surface

Binary `quillmark` (`crates/bindings/cli/src/main.rs`), five subcommands, `clap` derive, `#[command(version)]` for global `--version`/`-V`, `--help`/`-h` free on every level.

| Command | Positional args | Flags | Default | Stdout | Stderr | Notes |
|---|---|---|---|---|---|---|
| `render` | `QUILL_PATH` (required), `MARKDOWN_FILE` (optional; seeded doc if omitted) | `-o/--output <FILE>`, `-f/--format <FORMAT>` (pdf\|svg\|png\|txt), `--stdout`, `--output-data <DATA_FILE>`, `-v/--verbose`, `--quiet` | format=pdf, output=derived from input filename or `example.<ext>` | rendered bytes (`--stdout`) or "Output written to:" | diagnostics, warnings | `txt` is accepted but no backend implements it (see Findings) |
| `schema` | `QUILL_PATH` | `-o/--output <FILE>` | none | YAML schema (no `-o`) | errors only | |
| `blueprint` | `QUILL_PATH` | `-o/--output <FILE>` | none | Markdown blueprint (no `-o`) | errors only | |
| `validate` | `QUILL_PATH` | `-v/--verbose` | none | pass/fail summary | issue list, errors | not `--json`; hand-rolled diagnostic model, not core `Diagnostic` |
| `info` | `QUILL_PATH` | `--json` | human-readable | metadata (text or JSON) | errors only | only command with a machine-readable mode |

Exit codes: `0` success, `1` every failure (`CliError::{Io,Render,Parse,InvalidArgument}` all map to the same code in `main.rs:46-49`). No `--json`/structured mode for errors on any command — `errors::print_cli_error` (`crates/bindings/cli/src/errors.rs:48-64`) always calls `fmt_pretty()`/`print_errors()`, prose only, even though every `Diagnostic` is `serde::Serialize` (`ERROR.md` "Machine-readable" section).

`info --json` shape: `{name, backend, version?, author?, description?, field_count, card_count?, metadata?}` (`commands/info.rs:34-89`). No JSON schema is emitted for this shape (ad hoc `serde_json::Map` construction, not a typed struct).

Stdin/stdout: no subcommand reads from stdin or accepts `-` as a path; `render --stdout` writes raw bytes unconditionally, with no `is_terminal` guard.

## Findings

### `--format txt` is a dead value on every registered backend
Severity: High
`crates/bindings/cli/src/commands/render.rs:24-26` and `README.md:105`/`prose/canon/CLI.md:24` all advertise `pdf, svg, png, txt` as the four `-f/--format` choices, and `quillmark_core::OutputFormat::ALL` (`crates/core/src/types.rs:20-23`) does include `Txt`. But neither built-in backend supports it: `crates/backends/typst/src/lib.rs:44-45` (`SUPPORTED_FORMATS = [Pdf, Svg, Png]`) and its `compile.rs:184` (`OutputFormat::Txt => Err(...)`), and `crates/backends/pdfform/src/lib.rs:50-51` (`SUPPORTED_FORMATS = [Pdf, Svg, Png]`). `quillmark render --format txt` will fail on every quill shipped with this workspace's two default backends. A user typing a value straight from `--help` or the README gets a backend compile error instead of a working render. Either drop `txt` from the CLI's advertised/help text until a backend implements it, or gate it behind `Quillmark::supported_formats(&quill)` (see next finding) so the failure is a fast, clear argument error.

### No CLI path to `Quillmark::supported_formats`/`registered_backends` — format errors surface late and opaque
Severity: Medium
`crates/quillmark/src/orchestration/engine.rs:106` (`supported_formats`) and `:51` (`registered_backends`) exist on the engine the CLI already constructs (`render.rs:136`), but no command calls them. `render` validates `--format` only by parsing the string into `OutputFormat` (`render.rs:102-105`), which succeeds for `txt` even though no backend supports it — the actual rejection happens deep in the backend's `render()` call after the quill is loaded and the markdown is parsed. `info` would be the natural place to add `--formats`/a `supported_formats` field so a user (or a script) can ask "what can I even render this to" without attempting a doomed render.

### `validate` reinvents `Diagnostic` instead of reusing it
Severity: Medium
`crates/bindings/cli/src/commands/validate.rs:20-47` defines a local `Severity`/`ValidationIssue`/`ValidationResult` that shadows the core `Diagnostic`/`Severity` model documented in `ERROR.md`. Worse, at `validate.rs:129-131` the *actual* `Diagnostic`s returned by `QuillConfig::from_yaml_with_warnings` are downgraded to bare strings (`result.add_warning(diag.message.clone())`), discarding `code`, `location`, and `hint` for every config-level warning — only the hard-error path (`validate.rs:115-126`) still calls `diag.fmt_pretty()` and keeps full detail. A YAML warning that core anchored to a line/column prints with no location once it reaches this command. This also means `validate` cannot get a `--json` mode for free the way `info` could (there's no `Serialize` diagnostic list to hand out) without a rewrite.

### `--quiet` does not suppress `--verbose` output in `render`
Severity: Medium
`render.rs:46,53,68,78,91,97,107` gate progress lines on `if args.verbose` alone; only two later sites (`render.rs:130,176`) additionally check `!args.quiet`. `quillmark render quill doc.md --verbose --quiet` still prints "Loading quill from: …", "Quill loaded: …", "Reading markdown from: …", etc. — contradicting both the README ("Quiet mode (suppress all non-error output)") and CLI.md's identical framing. `--quiet` should win over `--verbose` consistently, or the two flags should be declared mutually exclusive via clap (`conflicts_with`).

### `--output`/`-o` is silently discarded when `--stdout` is also passed
Severity: Low
`render.rs:161-171`: `if args.stdout { None } else { Some(args.output.unwrap_or_else(...)) }` — `--stdout` always wins with no warning. `quillmark render q doc.md -o out.pdf --stdout > x.pdf` silently ignores `-o`. Neither flag is declared `conflicts_with` the other in the `clap::Parser` derive (`render.rs:20-30`), so clap can't catch the contradiction either. A `conflicts_with("stdout")` on `output`, or at minimum a stderr note when both are set, would remove the surprise.

### `render --stdout` writes binary to stdout with no TTY guard
Severity: Low
`output.rs:23-26` calls `io::stdout().write_all(bytes)` unconditionally when `use_stdout` is true; nothing checks `std::io::IsTerminal`. `quillmark render quill doc.md --stdout` run interactively (forgetting the redirect) dumps raw PDF/PNG bytes to the terminal instead of refusing with a "refusing to write binary to a terminal, use `--stdout | prog` or `-o file`" message — the Unix-idiom convention most binary-emitting CLIs (e.g. `git diff --binary`, `curl` without `-o`) follow. No `is-terminal`/`atty` dependency is present in `Cargo.toml` to support this today.

### No `-` / stdin support for `MARKDOWN_FILE`
Severity: Low
`render.rs:16-18` types `markdown_file: Option<PathBuf>` and the existence check at `render.rs:61` (`markdown_path.exists()`) has no special case for `-`. Every other verb in the workspace treats markdown as a string the caller already holds (`Document::parse`), so piping generated markdown into `quillmark render quill -` (a common pattern for scripts that generate card-yaml on the fly) isn't possible — the caller must always write a temp file first.

## Cross-cutting

- The CLI's `render --output-data` (`render.rs:112-133`) calls `quill.compile_data(&parsed)` (`crates/core/src/quill/compose.rs:20`) directly rather than going through `quillmark::Quillmark`; this mirrors what the WASM/Python bindings do for their own intermediate-data hooks, so it's consistent with the workspace's binding pattern rather than a CLI-only duplication — noted for the `quillmark`/`core` reviewers as a data point on where `compile_data` is consumed outside tests.
- `validate`'s hand-rolled diagnostic model (see Finding above) is CLI-only; the `core`/`quillmark` reviewers should confirm there's no shared "lint a Quill.yaml and hand back `Vec<Diagnostic>`" helper the CLI could call instead of re-deriving `Diagnostic.message` into custom structs — if core added one, this whole command could shrink to formatting.
- `LiveSession`/`Quillmark::open` (interactive canvas preview, `crates/quillmark/src/orchestration/engine.rs:77`) has no CLI verb — this matches `BINDINGS.md`'s explicit "canvas preview is WASM-only" statement, so it is not treated as a gap here.
- The `txt` dead-format finding is really a `core`/backend-level fact (`OutputFormat::ALL` includes a variant neither shipped backend implements) that the CLI merely surfaces to users first, since it's the only binding that free-text-lists format choices in `--help`; worth flagging to whichever reviewer owns `crates/core/src/types.rs` and the backend crates.
