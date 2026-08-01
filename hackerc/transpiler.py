from __future__ import annotations

import re
import textwrap
from dataclasses import dataclass, field
from pathlib import Path

from .parser import parse, ParseError
from .lexer import LexError
from .codegen import generate
from .native_codegen import generate_native_module, generate_native_cargo_toml, NativeCodegenError

_DIRECT_RE = re.compile(r"\bdirect\b")


class TranspileError(Exception):
    def __init__(self, message: str, line: int | None = None, col: int | None = None):
        super().__init__(message)
        self.line = line
        self.col = col


def _wrap(exc: ParseError | LexError, filename: str) -> TranspileError:
    return TranspileError(f"{filename}: {exc.message}", line=exc.line, col=getattr(exc, "col", 1))


@dataclass
class TranspileResult:
    """Wynik transpilacji jednego pliku .hcs - dwa backendy naraz.

    `python_code` to zawsze kompletny, samodzielny plik .py.
    `native_rust` jest NIE-None tylko jesli plik zawieral co najmniej
    jedna `native fun` - wtedy `python_code` importuje z modulu, ktory
    trzeba osobno skompilowac z `native_rust` (+ `native_cargo_toml`)
    przez `virus build` (kompilacja Rust -> cdylib -> link statyczny).
    """

    python_code: str
    native_rust: str | None = None
    native_cargo_toml: str | None = None
    native_package: str = "hackerscript"
    native_fun_names: list = field(default_factory=list)


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


def transpile_source_full(source: str, filename: str = "<hcs>", native_package: str = "hackerscript") -> TranspileResult:
    """Transpiluje zrodlo .hcs zwracajac WYNIK DLA OBU BACKENDOW:
    zwykle `fun` -> Python, `native fun` -> Rust (+ bindingi PyO3)."""
    stripped, direct_blocks = _extract_direct_blocks(source)
    try:
        program = parse(stripped)
    except (ParseError, LexError) as exc:
        raise _wrap(exc, filename) from exc

    python_code, natives = generate(program, direct_blocks=direct_blocks, native_package=native_package)

    if not natives:
        return TranspileResult(python_code=python_code)

    try:
        rust_code = generate_native_module(natives, native_package, direct_blocks)
    except NativeCodegenError as exc:
        raise TranspileError(f"{filename}: {exc}", line=exc.line) from exc

    return TranspileResult(
        python_code=python_code,
        native_rust=rust_code,
        native_cargo_toml=generate_native_cargo_toml(f"{native_package}_native"),
        native_package=native_package,
        native_fun_names=[fn.name for fn in natives],
    )


def transpile_source(source: str, filename: str = "<hcs>") -> str:
    """Transpiluje zrodlo HackerScript (.hcs) do zrodla Pythona (str).

    Skrot dla przypadku bez `native fun`. Jesli plik zawiera `native fun`,
    zwrocony Python nadal bedzie poprawny (z importem z modulu native),
    ale sam kod Rust NIE zostanie nigdzie zwrocony/zapisany - do pelnego
    builda uzyj `transpile_source_full` / `transpile_file` (ktore go
    zwracaja/zapisuja) - tak wlasnie robi `virus build`.
    """
    return transpile_source_full(source, filename=filename).python_code


def transpile_file(src_path: str | Path, out_path: str | Path, native_package: str = "hackerscript") -> TranspileResult:
    """Transpiluje plik .hcs -> .py (zapisuje `out_path`). Jesli plik
    zawiera `native fun`, zapisuje TEZ `<out_path bez rozszerzenia>_native/`
    z `lib.rs` + `Cargo.toml` gotowymi do `cargo build` przez `virus`."""
    src_path = Path(src_path)
    out_path = Path(out_path)
    source = src_path.read_text(encoding="utf-8")
    result = transpile_source_full(source, filename=str(src_path), native_package=native_package)

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(result.python_code, encoding="utf-8")

    if result.native_rust is not None:
        native_dir = out_path.parent / f"{out_path.stem}_native"
        (native_dir / "src").mkdir(parents=True, exist_ok=True)
        (native_dir / "src" / "lib.rs").write_text(result.native_rust, encoding="utf-8")
        (native_dir / "Cargo.toml").write_text(result.native_cargo_toml, encoding="utf-8")

    return result
