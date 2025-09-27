#import "@preview/tonguetoquill-usaf-memo:0.1.1": official-memorandum, indorsement

// Generate the official memorandum with validated and processed input
#show:official-memorandum.with(
  // Letterhead configuration
  letterhead-title: {{ letterhead_title | default| String }},
  letterhead-caption: {{ letterhead_caption | Array }},
  letterhead-seal: image("assets/dod_seal.gif"),

  // Frontmatter
  date: {{ date | Date }},
  memo-for: {{ memo_for | Array }},

  // Sender information
  memo-from: {{ memo_from | Array }},
  
  // Subject line
  subject: {{ subject | String}},
  
  // Optional references
  references: {{ references | Array}},

  //Optional cc
  cc: {{ cc | Array }},

  //Optional distribution
  distribution: {{ distribution | Array }},

  // Optional attachments
  attachments: {{ attachments | Array }},
  
  // Signature block
  signature-block: {{ signature_block | Array }},

  {{body | Body }}
)