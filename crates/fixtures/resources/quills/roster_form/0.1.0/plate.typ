#import "@local/quillmark-helper:0.2.0" as qm

// `roster` is the vocabulary the page prints: every member in roster order,
// held or not, then each item the document adds. Printing the roster as the
// schema lists it is the type's purpose, so the plate asserts the order on
// every render.
#let rows = qm.roster(qm.data, "qualifications")
#assert.eq(
  rows.slice(0, 5).map(row => row.id),
  ("sq_cc_candidate", "flight_cc", "dodin_ops", "dco", "cyber_200"),
  message: "the roster reached the plate as " + repr(rows.map(row => row.id)),
)

#underline(qm.ink(qm.data, "title"))

// Each row claims its member's address, so an unticked box is clickable too. A
// held row's columns read without a guard; an added item's title is a cell,
// so its ink keeps its own address inside the claim. A roster member's title
// is the roster's, and `title` is no address on it.
#for row in rows {
  let title = if row.held and "title" in row.value { qm.ink(row.value, "title") } else { row.title }
  qm.region(row.path)[#(if row.held { "[x]" } else { "[ ]" }) #title]
  if row.held [ #qm.ink(row.value, "detail")]
  linebreak()
}

#for line in qm.ink(qm.data, "endorsements") [#line \ ]

#for tour in qm.data.tours [#qm.ink(tour, "unit") — #qm.ink(tour, "duration") \ ]
