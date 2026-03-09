// frontmatter.typ: Frontmatter show rule for CMU letterhead
//
// This module implements the frontmatter (heading section) of a CMU letter
// per the official letterhead guidelines. It handles:
// - Page setup with proper margins
// - Header rendering (logo, department, address, URL)
// - Date, recipient, and salutation placement

#import "config.typ": *
#import "primitives.typ": *
#import "utils.typ": *

#let frontmatter(
  // Header content
  wordmark: none,
  department: none,
  address: none,
  url: none,

  // Letter metadata
  date: none,
  recipient: none,

  // Typography options
  body_font: DEFAULT_BODY_FONTS,
  font_size: DEFAULT_FONT_SIZE,

  it
) = {
  let actual_date = if date == none { datetime.today() } else { date }

  set page(
    paper: "us-letter",
    // Standard 1" margins for vertical flow layout
    margin: MARGINS,
  )

  set text(
    font: DEFAULT_BODY_FONTS,
    size: font_size,
  )

  set par(
    spacing: .7em,
    justify: false
  )

  // Render the header (in normal document flow)
  render-header(
    wordmark,
    department: department,
    address: address,
    url: url,
  )

  blank-lines(3)

  // Date and recipient in same paragraph to avoid extra spacing
  [#display-date(actual_date)
    #linebreak()
    #ensure-string(recipient)
  ]

  // Store metadata for downstream sections
  metadata((
    date: actual_date,
    body_font: body_font,
    font_size: font_size,
  ))

  it
}
