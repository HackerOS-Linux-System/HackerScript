# `virus` — menedżer pakietów i narzędzie budowania

Status: **0.4**. Do tej pory `virus` był opisany wyłącznie przez
`virus --help` (`virus/cmd/main.hcs::cmd_usage`) i rozproszone
wzmianki w `docs/SYNTAX.md`/`docs/ROADMAP.md`. To jego pierwszy
osobny dokument.

`virus` jest do `hackerc` tym, czym `cargo` jest do `rustc`: `hackerc`
kompiluje pojedynczy plik `.hcs` → Rust → binarkę, `virus` zarządza
całym projektem/workspace (manifest `Virus.hk`, zależności, cache,
komendy pomocnicze) i pod spodem woła `hackerc` przez
`virus/cmd/hackerc_bridge.hcs`.

## Manifest `Virus.hk`

```
[package]
name = "moj_projekt"
version = "0.1.0"
using = "0.4"

[dependencies]
!! wypełniane przez `virus install` / edytowane ręcznie

[workspace]
members = ["hackerc", "virus", "libs/core", "libs/std"]
```

Sekcja `[build] -> entry => <ścieżka>` **nie istnieje od 0.4** (patrz
`docs/ROADMAP.md`) — plik wejściowy dowolnego budowalnego członka to
zawsze `cmd/main.hcs`, analogicznie do tego jak Cargo samo znajduje
`src/main.rs` bez pola w `Cargo.toml`.

### `[native_sources]` (nowość — zaimplementowane w tej rundzie)

```
[native_sources]
-> c   => ["native/hash.c"]
-> cpp => ["native/matrix.cpp", "native/blas_shim.cpp"]
```

Listy ścieżek `.c`/`.cpp` **własnych źródeł projektu**, względem
korzenia projektu — kompilowane przez crate `cc` i linkowane
statycznie (`virus/cmd/build.hcs::build_wire_extern_dependencies`).
W odróżnieniu od `get <c:nazwa>`/`get <cpp:nazwa>` w kodzie `.hcs`
(które tylko linkują *systemową* bibliotekę po nazwie, nic nie
kompilują), to jedyny sposób wkompilowania WŁASNEGO kodu C/C++
projektu. `virus build` sprawdza z góry dostępność kompilatora
(`cc`/`gcc`/`clang`/`cl`) i zwraca czytelny błąd, zanim w ogóle
spróbuje uruchomić `cargo build`.

### `[[bin]]` przez `cmd/bin/*.hcs` (nowość — zaimplementowane w tej rundzie)

Każdy plik `cmd/bin/<nazwa>.hcs` obok `cmd/main.hcs` to dodatkowa
binarka tego samego pakietu (patrz "Plan: multi-binarki na członka"
niżej). `virus build` buduje domyślnie wszystkie; `virus build --bin
<nazwa>` buduje tylko jedną (główną albo jedną z `cmd/bin/`).

## Komendy

### `virus init [--name NAZWA]`
Tworzy szkielet nowego projektu (`Virus.hk`, `cmd/main.hcs`).

### `virus build [plik] [--release|--library|--wasm|--jar] [--bin NAZWA]`
Buduje projekt. Domyślnie (bez `--bin`) buduje `cmd/main.hcs` ORAZ
wszystkie `cmd/bin/*.hcs` naraz (patrz `[[bin]]` wyżej); `--bin
<nazwa>` buduje tylko jedną z nich. Warianty celu
(`virus/cmd/build.hcs`):

| Flaga | Cel (`Target...`) | Status |
|---|---|---|
| *(brak)* | `TargetDefault` | debug build, gotowe |
| `--release` | `TargetRelease` | `cargo build --release`, gotowe |
| `--wasm` | `TargetWasm` | `wasm32-unknown-unknown` (+ `wasm-bindgen` gdy dostępny), gotowe od 0.4 |
| `--jar` | `TargetJar` | gotowe |
| `--library` | `TargetLibrary` | ✅ zaimplementowane (ta runda) — `[lib] crate-type=["rlib"]`, patrz "Plan: `--library`" niżej (opis zostaje jako dziennik decyzji projektowej) |

### `virus cache`
Tworzy/odświeża wspólny `cache/` całego workspace (patrz
`docs/ROADMAP.md`, sekcja "Zrobione w 0.4" — `find_project_root`
wchodzi po wszystkich przodkach, nie tylko najbliższym `Virus.hk`).

### `virus check` / `virus lint [plik]` / `virus fmt [plik] [--check]`
Sprawdzanie typów bez budowania / tylko warningi / formatowanie
`.hcs` (ewentualnie tryb `--check`, bez zapisu, kod wyjścia ≠ 0 przy
niesformatowanym pliku).

### `virus install <źródło> <nazwa> [--version WERSJA]`
Instaluje zależność. Źródła: `pypi`, `crates`, `npm`, `jsr`, `vira`,
`bytes`, `bit`, `std`, `core`. Dla `vira` rozpoznaje typy `git` /
`static-lib` / `shared-lib` / `rust-lib` z rejestru `vira.io`
(`virus/cmd/vira.hcs`) — patrz "Plan: rozpoznawanie rozszerzeń" niżej
w kwestii nieznanych rozszerzeń plików.

### `virus remove <źródło> <nazwa>`
Usuwa zależność z manifestu.

### `virus repair <kod>`
Tłumaczy kod diagnostyczny kompilatora (np. `E0001`) na czytelne
wyjaśnienie + sugerowaną naprawę — patrz "Plan: baza diagnostyk"
niżej w kwestii zakresu dzisiejszej bazy.

### `virus clean`
Usuwa cały `cache/`.

### `virus lsp` *(nowość — projektowane w tej rundzie)*
Patrz `docs/LSP.md`.

---

## Rozbudowa (zaprojektowane, a następnie zaimplementowane)

Poniższe punkty odpowiadają 1:1 sekcji 6 w `docs/ROADMAP.md`. Cztery
z nich są od tej rundy **zaimplementowane w kodzie** (pliki/funkcje
wymienione niżej naprawdę istnieją, nie tylko są zaplanowane) —
opisy zostają jako dziennik decyzji projektowych, nie jako lista "do
zrobienia".

### ✅ Baza diagnostyk `virus repair` (zrobione)

Dziś (`virus/cmd/repair.hcs`) baza to płaska lista trzech
`RepairDiagnostic(kod, tytuł, opis, sugestia)` (`E0001`–`E0003`);
nieznany kod → `"zgłoś to, żeby dodać do bazy"`.

Plan:
1. Przenieść listę z hardkodowanej funkcji do `virus/cmd/repair.hcs::
   diagnostics_table()` zwracającej `List<RepairDiagnostic>`, jedno
   miejsce = jeden dodany kod (dziś już blisko tego kształtu — zmiana
   głównie porządkowa).
2. Rozszerzyć zakres kodów o resztę realnie zwracanych przez
   `hackerc/cmd/diagnostics.hcs` (dziś `repair.hcs` zna tylko 3 z
   nich) — docelowo każdy kod, jaki `diagnostics.hcs` może wyemitować,
   ma odpowiednik w `repair.hcs`; test spójności: skrypt/asercja przy
   `virus check` na samym `hackerc`u porównujący dwa zbiory kodów.
3. Nieznany kod: zamiast tylko komunikatu "zgłoś to", `virus repair`
   próbuje dopasowania przybliżonego (odległość Levenshteina po
   liście znanych kodów) i podpowiada najbliższy, zanim odeśle do
   zgłoszenia.

### ✅ Rozpoznawanie rozszerzeń `.a`/`.so` przy instalacji (zrobione)

Dziś `virus/cmd/vira.hcs` (typ `static-lib`/`shared-lib` z rejestru
`vira.io`) częściowo zgaduje tryb linkowania po nieznanym rozszerzeniu
pliku. Plan:
1. Jawna tabela rozszerzenie → tryb w `vira.hcs`:
   `.a`/`.lib` → `static`, `.so`/`.dylib`/`.dll` → `dynamic`,
   reszta → błąd z prośbą o jawne `use <static|dynamic>` zamiast
   cichego zgadywania.
2. Rejestr `vira.io` dostaje opcjonalne pole `link_mode` w opisie
   pakietu typu `static-lib`/`shared-lib` — gdy obecne, wygrywa nad
   tabelą rozszerzeń (jawna deklaracja autora paczki > heurystyka).

### ✅ `--library` / `TargetLibrary` (zrobione)

Dziś flaga jest parsowana (`virus/cmd/main.hcs`), ale
`virus/cmd/build.hcs::TargetLibrary` nie ma osobnej ścieżki budowania
— a `workspace_member_buildable_entry` pomija członków bez
`cmd/main.hcs` przy `virus build` na całym workspace, zamiast budować
ich jako `.rlib`. Plan:
1. `project.hcs::cargo_toml_text` dla `TargetLibrary`: `[lib]` z
   `crate-type = ["rlib"]` (bez `cdylib` — to nie jest ścieżka WASM),
   `path = "src/lib.rs"`, wejście = `lib/mod.hcs` członka (nie
   `cmd/main.hcs` — biblioteka, patrz `docs/SYNTAX.md`, tabela
   "Kształt biblioteki/binarki").
2. `build.hcs::TargetLibrary` (dziś pusta gałąź, patrz linia 37):
   uruchamia `cargo build --lib` w wygenerowanym crate'cie.
3. `workspace_member_buildable_entry`: gdy `virus build` (bez pliku)
   idzie po całym workspace, członkowie z samym `lib/mod.hcs`
   (bez `cmd/`) są teraz budowani jako `TargetLibrary` zamiast
   pomijani — parytet z tym, jak `cargo build` w workspace kompiluje
   też crate'y biblioteczne, nie tylko binarne.

### ✅ Multi-binarki na członka (`cmd/bin/*.hcs`, zrobione)

Dziś jeden `cmd/main.hcs` = jeden plik wejściowy na członka. Plan,
analogiczny do Cargo:
1. Nowy, opcjonalny katalog `cmd/bin/*.hcs` obok `cmd/main.hcs` —
   każdy plik to osobna, dodatkowa binarka tego samego członka.
2. `project.hcs::cargo_toml_text` emituje jeden wpis `[[bin]]` na
   `cmd/main.hcs` (`name = <member>`) plus jeden `[[bin]]` na każdy
   `cmd/bin/<x>.hcs` (`name = <x>`, `path = "src/bin/<x>.rs"`).
3. `virus build --bin <nazwa>` — nowa flaga wybierająca, którą z
   wielu binarek budować/uruchamiać (domyślnie bez `--bin`: wszystkie,
   parytet z `cargo build` bez `--bin`).
4. Bez zmian w `get <work:...>` — multi-binarki nie wpływają na
   importowalność członka jako biblioteki (to zależy wyłącznie od
   obecności `lib/mod.hcs`, patrz `docs/SYNTAX.md`).
