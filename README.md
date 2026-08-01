# HackerScript

Jezyk programowania ogolnego przeznaczenia inspirowany Swiftem, Zigiem,
Rustem i Lua. Statyczne typowanie (z realna inferencja), bezpieczenstwo
pamieci, wykrywanie bledow w trakcie kompilacji (jak w Adzie) - ale ze
skladnia zaprojektowana tak, zeby byla **holernie latwa**.

```hcs
using <1.2>

native fun add(a: Int, b: Int) -> Int [
    end a + b
]

fun main() [
    log("2 + 3 =", add(2, 3))
    end
]
```

`fun` -> transpilowane do Pythona. `native fun` -> **kompilowane do
prawdziwego Rusta** (przez PyO3) i linkowane statycznie - to jest
realizacja zalozenia "mala czesc kodu w Pythonie, reszta wydajnosciowa w
Rust jako wrapper", nie tylko deklaracja w dokumentacji.

## Struktura repozytorium

```
HackerScript/
  hackerc/          transpilator .hcs -> Python + Rust/PyO3 (Python)
  virus/            manager pakietow i narzedzie budowania (Rust)
    cli/              binarka `virus`
    hk-parser/        parser formatu manifestu Virus.hk
  libs/             biblioteki napisane w 100% w HackerScript
    core/             alternatywne systemy pamieci (areny, ...)
    std/              biblioteka standardowa (na razie pusta)
  examples/
    hello-world/      podstawowy przykladowy projekt
    module-demo/       przyklad systemu modulow (get <core:...>)
  docs/
    SYNTAX.md         pelny opis skladni
    ROADMAP.md        co jeszcze brakuje do wersji 0.0.1
  .github/workflows/  CI (testy) + Release (publikacja binarek)
```

## Jak to dziala

1. Kod uzytkownika (`cmd/*.hcs`) jest **transpilowany** przez `hackerc`.
   Zwykla `fun` -> Python. `native fun` -> Rust + bindingi PyO3,
   kompilowane przez `cargo` do `.so`/`.pyd`/`.dylib` i importowane z
   powrotem do Pythona (`hackerc/native_codegen.py`).
2. `get <core:...>` / `get <std:...>` **realnie importuje kod z innych
   plikow `.hcs`** (`hackerc/project.py`) - nie tylko generuje pusty
   import. `get <pypi:...>` / `get <crates:...>` pobiera zewnetrzne
   zaleznosci.
3. `virus` (odpowiednik `cargo`, ale niezalezny od `pip`/`cargo` jako
   menedzer WLASNYCH zaleznosci - rozmawia bezposrednio z PyPI/crates.io
   API) orkiestruje cala reszte: pobiera `hackerc`, zarzadza
   zaleznosciami, woła `hackerc build`, kompiluje wygenerowany `native
   fun` przez `cargo` (jako toolchain, nie menedzer pakietow), pakuje
   wynik do wybranego targetu.

Wszystko ladujde sie do `cache/` projektu uzytkownika:

```
cache/
  libs/     pobrane zaleznosci
  source/   przetlumaczony (przez hackerc) kod Pythona + skompilowany native
  env/      pobrane narzedzia (hackerc, itd.)
  build/    finalna binarka
```

## Szybki start

```bash
cd hackerc && pip install -e . && cd ..

# podstawowy przyklad
hackerc check examples/hello-world/cmd/main.hcs
hackerc build examples/hello-world/cmd/main.hcs -o /tmp/out
python3 /tmp/out/main.py

# przyklad z systemem modulow (get <core:memory::arena>)
hackerc build examples/module-demo/cmd/main.hcs -o /tmp/out2
python3 /tmp/out2/main.py   # "hello 42"

# formatowanie i lint
hackerc fmt cmd/main.hcs
hackerc lint cmd/main.hcs
```

Docelowo (po zbudowaniu `virus` z `virus/`) to wszystko dzieje sie przez
`virus build`/`virus check`/`virus fmt` - `hackerc` jest wywolywane jako
podproces.

## Status projektu

**Strona Python (`hackerc/`) jest w calosci przetestowana**: 23/23
testow przechodzi (`hackerc/tests/test_hackerc.py`), pokrywajac
transpilacje, inferencje typow, typecheck, `native fun` -> Rust,
system modulow, formatter i diagnostyke.

**Strona Rust (`virus/`) jest napisana kompletnie, ale NIE zostala
skompilowana** w srodowisku, w ktorym powstal ten kod (brak `cargo` w
sandboxie). Przed pierwszym uzyciem: `cd virus && cargo build
--workspace` i napraw ewentualne bledy kompilacji wynikajace z wersji
zaleznosci (`indicatif`/`clap`/`reqwest`/`zip`/`serde_json`/PyO3).

CI (`.github/workflows/ci.yml`, `release.yml`) jest napisane i powinno
to zweryfikowac automatycznie przy pierwszym pushu do repo, ale rowniez
nie zostalo jeszcze uruchomione naprawde (brak dostepu do GitHub
Actions z tego srodowiska).

Pelna, szczera lista tego co jeszcze brakuje: **`docs/ROADMAP.md`**.
Najpilniejsze pozostale: **realne zbudowanie `virus` przez `cargo`**
i naprawienie tego co przy tym wyjdzie.

## Dokumentacja

- [`docs/SYNTAX.md`](docs/SYNTAX.md) - pelny opis skladni
- [`docs/ROADMAP.md`](docs/ROADMAP.md) - co brakuje do bootstrapu / 0.0.1
- [`hackerc/README.md`](hackerc/README.md) - architektura transpilatora
- Format `Virus.hk`: https://hackeros-linux-system.github.io/HackerOS-Website/tools-docs/hk.html
