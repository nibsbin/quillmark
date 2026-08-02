//! Markdown export: content → markdown, per island type.
//!
//! The projection back to markdown. Marks become syntax; islands are emitted
//! per their type ([`Loss`](crate::model::Loss) describes fidelity, it does not
//! gate the emit; see [`to_markdown`]); identity ([`MarkKind::Anchor`]) marks
//! are **omitted**; they survive across edits via diff-rebase, not the
//! projection (§ Codecs). Anchors carry no markdown encoding, so
//! dropping them here is by design, not loss.
//!
//! The block vocabulary is open: a [`LineKind::Unknown`] projects
//! as a paragraph and a [`Container::Unknown`] transparently, the block-axis
//! twin of an unknown mark contributing no delimiters.
//!
//! The contract this crate pins is the **content fixed point**: for a content `rt`
//! obtained from [`crate::import::from_markdown`],
//! `from_markdown(to_markdown(rt)) == rt` modulo island loss class, markdown
//! source is not canonical, but the content is, so round-trip is defined at the
//! content, not the string.
//!
//! ## Export is defined by import
//!
//! The `render_marked_core` safety net settles a line's markdown by re-parsing it
//! with [`crate::import::from_markdown`] and dropping marks until the text comes
//! back intact. Export's correctness is therefore *defined* by running import,
//! by design. CommonMark's emphasis algorithm (delimiter-run matching, the rule
//! of 3, `\*`-escape adjacency) has corners no local rule captures, and a local
//! rule that approximates it drops good marks on the cases it gets wrong;
//! verifying against the parser is the only check exactly as strict as the
//! format.
//!
//! The coupling costs two things. The codecs cannot be tested or changed
//! independently: an importer change moves exporter output. And every
//! [`to_markdown`] call transitively depends on `pulldown_cmark`, which is why
//! this crate is the workspace's only home for a CommonMark parser.
//!
//! ## Documented codec limits (degenerate, non-authorable content values)
//!
//! The fixed point holds for the content a well-behaved producer emits. Two
//! degenerate shapes markdown cannot represent do **not** round-trip, and are
//! recorded here rather than hidden (see `tests::known_hard_break_limits`):
//!
//! - **A mark spanning a hard break**: per-line rendering splits it into two
//!   per-line marks (they do not re-union across the `\n`).
//! - **An empty first line in a hard-break block**: markdown has no
//!   blank-then-forced-break syntax, so the leading empty line is dropped.
//!
//! Both arise only from adversarial delimiter/break placement, never from clean
//! markdown or a form editor. Neither is hardened: no live editor yet defines
//! what it can produce.

use crate::island::KnownIslandType;
use crate::model::{Container, Island, LineKind, MarkKind, Content, ISLAND_SLOT};

/// Render a content to markdown. An island projects by **type**: a type this
/// build knows emits its markdown, any other a placeholder comment.
///
/// [`Loss`](crate::model::Loss) does not gate the emit and is not read here. It
/// *describes* the projection's fidelity for a consumer to surface; the type
/// decides whether a projection exists at all.
///
/// Keeping the two apart is what makes a decode-time degrade safe. A known type
/// a future writer stamped with a loss class this build lacks keeps that class
/// verbatim (reading as `Fidelity::Unrepresentable` through `Loss::fidelity`)
/// and still emits its table: the conservative reading describes the value, it
/// does not suppress it.
pub fn to_markdown(rt: &Content) -> String {
    // Per-line char ranges, so global marks can be clipped to a line.
    let segments = line_segments(rt);
    let ctx = Ctx {
        rt,
        segments: &segments,
    };
    let mut out = String::new();
    emit_block(&ctx, 0..rt.lines.len(), 0, &mut out);
    // Collapse any trailing blank lines. `to_markdown` projects a *value*, not a
    // file: it emits no final newline, so `writer.set("subject", "Hello")` reads
    // back as `"Hello"`, not `"Hello\n"` (the read-back-grows-a-newline
    // footgun). Document-file writers own the file-final newline
    // (`Document::to_markdown`); the content fixed point is defined at the content,
    // and import is newline-insensitive, so dropping it is round-trip-invisible.
    while out.ends_with('\n') {
        out.pop();
    }
    out
}

/// Render a content to plaintext: [`Content::text`] with island slots
/// ([`ISLAND_SLOT`]) removed. The lossy sibling of [`to_markdown`]: it drops
/// every mark and island (tables, images have no plaintext projection), keeping
/// only literal text. Callers that want a non-empty result should check for the
/// empty string themselves.
///
/// Tables (and images) having no plaintext form is a **decided limitation**, not
/// an oversight: the pdfform backend fills a form field from this
/// projection, so a field bound to a table-bearing content renders the surrounding
/// text and silently omits the table. A degraded row/tab dump was rejected (it
/// would read as a faithful table and mislead) so the projection drops the
/// island outright. Revisit only if a form field ever needs tabular fill.
pub fn to_plaintext(rt: &Content) -> String {
    rt.text.chars().filter(|&c| c != ISLAND_SLOT).collect()
}

struct Ctx<'a> {
    rt: &'a Content,
    segments: &'a [Segment],
}

/// One line's char range `[start, end)` into the content, with the matching byte
/// range and the count of island slots before the line, so a caller indexes
/// the content text and the island list in O(1) without rescanning.
///
/// **Deliberately not `#[non_exhaustive]`**, unlike the model types it indexes.
/// It is a derived view, not a stored one: [`line_segments`] recomputes the
/// whole of it from a [`Content`], and the five fields are one line's position
/// in the two coordinate spaces the model has. A sixth would mean a third
/// space, which moves the model before it moves this, so the literal is worth
/// keeping and a new field is a semver-major.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    /// USV index of the line's first char.
    pub start: usize,
    /// USV index one past the line's last char (its `\n`, or the content end).
    pub end: usize,
    /// Byte offset of `start` in the content text.
    pub byte_start: usize,
    /// Byte offset of `end` in the content text.
    pub byte_end: usize,
    /// Count of [`ISLAND_SLOT`] chars before `start`.
    pub slots_before: usize,
}

/// Per-line char/byte ranges and slot prefixes over a content, in line order.
pub fn line_segments(rt: &Content) -> Vec<Segment> {
    let mut segs = Vec::with_capacity(rt.lines.len());
    let mut start = 0usize;
    let mut byte_start = 0usize;
    let mut slots_before = 0usize;
    let mut line_slots = 0usize;
    let mut pos = 0usize;
    for (b, c) in rt.text.char_indices() {
        if c == '\n' {
            segs.push(Segment {
                start,
                end: pos,
                byte_start,
                byte_end: b,
                slots_before,
            });
            start = pos + 1;
            byte_start = b + 1; // `\n` is one byte
            slots_before += line_slots;
            line_slots = 0;
        } else if c == ISLAND_SLOT {
            line_slots += 1;
        }
        pos += 1;
    }
    segs.push(Segment {
        start,
        end: pos,
        byte_start,
        byte_end: rt.text.len(),
        slots_before,
    });
    // Defensive: a malformed content (lines.len() != segments) still gets one
    // segment per line so indexing never panics.
    let total_slots = slots_before + line_slots;
    while segs.len() < rt.lines.len() {
        segs.push(Segment {
            start: pos,
            end: pos,
            byte_start: rt.text.len(),
            byte_end: rt.text.len(),
            slots_before: total_slots,
        });
    }
    segs
}

/// Emit the lines in `range`, all sharing the container prefix of length
/// `depth`. Leaf lines (containers.len() == depth) render at this level;
/// deeper lines are grouped by their `depth`-th container and recursed into.
fn emit_block(ctx: &Ctx, range: std::ops::Range<usize>, depth: usize, out: &mut String) {
    let lines = &ctx.rt.lines;
    let mut i = range.start;
    let mut first_block = true;
    while i < range.end {
        let line = &lines[i];
        if line.containers.len() > depth {
            // A nested container starts here; gather its run and recurse.
            let key = &line.containers[depth];
            let mut j = i + 1;
            while j < range.end
                && lines[j].containers.len() > depth
                && &lines[j].containers[depth] == key
            {
                j += 1;
            }
            block_separator(out, first_block);
            emit_container(ctx, key, i..j, depth, out);
            first_block = false;
            i = j;
        } else {
            // A leaf block: this line (continues == false) plus every following
            // line that continues it (a hard-break run, or a code fence's lines).
            let mut j = i + 1;
            while j < range.end && lines[j].containers.len() == depth && lines[j].continues {
                j += 1;
            }
            block_separator(out, first_block);
            emit_leaf_block(ctx, i..j, out);
            first_block = false;
            i = j;
        }
    }
}

fn block_separator(out: &mut String, first_block: bool) {
    if !first_block {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
    }
}

/// Emit a container run (a list item's block, or a quote's block) by prefixing
/// each produced line. The inner blocks are emitted into a scratch buffer, then
/// each of its lines is prefixed.
fn emit_container(
    ctx: &Ctx,
    key: &Container,
    range: std::ops::Range<usize>,
    depth: usize,
    out: &mut String,
) {
    let mut inner = String::new();
    emit_block(ctx, range, depth + 1, &mut inner);

    match key {
        Container::ListItem {
            ordered,
            start,
            ordinal,
        } => {
            let marker = if *ordered {
                // `start`/`ordinal` are unbounded `u64` and `validate` does not
                // ceiling them, so a corrupt/adversarial content can drive the
                // sum past `u64::MAX`; saturate rather than panic (or wrap under
                // release overflow-checks) on the render path.
                format!("{}. ", start.saturating_add(*ordinal))
            } else {
                "- ".to_string()
            };
            let indent = " ".repeat(marker.len());
            prefix_lines(&inner, &marker, &indent, out);
        }
        Container::Quote => {
            // `> ` on content lines, `>` on blank lines so paragraphs stay in
            // one quote on re-import.
            prefix_quote(&inner, out);
        }
        // Open set: a container this build does not know has no markdown syntax
        // to prefix with, so it projects transparently, its blocks land at the
        // enclosing level. The container survives via storage, not the
        // projection, exactly as an unknown island's props do.
        Container::Unknown { .. } => out.push_str(&inner),
    }
}

/// Prefix the first produced line with `first`, the rest with `cont`.
fn prefix_lines(inner: &str, first: &str, cont: &str, out: &mut String) {
    for (idx, line) in inner.split('\n').enumerate() {
        if idx == 0 {
            out.push_str(first);
            out.push_str(line);
        } else {
            out.push('\n');
            if line.is_empty() {
                // blank continuation line: no trailing indent
            } else {
                out.push_str(cont);
                out.push_str(line);
            }
        }
    }
}

fn prefix_quote(inner: &str, out: &mut String) {
    for (idx, line) in inner.split('\n').enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        if line.is_empty() {
            out.push('>');
        } else {
            out.push_str("> ");
            out.push_str(line);
        }
    }
}

fn emit_code(ctx: &Ctx, range: std::ops::Range<usize>, lang: Option<&str>, out: &mut String) {
    // Choose a fence long enough to not collide with backtick runs in content.
    let mut max_ticks = 0usize;
    for i in range.clone() {
        max_ticks = max_ticks.max(longest_backtick_run(seg_str(ctx, i)));
    }
    let fence = "`".repeat(max_ticks.max(2) + 1);
    out.push_str(&fence);
    if let Some(l) = lang {
        out.push_str(l);
    }
    out.push('\n');
    for i in range {
        out.push_str(seg_str(ctx, i));
        out.push('\n');
    }
    out.push_str(&fence);
}

/// Emit one leaf block: the lines `range.start` (a block start) plus any
/// continuation lines. A paragraph block joins its lines with a markdown hard
/// break (`\` + newline); a code block renders one fence; a heading/island is a
/// single line.
fn emit_leaf_block(ctx: &Ctx, range: std::ops::Range<usize>, out: &mut String) {
    let first = &ctx.rt.lines[range.start];
    match &first.kind {
        LineKind::Code { lang } => emit_code(ctx, range, lang.as_deref(), out),
        LineKind::Island => {
            // A block island: the segment is a single slot. Resolve and emit.
            if let Some(isl) = slot_island(ctx, range.start) {
                emit_island(isl, out);
            }
        }
        LineKind::Heading { level } => {
            for _ in 0..*level {
                out.push('#');
            }
            out.push(' ');
            // Headings never carry continuations (import maps a hard break in a
            // heading to a space), so only the first line contributes.
            let mut inline = render_inline(ctx, range.start, false);
            // A trailing `#` run reads as an ATX closing sequence on re-import
            // (`# a #` → heading text "a", dropping the `#`). Escape the last `#`
            // so the run does not reach end-of-line as a bare hash sequence:
            // one escaped hash defeats the whole closer, and `\#` re-imports as a
            // literal `#`.
            if inline.ends_with('#') {
                inline.pop();
                inline.push_str("\\#");
            }
            out.push_str(&inline);
        }
        // An unknown block role projects as a paragraph: the role is lost to
        // markdown (it round-trips through storage), the text is not.
        LineKind::Para | LineKind::Unknown { .. } => {
            // Join continuation lines with a backslash hard break.
            let parts: Vec<String> = range.map(|i| render_inline(ctx, i, true)).collect();
            out.push_str(&parts.join("\\\n"));
        }
        LineKind::Rule => out.push_str("---"),
    }
}

fn seg_str<'a>(ctx: &'a Ctx, i: usize) -> &'a str {
    let seg = &ctx.segments[i];
    &ctx.rt.text[seg.byte_start..seg.byte_end]
}

/// The island backing the single slot on a block-island line `i`.
fn slot_island<'a>(ctx: &'a Ctx, i: usize) -> Option<&'a Island> {
    ctx.rt.islands.get(ctx.segments[i].slots_before)
}

fn emit_island(isl: &Island, out: &mut String) {
    match KnownIslandType::parse(&isl.island_type) {
        Some(KnownIslandType::Table) => emit_table(isl, out),
        Some(KnownIslandType::Image) => emit_image(isl, out),
        None => {
            // Unknown island (the open set): a comment placeholder that survives
            // round-trip as no content text (HTML comments are stripped on
            // re-import). The island is preserved via storage, not the projection.
            // A known type can't reach here: the match is exhaustive, so a new
            // type is a compile error, not a silent placeholder.
            out.push_str(&format!("<!-- island:{} -->", isl.island_type));
        }
    }
}

fn emit_table(isl: &Island, out: &mut String) {
    let header = isl.props.get("header").and_then(|v| v.as_array());
    let rows = isl.props.get("rows").and_then(|v| v.as_array());
    let aligns = isl.props.get("aligns").and_then(|v| v.as_array());
    let cols = header.map(|h| h.len()).unwrap_or(0);
    if cols == 0 {
        return;
    }
    // Cells are canonical `{text, marks}`; reconstruct each cell's markdown from
    // its structure (the prose mark→syntax rendering, plus `|`→`\|`), so nothing
    // re-parses markdown and `import(export(table))` is a fixed point.
    // header row
    out.push_str("| ");
    if let Some(h) = header {
        out.push_str(&h.iter().map(render_cell_md).collect::<Vec<_>>().join(" | "));
    }
    out.push_str(" |\n|");
    for k in 0..cols {
        let a = aligns
            .and_then(|a| a.get(k))
            .and_then(|v| v.as_str())
            .unwrap_or("none");
        out.push_str(match a {
            "left" => " :--- |",
            "center" => " :---: |",
            "right" => " ---: |",
            _ => " --- |",
        });
    }
    if let Some(rs) = rows {
        for row in rs {
            if let Some(r) = row.as_array() {
                // Pad/truncate to the header's column count so a ragged island
                // (one that skipped normalization) still emits a rectangular
                // table: the same count the Typst projection uses post-normalize.
                let mut cells: Vec<String> = r.iter().map(render_cell_md).collect();
                cells.resize(cols, String::new());
                out.push_str("\n| ");
                out.push_str(&cells.join(" | "));
                out.push_str(" |");
            }
        }
    }
}

fn emit_image(isl: &Island, out: &mut String) {
    let url = isl.props.get("url").and_then(|v| v.as_str()).unwrap_or("");
    let alt = isl.props.get("alt").and_then(|v| v.as_str()).unwrap_or("");
    // Alt is inline content of `![…]`: escape it like a link's display text so a
    // `]`/`\`/`&`/delimiter can't terminate the markup or decode on re-import.
    // Url goes through `emit_url` (bare when safe, else angle-wrapped + escaped).
    out.push_str("![");
    out.push_str(&escape_run(&alt.chars().collect::<Vec<_>>(), false));
    out.push_str("](");
    emit_url(url, out);
    out.push(')');
}

/// Emit a link/image destination that re-imports to `url` verbatim. A bare
/// (unbracketed) destination round-trips only for a URL with no whitespace,
/// control char, `<`, `>`, `\`, or `&`, and balanced parentheses: CommonMark's
/// unbracketed form. Anything else is angle-wrapped, with `<`, `>`, `\`, and `&`
/// backslash-escaped so the sequence re-parses to the exact URL: unescaped `<`/`>`
/// are illegal even inside the wrap, and `\`/`&` would otherwise be consumed as an
/// escape or entity reference on re-import (the same `&`-always-escaped rule
/// [`escape_char_into`] applies to prose). Spaces and parentheses need no escape
/// inside the wrap, so a spaced or unbalanced-paren URL wraps without further work.
fn emit_url(url: &str, out: &mut String) {
    if url_is_bare_safe(url) {
        out.push_str(url);
        return;
    }
    out.push('<');
    for c in url.chars() {
        if matches!(c, '<' | '>' | '\\' | '&') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('>');
}

/// Whether `url` re-imports verbatim as a bare (unbracketed) link destination:
/// no whitespace/control/`<`/`>`/`\`/`&`, and balanced parentheses. A `false`
/// routes the URL through [`emit_url`]'s angle-wrapped, escaped form.
fn url_is_bare_safe(url: &str) -> bool {
    let mut depth: i32 = 0;
    for c in url.chars() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            '<' | '>' | '\\' | '&' => return false,
            c if c.is_whitespace() || c.is_control() => return false,
            _ => {}
        }
    }
    depth == 0
}

// ---------------------------------------------------------------------------
// Inline rendering: marks -> syntax, over one line's char range.
// ---------------------------------------------------------------------------

fn render_inline(ctx: &Ctx, i: usize, escape_leading_block: bool) -> String {
    let seg = &ctx.segments[i];
    let line_start = seg.start;
    let text = seg_str(ctx, i);
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();

    // Clip marks to this line, split into code (atomic) and formatting (nested).
    let mut code_ranges: Vec<(usize, usize)> = Vec::new();
    let mut fmt: Vec<(usize, usize, &MarkKind)> = Vec::new();
    let mut links: Vec<(usize, usize, &str)> = Vec::new();
    for m in &ctx.rt.marks {
        let s = m.start.saturating_sub(line_start);
        let e = m.end.saturating_sub(line_start);
        if m.end <= line_start || m.start >= line_start + n {
            continue; // outside this line
        }
        let s = s.min(n);
        let e = e.min(n);
        match &m.kind {
            MarkKind::Anchor { .. } => {} // omitted from the projection
            MarkKind::Code => code_ranges.push((s, e)),
            MarkKind::Link { url } => links.push((s, e, url)),
            _ => fmt.push((s, e, &m.kind)),
        }
    }

    // Leading ordered-list marker: a line whose text starts `<digits>.` or
    // `<digits>)` would re-import as an ordered list, so escape that punctuation.
    let escape_punct_at = if escape_leading_block {
        let lead_digits = chars.iter().take_while(|c| c.is_ascii_digit()).count();
        if lead_digits > 0 && lead_digits < n && matches!(chars[lead_digits], '.' | ')') {
            Some(lead_digits)
        } else {
            None
        }
    } else {
        None
    };

    let slots_before_line = seg.slots_before;
    render_marked_core(
        &chars,
        &code_ranges,
        &fmt,
        &links,
        escape_punct_at,
        escape_leading_block,
        false, // prose text does not escape `|`
        |pos_local| {
            // Segment prefix + slots earlier on this line: the island index for
            // the slot at `pos_local`, without rescanning the whole content.
            let before = slots_before_line
                + chars[..pos_local]
                    .iter()
                    .filter(|&&c| c == ISLAND_SLOT)
                    .count();
            ctx.rt.islands.get(before).map(|isl| {
                let mut tmp = String::new();
                emit_island(isl, &mut tmp);
                tmp
            })
        },
    )
}

/// Render marks over a standalone char slice to markdown: the projection's mark
/// boundary sweep, shared by prose lines and table cells. `code_ranges`/`fmt`/
/// `links` are the marks clipped to `chars` (local offsets); `escape_pipe` adds
/// `|`→`\|` for cells; `island_markup_at` renders an island slot (prose) or
/// yields `None` (cells carry no slot).
///
/// The model permits free (Peritext-style) overlap (an editor's `apply_mark_ops`
/// can produce `strong[0,4)` + `strike[2,6)`) but markdown syntax nests. The
/// sweep closes every mark ending at a boundary and reopens the deeper survivors,
/// so a partial overlap lowers to balanced markdown (`**ab~~cd~~**~~ef~~`), which
/// re-imports to the same content for marks with *distinct* delimiters. Two
/// preconditions the sweep can't express in reopened markdown are clipped away
/// first:
///
/// - **Atomic spans** (`code`/`link`) can't carry a partial wrap, and the sweep's
///   cursor jumps their interior: a wrap edge hiding inside would be missed and
///   left unbalanced. [`clip_fmt_to_atomic`] pulls such edges to the span's
///   boundary (the atomic-balance shape, in markdown).
/// - **`*`/`**` (emph/strong) share a delimiter character**, so a reopened
///   `*` abutting a `**` merges into an ambiguous `***` run that CommonMark
///   re-segments wrong, this overlap is *unrepresentable*, so
///   [`clip_asterisk_overlap`] nests the two by truncation (a documented codec
///   limit: `strong`+`emph` overlap keeps the text but loses the crossing tail).
#[allow(clippy::too_many_arguments)]
fn render_marked_core(
    chars: &[char],
    code_ranges: &[(usize, usize)],
    fmt: &[(usize, usize, &MarkKind)],
    links: &[(usize, usize, &str)],
    escape_punct_at: Option<usize>,
    escape_leading_block: bool,
    escape_pipe: bool,
    island_markup_at: impl Fn(usize) -> Option<String>,
) -> String {
    let n = chars.len();

    // A code span cannot carry an island: the span is emitted verbatim between
    // backticks, so a slot inside it lands raw in the output and re-imports as
    // nothing, the island, its slot, and the code mark all vanish. Markdown has
    // no honest encoding (`` `![x](y)` `` is the *literal* text), so split each
    // code range around every slot it covers and let the island render as its own
    // markup between the surviving code spans: text and island preserved, one
    // mark lowered to several, the same trade the sweep makes for free overlap.
    let mut code_ranges: Vec<(usize, usize)> = code_ranges
        .iter()
        .flat_map(|&(s, e)| split_around_slots(chars, s, e))
        .collect();
    // The sweep reaches the atomic spans with a cursor, so both lists are put in
    // start order here (a local guarantee rather than an obligation on every
    // caller) instead of being re-scanned at every position.
    code_ranges.sort_unstable();
    let code_ranges = &code_ranges[..];
    let mut links: Vec<(usize, usize, &str)> = links.to_vec();
    links.sort_by_key(|&(s, _, _)| s);
    let links = &links[..];

    // Clip the wrapping marks so the sweep only ever sees a representable shape.
    let mut fmt: Vec<(usize, usize, &MarkKind)> = fmt.to_vec();
    let mut atomics: Vec<(usize, usize)> = code_ranges.to_vec();
    atomics.extend(links.iter().map(|(s, e, _)| (*s, *e)));
    clip_fmt_to_atomic(&mut fmt, &atomics);
    clip_asterisk_overlap(&mut fmt);

    // `fmt` by start, longest span (outer) first at a tie. `pos` only ever
    // advances, so one cursor over this order opens every mark exactly once: the
    // sweep is linear in `chars` + `fmt` rather than rescanning all three mark
    // lists at every position. Built once here, not per sweep: the
    // net below sweeps the same `fmt` up to `PROBE_BUDGET` times.
    let mut by_start: Vec<usize> = (0..fmt.len()).collect();
    by_start.sort_by(|&a, &b| fmt[a].0.cmp(&fmt[b].0).then(fmt[b].1.cmp(&fmt[a].1)));

    // One mark sweep over the marks `keep` selects (indices into `fmt`) → inline
    // markdown. Free (Peritext) overlap is lowered to nesting by closing every
    // mark ending at `pos` and reopening the deeper survivors.
    let sweep = |keep: &[bool]| -> String {
        let mut out = String::new();
        // Indices into `fmt` for the marks currently open, outermost first.
        // Storing the index (not `(end, kind)`) keeps each open mark's identity,
        // so a reopened mark re-emits its OWN delimiter.
        let mut stack: Vec<usize> = Vec::new();
        let (mut oi, mut li, mut ci) = (0usize, 0usize, 0usize);
        let mut pos = 0usize;
        while pos <= n {
            if let Some(idx) = stack.iter().position(|&fi| fmt[fi].1 == pos) {
                let mut reopen: Vec<usize> = Vec::new();
                while stack.len() > idx {
                    let fi = stack.pop().unwrap();
                    out.push_str(&delim_close(fmt[fi].2));
                    if fmt[fi].1 != pos {
                        reopen.push(fi);
                    }
                }
                for fi in reopen.into_iter().rev() {
                    out.push_str(&delim_open(fmt[fi].2));
                    stack.push(fi);
                }
            }
            // Open formatting marks starting here, longest span (outer) first:
            // BEFORE any atomic run, so a formatting mark that begins at the same
            // position as inline code/link still wraps it (`**` + code →
            // `**`code`…**`, not a dropped strong). A mark whose start the cursor
            // has already passed was jumped over by an atomic span's interior and
            // never opens (clipping keeps that set empty).
            while oi < by_start.len() && fmt[by_start[oi]].0 < pos {
                oi += 1;
            }
            while oi < by_start.len() && fmt[by_start[oi]].0 == pos {
                let fi = by_start[oi];
                oi += 1;
                if !keep[fi] {
                    continue;
                }
                out.push_str(&delim_open(fmt[fi].2));
                stack.push(fi);
            }
            // A link is emitted atomically as [text](url); its display text
            // carries plain content (nested marks in link text are not supported)
            // but does carry islands (a linked image is `[![alt](src)](url)`,
            // plain CommonMark) so each char routes through `island_markup_at`
            // the way the plain-text path below does. Emitting the slice raw
            // would leak the slot char and lose the island with it.
            while li < links.len() && links[li].0 < pos {
                li += 1;
            }
            if let Some(&(ls, le, url)) = links.get(li).filter(|l| l.0 == pos) {
                out.push('[');
                for (i, &c) in chars[ls..le].iter().enumerate() {
                    if c == ISLAND_SLOT {
                        if let Some(markup) = island_markup_at(ls + i) {
                            out.push_str(&markup);
                        }
                    } else {
                        escape_char_into(c, i == 0, escape_pipe, &mut out);
                    }
                }
                out.push_str("](");
                emit_url(url, &mut out);
                out.push(')');
                pos = le;
                continue;
            }
            // A code range is atomic.
            while ci < code_ranges.len() && code_ranges[ci].0 < pos {
                ci += 1;
            }
            if let Some(&(cs, ce)) = code_ranges.get(ci).filter(|r| r.0 == pos) {
                let content: String = chars[cs..ce].iter().collect();
                let ticks = longest_backtick_run(&content) + 1;
                let fence = "`".repeat(ticks.max(1));
                out.push_str(&fence);
                out.push_str(&content);
                out.push_str(&fence);
                pos = ce;
                continue;
            }
            if pos < n {
                let c = chars[pos];
                if c == ISLAND_SLOT {
                    if let Some(markup) = island_markup_at(pos) {
                        out.push_str(&markup);
                    }
                } else if Some(pos) == escape_punct_at {
                    out.push('\\');
                    out.push(c);
                } else {
                    escape_char_into(c, pos == 0 && escape_leading_block, escape_pipe, &mut out);
                }
            }
            pos += 1;
        }
        // Drain any mark still open at end of sweep. Clipping keeps every wrap
        // `end` reachable, so this normally drains nothing; the final close guard.
        while let Some(fi) = stack.pop() {
            out.push_str(&delim_close(fmt[fi].2));
        }
        out
    };

    // Verify-and-drop safety net. The clips above and the sweep round-trip every
    // content `import` produces, but an editor's `apply_mark_ops` can build a mark
    // over a span markdown can't represent: CommonMark's full emphasis algorithm
    // (delimiter-run matching, the rule of 3, `\*`-escape adjacency) has corners
    // no local rule captures, and it lowers to a `**`/`*`/`~~` run pulldown
    // re-reads as literal text, leaking a delimiter into the content. Re-parse the
    // rendered line and, if its plain text drifted, search for a set of flanking
    // marks that keeps the text intact. Only lines carrying a flanking mark probe
    // at all; a line markdown already round-trips passes on the first.
    //
    // The probe wraps the fragment in `,…,`: this is an *inline* fragment, but
    // parsed standalone a leading `0. ` / `# ` / `> ` would read as a list/heading/
    // quote marker (a false positive that would drop a good mark). A punctuation
    // sentinel blocks every leading-block construct, preserves edge whitespace,
    // and is flanking-equivalent to the line start/end it replaces (a run's
    // open/close decision is identical whether the neighbor is line-boundary
    // whitespace or a punctuation char), so it never masks or invents a leak.
    // `expected` is the content text with islands as their slot char, which
    // `import` restores from the emitted island markup.
    let expected: String = chars.iter().collect();
    let is_flanking = |k: &MarkKind| {
        matches!(k, MarkKind::Strong | MarkKind::Emph | MarkKind::Strike)
    };
    let want = format!(",{expected},");
    let text_safe = |md: &str| {
        crate::import::from_markdown(&format!(",{md},"))
            .map(|rt| rt.text == want)
            .unwrap_or(false)
    };
    let out = sweep(&vec![true; fmt.len()]);
    if !fmt.iter().any(|m| is_flanking(m.2)) || text_safe(&out) {
        return out;
    }
    // The flanking marks in document order (`marks` is normalized, so: start
    // ascending, then span, then kind). That order is the re-add priority below
    // (when two marks can't both survive, the earlier one wins) and it is the
    // one user-visible choice in this search, so it is fixed here rather than
    // falling out of the traversal.
    let cands: Vec<usize> = (0..fmt.len()).filter(|&i| is_flanking(fmt[i].2)).collect();
    // Render with only the flanking marks in `keep`; every other mark always
    // rides.
    let render = |keep: &[usize]| -> String {
        let mut mask: Vec<bool> = fmt.iter().map(|m| !is_flanking(m.2)).collect();
        for &i in keep {
            mask[i] = true;
        }
        sweep(&mask)
    };
    // Drop the whole flanking set, then re-add by halves: a chunk that stays
    // text-safe is accepted whole, one that doesn't splits and its halves are
    // retried, a lone mark that still leaks is dropped. `m` marks cost ~2·log(m)
    // probes when one is at fault and 2 when none can survive, so a densely
    // marked paragraph costs re-parses in its mark count's logarithm rather than
    // one per dropped mark. Greedy, so the result is a maximal
    // text-safe set, not necessarily the largest one: a chunk taken early can
    // foreclose a later mark that a different partition would have kept.
    //
    // Dropping every flanking mark is the floor the search can't go below, so if
    // even that leaks, the leak is not a flanking mark's and no re-add can fix it:
    // return the floor rather than probe a search that cannot succeed. A lone
    // candidate has nowhere to split, so the floor is already its answer.
    let mut out = render(&[]);
    if cands.len() == 1 || !text_safe(&out) {
        return out;
    }
    let mut kept: Vec<usize> = Vec::new();
    // A chunk `(lo, hi)` and the `kept` length at which its trial is *already
    // known to fail*: a right half popped with every mark of its left sibling
    // accepted re-renders exactly the chunk their parent failed on, so it splits
    // without spending a probe. The whole set is the render that already failed,
    // so the search starts one level down.
    let mid = cands.len() / 2;
    let mut work: Vec<(usize, usize, usize)> = vec![(mid, cands.len(), mid), (0, mid, usize::MAX)];
    let mut budget = PROBE_BUDGET;
    while let Some((lo, hi, known_bad_at)) = work.pop() {
        let split = |work: &mut Vec<_>, kept_len: usize| {
            if hi - lo > 1 {
                let mid = lo + (hi - lo) / 2;
                work.push((mid, hi, kept_len + (mid - lo)));
                work.push((lo, mid, usize::MAX));
            }
        };
        if known_bad_at == kept.len() {
            split(&mut work, kept.len());
            continue;
        }
        if budget == 0 {
            break;
        }
        budget -= 1;
        // `work` is a left-to-right DFS, so every accepted mark precedes this
        // chunk and `trial` stays in document order.
        let trial: Vec<usize> = kept.iter().chain(&cands[lo..hi]).copied().collect();
        let md = render(&trial);
        if text_safe(&md) {
            kept = trial;
            out = md;
        } else {
            split(&mut work, kept.len());
        }
    }
    out
}

/// Probes the verify-and-drop net will spend on one line before giving up and
/// dropping the flanking marks it hasn't cleared. Resolving `m` of them exactly
/// costs at most `2m-2` probes, so at 64 no line carrying 32 or fewer ever loses
/// a mark to the budget; past that the line is one an editor filled with
/// unrepresentable marks, and dropping those is the net's own remedy anyway.
/// The ceiling is the point: export cost stays linear in document size whatever
/// marks are thrown at it.
const PROBE_BUDGET: usize = 64;

/// [`clip_range_to_atomic`] over every wrapping mark, dropping the ones an
/// atomic span swallowed whole.
fn clip_fmt_to_atomic(fmt: &mut Vec<(usize, usize, &MarkKind)>, atomics: &[(usize, usize)]) {
    for m in fmt.iter_mut() {
        clip_range_to_atomic(&mut m.0, &mut m.1, atomics);
    }
    fmt.retain(|m| m.0 < m.1);
}

/// The atomic-balance rule for a single `[*start, *end)` range against a set of
/// atomic spans. A range edge landing strictly inside an atomic `[cs, ce)` span
/// is pulled to that span's boundary (`start`→`ce`, `end`→`cs`); an edge outside
/// every span is untouched. An atomic span can't carry partial styling, and a
/// mark-sweep cursor jumps a span's interior start→end, so an edge left hiding
/// inside would be missed and render unbalanced. A range that strictly *contains*
/// a span keeps both edges, so the span still nests inside it. Applying the spans
/// in sequence is order-independent across ranges, so a caller may loop ranges
/// or spans on the outside: a range whose edges cross after clipping (swallowed
/// whole) is left empty for the caller to drop. Shared by this crate's export
/// and the Typst backend's inline emitter, the two sites that enforce it.
pub fn clip_range_to_atomic(start: &mut usize, end: &mut usize, atomics: &[(usize, usize)]) {
    for &(cs, ce) in atomics {
        if cs < *start && *start < ce {
            *start = ce;
        }
        if cs < *end && *end < ce {
            *end = cs;
        }
    }
}

/// The maximal slot-free subranges of `[start, end)`: the range itself when it
/// covers no [`ISLAND_SLOT`], else one range per run between slots (empty runs
/// dropped). Used to keep a code span off an island it cannot represent.
fn split_around_slots(chars: &[char], start: usize, end: usize) -> Vec<(usize, usize)> {
    let end = end.min(chars.len());
    if !chars[start.min(end)..end].contains(&ISLAND_SLOT) {
        return vec![(start, end)];
    }
    let mut out = Vec::new();
    let mut run = start;
    for (i, _) in chars[start..end]
        .iter()
        .enumerate()
        .map(|(i, c)| (start + i, c))
        .filter(|&(_, &c)| c == ISLAND_SLOT)
    {
        if run < i {
            out.push((run, i));
        }
        run = i + 1;
    }
    if run < end {
        out.push((run, end));
    }
    out
}

/// Nest `strong`/`emph` marks that partially overlap, by truncating the
/// later-opening one to its enclosing sibling's end. Both render as runs of the
/// same character (`**`/`*`), so a reopened `*` abutting a `**` would merge into
/// an ambiguous `***`: this overlap is unrepresentable in CommonMark. Truncation
/// keeps the text and the nested portion of both marks, dropping only the
/// crossing tail (a documented codec limit). Marks with distinct delimiters
/// (`strike`, `underline`, `link`) are left to the sweep's close-and-reopen,
/// which round-trips them exactly. No-op when the marks already nest.
fn clip_asterisk_overlap(fmt: &mut [(usize, usize, &MarkKind)]) {
    let is_ast = |k: &MarkKind| matches!(k, MarkKind::Strong | MarkKind::Emph);
    // Asterisk-family marks, outermost first (start asc, then longer span first).
    let mut idx: Vec<usize> = (0..fmt.len()).filter(|&i| is_ast(fmt[i].2)).collect();
    idx.sort_by(|&a, &b| fmt[a].0.cmp(&fmt[b].0).then(fmt[b].1.cmp(&fmt[a].1)));
    // Ends of the enclosing ancestors still open at the current mark's start.
    let mut open_ends: Vec<usize> = Vec::new();
    for &i in &idx {
        let (s, mut e, _) = fmt[i];
        while open_ends.last().is_some_and(|&end| end <= s) {
            open_ends.pop();
        }
        if let Some(&parent_end) = open_ends.last() {
            if parent_end < e {
                e = parent_end;
                fmt[i].1 = e;
            }
        }
        open_ends.push(e);
    }
}

/// Reconstruct a table cell's markdown from its `{text, marks}`: the same mark
/// sweep as prose (`render_marked_core`) with `|`→`\|` escaping so the cell
/// survives re-import through `pulldown`'s pipe splitting. A cell is flat inline
/// (no islands, no leading-block escape) so `import(export(table))` is a fixed
/// point.
fn render_cell_md(v: &serde_json::Value) -> String {
    let (text, marks) = crate::serial::parse_cell(v);
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut code_ranges: Vec<(usize, usize)> = Vec::new();
    let mut fmt: Vec<(usize, usize, &MarkKind)> = Vec::new();
    let mut links: Vec<(usize, usize, &str)> = Vec::new();
    for m in &marks {
        if m.start >= n {
            continue;
        }
        let s = m.start;
        let e = m.end.min(n);
        if s >= e {
            continue;
        }
        match &m.kind {
            MarkKind::Anchor { .. } => {}
            MarkKind::Code => code_ranges.push((s, e)),
            MarkKind::Link { url } => links.push((s, e, url)),
            _ => fmt.push((s, e, &m.kind)),
        }
    }
    render_marked_core(
        &chars,
        &code_ranges,
        &fmt,
        &links,
        None,
        false,
        true,
        |_| None,
    )
}

fn delim_open(kind: &MarkKind) -> String {
    match kind {
        MarkKind::Strong => "**".into(),
        // `*`, not `_`: `_` cannot do intraword emphasis (CommonMark flanking),
        // so `_a_你` re-imports as literal text; `*a*你` emphasizes correctly.
        MarkKind::Emph => "*".into(),
        MarkKind::Underline => "<u>".into(),
        MarkKind::Strike => "~~".into(),
        // Code/Link/Anchor handled elsewhere.
        _ => String::new(),
    }
}

fn delim_close(kind: &MarkKind) -> String {
    match kind {
        MarkKind::Strong => "**".into(),
        MarkKind::Emph => "*".into(),
        MarkKind::Underline => "</u>".into(),
        MarkKind::Strike => "~~".into(),
        _ => String::new(),
    }
}

fn escape_run(chars: &[char], escape_pipe: bool) -> String {
    let mut s = String::new();
    for (i, c) in chars.iter().enumerate() {
        escape_char_into(*c, i == 0, escape_pipe, &mut s);
    }
    s
}

/// Push `c` into `out` escaped so it re-imports as literal text: the char
/// verbatim, or a `&'static str` escape. `leading` also escapes block-starter
/// chars that would otherwise open a heading/list/quote; `escape_pipe` adds
/// `|`→`\|`, so a table cell survives `pulldown`'s pipe split.
fn escape_char_into(c: char, leading: bool, escape_pipe: bool, out: &mut String) {
    let esc: &str = match c {
        '\\' => "\\\\",
        '*' => "\\*",
        '_' => "\\_",
        '`' => "\\`",
        '[' => "\\[",
        ']' => "\\]",
        '<' => "\\<",
        '~' => "\\~",
        // `&` starts a CommonMark entity/numeric reference (`&amp;`, `&#38;`),
        // decoded on re-import: an unescaped `&` in `&word;`-shaped text would
        // silently collapse to the entity's character. Always escaped (a bare `&`
        // is harmless, but detecting "would form an entity" is not worth the
        // fragility); `\&` re-imports as a literal `&`.
        '&' => "\\&",
        '|' if escape_pipe => "\\|",
        '#' if leading => "\\#",
        '>' if leading => "\\>",
        '-' if leading => "\\-",
        '+' if leading => "\\+",
        other => {
            out.push(other);
            return;
        }
    };
    out.push_str(esc);
}

fn longest_backtick_run(s: &str) -> usize {
    let mut max = 0;
    let mut run = 0;
    for c in s.chars() {
        if c == '`' {
            run += 1;
            max = max.max(run);
        } else {
            run = 0;
        }
    }
    max
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::from_markdown;
    use crate::model::{Line, Loss, Mark};

    /// The contract: export∘import is the identity on the content (modulo
    /// island loss class, which our test islands don't trigger).
    fn round_trips(md: &str) {
        let rt = from_markdown(md).unwrap();
        let md2 = to_markdown(&rt);
        let rt2 = from_markdown(&md2).unwrap();
        assert_eq!(
            rt, rt2,
            "content not a fixed point.\n  in:  {md:?}\n  mid: {md2:?}"
        );
    }

    /// A link over an island slot. A linked image is plain
    /// CommonMark, so it sits inside the fixed-point domain: the link arm must
    /// route its display text through the island renderer rather than emit the
    /// slot char raw (which re-imports as nothing, losing island, link, and the
    /// char with it).
    #[test]
    fn link_over_island_slot_round_trips() {
        round_trips("[![a cat](cat.png)](https://e.com)");
        round_trips("[a ![cat](cat.png) b](https://e.com)");
        assert_eq!(
            to_markdown(&from_markdown("[![a cat](cat.png)](https://e.com)").unwrap()),
            "[![a cat](cat.png)](https://e.com)"
        );
    }

    /// A `Code` mark over an island slot: editor-buildable, not
    /// importable. A code span is emitted verbatim, so it cannot carry the
    /// island; the span splits around the slot, keeping both the text and the
    /// island (one code mark lowered to two).
    #[test]
    fn code_mark_over_island_slot_keeps_the_island() {
        let mut rt = from_markdown("a ![x](y.png) b").unwrap();
        rt.marks.push(Mark { start: 0, end: 5, kind: MarkKind::Code });
        rt.normalize();
        assert_eq!(rt.validate(), Ok(()));
        let md = to_markdown(&rt);
        assert_eq!(md, "`a `![x](y.png)` b`");
        let rt2 = from_markdown(&md).unwrap();
        assert_eq!(rt2.text, rt.text);
        assert_eq!(rt2.islands.len(), 1);
    }

    /// An unknown block role projects as a paragraph and an unknown
    /// container projects transparently: the older reader renders a future
    /// construct as plain prose instead of refusing it. Text and marks survive;
    /// only the role/container is lost, and only to *markdown* (both round-trip
    /// through storage).
    #[test]
    fn unknown_block_vocabulary_projects_as_prose() {
        let mut rt = Content {
            text: "heads up\nstill inside".into(),
            lines: vec![
                Line {
                    kind: LineKind::Unknown {
                        tag: "callout".into(),
                        attrs: serde_json::json!({"variant": "warn"}),
                    },
                    containers: vec![Container::Unknown {
                        tag: "indent".into(),
                        attrs: serde_json::Value::Null,
                    }],
                    continues: false,
                },
                Line {
                    kind: LineKind::Para,
                    containers: vec![Container::Unknown {
                        tag: "indent".into(),
                        attrs: serde_json::Value::Null,
                    }],
                    continues: false,
                },
            ],
            marks: vec![Mark { start: 0, end: 5, kind: MarkKind::Strong }],
            islands: vec![],
        };
        rt.normalize();
        assert_eq!(rt.validate(), Ok(()));
        // Two paragraphs at the top level: no container prefix, no placeholder.
        assert_eq!(to_markdown(&rt), "**heads** up\n\nstill inside");
        // An unknown container nested inside a known one keeps the known
        // prefix and adds none of its own.
        rt.lines[0].containers.insert(0, Container::Quote);
        rt.lines[1].containers.insert(0, Container::Quote);
        assert_eq!(to_markdown(&rt), "> **heads** up\n>\n> still inside");
    }

    /// [`to_plaintext`] keeps literal text, drops marks, and strips island
    /// slots: the lossy projection pdfform binds to non-content fields.
    #[test]
    fn plaintext_drops_marks_and_islands() {
        // Marks contribute no delimiters to plaintext.
        let rt = marked(
            "bold text",
            vec![Mark { start: 0, end: 4, kind: MarkKind::Strong }],
        );
        assert_eq!(to_plaintext(&rt), "bold text");
        // An island slot in the text is removed.
        let mut rt = Content {
            text: format!("see {ISLAND_SLOT} here"),
            lines: vec![Line { kind: LineKind::Para, containers: vec![], continues: false }],
            marks: vec![],
            islands: vec![Island {
                id: String::new(),
                island_type: "image".into(),
                props: serde_json::Value::Null,
                loss: Loss::UNREPRESENTABLE,
            }],
        };
        rt.normalize();
        assert_eq!(to_plaintext(&rt), "see  here");
    }

    /// A single-paragraph content over `text` with hand-placed `marks`: the
    /// free-overlap shapes an editor's `apply_mark_ops` produces but markdown
    /// import never does. Normalized + validated before use.
    fn marked(text: &str, marks: Vec<Mark>) -> Content {
        let mut rt = Content {
            text: text.to_string(),
            lines: vec![Line {
                kind: LineKind::Para,
                containers: vec![],
                continues: false,
            }],
            marks,
            islands: vec![],
        };
        rt.normalize();
        assert_eq!(rt.validate(), Ok(()), "content invariants");
        rt
    }

    /// One deterministic fixed-point smoke per block/inline construct. Each is
    /// a `document()` generator arm that `properties.rs` fuzzes; kept as a
    /// labeled table so a break localizes to the exact construct without a
    /// proptest seed. Constructs with NO generator coverage (nested_marks,
    /// code_block, thematic_break, hard breaks) stay as their own tests below.
    #[test]
    fn single_constructs_round_trip() {
        for (label, md) in [
            ("paragraph", "Hello world"),
            ("two_paragraphs", "one\n\ntwo"),
            ("marks", "a **b** _c_ ~~d~~ <u>e</u>"),
            ("heading", "## Title here"),
            ("inline_code", "run `cargo test` now"),
            ("bullet_list", "- a\n- b\n- c"),
            ("ordered_list", "3. a\n4. b"),
            ("multi_paragraph_item", "- first\n\n  second"),
            ("blockquote", "> quoted text"),
            ("link", "see [our site](https://example.com) now"),
            ("table", "| a | b |\n| --- | --- |\n| 1 | 2 |"),
            ("image", "see ![a cat](cat.png) here"),
        ] {
            println!("construct: {label}");
            round_trips(md);
        }
    }

    #[test]
    fn nested_marks() {
        round_trips("**bold _and italic_**");
    }

    #[test]
    fn code_block() {
        round_trips("```rust\nfn a() {}\nfn b() {}\n```");
    }

    #[test]
    fn thematic_break() {
        round_trips("one\n\n***\n\ntwo");
    }

    #[test]
    fn thematic_break_canonicalizes_to_dashes() {
        // `***`/`___` and `---` all import to the same `Rule` line, so export
        // re-emits the canonical `---` whatever the source delimiter was.
        for src in ["***", "___", "- - -"] {
            let rt = from_markdown(&format!("one\n\n{src}\n\ntwo")).unwrap();
            let md = to_markdown(&rt);
            assert!(md.contains("\n\n---\n\n"), "source: {src}, got: {md:?}");
        }
    }

    #[test]
    fn literal_asterisks_escaped() {
        round_trips("2 * 3 = 6 and a_b_c");
    }

    #[test]
    fn hard_break_round_trips() {
        round_trips("line one\\\nline two");
    }

    #[test]
    fn hard_break_in_list_item() {
        round_trips("- one\\\ntwo\n- three");
    }

    #[test]
    fn leading_ordered_marker_escaped() {
        // Content prose that begins `N.` must not re-import as an ordered list.
        let mut rt = from_markdown("x").unwrap();
        rt.text = "1. not a list".into();
        let md = to_markdown(&rt);
        let back = from_markdown(&md).unwrap();
        assert_eq!(back.lines[0].kind, LineKind::Para);
        assert!(back.lines[0].containers.is_empty());
        assert_eq!(back, rt);
    }

    #[test]
    fn table_with_formatted_cells_round_trips() {
        // Option A: cells carry {text, marks}; export reconstructs their markdown
        // from structure, so the content is a fixed point across formatted cells.
        round_trips("| Name | Note |\n| --- | --- |\n| **bold** | _italic_ |");
        round_trips("| A |\n| --- |\n| **b** and _i_ `c` [d](https://e.com) ~~e~~ |");
        round_trips("| A |\n| --- |\n| <u>under</u> |");
        // A literal pipe inside a cell survives via `\|` re-escaping on export.
        round_trips("| A |\n| --- |\n| a \\| b |");
    }

    #[test]
    fn formatted_cell_marks_are_structured_not_reparsed() {
        // The cell stores marks, not a markdown slice: a strong cell's island
        // props carry a `strong` mark over the cell-local range, and export
        // renders it back to `**bold**` from that structure.
        let rt = from_markdown("| H |\n| --- |\n| **bold** |").unwrap();
        let cell = &rt.islands[0].props["rows"][0][0];
        assert_eq!(cell["text"], "bold");
        assert_eq!(cell["marks"][0]["type"], "strong");
        assert_eq!(cell["marks"][0]["start"], 0);
        assert_eq!(cell["marks"][0]["end"], 4);
        assert!(to_markdown(&rt).contains("**bold**"));
    }

    #[test]
    fn known_hard_break_limits() {
        // Recorded, not hidden: a mark spanning a hard break splits per line.
        let rt = from_markdown("**one\\\ntwo**").unwrap();
        let rt2 = from_markdown(&to_markdown(&rt)).unwrap();
        // The spanning strong becomes two per-line strongs on round-trip.
        assert!(
            rt != rt2,
            "if this ever round-trips, promote it out of the known-limits list"
        );
        assert_eq!(rt2.marks.len(), 2, "mark split across the hard break");
    }

    #[test]
    fn anchor_marks_omitted_but_text_survives() {
        let mut rt = from_markdown("comment target here").unwrap();
        rt.marks.push(Mark {
            start: 8,
            end: 14,
            kind: MarkKind::Anchor { id: "c1".into() },
        });
        rt.normalize();
        let md = to_markdown(&rt);
        // No anchor syntax in the projection, but the text round-trips.
        let rt2 = from_markdown(&md).unwrap();
        assert_eq!(rt2.text, "comment target here");
        assert!(!md.contains("c1"));
    }

    // ---------------------------------------------------------------------
    // Markdown export fixed-point violations.
    // ---------------------------------------------------------------------

    /// The exact repro: `strong[0,4)` + `emph[2,6)` over "abcdef" must
    /// export balanced markdown that preserves the text, not `**ab*cdef*`,
    /// which re-imports as LITERAL `**abcdef` text (a silent content change).
    /// The two marks share the `*` delimiter character, so their
    /// overlap is unrepresentable in CommonMark (a reopened `*` abutting `**`
    /// merges into `***`); the export nests them by truncation, a documented
    /// codec limit: text intact, the crossing tail of the inner mark dropped.
    #[test]
    fn overlapping_asterisk_marks_stay_text_safe() {
        let rt = marked(
            "abcdef",
            vec![
                Mark {
                    start: 0,
                    end: 4,
                    kind: MarkKind::Strong,
                },
                Mark {
                    start: 2,
                    end: 6,
                    kind: MarkKind::Emph,
                },
            ],
        );
        let md = to_markdown(&rt);
        assert_eq!(md, "**ab*cd***ef", "balanced, no literal `**` leak");
        let rt2 = from_markdown(&md).unwrap();
        // Text is preserved exactly: the corruption the issue reported is gone.
        assert_eq!(rt2.text, "abcdef");
        // Documented limit: same-delimiter overlap degrades to its nested subset.
        assert_eq!(
            rt2.marks,
            vec![
                Mark {
                    start: 0,
                    end: 4,
                    kind: MarkKind::Strong
                },
                Mark {
                    start: 2,
                    end: 4,
                    kind: MarkKind::Emph
                },
            ]
        );
    }

    /// Overlap between marks with *distinct* delimiters round-trips exactly:
    /// the close-and-reopen sweep lowers `strong[0,4)` + `strike[2,6)` to
    /// `**ab~~cd~~**~~ef~~`, which re-imports to the same overlapping content.
    #[test]
    fn overlapping_distinct_delim_marks_round_trip_exactly() {
        for (k1, k2) in [
            (MarkKind::Strong, MarkKind::Strike),
            (MarkKind::Strike, MarkKind::Strong),
            (MarkKind::Emph, MarkKind::Strike),
            (MarkKind::Underline, MarkKind::Emph),
            (MarkKind::Strong, MarkKind::Underline),
        ] {
            let rt = marked(
                "abcdef",
                vec![
                    Mark {
                        start: 0,
                        end: 4,
                        kind: k1.clone(),
                    },
                    Mark {
                        start: 2,
                        end: 6,
                        kind: k2.clone(),
                    },
                ],
            );
            let md = to_markdown(&rt);
            let rt2 = from_markdown(&md).unwrap();
            assert_eq!(rt, rt2, "{k1:?}+{k2:?} overlap not a fixed point: {md:?}");
        }
    }

    /// A formatting mark partially overlapping an atomic `code` span can't wrap
    /// the code's interior; the wrap clips to the text outside so the markdown
    /// stays balanced (the atomic-balance shape, here in the markdown emitter).
    #[test]
    fn wrap_over_code_stays_balanced() {
        let rt = marked(
            "abcdef",
            vec![
                Mark {
                    start: 0,
                    end: 4,
                    kind: MarkKind::Strong,
                },
                Mark {
                    start: 2,
                    end: 6,
                    kind: MarkKind::Code,
                },
            ],
        );
        let md = to_markdown(&rt);
        assert_eq!(md, "**ab**`cdef`");
        let rt2 = from_markdown(&md).unwrap();
        assert_eq!(rt2.text, "abcdef");
    }

    /// A literal `&` (or an entity-shaped `&amp;`) must not
    /// re-import as the decoded entity. `from_markdown("\\&amp;")` yields content
    /// text "&amp;"; exporting it unescaped as `&amp;` would re-import as "&".
    #[test]
    fn ampersand_and_entities_round_trip() {
        // A bare `&` and an entity-shaped run both survive.
        round_trips("a & b");
        round_trips("copyright \\&copy; sign");
        // The pinned repro: literal "&amp;" text.
        let rt = from_markdown("\\&amp;").unwrap();
        assert_eq!(rt.text, "&amp;");
        let md = to_markdown(&rt);
        assert!(md.contains("\\&"), "the `&` must be escaped, got {md:?}");
        let rt2 = from_markdown(&md).unwrap();
        assert_eq!(rt2.text, "&amp;", "entity-shaped text must not decode");
        assert_eq!(rt, rt2);
    }

    /// Heading text ending in a `#` run must not re-import as
    /// an ATX closing sequence. `from_markdown("# a \\#")` yields heading text
    /// "a #"; exporting it as `# a #` would re-import as "a", dropping the `#`.
    #[test]
    fn heading_trailing_hash_round_trips() {
        let rt = from_markdown("# a \\#").unwrap();
        assert_eq!(rt.text, "a #");
        let md = to_markdown(&rt);
        assert!(md.contains("\\#"), "trailing `#` must be escaped, got {md:?}");
        let rt2 = from_markdown(&md).unwrap();
        assert_eq!(rt2.text, "a #", "trailing `#` must survive");
        assert_eq!(rt, rt2);
        // A multi-`#` trailing run and a no-space `#` both round-trip.
        round_trips("# heading \\#\\#");
        round_trips("## title\\#");
    }

    // ---------------------------------------------------------------------
    // Unescaped image alt/URL and link URL cause silent content
    // loss on round-trip (a special char terminates the markup early).
    // ---------------------------------------------------------------------

    /// The exact image repro: an alt with `\]` imports to alt text "a]b";
    /// exporting it unescaped as `![a]b](x.png)` re-imports as prose with the
    /// image gone. The alt must be escaped so the island survives.
    #[test]
    fn image_alt_specials_round_trip() {
        // `]`, `\`, `&`, and emphasis delimiters in alt all survive.
        round_trips("see ![a\\]b](x.png) here");
        round_trips("see ![a\\\\b](x.png) here");
        round_trips("see ![a&b](x.png) here");
        round_trips("see ![a\\*b\\_c](x.png) here");
        // The pinned repro: the image is not lost.
        let rt = from_markdown("see ![a\\]b](x.png) here").unwrap();
        assert_eq!(rt.islands.len(), 1, "one image island");
        assert_eq!(rt.islands[0].props["alt"], "a]b");
        let md = to_markdown(&rt);
        let rt2 = from_markdown(&md).unwrap();
        assert_eq!(rt2.islands.len(), 1, "image survived, got md {md:?}");
        assert_eq!(rt2.islands[0].props["alt"], "a]b");
    }

    /// The exact link repro: a URL with a space imports to link url
    /// "foo bar"; exporting it unescaped as `[t](foo bar)` re-imports as prose
    /// with the link gone. The URL must be angle-wrapped.
    #[test]
    fn link_url_with_space_round_trips() {
        round_trips("a [t](<foo bar>) b");
        let rt = from_markdown("a [t](<foo bar>) b").unwrap();
        assert!(
            rt.marks
                .iter()
                .any(|m| matches!(&m.kind, MarkKind::Link { url } if url == "foo bar")),
            "link url with space imported"
        );
        let md = to_markdown(&rt);
        assert!(md.contains("<foo bar>"), "url angle-wrapped, got {md:?}");
        let rt2 = from_markdown(&md).unwrap();
        assert_eq!(rt, rt2);
    }

    /// Image and link URLs carrying the destination-terminating specials
    /// (unbalanced parens, `&`, `<`/`>`, backslash) all round-trip.
    #[test]
    fn url_specials_round_trip() {
        // Balanced parens stay bare (CommonMark permits them); the others wrap.
        round_trips("see [t](https://en.wikipedia.org/wiki/Rust_(programming_language)) x");
        round_trips("see ![a](<x y.png>) here");
        round_trips("see [t](<a )b>) x");
        round_trips("see [t](<a&b>) x");
        round_trips("see [t](<a\\<b\\>c>) x");
        round_trips("see [t](<a\\\\b>) x");
    }

    /// `emit_url` chooses bare for a clean URL and angle-wraps only when a
    /// special forces it: the aesthetic contract (common URLs stay unbracketed).
    #[test]
    fn emit_url_bare_when_safe() {
        let mut bare = String::new();
        emit_url("https://ex.com/a(b)c", &mut bare);
        assert_eq!(bare, "https://ex.com/a(b)c", "balanced parens stay bare");
        let mut wrapped = String::new();
        emit_url("a b", &mut wrapped);
        assert_eq!(wrapped, "<a b>", "space forces the wrap");
        let mut esc = String::new();
        emit_url("a&<\\b", &mut esc);
        assert_eq!(esc, "<a\\&\\<\\\\b>", "specials escaped inside the wrap");
    }

    // ---------------------------------------------------------------------
    // The verify-and-drop net's search.
    // ---------------------------------------------------------------------

    /// A strong mark whose delimiters land where CommonMark won't read them as a
    /// run (`**a.**b` re-imports as literal text) is dropped, and every other
    /// mark on the line is kept. The re-add order is document order, so position
    /// does not decide survival: a good mark before *or* after a leaking one
    /// lives.
    #[test]
    fn net_drops_only_the_leaking_marks() {
        for (label, text, spans, want) in [
            ("leaking alone", "a.b", vec![(0, 2)], "a.b"),
            ("representable alone", "ab cd", vec![(0, 2)], "**ab** cd"),
            (
                "leaking then representable",
                "a.b and cd ef",
                vec![(1, 3), (8, 10)],
                "a.b and **cd** ef",
            ),
            (
                "representable then leaking",
                "cd ef and a.b",
                vec![(0, 2), (11, 13)],
                "**cd** ef and a.b",
            ),
        ] {
            let rt = marked(
                text,
                spans
                    .into_iter()
                    .map(|(start, end)| Mark {
                        start,
                        end,
                        kind: MarkKind::Strong,
                    })
                    .collect(),
            );
            let md = to_markdown(&rt);
            assert_eq!(md, want, "{label}");
            assert_eq!(from_markdown(&md).unwrap().text, text, "{label}: text drift");
        }
    }

    /// A line at the [`PROBE_BUDGET`] guarantee (32 flanking marks, half of them
    /// unrepresentable) resolves exactly: the budget covers the full search, so
    /// no representable mark is lost to it.
    #[test]
    fn net_resolves_a_line_at_the_budget_guarantee() {
        let (mut text, mut marks) = (String::new(), Vec::new());
        for i in 0..32 {
            let at = i * 6;
            if i % 2 == 0 {
                text.push_str("abcde "); // `**abcde**`: a run on both edges
                marks.push((at, at + 5));
            } else {
                text.push_str("abc.e "); // `**.e**`: cannot open against `.`
                marks.push((at + 3, at + 5));
            }
        }
        let text = text.trim_end();
        let rt = marked(
            text,
            marks
                .into_iter()
                .map(|(start, end)| Mark {
                    start,
                    end,
                    kind: MarkKind::Strong,
                })
                .collect(),
        );
        let md = to_markdown(&rt);
        assert_eq!(md.matches("**abcde**").count(), 16, "every good mark kept");
        assert_eq!(md.matches("**").count(), 32, "every leaking mark dropped");
        assert_eq!(from_markdown(&md).unwrap().text, text);
    }

    /// Past the budget the net still terminates and still preserves the text; it
    /// drops what it has not cleared. The pathological shape (one
    /// unrepresentable mark per five chars) costs a bounded number of re-parses
    /// instead of one per mark.
    #[test]
    fn net_bounds_a_pathological_line() {
        let text = "a.b, ".repeat(200);
        let text = text.trim_end();
        let rt = marked(
            text,
            (0..200)
                .map(|i| Mark {
                    start: i * 5 + 1,
                    end: i * 5 + 4,
                    kind: MarkKind::Strong,
                })
                .collect(),
        );
        let md = to_markdown(&rt);
        assert!(!md.contains('*'), "every leaking mark dropped: {md:?}");
        assert_eq!(from_markdown(&md).unwrap().text, text);
    }

    #[test]
    fn ordered_list_marker_saturates_on_overflow() {
        // `validate` does not ceiling `start`/`ordinal`, so a corrupt content can
        // carry `start == u64::MAX`. Export must not panic (or wrap silently) on
        // the `start + ordinal` marker; it saturates instead.
        let json = format!(
            r#"{{"text":"x","lines":[{{"kind":"para","containers":[{{"container":"list_item","ordered":true,"start":{},"ordinal":5}}]}}],"marks":[],"islands":[]}}"#,
            u64::MAX
        );
        let rt = Content::from_canonical_json(&json).unwrap();
        let md = to_markdown(&rt);
        assert!(
            md.contains(&format!("{}. ", u64::MAX)),
            "marker saturates to u64::MAX: {md:?}"
        );
    }
}
