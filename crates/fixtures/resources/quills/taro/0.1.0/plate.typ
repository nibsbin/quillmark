#import "@local/quillmark-helper:0.2.0" as qm

#set text(font: "Figtree")

// Advanced: Use show filter to color text
#show regex("(?i)taro"): it => text(fill: purple)[#it]

// Filters like `String` render to code mode automatically,
#underline(qm.ink(qm.data, "title"))

// When using filters in markup mode,
// add `#` before the template expression to enter code mode.
*Author: #qm.ink(qm.data, "author")*

*Favorite Ice Cream: #qm.ink(qm.data, "ice_cream")*


#qm.data.at("$body")

// Present each sub-document programatically
#for card in qm.data.at("$cards") {
  if card.at("$kind", default: none) == "quotes" [
    *#qm.ink(card, "author")*: _#card.at("$body") _
  ]
}


#if qm.data.picture != "" {
  image(qm.data.picture)
}
