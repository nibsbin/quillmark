# Quillmark WASM End-to-End Tests

This directory contains end-to-end tests for the `quillmark-wasm` package using Vite and Vitest.

## Overview

These tests validate the complete WASM API in a browser-like environment (using happy-dom). They test:

- **Basic API operations**: Parsing markdown, creating engines, registering Quills
- **Complete rendering workflow**: From markdown input to final PDF/SVG/TXT artifacts
- **Edge cases**: Unicode, special characters, empty inputs, large documents
- **Error handling**: Invalid inputs, missing Quills, malformed data
- **Performance**: Memory management, concurrent operations, repeated renders

## Prerequisites

Before running the tests, ensure you have:

1. Built the WASM module:
   ```bash
   cd /path/to/quillmark
   bash scripts/build-wasm.sh
   ```

2. Installed Node.js and npm (v20+ recommended)

## Installation

From this directory (`quillmark-wasm/e2e-tests/`):

```bash
npm install
```

## Running Tests

### Run all tests once

```bash
npm test
```

### Watch mode (re-run on changes)

```bash
npm run test:watch
```

### UI mode (interactive browser UI)

```bash
npm run test:ui
```

Then open your browser to the URL shown (usually `http://localhost:51204/`)

## Test Structure

The test suite is organized into three main files:

### `basic-api.test.js`

Tests fundamental API operations:
- `parseMarkdown()` - Parsing markdown with YAML frontmatter
- `new Quillmark()` - Creating engine instances
- `registerQuill()` - Registering Quill templates
- `getQuillInfo()` - Retrieving Quill metadata
- `listQuills()` - Listing registered Quills
- `unregisterQuill()` - Removing Quills

### `rendering.test.js`

Tests the complete rendering workflow:
- Full workflow: parse → register → info → render
- Different output formats (PDF, SVG, TXT)
- Render options (quillName, format)
- `renderGlue()` debugging helper
- Artifact structure and validation
- Complex documents with formatting

### `edge-cases.test.js`

Tests edge cases and error handling:
- Empty and minimal inputs
- Unicode and special characters
- Whitespace handling
- Various YAML field types (nested, null, boolean, numeric)
- Error messages and validation
- Memory management
- Concurrent operations
- Performance under load

## Test Fixtures

Test data is defined in `fixtures/test-data.js`:

- `SMALL_QUILL_JSON` - Minimal Quill for basic testing
- `LETTER_QUILL_JSON` - More complex letter template
- `SIMPLE_MARKDOWN` - Basic markdown document
- `LETTER_MARKDOWN` - Letter-style document
- `INVALID_QUILL_JSON` - Invalid Quill for error testing
- `INVALID_MARKDOWN` - Malformed markdown for error testing

## Configuration

### `vite.config.js`

Vite configuration for the test environment:
- Test environment: `happy-dom` (lightweight DOM implementation)
- WASM module alias: `@quillmark-test/wasm` → `../../pkg/bundler`
- File system access for loading WASM from parent directories

### `package.json`

Test dependencies:
- `vitest` - Test runner (Vite-native, fast)
- `@vitest/ui` - Interactive browser UI for tests
- `happy-dom` - Lightweight DOM for WASM environment
- `vite` - Build tool and dev server

## Writing New Tests

To add new tests:

1. Create a new `.test.js` file in this directory
2. Import from `@quillmark-test/wasm` and `./fixtures/test-data.js`
3. Use Vitest's `describe()`, `it()`, and `expect()` APIs
4. Run tests with `npm test`

Example:

```javascript
import { describe, it, expect } from 'vitest';
import { Quillmark } from '@quillmark-test/wasm';
import { SIMPLE_MARKDOWN, SMALL_QUILL_JSON } from './fixtures/test-data.js';

describe('My New Feature', () => {
  it('should work as expected', () => {
    const engine = new Quillmark();
    engine.registerQuill('test-quill', SMALL_QUILL_JSON);
    
    const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);
    const result = engine.render(parsed, { format: 'pdf' });
    
    expect(result.artifacts.length).toBeGreaterThan(0);
  });
});
```

## CI/CD Integration

To run these tests in CI:

```bash
# Build WASM first
cd /path/to/quillmark
bash scripts/build-wasm.sh

# Run tests
cd quillmark-wasm/e2e-tests
npm install
npm test
```

Exit code will be 0 for success, non-zero for failures.

## Troubleshooting

### Tests fail with "Cannot find module '@quillmark-test/wasm'"

Make sure you've built the WASM module first:
```bash
cd ../..
bash scripts/build-wasm.sh
```

### Tests hang or timeout

Some tests render PDFs which can be slow. Default timeout is 5000ms. If needed, increase timeout in `vite.config.js`:

```javascript
export default defineConfig({
  test: {
    testTimeout: 10000, // 10 seconds
    // ...
  },
});
```

### WASM initialization errors

Ensure you're using a recent version of Node.js (v20+) with WASM support.

## Performance Benchmarks

Typical performance on a modern development machine:

- Test suite execution: ~5-15 seconds (full suite)
- Individual render operation: 50-500ms
- Markdown parsing: <5ms
- Quill registration: <10ms

## License

Apache-2.0 (same as the main Quillmark project)
