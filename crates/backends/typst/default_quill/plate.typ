#import "@local/quillmark-helper:0.1.0": data, content

// Display document metadata
#data

#line(length: 100%)

// Render body content
#content(data.at("body", default: ""))
