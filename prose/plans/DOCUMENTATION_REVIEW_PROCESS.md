# Documentation Review Process

> **Status**: Process Guide
>
> This document establishes the process for reviewing and maintaining documentation across the Quillmark project.

---

## Overview

The Quillmark project maintains three types of documentation:
1. **Design Documents** (`prose/designs/`) - Architecture and design decisions
2. **Plan Documents** (`prose/plans/`) - Implementation plans and roadmaps
3. **Debrief Documents** (`prose/debriefs/`) - Post-implementation reviews
4. **Inline Documentation** - Code comments, rustdoc, module documentation

This process ensures documentation stays aligned with the codebase and provides value to contributors.

---

## Review Triggers

Documentation should be reviewed when:

1. **Major Feature Implementation**
   - Design document updated BEFORE implementation
   - Debrief created AFTER implementation
   - Related design docs cross-referenced

2. **API Changes**
   - Public API changes documented in design docs
   - Examples updated to reflect new API
   - Breaking changes clearly marked

3. **Architecture Changes**
   - ARCHITECTURE.md updated
   - Affected design docs updated
   - Cross-references validated

4. **Bug Fixes** (if design-related)
   - Design assumptions reconsidered
   - Edge cases documented
   - Test coverage verified

5. **Quarterly Reviews**
   - All design docs verified against implementation
   - Outdated information identified and updated
   - Missing documentation identified

---

## Design Document Lifecycle

### 1. Creation Phase

**When:** Before implementing a major feature or making architectural changes

**Process:**
1. Create design document in `prose/designs/`
2. Include:
   - Problem statement
   - Design principles
   - Proposed architecture
   - Trade-offs and alternatives considered
   - Implementation guidance
3. Get review from at least one other contributor
4. Mark status as `Planning` or `Draft`

**Template:**
```markdown
# Feature Name

> **Status**: Draft
> **Created**: YYYY-MM-DD
> **Related**: [Other design docs]

## Problem Statement

## Design Principles

## Architecture

## Implementation Guidance

## Alternatives Considered

## Cross-References
```

### 2. Implementation Phase

**When:** During feature implementation

**Process:**
1. Reference design document in code comments
2. Update design doc if implementation reveals issues
3. Keep design doc at high/medium level (not code-level details)
4. Mark status as `In Progress`

### 3. Completion Phase

**When:** After feature implementation

**Process:**
1. Create debrief in `prose/debriefs/`
2. Document what went well, challenges, deviations
3. Update design doc to match final implementation
4. Mark design doc status as `Final` or `Implemented`
5. Update cross-references

**Debrief Template:**
```markdown
# Feature Name Implementation Debrief

> **Status**: ✅ COMPLETED
> **Created**: YYYY-MM-DD
> **Related**: [Design doc link]

## Implementation Summary

## What Went Well

## Challenges & Solutions

## Deviations from Plan

## Files Changed

## Test Results

## Cross-References
```

### 4. Maintenance Phase

**When:** Ongoing

**Process:**
1. Keep design doc accurate with quarterly reviews
2. Update when related changes occur
3. Archive if feature is deprecated (move to `prose/archive/`)
4. Maintain cross-references

---

## Review Checklist

### For Design Documents

- [ ] Problem statement is clear
- [ ] Design principles are documented
- [ ] Architecture diagrams/descriptions are present
- [ ] Implementation guidance is actionable
- [ ] Cross-references to related docs are included
- [ ] Status field is accurate
- [ ] Examples are working and up-to-date
- [ ] Code references point to actual files (if applicable)

### For Code Documentation

- [ ] All public APIs have rustdoc comments
- [ ] Module-level documentation explains purpose
- [ ] Examples compile and run
- [ ] Complex algorithms are explained
- [ ] Panics and errors are documented
- [ ] Cross-references to design docs where appropriate

### For Test Documentation

- [ ] Test files have module-level documentation
- [ ] Test purpose is clearly stated
- [ ] Test strategy is explained
- [ ] Relationship to other tests is documented
- [ ] Coverage gaps are identified

---

## Documentation Standards

### Writing Style

1. **Clarity over Cleverness**: Write for understanding, not brevity
2. **Active Voice**: "The engine registers backends" not "Backends are registered"
3. **Present Tense**: "The filter processes values" not "The filter will process"
4. **Concrete Examples**: Show real code, not pseudocode (when possible)
5. **Cross-Reference**: Link to related documents and code

### Structure

1. **Top-Down**: Start with overview, then details
2. **Sections**: Use clear section headers
3. **Lists**: Use for sequential steps or related items
4. **Tables**: Use for comparisons or structured data
5. **Code Blocks**: Use with syntax highlighting

### Versioning

1. **Status Field**: Every design doc has status
   - `Draft` - Work in progress
   - `Planning` - Under discussion
   - `In Progress` - Being implemented
   - `Implemented` / `Final` - Matches current implementation
   - `Deprecated` - No longer relevant

2. **Created Date**: Record when document was created
3. **Last Updated**: Update when significant changes made (optional)
4. **Version Notes**: For major revisions, note what changed (optional)

---

## Review Roles

### Design Document Author
- Creates initial design
- Updates based on review feedback
- Maintains document during implementation
- Creates debrief after completion

### Reviewer
- Validates design makes sense
- Checks for conflicts with existing designs
- Verifies cross-references
- Suggests improvements

### Maintainer
- Conducts quarterly reviews
- Identifies outdated documentation
- Coordinates updates
- Archives deprecated docs

---

## Quarterly Review Process

**Schedule:** Every 3 months

**Process:**
1. Review all design documents in `prose/designs/`
2. For each document:
   - Verify status field is accurate
   - Check if implementation matches design
   - Validate cross-references
   - Update examples if needed
   - Mark issues for follow-up
3. Review plan documents in `prose/plans/`
   - Check completion status
   - Archive completed plans
4. Generate review report
5. Create issues for identified problems

**Review Report Template:**
```markdown
# Documentation Review - YYYY-MM-DD

## Documents Reviewed: [count]

## Issues Found:
- Document X: Outdated API examples
- Document Y: Missing cross-references

## Action Items:
- [ ] Update Document X examples
- [ ] Add cross-refs to Document Y
- [ ] Archive completed Plan Z

## Next Review: YYYY-MM-DD
```

---

## Tools and Automation

### Markdown Linting

Use `markdownlint` or similar to enforce style:
```bash
markdownlint prose/**/*.md
```

### Link Checking

Verify internal links are valid:
```bash
markdown-link-check prose/**/*.md
```

### Code Example Testing

Ensure code examples compile:
```bash
# Extract and test code examples from markdown
cargo test --doc
```

---

## Common Issues and Solutions

### Issue: Design doc doesn't match implementation

**Solution:**
1. Determine if implementation or design is "correct"
2. If design is correct: File issue to fix implementation
3. If implementation is correct: Update design doc
4. Create debrief explaining deviation

### Issue: Missing cross-references

**Solution:**
1. Search for related documents using keywords
2. Add cross-reference section
3. Update related docs to link back

### Issue: Outdated examples

**Solution:**
1. Update examples to current API
2. Test examples compile
3. Consider adding as doc test

### Issue: Duplicated information

**Solution:**
1. Choose canonical location
2. Replace duplicates with cross-references
3. Keep details in one place

---

## Best Practices

1. **Document Before Implementing**: Design docs should precede code
2. **Debrief After Completing**: Capture learnings while fresh
3. **Update Proactively**: Fix docs as you notice issues
4. **Link Generously**: Cross-reference related information
5. **Keep It Current**: Better to update incrementally than in big batches
6. **Seek Feedback**: Have others review your documentation
7. **Test Examples**: Ensure code examples actually work
8. **Maintain History**: Use git for version history, not in-document versioning

---

## Related Documents

- [TECHNICAL_DEBT_REDUCTION.md](TECHNICAL_DEBT_REDUCTION.md) - Technical debt reduction plan
- [TEST_COVERAGE_MATRIX.md](TEST_COVERAGE_MATRIX.md) - Test coverage overview
- [../MAINTAINABILITY_SUMMARY.md](../MAINTAINABILITY_SUMMARY.md) - Project maintainability
- [DESIGN_DOCUMENT_TEMPLATE.md](DESIGN_DOCUMENT_TEMPLATE.md) - Template for new designs

---

## Getting Started

### For New Contributors

1. Read [ARCHITECTURE.md](../designs/ARCHITECTURE.md) first
2. Explore design docs for area you're working on
3. Follow this review process when making changes
4. Ask questions if documentation is unclear

### For Maintainers

1. Schedule quarterly reviews in calendar
2. Set up markdown linting in CI
3. Review new design docs within 1 week
4. Keep this process document up-to-date
