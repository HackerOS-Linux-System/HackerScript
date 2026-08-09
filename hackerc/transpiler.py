from __future__ import annotations

import re
import textwrap
from dataclasses import dataclass
from pathlib import Path

from .parser import parse, ParseError
from .lexer import LexError
from .codegen import generate, CodegenError

_DIRECT_RE = re.compile(r"\bdirect\b")


class TranspileError(Exception):
    def __init__(self, message: str, line: int | None = None, col: int | None = None):
        super().__init__(message)
        self.line = line
        self.col = col


def _wrap(exc, filename: str) -> TranspileError:
    return TranspileError(f"{filename}: {exc.message}", line=exc.line, col=getattr(exc, "col", 1))


@dataclass
class TranspileResult:
    rust_code: str
    needs_pyo3: bool = False


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


def transpile_source_full(
    source: str,
    filename: str = "<hcs>",
    module_name: str = "module",
    extra_functions: dict | None = None,
    extra_structs: dict | None = None,
    extra_enums: dict | None = None,
    extra_mut_params: dict | None = None,
    extra_methods: dict | None = None,
    extra_method_mut_params: dict | None = None,
) -> TranspileResult:
    """Transpiluje zrodlo .hcs -> kod Rust (+ info czy potrzebne PyO3).

    `extra_functions`/`extra_structs`/`extra_enums`/`extra_mut_params`/
    `extra_methods`/`extra_method_mut_params` to sygnatury pochodzace z
    INNYCH plikow .hcs (importowanych przez `get <std/core/selfhost:...>`)
    - potrzebne, zeby wywolania funkcji/metod z innych modulow poprawnie
    dostaly `&`/`&mut` przy argumentach typu struct (w tym `&mut self`
    gdy metoda W TYM pliku wola metode Z INNEGO PLIKU, ktora mutuje - np.
    `impl TenStruct` rozbite na wiele plikow), a konstruktory wariantow
    enum (`Circle(...)`) poprawnie rozpoznaly, do ktorego enuma naleza.
    Wypelnia je `hackerc.project.build_project` (dwufazowo: najpierw
    zbiera sygnatury ze wszystkich plikow projektu, potem generuje)."""
    stripped, direct_blocks = _extract_direct_blocks(source)
    try:
        program = parse(stripped)
    except (ParseError, LexError) as exc:
        raise _wrap(exc, filename) from exc

    try:
        rust_code, needs_pyo3 = generate(
            program,
            direct_blocks=direct_blocks,
            module_name=module_name,
            extra_functions=extra_functions,
            extra_structs=extra_structs,
            extra_enums=extra_enums,
            extra_mut_params=extra_mut_params,
            extra_methods=extra_methods,
            extra_method_mut_params=extra_method_mut_params,
        )
    except CodegenError as exc:
        raise TranspileError(f"{filename}: {exc}", line=exc.line) from exc

    return TranspileResult(rust_code=rust_code, needs_pyo3=needs_pyo3)


def transpile_source(source: str, filename: str = "<hcs>") -> str:
    """Skrot: zwraca tylko kod Rust (bez metadanych typu needs_pyo3)."""
    return transpile_source_full(source, filename=filename).rust_code


def transpile_file(src_path: str | Path, out_path: str | Path, module_name: str | None = None) -> TranspileResult:
    src_path = Path(src_path)
    out_path = Path(out_path)
    source = src_path.read_text(encoding="utf-8")
    result = transpile_source_full(source, filename=str(src_path), module_name=module_name or src_path.stem)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(result.rust_code, encoding="utf-8")
    return result
