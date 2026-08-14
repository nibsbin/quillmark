"""Tests for the value and obligation axes of the schema surface.

- No `default:` -> the field is **obliged**: the blueprint renders the
  ``!must_fill`` marker. A marker left in the document is non-fatal: validate
  reports a ``validation::must_fill`` warning and render still succeeds (the
  field blank-fills or uses its suggested value).
- With `default:` -> the blueprint renders the default value with a type-only
  ``# <type>`` annotation; the field is unobliged and the default is used when
  absent.
"""

from quillmark import Document, OutputFormat, Quill


QUILL_YAML_CONTENT = """quill:
  name: py_schema_smoke
  version: "1.0"
  backend: typst
  description: Python schema/blueprint smoke test

typst:
  plate_file: plate.typ

main:
  fields:
    title:
      description: Document title
      type: string
    status:
      description: Document status
      type: string
      default: draft
    count:
      type: integer
"""

PLATE_TYP = "Title: {{ title }} / Status: {{ status }} / Count: {{ count }}"


def make_quill(tmp_path, yaml_content=QUILL_YAML_CONTENT, plate=PLATE_TYP):
    quill_dir = tmp_path / "quill"
    quill_dir.mkdir()
    (quill_dir / "Quill.yaml").write_text(yaml_content)
    (quill_dir / "plate.typ").write_text(plate)
    return Quill.from_path(str(quill_dir))


# ---------------------------------------------------------------------------
# Schema surface: `required:` is gone; cells are inferred from `default:`.
# ---------------------------------------------------------------------------

def test_schema_has_no_required_key(tmp_path):
    """The schema dict never carries a `required:` key on a field.

    Cell is inferred from the presence/absence of `default:`.
    """
    quill = make_quill(tmp_path)
    schema = quill.schema

    fields = schema["main"]["fields"]
    for name, field in fields.items():
        assert "required" not in field, (
            f"field {name!r} unexpectedly carries `required`; "
            "the schema axis is now `default`-driven"
        )


def test_schema_reports_declared_default(tmp_path):
    """A defaulted field carries the `default` key; a defaultless one does not."""
    quill = make_quill(tmp_path)
    fields = quill.schema["main"]["fields"]

    # Defaultless: no `default`
    assert "default" not in fields["title"], (
        "title is defaultless: no default should be reported"
    )
    assert "default" not in fields["count"], (
        "count is defaultless: no default should be reported"
    )

    # Defaulted: schema carries `default`
    assert fields["status"]["default"] == "draft"


# ---------------------------------------------------------------------------
# Blueprint surface: annotations and markers
# ---------------------------------------------------------------------------

def test_blueprint_must_fill_marker(tmp_path):
    quill = make_quill(tmp_path)
    bp = quill.blueprint

    # Obliged fields carry the marker
    assert "title: !must_fill" in bp, (
        f"expected `title: !must_fill` in blueprint; got:\n{bp}"
    )
    assert "count: !must_fill" in bp, (
        f"expected `count: !must_fill` in blueprint; got:\n{bp}"
    )


def test_blueprint_defaulted_value(tmp_path):
    """A defaulted cell renders the concrete default with a type-only annotation."""
    quill = make_quill(tmp_path)
    bp = quill.blueprint

    # The defaulted `status` field renders its default value with a type-only
    # annotation. The exact format is `status: draft # string`.
    assert "status: draft" in bp, f"expected default in blueprint; got:\n{bp}"
    # Shippability is the value cell: the `; delete-ok` tag is gone entirely.
    assert "delete-ok" not in bp, (
        f"expected no `; delete-ok` tag in blueprint; got:\n{bp}"
    )


def test_blueprint_no_legacy_required_optional_tags(tmp_path):
    quill = make_quill(tmp_path)
    bp = quill.blueprint

    # Role tags are never emitted.
    assert "; required" not in bp, (
        f"`; required` tag must not appear in blueprint:\n{bp}"
    )
    assert "; optional" not in bp, (
        f"`; optional` tag must not appear in blueprint:\n{bp}"
    )


# ---------------------------------------------------------------------------
# Validation surface: new diagnostic codes
# ---------------------------------------------------------------------------

def test_render_tolerates_must_fill_marker(engine, tmp_path):
    """A ``!must_fill`` marker left in the document is non-fatal.

    Render still succeeds (the field blank-fills or uses its suggested value),
    and ``quill.validate`` surfaces a non-fatal ``validation::must_fill``
    warning for the marker.
    """
    quill = make_quill(tmp_path)
    md = (
        "~~~card-yaml\n"
        "$quill: py_schema_smoke\n"
        "$kind: main\n"
        "title: !must_fill\n"       # marker left in place
        "count: 1\n"
        "~~~\n"
    )
    doc = Document.from_markdown(md)

    # The marker does not gate render.
    result = engine.render(quill, doc, OutputFormat.PDF)
    assert len(result.artifacts) > 0

    # validate surfaces a non-fatal warning for the marker.
    diags = quill.validate(doc)
    fill = [d for d in diags if d.get("code") == "validation::must_fill"]
    assert any(d.get("path") == "main.title" for d in fill), (
        f"expected a validation::must_fill warning on `main.title`; got: {diags}"
    )
    assert all(d.get("severity") == "warning" for d in fill), (
        f"validation::must_fill must be a non-fatal warning; got: {fill}"
    )

    codes = [d.get("code") for d in diags]
    assert "validation::field_absent" not in codes, (
        f"field_absent is removed and must not be surfaced; got: {codes}"
    )

    # `trigger` is what a consumer routes on, so pin it crossing the boundary.
    by_path = {d.get("path"): d.get("args", {}).get("trigger") for d in fill}
    assert by_path.get("main.title") == "marker", f"got: {fill}"
    assert by_path.get("main.count") is None, "an authored cell is discharged"


def test_render_succeeds_when_obliged_fields_supplied(engine, tmp_path):
    """Filling every obliged field renders successfully: defaulted fields
    fall back to their declared default."""
    quill = make_quill(tmp_path)
    md = (
        "~~~card-yaml\n"
        "$quill: py_schema_smoke\n"
        "$kind: main\n"
        "title: Hello\n"
        "count: 7\n"
        # status omitted → falls back to its default "draft"
        "~~~\n"
    )
    doc = Document.from_markdown(md)

    result = engine.render(quill, doc, OutputFormat.PDF)
    assert len(result.artifacts) > 0
    assert result.artifacts[0].format == OutputFormat.PDF
