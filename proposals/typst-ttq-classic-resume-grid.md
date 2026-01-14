# Proposal: Typst ttq-classic-resume Grid Consolidation

## Summary

Consolidate `category_grid` and `item_grid` into a unified `content_grid` component that renders structured content based on input shape, supporting the new BODY-centric card design.

---

## Current State

### Existing Components

```typst
// Category Grid - for key/value pairs (skills)
#let category_grid(items: (), columns: 2)
// items: ((key: "Programming", value: "Python, R"), ...)

// Item Grid - for flat lists (certifications)
#let item_grid(items: (), columns: 2)
// items: ("OSCP", "GCTI", ...)
```

### Problems

1. **Redundant APIs** - Both components do grid layout with minor rendering differences
2. **Rigid input shape** - `category_grid` expects exactly `(key, value)` tuples
3. **No content passthrough** - Cannot accept native Typst content from BODY
4. **Caller decides** - plate.typ must choose which component to call

---

## Proposed Design

### Option A: Unified `content_grid` with Structured Input

A single component that accepts pre-parsed items from the Quillmark engine.

```typst
#let content_grid(
  items: (),
  columns: 2,
  style: "auto",  // "auto" | "categorized" | "flat"
) = {
  // Detect style from item shape if "auto"
  let resolved_style = if style == "auto" {
    if items.len() > 0 and type(items.at(0)) == dictionary {
      "categorized"
    } else {
      "flat"
    }
  } else {
    style
  }

  vgap(config.entry_spacing)

  let render_cell(item) = {
    if resolved_style == "categorized" {
      block({
        text(weight: "bold", item.key)
        linebreak()
        item.value
      })
    } else {
      item
    }
  }

  grid(
    columns: (1fr,) * columns,
    row-gutter: if resolved_style == "categorized" {
      config.leading + config.entry_spacing
    } else {
      config.leading
    },
    column-gutter: 1em,
    ..items.map(render_cell)
  )
}
```

**Usage from plate.typ:**

```typst
// Quillmark provides _grid_items based on BODY parsing
#content_grid(
  items: {{ card._grid_items | GridItems }},
  columns: {{ card.columns | Int | default: 2 }},
)
```

**Pros:**
- Single API surface
- Auto-detection reduces plate.typ complexity
- Backwards compatible via explicit `style` parameter

**Cons:**
- Requires Quillmark to pre-parse BODY into `_grid_items`
- Magic auto-detection could surprise users

---

### Option B: Native Content with Typst Introspection

Accept raw Typst content and use Typst's introspection to determine rendering.

```typst
#let content_grid(
  body: none,
  columns: 2,
) = {
  // This approach requires Typst content introspection
  // which is limited - we can't easily "peek" at content structure

  // Would need show rules or content inspection APIs
  // that don't cleanly exist in Typst today
}
```

**Verdict:** Not recommended. Typst's content model doesn't support easy structural introspection. Parsing should happen in Quillmark, not Typst.

---

### Option C: Keep Separate Components, Modernize APIs

Retain `category_grid` and `item_grid` but improve their flexibility.

```typst
// Enhanced category_grid - variable item count, optional values
#let category_grid(
  items: (),
  columns: 2,
  show_keys: true,
) = {
  vgap(config.entry_spacing)

  let cell(item) = {
    block({
      if show_keys and "key" in item {
        text(weight: "bold", item.key)
        linebreak()
      }
      if "value" in item {
        item.value
      } else if "content" in item {
        item.content
      }
    })
  }

  grid(
    columns: (1fr,) * columns,
    row-gutter: config.leading + config.entry_spacing,
    column-gutter: 1em,
    ..items.map(cell)
  )
}

// Enhanced item_grid - accepts content or strings
#let item_grid(
  items: (),
  columns: 2,
  bullet: none,  // Optional bullet prefix
) = {
  vgap(config.entry_spacing)

  let cell(item) = {
    if bullet != none {
      [#bullet #item]
    } else {
      item
    }
  }

  grid(
    columns: (1fr,) * columns,
    row-gutter: config.leading,
    column-gutter: 1em,
    ..items.map(cell)
  )
}
```

**Pros:**
- Explicit, no magic
- Easier to understand and debug
- plate.typ logic remains clear

**Cons:**
- Two components to maintain
- plate.typ still needs conditional logic

---

## Recommendation

**Option A (Unified `content_grid`)** with explicit `style` parameter.

The auto-detection is a convenience, but callers can always specify `style: "categorized"` or `style: "flat"` explicitly. This gives us:

1. Single component to maintain
2. Consistent API for plate.typ
3. Flexibility for future grid styles (e.g., "compact", "spaced")

---

## Implementation Changes

### File: `src/components.typ`

```typst
// --- Grid Components ---

// Unified content grid for skills, certifications, and list content
// Replaces: category_grid, item_grid
#let content_grid(
  items: (),
  columns: 2,
  style: "auto",  // "auto" | "categorized" | "flat"
) = {
  if items.len() == 0 { return }

  // Resolve style from item shape
  let resolved_style = if style == "auto" {
    if type(items.at(0)) == dictionary and "key" in items.at(0) {
      "categorized"
    } else {
      "flat"
    }
  } else {
    style
  }

  vgap(config.entry_spacing)

  let render_cell(item) = {
    if resolved_style == "categorized" {
      block({
        text(weight: "bold", item.key)
        linebreak()
        if type(item.value) == content {
          item.value
        } else {
          [#item.value]
        }
      })
    } else {
      if type(item) == content {
        item
      } else {
        [#item]
      }
    }
  }

  grid(
    columns: (1fr,) * columns,
    row-gutter: if resolved_style == "categorized" {
      config.leading + config.entry_spacing
    } else {
      config.leading
    },
    column-gutter: 1em,
    ..items.map(render_cell)
  )
}

// Deprecation shims (remove in next major version)
#let category_grid(items: (), columns: 2) = {
  content_grid(items: items, columns: columns, style: "categorized")
}

#let item_grid(items: (), columns: 2) = {
  content_grid(items: items, columns: columns, style: "flat")
}
```

### File: `src/lib.typ`

```typst
#import "components.typ": (
  resume_header,
  section_header,
  timeline_entry,
  project_entry,
  content_grid,
  // Deprecated but exported for backwards compatibility
  category_grid,
  item_grid,
)
```

---

## Migration Path

1. **v0.1.x** - Add `content_grid`, keep `category_grid`/`item_grid` as shims
2. **v0.2.0** - Deprecation warnings in documentation
3. **v0.3.0** - Remove shims, `content_grid` only

---

## Testing Checklist

- [ ] `content_grid` with categorized items renders correctly
- [ ] `content_grid` with flat items renders correctly
- [ ] `style: "auto"` correctly detects item shape
- [ ] Empty items array renders nothing (no crash)
- [ ] Columns parameter works for 1, 2, 3, 4 column layouts
- [ ] Deprecation shims produce identical output to new component
- [ ] Rich content (links, emphasis) renders in values

---

## Open Questions

1. **Should `style` support additional values?** e.g., `"compact"` for tighter spacing
2. **Should we support mixed items?** (some categorized, some flat in same grid)
3. **Column responsiveness?** Should columns auto-adjust based on content width?
