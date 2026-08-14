#import "@local/quillmark-helper:0.1.0": data
#import "@local/ttq-classic-resume:0.1.0": *

#let section-kinds = (
  "experience_section",
  "skills_section",
  "projects_section",
  "certifications_section",
)

#show: resume

#resume-header(
  name: data.name,
  contacts: data.contacts,
)

#for card in data.at("$cards") {
  // `$kind` is document-defined, so a card block with no `$kind:` line carries
  // none: read it totally. A kindless or unknown-kind card carries its authored
  // fields verbatim with no schema fill, so nothing below may assume a field.
  let kind = card.at("$kind", default: none)
  if kind not in section-kinds { continue }

  // Every kind here declares `title`, so it always arrives; only its value
  // needs guarding.
  if card.title != "" {
    section-header(card.title)
  }

  if kind == "experience_section" {
    timeline-entry(
      heading-left: card.heading_left,
      heading-right: card.heading_right,
      subheading-left: card.subheading_left,
      subheading-right: card.subheading_right,
      body: card.at("$body", default: ""),
    )
  } else if kind == "skills_section" {
    table(
      columns: 2,
      items: card.cells.map(item => (
        category: item.category,
        text: item.skills,
      ))
    )
  } else if kind == "projects_section" {
    project-entry(
      name: card.name,
      url: card.url,
      body: card.at("$body", default: ""),
    )
  } else if kind == "certifications_section" {
    table(
      columns: 2,
      items: card.cells
    )
  }
}
