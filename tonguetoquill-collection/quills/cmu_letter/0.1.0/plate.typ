#import "@local/quillmark-helper:0.1.0": data, eval-markup, parse-date
#import "@preview/tonguetoquill-cmu-letter:0.1.0": backmatter, frontmatter, mainmatter

#show: frontmatter.with(
  wordmark: image("assets/cmu-wordmark.svg"),
  department: data.department,
  address: data.address,
  url: data.url,
  date: if "date" in data { parse-date(data.date) } else { datetime.today() },
  recipient: data.recipient,
)

#show: mainmatter

#eval-markup(data.BODY)

#backmatter(
  signature_block: data.signature_block,
)
