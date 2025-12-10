#import "@preview/tonguetoquill-usaf-memo:1.0.0": frontmatter, mainmatter, backmatter, indorsement

// Frontmatter configuration
#show: frontmatter.with(
  // Letterhead configuration
  letterhead_title: {{ letterhead_title | String(default="letterhead-title") }},
  letterhead_caption: {{ letterhead_caption | Lines(default=["letterhead-caption"]) }},
  letterhead_seal: image("assets/dow_seal.png"),

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

#{{ body | Content }}

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

// Indorsements
{% if indorsements is defined %}
{% for ind in indorsements %}
#indorsement(
  from: {{ ind.from | String }},
  to: {{ ind.for | String }},
  signature_block: {{ ind.signature_block | Lines }},
  {% if ind.attachments is defined %}
  attachments: {{ ind.attachments | Lines }},
  {% endif %}
  {% if ind.cc is defined %}
  cc: {{ ind.cc | Lines }},
  {% endif %}
  new_page: {{ ind.new_page | default(false) }},
  informal: {{ ind.informal | default(false) }},
  {% if ind.date is defined %}
  date: {{ ind.date | String }},
  {% endif %}
)[
  #{{ ind.body | Content }}
]
{% endfor %}
{% endif %}
