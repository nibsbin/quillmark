#import "@local/quillmark-helper:0.1.0": data, display
#import "@local/tonguetoquill-cmu-letter:0.1.0": backmatter, frontmatter, mainmatter, DATE_PATTERN

#show: frontmatter.with(
  wordmark: image("assets/cmu-wordmark.svg"),
  department: data.department,
  address: data.address,
  url: data.url,
  // The field's content projection, not `data.date`: the package inks the date
  // internally, and only generated ink carries the schema address a click
  // routes on. `none` for a blank date, which `frontmatter` reads as "today".
  date: display("date", DATE_PATTERN),
  recipient: data.recipient,
)

#show: mainmatter

#data.at("$body")

#backmatter(
  signature_block: data.signature_block,
)
