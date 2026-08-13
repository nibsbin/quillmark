#import "@local/quillmark-helper:0.1.0": data
#import "@local/ttq-classic-resume:0.1.0": *

#show: resume

// A card's scalar fields arrive as value objects: `(…display)()` renders ink
// that carries the field's region, `.value` is the raw `str` a package needs
// for string work (here, `project-entry`'s `url.starts-with`).
#let ink(cell) = if cell == none { none } else { (cell.display)() }
#let text-of(cell) = if cell == none { none } else { cell.value }

#resume-header(
  name: data.name,
  contacts: data.contacts,
)

#for card in data.at("$cards") {
  if "title" in card and card.title.value != "" {
    section-header((card.title.display)())
  }

  let kind = card.at("$kind")
  if kind == "experience_section" {
    timeline-entry(
      heading-left: ink(card.at("heading_left", default: none)),
      heading-right: ink(card.at("heading_right", default: none)),
      subheading-left: ink(card.at("subheading_left", default: none)),
      subheading-right: ink(card.at("subheading_right", default: none)),
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
      name: (card.name.display)(),
      url: text-of(card.at("url", default: none)),
      body: card.at("$body", default: ""),
    )
  } else if kind == "certifications_section" {
    table(
      columns: 2,
      items: card.cells
    )
  }
}
