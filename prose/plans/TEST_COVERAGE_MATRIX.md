# Test Coverage Matrix

> **Status**: Documentation
>
> This document provides a comprehensive overview of test coverage across the Quillmark codebase.

---

## Test File Overview

| Test File | Lines | Tests | Primary Focus | Related Design Docs |
|-----------|-------|-------|---------------|---------------------|
| `acroform_integration_tests.rs` | 110 | 2 | AcroForm backend PDF form filling | [ACROFORM.md](../designs/ACROFORM.md) |
| `api_rework_test.rs` | 189 | 5 | Workflow API validation | [ARCHITECTURE.md](../designs/ARCHITECTURE.md) |
| `auto_glue_test.rs` | 174 | 3 | Automatic JSON glue generation | [ARCHITECTURE.md](../designs/ARCHITECTURE.md) |
| `backend_registration_test.rs` | 184 | 5 | Custom backend registration | [ARCHITECTURE.md](../designs/ARCHITECTURE.md) |
| `default_quill_test.rs` | 253 | 7 | Default Quill system | [DEFAULT_QUILL.md](../designs/DEFAULT_QUILL.md) |
| `default_values_test.rs` | 261 | 4 | Field default values from schema | [SCHEMAS.md](../designs/SCHEMAS.md) |
| `dynamic_assets_test.rs` | 116 | 4 | Runtime asset injection | [ARCHITECTURE.md](../designs/ARCHITECTURE.md) |
| `dynamic_fonts_test.rs` | 163 | 4 | Runtime font injection | [ARCHITECTURE.md](../designs/ARCHITECTURE.md) |
| `feature_flag_test.rs` | 60 | 2 | Feature flag conditional backend registration | [ARCHITECTURE.md](../designs/ARCHITECTURE.md) |
| `quill_engine_test.rs` | 290 | 11 | End-to-end engine integration | [ARCHITECTURE.md](../designs/ARCHITECTURE.md), [QUILL.md](../designs/QUILL.md) |
| **Total** | **1,891** | **47** | | |

---

## Feature Coverage

### Core Engine Features

| Feature | Test Files | Test Count | Coverage Level |
|---------|------------|------------|----------------|
| Engine creation & initialization | `quill_engine_test.rs`, `feature_flag_test.rs` | 3 | ✅ High |
| Quill registration | `quill_engine_test.rs`, `backend_registration_test.rs` | 4 | ✅ High |
| Workflow creation (by name) | `api_rework_test.rs`, `quill_engine_test.rs` | 3 | ✅ High |
| Workflow creation (by object) | `api_rework_test.rs`, `quill_engine_test.rs` | 2 | ✅ High |
| Workflow creation (from parsed) | `quill_engine_test.rs`, `default_quill_test.rs` | 4 | ✅ High |
| Backend auto-registration | `feature_flag_test.rs` | 2 | ✅ High |

### Parsing & Templating

| Feature | Test Files | Test Count | Coverage Level |
|---------|------------|------------|----------------|
| Markdown parsing | `api_rework_test.rs`, `quill_engine_test.rs` | 5 | ✅ High |
| Glue template processing | `api_rework_test.rs`, `quill_engine_test.rs` | 3 | ✅ High |
| Auto glue generation | `auto_glue_test.rs` | 3 | ✅ High |
| Default values from schema | `default_values_test.rs` | 4 | ✅ High |

### Backend System

| Feature | Test Files | Test Count | Coverage Level |
|---------|------------|------------|----------------|
| Custom backend registration | `backend_registration_test.rs` | 5 | ✅ High |
| Default Quill registration | `default_quill_test.rs` | 7 | ✅ High |
| Default Quill usage | `default_quill_test.rs` | 3 | ✅ High |
| AcroForm backend | `acroform_integration_tests.rs` | 2 | ⚠️ Medium |
| Typst backend | Various | Multiple | ✅ High |

### Dynamic Resources

| Feature | Test Files | Test Count | Coverage Level |
|---------|------------|------------|----------------|
| Dynamic asset injection | `dynamic_assets_test.rs` | 4 | ✅ High |
| Dynamic font injection | `dynamic_fonts_test.rs` | 4 | ✅ High |
| Asset collision handling | `dynamic_assets_test.rs` | 1 | ✅ High |
| Asset clearing | `dynamic_assets_test.rs` | 1 | ✅ High |

### Error Handling

| Feature | Test Files | Test Count | Coverage Level |
|---------|------------|------------|----------------|
| Missing quill error | `quill_engine_test.rs`, `default_quill_test.rs` | 2 | ✅ High |
| Invalid backend error | `quill_engine_test.rs` | 1 | ⚠️ Medium |
| Validation errors | `default_values_test.rs` | 2 | ✅ High |

---

## Test Strategy by File

### Integration Tests

**`quill_engine_test.rs`** (290 lines, 11 tests)
- **Purpose**: Comprehensive end-to-end integration testing
- **Strategy**: Temporary directories, custom quills, full pipeline validation
- **Coverage**: Engine lifecycle, quill management, workflow orchestration, rendering
- **Key Tests**:
  - Engine creation and backend registration
  - Quill loading and registration
  - Workflow creation patterns
  - End-to-end rendering validation

**`api_rework_test.rs`** (189 lines, 5 tests)
- **Purpose**: Focused API method validation
- **Strategy**: Minimal quills, specific API contracts
- **Coverage**: Public API surface for workflows and parsing
- **Relationship**: Complements `quill_engine_test.rs` with targeted API validation
- **Key Tests**:
  - `ParsedDocument::from_markdown()`
  - `Workflow::render()`
  - `Workflow::process_glue()`
  - Workflow creation methods

### Feature-Specific Tests

**`default_quill_test.rs`** (253 lines, 7 tests)
- **Purpose**: Default Quill system behavior
- **Coverage**: Auto-registration, fallback behavior, precedence rules
- **Key Tests**:
  - Default Quill registered on backend registration
  - Used when no QUILL tag present
  - Explicit QUILL tag takes precedence
  - Error when neither available

**`default_values_test.rs`** (261 lines, 4 tests)
- **Purpose**: Schema-based default value population
- **Coverage**: Field defaults, validation with/without defaults
- **Key Tests**:
  - Defaults applied to missing fields
  - Defaults don't override existing values
  - Validation with and without defaults

**`auto_glue_test.rs`** (174 lines, 3 tests)
- **Purpose**: Automatic JSON glue generation
- **Coverage**: Auto glue for backends that support it
- **Key Tests**:
  - Auto glue without glue file
  - Auto glue output structure
  - Nested data handling

**`dynamic_assets_test.rs`** (116 lines, 4 tests)
- **Purpose**: Runtime asset injection
- **Coverage**: Asset addition, naming, collision avoidance
- **Key Tests**:
  - Basic asset injection
  - Multiple assets
  - Collision handling
  - Asset clearing

**`dynamic_fonts_test.rs`** (163 lines, 4 tests)
- **Purpose**: Runtime font injection
- **Coverage**: Font addition, registration, backend accessibility
- **Key Tests**:
  - Basic font injection
  - Multiple fonts
  - Font accessibility to backend

**`backend_registration_test.rs`** (184 lines, 5 tests)
- **Purpose**: Custom backend extension system
- **Coverage**: Backend registration, replacement, integration
- **Uses**: Mock backend for isolation
- **Key Tests**:
  - Basic registration
  - Multiple backends
  - Backend replacement
  - Workflow with custom backend

**`feature_flag_test.rs`** (60 lines, 2 tests)
- **Purpose**: Conditional compilation and backend registration
- **Coverage**: Feature-based backend inclusion
- **Strategy**: Conditional compilation directives
- **Key Tests**:
  - Backend registered when feature enabled
  - No backend when feature disabled

**`acroform_integration_tests.rs`** (110 lines, 2 tests)
- **Purpose**: AcroForm backend functionality
- **Coverage**: PDF form filling, field mapping
- **Requirement**: `acroform` feature flag
- **Key Tests**:
  - Backend compilation
  - Field type preservation

---

## Coverage Gaps & Recommendations

### Current Gaps

1. **Error Path Coverage**: Limited testing of error propagation and diagnostic messages
   - **Recommendation**: Add dedicated error handling test suite
   - **Priority**: Medium

2. **Parse Module**: Limited direct testing of Extended YAML Metadata Standard
   - **Note**: Parse module has extensive unit tests in `quillmark-core`
   - **Recommendation**: Add integration tests for complex metadata scenarios
   - **Priority**: Low

3. **Glue Metadata Access**: Limited testing of `__metadata__` field
   - **Note**: Covered by design document [GLUE_METADATA.md](../designs/GLUE_METADATA.md)
   - **Recommendation**: Add tests validating metadata access patterns
   - **Priority**: Low

4. **Multi-Backend Scenarios**: Limited testing of multiple backends in same engine
   - **Note**: Basic coverage in `backend_registration_test.rs`
   - **Recommendation**: Add tests for backend switching and format selection
   - **Priority**: Low

### Strengths

1. ✅ **Comprehensive Integration Coverage**: End-to-end workflows well tested
2. ✅ **Feature Isolation**: Each feature has dedicated test file
3. ✅ **API Contract Validation**: Public API methods thoroughly tested
4. ✅ **Documentation**: All test files have module-level documentation
5. ✅ **Test Organization**: Clear separation of concerns across test files

---

## Test Metrics

- **Total Integration Tests**: 47
- **Total Test Lines**: 1,891
- **Average Tests per File**: 4.7
- **Test Documentation**: 100% (all files have module docs)
- **Feature Coverage**: High (all major features tested)

---

## Relationship to Design Documents

| Design Document | Primary Test Files | Coverage |
|-----------------|-------------------|----------|
| [ARCHITECTURE.md](../designs/ARCHITECTURE.md) | Most test files | ✅ High |
| [DEFAULT_QUILL.md](../designs/DEFAULT_QUILL.md) | `default_quill_test.rs` | ✅ High |
| [QUILL.md](../designs/QUILL.md) | `quill_engine_test.rs` | ✅ High |
| [PARSE.md](../designs/PARSE.md) | Core unit tests | ⚠️ Medium |
| [ERROR.md](../designs/ERROR.md) | Scattered across tests | ⚠️ Medium |
| [SCHEMAS.md](../designs/SCHEMAS.md) | `default_values_test.rs` | ✅ High |
| [ACROFORM.md](../designs/ACROFORM.md) | `acroform_integration_tests.rs` | ⚠️ Medium |
| [GLUE_METADATA.md](../designs/GLUE_METADATA.md) | Limited coverage | ⚠️ Low |

---

## Test Execution

### Running All Tests

```bash
cargo test
```

### Running Specific Test Files

```bash
cargo test --test quill_engine_test
cargo test --test api_rework_test
cargo test --test default_quill_test
```

### Running with Features

```bash
# With AcroForm backend
cargo test --features acroform

# Without Typst backend (non-default)
cargo test --no-default-features
```

### Running Doc Tests

```bash
cargo test --doc
```

---

## Maintenance Guidelines

1. **New Features**: Add dedicated test file for major features
2. **Bug Fixes**: Add regression test in appropriate file
3. **API Changes**: Update `api_rework_test.rs` and `quill_engine_test.rs`
4. **Backend Changes**: Update `backend_registration_test.rs` or backend-specific files
5. **Documentation**: Keep module-level docs in sync with test purpose

---

## Related Documents

- [../plans/TECHNICAL_DEBT_REDUCTION.md](../plans/TECHNICAL_DEBT_REDUCTION.md) - Overall technical debt plan
- [../plans/DEFAULT_QUILL.md](../plans/DEFAULT_QUILL.md) - Default Quill implementation plan
- [../designs/ARCHITECTURE.md](../designs/ARCHITECTURE.md) - System architecture
- [../MAINTAINABILITY_SUMMARY.md](../MAINTAINABILITY_SUMMARY.md) - Project maintainability overview
