from __future__ import annotations

from dataclasses import dataclass


@dataclass
class Diagnostic:
    severity: str  # "error" | "warning"
    code: str      # np. "E0001", "W0001"
    message: str
    line: int
    col: int = 1
    length: int = 1
    filename: str = "<hcs>"

    def render(self, source: str) -> str:
        return render(
            source=source,
            filename=self.filename,
            line=self.line,
            col=self.col,
            message=self.message,
            code=self.code,
            severity=self.severity,
            length=self.length,
        )


def render(
    source: str,
    filename: str,
    line: int,
    col: int,
    message: str,
    code: str | None = None,
    severity: str = "error",
    length: int = 1,
) -> str:
    lines = source.splitlines() or [""]
    line_idx = max(0, min(line - 1, len(lines) - 1))
    src_line = lines[line_idx] if lines else ""

    gutter = str(line)
    gutter_width = len(gutter)
    pad = " " * gutter_width

    col = max(1, col)
    length = max(1, length)
    caret_line = " " * (col - 1) + "^" * length

    tag = f"{severity}[{code}]" if code else severity
    out = [
        f"{tag}: {message}",
        f"{pad} --> {filename}:{line}:{col}",
        f"{pad} |",
        f"{gutter} | {src_line}",
        f"{pad} | {caret_line}",
    ]
    return "\n".join(out)


def render_many(source: str, filename: str, diagnostics: list[Diagnostic]) -> str:
    blocks = []
    for d in diagnostics:
        d.filename = filename
        blocks.append(d.render(source))
    return "\n\n".join(blocks)
