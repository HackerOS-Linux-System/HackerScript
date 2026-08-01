from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .transpiler import transpile_file, transpile_source_full, TranspileError
from .project import build_project, ProjectError
from .parser import parse, ParseError
from .lexer import LexError
from .typecheck import check_program
from .diagnostics import render_many, render
from .formatter import format_source

__version__ = "0.0.1"

_SUBCOMMANDS = {"check", "build", "fmt", "lint"}


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def _print_diagnostics(source: str, filename: str, diags, only_severity: str | None = None) -> int:
    if only_severity:
        diags = [d for d in diags if d.severity == only_severity]
    if not diags:
        return 0
    print(render_many(source, filename, diags), file=sys.stderr)
    return sum(1 for d in diags if d.severity == "error")


def cmd_check(args) -> int:
    src_path = Path(args.source)
    if not src_path.exists():
        print(f"hackerc: nie znaleziono pliku {src_path}", file=sys.stderr)
        return 1
    source = _read(src_path)
    try:
        program = parse(source)
    except (ParseError, LexError) as exc:
        print(render(source, str(src_path), exc.line, exc.col, str(exc.message), severity="error"), file=sys.stderr)
        return 1

    diags = check_program(program)
    errors = _print_diagnostics(source, str(src_path), diags)
    if errors:
        print(f"\nhackerc check: {errors} blad(ow)", file=sys.stderr)
        return 1
    print(f"hackerc check: {src_path} OK ({len(diags)} warning(ow))")
    return 0


def cmd_lint(args) -> int:
    src_path = Path(args.source)
    if not src_path.exists():
        print(f"hackerc: nie znaleziono pliku {src_path}", file=sys.stderr)
        return 1
    source = _read(src_path)
    try:
        program = parse(source)
    except (ParseError, LexError) as exc:
        print(render(source, str(src_path), exc.line, exc.col, str(exc.message), severity="error"), file=sys.stderr)
        return 1
    diags = check_program(program)
    warnings = [d for d in diags if d.severity == "warning"]
    if not warnings:
        print(f"hackerc lint: {src_path} - brak uwag")
        return 0
    print(render_many(source, str(src_path), warnings), file=sys.stderr)
    return 0


def cmd_build(args) -> int:
    src_path = Path(args.source)
    if not src_path.exists():
        print(f"hackerc: nie znaleziono pliku {src_path}", file=sys.stderr)
        return 1
    source = _read(src_path)

    # Najpierw diagnostyka - nie generujemy kodu z bledami typow/wywolan.
    try:
        program = parse(source)
    except (ParseError, LexError) as exc:
        print(render(source, str(src_path), exc.line, exc.col, str(exc.message), severity="error"), file=sys.stderr)
        return 1
    diags = check_program(program)
    errors = _print_diagnostics(source, str(src_path), diags)
    if errors:
        print(f"\nhackerc build: przerwano - {errors} blad(ow) w {src_path}", file=sys.stderr)
        return 1

    out_dir = Path(args.output) if args.output else src_path.parent
    libs_root = Path(args.libs_root) if args.libs_root else None
    try:
        result = build_project(src_path, out_dir, libs_root=libs_root, native_package=args.native_package)
    except ProjectError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    for w in result.warnings:
        print(f"hackerc build: uwaga: {w}", file=sys.stderr)

    print(f"hackerc build: zapisano {result.entry_output}")
    for flat, path in result.module_outputs.items():
        print(f"hackerc build: modul {flat} -> {path}")
    for native_dir in result.native_dirs:
        print(f"hackerc build: native fun -> {native_dir}/ (wymaga 'cargo build' - patrz virus)")
    return 0


def cmd_fmt(args) -> int:
    src_path = Path(args.source)
    if not src_path.exists():
        print(f"hackerc: nie znaleziono pliku {src_path}", file=sys.stderr)
        return 1
    source = _read(src_path)
    try:
        formatted = format_source(source, filename=str(src_path))
    except (ParseError, LexError) as exc:
        print(render(source, str(src_path), exc.line, exc.col, str(exc.message), severity="error"), file=sys.stderr)
        return 1

    if args.check:
        if formatted != source:
            print(f"hackerc fmt --check: {src_path} NIE jest sformatowany")
            return 1
        print(f"hackerc fmt --check: {src_path} OK")
        return 0

    dest = Path(args.output) if args.output else src_path
    dest.write_text(formatted, encoding="utf-8")
    print(f"hackerc fmt: sformatowano {dest}")
    return 0


def cmd_transpile_legacy(args) -> int:
    """Tryb kompatybilnosci wstecznej (uzywany przez virus/hackerc_bridge.rs)."""
    src_path = Path(args.source)
    if not src_path.exists():
        print(f"hackerc: nie znaleziono pliku {src_path}", file=sys.stderr)
        return 1
    try:
        if args.output:
            result = transpile_file(src_path, args.output, native_package=args.native_package)
            if args.emit_stdout:
                print(Path(args.output).read_text(encoding="utf-8"))
        else:
            source = _read(src_path)
            result = transpile_source_full(source, filename=str(src_path), native_package=args.native_package)
            print(result.python_code)
    except TranspileError as exc:
        if exc.line:
            print(
                render(_read(src_path), str(src_path), exc.line, exc.col or 1, str(exc), severity="error"),
                file=sys.stderr,
            )
        else:
            print(f"error: {exc}", file=sys.stderr)
        return 1
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="hackerc", description="Transpilator HackerScript -> Python (+ native/Rust)")
    parser.add_argument("--version", action="store_true", help="pokaz wersje hackerc")

    sub = parser.add_subparsers(dest="command")

    p_check = sub.add_parser("check", help="typecheck bez generowania kodu")
    p_check.add_argument("source")

    p_lint = sub.add_parser("lint", help="tylko warningi (podzbior 'check')")
    p_lint.add_argument("source")

    p_build = sub.add_parser("build", help="pelna transpilacja (Python + ew. native/Rust)")
    p_build.add_argument("source")
    p_build.add_argument("-o", "--output", help="katalog wyjsciowy (domyslnie obok zrodla)")
    p_build.add_argument("--native-package", default="hackerscript", help="nazwa bazowa modulu native")
    p_build.add_argument("--libs-root", help="katalog libs/ (domyslnie: szukany w gore od pliku zrodlowego)")

    p_fmt = sub.add_parser("fmt", help="formatuje kod .hcs")
    p_fmt.add_argument("source")
    p_fmt.add_argument("-o", "--output", help="zapisz gdzie indziej (domyslnie w miejscu)")
    p_fmt.add_argument("--check", action="store_true", help="tylko sprawdz, nie zapisuj")

    return parser


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)

    if argv and argv[0] in _SUBCOMMANDS:
        parser = build_parser()
        args = parser.parse_args(argv)
        if args.command == "check":
            return cmd_check(args)
        if args.command == "lint":
            return cmd_lint(args)
        if args.command == "build":
            return cmd_build(args)
        if args.command == "fmt":
            return cmd_fmt(args)

    # Tryb legacy / kompatybilnosci wstecznej: `hackerc plik.hcs [-o wyjscie.py]`
    legacy = argparse.ArgumentParser(prog="hackerc")
    legacy.add_argument("source", nargs="?")
    legacy.add_argument("-o", "--output")
    legacy.add_argument("--version", action="store_true")
    legacy.add_argument("--emit-stdout", action="store_true")
    legacy.add_argument("--native-package", default="hackerscript")
    args = legacy.parse_args(argv)

    if args.version:
        print(f"hackerc {__version__}")
        return 0
    if not args.source:
        build_parser().print_help()
        return 1
    return cmd_transpile_legacy(args)


if __name__ == "__main__":
    raise SystemExit(main())
