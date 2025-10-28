# AcroForm Backend

The AcroForm backend fills PDF form fields using templates, making it easy to generate filled forms from structured data.

## Overview

The AcroForm backend:

- Reads existing PDF forms (AcroForms)
- Templates field values using MiniJinja
- Fills forms with data from YAML frontmatter
- Returns completed PDF forms

This is ideal for standardized forms like applications, certificates, and government documents.

## Basic Usage

Specify `backend = "acroform"` in your `Quill.toml`:

```toml
[Quill]
name = "my-form"
backend = "acroform"
description = "Employee information form"
example_file = "example.md"
```

## Quill Structure

An AcroForm Quill requires a PDF form file:

```
my-form-quill/
├── Quill.toml
├── form.pdf          # PDF form with fillable fields
└── example.md        # Example usage
```

The `form.pdf` file must be a PDF with AcroForm fields (fillable form fields).

## How It Works

The AcroForm backend uses two templating strategies:

### 1. Tooltip-Based Templating

Field descriptions (tooltips) can contain templates using the format `description__{{template}}`:

**PDF Form Field:**
- Name: `employee_name`
- Tooltip: `Employee full name__{{ first_name }} {{ last_name }}`

**Frontmatter:**
```yaml
first_name: John
last_name: Doe
```

**Result:** The `employee_name` field is filled with "John Doe"

### 2. Value-Based Templating

If a field has no tooltip template, its current value is used as a template:

**PDF Form Field:**
- Name: `department`
- Value: `{{ dept }}`

**Frontmatter:**
```yaml
dept: Engineering
```

**Result:** The `department` field is filled with "Engineering"

## Example Quill

### Quill.toml

```toml
[Quill]
name = "employee-form"
backend = "acroform"
description = "Employee information form"
example_file = "example.md"

[fields]
first_name = { description = "First name", type = "str" }
last_name = { description = "Last name", type = "str" }
employee_id = { description = "Employee ID", type = "str" }
department = { description = "Department name", type = "str" }
hire_date = { description = "Hire date", type = "str" }
```

### example.md

```markdown
---
first_name: Jane
last_name: Smith
employee_id: EMP-12345
department: Engineering
hire_date: 2025-01-15
---

This is an example of filling an employee form.
```

### Creating the PDF Form

You can create PDF forms using:

- **Adobe Acrobat** - Form editing tools
- **LibreOffice Writer** - Export as PDF with form fields
- **PDF form creators** - Online tools and desktop applications

Each fillable field should have a unique name that matches your template variables.

## Field Naming

Field names in your PDF form should:

- Use clear, descriptive names
- Match your frontmatter field names
- Use underscores instead of spaces (e.g., `first_name` not `first name`)
- Be unique across the form

Example field names:
- `employee_name`
- `date_of_birth`
- `home_address`
- `phone_number`

## Template Syntax

Templates use MiniJinja syntax:

### Simple Variables

```
{{ variable_name }}
```

### Concatenation

```
{{ first_name }} {{ last_name }}
```

### Conditional Content

```
{% if has_middle_name %}{{ middle_name }}{% endif %}
```

### Formatting

```
Employee ID: {{ employee_id | upper }}
```

## Supported Field Types

The AcroForm backend supports standard PDF form field types:

- **Text fields** - Single or multi-line text
- **Checkboxes** - Boolean values (use `true`/`false` in frontmatter)
- **Radio buttons** - Single selection from options
- **Dropdowns** - Selection from a list

## Rendering Forms

### Python

```python
from quillmark import Quillmark, ParsedDocument, OutputFormat

engine = Quillmark()
quill = engine.load_quill_from_path("path/to/form-quill")
engine.register_quill(quill)

markdown = """---
first_name: Jane
last_name: Smith
employee_id: EMP-12345
---
"""

parsed = ParsedDocument.from_markdown(markdown)
workflow = engine.workflow_from_quill_name("employee-form")
result = workflow.render(parsed, OutputFormat.PDF)

with open("filled-form.pdf", "wb") as f:
    f.write(result.artifacts[0].bytes)
```

### Rust

```rust
use quillmark::{Quillmark, OutputFormat, ParsedDocument};
use quillmark_core::Quill;

let mut engine = Quillmark::new();
let quill = Quill::from_path("path/to/form-quill")?;
engine.register_quill(quill);

let markdown = r#"---
first_name: Jane
last_name: Smith
employee_id: EMP-12345
---
"#;

let parsed = ParsedDocument::from_markdown(markdown)?;
let workflow = engine.workflow_from_quill_name("employee-form")?;
let result = workflow.render(&parsed, Some(OutputFormat::Pdf))?;

std::fs::write("filled-form.pdf", &result.artifacts[0].bytes)?;
```

## Best Practices

1. **Test your PDF form** - Verify all fields are fillable before using with Quillmark
2. **Use descriptive field names** - Match frontmatter field names to PDF form fields
3. **Add tooltips** - Use tooltip templates for complex field values
4. **Validate inputs** - Define field schemas in `Quill.toml`
5. **Provide examples** - Include example.md with sample data

## Limitations

- Only supports PDF AcroForm format (not XFA forms)
- Cannot modify form structure (only fills existing fields)
- Cannot add new fields or pages
- Field formatting depends on PDF form settings

## Use Cases

Perfect for:

- **Government forms** - Applications, certificates, permits
- **Business forms** - Invoices, purchase orders, contracts
- **HR documents** - Employee records, timesheets
- **Healthcare forms** - Patient intake, consent forms
- **Educational forms** - Transcripts, applications

## Troubleshooting

### Fields Not Filling

- Check field names match between PDF and frontmatter
- Verify the PDF has fillable form fields (not just text annotations)
- Ensure field names don't contain special characters

### Template Errors

- Check MiniJinja syntax in field values or tooltips
- Verify variable names in templates match frontmatter
- Test templates with simple values first

### PDF Issues

- Confirm the PDF is an AcroForm (not XFA or static)
- Check PDF permissions allow form filling
- Verify the PDF isn't corrupted

## Resources

- [AcroForm crate documentation](https://crates.io/crates/acroform)
- [MiniJinja template syntax](https://docs.rs/minijinja/)
- [Adobe Acrobat form creation](https://helpx.adobe.com/acrobat/using/creating-distributing-pdf-forms.html)

## Next Steps

- [Create your own AcroForm Quill](creating-quills.md)
- [Learn about Quill Markdown](quill-markdown.md)
- [Explore the Typst backend](typst-backend.md)
