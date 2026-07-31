# hackerc

Transpilator jezyka **HackerScript** (`.hcs`) do Pythona.

`hackerc` **tylko tlumaczy** kod - nie kompiluje, nie pobiera zaleznosci,
nie zarzadza cache'em. Tym wszystkim zajmuje sie `virus` (patrz `../virus`),
ktory wywoluje `hackerc` jako podproces w ramach `virus build`.

## Instalacja (dev)

```bash
cd hackerc
pip install -e .
```

## Uzycie samodzielne

```bash
hackerc cmd/main.hsc                # wypisuje wygenerowany kod Pythona na stdout
hackerc cmd/main.hsc -o out/main.py # zapisuje do pliku
```

## Architektura

```
hackerc/
  lexer.py       tokenizacja + usuwanie komentarzy (!, !=...=!, !!)
  ast_nodes.py    definicje wezlow AST (dataclasses)
  parser.py       recursive-descent parser: tokeny -> AST
  codegen.py      AST -> kod Pythona
  transpiler.py   preprocessing `direct [ ... ]` + spinanie lexer/parser/codegen
  cli.py          `hackerc <plik.hsc>`
```

## Status (bootstrap 0.0.1)

Zaimplementowane: `fun`, `let`/`const`, `if/elif/else`, `while`, `for..in`,
`end` (return), `break`/`continue`, `log(...)`, `get <...> import <...>`,
`using <wersja>`, `direct [ ... ]`, `manual [ ... ]`, `gc:use::tryb`,
`struct`, wyrazenia z pelnym priorytetem operatorow, listy `[ ]`.

Do zrobienia przed produkcyjna wersja 0.0.1 - patrz `../docs/ROADMAP.md`.
