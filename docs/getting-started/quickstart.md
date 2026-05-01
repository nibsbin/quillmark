# Quickstart

Get started with Quillmark in Python or JavaScript.

=== "Python"

    ## Installation

    ```bash
    uv pip install quillmark
    ```

    ## Basic Usage

    ```python
    from quillmark import Document, Quillmark, OutputFormat

    engine = Quillmark()
    quill = engine.quill_from_path("path/to/quill")

    markdown = """---
    QUILL: my_quill
    title: Example Document
    ---

    # Hello World
    """

    doc = Document.from_markdown(markdown)
    result = quill.render(doc, OutputFormat.PDF)

    with open("output.pdf", "wb") as f:
        f.write(result.artifacts[0].bytes)
    ```

=== "JavaScript"

    ## Installation

    ```bash
    npm install @quillmark/wasm
    ```

    ## Basic Usage

    ```javascript
    import { Document, Quillmark } from "@quillmark/wasm";

    const engine = new Quillmark();
    const enc = new TextEncoder();

    const quill = engine.quill(new Map([
      ["Quill.yaml", enc.encode("quill:\n  name: my_quill\n  backend: typst\n  version: \"1.0.0\"\n  description: Demo\n  plate_file: plate.typ\n")],
      ["plate.typ", enc.encode("#import \"@local/quillmark-helper:0.1.0\": data\n#data.BODY\n")],
    ]));

    const markdown = `---
    QUILL: my_quill
    title: Example Document
    ---

    # Hello World`;

    const doc = Document.fromMarkdown(markdown);
    const result = quill.render(doc, { format: "pdf" });
    const pdfBytes = result.artifacts[0].bytes;
    ```

    ## Live Preview (Canvas)

    For editor-style previews, paint pages directly into a `<canvas>` instead
    of round-tripping through PNG/SVG. This skips PNG encode/decode and SVG
    parse, and lets you bound memory by only painting visible pages.

    `paint` is Typst-only and WASM-only. It is unaffected by the byte-output
    `render` path — the same `RenderSession` serves both.

    ```javascript
    import { Document, Quillmark } from "@quillmark/wasm";

    const engine = new Quillmark();
    const quill = engine.quill(tree);                  // see Basic Usage

    const doc = Document.fromMarkdown(markdown);
    const session = quill.open(doc);                   // compile once

    // Surface any session-level diagnostics (e.g. version-compat shims).
    for (const w of session.warnings) console.warn(w.message);

    function renderPage(canvas, page, userZoom = 1) {
      const dpr = window.devicePixelRatio || 1;
      const scale = dpr * userZoom;                    // multiplier on 72 ppi

      const { widthPt, heightPt } = session.pageSize(page);

      // Backing store (device pixels). Setting width/height clears the canvas.
      canvas.width  = Math.round(widthPt  * scale);
      canvas.height = Math.round(heightPt * scale);

      // CSS box (layout pixels) — independent of DPR.
      canvas.style.width  = `${widthPt  * userZoom}px`;
      canvas.style.height = `${heightPt * userZoom}px`;

      session.paint(canvas.getContext("2d"), page, scale);
    }

    for (let p = 0; p < session.pageCount; p++) {
      renderPage(canvases[p], p);
    }

    // When the document changes, free the old session before opening a new one.
    session.free();
    ```

    ### Notes

    - **`scale` is a multiplier on 72 ppi**, not a ppi value. `scale = 1`
      gives 1 device pixel per Typst point. Always include
      `devicePixelRatio` so retina displays are crisp.
    - **`pageCount` and `pageSize(page)` are stable** for the lifetime of a
      session — the underlying compiled document is an immutable snapshot.
      Cache them.
    - **Canvas reuse.** Setting `canvas.width` / `canvas.height` clears the
      backing store. If you reuse a canvas without resizing (same page,
      same scale, repaint), call
      `ctx.clearRect(0, 0, canvas.width, canvas.height)` before `paint` to
      avoid stale pixels in transparent regions.
    - **Worker rendering.** The painter currently accepts only
      `CanvasRenderingContext2D` and runs on the main thread. For
      multi-page documents this can jank typing in an editor; route the
      paint loop through `requestIdleCallback` or coalesce to the visible
      viewport.
    - **No text selection / find-in-page.** Canvas pixels are opaque to the
      DOM. If you need accessibility or text selection in the preview,
      keep an SVG/PDF export path alongside.
