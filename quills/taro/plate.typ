
#set text(font:("Figtree"))

// Advanced: Use show filter to color text
#show regex("(?i)taro"): it => text(fill: purple)[#it]

// Filters like `String` render to code mode automatically,
#underline({{title | String}})

// When using filters in markup mode,
// add `#` before the template expression to enter code mode.
*Author: #{{ author | String }}*

*Favorite Ice Cream: #{{ ice_cream | String}}*__


#{{ BODY | Content }}

// Present each sub-document programatically
{% for card in CARDS %}
{% if card.CARD == "quotes" %}
*#{{ card.author | String }}*: _#{{ card.BODY | Content }}_
{% endif %}
{% endfor %}


// Include an image with a dynamic asset
{% if picture is defined %}
#image({{ picture | Asset }})
{% endif %}