# Schema and Validation

This document describes the configuration and data schemas used in Quillmark.

## Backend

### Backend Trait Schema

The Backend trait defines the interface for implementing backends in the quillmark system. Implementations must define the following configurations:

- id -> str: Returns the backend identifier (e.g., "typst", "latex").
- supported_formats -> str[]: Returns the supported output formats.
- glue_extension_types -> str[]: Returns the glue file extensions (e.g., ".typ", ".tex"). Returns an empty array to disable custom glue files.
- allow_auto_glue -> bool: Whether to allow automatic glue generation.

## Quill

Quills encapsulate the metadata, configuration, and behavior for generating a specific formatted document. They specify how the inputted ParsedDocument should be composed and compiled to produce a final document. A Quill's `Quill.toml` file specifies the following configuration:

- name -> str: The name of the Quill (required).
    - Upon registering the Quill to a Quillmark instance, ensure the name has not already been registered
- description -> str: A brief description of the Quill (required, cannot be empty).
- backend -> str: The backend identifier to use (required).
    - Upon registering the Quill to a Quillmark instance, ensure the backend is already registered
- author -> Option[str]: The author of the Quill.
- version -> Option[str]: The version of the Quill.
- glue_file -> Option[str]: Path to a custom glue file. If not provided, automatic glue generation is used. Validation:
    - Ensure extension is in the backend's `glue_extension_types`
    - If not provided, ensure `backend.allow_auto_glue` is true
- example_file -> Option[str]: Path to an example markdown file demonstrating the Quill's capabilities. Developers should include usage instructions in the content for human and LLM consumers.

### Quill Field

Developers can define the schema for ParsedDocument input within the `[fields]` section of Quill.toml. This schema will be used for ParsedDocument validation and to build a JSON schema.

Field properties:
- name -> str: This is the key; e.g., for the TOML section `[fields.title]`, the name would be "title".
- description -> str: A description of the field.
- type -> "str", "array", "dict", "date", "datetime", or "number": The value type of the field.
- default -> any: The default value for the field. If defined, this makes the field optional (not required).
- example -> any: An example value for the field (added to the examples array in the JSON schema).
- ui -> Option[Table]: A table containing UI-specific metadata (see below).

**UI Configuration (Nested `[ui]` table):**
- group -> Option[str]: UI group/section name for organizing fields (e.g., "Personal Info").
- tooltip -> Option[str]: Short hint text for the field (concise hint, unlike verbose description).
- order -> Option[int]: Ordering index for sorting fields in the UI (auto-generated from TOML field position).

**Implementation Status:**
| Property | Status |
|----------|--------|
| group | ✅ Implemented |
| tooltip | ✅ Implemented |
| order | ✅ Implemented (auto-generated) |
| component | ❌ Not yet implemented |

**Type Mapping (TOML to JSON Schema):**
- "str" → "string"
- "number" → "number"
- "array" → "array"
- "dict" → "object"
- "date" → "string" with format "date"
- "datetime" → "string" with format "date-time"

**Required Field Logic:**
- If a field has a `default` value: field is optional
- If a field has no `default` value: field is required

### JSON Schema Custom Properties

Field schemas support a custom `x-ui` property for UI metadata that is included in the generated JSON schema. This property contains the serialized content of the `[ui]` table from the TOML configuration.

- `x-ui`: An object containing UI metadata (group, tooltip, order, etc.)

This property follows the JSON Schema specification for custom extensions. Validation logic ignores this property, but frontend UIs consume it for dynamic wizard generation.

**Example**:
```json
{
  "type": "object",
  "properties": {
    "author": {
      "type": "string",
      "description": "The full name of the document author...",
      "default": "Anonymous",
      "x-ui": {
        "group": "Author Info",
        "tooltip": "Your full name",
        "order": 1
      }
    }
  }
}
```
