---
#===Essential===
QUILL: usaf_memo
letterhead_title: DEPARTMENT OF THE AIR FORCE  # Only change for Joint Commands or DoD Agencies
letterhead_caption:
  - HEADQUARTERS YOUR UNIT NAME
date: 2504-10-05  # YYYY-MM-DD format; leave blank to use today's date
memo_for:
  - ORG/SYMBOL  # Organization/office symbol in UPPERCASE
  #- ORG/SYMBOL (LT COL JANE DOE)  # To address a specific person, add rank and name in parentheses
  #- DISTRIBUTION  # For numerous recipients, use 'DISTRIBUTION' and list them below
memo_from:
  - ORG/SYMBOL  # If recipients are on same installation, use only office symbol
  - Organization Name  # For recipients on other installations, include full mailing address
  - 123 Street Ave  # to enable return correspondence
  - City ST 12345-6789
subject: Subject of the Memorandum  # Be brief and clear; capitalize first letter of each word except articles, prepositions, and conjunctions

#===Optional===
references:
  - AFM 33-326, 31 July 2019, Preparing Official Communications  # Cite by organization, type, date, and title
cc:
  - ORG/SYMBOL  # List office symbols of recipients to receive copies
distribution:
  - 1st ORG/SYMBOL  # Used when "DISTRIBUTION" is specified in memo_for
  #- 2nd ORG/SYMBOL
attachments:
  - Attachment description, YYYY MMM DD  # List in order mentioned; briefly describe each (do not use "as stated" or abbreviations)
signature_block:
  - FIRST M. LAST, Rank, USAF  # Line 1: Name in UPPERCASE as signed, grade, and service (spell out "Colonel" and general officer ranks)
  - Duty Title  # Line 2: Duty title
tag_line: Aim High
classification: SECRET//FICTIONAL  # Follow AFI 31-401 and applicable DoD guidance; leave blank for unclassified
---

The `usaf_memo` Quill package takes care of many formatting details for AFH 33-337 official memorandums to let you focus on the content.

**Numbering** Top-level paragraphs like this one are automatically numbered. NEVER manually number your paragraphs.

- Use bullets for hierarchical paragraph nesting. These are automatically numbered as well.
  - Up to five nested levels are supported

**Headings** Do NOT use markdown headings. If you want to title paragraphs/sections, use bold text in-line with the paragraph or nest paragraphs underneath.

Do not include a complimentary close (e.g. "Respectfully,") in official memorandums.