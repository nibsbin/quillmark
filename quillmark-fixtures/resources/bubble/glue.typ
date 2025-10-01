#import "@preview/bubble:0.2.2": *

#show: bubble.with(
  title: {{ title | String }},
  subtitle: {{ subtitle | String }},
  author: {{ author | String }},
  affiliation: {{ affiliation | String }},
  date: datetime.today().display(),
  year: {{ year | String }},
  class: {{ class | String }},
  other: {{ other | Lines }},
  main-color: {{ main_color | String }},
  logo: image("assets/logo.png"),
) 

#{{ body | Body }}