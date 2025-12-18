#import "@preview/tonguetoquill-usaf-memo:1.0.0": frontmatter, mainmatter, backmatter, indorsement

// Frontmatter configuration
#show: frontmatter.with(
  // Letterhead configuration
  letterhead_title: {{ letterhead_title | String(default="letterhead-title") }},
  letterhead_caption: {{ letterhead_caption | Lines(default=["letterhead-caption"]) }},
  letterhead_seal: image("assets/dow_seal.jpg"),

  // Date
  date: {{ date | Date }},

  // Receiver information
  memo_for: {{ memo_for | Lines(default=["memo_for"]) }},

  // Sender information
  memo_from: {{ memo_from | Lines(default=["memo_from"]) }},

  // Subject line
  subject: {{ subject | String(default="subject") }},

  // Optional references
  {% if references is defined %}
  references: {{ references | Lines }},
  {% endif %}

  // Optional footer tag line
  {% if tag_line is defined %}
  footer_tag_line: {{ tag_line | String }},
  {% endif %}

  // Optional classification level
  {% if classification is defined %}
  classification_level: {{ classification | String }},
  {% endif %}

  // Font size
  {% if font_size is defined %}
  font_size: {{ font_size }}pt,
  {% endif %}

  // List recipients in vertical list
  memo_for_cols: 1,
)

// Mainmatter configuration
#show: mainmatter

#{{ BODY | Content }}

// Backmatter
#backmatter(
  // Signature block
  signature_block: {{ signature_block | Lines(default=["signature_block"]) }},

  // Optional cc
  {% if cc is defined %}
  cc: {{ cc | Lines }},
  {% endif %}

  // Optional distribution
  {% if distribution is defined %}
  distribution: {{ distribution | Lines }},
  {% endif %}

  // Optional attachments
  {% if attachments is defined %}
  attachments: {{ attachments | Lines }},
  {% endif %}
)

// Indorsements - iterate through CARDS array and filter by CARD type
{% for card in CARDS %}
{% if card.CARD == "indorsement" %}
#indorsement(
  from: {{ card.from | String }},
  to: {{ card.for | String }},
  signature_block: {{ card.signature_block | Lines }},
  {% if card.attachments is defined %}
  attachments: {{ card.attachments | Lines }},
  {% endif %}
  {% if card.cc is defined %}
  cc: {{ card.cc | Lines }},
  {% endif %}
  format: {{ card.format | String(default="standard") }},
  {% if card.date is defined %}
  date: {{ card.date | String }},
  {% endif %}
)[
  #{{ card.BODY | Content }}
]
{% endif %}
{% endfor %}