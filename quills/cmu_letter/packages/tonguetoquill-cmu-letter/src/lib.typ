// lib.typ: Public API for CMU letterhead template
//
// This module provides a composable API for creating Carnegie Mellon University
// letters that comply with the official letterhead standards.
//
// CMU Letterhead Guidelines specify:
// - "Lefthead" layout with 2.25" left margin for branding column
// - Open Sans font at 10-11pt for body text
// - Block style paragraphs with no indentation
// - Flush left / ragged right alignment
// - Carnegie Red (#C41230) restricted to wordmark only
// - Iron Gray (#6D6E71) for secondary text
//
// Key features:
// - Composable show rules for frontmatter and mainmatter
// - Function-based backmatter for closing section
// - CMU-compliant "Lefthead" layout with branding column
// - Proper typography and spacing throughout
//
// Basic usage:
//
// #import "@preview/tonguetoquill-cmu-letter:0.1.0": frontmatter, mainmatter, backmatter
//
// #show: frontmatter.with(
//   wordmark: image("assets/cmu-wordmark.svg"),
//   department: "School of Computer Science",
//   address: ("5000 Forbes Avenue", "Pittsburgh, PA 15213"),
//   url: "cs.cmu.edu",
//   date: datetime.today(),
//   recipient: ("Dr. Jane Smith", "Department of Engineering", "University of Example", "123 Main Street", "City, ST 12345"),
// )
//
// #show: mainmatter
//
// Dear Dr. Smith,
//
// Your letter body content here.
//
// #backmatter(
//   closing: "Sincerely,",
//   sender_name: "John Doe",
//   sender_title: "Associate Professor",
// )

#import "frontmatter.typ": frontmatter
#import "mainmatter.typ": mainmatter
#import "backmatter.typ": backmatter
