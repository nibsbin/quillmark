#import "@local/quillmark-helper:0.1.0": data, display-of, value-of
#import "@local/ttq-classic-resume:0.1.0": *

#show: resume

#resume-header(
  name: data.name,
  contacts: data.contacts,
)

#for card in data.at("$cards") {
  if "title" in card and card.title.value != "" {
    section-header(display-of(card.title))
  }

  let kind = card.at("$kind")
  if kind == "experience_section" {
    timeline-entry(
      heading-left: display-of(card.at("heading_left", default: none)),
      heading-right: display-of(card.at("heading_right", default: none)),
      subheading-left: display-of(card.at("subheading_left", default: none)),
      subheading-right: display-of(card.at("subheading_right", default: none)),
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
      name: display-of(card.name),
      url: value-of(card.at("url", default: none)),
      body: card.at("$body", default: ""),
    )
  } else if kind == "certifications_section" {
    table(
      columns: 2,
      items: card.cells
    )
  }
}
