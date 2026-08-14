#import "@local/quillmark-helper:0.1.0": data, signature-field
#import "@local/tonguetoquill-usaf-memo:3.0.0": backmatter, frontmatter, indorsement, mainmatter

// Frontmatter configuration
#show: frontmatter.with(
  // Letterhead configuration
  letterhead_title: data.letterhead_title,
  letterhead_caption: data.letterhead_caption,
  letterhead_seal_subtitle: data.at("letterhead_seal_subtitle", default: none),
  // Every enum branch below covers `values ∪ blank`. The blank is valid present
  // input, so it must land somewhere chosen rather than fall through an `else`
  // into a variant nobody picked. Here it omits the seal, which is exactly what
  // `frontmatter`'s own `letterhead_seal: none` default means.
  ..if data.at("letterhead_seal", default: "") != "" {
    (letterhead_seal: image(
      if data.letterhead_seal == "dod" {
        "assets/dod_seal.png"
      } else {
        "assets/dow_seal.png"
      }
    ))
  },

  // Date
  date: data.at("date", default: none),

  // Receiver information
  memo_for: data.memo_for,

  // Sender information (omitted for Memorandum for Record)
  ..if data.at("memo_from", default: ()).len() > 0 { (memo_from: data.memo_from) },

  // Subject line
  subject: data.subject,

  // Optional references
  ..if "references" in data { (references: data.references) },

  // Optional footer tag line
  ..if "tag_line" in data { (footer_tag_line: data.tag_line) },

  // Optional classification level
  ..if "classification" in data { (classification_level: data.classification) },

  ..if "dissemination" in data { (dissemination: data.dissemination) },

  // CUI designation indicator block fields (DoDM 5200.48)
  ..if "cui_controlled_by" in data { (cui_controlled_by: data.cui_controlled_by) },
  ..if "cui_category" in data { (cui_category: data.cui_category) },
  ..if "cui_limited_dissemination" in data { (cui_limited_dissemination: data.cui_limited_dissemination) },
  ..if "cui_poc" in data { (cui_poc: data.cui_poc) },

  // USAF vs DAF memorandum style (date format, body indentation). The blank
  // takes the package's own default: `frontmatter` asserts membership of
  // ("usaf", "daf"), so passing the blank through would fail the compile, and a
  // memo has no "no style" state to render.
  ..if data.at("memo_style", default: "") != "" { (memo_style: data.memo_style) },

  // Font size
  font_size: data.at("font_size", default: 12) * 1pt,

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

  // Optional cc
  ..if "cc" in data { (cc: data.cc) },

  // Optional distribution
  ..if "distribution" in data { (distribution: data.distribution) },

  // Optional attachments
  ..if "attachments" in data { (attachments: data.attachments) },
)

// Indorsements - iterate through CARDS array and filter by CARD tag
#for (i, card) in data.at("$cards").enumerate() {
  if card.at("$kind") == "indorsement" {
    // The quillmark helper leaves an unset/whitespace-only markdown body as
    // the empty string `""`; only non-empty bodies are eval'd into content.
    // Pass truly empty content (`[]`) in the empty case so indorsement can
    // collapse the body's surrounding spacing.
    let body = card.at("$body", default: "")
    let body_content = if type(body) == str { [] } else { body }
    // Per AFH 33-337 Ch. 14, an indorsement is dated when the endorser signs
    // it (distinct from the originating memo's date). The signing date is
    // generally unknown at compile time and filled in by hand, so a blank or
    // omitted date renders a fill-in line rather than stamping the compile date.
    let card_date = card.at("date", default: none)
    let resolved_date = if card_date == none or card_date == "" {
      none
    } else {
      card_date
    }
    // The card's `$path` prefix composes its canonical schema addresses
    // (`$cards.indorsement.<n>.…`, per-kind ordinal): the absolute loop
    // index `i` is NOT that ordinal once kinds interleave, so it stays a
    // widget-name suffix only. The card body's region rides its own glyph
    // spans through the package rebuild, per-card because each card's body
    // has its own backend-generated eval site.
    indorsement(
      from: card.at("from", default: ""),
      to: card.at("for", default: ""),
      signature_block: card.signature_block,
      signing_field: signature-field(
        "Ind_" + str(i) + "_Signature",
        field: card.at("$path") + "signature_block",
      ),
      // The blank takes the package's own default (`standard`): `indorsement`
      // asserts membership, and an indorsement has no "no layout" state. The
      // `action` enum needs no such guard — the package already reads a
      // blank action as "no action line".
      ..if card.at("format", default: "") != "" { (format: card.format) },
      date: resolved_date,
      ..if "action" in card { (action: card.action) },
      body_content,
    )
  }
}
