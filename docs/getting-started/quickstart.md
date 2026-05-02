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
      const densityScale = (window.devicePixelRatio || 1) * userZoom;

      // Painter sizes canvas.width/height itself; consumer reads back the
      // layout dimensions to drive layout.
      const result = session.paint(canvas.getContext("2d"), page, {
        layoutScale: 1,
        densityScale,
      });

      canvas.style.width  = `${result.layoutWidth}px`;
      canvas.style.height = `${result.layoutHeight}px`;
    }

    for (let p = 0; p < session.pageCount; p++) {
      renderPage(canvases[p], p);
    }

    // When the document changes, free the old session before opening a new one.
    session.free();
    ```

    ### Notes

    - **`layoutScale` vs `densityScale`.** `layoutScale` is layout-space
      pixels per Typst point — a layout decision (how big does the page
      look on screen). `densityScale` is the backing-store density
      multiplier — a sharpness decision. Fold `window.devicePixelRatio`,
      any in-app zoom level, and `visualViewport.scale` (pinch-zoom) into
      one `densityScale` value. Both default to `1`.
    - **Painter owns backing store.** Don't write to `canvas.width` /
      `canvas.height` yourself — the painter does it on every call. Don't
      call `clearRect` either; setting the backing-store size clears it.
    - **Consumer owns layout.** The painter doesn't touch
      `canvas.style.*`. Use `result.layoutWidth` / `result.layoutHeight`
      to size the canvas's display box.
    - **Backing-store clamp.** If `layoutScale * densityScale` would push
      either dimension past 16384 px, the painter clamps `densityScale`
      to fit and the result reflects what it actually wrote. Detect via
      `result.pixelWidth < Math.round(result.layoutWidth * densityScale)`.
    - **`pageCount` and `pageSize(page)` are stable** for the lifetime of a
      session — the underlying compiled document is an immutable snapshot.
      Cache them.
    - **Worker rendering.** Pass an `OffscreenCanvasRenderingContext2D`
      to the same `paint` call to rasterize off the main thread. Loading
      the WASM module inside the Worker is the host's responsibility.
    - **No text selection / find-in-page.** Canvas pixels are opaque to the
      DOM. If you need accessibility or text selection in the preview,
      keep an SVG/PDF export path alongside.
