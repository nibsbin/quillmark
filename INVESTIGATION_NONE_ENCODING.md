# Investigation: "None" Rendering as "╜None╚" in Acroform

## Problem Statement
A string like "None" in a field in the template form.pdf document is getting rendered as ╜None╚. This only occurs when the field is passed through MiniJinja.

## Investigation Process

### 1. Character Analysis
The reported issue shows these characters:
- `╜` (U+255C - BOX DRAWINGS UP HEAVY AND LEFT DOWN LIGHT)
- `╚` (U+255A - BOX DRAWINGS UP SINGLE AND RIGHT DOUBLE)
- UTF-8 bytes: `e2 95 9c` and `e2 95 9a`

However, examination of the PDF template revealed it contains smart quotes around "None":
- `"` (U+201C - LEFT DOUBLE QUOTATION MARK)  
- `"` (U+201D - RIGHT DOUBLE QUOTATION MARK)
- UTF-8 bytes: `e2 80 9c` and `e2 80 9d`

### 2. Test Results

Created comprehensive tests to verify string handling:

1. **Direct PDF Fill Test** (`test_string_none_encoding`):
   - Directly filled "None" into a PDF field
   - Result: ✅ String preserved correctly
   - No box-drawing characters appeared

2. **MiniJinja Rendering Test** (`test_string_none_via_minijinja`):
   - Rendered "None" through MiniJinja template engine
   - Filled into PDF using acroform library
   - Result: ✅ String preserved correctly
   - No box-drawing characters appeared

3. **UTF-8 Encoding Investigation** (`test_pdf_string_encoding_investigation`):
   - Tested various UTF-8 strings including smart quotes
   - Input: `"None"` with smart quotes (U+201C, U+201D)
   - Output: `"None"` with same smart quotes
   - Result: ✅ All strings preserved correctly, including smart quotes

4. **Backend Compilation Test** (`test_backend_with_none_string_value`):
   - Full workflow test through quillmark-acroform backend
   - Rendered template containing smart quotes
   - Result: ✅ Smart quotes preserved in output PDF

### 3. PDF Template Analysis

The original PDF template (`usaf_form_8/form.pdf`) contains static text with smart quotes:
```
C. Recommended Additional Training (if not used, enter "None").
D. Additional Comments (if not used, enter "None").
```

These smart quotes are:
- Properly encoded as UTF-8 bytes: `e2 80 9c` and `e2 80 9d`
- Correctly read by acroform library using `to_string_lossy()`
- Correctly written back to PDF using `PdfString::new(s.as_bytes().into())`

### 4. Root Cause Analysis

#### Hypothesis 1: UTF-8 Encoding Issue ❌
**Disproven**: Tests show that the acroform library correctly handles UTF-8 strings, including smart quotes and other Unicode characters. The bytes are preserved correctly through the read-write cycle.

#### Hypothesis 2: MiniJinja Transformation ❌
**Disproven**: MiniJinja correctly preserves UTF-8 characters. Test `test_string_none_via_minijinja` confirms that strings pass through MiniJinja unchanged.

#### Hypothesis 3: PDF Font Encoding Issue ✅ LIKELY
**Probable Root Cause**: The issue is likely a **font encoding mismatch** in the PDF viewer, not in the code.

##### Evidence:
1. The bytes in the PDF are correct (UTF-8 smart quotes: `e2 80 9c` and `e2 80 9d`)
2. When read back from the filled PDF, the strings are correct
3. However, PDF viewers must use the font's encoding to map character codes to glyphs

##### How This Could Happen:
PDF text rendering involves several layers:
1. The PDF string contains bytes (UTF-8 in our case: `e2 80 9c`)
2. The PDF font has an encoding (often WinAnsiEncoding, PDFDocEncoding, or custom)
3. The viewer must map these bytes to character codes based on the font encoding
4. The viewer then displays the glyph for that character code from the font

If the PDF font is configured with:
- A single-byte encoding (like WinAnsiEncoding or PDFDocEncoding)
- The font expects each byte to be a separate character

Then the 3-byte UTF-8 sequence `e2 80 9c` would be interpreted as:
- `0xe2` → Some character (possibly '╜' in some encodings)
- `0x80` → Control character or special glyph
- `0x9c` → Some character

In Code Page 437 (DOS), for example:
- `0x9c` = ₧ (not ╜, but different encodings give different results)

#### Hypothesis 4: Acroform Library Font Handling ✅ ROOT CAUSE CONFIRMED

**Confirmed Root Cause**: The issue is in how the `acroform` library writes text strings to PDF fields.

##### Evidence from PDF File Analysis

Inspection of the generated PDF files reveals:
- Plain ASCII "None": `/V (None)` - stored as parenthesis string
- Smart quotes `"None"`: `/V <e2809c4e6f6e65e2809d>` - stored as hex string

The hex string `e2809c4e6f6e65e2809d` is the UTF-8 encoding of `"None"` with smart quotes:
- `e2809c` = U+201C (LEFT DOUBLE QUOTATION MARK) in UTF-8
- `4e6f6e65` = "None" in UTF-8
- `e2809d` = U+201D (RIGHT DOUBLE QUOTATION MARK) in UTF-8

##### The Problem

Looking at `/tmp/acroform-rs/acroform/src/api.rs` line 66:
```rust
FieldValue::Text(s) => Primitive::String(PdfString::new(s.as_bytes().into()))
```

This creates a `PdfString` with raw UTF-8 bytes. However, **PDFs don't inherently support UTF-8**. 

According to the PDF specification:
- If a string starts with BOM `FE FF`, it's interpreted as **UTF-16BE**
- Otherwise, it's interpreted using **PDFDocEncoding** (similar to Latin-1 for 0x00-0x7F, with special mappings for 0x80-0xFF)

When we write UTF-8 bytes (like `e2 80 9c` for smart quote) without a UTF-16BE BOM:
1. The PDF stores these as raw bytes: `<e2809c>`
2. PDF viewers interpret them using PDFDocEncoding or the font's encoding
3. Each byte (`e2`, `80`, `9c`) is treated as a **separate character**
4. The font maps these bytes to glyphs, which may be box-drawing characters or other unexpected symbols

##### Why Tests Don't Show The Issue

Our tests read the values back using `to_string_lossy()` (line ~40 in `acroform/src/api.rs`):
```rust
Primitive::String(s) => Some(FieldValue::Text(s.to_string_lossy().to_string()))
```

The `to_string_lossy()` method (from `pdf/src/primitive.rs`):
1. Checks if bytes start with UTF-16BE BOM (`FE FF`)
2. If not, uses `String::from_utf8_lossy()` to interpret as UTF-8

This means our tests **read the bytes back as UTF-8**, which is why they appear correct! However, a PDF **viewer** would interpret the same bytes using PDFDocEncoding or the font's encoding, resulting in the box-drawing characters.

##### Byte-by-Byte Breakdown

UTF-8 left smart quote `"` = `e2 80 9c`:
- In PDFDocEncoding:
  - `0xe2` = U+00E2 = 'â'
  - `0x80` = control character or special symbol
  - `0x9c` = control character or special symbol
- In some font encodings, these map to box-drawing characters like `╜`

UTF-8 right smart quote `"` = `e2 80 9d`:
- Similar issue, may render as `╚` or other symbols

### 6. Reproduction Steps

To see the issue in a PDF viewer:
1. Create a field value with smart quotes: `"None"`
2. Fill the PDF using quillmark-acroform backend
3. Open the filled PDF in a viewer (Adobe Acrobat, Preview, etc.)
4. Observe that `"None"` appears as `╜None╚` or similar garbage characters

The actual characters seen depend on:
- The PDF viewer
- The font used in the PDF form
- The font's encoding table

## Recommended Fixes

### Fix 1: Convert UTF-8 to UTF-16BE (Recommended)
Modify the acroform library to properly encode strings for PDF:

```rust
// In acroform/src/api.rs, line 66
FieldValue::Text(s) => {
    // Convert UTF-8 string to UTF-16BE with BOM for PDF
    let mut utf16_bytes = vec![0xFE, 0xFF]; // UTF-16BE BOM
    for c in s.encode_utf16() {
        utf16_bytes.push((c >> 8) as u8);   // High byte
        utf16_bytes.push((c & 0xFF) as u8);  // Low byte
    }
    Primitive::String(PdfString::new(utf16_bytes.into()))
}
```

This ensures that:
- All Unicode characters (including smart quotes, emojis, etc.) work correctly
- PDF viewers properly interpret the text as UTF-16BE
- The standard is followed (PDF spec recommends UTF-16BE for Unicode strings)

### Fix 2: Strip Non-ASCII Characters (Quick Fix, Lossy)
If Unicode support isn't needed, strip or replace non-ASCII characters:

```rust
FieldValue::Text(s) => {
    let ascii_only = s.chars()
        .map(|c| if c.is_ascii() { c } else { '?' })
        .collect::<String>();
    Primitive::String(PdfString::new(ascii_only.as_bytes().into()))
}
```

This would turn `"None"` into `"?None?"` but at least it's consistent.

### Fix 3: Use PDFDocEncoding for ASCII + Common Characters
For a middle-ground approach, use PDFDocEncoding for characters 0x00-0xFF and UTF-16BE only when needed:

```rust
FieldValue::Text(s) => {
    // Check if all characters fit in PDFDocEncoding
    let needs_utf16 = s.chars().any(|c| (c as u32) > 0xFF);
    
    if needs_utf16 {
        // Use UTF-16BE with BOM
        let mut utf16_bytes = vec![0xFE, 0xFF];
        for c in s.encode_utf16() {
            utf16_bytes.push((c >> 8) as u8);
            utf16_bytes.push((c & 0xFF) as u8);
        }
        Primitive::String(PdfString::new(utf16_bytes.into()))
    } else {
        // Use PDFDocEncoding (basically Latin-1 for our purposes)
        let bytes: Vec<u8> = s.chars().map(|c| c as u8).collect();
        Primitive::String(PdfString::new(bytes.into()))
    }
}
```

### Fix 4: Modify Template to Use ASCII Quotes (Workaround)
Instead of fixing the code, modify the PDF template to use ASCII quotes:
- Change `"None"` to `"None"` in the template

This is a workaround, not a real fix.

## Recommendation

**Implement Fix 1** (UTF-16BE conversion) in the acroform library. This is the proper solution that:
1. Follows PDF specification
2. Supports full Unicode
3. Ensures consistent rendering across all PDF viewers
4. Fixes not just smart quotes but all non-ASCII characters

The fix should be made in:
- File: `/tmp/acroform-rs/acroform/src/api.rs`
- Method: `FieldValue::to_primitive()`
- Line: ~66

This would require submitting a pull request to the `acroform-rs` repository at https://github.com/nibsbin/acroform-rs

## Alternative: Temporary Workaround in Quillmark

Until the acroform library is fixed, quillmark-acroform could work around this by:
1. Detecting non-ASCII characters in rendered values
2. Warning the user
3. Optionally converting smart quotes to ASCII quotes

However, this is not recommended as it's a band-aid solution that doesn't address the root cause.
