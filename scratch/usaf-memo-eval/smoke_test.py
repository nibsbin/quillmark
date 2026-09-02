#!/usr/bin/env python3
"""Smoke-test get_blueprint / create_document against usaf_memo@0.3.0."""

from __future__ import annotations

import json
import sys

from quillmark_tools import create_document, get_blueprint

GOOD = """~~~
$quill: usaf_memo@0.3.0
$kind: main
memo_for:
  - 88 CS/CC
subject: Smoke Test
signature_block:
  - JANE A. DOE, Col, USAF
  - Commander
~~~

This is a one-paragraph smoke test.
"""

COLON = """~~~
$quill: usaf_memo@0.3.0
$kind: main
memo_for:
  - 88 CS/CC
subject: Request: Additional Manning
signature_block:
  - JANE A. DOE, Col, USAF
  - Commander
~~~

Body.
"""


def main() -> None:
    bp = get_blueprint()
    assert bp["quill_ref"] == "usaf_memo@0.3.0", bp["quill_ref"]
    assert "create_document" in bp["instruction"]
    assert "!must_fill" in bp["blueprint"]
    ok = create_document(GOOD)
    assert ok["ok"] is True, json.dumps(ok, indent=2)
    bad = create_document(COLON)
    assert bad["ok"] is False
    assert bad["stage"] == "parse"
    leftover = create_document(bp["blueprint"])
    assert leftover["ok"] is False
    codes = {d["code"] for d in leftover["diagnostics"]}
    assert "validation::must_fill" in codes
    print("smoke ok")


if __name__ == "__main__":
    sys.exit(main())
