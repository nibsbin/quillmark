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
  //main-color: "4DA6FF", //set the main color
  logo: image("logo.png"), //set the logo
) 

#{{ body | Body }}