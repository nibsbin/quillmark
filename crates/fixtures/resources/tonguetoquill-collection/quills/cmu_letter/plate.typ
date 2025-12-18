#import "@preview/tonguetoquill-cmu-letter:0.1.0": frontmatter, mainmatter, backmatter

#show: frontmatter.with(
  wordmark: image("assets/cmu-wordmark.svg"),
  department: {{ department | String(default="Department Name") }},
  address: {{ address | Lines(default=["Address Line 1", "Address Line 2"]) }},
  url: {{ url | String(default="www.cmu.edu") }},
  date: {{ date | Date }},
  recipient: {{ recipient | Lines(default=["Recipient Name", "Address"]) }},
)

#show: mainmatter

#{{ BODY | Content }}

#backmatter(
  signature_block: {{ signature_block | Lines(default=["First M. Last", "Title"]) }}
)
