---
#===Essential===
QUILL: usaf_memo
letterhead_title: DEPARTMENT OF THE AIR FORCE
letterhead_caption:
  - YOUR SQUADRON HERE
date: 2504-10-05
memo_for:
  - 1st ORG/SYMBOL
  #- 2nd ORG/SYMBOL
memo_from:
  - ORG/SYMBOL
  - Organization Name
  - 123 Street Ave
  - City ST 12345-6789
subject: Subject of the Memorandum

#===Optional===
references:
  - Document, YYYY MMM DD, Description/Title
cc:
  - Rank Name, ORG/SYMBOL
distribution:
  - 1st ORG/SYMBOL
  #- 2nd ORG/SYMBOL
attachments:
  - Attachment_Name, YYYY MMM DD
signature_block:
  - FIRST M. LAST, Rank, USAF
  #- Optional Duty Title
tag_line: Aim High
classification: SECRET//FICTIONAL
---

The `usaf_memo` Quill package takes care of many formatting details for AFH 33-337 official memorandums to let you focus on the content.

**Numbering** Top-level paragraphs like this one are automatically numbered. NEVER manually number your paragraphs.

- Use bullets for hierarchical paragraph nesting. These are automatically numbered as well.
  - Up to five nested levels are supported

**Headings** Do NOT use markdown headings. If you want to title paragraphs/sections, use bold text in-line with the paragraph or nest paragraphs underneath.

Do not include a complimentary close (e.g. "Respectfully,") in official memorandums.