#import "@local/quillmark-helper:0.1.0": data, content, parse-date
#import "@preview/tonguetoquill-cmu-letter:0.1.0": frontmatter, mainmatter, backmatter

#show: frontmatter.with(
  wordmark: image("assets/cmu-wordmark.svg"),
  department: data.at("department", default: "Department Name"),
  address: data.at("address", default: ("Address Line 1", "Address Line 2")),
  url: data.at("url", default: "www.cmu.edu"),
  date: parse-date(data.date),
  recipient: data.at("recipient", default: ("Recipient Name", "Address")),
)

#show: mainmatter

#content(data.BODY)

#backmatter(
  signature_block: data.at("signature_block", default: ("First M. Last", "Title"))
)
