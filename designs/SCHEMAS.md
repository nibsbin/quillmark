# Schema and Validation

This document describes the configuration and data schemas used in Quillmark.

## Backend

## Backend Trait Schema

The Backend trait defines the interface for implementing backends in the quillmark system. Implementations must define the following configurations:

- id -> str: Returns the backend identifier (e.g., "typst", "latex").
- supported_formats -> str[]: Returns the supported output formats.
- glue_extension_types -> str[]: Returns the glue file extensions (e.g., ".typ", ".tex"). Returns an empty array to disable custom glue files.
- allow_auto_glue -> bool: Whether to allow automatic auto glue generation.

## Quill

Quills encapsulate the metadata, configuration, and behavior for generating a specific formatted document. They specify how the inputted ParsedDocument should be composed and compiled to produce a final document. A Quill's `Quill.toml` file specifies the following configuration:

- name -> str: The name of the Quill.
    - Upon registering the Quill to a Quillmark instance, ensure the name has not already been registered
- description -> Option[str]: A brief description of the Quill.
- author -> Option[str]: The author of the Quill.
- version -> Option[str]: The version of the Quill.
- backend -> str: The backend identifier to use.
    - Upon registering the Quill to a Quillmark instance, ensure the backend is already registered
- glue_file -> Option[str]: Path to a custom glue file. If not provided, automatic glue generation is used. Validation:
    - Ensure extension is in the backend's `glue_extension_types`
    - If not provided, ensure `backend.allow_auto_glue` is true
- example_markdown -> Option[str]: Path to an example markdown file demonstrating the Quill's capabilities. Developers should include usage instructions in the content for human and LLM consumers.
- json_schema -> Option[str]: Path to a comprehensive json schema file that overrides `[fields]`. If `json_schema` and fields are defined, emit a warning that the fields are overrided by the json_schema.

### Quill Field

Developers can define the schema for ParsedDocument input within the `fields` dictionary. This schema will be used for ParsedDocument validation.

- name -> str: This is the key; e.g. for the TOML section `[fields.title]`, the name would be "title".
- description -> str: A description of the field.
- type -> "str", "array", "dict", "date", "datetime", or "number": The value type of the field
- default -> any: The default value for the field. If defined, this makes the field not required.

