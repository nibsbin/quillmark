"""The portable values shape: ``quill.project`` and ``writer.set_values``.

The shape's semantics are core's (``core/src/quill/values.rs``). At this
boundary the questions are narrower: does the shape arrive as a plain dict,
does a content leaf one level in arrive as text, does the write reach core, and
do refusals arrive as diagnostics carrying their own ``path``.
"""

import pytest

from quillmark import Quill
from conftest import raises_edit_code


QUILL_YAML = """quill:
  name: py_values
  version: "1.0"
  backend: typst
  description: Portable values coverage

typst:
  plate_file: plate.typ

main:
  fields:
    subject:
      type: richtext
      inline: true
    note:
      type: plaintext
    qty:
      type: integer
      default: 1
    paragraphs:
      type: array
      items:
        type: richtext

card_kinds:
  line_item:
    fields:
      desc:
        type: richtext
        inline: true
"""

MD = """~~~card-yaml
$quill: py_values@1.0
$kind: main
$ext:
  app:
    k: 1
subject: Hello **world**
note: a *literal* line
paragraphs:
  - Para **one**
~~~

Body prose.

~~~card-yaml
$kind: line_item
desc: Widget __A__
~~~
Item note.
"""


def make_quill(tmp_path):
    quill_dir = tmp_path / "quill"
    quill_dir.mkdir()
    (quill_dir / "Quill.yaml").write_text(QUILL_YAML)
    (quill_dir / "plate.typ").write_text("#set page(width: 100pt, height: 100pt)\n")
    return Quill.from_path(str(quill_dir))


def test_project_reads_content_leaves_as_text_at_every_depth(tmp_path):
    quill = make_quill(tmp_path)
    values = quill.project(quill.parse(MD))

    assert values["fields"]["subject"] == "Hello **world**"
    assert values["fields"]["note"] == "a *literal* line"
    assert values["fields"]["paragraphs"] == ["Para **one**"]
    assert values["body"] == "Body prose."
    assert values["ext"] == {"app": {"k": 1}}

    (card,) = values["cards"]
    assert card["kind"] == "line_item"
    assert card["fields"]["desc"] == "Widget **A**"
    assert card["body"] == "Item note."


def test_project_is_sparse(tmp_path):
    quill = make_quill(tmp_path)
    values = quill.project(quill.parse(MD))
    assert "qty" not in values["fields"], "an absent field is not its `default`"


def test_writing_back_an_unedited_projection_changes_no_bytes(tmp_path):
    quill = make_quill(tmp_path)
    doc = quill.parse(MD)
    before = doc.to_stored()
    quill.writer(doc).set_values(quill.project(doc))
    assert doc.to_stored() == before


def test_set_values_writes_an_edit_and_removes_an_omitted_field(tmp_path):
    quill = make_quill(tmp_path)
    doc = quill.parse(MD)
    values = quill.project(doc)
    values["fields"]["subject"] = "Goodbye *world*"
    del values["fields"]["note"]
    quill.writer(doc).set_values(values)

    after = quill.project(doc)
    assert after["fields"]["subject"] == "Goodbye *world*"
    assert "note" not in after["fields"]


def test_an_undeclared_name_is_refused_under_its_own_path(tmp_path):
    quill = make_quill(tmp_path)
    doc = quill.parse(MD)
    before = doc.to_stored()
    with raises_edit_code("edit::unknown_field") as excinfo:
        quill.writer(doc).set_values({"fields": {"nope": "x"}})
    assert excinfo.value.diagnostics[0].path == "main.nope"
    assert doc.to_stored() == before, "an all-or-nothing batch writes nothing"


def test_a_malformed_shape_raises_value_error(tmp_path):
    quill = make_quill(tmp_path)
    doc = quill.parse(MD)
    with pytest.raises(ValueError):
        quill.writer(doc).set_values({"feilds": {}})
