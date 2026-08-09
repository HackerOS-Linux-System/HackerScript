# hackerc

Transpilator jezyka **HackerScript** (`.hcs`) -> **Rust**. `direct[]`
jest jedynym miejscem, gdzie mozna uzyc czystego Pythona (wykonywanego
przez wbudowany interpreter PyO3).

## Instalacja (dev)

```bash
cd hackerc
pip install -e .
pip install pytest
pytest tests/        # albo: python3 tests/test_hackerc.py (fallback bez pytest)
```

## CLI

```bash
hackerc check plik.hcs              # typecheck, bez generowania kodu
hackerc lint plik.hcs               # tylko warningi
hackerc build plik.hcs -o katalog/  # pelny crate Rust (Cargo.toml + src/)
hackerc fmt plik.hcs [--check]      # formatter (idempotentny)

# tryb kompatybilnosci wstecznej (uzywany przez virus/hackerc_bridge.rs):
hackerc plik.hcs -o wyjscie.rs
```

## Architektura

```
hackerc/
  lexer.py          tokenizacja; flaga tight na '[' (x[i] vs blok)
  ast_nodes.py       definicje wezlow AST
  parser.py          recursive-descent parser: tokeny -> AST
  typeinfer.py         inferencja typow dla let x = ... bez adnotacji
  typecheck.py          statyczna analiza AST (E0001-E0005, W0001-W0002)
  diagnostics.py          renderowanie bledow (fragment + karetka)
  codegen.py                JEDYNY backend: AST -> kod Rust
  project.py                  system modulow (get<std/core>), DWUFAZOWO:
                               faza 1 zbiera sygnatury z calego projektu,
                               faza 2 generuje kod z pelna widocznoscia
                               (potrzebne do poprawnych &/&mut na
                               wywolaniach cross-plikowych)
  formatter.py                  AST -> kanoniczny tekst .hcs (hackerc fmt)
  transpiler.py                   preprocessing direct[...], spina reszte
  cli.py                            hackerc check/lint/build/fmt/<plik.hcs>
entrypoint.py             punkt wejscia dla PyInstaller (naprawia
                           ImportError przy budowaniu binarki)
```

## Status

23/23 testow przechodzi (`tests/test_hackerc.py`). Pokrywaja: struct ->
Rust struct + konstruktor, &/&mut na parametrach (w tym cross-plikowo
przez system modulow), direct[] -> PyO3, manual[] -> unsafe{}, extern,
rzutowanie as, typecheck, formatter, diagnostyke.

WAZNE: te testy sprawdzaja poprawnosc STRUKTURALNA wygenerowanego Rusta
(obecnosc oczekiwanych konstrukcji w tekscie) - w srodowisku, w ktorym
hackerc powstal, nie bylo rustc/cargo, wiec zaden wygenerowany kod nie
przeszedl jeszcze przez prawdziwy kompilator. Patrz
.github/workflows/ci.yml, ktore to robi na prawdziwym runnerze, i
../docs/ROADMAP.md po pelna liste zastrzezen.
