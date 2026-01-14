# Codebase Audit Plan: Redundant, Legacy, and Leftover Code

**Goal**: Procedurally audit the quillmark codebase to identify and document redundant, legacy, or leftover code. This plan uses a divide-and-conquer approach with general-purpose agents orchestrating completion of various phases.

**Scope**: All Rust crates, bindings, documentation, and supporting files in the repository.

---

## Overview

This audit plan is designed to be executed by a general-purpose agent that orchestrates specialized sub-tasks. Each phase can be executed independently, and findings are consolidated into a final report.

### Audit Categories

1. **Dead Code**: Functions, methods, types, modules, and imports that are defined but never used
2. **Legacy Code**: Deprecated patterns, outdated APIs, or code marked for removal
3. **Leftover Code**: Orphaned files, unused fixtures, stale configuration, and abandoned features

---

## Phase 1: Dead Code Analysis

**Objective**: Identify code that is defined but never called or referenced.

### Task 1.1: Unused Functions and Methods

**Agent Prompt**:
```
Analyze the quillmark repository for unused functions and methods. 

Steps:
1. Run `cargo clippy --workspace --all-features -- -W dead_code -W unused` to get Rust's dead code warnings
2. For each crate, identify public functions that are not exported or called
3. Check for private functions that are never called within their module
4. Document findings with file paths and line numbers

Focus on these crates:
- crates/core/src/
- crates/quillmark/src/
- crates/backends/typst/src/
- crates/backends/acroform/src/
- crates/bindings/cli/src/
- crates/bindings/python/src/
- crates/bindings/wasm/src/

Output a markdown list of potentially unused functions with justification.
```

### Task 1.2: Unreachable Code Paths

**Agent Prompt**:
```
Identify unreachable code paths in the quillmark repository.

Steps:
1. Search for `unreachable!()`, `todo!()`, `unimplemented!()` macros
2. Check for match arms that can never be reached
3. Look for conditional branches with impossible conditions
4. Identify cfg-gated code for non-existent features

Report format:
- File path and line number
- Type of unreachable code
- Recommendation (keep, remove, or investigate)
```

### Task 1.3: Unused Imports and Dependencies

**Agent Prompt**:
```
Audit imports and dependencies in the quillmark workspace.

Steps:
1. Run `cargo machete` to find unused dependencies (install if needed)
2. Check each Cargo.toml for dependencies not used in src files
3. Look for `use` statements that import unused items
4. Verify all workspace dependencies in root Cargo.toml are used

Output: List of unused dependencies and imports with removal recommendations.
```

---

## Phase 2: Legacy Code Detection

**Objective**: Find deprecated patterns, outdated constructs, and code marked for removal.

### Task 2.1: TODO/FIXME/HACK Comments

**Agent Prompt**:
```
Find all TODO, FIXME, HACK, XXX, and similar comments in the codebase.

Steps:
1. Search for patterns: TODO, FIXME, HACK, XXX, DEPRECATED, REMOVE, TEMP, TEMPORARY
2. Categorize by urgency and type
3. Check if any reference resolved issues or outdated requirements

Search in:
- All .rs files in crates/
- All .md files in docs/ and prose/
- All .typ files in crates/backends/typst/
- All .py files in crates/bindings/python/

Output: Categorized list with context and recommendations.
```

### Task 2.2: Deprecated Function Usage

**Agent Prompt**:
```
Identify usage of deprecated functions and APIs in quillmark.

Steps:
1. Search for #[deprecated] attributes in the codebase
2. Check if deprecated items are still being used
3. Look for deprecated patterns in dependencies (check Cargo.lock warnings)
4. Identify Rust edition-specific deprecated patterns

Focus on:
- Deprecated methods in std library
- Deprecated APIs from dependencies
- Self-deprecated functions still in use
```

### Task 2.3: Outdated Patterns and Idioms

**Agent Prompt**:
```
Find outdated Rust patterns and idioms that should be modernized.

Look for:
1. `.unwrap()` calls that should use `?` operator
2. Manual `impl` blocks that could use derive macros
3. Old-style error handling (pre-thiserror patterns)
4. Unnecessary `clone()` calls
5. Box<dyn Error> instead of concrete error types
6. Verbose pattern matching that could use if-let or let-else

Report: Specific instances with modernization suggestions.
```

---

## Phase 3: Leftover/Orphan Code Discovery

**Objective**: Find files, modules, and assets that are no longer referenced or needed.

### Task 3.1: Orphaned Source Files

**Agent Prompt**:
```
Find source files that are not included in any module tree.

Steps:
1. List all .rs files in crates/
2. Check each file is either:
   - Referenced in a lib.rs or mod.rs
   - A binary entry point (main.rs)
   - A test file in tests/
3. Identify files that exist but are not part of the module tree

Also check:
- Orphaned .typ files in backends/typst/
- Orphaned .py files in bindings/python/
- Orphaned test files not run by test harness
```

### Task 3.2: Unused Test Fixtures

**Agent Prompt**:
```
Audit test fixtures in crates/fixtures/resources/ for usage.

Steps:
1. List all directories and files in crates/fixtures/resources/
2. Search the codebase for references to each fixture
3. Identify fixtures that are:
   - Never referenced in tests
   - Referenced but tests are disabled
   - Duplicates of other fixtures

Output: List of potentially unused fixtures with verification status.
```

### Task 3.3: Unused Assets and Configuration

**Agent Prompt**:
```
Find unused assets and configuration files.

Check for:
1. Fonts in assets/ directories not loaded by code
2. Template files not referenced
3. Example files not run or documented
4. Configuration files for removed features

Search directories:
- crates/backends/typst/assets/
- crates/backends/typst/default_quill/
- crates/fixtures/
- scripts/
```

---

## Phase 4: Documentation Audit

**Objective**: Ensure documentation references valid, existing code.

### Task 4.1: Stale Documentation References

**Agent Prompt**:
```
Verify that documentation references existing code.

Steps:
1. Extract all code references from docs/*.md (function names, type names, paths)
2. Extract all code references from prose/designs/*.md
3. Verify each reference exists in the codebase
4. Check for broken internal links in markdown files

Report: List of stale references with current status.
```

### Task 4.2: Outdated Examples

**Agent Prompt**:
```
Verify all examples in documentation work with current API.

Steps:
1. List all code examples in README.md
2. List all examples in docs/ folder
3. Compare example API usage against current signatures
4. Identify examples using deprecated or removed APIs

Focus on:
- Code blocks in markdown files
- Example files in crates/quillmark/examples/
- Example usage in docstrings
```

### Task 4.3: Design Document Review

**Agent Prompt**:
```
Review design documents for obsolete content.

Check prose/designs/*.md for:
1. References to removed features
2. Outdated architecture descriptions
3. Implementation notes that no longer match code
4. TODOs that have been completed or abandoned

Compare against actual implementation in crates/.
```

---

## Phase 5: Cross-Reference Analysis

**Objective**: Analyze module and feature relationships for unused exports and features.

### Task 5.1: Unused Exports

**Agent Prompt**:
```
Find public exports that are never imported externally.

Steps:
1. List all `pub` items in each crate's lib.rs
2. Search for imports of each item outside its crate
3. Identify pub items only used internally (could be pub(crate))
4. Check re-exports in parent modules

Focus on:
- crates/core: What's exported but unused by quillmark?
- crates/quillmark: What's exported but unused by bindings?
```

### Task 5.2: Feature Flag Audit

**Agent Prompt**:
```
Audit Cargo feature flags for unused or incomplete features.

Steps:
1. List all features defined in each Cargo.toml
2. Search for #[cfg(feature = "...")] usage
3. Identify features that:
   - Are defined but never used
   - Are partially implemented
   - Are never enabled in CI or defaults

Check all crates in the workspace.
```

### Task 5.3: Circular and Unused Module Dependencies

**Agent Prompt**:
```
Analyze module dependencies for issues.

Steps:
1. Map `use` statements between modules
2. Identify circular dependencies (A uses B, B uses A)
3. Find modules that import but don't use items
4. Check for over-broad imports (use crate::* or mod::*)

Focus on inter-crate dependencies in the workspace.
```

---

## Execution Instructions

### Running the Audit

1. **Sequential Execution**: Run phases 1-5 in order, as later phases may depend on earlier findings.

2. **Per-Phase Orchestration**: For each phase, spawn general-purpose agent tasks for each sub-task (1.1, 1.2, etc.).

3. **Consolidation**: After each phase, consolidate findings into a single phase report.

### Agent Orchestration Pattern

For each task, use the general-purpose agent with this pattern:

```
task(
  agent_type: "general-purpose",
  description: "Phase X.Y: Task Name",
  prompt: "<task-specific prompt from above>"
)
```

### Output Format

Each agent should output findings in this format:

```markdown
## Findings: [Task Name]

### Summary
- Total items found: N
- High priority: X
- Medium priority: Y  
- Low priority: Z

### High Priority
| Location | Issue | Recommendation |
|----------|-------|----------------|
| file:line | description | action |

### Medium Priority
...

### Low Priority
...
```

---

## Final Report Structure

After all phases complete, compile into:

```markdown
# Quillmark Codebase Audit Report

## Executive Summary
- Total issues found: N
- Estimated cleanup effort: X hours
- Risk assessment: Low/Medium/High

## Phase Summaries
### Phase 1: Dead Code
...
### Phase 2: Legacy Code
...
### Phase 3: Leftover Code
...
### Phase 4: Documentation
...
### Phase 5: Cross-References
...

## Recommended Actions
1. Immediate (blocking issues)
2. Short-term (technical debt)
3. Long-term (nice to have)

## Appendix: Detailed Findings
...
```

---

## Success Criteria

The audit is complete when:

- [ ] All 5 phases have been executed
- [ ] Each sub-task (15 total) has produced findings
- [ ] Findings have been consolidated into a single report
- [ ] Recommendations are prioritized
- [ ] Report is saved to `prose/plans/CODEBASE_AUDIT_REPORT.md`

---

## Notes

- This audit is non-destructive; it only identifies issues
- Actual code removal should be done in separate, focused PRs
- Some findings may be false positives requiring human review
- Consider running this audit periodically (e.g., quarterly)
