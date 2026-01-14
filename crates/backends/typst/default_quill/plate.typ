#import "@local/quillmark-helper:0.1.0": data, eval-markup

// Display document metadata
#data

#line(length: 100%)

// Render body content
#eval-markup(data.at("body", default: ""))
