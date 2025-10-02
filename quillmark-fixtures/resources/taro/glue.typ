
#set text(font:("Arial", "times", "sans-serif"))

**Author: {{ author }}**

**Favorite Ice Cream: {{ taro }}**

*{{title}}*

{% if picture is defined %}
#image({{ picture | Asset }})
{% endif %}

#{{ body | Content }}
