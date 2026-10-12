# Why Quillmark

Quillmark is for documents whose format someone else sets: a memo standard, a government form, a company letter, a report template. Many people, apps, or LLMs produce the same kind of document, and every one must come out in that format.

A quill holds the format in one place. Its schema validates, coerces, and fills defaults for each document's fields, and its plate (or, for a PDF form, its field spec) lays them out. A document is Markdown: card-yaml blocks for the fields, prose for the body. The same document renders the same way from Rust, Python, JavaScript, or the CLI.

## Compared with other approaches

| If you use | It does | Quillmark differs in |
|---|---|---|
| HTML and CSS to PDF (Puppeteer, WeasyPrint) | Lays out pages with a browser engine and print CSS | Pages are typeset by Typst, in process, with no browser and no network access. The quill author writes the layout in Typst rather than CSS. |
| Office templates (docxtemplater, Carbone) | Fills placeholders in a DOCX or ODT template with JSON | The format lives in a schema and a plate rather than an editable Office file, and every field is validated before render. Output is PDF, SVG, or PNG; there is no DOCX output. |
| Typst alone | Typesets a document; reads outside data through `sys.inputs` or `json()` | Quillmark is the data layer in front of Typst: the schema, coercion and defaults, rich-text Markdown fields, diagnostics with stable codes, and bindings for Python and JavaScript. Plates are ordinary Typst. |
| LaTeX document classes | Typesets a document from LaTeX source | Authors write Markdown and YAML; diagnostics name the field that failed. |
| Pandoc, Quarto | Convert Markdown into papers, books, slides, and websites | Quillmark targets one format per quill and validates every document against it. See the limits below for papers and reports. |
| PDF form libraries (pdf-lib, PyMuPDF) | Fill a PDF form's fields from code | The `acroform` backend fills a form from a validated Quillmark document, with the same model and diagnostics as a typeset quill. |

## Who writes the document

- **People** write Markdown, in a text editor or in an app built on the WebAssembly package, which paints a live preview to a canvas.
- **Apps** build a `Document` in code, from a database row or a form. See [Programmatic Construction](../integration/programmatic.md).
- **LLMs** fill in the quill's [blueprint](../quills/blueprint.md): an annotated Markdown document that parses and renders as emitted. Validation returns diagnostics with stable codes an agent can act on.

## When not to use Quillmark

- **Papers, books, and long reports.** Math, footnotes, and definition lists are not supported, and no backend typesets an image in the body ([Markdown Syntax](../authoring/markdown-syntax.md#out-of-scope)). Quarto or Pandoc fit that work.
- **Word documents as the deliverable.** Quillmark renders PDF, SVG, and PNG only.
- **One-off documents.** A quill pays for itself when many documents share a format. For a single document, write it in Typst directly.
- **Untrusted plates on a shared server without your own timeout.** Quillmark bounds what it parses but not how long a render takes ([Operations](../integration/operations.md)).
- **A form quill from an arbitrary PDF in one step.** An `acroform` quill's `form.pdf` and `form.json` are prepared by hand ([PDF Form Backend](../quills/acroform-backend.md)).
