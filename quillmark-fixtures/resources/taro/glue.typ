
#set text(font:("Arial", "times", "sans-serif"))

**Author: {{ author }}**

**Favorite Ice Cream: {{ taro }}**

*{{title}}*

#{{ body | Content }}


{% if picture is defined %}
#image({{ picture | Asset }})
{% endif %}
