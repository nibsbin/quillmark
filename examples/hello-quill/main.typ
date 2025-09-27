#import "@preview/tonguetoquill-usaf-memo:0.1.1": official-memorandum, indorsement

// Generate the official memorandum with validated and processed input
#show:official-memorandum.with(
  // Letterhead configuration
  letterhead-title: {{ letterhead-title }},
  letterhead-caption: {{ letterhead-caption }},
  letterhead-seal: image("assets/dod_seal.gif"),

  // Frontmatter
  date: {{ date }},
  memo-for: {{ memo-for }},

  // Sender information
  memo-from: {{ memo-from }},
  
  // Subject line
  subject: {{ subject }},
  
  // Optional references
  references: {{ references }},

  //Optional cc
  cc: {{ cc }},

  //Optional distribution
  distribution: {{ distribution }},

  // Optional attachments
  attachments: {{ attachments }},
  
  // Signature block
  signature-block: {{ signature-block }},

  {{BODY}}
)


eval({{ BODY }}, mode: "markup")