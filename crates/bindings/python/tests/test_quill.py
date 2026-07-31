"""Tests for quill loading."""
import pytest
from quillmark import Quillmark, Quill, Document, OutputFormat, QuillmarkError


def test_quill_from_path(taro_quill_dir):
    """Quill.from_path loads engine-free, validated config data."""
    quill = Quill.from_path(str(taro_quill_dir))
    assert quill is not None
    assert quill.metadata["name"] == "taro"
    assert quill.backend_id == "typst"


def test_quill_from_path_bad_backend_loads_then_fails_at_render(tmp_path):
    """An unregistered backend crosses as a plain `backend_id` string on a
    loaded quill and as a raised QuillmarkError at render. Which engine calls
    resolve the backend is core's contract
    (`crates/quillmark/tests/quill_engine_test.rs`)."""
    quill_dir = tmp_path / "test_quill"
    quill_dir.mkdir()
    (quill_dir / "Quill.yaml").write_text(
        'quill:\n  name: "test"\n  version: "1.0"\n  backend: "nonexistent"\n  description: "Test"\n'
    )

    # Engine-free load succeeds: the config is valid, the backend is not resolved.
    quill = Quill.from_path(str(quill_dir))
    assert quill.backend_id == "nonexistent"
    assert quill.metadata["backend"] == "nonexistent"

    doc = Document.from_markdown(
        "~~~card-yaml\n$quill: test\n$kind: main\n~~~\n\nBody.\n"
    )
    with pytest.raises(QuillmarkError):
        Quillmark().render(quill, doc, OutputFormat.PDF)


# One case per selector arm against taro's 0.1.0: the table a resolver would
# otherwise re-derive.
SATISFIES_CASES = [
    ("taro", True),
    ("taro@latest", True),
    ("taro@0", True),
    ("taro@1", False),
    ("taro@0.1", True),
    ("taro@0.2", False),
    ("taro@0.1.0", True),
    ("taro@0.1.1", False),
    ("other_quill", False),
    ("other_quill@0.1.0", False),
]


@pytest.mark.parametrize("quill_ref,want", SATISFIES_CASES)
def test_quill_satisfies(taro_quill_dir, quill_ref, want):
    """`satisfies` answers the engine's own predicate over every selector arm."""
    quill = Quill.from_path(str(taro_quill_dir))
    assert quill.satisfies(quill_ref) is want
    assert Quill.satisfies_ref(quill_ref, "taro", "0.1.0") is want


@pytest.mark.parametrize("quill_ref,want", SATISFIES_CASES)
def test_quill_satisfies_agrees_with_render(taro_quill_dir, quill_ref, want):
    """The predicate's whole promise is about the render path, so the two are
    checked against each other rather than against a second copy of the table.
    Only the mismatch codes count: a blank document can fail for its own
    reasons, and those are not what `satisfies` answers."""
    quill = Quill.from_path(str(taro_quill_dir))
    doc = Document(quill_ref)

    mismatch = None
    try:
        Quillmark().render(quill, doc, OutputFormat.PDF)
    except QuillmarkError as err:
        code = err.diagnostics[0].code or ""
        if code in ("quill::name_mismatch", "quill::version_mismatch"):
            mismatch = code

    assert (mismatch is None) is want


def test_quill_satisfies_rejects_a_malformed_reference(taro_quill_dir):
    """A reference that is not well-formed raises; a `False` means well-formed
    and unsatisfied, so the two outcomes stay distinguishable."""
    quill = Quill.from_path(str(taro_quill_dir))
    with pytest.raises(ValueError):
        quill.satisfies("Not A Ref")
    with pytest.raises(ValueError):
        Quill.satisfies_ref("Not A Ref", "taro", "0.1.0")


def test_quill_satisfies_ref_tolerates_an_unreadable_version():
    """A version the engine cannot read skips the selector rather than failing
    it, so the predicate keeps agreeing with the engine on a malformed quill."""
    assert Quill.satisfies_ref("taro@0.1.0", "taro", "not-a-version") is True
