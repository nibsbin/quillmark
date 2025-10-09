# Quillmark MCP Server Design

> **Status**: Design Phase  
> **Package Name**: `quillmark-mcp`  
> **Target**: Python 3.10+  
> **Protocol**: Model Context Protocol (MCP)  

## Executive Summary

This document outlines the design for `quillmark-mcp`, a Model Context Protocol (MCP) server that exposes Quillmark's rendering capabilities to AI models and MCP clients. The server enables AI assistants to help users create properly formatted Markdown documents with correct frontmatter fields, discover available templates, and render documents to PDF or SVG formats.

**Design Goals:**
- Expose Quillmark functionality through MCP tools for AI model consumption
- Provide rich context about Quill templates including fields and default templates
- Enable rendering with comprehensive error diagnostics to help users fix issues
- Support template discovery and exploration workflows
- Use the existing `quillmark` PyPI package for all rendering operations
- Support the Extended YAML Metadata Standard with SCOPE and QUILL keys for structured content

**Non-Goals:**
- Custom template creation or modification through MCP (v1.0)
- Dynamic asset management beyond what's in registered Quills
- Async streaming for long-running renders (renders are typically <100ms)
- Custom backend implementations
- Provide your own diagnostic system (use Quillmark's built-in diagnostics)

---

## Table of Contents

1. [Purpose](#purpose)
2. [Architecture](#architecture)
3. [MCP Tools](#mcp-tools)
4. [Data Models](#data-models)
5. [Error Handling](#error-handling)
6. [Implementation Details](#implementation-details)
7. [Usage Examples](#usage-examples)
8. [Performance Considerations](#performance-considerations)
9. [Security Considerations](#security-considerations)

---

## Purpose

The Quillmark MCP server serves three primary use cases:

### 1. Template Discovery & Exploration

AI models can query available Quill templates and their metadata to help users:
- Discover what templates are available (letters, memos, reports, etc.)
- Understand what frontmatter fields each template requires
- Access default/example markdown content for each template
- Learn about template-specific features and capabilities

### 2. Context Provision for Authoring

Provide AI models with the necessary context to assist users in writing markdown:
- Required and optional frontmatter fields for a specific Quill
- Field types, descriptions, and validation rules
- Example values and formatting guidance
- Template-specific markdown syntax and features
- Extended YAML Metadata Standard capabilities (SCOPE for collections, QUILL for template selection)

### 3. Document Rendering & Validation

Enable AI models to render documents and help users fix errors:
- Render markdown to PDF or SVG using a specific Quill
- Return detailed error diagnostics with line/column information
- Provide hints and suggestions from Quillmark's diagnostic system
- Validate frontmatter structure before rendering

---

## Architecture

### Component Diagram

```
┌──────────────────────────────┐
│       AI Model / Client      │
│    (via MCP protocol)        │
└──────────────┬───────────────┘
               │ MCP JSON-RPC
               ▼
┌──────────────────────────────┐
│     Quillmark MCP Server     │
│  ┌──────────────────────────┐│
│  │    MCP Tool Handlers     ││
│  │  - list_quills           ││
│  │  - get_quill_info        ││
│  │  - get_quill_template    ││
│  │  - render_document       ││
│  │  - validate_frontmatter  ││
│  └──────────────────────────┘│
│  ┌──────────────────────────┐│
│  │   Quillmark Engine       ││
│  │  (via Python package)    ││
│  └──────────────────────────┘│
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│    quillmark Python Package  │
│  - Quillmark engine          │
│  - Workflow rendering        │
│  - Quill management          │
│  - Error diagnostics         │
└──────────────────────────────┘
```

### Design Principles

1. **MCP-Native**: Follow MCP protocol specifications for tool definitions and responses
2. **Rich Diagnostics**: Leverage Quillmark's diagnostic system for helpful error messages
3. **Stateless Operations**: Each tool call is independent (except for initial Quill registration)
4. **JSON-Based**: All data exchanged via JSON for MCP compatibility

---

## MCP Tools

### Tool 1: `list_quills`

List all registered Quill templates available on the server.

**Input Parameters:**
```json
{
  "include_metadata": boolean  // Optional, default: false
}
```

**Output:**
```json
{
  "quills": [
    {
      "name": "appreciated_letter",
      "backend": "typst",
      "description": "Professional business letter template",  // If include_metadata=true
      "tags": ["letter", "professional"]  // If include_metadata=true
    }
  ]
}
```

**Use Case:** AI model discovers what templates are available to suggest to the user.

---

### Tool 2: `get_quill_info`

Get detailed information about a specific Quill template, including its frontmatter schema.

**Input Parameters:**
```json
{
  "quill_name": string  // Required
}
```

**Output:**
```json
{
  "name": "appreciated_letter",
  "backend": "typst",
  "metadata": {
    "description": "Professional business letter template",
    "author": "Quillmark Contributors",
    "version": "1.0.0",
    "tags": ["letter", "professional"]
  },
  "frontmatter_fields": {
    "sender": {
      "type": "string",
      "required": true,
      "description": "Sender's name and address",
      "example": "Jane Smith, Universal Exports, 1 Heavy Plaza, Morristown, NJ 07964"
    },
    "recipient": {
      "type": "string",
      "required": true,
      "description": "Recipient's name and address",
      "example": "Mr. John Doe\n123 Main Street\nSpringfield, IL 62701"
    },
    "date": {
      "type": "string",
      "required": true,
      "description": "Letter date",
      "example": "Morristown, June 9th, 2023"
    },
    "subject": {
      "type": "string",
      "required": true,
      "description": "Letter subject line",
      "example": "Revision of our Procurement Contract"
    },
    "name": {
      "type": "string",
      "required": true,
      "description": "Sender's name and title",
      "example": "Jane Smith, Regional Director"
    }
  },
  "supported_formats": ["PDF", "SVG"]
}
```

**Implementation Note:** The frontmatter schema is derived by:
1. Loading the Quill's glue template
2. Parsing MiniJinja template variables (e.g., `{{ sender | String }}`)
3. Optionally reading from a schema file if present in the Quill
4. Extracting examples from the default template markdown file

**Use Case:** AI model helps user understand what fields to include in their markdown frontmatter.

---

### Tool 3: `get_quill_template`

Get the default/example markdown template for a Quill.

**Input Parameters:**
```json
{
  "quill_name": string  // Required
}
```

**Output:**
```json
{
  "quill_name": "appreciated_letter",
  "template_markdown": "---\nsender: Jane Smith, Universal Exports...\n---\n\nDear Joe,\n\n..."
}
```

**Implementation Note:** Returns the content of the file specified in the Quill.toml's `template` field (e.g., `appreciated_letter.md`).

**Use Case:** AI model provides the user with a starting template to modify.

---

### Tool 4: `render_document`

Render a markdown document to PDF or SVG using a specified Quill.

**Input Parameters:**
```json
{
  "quill_name": string,        // Optional - Quill name to use for rendering
  "markdown": string,          // Required - the markdown content with frontmatter
  "output_format": string,     // Optional - "PDF" or "SVG", defaults to "PDF"
  "validate_only": boolean     // Optional - if true, only validate without rendering
}
```

**Quill Name Resolution:**
- If `quill_name` is provided, use that Quill for rendering
- If `quill_name` is omitted, the rendering engine will just use the `QUILL` YAML frontmatter field in the markdown and return an error if missing

**Example with QUILL directive:**
```markdown
---
QUILL: appreciated_letter
sender: Jane Smith
recipient: John Doe
---

Letter content...
```

**Extended YAML Metadata Standard:**

Quillmark supports the Extended YAML Metadata Standard for structured content with collections. Documents can include multiple scoped blocks using the `SCOPE` key:

```markdown
---
title: Product Catalog
---

Main document description.

---
SCOPE: products
name: Widget
price: 19.99
---

Widget description with *markdown formatting*.

---
SCOPE: products
name: Gadget
price: 29.99
---

Gadget description.
```

Key features:
- **SCOPE key**: Creates collections (arrays) of structured content blocks
- **QUILL key**: Specifies which quill template to use for rendering
- **Contiguity requirement**: Metadata blocks must be contiguous (no blank lines within YAML content)
- **Horizontal rule disambiguation**: `---` preceded by blank line is a horizontal rule, not metadata
- Each scoped block contains metadata fields plus a `body` field
- Collections preserve document order
- See [PARSE.md](PARSE.md) for complete documentation

**Success Output:**
```json
{
  "success": true,
  "artifacts": [
    {
      "format": "PDF",
      "bytes_base64": "JVBERi0xLjcKCjEgMCBvYmogICUg...",
      "mime_type": "application/pdf",
      "size_bytes": 45231
    }
  ],
}
```

**Error Output:**
```json
{
  "success": false,
  "error_type": "CompilationError",
  "error_message": "Typst compilation failed",
  "diagnostics": [
    {
      "severity": "ERROR",
      "message": "undefined variable 'recepient'",
      "code": "undefined_var",
      "location": {
        "file": "input.md",
        "line": 3,
        "column": 1
      },
      "hint": "Did you mean 'recipient'? Check your frontmatter field names against the template requirements."
    }
  ]
}
```

**Implementation:** Uses `quillmark.Quillmark` and `Workflow.render()` from the Python package.

**Use Case:** AI model renders the document and provides detailed feedback to help the user fix any errors.

---

### Tool 5: `validate_frontmatter`

Validate frontmatter against a Quill's schema without rendering.

**Input Parameters:**
```json
{
  "quill_name": string,  // Optional - Quill name for validation
  "markdown": string     // Required - can be just frontmatter or full document
}
```

**Output:**
```json
{
  "valid": true,
  "parsed_fields": {
    "sender": "Jane Smith, Universal Exports...",
    "recipient": "Mr. John Doe...",
    "date": "Morristown, June 9th, 2023",
    "subject": "Revision of our Procurement Contract",
    "name": "Jane Smith, Regional Director"
  },
  "missing_required_fields": [],
}
```

**Error Output:**
```json
{
  "valid": false,
  "parsed_fields": {
    "sender": "Jane Smith"
  },
  "missing_required_fields": ["recipient", "date", "subject", "name"],
  "errors": [
    {
      "severity": "ERROR",
      "message": "Required field 'recipient' is missing",
      "code": "missing_field",
      "hint": "Add 'recipient: <value>' to your frontmatter"
    }
  ]
}
```

**Use Case:** Quick validation loop for the AI to help users fix frontmatter issues before attempting a full render.

---

## Data Models

### Diagnostic

Used across all error responses:

```python
class Diagnostic:
    severity: Literal["ERROR", "WARNING", "NOTE"]
    message: str
    code: str | None
    location: Location | None
    hint: str | None
```

### Location

```python
class Location:
    file: str | None
    line: int
    column: int
```

### QuillInfo

```python
class QuillInfo:
    name: str
    backend: str
    metadata: dict[str, Any]
    frontmatter_fields: dict[str, FieldSchema]
    supported_formats: list[str]
```

### FieldSchema

```python
class FieldSchema:
    type: str  # "string", "number", "boolean", "object", "array"
    required: bool
    description: str
    example: Any
    default: Any | None
```

---

## Error Handling

### Error Categories

1. **Invalid Quill Name**: Quill not registered or doesn't exist
1. **Parse Error**: YAML frontmatter is malformed
1. **Validation Error**: Frontmatter fields don't match schema
1. **Template Error**: MiniJinja template processing failed
1. **Compilation Error**: Backend (Typst) compilation failed
1. **System Error**: File I/O, permissions, or other system issues

### Error Response Pattern

All errors follow this structure:

```json
{
  "success": false,
  "error_type": "ParseError" | "TemplateError" | "CompilationError" | "ValidationError",
  "error_message": "Human-readable summary",
  "diagnostics": [
    {
      "severity": "ERROR",
      "message": "Specific error description",
      "code": "error_code",
      "location": {
        "file": "input.md",
        "line": 5,
        "column": 12
      },
      "hint": "Suggestion for fixing the error"
    }
  ]
}
```

### Hint Generation Strategy

Lean on the rendering engine's diagnostics.

---

## Implementation Details

### Technology Stack

- **Language**: Python 3.10+
- **MCP Library**: `mcp` Python package (official MCP SDK)
- **Rendering Engine**: `quillmark` Python package
- **Server Framework**: MCP server implementation
- **Serialization**: `pydantic` for data validation and JSON schema

### Project Structure

```
quillmark-mcp/
├── src/
│   └── quillmark_mcp/
│       ├── __init__.py
│       ├── server.py          # MCP server setup
│       ├── tools/
│       │   ├── __init__.py
│       │   ├── list_quills.py
│       │   ├── get_quill_info.py
│       │   ├── get_template.py
│       │   ├── render.py
│       │   └── validate.py
│       ├── models.py          # Pydantic models
│       ├── schema_extractor.py  # Extract schema from Quill
│       └── error_formatter.py   # Format diagnostics for MCP
├── tests/
│   ├── test_list_quills.py
│   ├── test_get_info.py
│   ├── test_render.py
│   └── fixtures/
├── pyproject.toml
├── README.md
└── .gitignore
```

### Dependencies

```toml
[project]
name = "quillmark-mcp"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = [
    "quillmark>=0.1.0",
    "mcp>=0.1.0",
    "pydantic>=2.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=8.0",
    "pytest-asyncio>=0.23",
    "mypy>=1.8",
    "ruff>=0.3",
]
```

### Quill Registration

Quills must be registered with the MCP server at startup. Configuration options:

**Option 1: Configuration File**
```toml
# mcp_config.toml
[quills]
path = "/path/to/quills"
auto_register = true

[[quills.manual]]
name = "appreciated_letter"
path = "/path/to/appreciated_letter"
```

**Option 2: Environment Variables**
```bash
QUILLMARK_MCP_QUILLS_PATH=/path/to/quills
QUILLMARK_MCP_AUTO_REGISTER=true
```

**Option 3: Programmatic Registration**
```python
from quillmark_mcp import QuillmarkMCPServer

server = QuillmarkMCPServer()
server.register_quills_from_directory("/path/to/quills")
server.start()
```

### Schema Extraction Implementation

Extract frontmatter schema from a Quill:

```python
def extract_schema(quill: Quill) -> dict[str, FieldSchema]:
    """Extract frontmatter schema from Quill glue template and example."""
    
    # 1. Parse glue template for MiniJinja variables
    glue_content = quill.glue_template
    variables = extract_template_variables(glue_content)
    
    # 2. Read example template if available
    template_path = quill.metadata.get("template")
    examples = {}
    if template_path:
        example_content = quill.get_file(template_path)
        parsed = ParsedDocument.from_markdown(example_content)
        examples = parsed.fields()
    
    # 3. Check for optional schema.json
    schema_override = {}
    if quill.has_file("schema.json"):
        schema_override = json.loads(quill.get_file("schema.json"))
    
    # 4. Merge and create FieldSchema objects
    schema = {}
    for var_name, var_info in variables.items():
        schema[var_name] = FieldSchema(
            type=infer_type(var_info, examples.get(var_name)),
            required=var_name not in schema_override.get("optional", []),
            description=schema_override.get(var_name, {}).get("description", ""),
            example=examples.get(var_name),
            default=schema_override.get(var_name, {}).get("default")
        )
    
    return schema
```

### Python API Surface

The `quillmark` Python package exposes the following classes and exceptions:

**Core Classes:**
- `Quillmark` - Engine for managing backends and quills
- `Workflow` - Render execution API
- `Quill` - Template bundle representation
- `ParsedDocument` - Parsed markdown with frontmatter
- `RenderResult` - Rendering output with artifacts and warnings
- `Artifact` - Individual output file (PDF, SVG, etc.)
- `Diagnostic` - Error/warning diagnostic with location
- `Location` - File/line/column position

**Enums:**
- `OutputFormat` - PDF, SVG, etc.
- `Severity` - ERROR, WARNING, NOTE

**Exceptions:**
- `QuillmarkError` - Base exception
- `ParseError` - YAML parsing errors
- `TemplateError` - Template processing errors
- `CompilationError` - Backend compilation errors

All classes are imported from the `quillmark` module:
```python
from quillmark import Quillmark, Workflow, Quill, ParsedDocument
from quillmark import QuillmarkError, ParseError, TemplateError, CompilationError
```

### Error Mapping

Map Quillmark Python exceptions to MCP error responses:

```python
from quillmark import QuillmarkError, ParseError, TemplateError, CompilationError

def format_error_for_mcp(error: Exception) -> dict:
    """Convert Quillmark exception to MCP error response."""
    
    if isinstance(error, ParseError):
        return {
            "success": False,
            "error_type": "ParseError",
            "error_message": str(error),
            "diagnostics": extract_diagnostics(error)
        }
    
    elif isinstance(error, TemplateError):
        return {
            "success": False,
            "error_type": "TemplateError",
            "error_message": str(error),
            "diagnostics": extract_diagnostics(error),
        }
    
    elif isinstance(error, CompilationError):
        return {
            "success": False,
            "error_type": "CompilationError",
            "error_message": str(error),
            "diagnostics": extract_diagnostics(error)
        }
    
    else:
        return {
            "success": False,
            "error_type": "QuillmarkError",
            "error_message": str(error),
            "diagnostics": []
        }

def extract_diagnostics(error: Exception) -> list[dict]:
    """Extract diagnostics from Quillmark RenderResult or error context.
    
    Note: Diagnostics are available through RenderResult.warnings for successful
    renders. For errors, diagnostic information should be accessed through the
    error's context if available (implementation-specific).
    """
    # Implementation depends on how diagnostics are exposed in the Python API
    # This is a placeholder for the actual implementation
    return []
```

---

## Usage Examples

### Example 1: AI Helps User Create a Letter

**User:** "Help me write a business letter"

**AI Model's Tool Calls:**

1. Call `list_quills()` → discovers "appreciated_letter"
2. Call `get_quill_info("appreciated_letter")` → learns required fields
3. Call `get_quill_template("appreciated_letter")` → gets example template with `QUILL: appreciated_letter` directive
4. AI generates markdown based on user's needs (includes `QUILL:` directive from template)
5. Call `validate_frontmatter(markdown=...)` → checks if frontmatter is correct (no quill_name needed)
6. Call `render_document(markdown=...)` → produces PDF (no quill_name needed)

**AI Response:** "I've created a professional business letter for you and rendered it as a PDF. Here's the document..."

---

### Example 2: User Has Rendering Error

**User provides markdown with a typo:**
```markdown
---
QUILL: appreciated_letter
sender: John Doe
recepient: Jane Smith  # Typo: should be "recipient"
---

Dear Jane,
...
```

**AI Model's Tool Calls:**

1. Call `render_document(markdown=...)` → returns error (note: no quill_name needed)

**Error Response:**
```json
{
  "success": false,
  "error_type": "TemplateError",
  "diagnostics": [
    {
      "severity": "ERROR",
      "message": "undefined variable 'recipient'",
      "hint": "Did you mean 'recipient'? You have a typo in your frontmatter field name."
    }
  ]
}
```

**AI Response:** "I found an error in your document. You have a typo: 'recepient' should be 'recipient'. Let me fix that for you..."

---

### Example 3: Discovery Workflow

**User:** "What templates are available?"

**AI Model's Tool Call:**
```python
list_quills(include_metadata=True)
```

**Response:**
```json
{
  "quills": [
    {
      "name": "appreciated_letter",
      "backend": "typst",
      "description": "Professional business letter template",
      "tags": ["letter", "professional"]
    },
    {
      "name": "usaf_memo",
      "backend": "typst",
      "description": "US Air Force memorandum template",
      "tags": ["memo", "military", "official"]
    }
  ]
}
```

**AI Response:** "You have 2 templates available:
1. **appreciated_letter** - Professional business letter template
2. **usaf_memo** - US Air Force memorandum template

Which one would you like to use?"

---

### Example 4: Using QUILL Directive

**User provides markdown with QUILL directive:**
```markdown
---
QUILL: appreciated_letter
sender: Jane Smith, Universal Exports, 1 Heavy Plaza, Morristown, NJ 07964
recipient: |
    Mr. John Doe
    123 Main Street
    Springfield, IL 62701
date: Morristown, June 9th, 2023
subject: Revision of our Producrement Contract
name: Jane Smith, Regional Director
---

Dear Joe...
```

**AI Model's Tool Call:**
```json
{
  "markdown": "---\nQUILL: appreciated_letter\nsender: Jane Smith...",
  "output_format": "PDF"
}
```

**Note:** No `quill_name` parameter is needed because the markdown contains `QUILL: appreciated_letter` in the frontmatter.

**Success Response:**
```json
{
  "success": true,
  "artifacts": [
    {
      "format": "PDF",
      "bytes_base64": "JVBERi0xLjcKCjEgMCBvYmogICUg...",
      "mime_type": "application/pdf",
      "size_bytes": 45231
    }
  ],
}
```

**AI Response:** "I've rendered your letter as a PDF using the 'appreciated_letter' template specified in your document."

---

## Performance Considerations

### Caching Strategy

- **Quill Information**: Cache schema extraction results (invalidate on Quill reload)
- **Template Content**: Cache template markdown files in memory
- **Engine Instances**: Reuse Quillmark engine instance across requests

### Expected Performance

- **list_quills**: < 10ms (in-memory lookup)
- **get_quill_info**: < 50ms (includes schema extraction)
- **get_quill_template**: < 10ms (cached file read)
- **validate_frontmatter**: < 20ms (YAML parsing only)
- **render_document**: < 100ms (full render with Typst backend)

### Optimization Opportunities

1. Pre-extract schemas at server startup
2. Keep parsed templates in memory
3. Use Quillmark's workflow caching
4. Batch multiple validation checks when possible

---

## Security Considerations

### Input Validation

- **Markdown Size Limit**: Restrict input markdown to reasonable size (e.g., 1MB)
- **Frontmatter Depth**: Limit YAML nesting depth to prevent billion laughs attack
- **File Path Sanitization**: Never allow path traversal in quill names or template paths
- **Resource Limits**: Set memory and CPU limits for rendering operations

### Output Safety

- **Binary Output**: Base64-encode all binary artifacts (PDF, SVG)
- **Error Messages**: Sanitize file paths in error messages (show relative paths only)
- **Diagnostic Filtering**: Don't expose internal system paths or sensitive info

### Access Control

- **Quill Registration**: Only server operator can register Quills (not exposed via MCP)
- **Read-Only Operations**: All MCP tools are read-only (no file writes through API)
- **Sandboxing**: Consider running Typst compilation in sandboxed environment

### Dependencies

- **Quillmark Package**: Trust boundary - assumes `quillmark` package is secure
- **MCP Library**: Use official MCP SDK and keep updated
- **Typst Backend**: Inherits Typst's security properties (package installation, etc.)

---

## Future Enhancements

### Phase 2 Features

- **Custom Schema Files**: Support `schema.json` in Quill directories for explicit field definitions
- **Multi-format Rendering**: Return both PDF and SVG in single render call
- **Template Snippets**: Get partial templates for specific document sections
- **Asset Management**: Tools to list and retrieve Quill assets
- **Batch Rendering**: Render multiple documents in parallel
- **Scoped Content Tools**: Tools to help AI models construct documents with SCOPE collections
- **Dynamic Assets**: Support for runtime asset injection via Workflow API (note: currently not exposed in Python bindings)

### Phase 3 Features

- **Template Authoring**: Tools to create/modify Quills through MCP
- **Live Preview**: Streaming updates for document editing workflows
- **Custom Filters**: Register custom MiniJinja filters via MCP
- **Analytics**: Track usage patterns and common errors

---

## References

- **MCP Specification**: https://spec.modelcontextprotocol.io/
- **Quillmark Architecture**: See [DESIGN.md](DESIGN.md) - Complete architecture and core design principles
- **Parsing & Extended YAML**: See [PARSE.md](PARSE.md) - Detailed parsing and Extended YAML Metadata Standard documentation
- **Quillmark Python Package**: See `https://github.com/nibsbin/quillmark/blob/main/quillmark-python/README.md`
- **Python API Design**: See `https://github.com/nibsbin/quillmark/blob/main/designs/PYTHON.md`
- **Error Handling**: See [ERROR.md](ERROR.md) - Error handling system documentation and implementation guide
- **Quill Structure**: See `https://github.com/nibsbin/quillmark/blob/main/designs/QUILL_DESIGN.md`

---

## Implementation Roadmap

### Milestone 1: Core Infrastructure (Week 1-2)
- [ ] Set up project structure with pyproject.toml
- [ ] Implement MCP server skeleton with basic tool registration
- [ ] Create Pydantic models for all data types
- [ ] Set up testing infrastructure with fixtures

### Milestone 2: Basic Tools (Week 3-4)
- [ ] Implement `list_quills` tool
- [ ] Implement `get_quill_template` tool
- [ ] Implement basic error formatting
- [ ] Add integration tests with quillmark package

### Milestone 3: Schema & Validation (Week 5-6)
- [ ] Implement schema extraction from glue templates
- [ ] Implement `get_quill_info` tool
- [ ] Implement `validate_frontmatter` tool
- [ ] Add comprehensive unit tests

### Milestone 4: Rendering (Week 7-8)
- [ ] Implement `render_document` tool
- [ ] Map all Quillmark errors to MCP format
- [ ] Add error hint generation
- [ ] End-to-end testing with real Quills

### Milestone 5: Polish & Documentation (Week 9-10)
- [ ] Performance optimization and caching
- [ ] Security audit and hardening
- [ ] Complete API documentation
- [ ] Usage examples and tutorials
- [ ] Deployment guide

---

## Changelog

### 2024-10-09 - Consistency Update
- Updated to reflect Extended YAML Metadata Standard (SCOPE/QUILL keys)
- Added documentation for parsing contiguity requirements
- Updated error handling to match current Python API
- Added Python API surface documentation
- Updated references to include DESIGN.md and PARSE.md
- Clarified diagnostic severity levels (ERROR, WARNING, NOTE)
- Added horizontal rule disambiguation documentation

### 2024-10-09 - Initial Design
- Created comprehensive design document
- Defined all MCP tools and their signatures
- Established error handling patterns
- Outlined implementation approach
