# [Feature/Component Name]

> **Status**: Draft | Planning | In Progress | Implemented | Deprecated
>
> **Created**: YYYY-MM-DD
>
> **Last Updated**: YYYY-MM-DD (optional)
>
> **Related**: [Links to related design documents]

---

## Problem Statement

[Describe the problem this design addresses. What user need or technical challenge are we solving?]

**Context:**
- [Relevant background information]
- [Current pain points or limitations]
- [Why this matters to users/developers]

**Goals:**
- [Primary goal]
- [Secondary goal]
- [Additional objectives]

**Non-Goals:**
- [What this design explicitly does NOT address]
- [Out of scope items]

---

## Design Principles

1. **[Principle Name]**
   - [Description of this guiding principle]
   - [Why it matters]

2. **[Another Principle]**
   - [Description]
   - [Rationale]

[Add as many principles as needed to guide the design]

---

## Architecture

### High-Level Overview

[Provide a bird's-eye view of the solution]

```
[Optional: ASCII diagram, flowchart, or component diagram]
```

### Components

#### Component 1: [Name]

**Purpose:** [What this component does]

**Responsibilities:**
- [Responsibility 1]
- [Responsibility 2]

**Interface:**
```rust
// Key types, traits, or functions
pub struct ComponentName {
    // fields
}

impl ComponentName {
    // key methods
}
```

**Implementation Notes:**
- [Key implementation detail]
- [Design decision explanation]

#### Component 2: [Name]

[Same structure as Component 1]

### Data Flow

[Describe how data flows through the system]

1. [Step 1: Initial input]
2. [Step 2: Processing]
3. [Step 3: Output]

**Example:**
```rust
// Example of how components work together
let input = ...;
let result = component1.process(input)?;
let output = component2.finalize(result)?;
```

---

## Detailed Design

### [Aspect 1: e.g., API Design]

**Public API:**
```rust
// API definition
pub fn public_method(&self, param: Type) -> Result<Output, Error> {
    // ...
}
```

**Usage Example:**
```rust
// Show how users will interact with this feature
let instance = FeatureName::new();
let result = instance.public_method(value)?;
```

### [Aspect 2: e.g., Error Handling]

**Error Types:**
- `ErrorVariant1` - When [condition]
- `ErrorVariant2` - When [condition]

**Error Messages:**
```
Error message template with helpful context
```

### [Aspect 3: e.g., Configuration]

**Configuration Schema:**
```toml
[section]
key = "value"  # Description
```

---

## Implementation Guidance

### Implementation Phases

**Phase 1: [Foundation]**
- [ ] [Task 1]
- [ ] [Task 2]
- [ ] [Testing approach]

**Phase 2: [Core Functionality]**
- [ ] [Task 1]
- [ ] [Task 2]
- [ ] [Integration testing]

**Phase 3: [Polish & Documentation]**
- [ ] [Documentation updates]
- [ ] [Example creation]
- [ ] [Performance validation]

### File Organization

**New Files:**
- `path/to/new_file.rs` - [Purpose]

**Modified Files:**
- `path/to/existing.rs` - [Changes needed]

### Testing Strategy

**Unit Tests:**
- Test [specific behavior]
- Validate [error conditions]
- Verify [edge cases]

**Integration Tests:**
- End-to-end workflow test
- Multi-component interaction test

**Doc Tests:**
- API usage examples
- Common patterns

---

## Alternatives Considered

### Alternative 1: [Approach Name]

**Description:** [How this alternative would work]

**Pros:**
- [Advantage 1]
- [Advantage 2]

**Cons:**
- [Disadvantage 1]
- [Disadvantage 2]

**Decision:** [Why this was not chosen]

### Alternative 2: [Another Approach]

[Same structure as Alternative 1]

---

## Trade-offs and Limitations

### Trade-offs

1. **[Trade-off 1]**
   - We chose [option A] over [option B]
   - Reason: [explanation]
   - Impact: [consequences]

2. **[Trade-off 2]**
   - [Description]

### Known Limitations

1. **[Limitation 1]**
   - Description: [what doesn't work]
   - Workaround: [if any]
   - Future work: [potential solution]

2. **[Limitation 2]**
   - [Description]

---

## Migration Path

[If this changes existing behavior]

### For Users

**Before:**
```rust
// Old usage pattern
```

**After:**
```rust
// New usage pattern
```

**Migration Steps:**
1. [Step 1]
2. [Step 2]

**Breaking Changes:**
- [Change 1] - [migration guidance]
- [Change 2] - [migration guidance]

### For Backend Implementers

[If relevant]

**Required Changes:**
- [Change 1]
- [Change 2]

**Optional Enhancements:**
- [Enhancement 1]

---

## Performance Considerations

**Memory Usage:**
- [Expected memory impact]
- [Optimization strategies]

**Computation:**
- [Time complexity]
- [Optimization opportunities]

**Benchmarks:**
[If applicable, include benchmark results or targets]

---

## Security Considerations

**Potential Risks:**
1. [Risk 1] - [mitigation]
2. [Risk 2] - [mitigation]

**Input Validation:**
- [What needs validation]
- [Validation approach]

**Dependencies:**
- [New dependencies and their security posture]

---

## Future Considerations

[Ideas for future enhancement that are out of scope now]

1. **[Future Enhancement 1]**
   - [Description]
   - [Why deferred]

2. **[Future Enhancement 2]**
   - [Description]

---

## Cross-References

**Related Design Documents:**
- [ARCHITECTURE.md](ARCHITECTURE.md) - [Relevance]
- [OTHER_DOC.md](OTHER_DOC.md) - [Relevance]

**Implementation:**
- [Link to implementation file or module]
- [Link to tests]

**Issues/Discussions:**
- [Issue #123](https://github.com/org/repo/issues/123) - [Description]

**External References:**
- [External resource 1]
- [External resource 2]

---

## Review Notes

[Optional section for capturing review feedback]

**Reviewed By:** [Name/Date]

**Key Feedback:**
- [Point 1]
- [Point 2]

**Decisions:**
- [Decision 1]
- [Decision 2]

---

## Appendix

[Optional: Additional information that doesn't fit above]

### Appendix A: [Title]

[Content]

### Appendix B: [Title]

[Content]
