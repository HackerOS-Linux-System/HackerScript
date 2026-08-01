# hackerc

Transpilator jezyka **HackerScript** (`.hcs`) - DWA backendy: zwykla
`fun` -> Python, `native fun` -> Rust + bindingi PyO3.

`hackerc` **tylko tlumaczy/analizuje** kod - nie pobiera zaleznosci
zewnetrznych (`pypi`/`crates`), nie zarzadza cache'em projektu. Tym
zajmuje sie `virus` (patrz `../virus`), ktory wywoluje `hackerc` jako
podproces w ramach `virus build`/`check`/`fmt`/`lint`.

## Instalacja (dev)

```bash
cd hackerc
pip install -e .
pip install pytest   # do testow
pytest tests/        # albo: python3 tests/test_hackerc.py (fallback bez pytest)
```

## CLI

```bash
hackerc check plik.hcs              # typecheck, bez generowania kodu
hackerc lint plik.hcs               # tylko warningi (podzbior check)
hackerc build plik.hcs -o katalog/  # pelna transpilacja + system modulow + native
hackerc fmt plik.hcs [--check]      # formatter (idempotentny)

# tryb kompatybilnosci wstecznej (uzywany przez virus/hackerc_bridge.rs):
hackerc plik.hcs -o wyjscie.py
```

## Architektura

```
hackerc/
  lexer.py          tokenizacja, komentarze (!, !=...=!, !!), flaga
                     `tight` na '[' (rozroznia x[i] od bloku "if x [")
  ast_nodes.py       definicje wezlow AST (dataclasses)
  parser.py          recursive-descent parser: tokeny -> AST
  typeinfer.py        inferencja typow dla `let x = ...` bez adnotacji
  typecheck.py         statyczna analiza AST (E0001-E0005, W0001-W0002)
  diagnostics.py       renderowanie bledow w stylu Rust/Elm (fragment+karetka)
  codegen.py           AST -> kod Pythona (backend #1)
  native_codegen.py     AST (tylko `native fun`) -> kod Rust + PyO3 (backend #2)
  project.py            system modulow: get<std/core> -> realne pliki .hcs,
                         splaszczone nazwy modulow, budowanie rekurencyjne
  formatter.py           AST -> kanoniczny tekst .hcs (`hackerc fmt`)
  transpiler.py           spina lexer/parser/codegen/native_codegen,
                           preprocessing `direct [ ... ]`
  cli.py                  `hackerc check|lint|build|fmt|<plik.hcs>`
```

## Status (bootstrap 0.0.1)

Zaimplementowane i pokryte testami (`tests/test_hackerc.py`, 23/23
przechodzi): `fun`/`native fun`, `let`/`const` (z inferencja typow),
`if/elif/else`, `while`, `for..in`, `end` (return), `break`/`continue`,
`log(...)`, `get <...> import <...>` (w tym dzialajacy system modulow
dla `std`/`core`), `using <wersja>`, `direct [ ... ]`, `manual [ ... ]`
(prawdziwy `unsafe{}` w `native fun`), `gc:use::tryb`, `struct`,
indeksowanie `x[i]` (rozroznione od bloku przez brak spacji),
wyrazenia z pelnym priorytetem operatorow.

Do zrobienia przed produkcyjna wersja 0.0.1 - patrz `../docs/ROADMAP.md`.
