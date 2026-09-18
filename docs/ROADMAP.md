# Roadmap / znane braki HackerScript

Ten plik jest cytowany z dziesiątek komentarzy `!!!` w całym kodzie
(`hackerc/cmd/*.hcs`, `virus/cmd/*.hcs`, `libs/*/lib/mod.hcs`,
`README.adoc`, `docs/SYNTAX.md`) jako "pełna, szczera lista braków" —
ale fizycznie nie istniał w repozytorium. To jest jego pierwsza wersja,
zebrana z tamtych komentarzy plus wiedzy zdobytej przy pracy nad 0.4.

Status: **0.4**, self-hosted (`hackerc` skompilowany przez samego
siebie). Nic poniżej nie jest "krytyczne" w sensie "hackerc nie
działa" — to lista miejsc, gdzie bootstrap wziął skrót, celowo albo z
braku czasu, i gdzie kolejna runda pracy przyniesie najwięcej.

## Zrobione w 0.4

* **Wspólny `cache/` dla całego workspace** (naprawiony bug). Każdy
  członek workspace (`hackerc/`, `virus/`, `libs/core/`, `libs/std/`)
  ma własny, w pełni poprawny `Virus.hk` — co wcześniej powodowało, że
  `find_project_root` (virus/cmd/cache.hcs) zatrzymywał się na
  NAJBLIŻSZYM `Virus.hk` zamiast na korzeniu całego workspace, i
  `virus <cokolwiek>` odpalone z wnętrza `hackerc/` czy `virus/`
  tworzyło **własny, osobny** `cache/` zamiast dzielić jeden wspólny z
  korzeniem — dokładnie tak jak `cargo` w workspace zawsze dzieli
  jeden `target/`, niezależnie z którego członka go odpalisz. Naprawa:
  `find_project_root` teraz idzie w górę przez WSZYSTKICH przodków
  (nie tylko bezpośredniego rodzica), sprawdzając, czy jakiś dalszy
  `Virus.hk` z sekcją `[workspace]` rości sobie prawa (przez
  `-> members`) do znalezionego katalogu — i bierze najdalszego takiego
  przodka. Patrz `virus/cmd/cache.hcs::climb_to_workspace_root`.

* **Usunięcie `[build]` z `Virus.hk`.** Sekcja `[build] -> entry =>
  <ścieżka>`, pozwalająca nadpisać plik wejściowy, została CAŁKOWICIE
  usunięta z formatu `.hk`. Powód: to była jedyna sekcja w całym
  manifeście pozwalająca odejść od konwencji `cmd/main.hcs` — a w
  praktyce (`hackerc/Virus.hk`: `-> entry => cli.hcs`, bez `cmd/`)
  wskazywała na plik, który **nigdy nie istniał** (prawdziwa ścieżka
  to zawsze była `hackerc/cmd/cli.hcs`), więc `virus build` na
  korzeniu workspace po cichu pomijał `hackerc` jako rzekomą
  "bibliotekę bez entry point". Naprawa: plik przemianowany na
  `hackerc/cmd/main.hcs` (jedyna dopuszczalna nazwa od 0.4), pole
  `[build]` usunięte wszędzie — patrz `find_cmd_entry()` w
  `virus/cmd/manifest.hcs`, dokładny analog tego, jak Cargo samo
  znajduje `src/main.rs` bez żadnego pola w `Cargo.toml`.

* **`get <work:członek[::plik]>`** — ogólny import dowolnego członka
  workspace mającego `lib/mod.hcs` (nie tylko uprzywilejowanych
  `core`/`std`), analog Rustowego `use nazwa_membera::modul::*;` dla
  członka `cargo`-owego workspace. Patrz `docs/SYNTAX.md`, sekcja
  "`get <work:...>`" i `find_workspace_root`/`work_module_file_path`
  w `hackerc/cmd/project.hcs`. Przy okazji naprawiono też drobne,
  wcześniej istniejące przeoczenie: `"hlib"` nie był wypisany w liście
  znanych źródeł w `typecheck.hcs` (`is_known_get_source`), mimo że
  `gen_get_import`/`Discovery.resolve` już go w pełni obsługiwały —
  poprawny `get <hlib:nazwa>` dostawał fałszywy błąd E0003.

* **`get <work:członek[::plik]>`** — ogólny import dowolnego członka
  workspace mającego `lib/mod.hcs` (nie tylko uprzywilejowanych
  `core`/`std`), analog Rustowego `use nazwa_membera::modul::*;` dla
  członka `cargo`-owego workspace. Patrz `docs/SYNTAX.md`, sekcja
  "`get <work:...>`" i `find_workspace_root`/`work_module_file_path`
  w `hackerc/cmd/project.hcs`. Przy okazji naprawiono też drobne,
  wcześniej istniejące przeoczenie: `"hlib"` nie był wypisany w liście
  znanych źródeł w `typecheck.hcs` (`is_known_get_source`), mimo że
  `gen_get_import`/`Discovery.resolve` już go w pełni obsługiwały —
  poprawny `get <hlib:nazwa>` dostawał fałszywy błąd E0003.

* **`include <work:członek[::plik]>`** — drugi kształt `include`
  (obok zwykłego, względem katalogu pliku): PRAWDZIWE statyczne
  linkowanie źródła innego członka workspace (scalane bez prefiksu,
  jak Rustowe `mod`), rozwiązywane względem korzenia workspace —
  bez duplikowania plików. Patrz `docs/SYNTAX.md`, sekcja
  "`include <work:...>`".

* **`@wasm_export` + `virus build --wasm`** — pierwszy działający
  pipeline kompilacji do WASM: marker `@wasm_export` (wzorowany na
  `@hot_reload`) oznacza funkcję do wyeksportowania przez
  `wasm-bindgen`; obecność choć jednej takiej funkcji przełącza
  wygenerowany `Cargo.toml` z `[[bin]]` na `[lib]` (cdylib+rlib);
  `virus build --wasm` kompiluje na `wasm32-unknown-unknown` i (gdy
  `wasm-bindgen-cli` jest zainstalowany) generuje glue JS. Patrz
  `docs/SYNTAX.md`, oraz **`playground/`** — pierwszy prawdziwy
  konsument obu nowości: `playground/cmd/main.hcs` statycznie linkuje
  cały checker `hackerc`a (`ast_nodes`/`parser`/`typecheck`/
  `diagnostics`) przez `include <work:hackerc::...>`, eksportuje
  `check_source` przez `@wasm_export`, a `playground/web/`
  (`index.html`+`main.js`) to działająca strona w stylu Rust
  Playground — patrz `playground/web/README.md` po dokładne kroki
  budowania (wymaga lokalnego Rust + `wasm-bindgen-cli`, nie da się
  zweryfikować end-to-end w tym środowisku bez pełnego toolchaina).

## 1. Braki językowe samego bootstrapu (`hackerc`)

* Brak iteracji po `Dict` (`.keys()`/`.values()`/`.items()`) —
  dostępny jest tylko `.fetch(known_key)`. Wymusza to obejścia w
  całym kompilatorze (ręczne listy `*_names` towarzyszące każdemu
  `Dict`, np. w `typeinfer.hcs`/`transpiler.hcs`).
* Brak realnego `ParseError`/wyjątków — parser (`parser.hcs`) loguje
  błąd i próbuje kontynuować zamiast rzucać wyjątek jak wersja
  pythonowa.
* Brak numerów linii w AST — utrudnia dokładną diagnostykę błędów.
* `log()` nie obsługuje structów/enumów (brak odpowiednika
  `Display`/`Debug` z Rusta) — `typeinfer.hcs`.
* Brak `Set` jako typu.
* Brak prymitywu tworzenia katalogów (`mkdir(parents=True)`) —
  `out_path` w `transpile_file`/`build_project` musi już istnieć
  (patrz `create_dir` bez rekursji w `project.hcs`).

## 2. Uproszczenia per-plik w kompilatorze

`codegen.hcs`, `typeinfer.hcs`, `project.hcs`, `cli.hcs` (dziś
`main.hcs`), `formatter.hcs`, `parser.hcs` — każdy ma odziedziczone
uproszczenia z wersji pythonowej sprzed self-hostingu, częściowo tylko
nadrobione podczas bootstrapu.

## 3. FFI 0.3/0.4

* `get <c:...>`/`get <cpp:...>` linkuje dziś tylko systemowe
  biblioteki po nazwie — kompilacja WŁASNYCH źródeł `.c`/`.cpp` z
  projektu przez crate `cc` jeszcze nie działa (`virus/cmd/build.hcs`).
* `native {C++}` wymaga zainstalowanego kompilatora; brak lepszej
  obsługi błędów przy jego braku.
* Sygnatury zadeklarowane w `region [ ... ]` nie są dziś wpuszczane do
  `typecheck.hcs`/`typeinfer.hcs` — literówka w nazwie/typie parametru
  nie da błędu kompilatora HackerScript, tylko błąd `rustc` na
  wygenerowanym kodzie.
* Format `.hlib` (biblioteki binarne) — dopiero raczkuje:
  `hackerc hlib build/inspect/verify` działa, ale integracja z `virus
  install` (auto-generowanie stubów `get <extern:...>`) jest
  częściowa.

## 4. Luki w `libs/std` poza rdzeniem

Rdzeń (`fs`, `io`, `string`, `math`, `json`, `result`) jest solidny,
ale moduły dodatkowe (`toml`, `http`, `process`, `term`,
`cybersecurity/`) mają mniejsze pokrycie funkcji niż odpowiedniki w
Pythonie/Rust.

## 5. Sandbox i uprawnienia (`codegen.hcs`)

Izolacja sieciowa (`CLONE_NEWNET`) i systemu plików (`CLONE_NEWNS`) po
prostu wypisuje ostrzeżenie i uruchamia bez izolacji, gdy brakuje
uprawnień (np. `CAP_SYS_ADMIN`) — zamiast twardo odmówić lub
zaoferować alternatywę.

## 6. Menedżer pakietów `virus`

* Baza diagnostyk dla `virus repair` jest niepełna — nieznane kody
  błędów kończą się komunikatem "zgłoś to, żeby dodać do bazy".
* Rejestr `vira.io` obsługuje typy `git`/`static-lib`/`shared-lib`/
  `rust-lib`, ale rozpoznawanie nieznanych rozszerzeń plików
  (`.a`/`.so`) czasem polega na zgadywaniu trybu linkowania.
* `--library`/`TargetLibrary` (budowanie członka workspace jako
  biblioteki .rlib, bez próby "uruchomienia" go) nie jest jeszcze
  zaimplementowane — dziś członkowie bez `cmd/main.hcs` są po prostu
  pomijani przy `virus build` na całym workspace (patrz
  `workspace_member_buildable_entry`), a nie kompilowani osobno jako
  `.rlib`.
* Multi-binarki na jednego członka (`[[bin]]` jak w Cargo) — dziś
  jeden `cmd/main.hcs` = jeden budowalny plik wejściowy na członka;
  brak odpowiednika Cargo `src/bin/*.rs`.

## 7. Dokumentacja

Ten plik jest pierwszą wersją — wcześniej cytowany wszędzie, ale
fizycznie nieobecny, więc lista braków była rozproszona po
komentarzach `!!!` w kodzie. Wciąż brakuje:

* Formalnej gramatyki `hackerc` (BNF/EBNF) — `docs/SYNTAX.md` jest
  ręcznie pisaną mapą, nie specyfikacją.
* Osobnego dokumentu opisującego `virus` (dziś tylko `--help` w
  `virus/cmd/main.hcs` + ten plik).
