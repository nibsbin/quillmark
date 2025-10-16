# Acroform UTF-8 Encoding Investigation Summary

## Issue
When filling PDF forms with the quillmark-acroform backend, strings containing Unicode characters like smart quotes (`"None"`) are displayed incorrectly as box-drawing characters (`╜None╚`) in PDF viewers.

## Root Cause
The `acroform` library (version 0.0.10) writes UTF-8 bytes directly to PDF strings:

```rust
// acroform/src/api.rs, line 66
FieldValue::Text(s) => Primitive::String(PdfString::new(s.as_bytes().into()))
```

**Problem**: PDFs don't support UTF-8 encoding for text strings. According to PDF specification:
- Strings starting with BOM `FE FF` are interpreted as **UTF-16BE**
- All other strings use **PDFDocEncoding** (similar to Latin-1)

When UTF-8 bytes like `e2 80 9c` (smart quote `"`) are written without UTF-16BE BOM:
1. PDF viewers interpret each byte separately using PDFDocEncoding
2. `e2`, `80`, `9c` map to unrelated characters in the font
3. Result: box-drawing characters or garbage instead of the intended character

## Evidence
From PDF file analysis:
```
Plain ASCII:     /V (None)
Smart quotes:    /V <e2809c4e6f6e65e2809d>
                      ^^^^^^           ^^^^^^
                      UTF-8 "          UTF-8 "
```

The bytes are stored correctly but interpreted incorrectly by PDF viewers.

## Why Tests Pass
Tests use `to_string_lossy()` which:
1. Reads bytes from PDF
2. Interprets them as UTF-8 (falls back to UTF-8 if no UTF-16BE BOM)
3. Returns correct string

But PDF viewers follow PDF spec and use PDFDocEncoding, causing the display issue.

## Recommended Fix

Modify `acroform-rs` library to convert strings to UTF-16BE with BOM:

```rust
// In acroform/src/api.rs, FieldValue::to_primitive()
FieldValue::Text(s) => {
    // Convert to UTF-16BE with BOM for proper PDF encoding
    let mut utf16_bytes = vec![0xFE, 0xFF]; // UTF-16BE BOM
    for code_unit in s.encode_utf16() {
        utf16_bytes.push((code_unit >> 8) as u8);   // High byte
        utf16_bytes.push((code_unit & 0xFF) as u8);  // Low byte
    }
    Primitive::String(PdfString::new(utf16_bytes.into()))
}
```

This ensures:
- All Unicode characters work correctly in PDF viewers
- Compliance with PDF specification
- Consistent rendering across different viewers

## Fix Location
- Repository: https://github.com/nibsbin/acroform-rs
- File: `acroform/src/api.rs`
- Method: `FieldValue::to_primitive()`
- Line: ~66

## Demonstration
```
Input string:        "None"
UTF-8 bytes:         e2809c4e6f6e65e2809d
UTF-16BE (correct):  feff201c004e006f006e0065201d
                     ^^^^
                     BOM tells PDF viewer to use UTF-16BE
```

See `INVESTIGATION_NONE_ENCODING.md` for complete analysis with test results.
