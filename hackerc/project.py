from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from .parser import parse, ParseError
from .lexer import LexError
from . import ast_nodes as A
from .transpiler import transpile_source_full, TranspileError, _extract_direct_blocks
from .codegen import _compute_mut_params, _compute_method_mut_params

_MODULE_SOURCES = ("std", "core", "selfhost", "virus")

CARGO_TOML_TEMPLATE = """[package]
name = "{name}"
version = "0.0.1"
edition = "2021"

[[bin]]
name = "{name}"
path = "src/main.rs"

[dependencies]
{deps}
"""


class ProjectError(Exception):
    pass


def flat_module_name(source: str, name: str, version: str | None) -> str:
    """Splaszczona, deterministyczna nazwa modulu Rust dla `get
    <std:...>` / `get <core:...>`. MUSI byc identyczna po obu stronach
    (patrz codegen.gen_get_import) - to jest cala "umowa" systemu
    modulow. Musi tez byc poprawnym identyfikatorem Rusta (snake_case)."""
    parts = [source, name] + ([version] if version else [])
    safe = [p.replace("-", "_") for p in parts]
    return "_hks_" + "_".join(safe)


def flat_include_module_name(path: str) -> str:
    """Splaszczona nazwa modulu dla `include <sciezka>` - odpowiednik
    `flat_module_name` ale dla sciezek wzglednych zamiast trojki
    (source, name, version) - patrz `IncludeStmt` w ast_nodes.py i
    `resolve_include_path` ponizej. Prefiks `_hks_inc_` (zamiast
    samego `_hks_`) zeby NIGDY nie kolidowac z nazwa modulu z `get
    <...>` nawet przy tej samej "koncowce" (np. `include <io>` vs
    `get <std:io>` musza dac RozNE nazwy)."""
    p = path[:-4] if path.endswith(".hcs") else path
    safe = p.replace("-", "_").replace("/", "_")
    return "_hks_inc_" + safe


def resolve_include_path(raw_path: str, base_dir: Path) -> Path | None:
    """Rozwiazuje `include <raw_path>` na prawdziwy plik `.hcs`, WZGLEDEM
    `base_dir` (katalog pliku, w ktorym jest `include`) - w odroznieniu
    od `get <...>`, ktory zawsze szuka od `libs_root`/`bootstrap_root`.
    Kolejnosc dokladnie jak Rustowe `mod nazwa;` (plik przed
    katalogiem):
      1. `raw_path` juz konczy sie na `.hcs` -> uzyj WPROST.
      2. `base_dir / (raw_path + ".hcs")` jesli istnieje jako PLIK.
      3. `base_dir / raw_path / "mod.hcs"` jesli istnieje (katalog).
    Zwraca `None` jesli zaden wariant nie istnieje."""
    if raw_path.endswith(".hcs"):
        candidate = base_dir / raw_path
        return candidate if candidate.is_file() else None
    file_candidate = base_dir / f"{raw_path}.hcs"
    if file_candidate.is_file():
        return file_candidate
    dir_candidate = base_dir / raw_path / "mod.hcs"
    if dir_candidate.is_file():
        return dir_candidate
    return None


def _module_file(libs_root: Path, source: str, name: str, version: str | None) -> Path:
    parts = [name] + ([version] if version else [])
    return libs_root / source / "lib" / Path(*parts).with_suffix(".hcs")


def _selfhost_module_file(bootstrap_root: Path, name: str, version: str | None) -> Path:
    """Jak `_module_file`, ale dla `get <selfhost:...>` - pliki
    `bootstrap/hackerc-self/*.hcs` leza PLASKO (bez zagniezdzenia
    `<source>/lib/`), bo to nie jest 'prawdziwa' biblioteka tylko
    kolekcja modulow samo-hostowanego kompilatora - patrz
    bootstrap/README.md."""
    parts = [name] + ([version] if version else [])
    return bootstrap_root / Path(*parts).with_suffix(".hcs")


def find_libs_root(start: Path) -> Path | None:
    """Szuka katalogu `libs/` (z podkatalogiem core/) zaczynajac od
    `start` i idac w gore drzewa katalogow - tak jak virus szuka Virus.hk.

    NAPRAWA BUGA: `start` MUSI byc rozwiazane do sciezki BEZWZGLEDNEJ
    (`.resolve()`) PRZED petla - dla wzglednej "golej" nazwy pliku bez
    katalogu (np. `hackerc build cli.hcs`) `Path("cli.hcs").parent`
    daje `Path(".")`, a `Path(".").parent` daje PONOWNIE `Path(".")`
    (self-referencyjne dla nierozwiazanych sciezek wzglednych) - warunek
    petli `d.parent == d` byl wiec PRAWDZIWY juz na starcie, konczac
    szukanie po SPRAWDZENIU WYLACZNIE biezacego katalogu, bez
    faktycznego wejscia w gore prawdziwego drzewa katalogow."""
    d = (start if start.is_dir() else start.parent).resolve()
    while True:
        candidate = d / "libs"
        if candidate.is_dir() and (candidate / "core").is_dir():
            return candidate
        if d.parent == d:
            return None
        d = d.parent


def find_bootstrap_root(start: Path) -> Path | None:
    """Jak `find_libs_root`, ale szuka `bootstrap/hackerc-self/` - korzen
    modulow dla `get <selfhost:...>` (patrz bootstrap/README.md).
    Ta sama naprawa co `find_libs_root` - `.resolve()` PRZED petla."""
    d = (start if start.is_dir() else start.parent).resolve()
    while True:
        candidate = d / "bootstrap" / "hackerc-self"
        if candidate.is_dir():
            return candidate
        if d.parent == d:
            return None
        d = d.parent


@dataclass
class ProjectResult:
    crate_dir: Path
    main_rs: Path
    module_files: dict[str, Path] = field(default_factory=dict)  # flat_name -> .rs
    cargo_toml: Path | None = None
    needs_pyo3: bool = False
    crates_deps: dict[str, str] = field(default_factory=dict)  # nazwa -> wersja ("*" jesli brak)
    warnings: list[str] = field(default_factory=list)


@dataclass
class _DiscoveredFile:
    path: Path
    flat_name: str  # "main" dla entry, w przeciwnym razie flat_module_name(...)
    source: str
    program: A.Program


def _resolve_project_files(
    entry: Path, libs_root: Path | None, bootstrap_root: Path | None, warnings: list[str]
) -> list[_DiscoveredFile]:
    """Faza 1 (discovery): parsuje entry + rekurencyjnie wszystkie
    zaimportowane moduly std/core/selfhost. Zwraca liste w kolejnosci
    odkrycia (entry jako pierwszy)."""
    files: list[_DiscoveredFile] = []
    seen: set[str] = set()

    def parse_one(path: Path, flat_name: str) -> _DiscoveredFile | None:
        source = path.read_text(encoding="utf-8")
        try:
            program = parse(_extract_direct_blocks(source)[0])
        except (ParseError, LexError) as exc:
            raise ProjectError(f"{path}: {exc}") from exc
        return _DiscoveredFile(path=path, flat_name=flat_name, source=source, program=program)

    entry_file = parse_one(entry, "main")
    files.append(entry_file)

    # Kolejka trzyma pary (stmt, base_dir) - `base_dir` to katalog PLIKU,
    # w ktorym `stmt` sie znajduje (potrzebne wylacznie dla `IncludeStmt`,
    # ktory jest rozwiazywany WZGLEDEM tego katalogu - `GetImportStmt`
    # ignoruje `base_dir`, zawsze szuka od libs_root/bootstrap_root).
    def module_stmts(df: "_DiscoveredFile"):
        base_dir = df.path.parent
        for s in df.program.body:
            if isinstance(s, A.GetImportStmt) and s.source in _MODULE_SOURCES:
                yield (s, base_dir)
            elif isinstance(s, A.IncludeStmt):
                yield (s, base_dir)

    queue: list[tuple] = list(module_stmts(entry_file))

    while queue:
        imp, base_dir = queue.pop(0)

        if isinstance(imp, A.IncludeStmt):
            flat = flat_include_module_name(imp.path)
            if flat in seen:
                continue
            seen.add(flat)
            mod_file = resolve_include_path(imp.path, base_dir)
            if mod_file is None:
                warnings.append(
                    f"include <{imp.path}> -> nie znaleziono (szukano {base_dir / (imp.path + '.hcs')} "
                    f"i {base_dir / imp.path / 'mod.hcs'})"
                )
                continue
            df = parse_one(mod_file, flat)
            files.append(df)
            queue.extend(module_stmts(df))
            continue

        flat = flat_module_name(imp.source, imp.name, imp.version)
        if flat in seen:
            continue
        seen.add(flat)

        if imp.source == "selfhost":
            if bootstrap_root is None:
                warnings.append(
                    f"get <selfhost:{imp.name}> - nie znaleziono katalogu bootstrap/hackerc-self/ "
                    f"(szukano w gore od {entry}) - modul {flat} nie zostanie zbudowany"
                )
                continue
            mod_file = _selfhost_module_file(bootstrap_root, imp.name, imp.version)
        else:
            if libs_root is None:
                warnings.append(
                    f"get <{imp.source}:{imp.name}> - nie znaleziono katalogu libs/ "
                    f"(szukano w gore od {entry}) - modul {flat} nie zostanie zbudowany"
                )
                continue
            mod_file = _module_file(libs_root, imp.source, imp.name, imp.version)

        if not mod_file.exists():
            warnings.append(f"get <{imp.source}:{imp.name}> -> nie znaleziono {mod_file}")
            continue

        df = parse_one(mod_file, flat)
        files.append(df)
        queue.extend(module_stmts(df))

    return files


def collect_project_signatures(
    entry: Path, libs_root: Path | None = None, bootstrap_root: Path | None = None
) -> tuple[dict, dict, dict, dict, dict, dict, list[str]]:
    """Faza 1 z `build_project` (discovery + globalny rejestr sygnatur),
    wydzielona do samodzielnego uzycia - pozwala np. `cmd_build`
    (cli.py) uruchomic `check_program()` na pliku wejsciowym ZE
    ZNAJOMOSCIA wariantow enum zaimportowanych z innych modulow, PRZED
    (a nie dopiero PO) pelnym `build_project()`. Bez tego uzycie
    konstruktora wariantu z importowanego `enum` (np. `Var(...)` z
    zaimportowanego `Expr`) dawalo spurious ostrzezenie W0002
    ("nieznana funkcja") na etapie typecheck, mimo ze `build_project()`
    samo w sobie dzialalo poprawnie - patrz docs/ROADMAP.md.

    Zwraca (global_functions, global_structs, global_enums,
    global_mut_params, global_methods, global_method_mut_params,
    warnings)."""
    if libs_root is None:
        libs_root = find_libs_root(entry)
    if bootstrap_root is None:
        bootstrap_root = find_bootstrap_root(entry)

    warnings: list[str] = []
    files = _resolve_project_files(entry, libs_root, bootstrap_root, warnings)

    global_functions: dict[str, A.FunDecl] = {}
    global_structs: dict[str, A.StructDecl] = {}
    global_enums: dict[str, A.EnumDecl] = {}
    global_mut_params: dict[str, set] = {}
    global_methods: dict[tuple, A.FunDecl] = {}
    global_method_mut_params: dict[str, set] = {}
    for df in files:
        for stmt in df.program.body:
            if isinstance(stmt, A.FunDecl):
                global_functions[stmt.name] = stmt
            elif isinstance(stmt, A.StructDecl):
                global_structs[stmt.name] = stmt
            elif isinstance(stmt, A.EnumDecl):
                global_enums[stmt.name] = stmt
            elif isinstance(stmt, A.ImplDecl):
                for m in stmt.methods:
                    global_methods[(stmt.struct_name, m.name)] = m
        global_mut_params.update(_compute_mut_params(df.program))
        global_method_mut_params.update(_compute_method_mut_params(df.program))

    return (
        global_functions,
        global_structs,
        global_enums,
        global_mut_params,
        global_methods,
        global_method_mut_params,
        warnings,
    )


def build_project(
    entry: Path,
    out_dir: Path,
    libs_root: Path | None = None,
    bootstrap_root: Path | None = None,
    crate_name: str = "hackerscript_app",
) -> ProjectResult:
    """Transpiluje `entry` + rekurencyjnie kazdy modul `std`/`core`/
    `selfhost`, ktory importuje, do PELNEGO crate'a Rust w `out_dir`
    (Cargo.toml + src/). Nie kompiluje (to robi `virus` przez `cargo
    build`)."""
    src_dir = out_dir / "src"
    src_dir.mkdir(parents=True, exist_ok=True)
    result = ProjectResult(crate_dir=out_dir, main_rs=src_dir / "main.rs")

    if libs_root is None:
        libs_root = find_libs_root(entry)
    if bootstrap_root is None:
        bootstrap_root = find_bootstrap_root(entry)

    # -- Faza 1: discovery + globalny rejestr sygnatur -------------------
    files = _resolve_project_files(entry, libs_root, bootstrap_root, result.warnings)

    global_functions: dict[str, A.FunDecl] = {}
    global_structs: dict[str, A.StructDecl] = {}
    global_enums: dict[str, A.EnumDecl] = {}
    global_mut_params: dict[str, set] = {}
    global_methods: dict[tuple, A.FunDecl] = {}
    global_method_mut_params: dict[str, set] = {}
    for df in files:
        for stmt in df.program.body:
            if isinstance(stmt, A.FunDecl):
                global_functions[stmt.name] = stmt
            elif isinstance(stmt, A.StructDecl):
                global_structs[stmt.name] = stmt
            elif isinstance(stmt, A.EnumDecl):
                global_enums[stmt.name] = stmt
            elif isinstance(stmt, A.ImplDecl):
                for m in stmt.methods:
                    global_methods[(stmt.struct_name, m.name)] = m
        global_mut_params.update(_compute_mut_params(df.program))
        global_method_mut_params.update(_compute_method_mut_params(df.program))

    # -- Faza 2: generacja z pelna widocznoscia calego projektu ----------
    mod_declarations: list[str] = []
    for df in files:
        try:
            r = transpile_source_full(
                df.source,
                filename=str(df.path),
                module_name=df.flat_name,
                extra_functions=global_functions,
                extra_structs=global_structs,
                extra_enums=global_enums,
                extra_mut_params=global_mut_params,
                extra_methods=global_methods,
                extra_method_mut_params=global_method_mut_params,
            )
        except TranspileError as exc:
            raise ProjectError(f"{df.path}: {exc}") from exc

        if r.needs_pyo3:
            result.needs_pyo3 = True

        for stmt in df.program.body:
            if isinstance(stmt, A.GetImportStmt) and stmt.source == "crates":
                result.crates_deps[stmt.name] = stmt.version or "*"

        if df.flat_name == "main":
            entry_rust = r.rust_code
        else:
            mod_out = src_dir / f"{df.flat_name}.rs"
            mod_out.write_text(r.rust_code, encoding="utf-8")
            result.module_files[df.flat_name] = mod_out
            mod_declarations.append(df.flat_name)

    # Deklaracje `mod X;` w Ruscie sa ZWYKLYMI ITEMAMI (moga stac gdziekolwiek
    # po atrybutach wewnetrznych `#![...]`), ale atrybuty wewnetrzne MUSZA
    # byc pierwszymi tokenami w pliku (poza komentarzami) - Rust odrzuca je
    # (E0753 "an inner attribute is not permitted in this context"), jesli
    # cokolwiek innego niz komentarz/inny atrybut wewnetrzny je poprzedza.
    # `entry_rust` zaczyna sie od `#![allow(...)]` (naglowek dopisywany przez
    # `codegen.py`/`codegen.hcs`) - dawne, naiwne `mod_header + entry_rust`
    # wstawialo `mod X;` PRZED tym atrybutem i lamalo kompilacje na KAZDYM
    # projekcie wieloplikowym. Bug znaleziony przy pierwszej realnej
    # kompilacji `cargo build` wielomodulowego projektu w tej sesji
    # (poprzednio niemozliwe bez dostepu do rustc, patrz bootstrap/README.md).
    # Naprawa: wstaw `mod X;` PO ostatniej wiodacej linii `#![...]`, nie przed
    # nia.
    mod_header = "\n".join(f"mod {m};" for m in mod_declarations)
    if mod_header:
        entry_lines = entry_rust.split("\n")
        insert_at = 0
        for idx, line in enumerate(entry_lines):
            if line.startswith("//") or line.startswith("#![") or line.strip() == "":
                insert_at = idx + 1
            else:
                break
        entry_lines[insert_at:insert_at] = ["", mod_header, ""]
        entry_rust = "\n".join(entry_lines)
    result.main_rs.write_text(entry_rust, encoding="utf-8")

    # Cargo.toml
    deps_lines = []
    if result.needs_pyo3:
        deps_lines.append('pyo3 = { version = "0.22", features = ["auto-initialize"] }')
    for name, ver in sorted(result.crates_deps.items()):
        deps_lines.append(f'{name} = "{ver}"' if ver != "*" else f'{name} = "*"')
    cargo_toml_text = CARGO_TOML_TEMPLATE.format(name=crate_name, deps="\n".join(deps_lines))
    result.cargo_toml = out_dir / "Cargo.toml"
    result.cargo_toml.write_text(cargo_toml_text, encoding="utf-8")

    return result
