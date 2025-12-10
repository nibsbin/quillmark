// mainmatter.typ: Mainmatter show rule for CMU letter body
//
// This module implements the body text section of a CMU letter per guidelines:
// - Block style paragraphs (no indentation)
// - Single line spacing (1.0 to 1.15)
// - Default paragraph spacing
// - Flush left / ragged right alignment

#import "config.typ": *
#import "utils.typ": *

/// Mainmatter show rule for CMU letter body content.
///
/// Guidelines section 3.1:
/// - Format: Block Style
/// - Indentation: None
/// - Spacing: Single line spacing (1.0 to 1.15)
/// - Separation: Default paragraph spacing
///
/// - content (content): The body content to render
/// -> content
#let mainmatter(it) = {
  blank-lines(3)


  // Set list styling with bigger bullet aligned to the left
  set list(
    indent: 0em,
    body-indent: 1em,
    marker: text(size: 1.2em)[•]
  )
  set enum(indent: 0em, body-indent: 1em)
  
  block()[
    #set par(
      first-line-indent: 0pt,
      justify: false,
      spacing: 2em
    )
    #it
  ]
}
