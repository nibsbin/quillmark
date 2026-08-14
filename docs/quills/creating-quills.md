# Creating Quills

A Quill is a format bundle that defines how your Markdown content is rendered. This tutorial walks from an empty directory to a rendered PDF.

## 1. Create the directory

Start with this layout:

```
my-quill/
├── Quill.yaml
└── plate.typ
```

## 2. Write `Quill.yaml`

Create a minimal but complete config:

```yaml
quill:
  name: my_quill
  backend: typst
  version: "1.0.0"
  description: A simple letter format

typst:
  plate_file: plate.typ

main:
  fields:
    sender:
      type: string
      description: Name of the sender
    recipient:
      type: string
      description: Name of the recipient
    date:
      type: date       # a bare calendar date (YYYY-MM-DD); use `datetime` for wall-clock time-of-day
      description: Letter date
```

`name`, `backend`, `version`, and `description` are all required. `name` must be `snake_case`. Define your document's expected root-block fields under `main.fields`. Each field has a `type`, optional `default`, `description`, and validation constraints. Use `integer` for whole numbers only and `number` for values that may include decimals. For the full list of field types, UI hints, typed arrays, and enum constraints, see the [Quill.yaml Reference](quill-yaml-reference.md).

Use `default` for the value most authors will accept as-is (filled in when the field is omitted). Use `example` to document the expected shape without supplying a default. A field with neither is one nobody has answered: the blueprint flags it with a `!must_fill` marker and `Quill::validate` warns while it stays unauthored. Set `must_fill:` explicitly where you want a default a human must still confirm, or an optional field with nothing to suggest — it is a warning either way, never a gate. See the [Quill.yaml Reference](quill-yaml-reference.md#obligation-must_fill) for details.

### Picking a text type

Four types hold text, and two questions pick one:

1. **Does the author write prose here, or does the plate compute with the value?** A name, URL, path, or reference key is data. A bio, an abstract, or a cover letter is content.
2. Then, for data: **is the set of allowed values closed?** For content: **should `*text*` render as emphasis, or stay literal?**

| | data — the plate computes with it | content — the author writes prose |
|---|---|---|
| **open / literal** | `string` | `plaintext`: `*text*` stays literal |
| **closed / formatted** | `enum`: a `values:` domain | `richtext`: `*text*` becomes emphasis |

The letter above needs no content field: its prose is the document body, which is already rich text. `plaintext` and `richtext` are for prose in a *named* field — an abstract, a summary. Such a field carries navigation, regions, and click-to-edit in editor consumers; `string` and `enum` carry none of that.

Pick before a corpus exists. Changing a declared type reinterprets every value already stored in that field, and data → content is lossy: see [Choosing among `string`, `enum`, `plaintext`, and `richtext`](quill-yaml-reference.md#choosing-among-string-enum-plaintext-and-richtext).

When you declare an `enum`, list only the real choices. Every enum also accepts a **blank** (`""`) that the engine supplies — what a document says when nobody has answered — so you never declare it, and declaring `""` in `values:` is a load error. Keep `default: ""` to mark the field optional. If the empty state is itself a decision worth recording, make it a member (`undecided`, `waived`), because the blank means nobody chose and a member means someone chose "none". See [the blank](quill-yaml-reference.md#the-blank-values-is-for-choices-not-for-the-absence-of-one).

## 3. Write `plate.typ`

Your first plate template:

```typst
#import "@local/quillmark-helper:0.1.0": data

#set page(margin: 1in)

Dear #data.recipient,

#data.at("$body", default: "")

Sincerely,

#data.sender
```

For data access patterns, helper package details, optional fields, and `$cards` iteration, see the [Typst Backend](typst-backend.md) guide.

## 4. Write a document

Create a `document.md` that matches the fields you defined:

```markdown
~~~
$quill: my_quill
$kind: main
sender: Jane Doe
recipient: John Smith
date: 2026-01-15
~~~

Thank you for your time.
```

## 5. Render it

From the same directory, render the document:

```bash
quillmark render ./my-quill document.md
```

For command options and output controls, see the [CLI Reference](../cli/reference.md).

## 6. Next steps

- [Quill.yaml Reference](quill-yaml-reference.md): full field types, UI hints, `card_kinds`, `typst` section
- [Typst Backend](typst-backend.md): data access patterns, `$cards` iteration, helper package
- [Quill Versioning](versioning.md)

**Tip:** To exclude files (fonts, build artifacts) from the bundle when loading from disk, add a `.quillignore` file at the bundle root using gitignore syntax.
