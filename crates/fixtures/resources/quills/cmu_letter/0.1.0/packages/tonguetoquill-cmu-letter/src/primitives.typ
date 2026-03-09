// primitives.typ: Reusable rendering primitives for CMU letterhead sections
//
// This module implements the visual rendering functions for all sections
// of a CMU letterhead letter following the official guidelines.

#import "config.typ": *
#import "utils.typ": *

// =============================================================================
// HEADER RENDERING
// =============================================================================
// Guidelines section 4: Header Composition
// - Wordmark (Top-Left)
// - Sender's Address Block (Immediately below wordmark)

#let render-header(
  wordmark,
  department: none,
  address: none,
  url: none,
) = {
  // CMU Wordmark
  // Guidelines 4.1: Width approx 2.25" to 2.5"
  if wordmark != none {
    let wordmark_content = box(width: 2.5in)[
      #set image(width: 100%)
      #wordmark
    ]

    // Hyperlink wordmark to URL if provided
    if url != none {
      link("https://" + url)[#wordmark_content]
    } else {
      wordmark_content
    }
    linebreak()
  }


  // Sender's Address Block
  // Guidelines 4.2: Flush Left

  // Department Name: Bold and darker
  if department != none {
    text(weight: "bold", fill: black)[#department]
    linebreak()
  }

  set text(fill: CMU_IRON_GRAY)

  // University Name: Regular
  [Carnegie Mellon University]
  linebreak()

  // Address Lines: Regular
  if address != none {
    let address_lines = if type(address) == "string" {
      (address,)
    } else {
      address
    }

    for line in address_lines {
      [#line]
      linebreak()
    }
  }

  // URL is now hyperlinked to wordmark, not displayed as text
}
