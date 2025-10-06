
#set text(font:("Arial", "times", "sans-serif"))

// When using filters in markup mode,
// add `#` before the template expression to enter code mode.
**Author: #{{ author | String }}**

**Favorite Ice Cream: #{{ ice_cream | String}}**

*#{{title | String}}*

#{{ body | Content }}

// Present each sub-document
{% for quote in quotes %}
*#{{ quote.author | String }}*: _#{{ quote.body | Content }}_
{% endfor %}


// Include an image with a dynamic asset
{% if picture is defined %}
#image({{ picture | Asset }})
{% endif %}