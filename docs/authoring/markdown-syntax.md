# Markdown Syntax

Quillmark supports a subset of CommonMark for document body content. The sections below cover what is supported; anything not listed (blockquotes, thematic breaks, raw HTML, math) is silently dropped.

## Your First Document

Start with a simple, realistic document body:

```markdown
# Project Update

## Wins this week

- Shipped v0.51.1
- Finalized onboarding copy

## Next steps

1. Prepare release notes
2. Review customer feedback
```

Use this as a base, then layer in the syntax patterns below.

## Headings

```markdown
# Heading 1
## Heading 2
### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6
```

## Text Formatting

```markdown
**Bold text**
*Italic text*
***Bold and italic***
~~Strikethrough~~
__Underline__
`Inline code`
```

## Lists

Unordered lists:

```markdown
- Item 1
- Item 2
  - Nested item
  - Another nested item
- Item 3
```

Ordered lists:

```markdown
1. First item
2. Second item
3. Third item
```

## Links

```markdown
[Link text](https://example.com)
```

## Images

```markdown
![Alt text](path/to/image.png)
```

The image source can be a path relative to the Quill bundle or an absolute path the backend can resolve. Alt text is currently ignored.

## Code Blocks

````markdown
```text
Any code or plain text content
can be placed inside fenced blocks.
```
````

## Tables

```markdown
| Name    | Role      |
| ------- | --------- |
| Alice   | Engineer  |
| Bob     | Designer  |
```

Column alignment is supported with `:` in the separator row.

## Line Breaks

Use `<br>` for a hard line break within a paragraph or table cell:

```markdown
First line<br>Second line
```

## Not Supported

The following are silently dropped and will not appear in rendered output:

- Blockquotes (`>`)
- Thematic breaks (`***`, `___`)
- Raw HTML (other than `<br>`)
- Math and footnotes

The `---` syntax is always reserved for metadata delimiters and cannot be used as a thematic break.

## Next Steps

- [YAML Frontmatter](yaml-frontmatter.md)
- [Cards](cards.md)
