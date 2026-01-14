#import "@preview/ttq-classic-resume:0.1.0": *

#show: resume

#resume_header(
  name: {{ name | String }},
  contacts: {{ contacts | Lines }},
)

{% for card in CARDS %}
  {% if card.title %}
    #section_header({{ card.title | String }})
  {% endif %}

  {% if card.CARD == "experience_section" %}
    #timeline_entry(
      headingLeft: {{ card.headingLeft | String }},
      headingRight: {{ card.headingRight | String }},
      subheadingLeft: {{ card.subheadingLeft | String }},
      subheadingRight: {{ card.subheadingRight | String }},
      body: {{ card.BODY | Content }},
    )
  {% elif card.CARD == "skills_section" %}
    #table(
      columns: 2,
      items: (
        {% for item in card.cells %}
          (category: {{ item.category | String }}, text: {{ item.skills | String }}),
        {% endfor %}
      )
    )
  {% elif card.CARD == "projects_section" %}
    #project_entry(
      name: {{ card.name | String }},
      url: {{ card.url | String }},
      body: {{ card.BODY | Content }},
    )
  {% elif card.CARD == "certifications_section" %}
    #table(
      columns: 2,
      items: {{ card.cells | Lines }}
    )
  {% endif %}
{% endfor %}
