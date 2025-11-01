# JavaScript/WASM API Reference

Complete reference for the Quillmark WebAssembly API for JavaScript and TypeScript.

## Installation

```bash
# Using npm
npm install @quillmark-test/wasm

# Using yarn
yarn add @quillmark-test/wasm

# Using pnpm
pnpm add @quillmark-test/wasm
```

## Quick Example

```javascript
import { Quillmark } from '@quillmark-test/wasm';

// Create engine
const engine = new Quillmark();

// Register a quill from JSON
const quillJson = {
  files: {
    "Quill.toml": {
      contents: `[Quill]
name = "my-quill"
backend = "typst"
description = "My template"
`
    },
    "glue.typ": {
      contents: "#set document(title: {{ title | String }})\n\n#{{ body | Content }}"
    }
  }
};

engine.registerQuill(JSON.stringify(quillJson));

// Parse markdown
const markdown = `---
title: My Document
---

# Hello World

This is my document.
`;

const parsed = Quillmark.parseMarkdown(markdown);

// Render to PDF
const result = engine.render(parsed, { format: 'pdf' });

// Access PDF bytes
const pdfBytes = result.artifacts[0].bytes;
```

## API Reference

### Quillmark Class

Main engine class for managing Quills and rendering documents.

#### Constructor

```typescript
constructor()
```

Creates a new Quillmark engine instance with auto-registered backends.

**Example:**
```javascript
const engine = new Quillmark();
```

#### Static Methods

##### parseMarkdown

```typescript
static parseMarkdown(markdown: string): ParsedDocument
```

Parse markdown with YAML frontmatter.

**Parameters:**
- `markdown` - Markdown content with optional YAML frontmatter

**Returns:** ParsedDocument with fields and optional quillTag

**Throws:** Error if YAML is invalid

**Example:**
```javascript
const parsed = Quillmark.parseMarkdown(`---
title: Example
author: Alice
---

# Content
`);

console.log(parsed.fields.title);  // "Example"
console.log(parsed.fields.body);   // "# Content"
```

#### Instance Methods

##### registerQuill

```typescript
registerQuill(quillJson: string | object): void
```

Register a Quill template from JSON.

**Parameters:**
- `quillJson` - JSON string or object with Quill file structure

**Throws:** Error if Quill validation fails

**Example:**
```javascript
const quill = {
  files: {
    "Quill.toml": {
      contents: `[Quill]
name = "demo"
backend = "typst"
description = "Demo quill"
`
    },
    "glue.typ": { contents: "#{{ body | Content }}" }
  }
};

engine.registerQuill(quill);
// or
engine.registerQuill(JSON.stringify(quill));
```

##### getQuillInfo

```typescript
getQuillInfo(name: string): QuillInfo
```

Get information about a registered Quill.

**Parameters:**
- `name` - Registered Quill name

**Returns:** QuillInfo object with Quill details

**Throws:** Error if Quill not found

**Example:**
```javascript
const info = engine.getQuillInfo("my-quill");
console.log(info.backend);           // "typst"
console.log(info.supportedFormats);  // ["pdf", "svg"]
```

##### processGlue

```typescript
processGlue(quillName: string, markdown: string): string
```

Process markdown through glue template only (no compilation).

**Parameters:**
- `quillName` - Name of registered Quill
- `markdown` - Markdown content

**Returns:** Processed glue output (backend-specific code)

**Example:**
```javascript
const glue = engine.processGlue("my-quill", markdown);
console.log(glue);  // Typst code
```

##### render

```typescript
render(parsedDoc: ParsedDocument, options?: RenderOptions): RenderResult
```

Render a parsed document to artifacts.

**Parameters:**
- `parsedDoc` - Parsed markdown document
- `options` - Optional rendering options

**Returns:** RenderResult with artifacts and warnings

**Throws:** Error if rendering fails

**Example:**
```javascript
const parsed = Quillmark.parseMarkdown(markdown);
const result = engine.render(parsed, { 
  format: 'pdf',
  quillName: 'my-quill'
});

// Save PDF
const blob = new Blob([result.artifacts[0].bytes], { type: 'application/pdf' });
```

##### listQuills

```typescript
listQuills(): string[]
```

Get list of registered Quill names.

**Returns:** Array of Quill names

**Example:**
```javascript
const quills = engine.listQuills();
console.log(quills);  // ["my-quill", "__default__"]
```

##### unregisterQuill

```typescript
unregisterQuill(name: string): void
```

Remove a registered Quill.

**Parameters:**
- `name` - Quill name to unregister

**Example:**
```javascript
engine.unregisterQuill("my-quill");
```

## Type Definitions

### ParsedDocument

```typescript
interface ParsedDocument {
  fields: object;      // YAML frontmatter fields (includes body)
  quillTag?: string;   // Value of QUILL field (if present)
}
```

**Example:**
```javascript
const parsed = Quillmark.parseMarkdown(markdown);
console.log(parsed.fields.title);   // Access frontmatter
console.log(parsed.fields.body);    // Access body
console.log(parsed.quillTag);       // Access QUILL field
```

### QuillInfo

```typescript
interface QuillInfo {
  name: string;
  backend: string;
  metadata: object;
  example?: string;
  fieldSchemas: object;
  supportedFormats: Array<'pdf' | 'svg' | 'txt'>;
}
```

**Example:**
```javascript
const info = engine.getQuillInfo("my-quill");
console.log(`Backend: ${info.backend}`);
console.log(`Formats: ${info.supportedFormats.join(', ')}`);
```

### RenderOptions

```typescript
interface RenderOptions {
  format?: 'pdf' | 'svg' | 'txt';
  assets?: Record<string, Uint8Array>;
  quillName?: string;
}
```

**Parameters:**
- `format` - Output format (defaults to first supported format)
- `assets` - Additional runtime assets as byte arrays
- `quillName` - Override QUILL field or use specific Quill

**Example:**
```javascript
const result = engine.render(parsed, {
  format: 'pdf',
  quillName: 'custom-template',
  assets: {
    'logo.png': logoBytes
  }
});
```

### RenderResult

```typescript
interface RenderResult {
  artifacts: Artifact[];
  warnings: Diagnostic[];
  renderTimeMs: number;
}
```

**Example:**
```javascript
const result = engine.render(parsed);
console.log(`Rendered in ${result.renderTimeMs}ms`);
console.log(`Artifacts: ${result.artifacts.length}`);

if (result.warnings.length > 0) {
  result.warnings.forEach(w => console.warn(w.message));
}
```

### Artifact

```typescript
interface Artifact {
  format: 'pdf' | 'svg' | 'txt';
  bytes: Uint8Array;
  mimeType: string;
}
```

**Example:**
```javascript
const artifact = result.artifacts[0];
console.log(`Format: ${artifact.format}`);
console.log(`MIME: ${artifact.mimeType}`);
console.log(`Size: ${artifact.bytes.length} bytes`);

// Save as blob
const blob = new Blob([artifact.bytes], { type: artifact.mimeType });
```

### Diagnostic

```typescript
interface Diagnostic {
  severity: 'error' | 'warning' | 'note';
  code?: string;
  message: string;
  location?: Location;
  hint?: string;
  sourceChain?: string[];
}
```

**Example:**
```javascript
result.warnings.forEach(diag => {
  console.log(`[${diag.severity.toUpperCase()}] ${diag.message}`);
  if (diag.hint) {
    console.log(`  Hint: ${diag.hint}`);
  }
  if (diag.location) {
    console.log(`  at ${diag.location.file}:${diag.location.line}:${diag.location.col}`);
  }
});
```

### Location

```typescript
interface Location {
  file?: string;
  line: number;
  col: number;
}
```

## Error Handling

All errors include structured diagnostic information:

```javascript
try {
  const result = engine.render(parsed, { format: 'pdf' });
} catch (error) {
  console.error(`Error: ${error.message}`);
  
  // Errors include diagnostic information
  if (error.location) {
    console.error(`  at ${error.location.file}:${error.location.line}:${error.location.col}`);
  }
  
  if (error.hint) {
    console.error(`  Hint: ${error.hint}`);
  }
  
  // Some errors have multiple diagnostics
  if (error.diagnostics) {
    error.diagnostics.forEach(diag => {
      console.error(`  - ${diag.severity}: ${diag.message}`);
    });
  }
}
```

## Quill JSON Format

Quills are represented as JSON with a `files` structure:

```javascript
const quillJson = {
  files: {
    // Required: Quill.toml configuration
    "Quill.toml": { 
      contents: `[Quill]
name = "my-quill"
backend = "typst"
description = "My template"
glue_file = "glue.typ"
` 
    },
    
    // Required: Glue template
    "glue.typ": { 
      contents: "#set document(title: {{ title | String }})\n\n#{{ body | Content }}" 
    },
    
    // Optional: Assets directory
    "assets": {
      "logo.png": { 
        contents: [137, 80, 78, 71, ...]  // Binary data as byte array
      },
      "fonts": {
        "CustomFont.ttf": { 
          contents: [...]  // Binary font data
        }
      }
    },
    
    // Optional: Example markdown
    "example.md": { 
      contents: "---\ntitle: Example\n---\n\n# Content" 
    }
  }
};
```

### File Node Types

- **Text file**: `{ contents: "string content" }`
- **Binary file**: `{ contents: [byte, array] }`
- **Directory**: Object containing nested files
- **Empty directory**: `{}`

## Complete Example

```javascript
import { Quillmark } from '@quillmark-test/wasm';

async function renderDocument() {
  // Create engine
  const engine = new Quillmark();
  
  // Load quill
  const quillJson = {
    files: {
      "Quill.toml": {
        contents: `[Quill]
name = "simple-doc"
backend = "typst"
description = "Simple document template"
glue_file = "glue.typ"

[fields]
title = { description = "Document title", type = "str" }
author = { description = "Author name", type = "str" }
`
      },
      "glue.typ": {
        contents: `#set document(title: {{ title | String }}, author: {{ author | String }})
#set page(margin: 1in)
#set text(font: "Arial", size: 11pt)

#align(center)[
  #text(size: 18pt, weight: "bold")[{{ title | String }}]
  
  #text(size: 12pt)[{{ author | String }}]
]

#{{ body | Content }}
`
      }
    }
  };
  
  engine.registerQuill(quillJson);
  
  // Parse markdown
  const markdown = `---
title: My Research Paper
author: Dr. Jane Smith
---

# Introduction

This is the introduction to my paper.

## Background

Some background information here.
`;
  
  const parsed = Quillmark.parseMarkdown(markdown);
  
  // Check available Quills
  console.log('Registered Quills:', engine.listQuills());
  
  // Get Quill info
  const info = engine.getQuillInfo('simple-doc');
  console.log('Supported formats:', info.supportedFormats);
  
  // Render to PDF
  const result = engine.render(parsed, { 
    format: 'pdf',
    quillName: 'simple-doc'
  });
  
  console.log(`Rendered in ${result.renderTimeMs}ms`);
  
  // Handle warnings
  if (result.warnings.length > 0) {
    console.warn('Warnings:');
    result.warnings.forEach(w => console.warn(`  - ${w.message}`));
  }
  
  // Save PDF (browser)
  const blob = new Blob([result.artifacts[0].bytes], { type: 'application/pdf' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'output.pdf';
  a.click();
  URL.revokeObjectURL(url);
}

renderDocument().catch(console.error);
```

## Browser Usage

```html
<!DOCTYPE html>
<html>
<head>
  <title>Quillmark Demo</title>
</head>
<body>
  <textarea id="markdown" rows="20" cols="80">---
title: My Document
---

# Hello World
</textarea>
  <button id="render">Render PDF</button>
  
  <script type="module">
    import { Quillmark } from './node_modules/@quillmark-test/wasm/quillmark_wasm.js';
    
    document.getElementById('render').addEventListener('click', () => {
      const engine = new Quillmark();
      const markdown = document.getElementById('markdown').value;
      const parsed = Quillmark.parseMarkdown(markdown);
      
      // Uses default Typst quill
      const result = engine.render(parsed, { format: 'pdf' });
      
      // Download PDF
      const blob = new Blob([result.artifacts[0].bytes], { type: 'application/pdf' });
      const url = URL.createObjectURL(blob);
      window.open(url);
    });
  </script>
</body>
</html>
```

## Node.js Usage

```javascript
import { Quillmark } from '@quillmark-test/wasm';
import { writeFileSync } from 'fs';

const engine = new Quillmark();
const markdown = `---
title: Server-Side Document
---

# Generated on Server
`;

const parsed = Quillmark.parseMarkdown(markdown);
const result = engine.render(parsed, { format: 'pdf' });

writeFileSync('output.pdf', result.artifacts[0].bytes);
console.log('PDF saved!');
```

## Performance

Typical performance metrics:

- **Typical render time**: 50-200ms for standard documents
- **Memory usage**: ~10-50MB depending on Quill complexity
- **Package size**: ~5-10MB WASM binary

### Optimization Tips

1. **Reuse engine instances** - Create once, render multiple times
2. **Unregister unused Quills** - Free memory with `unregisterQuill()`
3. **Minimize asset sizes** - Optimize images and fonts
4. **Batch operations** - Render multiple documents efficiently

## TypeScript Support

The package includes TypeScript definitions:

```typescript
import { Quillmark, ParsedDocument, RenderOptions, RenderResult } from '@quillmark-test/wasm';

const engine: Quillmark = new Quillmark();
const parsed: ParsedDocument = Quillmark.parseMarkdown(markdown);
const result: RenderResult = engine.render(parsed, { format: 'pdf' });
```

## Next Steps

- [Quickstart Guide](../getting-started/quickstart.md)
- [Creating Quills](../guides/creating-quills.md)
- [Python API Reference](../python/api.md)
- [Rust API Documentation](https://docs.rs/quillmark/latest/quillmark/)
