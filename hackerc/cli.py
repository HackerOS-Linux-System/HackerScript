"""
hackerc.cli
===========
hackerc TYLKO tlumaczy (transpiluje) - nie kompiluje, nie zarzadza
cache/. Za to odpowiada `virus`. hackerc jest wywolywany przez virus
jako podproces (albo recznie w trakcie developmentu hackerc).

Uzycie:
    hackerc <plik.hsc> [-o wyjscie.py]
    hackerc --version
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .transpiler import transpile_file, transpile_source, TranspileError

__version__ = "0.0.1"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="hackerc", description="Transpilator HackerScript -> Python")
    parser.add_argument("source", nargs="?", help="plik .hsc do przetlumaczenia")
    parser.add_argument("-o", "--output", help="plik wyjsciowy .py (domyslnie stdout)")
    parser.add_argument("--version", action="store_true", help="pokaz wersje hackerc")
    parser.add_argument("--emit-stdout", action="store_true", help="wypisz wynik na stdout nawet z -o")
    args = parser.parse_args(argv)

    if args.version:
        print(f"hackerc {__version__}")
        return 0

    if not args.source:
        parser.print_help()
        return 1

    src_path = Path(args.source)
    if not src_path.exists():
        print(f"hackerc: nie znaleziono pliku {src_path}", file=sys.stderr)
        return 1

    try:
        if args.output:
            transpile_file(src_path, args.output)
            if args.emit_stdout:
                print(Path(args.output).read_text(encoding="utf-8"))
        else:
            code = transpile_source(src_path.read_text(encoding="utf-8"), filename=str(src_path))
            print(code)
    except TranspileError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
