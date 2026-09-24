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

> Status tej rundy: lista niżej bez zmian merytorycznych — te braki
> są przesłanką (nie celem) dla `docs/LSP.md` (numery linii w AST są
> wymagane dla dokładnego `textDocument/publishDiagnostics`) i dla
> `docs/GRAMMAR.md` (sekcja "Znane rozbieżności"). Kolejność
> wdrożenia: numery linii w AST i `ParseError` idą przed Etapem 2a
> `lsp` (patrz `docs/LSP.md`), reszta punktów niżej pozostaje
> nieuszeregowana w czasie.

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

* ✅ **Zrobione (ta runda):** kompilacja WŁASNYCH źródeł `.c`/`.cpp`
  projektu przez crate `cc`, obok `get <c:...>`/`get <cpp:...>`
  (które nadal tylko linkują systemową bibliotekę po nazwie) — nowa
  sekcja manifestu `[native_sources] -> c = [...]` / `cpp = [...]`
  (`virus/cmd/manifest.hcs::NativeSourcesSection`), którą
  `build_wire_extern_dependencies` (`virus/cmd/build.hcs`) zamienia na
  wywołania `cc::Build::new().file(...).compile(...)` w generowanym
  `build.rs`, dopisując `[build-dependencies] cc = "1"` do Cargo.toml,
  gdy jeszcze go tam nie ma.
* ✅ **Zrobione (ta runda), częściowo:** `virus build` sprawdza z góry
  dostępność kompilatora C/C++ (`cc`/`gcc`/`clang`/`cl`), gdy projekt
  używa `[native_sources]`, i zwraca czytelny błąd zamiast surowego
  błędu `cc`/`rustc` w środku `cargo build`. **Zostaje:** inline
  `native {C++}[...]` (bez `[native_sources]`) wciąż nie ma tego
  prechecku — jego błąd braku kompilatora nadal wychodzi dopiero z
  samego `cc`.
* ✅ **Zrobione (ta runda):** sygnatury zadeklarowane w
  `region [ ... ]` trafiają teraz do tej samej tabeli `functions` co
  zwykłe `fun` (`hackerc/cmd/typeinfer.hcs::collect_signatures`) —
  `check_call` (typecheck.hcs) sprawdza teraz liczbę argumentów przy
  wywołaniach funkcji z `region`, zamiast dawać błąd dopiero z
  `rustc` na wygenerowanym kodzie. Patrz `docs/SYNTAX.md`, sekcja FFI
  "Ograniczenia".
* Format `.hlib` (biblioteki binarne) — dopiero raczkuje:
  `hackerc hlib build/inspect/verify` działa, ale integracja z `virus
  install` (auto-generowanie stubów `get <extern:...>`) jest
  częściowa. **Plan:** `virus install hlib <nazwa>` po pobraniu
  archiwum `.hlib` woła `hackerc hlib inspect --stubs` (nowa flaga) i
  zapisuje wygenerowane stuby `get <extern:...> use <...>` +
  `region [...]` do `cache/hlib_stubs/<nazwa>.hcs`, gotowe do
  `include`.

## 4. Luki w `libs/std` poza rdzeniem

Rdzeń (`fs`, `io`, `string`, `math`, `json`, `result`) jest solidny,
ale moduły dodatkowe (`toml`, `http`, `process`, `term`,
`cybersecurity/`) mają mniejsze pokrycie funkcji niż odpowiedniki w
Pythonie/Rust.

**Plan domykania, moduł po module** (kolejność wg tego, co dziś
najczęściej brakuje w praktyce, ustalona przy pisaniu `docs/VIRUS.md`
i `docs/FAST_DIRECT.md`):
1. `libs/std/lib/process.hcs` — brakuje przechwytywania stdout/stderr
   jako strumieni (dziś tylko odpowiednik `run-and-collect`); potrzebne
   wprost pod przyszłe `fast direct {multiprocessing}`/`{ray}`
   (patrz `docs/FAST_DIRECT.md`) do zarządzania procesami roboczymi.
2. `libs/std/lib/http.hcs` — brak klienta streamującego (dziś
   całe ciało odpowiedzi na raz); zderza się z tym samym obszarem co
   przyszłe `fast direct {httpx}`/`{aiohttp}`.
3. `libs/std/lib/term.hcs` — pokrycie kolorów/kursora wystarczające
   dla dzisiejszych potrzeb (`virus`, patrz `docs/VIRUS.md`), brakuje
   odczytu rozmiaru terminala i trybu raw (potrzebne pod interaktywne
   `virus repair`/przyszłe `lsp` logi debug).
4. `libs/std/lib/toml.hcs` — brak zapisu (tylko odczyt) — `Virus.hk`
   dziś edytowany przez `virus install`/`remove` na poziomie tekstu,
   nie przez ten moduł; docelowo `install.hcs`/`remove.hcs` powinny
   przejść na `toml.hcs` do zapisu, gdy ten zyska serializację.
5. `libs/std/lib/cybersecurity/entropy.hcs` — dziś tylko entropia
   Shannona; brak innych podstawowych prymitywów (hashowanie,
   stałoczasowe porównanie) używanych w przykładach z `docs/showcase`.

## 5. Sandbox i uprawnienia (`codegen.hcs`)

Izolacja sieciowa (`CLONE_NEWNET`) i systemu plików (`CLONE_NEWNS`) po
prostu wypisuje ostrzeżenie i uruchamia bez izolacji, gdy brakuje
uprawnień (np. `CAP_SYS_ADMIN`) — zamiast twardo odmówić lub
zaoferować alternatywę.

## 6. Menedżer pakietów `virus`

Cztery z pięciu punktów niżej są teraz **zaimplementowane w kodzie**
(nie tylko zaprojektowane) — pełny opis w **`docs/VIRUS.md`**, sekcja
"Plany rozbudowy" (nazwa sekcji zostaje historyczna, treść już
odzwierciedla stan "zrobione").

* ✅ **Zrobione (ta runda):** baza diagnostyk `virus repair`
  przepisana na zgodną 1:1 z realnymi kodami z `typecheck.hcs`
  (`virus/cmd/repair.hcs`), plus dopasowanie przybliżone
  (Levenshtein) nieznanego kodu do najbliższego znanego.
* ✅ **Zrobione (ta runda):** rozpoznawanie `.a`/`.so`/`.dylib`/`.dll`
  jawną tabelą zamiast cichego domyślnego "dynamic" dla wszystkiego
  poza `.a` (`virus/cmd/build.hcs`) — nieznane rozszerzenie teraz
  jawnie OSTRZEGA, że tryb linkowania jest zgadywany.
* ✅ **Zrobione (ta runda):** `--library`/`TargetLibrary` — pełny
  łańcuch `hackerc build --library` → `[lib] crate-type=["rlib"]`
  (`project.hcs`) → `cargo build --lib` → kopia `.rlib`
  (`virus/cmd/build.hcs`, `hackerc_bridge.hcs`). Członkowie workspace
  z samym `lib/mod.hcs` (np. `libs/core`, `libs/std`) są teraz
  budowani jako `.rlib` zamiast pomijani.
* ✅ **Zrobione (ta runda):** multi-binarki — `cmd/bin/*.hcs` obok
  `cmd/main.hcs` dopisują własne `[[bin]]` do Cargo.toml
  (`hackerc build --bin-name`, `project.hcs::build_project`), `virus
  build` buduje domyślnie wszystkie, `virus build --bin <nazwa>`
  buduje tylko jedną.
* **Nowość — `virus lsp`** (jeszcze nieistniejąca komenda, projekt
  gotowy, kod jeszcze nie napisany): patrz `docs/LSP.md`.

## 7. Dokumentacja

Ten plik jest pierwszą wersją — wcześniej cytowany wszędzie, ale
fizycznie nieobecny, więc lista braków była rozproszona po
komentarzach `!!!` w kodzie.

* ✅ **Formalna gramatyka `hackerc` (EBNF)** — `docs/GRAMMAR.md`,
  dodana w tej rundzie. `docs/SYNTAX.md` zostaje jako ręcznie pisana
  mapa/samouczek (nadal nadrzędny wobec obu w razie rozbieżności jest
  kod `lexer.hcs`/`parser.hcs`), `GRAMMAR.md` to formalne uzupełnienie
  w notacji EBNF, z osobną sekcją nazywającą wprost dwa miejsca, gdzie
  gramatyka formalna i dzisiejszy parser się rozjeżdżają (brak
  `ParseError`, brak numerów linii w AST — patrz sekcja 1 niżej).
* ✅ **Osobny dokument opisujący `virus`** — `docs/VIRUS.md`, dodana w
  tej rundzie. Opisuje manifest `Virus.hk`, wszystkie komendy
  (`init`/`build`/`cache`/`check`/`lint`/`fmt`/`install`/`remove`/
  `repair`/`clean`) i — nowość względem samego `--help` — rozpisany
  plan implementacji dla każdego z czterech braków w sekcji 6 niżej.

## 8. Plan kolejnej rundy: `lsp` i `fast direct {backend}`

Dwie duże, nowe rzeczy zaprojektowane w tej rundzie (Faza 1 —
specyfikacja + dokumentacja; kod przyjdzie w kolejnych rundach,
iteracyjnie, żeby każdy krok był realnie sprawdzalny zamiast
deklarowany "zrobiony" bez pokrycia w działającym kodzie):

* ✅ **Zrobione (ta runda) — Etap 2a:** `hackerc lsp`/`virus lsp`
  istnieją i działają — JSON-RPC po stdio (`hackerc/cmd/lsp.hcs`),
  `initialize`/`shutdown`/`exit`, `textDocument/didOpen`/`didChange`
  → prawdziwy `textDocument/publishDiagnostics` (reużywa
  `check_program`). Wymagało dwóch nowych prymitywów w samym
  `hackerc` (`read_stdin_line`/`read_stdin_exact`, `write_stdout`,
  `run_command_inherit` — patrz `codegen.hcs::gen_call`), bo bootstrap
  wcześniej w ogóle nie czytał stdin. **Zostaje (Etap 2b/2c):**
  hover/definition/completion/rename/formatting — pełny plan w
  **`docs/LSP.md`**. Zakresy diagnostyk są dziś zawsze `(0,0)-(0,1)`
  (patrz sekcja 1 wyżej — brak numerów linii w AST).
* **`fast direct {backend} [ ... ]`** — rozszerzenie dzisiejszego
  `direct[...]` o wybór jednego z 19 backendów przyspieszających
  Pythona (`pypy`, `numba`, `cython`, `numpy`, `polars`, `scipy`,
  `asyncio`, `uvloop`, `multiprocessing`, `jax`, `pythran`, `duckdb`,
  `cupy`, `vaex`, `numexpr`, `trio`, `aiohttp`, `granian`, `httpx`,
  `ray`), ze statycznym linkowaniem całego interpretera (PyPy albo
  CPython, zależnie od backendu) i bibliotek do finalnej binarki —
  poza udokumentowanymi wyjątkami narzuconymi przez same biblioteki
  (CUDA runtime dla `cupy`, proces `raylet` dla `ray`). Pełna
  specyfikacja, w tym kolejność wdrażania w 6 grupach, w
  **`docs/FAST_DIRECT.md`**. Gramatyka już dodana do
  `docs/GRAMMAR.md`, sekcja 6.
