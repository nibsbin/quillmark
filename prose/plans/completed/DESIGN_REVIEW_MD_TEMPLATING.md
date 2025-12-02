# Design Review: MD_TEMPLATING_PROPOSAL.md

**Reviewer**: GitHub Copilot Agent  
**Review Date**: 2025-12-01  
**Document Under Review**: `/home/runner/work/quillmark/quillmark/prose/MD_TEMPLATING_PROPOSAL.md`

## Executive Summary

**RECOMMENDATION**: Document requires significant restructuring before implementation.

**Status**: ❌ NOT READY - The document violates project conventions by combining design and implementation details with embedded pseudocode. It should be split into separate design and plan documents.

**Action Required**:
1. Move document to proper location (designs/ or plans/)
2. Remove implementation code and pseudocode
3. Separate design decisions from implementation steps
4. Add cross-references following DRY principles

## Critical Issues

### 1. Document Classification Violation

**Issue**: Document is titled as a "proposal" but contains both design and implementation content.

**Current State**:
- Located at root of `prose/` directory
- Contains design rationale AND implementation pseudocode
- Mixes "what" and "how" concerns
- Includes specific function names and code patterns

**Required Action**:
- Create separate design document in `prose/designs/TYPST_GUILLEMET_CONVERSION.md`
- Create implementation plan in `prose/plans/GUILLEMET_CONVERSION.md`
- Remove from root `prose/` directory

**Severity**: HIGH - Violates project organization standards

### 2. Code in Design Document

**Issue**: Document includes extensive pseudocode and implementation details.

**Examples**:
- Lines 155-172: Detailed pseudo-code for conversion logic
- Lines 107-116: Specific method signatures and constants
- Lines 82-100: Step-by-step technical implementation flow

**Agent Instructions State**: "Include zero fucking code in your designs/plans"

**Required Action**:
- Remove ALL code examples, pseudocode, and implementation snippets
- Replace with high-level behavioral descriptions
- Move technical details to rustdoc or implementation comments

**Severity**: HIGH - Direct violation of design document standards

### 3. Missing Cross-References

**Issue**: Document doesn't reference existing design documentation.

**Missing References**:
- `prose/designs/ARCHITECTURE.md` - System architecture
- `prose/designs/PARSE.md` - Markdown parsing principles
- `crates/backends/typst/docs/designs/CONVERT.md` - Existing converter spec
- `prose/designs/ERROR.md` - Error handling patterns

**Required Action**:
- Add "Related Documentation" section
- Reference existing designs following DRY
- Cross-link to converter spec for consistency

**Severity**: MEDIUM - Reduces design coherence

## Moderate Issues

### 4. Unclear Design vs Implementation Boundary

**Issue**: Design decisions intermixed with implementation choices.

**Design Questions** (belong in design doc):
- Why strip formatting instead of preserving it?
- Why use `<<`/`>>` as delimiters?
- Why require same-line matching?
- What security considerations matter?

**Implementation Questions** (belong in plan):
- Which specific files need changes?
- What order should changes be made?
- How to structure test cases?
- What constants and thresholds to use?

**Required Action**:
- Design doc answers "what" and "why"
- Plan doc answers "how" and "when"
- Clearly separate the two concerns

**Severity**: MEDIUM - Creates confusion for implementers

### 5. Over-Specification of Implementation Details

**Issue**: Design doc specifies implementation minutiae that should be left to engineer judgment.

**Examples**:
- Line 59: "Use `String::with_capacity`" - optimization detail
- Line 110: "const MAX_GUILLEMET_BUFFER: usize = 64 * 1024" - exact constant
- Line 84-86: Flag variable names and types
- Line 157-171: Complete algorithm pseudocode

**Required Action**:
- State requirements and constraints
- Avoid prescribing specific implementation techniques
- Trust implementer to make appropriate technical choices within design boundaries

**Severity**: MEDIUM - Reduces implementation flexibility

### 6. Testing Instructions in Design

**Issue**: Lines 117-128 and 140-152 contain detailed test naming and structure.

**Current State**:
- Specific test function names prescribed
- Test organization dictated
- Fixture locations specified

**Required Action**:
- Move to implementation plan
- Design should state *what* needs testing, not *how* to structure tests
- Test naming and organization are implementation concerns

**Severity**: LOW - Better suited to plan document

## Minor Issues

### 7. Inconsistent Terminology

**Issue**: Terms used inconsistently throughout document.

**Examples**:
- "guillemet conversion" vs "chevron conversion"
- "inner content" vs "inner region" vs "inner payload"
- "buffer limit" vs "maximum buffer size"

**Required Action**:
- Establish canonical terms in design doc
- Use consistently throughout both documents

**Severity**: LOW - Reduces clarity

### 8. Missing Design Rationale

**Issue**: Some decisions lack justification.

**Examples**:
- Why 64KB buffer limit specifically?
- Why 512 event count specifically?
- Why strip ALL formatting vs selective stripping?

**Required Action**:
- Add "Alternatives Considered" section to design
- Explain tradeoffs for key decisions
- Justify security thresholds

**Severity**: LOW - Helpful for future maintenance

## Technical Content Assessment

### Strengths

✅ **Comprehensive coverage** - Most edge cases identified  
✅ **Security awareness** - Buffer limits and DoS prevention considered  
✅ **Context awareness** - Correctly identifies where conversion should be disabled  
✅ **Safety-first approach** - Prefers literal output over risky conversions  
✅ **Integration with existing code** - Understands current converter architecture

### Concerns

⚠️ **Complexity** - Multi-phase scanning with sub-parsing may be over-engineered  
⚠️ **Event skipping** - Advancing main iterator past consumed events is error-prone  
⚠️ **Performance** - Creating temporary parsers for every chevron pair has overhead  
⚠️ **State management** - Multiple boolean flags increase cognitive load

### Suggested Simplifications

1. **Use simpler text extraction**: Instead of sub-parser, use regex or state machine to strip formatting characters from source substring
2. **Avoid iterator manipulation**: Process all events, emit guillemets during output phase
3. **Reduce flag count**: Combine related flags (e.g., `in_literal_context`)

## Recommendations

### Immediate Actions (Required)

1. ✅ **COMPLETED**: Create proper design document at `prose/designs/TYPST_GUILLEMET_CONVERSION.md`
2. ✅ **COMPLETED**: Create implementation plan at `prose/plans/GUILLEMET_CONVERSION.md`
3. **TODO**: Remove or relocate `prose/MD_TEMPLATING_PROPOSAL.md`
4. **TODO**: Update `prose/designs/INDEX.md` to reference new design

### Design Document Structure

**Required Sections**:
- Overview - What is being designed
- Rationale - Why this approach
- Behavior Specification - Expected behavior without implementation details
- Edge Cases - Boundary conditions and their handling
- Design Constraints - Limitations and non-negotiable requirements
- Alternatives Considered - Other approaches and why rejected
- Related Documentation - Cross-references

**Excluded Content**:
- Code examples or pseudocode
- Specific function/variable names
- Step-by-step implementation instructions
- Test structure and naming

### Implementation Plan Structure

**Required Sections**:
- Overview - Brief summary linking to design
- Prerequisites - What must be understood first
- Implementation Phases - Logical ordering of work
- Testing Strategy - What and how to test
- Rollout Considerations - Deployment and compatibility
- Success Criteria - Definition of done

**Included Content**:
- Specific files to modify
- Suggested approach (but not mandated)
- Test case ideas
- Integration points

## Edge Cases and Technical Review

### Well-Handled Cases

✅ Unmatched chevrons → literal output  
✅ Code spans/blocks → no conversion  
✅ Link destinations → no conversion  
✅ Buffer overflow protection  
✅ Same-line requirement

### Cases Needing Clarification

❓ **Nested chevrons**: Document says "nearest matching `>>`" but example suggests it's unclear  
❓ **HTML inside chevrons**: Should abort conversion, but what about `&nbsp;` entities?  
❓ **Inline code inside chevrons**: Should `` <<`code`>> `` become `«code»` (strip backticks) or abort?  
❓ **Images inside chevrons**: Extract alt text or abort conversion?

### Potential Implementation Pitfalls

⚠️ **UTF-8 boundaries**: Scanning source string must respect character boundaries  
⚠️ **Event consumption**: Advancing main parser past sub-parsed region is complex and error-prone  
⚠️ **Range calculation**: Source ranges may not align with event boundaries  
⚠️ **Performance**: Creating Parser for every `<<` has non-trivial cost

### Suggested Technical Approach

Instead of sub-parsing, consider:

1. Detect `<<` in Event::Text using source ranges
2. Scan source forward to find `>>` (with limits)
3. Extract substring between chevrons
4. Use simple state machine or regex to strip `**`, `__`, `*`, `_`, `~~`, `[...]`, backticks
5. Emit guillemets with escaped result
6. Track how many bytes consumed, continue main parse from there

This is simpler, faster, and avoids iterator manipulation challenges.

## Acceptance Criteria Validation

| Criterion | Status | Notes |
|-----------|--------|-------|
| `<<text>>` → `«text»` | ✅ Clear | Basic case well-defined |
| Strip inline formatting | ✅ Clear | Specified but could list exact formats |
| Code spans unchanged | ✅ Clear | Correctly identified |
| Link destinations unchanged | ✅ Clear | Tag::Link handling specified |
| Unmatched chevrons literal | ✅ Clear | Behavior defined |
| Bounded buffering | ✅ Clear | Limits specified (though arbitrary) |
| Same-line matching | ⚠️ Optional | Marked optional but used in examples |

## Documentation Gaps

### Missing from Design

- Performance expectations
- Relationship to Typst's native quote handling
- User-facing documentation location
- Migration guide (if applicable)
- Configuration options (or statement that there are none)

### Missing from Implementation Plan

- Dependency changes (if any)
- Feature flag approach (marked optional, but should decide)
- Rollout timeline or phases
- Backward compatibility testing

## Final Recommendation

**Status**: NOT READY for implementation as-is.

**Required Changes** (Blocking):
1. Split into separate design and plan documents
2. Remove all code and pseudocode from design
3. Add cross-references to existing designs
4. Clarify edge case handling (HTML entities, inline code, images)

**Recommended Changes** (Non-blocking):
1. Consider simpler implementation approach
2. Add "Alternatives Considered" section
3. Justify specific threshold values
4. Add user-facing documentation plan

**Estimated Effort**: 2-3 hours to properly restructure

**Next Steps**:
1. Review and approve separate design doc
2. Review and refine implementation plan
3. Update INDEX.md with new design reference
4. Proceed with implementation

## Conclusion

The original MD_TEMPLATING_PROPOSAL.md contains valuable technical analysis and thorough edge case consideration. However, it violates project documentation standards by combining design and implementation with embedded code.

I have created proper design and plan documents that separate concerns appropriately:
- `prose/designs/TYPST_GUILLEMET_CONVERSION.md` - Design document (what/why)
- `prose/plans/GUILLEMET_CONVERSION.md` - Implementation plan (how/when)

These documents follow project conventions, maintain DRY through cross-referencing, contain no code, and clearly separate design decisions from implementation details.

**Recommendation**: Archive or remove the original proposal and proceed with the properly structured documents.
