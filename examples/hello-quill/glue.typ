#import "@preview/tonguetoquill-usaf-memo:0.1.1": official-memorandum, indorsement

// Generate the official memorandum with validated and processed input
#show:official-memorandum.with(
  // Letterhead configuration
  letterhead-title: {{ letterhead-title | String }},
  letterhead-caption: {{ letterhead-caption | List }},
  letterhead-seal: image("assets/dod_seal.gif"),

  // Frontmatter
  date: {{ date | Date }},
  memo-for: {{ memo-for | List }},

  // Sender information
  memo-from: {{ memo-from | List }},
  
  // Subject line
  subject: {{ subject | String}},
  
  // Optional references
  references: {{ references | List}},

  //Optional cc
  cc: {{ cc | List }},

  //Optional distribution
  distribution: {{ distribution | List }},

  // Optional attachments
  attachments: {{ attachments | List }},
  
  // Signature block
  signature-block: {{ signature-block | List }},

  {{BODY | Body }}
)