# Markdown Syntax

Quillmark supports standard CommonMark syntax for document content.

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

## Code Blocks

````markdown
```text
Any code or plain text content
can be placed inside fenced blocks.
```
````

## Blockquotes

```markdown
> This is a blockquote
> It can span multiple lines
```

## Horizontal Rules

Use either:

```markdown
***
```

or:

```markdown
___
```

The `---` syntax is reserved for metadata delimiters, so it cannot be used as a horizontal rule in Quillmark documents.

## Next Steps

- [YAML Frontmatter](yaml-frontmatter.md)
- [Cards](cards.md)
