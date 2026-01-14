#import "@local/quillmark-helper:0.1.0": data, eval-markup, parse-date
#import "@preview/tonguetoquill-usaf-memo:1.0.0": frontmatter, mainmatter, backmatter, indorsement

// Frontmatter configuration
#show: frontmatter.with(
  // Letterhead configuration
  letterhead_title: data.at("letterhead_title", default: "letterhead-title"),
  letterhead_caption: data.at("letterhead_caption", default: ("letterhead-caption",)),
  letterhead_seal: image("assets/dow_seal.jpg"),

  // Date
  date: parse-date(data.date),

  // Receiver information
  memo_for: data.at("memo_for", default: ("memo_for",)),

  // Sender information
  memo_from: data.at("memo_from", default: ("memo_from",)),

  // Subject line
  subject: data.at("subject", default: "subject"),

  // Optional references
  ..if "references" in data {
    (references: data.references,)
  } else {
    (:)
  },

  // Optional footer tag line
  ..if "tag_line" in data {
    (footer_tag_line: data.tag_line,)
  } else {
    (:)
  },

  // Optional classification level
  ..if "classification" in data {
    (classification_level: data.classification,)
  } else {
    (:)
  },

  // Font size
  ..if "font_size" in data {
    (font_size: data.font_size * 1pt,)
  } else {
    (:)
  },

  // List recipients in vertical list
  memo_for_cols: 1,
)

// Mainmatter configuration
#mainmatter[
  #eval-markup(data.BODY)
]

// Backmatter
#backmatter(
  // Signature block
  signature_block: data.at("signature_block", default: ("signature_block",)),

  // Optional cc
  ..if "cc" in data {
    (cc: data.cc,)
  } else {
    (:)
  },

  // Optional distribution
  ..if "distribution" in data {
    (distribution: data.distribution,)
  } else {
    (:)
  },

  // Optional attachments
  ..if "attachments" in data {
    (attachments: data.attachments,)
  } else {
    (:)
  },
)

// Indorsements - iterate through CARDS array and filter by CARD type
#for card in data.at("CARDS", default: ()) {
  if card.CARD == "indorsement" {
    indorsement(
      from: card.from,
      to: card.for,
      signature_block: card.signature_block,
      ..if "attachments" in card {
        (attachments: card.attachments,)
      } else {
        (:)
      },
      ..if "cc" in card {
        (cc: card.cc,)
      } else {
        (:)
      },
      format: card.at("format", default: "standard"),
      ..if "date" in card {
        (date: card.date,)
      } else {
        (:)
      },
    )[
      #eval-markup(card.BODY)
    ]
  }
}
