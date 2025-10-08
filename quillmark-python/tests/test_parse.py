"""Tests for ParsedDocument."""

import pytest

from quillmark import ParsedDocument, ParseError


def test_parse_markdown(simple_markdown):
    """Test parsing markdown with frontmatter."""
    parsed = ParsedDocument.from_markdown(simple_markdown)
    assert parsed.get_field("title") == "Test Document"
    assert "Hello World" in parsed.body()


def test_parse_invalid_yaml():
    """Test parsing invalid YAML frontmatter."""
    invalid_md = """---
title: [unclosed bracket
---

Content
"""
    with pytest.raises(ParseError):
        ParsedDocument.from_markdown(invalid_md)


def test_fields_access(simple_markdown):
    """Test accessing all fields."""
    parsed = ParsedDocument.from_markdown(simple_markdown)
    fields = parsed.fields()
    assert "title" in fields
    assert fields["title"] == "Test Document"
    assert "body" in fields
