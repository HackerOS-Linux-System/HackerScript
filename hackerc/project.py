from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from .parser import parse, ParseError
from .lexer import LexError
from . import ast_nodes as A
from .transpiler import transpile_source_full, TranspileError

_MODULE_SOURCES = ("std", "core")


class ProjectError(Exception):
    pass


def flat_module_name(source: str, name: str, version: str | None) -> str:
    """Splaszczona, deterministyczna nazwa modulu Python dla `get
    <std:...>` / `get <core:...>`. MUSI byc identyczna po obu stronach
    (patrz codegen.gen_get_import) - to jest cala "umowa" systemu modulow."""
    parts = [source, name] + ([version] if version else [])
    safe = [p.replace("-", "_") for p in parts]
    return "_hks_" + "_".join(safe)


def _module_file(libs_root: Path, source: str, name: str, version: str | None) -> Path:
    parts = [name] + ([version] if version else [])
    return libs_root / source / "lib" / Path(*parts).with_suffix(".hcs")


def find_libs_root(start: Path) -> Path | None:
    """Szuka katalogu `libs/` (z podkatalogami core/std) zaczynajac od
    `start` i idac w gore drzewa katalogow - tak jak virus szuka Virus.hk."""
    d = start if start.is_dir() else start.parent
    while True:
        candidate = d / "libs"
        if candidate.is_dir() and (candidate / "core").is_dir():
            return candidate
        if d.parent == d:
            return None
        d = d.parent


def _extract_lib_imports(source: str) -> list[A.GetImportStmt]:
    """Parsuje zrodlo TYLKO po to, zeby wyciagnac `get <std/core:...>` -
    uzywane rekurencyjnie do przechodzenia grafu zaleznosci modulow."""
    try:
        program = parse(source)
    except (ParseError, LexError):
        return []  # bledy skladni zglosi wlasciwa transpilacja, tu pomijamy
    return [s for s in program.body if isinstance(s, A.GetImportStmt) and s.source in _MODULE_SOURCES]


@dataclass
class ProjectResult:
    entry_output: Path
    module_outputs: dict[str, Path] = field(default_factory=dict)  # flat_name -> .py
    native_dirs: list[Path] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)


def build_project(
    entry: Path,
    out_dir: Path,
    libs_root: Path | None = None,
    native_package: str = "hackerscript",
) -> ProjectResult:
    """Transpiluje `entry` DO `out_dir`, oraz rekurencyjnie kazdy modul
    `std`/`core`, ktory `entry` (lub jego zaleznosci) importuja przez
    `get <...>`. Wszystko ladujde do jednego plaskiego `out_dir`."""
    out_dir.mkdir(parents=True, exist_ok=True)
    result = ProjectResult(entry_output=out_dir / (entry.stem + ".py"))

    if libs_root is None:
        libs_root = find_libs_root(entry)

    def transpile_one(src_path: Path, out_path: Path, module_label: str):
        source = src_path.read_text(encoding="utf-8")
        try:
            r = transpile_source_full(source, filename=str(src_path), native_package=native_package)
        except TranspileError as exc:
            raise ProjectError(f"{src_path}: {exc}") from exc
        out_path.write_text(r.python_code, encoding="utf-8")
        if r.native_rust is not None:
            native_dir = out_path.parent / f"{out_path.stem}_native"
            (native_dir / "src").mkdir(parents=True, exist_ok=True)
            (native_dir / "src" / "lib.rs").write_text(r.native_rust, encoding="utf-8")
            (native_dir / "Cargo.toml").write_text(r.native_cargo_toml, encoding="utf-8")
            result.native_dirs.append(native_dir)
        return source

    # Plik wejsciowy
    entry_source = transpile_one(entry, result.entry_output, "entry")

    # BFS po grafie modulow std/core
    seen: set[str] = set()
    queue: list[A.GetImportStmt] = _extract_lib_imports(entry_source)

    while queue:
        imp = queue.pop(0)
        flat = flat_module_name(imp.source, imp.name, imp.version)
        if flat in seen:
            continue
        seen.add(flat)

        if libs_root is None:
            result.warnings.append(
                f"get <{imp.source}:{imp.name}> - nie znaleziono katalogu libs/ "
                f"(szukano w gore od {entry}) - import zostanie w wygenerowanym "
                f"kodzie, ale modul {flat} nie istnieje"
            )
            continue

        mod_file = _module_file(libs_root, imp.source, imp.name, imp.version)
        if not mod_file.exists():
            result.warnings.append(f"get <{imp.source}:{imp.name}> -> nie znaleziono {mod_file}")
            continue

        mod_out = out_dir / f"{flat}.py"
        mod_source = transpile_one(mod_file, mod_out, flat)
        result.module_outputs[flat] = mod_out

        # transytywne get<std/core> tego modulu tez trzeba rozwiazac
        queue.extend(_extract_lib_imports(mod_source))

    return result
