"""
hackerc.transpiler
==================
Publiczne API transpilatora HackerScript -> Python.

    from hackerc.transpiler import transpile_source, transpile_file

`direct [ ... ]` jest szczegolnym przypadkiem: to CZYSTY kod Pythona
wstawiany 1:1 do wyniku (patrz spec HackerScript). Wyciagamy go na
etapie preprocessingu (przed tokenizacja HackerScript), zeby nie
wymagac od uzytkownika pisania Pythona zgodnego ze skladnia .hsc.
"""

from __future__ import annotations

import re
import textwrap
from pathlib import Path

from .parser import parse, ParseError
from .lexer import LexError
from .codegen import generate

_DIRECT_RE = re.compile(r"\bdirect\b")


class TranspileError(Exception):
    pass


def _extract_direct_blocks(source: str) -> tuple[str, dict[int, str]]:
    """Zamienia kazdy blok `direct [ ... ]` na wyrazenie `__direct__(N)`
    i zwraca (nowe_zrodlo, {N: surowy_kod_pythona})."""
    blocks: dict[int, str] = {}
    out = []
    i = 0
    idx = 0
    n = len(source)
    while i < n:
        m = _DIRECT_RE.search(source, i)
        if not m:
            out.append(source[i:])
            break
        start = m.start()
        out.append(source[i:start])
        j = m.end()
        while j < n and source[j] in " \t":
            j += 1
        if j >= n or source[j] != "[":
            # nie blok direct[] - to np. identyfikator zawierajacy "direct"
            # (nie powinno sie zdarzyc bo \b, ale zachowujemy bezpieczenstwo)
            out.append(source[start:j])
            i = j
            continue
        depth = 0
        k = j
        while k < n:
            if source[k] == "[":
                depth += 1
            elif source[k] == "]":
                depth -= 1
                if depth == 0:
                    break
            k += 1
        if depth != 0:
            raise TranspileError("niezamkniety blok direct [ ... ]")
        raw = source[j + 1 : k]
        raw = textwrap.dedent(raw).strip("\n")
        blocks[idx] = raw
        out.append(f"__direct__({idx})")
        idx += 1
        i = k + 1
    return "".join(out), blocks


def transpile_source(source: str, filename: str = "<hsc>") -> str:
    """Transpiluje zrodlo HackerScript (.hsc) do zrodla Pythona (str)."""
    stripped, direct_blocks = _extract_direct_blocks(source)
    try:
        program = parse(stripped)
    except (ParseError, LexError) as exc:
        raise TranspileError(f"{filename}: {exc}") from exc
    return generate(program, direct_blocks=direct_blocks)


def transpile_file(src_path: str | Path, out_path: str | Path) -> None:
    src_path = Path(src_path)
    out_path = Path(out_path)
    source = src_path.read_text(encoding="utf-8")
    py_code = transpile_source(source, filename=str(src_path))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(py_code, encoding="utf-8")
