#import "@preview/tonguetoquill-usaf-memo:0.1.1": official-memorandum, indorsement

// Generate the official memorandum with validated and processed input
#show:official-memorandum.with(
  // Letterhead configuration
  letterhead-title: {{ letterhead_title | String(default="letterhead-title") }},
  letterhead-caption: {{ letterhead_caption | Lines(default=["letterhead-caption"]) }},
  letterhead-seal: image("assets/dod_seal.gif"),

  // Frontmatter
  date: {{ date | Date }},
  
  // Receiver information
  memo-for: {{ memo_for | Lines(default=["memo_for"]) }},

  // Sender information
  memo-from: {{ memo_from | Lines(default=["memo_from"]) }},
  
  // Subject line
  subject: {{ subject | String(default="subject") }},

  // Optional references
  {%- if references is defined -%}
  references: {{ references | Lines }},
  {%- endif -%}

  {%- if cc is defined -%}
  cc: {{ cc | Lines }},
  {%- endif -%}

  // //Optional distribution
  {%- if distribution is defined -%}
  distribution: {{ distribution | Lines }},
  {%- endif -%}

  // // Optional attachments
  {%- if attachments is defined -%}
  attachments: {{ attachments | Lines }},
  {%- endif -%}
  
  // Signature block
  signature-block: {{ signature_block | Lines(default=["signature_block"]) }},
)

#{{ body | Content }}
