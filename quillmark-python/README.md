# Quillmark — Python bindings for Quillmark

Compact docs and canonical developer workflow.

Installation
------------

```bash
pip install quillmark
```

Quick start
-----------

```python
from quillmark import Quillmark, ParsedDocument, OutputFormat

engine = Quillmark()
parsed = ParsedDocument.from_markdown("# Hello")
workflow = engine.workflow_from_quill_name("my-quill")
workflow.render(parsed, OutputFormat.PDF).artifacts[0].save("out.pdf")
```

Development (opinionated)
-------------------------

This repository standardizes on `uv` for local development (https://astral.sh/uv). Use the commands below on macOS (zsh).

Install uv (one-time):

```zsh
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Canonical flow:

```zsh
# create the uv-managed venv
uv venv

# install developer extras (includes maturin, pytest, mypy, ruff)
uv pip install -e "[dev]"

# Change to release for production builds
# uv pip install -e ".[dev]" --release

# develop-install (compile + install into the venv)
uv run python -m maturin develop

# run tests
uv run pytest
```

Notes
- `maturin` builds the PyO3 extension; `uv` manages the virtualenv and command execution.
- Ensure Rust (rustup) and macOS command-line tools are installed when building.

License
-------

Apache-2.0


Building
---------------------

If you prefer an opinionated, reproducible Python workflow the project designs recommend `uv` (https://astral.sh/uv). `uv` provides a small wrapper for creating venvs and running common commands. These commands are equivalent to the venv/maturin flow above but shorter.

Install uv (one-time):

```zsh
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Typical workflow with `uv`:

```zsh
# create a venv and activate it
uv venv

# Build in debug mode (faster, suitable for development)
uv pip install -e ".[dev]"

# Build in release mode (slower, suitable for production)
uv run python -m maturin develop --release

# run the test suite
uv run pytest

# run mypy and ruff checks (project recommends these)
uv run mypy python/quillmark
uv run ruff check python/
```