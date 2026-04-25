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

- **Quill Templates**: Sample Quill templates under `resources/quills/`, each with `plate.typ`, `Quill.yaml`, and assets (versioned subdirectories, e.g. `0.1.0/`)
  - `quills/usaf_memo/` - US Air Force memo template
  - `quills/taro/` - Custom template example
  - `quills/classic_resume/` - Classic resume template
  - `quills/cmu_letter/` - CMU letter template

- **Legacy Quill Template** (unversioned, directly under `resources/`)
  - `appreciated_letter/` - A formal letter template (uses `glue.typ` instead of `plate.typ`)

- **Sample Markdown Files**: Example markdown files for testing
  - `sample.md` - Basic markdown example
  - `frontmatter_demo.md` - Demonstrates YAML frontmatter
  - `extended_metadata_demo.md` - Extended metadata examples
  - `appreciated_letter/appreciated_letter.md` - Example content for the appreciated_letter template
  - `*.md` - Various markdown test files

## Rust Crate

This package is also available as a Rust crate `quillmark-fixtures` for use in Rust projects. The Rust crate provides helper functions for accessing fixture paths programmatically.

## License

Licensed under the MIT License. See LICENSE for details.
