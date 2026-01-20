# Analysis: Pure JSON Schema Configuration vs. TOML

**Question:** What if we threw out TOML schema configuration and used JSON Schema directly?

---

## What This Would Solve

### ✅ 1. TOML Nested Structure Verbosity (ELIMINATED)

**Current TOML pain:**
```toml
cells.type = "array"
cells.items.type = "object"
cells.items.properties.category = { type = "string", title = "Category" }
cells.items.properties.skills = { type = "string", title = "Skills" }
```

**JSON Schema (natural nesting):**
```json
{
  "cells": {
    "type": "array",
    "items": {
      "type": "object",
      "properties": {
        "category": { "type": "string", "title": "Category" },
        "skills": { "type": "string", "title": "Skills" }
      }
    }
  }
}
```

**Impact:** Nested structures become natural instead of verbose.

---

### ✅ 2. Field Schema Reusability (BUILT-IN)

**Current TOML limitation:**
```toml
# Must copy-paste address structure for each field
[fields.shipping_address]
type = "object"
properties.street = { type = "string" }
properties.city = { type = "string" }

[fields.billing_address]
type = "object"
properties.street = { type = "string" }  # Copy-paste
properties.city = { type = "string" }
```

**JSON Schema ($ref/$defs):**
```json
{
  "$defs": {
    "address": {
      "type": "object",
      "properties": {
        "street": { "type": "string" },
        "city": { "type": "string" },
        "zip": { "type": "string" }
      }
    }
  },

  "properties": {
    "shippingAddress": { "$ref": "#/$defs/address" },
    "billingAddress": { "$ref": "#/$defs/address" }
  }
}
```

**Impact:** Eliminates duplication, native composition.

---

### ✅ 3. Full Validation Expressiveness (UNLOCKED)

**Current TOML limitation:** Can't express advanced validation

**JSON Schema (all keywords available):**
```json
{
  "password": {
    "type": "string",
    "minLength": 8,
    "maxLength": 128,
    "pattern": "^(?=.*[A-Z])(?=.*[0-9])(?=.*[!@#$%]).*$",
    "description": "Must contain uppercase, number, and special char"
  },

  "email": {
    "type": "string",
    "format": "email"
  },

  "age": {
    "type": "number",
    "minimum": 0,
    "maximum": 150,
    "multipleOf": 1
  },

  "tags": {
    "type": "array",
    "minItems": 1,
    "maxItems": 10,
    "uniqueItems": true
  }
}
```

**Impact:** No need to push validation to template layer.

---

### ✅ 4. Conditional Schema (ENABLED)

**JSON Schema if/then/else:**
```json
{
  "properties": {
    "classification": { "type": "string", "enum": ["U", "C", "S", "TS"] },
    "classificationGuide": { "type": "string" }
  },

  "if": {
    "properties": { "classification": { "not": { "const": "U" } } }
  },
  "then": {
    "required": ["classificationGuide"]
  }
}
```

**Use case:** "If classification is not Unclassified, require classificationGuide"

**Impact:** Express field dependencies natively.

---

### ✅ 5. IDE Autocomplete & Validation (IMMEDIATE)

**JSON Schema has a schema:**
- VSCode/IntelliJ: Auto-complete properties, types, keywords
- Real-time validation as you type
- Hover documentation for keywords
- Schema-aware refactoring

**TOML:** No schema standard, limited IDE support

**Impact:** Authoring experience dramatically improved for developers.

---

### ✅ 6. Standard Tooling Ecosystem (UNLOCKED)

**Available tools:**
- [ajv](https://ajv.js.org/) - Validation library
- [json-schema-faker](https://github.com/json-schema-faker/json-schema-faker) - Generate fake data
- [quicktype](https://quicktype.io/) - Generate types from schema
- [Stoplight Studio](https://stoplight.io/) - Visual schema editor
- [openapi-generator](https://github.com/OpenAPITools/openapi-generator) - Code generation

**Impact:** Leverage existing ecosystem instead of building custom tooling.

---

### ✅ 7. No Impedance Mismatch (ELIMINATED)

**Current flow:**
```
TOML → Parse → QuillConfig → Build JSON Schema → Use
      ^^^ Conversion layer adds complexity
```

**Direct JSON Schema:**
```
JSON Schema → Parse → Use
              ^^^ No conversion
```

**Impact:** What you write IS what gets used. Simpler mental model.

---

## What This Would NOT Solve

### ❌ 1. First-Block QUILL Detection (Correct Design)
**Reason:** Essential architecture - first block must specify QUILL for template selection. This is not a bug, it's required for parsing to know which schema to validate against.

### ❌ 2. UPPERCASE Reserved Keywords
**Reason:** Markdown parsing design decision

### ❌ 3. Tag Naming Restrictions
**Reason:** Backend template language compatibility

### ❌ 4. Card vs. Field Mental Model
**Reason:** Core data model concept, not config format

### ❌ 5. Horizontal Rule Ambiguity
**Reason:** Markdown delimiter conflict

### ❌ 6. Fence Detection Strictness
**Reason:** Markdown parsing simplification

**Summary:** Solves ALL TOML-specific issues, but NONE of the markdown parsing issues.

---

## Proposed Configuration Format

### Option A: Pure JSON Schema with Quillmark Wrapper

```json
{
  "$schema": "https://quillmark.dev/schema/quill-v1.json",
  "$comment": "Quillmark configuration using JSON Schema",

  "quill": {
    "name": "classic_resume",
    "backend": "typst",
    "description": "A clean and modern resume template",
    "version": "1.0.0",
    "author": "John Doe",
    "plateFile": "plate.typ",
    "exampleFile": "example.md"
  },

  "backend": {
    "typst": {
      "packages": ["@preview/bubble:0.2.2"]
    }
  },

  "$defs": {
    "address": {
      "type": "object",
      "properties": {
        "street": { "type": "string" },
        "city": { "type": "string" },
        "state": { "type": "string" },
        "zip": { "type": "string", "pattern": "^[0-9]{5}$" }
      },
      "required": ["street", "city"]
    }
  },

  "type": "object",
  "properties": {
    "name": {
      "type": "string",
      "title": "Full Name",
      "description": "Candidate's full name",
      "x-ui": { "group": "Personal Info", "order": 1 }
    },

    "contacts": {
      "type": "array",
      "title": "Contact Information",
      "items": { "type": "string" },
      "minItems": 1,
      "examples": [["john@example.com", "(555) 123-4567"]],
      "x-ui": { "group": "Personal Info", "order": 2 }
    },

    "address": {
      "$ref": "#/$defs/address",
      "x-ui": { "group": "Personal Info", "order": 3 }
    }
  },

  "required": ["name", "contacts"],

  "cards": {
    "$defs": {
      "experience_section": {
        "title": "Experience/Education Entry",
        "description": "An entry with heading, subheading, and bullets",
        "type": "object",
        "properties": {
          "title": {
            "type": "string",
            "title": "Section Title",
            "default": "Experience"
          },
          "headingLeft": {
            "type": "string",
            "title": "Company/School",
            "description": "Organization name"
          },
          "headingRight": {
            "type": "string",
            "title": "Location"
          },
          "subheadingLeft": {
            "type": "string",
            "title": "Job Title"
          },
          "subheadingRight": {
            "type": "string",
            "title": "Dates",
            "pattern": "^[A-Za-z]+ [0-9]{4}( – [A-Za-z]+ [0-9]{4})?$",
            "examples": ["January 2020 – Present"]
          }
        },
        "required": ["headingLeft"]
      },

      "skills_section": {
        "title": "Skills Grid",
        "description": "Grid of skill categories",
        "type": "object",
        "properties": {
          "title": {
            "type": "string",
            "default": "Skills"
          },
          "cells": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "category": { "type": "string", "title": "Category" },
                "skills": { "type": "string", "title": "Skills" }
              },
              "required": ["category", "skills"]
            },
            "minItems": 1
          }
        },
        "x-ui": { "hideBody": true }
      }
    }
  }
}
```

### File: `Quill.schema.json`

---

## Comparison: Current vs. Proposed

| Aspect | TOML (Current) | JSON Schema (Proposed) |
|--------|----------------|------------------------|
| **Simple fields** | ⭐⭐⭐⭐⭐ Excellent | ⭐⭐⭐ Good (more verbose) |
| **Nested structures** | ⭐⭐ Poor (verbose) | ⭐⭐⭐⭐⭐ Excellent (natural) |
| **Reusability** | ⭐ None | ⭐⭐⭐⭐⭐ Built-in ($ref) |
| **Validation** | ⭐⭐ Basic types | ⭐⭐⭐⭐⭐ Full expressiveness |
| **Comments** | ⭐⭐⭐⭐⭐ Native | ⭐⭐ Only $comment |
| **Readability** | ⭐⭐⭐⭐⭐ Very readable | ⭐⭐⭐ More brackets |
| **IDE support** | ⭐⭐ Limited | ⭐⭐⭐⭐⭐ Excellent |
| **Tooling** | ⭐ Custom only | ⭐⭐⭐⭐⭐ Rich ecosystem |
| **Learning curve** | ⭐⭐⭐⭐ Shallow | ⭐⭐⭐ Steeper |
| **Non-dev friendly** | ⭐⭐⭐⭐⭐ Yes | ⭐⭐ Not really |

---

## User Persona Impact

### 👨‍💻 Web Developers (Primary Users)
**Current TOML:** ⭐⭐⭐ "It's fine but verbose for complex schemas"
**JSON Schema:** ⭐⭐⭐⭐⭐ "This is what I already know! Much better."

### 🔬 Data Scientists
**Current TOML:** ⭐⭐⭐⭐ "Similar to config files I use (pyproject.toml, conda.yaml)"
**JSON Schema:** ⭐⭐⭐ "JSON is okay but I prefer YAML for configs"

### ✍️ Content Creators / Writers
**Current TOML:** ⭐⭐⭐⭐ "Looks approachable, like a simple config"
**JSON Schema:** ⭐ "Too many brackets, looks like code"

### 🚀 DevOps Engineers
**Current TOML:** ⭐⭐⭐⭐ "Familiar from Rust/Cargo ecosystem"
**JSON Schema:** ⭐⭐⭐⭐ "JSON is universal, tooling is great"

### 🤖 LLM Generation
**Current TOML:** ⭐⭐⭐⭐ "Can generate TOML fine"
**JSON Schema:** ⭐⭐⭐⭐⭐ "JSON Schema is training data, very natural"

---

## Implementation Strategy

### Phase 1: Dual Support (Backward Compatible)

Accept BOTH formats:
- `Quill.toml` - Parse as TOML (current)
- `Quill.schema.json` - Parse as JSON Schema (new)

**Code changes:**
```rust
// In Quill::from_tree()
if files.file_exists("Quill.schema.json") {
    QuillConfig::from_json_schema(content)?
} else if files.file_exists("Quill.toml") {
    QuillConfig::from_toml(content)?
} else {
    Err("No Quill configuration found")
}
```

**Migration path:**
- Existing templates keep using TOML
- New templates can use JSON Schema
- Provide converter: `quillmark convert Quill.toml --to-json-schema`

---

### Phase 2: Recommendation Shift

- Document JSON Schema as preferred for complex templates
- TOML remains supported for simple templates
- Update examples to show both

---

### Phase 3 (Optional): TOML Deprecation

- Only if community overwhelmingly prefers JSON Schema
- Long deprecation period (1+ years)
- Clear migration guide

---

## Alternative: YAML as Middle Ground

**Advantages over JSON:**
- Supports comments
- More readable (less punctuation)
- Superset of JSON (valid JSON is valid YAML)

**Advantages over TOML:**
- Better nested structure handling
- Familiar to web/DevOps communities
- Established ecosystem

**Example:**
```yaml
# Quill.schema.yaml
$schema: https://quillmark.dev/schema/quill-v1.json

quill:
  name: classic_resume
  backend: typst
  description: A clean and modern resume template
  plateFile: plate.typ

$defs:
  address:
    type: object
    properties:
      street: { type: string }
      city: { type: string }
      zip: { type: string, pattern: '^[0-9]{5}$' }

properties:
  name:
    type: string
    title: Full Name
    x-ui:
      group: Personal Info
      order: 1

  address:
    $ref: '#/$defs/address'

cards:
  $defs:
    experience_section:
      title: Experience Entry
      type: object
      properties:
        company: { type: string }
        dates: { type: string }
```

**Comparison:**

| Format | Readability | Nesting | Comments | Tooling | Non-devs |
|--------|-------------|---------|----------|---------|----------|
| TOML | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ |
| JSON | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ |
| YAML | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |

---

## Recommendation

### 🎯 Implement Dual Format Support

**Reasoning:**
1. **JSON Schema** solves real architectural pain (nesting, reusability, validation)
2. **TOML** still valuable for simple templates and non-developers
3. Let users choose based on complexity:
   - Simple template (5-10 fields)? Use TOML
   - Complex template (30+ fields, nested objects)? Use JSON Schema
   - Medium complexity? Either works

**Migration strategy:**
1. Add JSON Schema parser (`QuillConfig::from_json_schema()`)
2. Auto-detect format based on filename
3. Provide converter tool
4. Document both in examples
5. Recommend JSON Schema for new complex templates

**Benefits:**
- ✅ Solves TOML verbosity for power users
- ✅ Preserves TOML simplicity for basic users
- ✅ Unlocks JSON Schema ecosystem
- ✅ No breaking changes
- ✅ Progressive enhancement

---

## Implementation Outline

### 1. Define Quillmark JSON Schema Spec

```json
{
  "$id": "https://quillmark.dev/schema/quill-v1.json",
  "$schema": "https://json-schema.org/draft/2019-09/schema",
  "title": "Quillmark Configuration",
  "description": "Configuration for a Quillmark template bundle",

  "type": "object",
  "required": ["quill", "properties"],

  "properties": {
    "quill": {
      "type": "object",
      "required": ["name", "backend", "description"],
      "properties": {
        "name": { "type": "string", "pattern": "^[a-z_][a-z0-9_]*$" },
        "backend": { "type": "string" },
        "description": { "type": "string", "minLength": 1 },
        "version": { "type": "string", "default": "0.1.0" },
        "author": { "type": "string", "default": "Unknown" },
        "plateFile": { "type": "string" },
        "exampleFile": { "type": "string" }
      }
    },

    "backend": {
      "type": "object",
      "description": "Backend-specific configuration"
    },

    "$defs": {
      "type": "object",
      "description": "Reusable schema definitions"
    },

    "properties": {
      "type": "object",
      "description": "Document field schemas (JSON Schema properties)"
    },

    "required": {
      "type": "array",
      "items": { "type": "string" }
    },

    "cards": {
      "type": "object",
      "properties": {
        "$defs": {
          "type": "object",
          "description": "Card type definitions"
        }
      }
    }
  }
}
```

### 2. Implement Parser

```rust
// crates/core/src/quill.rs

impl QuillConfig {
    pub fn from_json_schema(json_content: &str) -> Result<Self, Box<dyn Error>> {
        let config: serde_json::Value = serde_json::from_str(json_content)?;

        // Extract quill metadata
        let quill_section = config.get("quill")
            .ok_or("Missing 'quill' section")?;

        let name = quill_section.get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'name' in quill section")?
            .to_string();

        let backend = quill_section.get("backend")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'backend' in quill section")?
            .to_string();

        // ... extract other metadata ...

        // Parse document schema (JSON Schema properties)
        let properties = config.get("properties")
            .ok_or("Missing 'properties' section")?;

        let fields = Self::parse_json_schema_properties(properties)?;

        // Parse card definitions
        let cards = if let Some(cards_section) = config.get("cards") {
            Self::parse_card_defs(cards_section)?
        } else {
            HashMap::new()
        };

        // Build document schema
        let document = CardSchema {
            name: name.clone(),
            title: Some(name),
            description: Some(description),
            fields,
            ui: None, // Extract from x-ui if present
        };

        Ok(QuillConfig {
            document,
            backend,
            version,
            author,
            example_file,
            plate_file,
            cards,
            metadata,
            typst_config,
        })
    }

    fn parse_json_schema_properties(
        properties: &serde_json::Value
    ) -> Result<HashMap<String, FieldSchema>, Box<dyn Error>> {
        // Convert JSON Schema properties to FieldSchema
        // Handle $ref resolution
        // Extract x-ui metadata
        // ...
    }
}
```

### 3. Auto-Detection

```rust
impl Quill {
    pub fn from_tree(files: FileTreeNode) -> Result<Self, Box<dyn Error>> {
        // Try JSON Schema first
        if let Some(json_schema_content) = files.get_file("Quill.schema.json") {
            let config = QuillConfig::from_json_schema(
                std::str::from_utf8(json_schema_content)?
            )?;
            return Self::from_config(config, files);
        }

        // Fall back to TOML
        if let Some(toml_content) = files.get_file("Quill.toml") {
            let config = QuillConfig::from_toml(
                std::str::from_utf8(toml_content)?
            )?;
            return Self::from_config(config, files);
        }

        Err("No Quill configuration found (Quill.schema.json or Quill.toml)".into())
    }
}
```

---

## Conclusion

**Switching to JSON Schema solves ALL TOML-specific architectural issues:**
- ✅ Nested structure verbosity → Natural JSON nesting
- ✅ No reusability → Built-in $ref/$defs
- ✅ Limited validation → Full JSON Schema keywords
- ✅ Poor tooling → Rich ecosystem

**But doesn't solve markdown parsing issues** (asymmetric blocks, reserved keywords, etc.)

**Best approach: Dual format support**
- Simple templates continue using TOML
- Complex templates gain JSON Schema power
- Users choose based on needs
- No breaking changes
- Progressive enhancement path

**Estimated effort:**
- JSON Schema parser: 3-5 days
- Format detection: 1 day
- Tests: 2-3 days
- Documentation: 2-3 days
- **Total: ~2 weeks**

**Impact:** High-value feature for power users, unlocks JSON Schema ecosystem.
