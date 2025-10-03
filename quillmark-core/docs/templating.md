# Templating Module

MiniJinja-based template composition with stable filter API.

## Overview

The `templating` module provides the `Glue` type for template rendering and a stable
filter API for backends to register custom filters.

## Key Types

- **`Glue`**: Template rendering engine wrapper
- **`TemplateError`**: Template-specific error types
- **`filter_api`**: Stable API for filter registration (no direct minijinja dependency)

## Examples

### Basic Template Rendering

```rust,no_run
use quillmark_core::Glue;
use std::collections::HashMap;

let template = r#"
#set document(title: {{ title | String }})

{{ body | Content }}
"#;

let mut glue = Glue::new(template.to_string());

// Register filters (done by backends)
// glue.register_filter("String", string_filter);
// glue.register_filter("Content", content_filter);

let mut context = HashMap::new();
context.insert("title".to_string(), serde_yaml::Value::String("My Doc".into()));
context.insert("body".to_string(), serde_yaml::Value::String("Content".into()));

let output = glue.compose(context).unwrap();
```

### Custom Filter Implementation

```rust,no_run
use quillmark_core::templating::filter_api::{State, Value, Kwargs, Error, ErrorKind};
# use quillmark_core::Glue;
# let mut glue = Glue::new("template".to_string());

fn uppercase_filter(
    _state: &State,
    value: Value,
    _kwargs: Kwargs,
) -> Result<Value, Error> {
    let s = value.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "Expected string")
    })?;
    Ok(Value::from(s.to_uppercase()))
}

// Register with glue
glue.register_filter("uppercase", uppercase_filter);
```

## Filter API

The `filter_api` module provides a stable ABI that external crates can depend on
without requiring a direct minijinja dependency.

### Filter Function Signature

```rust,ignore
type FilterFn = fn(
    &filter_api::State,
    filter_api::Value,
    filter_api::Kwargs,
) -> Result<filter_api::Value, minijinja::Error>;
```

## Error Types

- **`RenderError`**: Template rendering error from MiniJinja
- **`InvalidTemplate`**: Template compilation failed
- **`FilterError`**: Filter execution error
