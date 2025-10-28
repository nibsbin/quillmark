# USAF Memo

The USAF Memo quill produces typesetted United States Air Force Official Memorandums following Air Force formatting standards.

## Overview

The `usaf_memo` quill generates professional military memorandums with:

- Official letterhead with customizable title and caption
- Automatic paragraph numbering
- Proper memo headers (MEMORANDUM FOR, FROM, SUBJECT)
- Optional references, carbon copy, distribution, and attachments
- Signature blocks
- Classification markings
- Tag lines

## Example

Here's a simple example of a USAF memo:

```yaml
---
QUILL: usaf_memo
letterhead_title: DEPARTMENT OF THE AIR FORCE
letterhead_caption:
  - 123rd Fighter Wing
date: 2025-01-15
memo_for:
  - HQ/CC
memo_from:
  - 123 FW/CC
  - 123rd Fighter Wing
  - 123 Main Street
  - City ST 12345-6789
subject: Example Memorandum
signature_block:
  - JOHN A. DOE, Colonel, USAF
  - Commander
---

This is the first paragraph of the memo. The USAF memo quill automatically numbers top-level paragraphs.

This is the second paragraph. You can use standard Markdown formatting like **bold** and *italic* text.

- Use bullets for hierarchical paragraph nesting.
  - Up to five levels are supported.
  - This helps organize complex information.
```

## Required Fields

### `memo_for`
**Type:** Array of strings  
**Description:** List of recipient organization symbols

```yaml
memo_for:
  - ORG1/SYMBOL
  - ORG2/SYMBOL
```

### `memo_from`
**Type:** Array of strings  
**Description:** Sender information as an array with organization symbol, organization name, street address, and city/state/zip

```yaml
memo_from:
  - ORG/SYMBOL
  - Organization Name
  - 123 Street Avenue
  - City ST 12345-6789
```

### `subject`
**Type:** String  
**Description:** Subject line of the memorandum

```yaml
subject: Annual Training Requirements
```

### `signature_block`
**Type:** Array of strings  
**Description:** Signature block lines, typically including name, rank, service, and duty title

```yaml
signature_block:
  - FIRST M. LAST, Rank, USAF
  - Duty Title
```

## Optional Fields

### `letterhead_title`
**Type:** String  
**Default:** `"DEPARTMENT OF THE AIR FORCE"`  
**Description:** Title displayed in the letterhead

```yaml
letterhead_title: DEPARTMENT OF THE AIR FORCE
```

### `letterhead_caption`
**Type:** Array of strings  
**Default:** `["123 Raynor's Raiders"]`  
**Description:** Caption lines displayed below the letterhead title

```yaml
letterhead_caption:
  - 123rd Fighter Wing
  - Squadron Headquarters
```

### `date`
**Type:** String (YYYY-MM-DD format)  
**Default:** Current date  
**Description:** Date of the memorandum

```yaml
date: 2025-01-15
```

### `references`
**Type:** Array of strings  
**Default:** `[]`  
**Description:** Reference documents cited in the memo

```yaml
references:
  - AFI 33-360, Publications and Forms Management, 1 Dec 2015
  - AFMAN 33-326, Preparing Official Communications, 31 Jul 2019
```

### `cc`
**Type:** Array of strings  
**Default:** `[]`  
**Description:** Carbon copy recipients

```yaml
cc:
  - Rank and Name, ORG/SYMBOL
  - Another Recipient, ORG2/SYMBOL
```

### `distribution`
**Type:** Array of strings  
**Default:** `[]`  
**Description:** Distribution list for the memorandum

```yaml
distribution:
  - ORG1/SYMBOL
  - ORG2/SYMBOL
  - ORG3/SYMBOL
```

### `attachments`
**Type:** Array of strings  
**Default:** `[]`  
**Description:** List of attachments with descriptions and dates

```yaml
attachments:
  - Training Plan, 15 Jan 2025
  - Budget Spreadsheet, 12 Jan 2025
```

### `tag_line`
**Type:** String  
**Default:** `"Aim High"`  
**Description:** Tag line displayed at the bottom of the memo

```yaml
tag_line: Aim High
```

### `classification`
**Type:** String  
**Default:** `""`  
**Description:** Classification level of the memo, displayed in a banner

```yaml
classification: UNCLASSIFIED
# or
classification: SECRET//NOFORN
```

## Markdown Body Guidelines

The USAF memo quill has specific formatting conventions:

### Automatic Paragraph Numbering

Top-level paragraphs are automatically numbered (1., 2., 3., etc.). You don't need to add numbers manually.

```markdown
This is paragraph 1.

This is paragraph 2.

This is paragraph 3.
```

### Hierarchical Nesting

Use bullet points to create nested paragraph structures:

```markdown
This is the main paragraph.

- First sub-paragraph (a.)
  - Nested sub-paragraph (1)
    - Deeper nesting (a)
      - Even deeper (1)
        - Fifth level (a)
```

### Section Titles

**Do NOT use Markdown headings** (`#`, `##`, etc.). Instead, use bold text inline with the paragraph:

```markdown
**Background.** This paragraph provides background information on the topic.

**Discussion.** This paragraph discusses the main points.
```

## Tips and Best Practices

1. **Keep paragraphs concise** - Military writing emphasizes clarity and brevity
2. **Use proper formatting** - Bold section titles but keep them inline with paragraphs
3. **Follow numbering conventions** - Let the quill handle automatic numbering
4. **Include all required fields** - Missing required fields will cause rendering errors
5. **Use appropriate classification markings** - Ensure classification levels match content sensitivity
6. **Verify recipient addresses** - Double-check organization symbols and addresses
7. **Proofread signature blocks** - Ensure names, ranks, and titles are correct

## Related Resources

- [Creating Quills](../guides/creating-quills.md) - Learn how to create your own quills
- [Quill Markdown](../guides/quill-markdown.md) - Markdown syntax reference
- [Typst Backend](../guides/typst-backend.md) - Learn about the Typst rendering backend
