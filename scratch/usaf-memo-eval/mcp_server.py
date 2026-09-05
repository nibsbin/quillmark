#!/usr/bin/env python3
"""Stdio MCP server exposing get_blueprint and create_document for Quillmark."""

from __future__ import annotations

from mcp.server.mcpserver import MCPServer

import quillmark_tools as tools

server = MCPServer(
    "quillmark",
    instructions=(
        "Author documents for a loaded Quillmark quill. Call get_blueprint, fill the "
        "annotated Markdown blueprint, then submit it with create_document. If "
        "create_document returns ok=false, read the diagnostics and retry."
    ),
)


@server.tool()
def get_blueprint() -> dict:
    """Return the instruction header, format rules, and annotated Markdown blueprint."""
    return tools.get_blueprint()


@server.tool()
def create_document(content: str) -> dict:
    """Parse and validate filled Markdown. Returns diagnostics on failure so you can retry."""
    return tools.create_document(content)


def main() -> None:
    tools.load_quill()
    server.run(transport="stdio")


if __name__ == "__main__":
    main()
