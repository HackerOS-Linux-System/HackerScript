# HackerScript

Jezyk programowania ogolnego przeznaczenia. Zwykla `fun` kompiluje sie
do **prawdziwego Rusta** (statyczne typy, bezpieczenstwo pamieci,
zero-cost abstractions). `direct [ ... ]` to ucieczka do czystego
Pythona, wykonywanego przez wbudowany interpreter (PyO3) - dla tego
niewielkiego fragmentu kodu, ktory nigdy nie bedzie wymagal wydajnosci.

```hcs
struct Point [
    x: Int,
    y: Int
]

fun distance_squared(p: Point) -> Int [
    end (p.x * p.x) + (p.y * p.y)
]

fun main() [
    let p = Point(3, 4)
    log("dist^2 =", distance_squared(p))

    direct [
        print("To jest czysty Python wykonywany wewnatrz binarki Rust.")
    ]
    end
]
```

Zobacz **`examples/showcase.hcs`** - jeden plik demonstrujacy CALA
dzisiejsza skladnie.

## Struktura repozytorium

```
HackerScript/
  hackerc/          transpilator .hcs -> Rust (Python)
  virus/            manager pakietow i narzedzie budowania (Rust)
    cli/              binarka `virus`
    hk-parser/        parser PRAWDZIWEGO formatu .hk (nie TOML!)
  libs/
    core/lib/memory/  4 alokatory: arena, chained_arena, stack_allocator, pool_allocator
    std/lib/cybersecurity/  constant_time_eq, shannon_entropy
  examples/
    hello-world/       podstawowy przyklad
    module-demo/        przyklad systemu modulow (get <core:...>)
    showcase.hcs         WSZYSTKIE funkcje skladni w jednym pliku
  docs/
    SYNTAX.md         pelny opis skladni
    ROADMAP.md        co jeszcze brakuje
  .github/workflows/  CI (buduje I URUCHAMIA wygenerowany crate) + Release
```

## Jak to dziala

1. `fun` -> `hackerc` generuje prawdziwy Rust (struct -> `struct`+`impl
   new()`, `manual[]` -> `unsafe{}`, `List<T>` -> `Vec<T>`, parametry
   struct/List/Str automatycznie dostaja `&`/`&mut` zamiast przenoszenia
   wlasnosci).
2. `direct [ ... ]` -> surowy Python wykonywany w trakcie dzialania
   programu przez `Python::with_gil` (PyO3, tryb `auto-initialize`) -
   Rust jest hostem.
3. `get <core:memory::arena>` -> realnie importuje kod z
   `libs/core/lib/memory/arena.hcs` (system modulow: `hackerc/project.py`
   dwufazowo zbiera sygnatury z calego projektu, zeby wywolania
   cross-plikowe tez dostaly poprawne `&`/`&mut`).
4. `get <crates:nazwa>` -> prawdziwa zaleznosc Cargo. `get
   <pypi/npm/jsr:...>` -> pobierane przez `virus install`
   (bezposrednio z PyPI/crates.io/npm/JSR API, BEZ `pip`/`cargo add`/
   `npm install`).
5. `virus build` -> `hackerc build` (generuje cargo crate) -> `cargo
   build` (jedyne miejsce gdzie `virus` uzywa cargo - jako kompilator,
   nie menedzer pakietow).

## Szybki start

```bash
cd hackerc && pip install -e . && cd ..

hackerc check examples/showcase.hcs
hackerc build examples/showcase.hcs -o /tmp/out
cd /tmp/out && cargo run   # wymaga zainstalowanego Rust
```

## Status projektu

**Strona Python (`hackerc/`) jest w calosci przetestowana**: 23/23
testow (`hackerc/tests/test_hackerc.py`) pokrywajacych transpilacje do
Rusta, `struct`/`&mut`/`&`, `direct[]`->PyO3, system modulow, formatter,
diagnostyke. W trakcie tej pracy znaleziono i naprawiono kilka realnych
bledow (m.in. `!=` mylone z komentarzem, `Vec::len()` zwracajace
`usize` a nie `i64`, przenoszenie wlasnosci struct/Vec/String przy
wielokrotnym uzyciu tej samej zmiennej) - patrz `docs/ROADMAP.md`.

**Strona Rust (`virus/`) jest napisana kompletnie, ale w tym
srodowisku nadal nie ma `cargo`/`rustc` do jej zbudowania.** CI
(`.github/workflows/ci.yml`) teraz KOMPILUJE I URUCHAMIA wygenerowany
crate na prawdziwym runnerze z Rustem - to jedyne miejsce gdzie
poprawnosc generowanego kodu Rust jest realnie zweryfikowana.

Pelna, szczera lista tego co jeszcze brakuje: **`docs/ROADMAP.md`**.

## Dokumentacja

- [`docs/SYNTAX.md`](docs/SYNTAX.md) - pelny opis skladni
- [`docs/ROADMAP.md`](docs/ROADMAP.md) - co brakuje
- [`hackerc/README.md`](hackerc/README.md) - architektura transpilatora
- Format `.hk`: https://hackeros-linux-system.github.io/HackerOS-Website/tools-docs/hk.html
