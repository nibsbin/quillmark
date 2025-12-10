# @quillmark-test/fixtures

Test fixtures and sample Quill templates for [Quillmark](https://github.com/nibsbin/quillmark).

## Overview

This package contains sample Quill templates and markdown files used for testing and examples in the Quillmark ecosystem. It's designed to be used by JavaScript/TypeScript applications that work with Quillmark.

## Usage

This package has no entrypoint and simply bundles the `resources/` directory. You can access the fixture files directly:

```javascript
// Using in Node.js
import { readFileSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixturesPath = join(__dirname, 'node_modules/@quillmark-test/fixtures/resources');

// Access a fixture file
const sampleMd = readFileSync(join(fixturesPath, 'sample.md'), 'utf-8');
```

```javascript
// In browser with bundler
// Import the path and fetch the resource
const response = await fetch('node_modules/@quillmark-test/fixtures/resources/sample.md');
const sampleMd = await response.text();
```

## Available Resources

The package includes:

- **Quill Templates**: Sample Quill templates with plate.typ, Quill.toml, and assets
  - `appreciated_letter/` - A formal letter template
  - `usaf_memo/` - US Air Force memo template
  - `taro/` - Custom template example

  Each Quill template now includes a `template` field in `Quill.toml` that points to a sample markdown file demonstrating the template's usage. This allows users to see example content for each template.

- **Sample Markdown Files**: Example markdown files for testing
  - `sample.md` - Basic markdown example
  - `frontmatter_demo.md` - Demonstrates YAML frontmatter
  - `extended_metadata_demo.md` - Extended metadata examples
  - Template-specific markdown files (e.g., `appreciated_letter.md`, `usaf_memo.md`, `taro.md`)
  - `*.md` - Various markdown test files

## Rust Crate

This package is also available as a Rust crate `quillmark-fixtures` for use in Rust projects. The Rust crate provides helper functions for accessing fixture paths programmatically.

## License

Licensed under the MIT License. See LICENSE for details.
