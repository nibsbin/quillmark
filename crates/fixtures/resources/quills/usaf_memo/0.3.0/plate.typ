#import "@local/quillmark-helper:0.1.0": data, signature-field
#import "@local/tonguetoquill-usaf-memo:3.0.0": backmatter, frontmatter, indorsement, mainmatter

// Frontmatter configuration
#show: frontmatter.with(
  // Letterhead configuration
  letterhead_title: data.letterhead_title,
  letterhead_caption: data.letterhead_caption,
  letterhead_seal_subtitle: data.letterhead_seal_subtitle,
  // Every enum branch below covers `values ∪ blank`: an `else` that swallows the
  // blank renders a variant nobody picked. Here the blank omits the seal, which
  // is what `frontmatter`'s own `letterhead_seal: none` default means.
  ..if data.letterhead_seal != "" {
    (letterhead_seal: image(
      if data.letterhead_seal == "dod" {
        "assets/dod_seal.png"
      } else {
        "assets/dow_seal.png"
      }
    ))
  },

  // Date
  date: data.date,

  // Receiver information
  memo_for: data.memo_for,

  // Sender information (omitted for Memorandum for Record). Every declared
  // field arrives filled, so the empty *value* is the signal here, not absence.
  ..if data.memo_from.len() > 0 { (memo_from: data.memo_from) },

  // Subject line
  subject: data.subject,

  references: data.references,
  footer_tag_line: data.tag_line,
  classification_level: data.classification,
  dissemination: data.dissemination,

  // CUI designation indicator block fields (DoDM 5200.48)
  cui_controlled_by: data.cui_controlled_by,
  cui_category: data.cui_category,
  cui_limited_dissemination: data.cui_limited_dissemination,
  cui_poc: data.cui_poc,

  // USAF vs DAF memorandum style (date format, body indentation). A memo has no
  // "no style" state, so the blank takes the package's default; `frontmatter`
  // asserts membership of ("usaf", "daf") and would fail the compile on a blank.
  ..if data.memo_style != "" { (memo_style: data.memo_style) },

  // Font size
  font_size: data.font_size * 1pt,

  // List recipients in vertical list
  memo_for_cols: 1,
)

// Mainmatter. The body's region needs no recovery step here: the package's
// render-body rebuilds paragraphs through a state buffer (AFH 33-337
// auto-numbering), but the rebuilt glyphs keep their spans, which is what
// the backend reads regions from.
#mainmatter[
  #data.at("$body")
]

// Backmatter
#backmatter(
  // Signature block
  signature_block: data.signature_block,
  signing_field: signature-field("Signature", field: "signature_block"),

  cc: data.cc,
  distribution: data.distribution,
  attachments: data.attachments,
)

// Indorsements - iterate through CARDS array and filter by CARD tag
#for (i, card) in data.at("$cards").enumerate() {
  // `$kind` is document-defined: a card block with no `$kind:` line carries
  // none, so read it with a default rather than a bare `.at`.
  if card.at("$kind", default: none) == "indorsement" {
    // The quillmark helper leaves an unset/whitespace-only markdown body as
    // the empty string `""`; only non-empty bodies are eval'd into content.
    // Pass truly empty content (`[]`) in the empty case so indorsement can
    // collapse the body's surrounding spacing.
    let body = card.at("$body", default: "")
    let body_content = if type(body) == str { [] } else { body }
    // Per AFH 33-337 Ch. 14, an indorsement is dated when the endorser signs
    // it (distinct from the originating memo's date). The signing date is
    // generally unknown at compile time and filled in by hand, so a blank date
    // renders a fill-in line rather than stamping the compile date.
    let card_date = card.date
    let resolved_date = if card_date == "" { none } else { card_date }
    // The card's `$path` prefix composes its canonical schema addresses
    // (`$cards.indorsement.<n>.…`, per-kind ordinal): the absolute loop
    // index `i` is NOT that ordinal once kinds interleave, so it stays a
    // widget-name suffix only. The card body's region rides its own glyph
    // spans through the package rebuild, per-card because each card's body
    // has its own backend-generated eval site.
    indorsement(
      from: card.from,
      // `for` is a Typst keyword, so this one declared field cannot be reached
      // as `card.for`; the `.at` is forced by syntax, not by any doubt that the
      // field is there.
      to: card.at("for"),
      signature_block: card.signature_block,
      signing_field: signature-field(
        "Ind_" + str(i) + "_Signature",
        field: card.at("$path") + "signature_block",
      ),
      // Same shape: `indorsement` asserts `format`'s membership, and an
      // indorsement has no "no layout" state. `action` needs no such guard —
      // the package reads a blank action as "no action line".
      ..if card.format != "" { (format: card.format) },
      date: resolved_date,
      action: card.action,
      body_content,
    )
  }
}
