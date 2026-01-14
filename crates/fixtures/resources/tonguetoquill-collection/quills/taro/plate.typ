#import "@local/quillmark-helper:0.1.0": data, content

#set text(font:("Figtree"))

// Advanced: Use show filter to color text
#show regex("(?i)taro"): it => text(fill: purple)[#it]

// Access title field directly
#underline(data.title)

// Access author and ice_cream fields
*Author: #data.author*

*Favorite Ice Cream: #data.ice_cream*

#content(data.BODY)

// Present each sub-document programatically
#for card in data.at("CARDS", default: ()) {
  if card.CARD == "quotes" {
    [*#card.author*: _#content(card.BODY)_]
  }
}

// Include an image with a dynamic asset
#if "picture" in data {
  image(data.picture)
}
