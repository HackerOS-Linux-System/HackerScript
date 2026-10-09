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

## Aktualizacja sesji (patrz SESSION_LOG.md po pelna liste zmian plik-po-pliku)

Kilka punktow powyzej bylo NIEAKTUALNYCH wzgledem faktycznego stanu kodu -
sprawdzone i skorygowane w tej sesji:

* **`mkdir -p`/`create_dir`** - JUZ zaimplementowane (`create_dir` mapuje sie
  na `std::fs::create_dir_all`, rekurencyjne). Doszlifowane: `transpile_file`
  teraz faktycznie tworzy katalog nadrzedny `out_path` przed zapisem.
* **`virus lsp`** - JUZ w pelni zaimplementowane i podpiete (`cmd_lsp_run` w
  `virus/cmd/lsp.hcs`, wywolywane z `main.hcs`). Diagnostyka dziala; zakres
  Etap 2b/2c (hover/definition/completion/rename/formatting) nadal otwarty.
* **`virus build --jar`** - JUZ w pelni zaimplementowane (`build_package_jar`
  + flaga `--jar` w `virus/cmd/main.hcs` -> `TargetJar`), NIE placeholder.
* **`virus build --release --wasm`** - naprawiony realny bug: `include
  <work:hackerc::...>` nigdy nie dzialalo dla czlonkow-binarek (`hackerc`,
  `playground`) trzymajacych kod w `cmd/` zamiast `lib/` - patrz
  `find_workspace_root`/`work_module_file_path` w `hackerc/cmd/project.hcs`.
  Plus naprawiony `#[wasm_bindgen]` + parametry `&String` (nieobslugiwane
  przez `wasm-bindgen`) - patrz `wasm_param_type_str` w `codegen.hcs`.
* **`Dict.keys()`/`.values()`** - dodane (typeinfer.hcs + codegen.hcs).
  `.items()` zostaje - wymaga typu pary/tupli.
* **Sandbox `$ ... $`** - zmienione z "ostrzezenie + uruchom bez izolacji" na
  fail-closed (kod wyjscia 111, polecenie NIE wykonuje sie bez izolacji).
* **NOWOSC: `using <wersja>` per plik** - patrz **`docs/MULTI_VERSION.md`** -
  rozne pliki `.hcs` w JEDNYM projekcie moga deklarowac rozne wersje jezyka;
  `virus build` pobiera wszystkie potrzebne binarki `hackerc` i orkiestruje
  delegacje per-plik. Plus `Virus.hk -> [package] -> all-versions => [...]`.
* **`hack3rc`** - dodany do `[workspace]`, manifest + plan architektury
  (frontend re-used z `hackerc`, backend Cranelift) - **implementacja
  jeszcze NIE zaczeta** (tylko manifest/plan), patrz `docs/HACK3RC.md`
  (do napisania w kolejnej sesji - w TEJ sesji nie zdazono).

Wciaz calkowicie nieruszone: Go FFI (`get <go:>`/`native {go}`), `fast direct
{backend}` (19 backendow), `ParseError`/numery linii w AST, typ `Set`,
LSP Etap 2b/2c, stuby `.hlib` w `virus install`.

## Aktualizacja sesji 2

* **Stuby `.hlib` w `virus install`** - odkryto, ze `hackerc` JUZ ma
  PELNA, automatyczna obsluge `get <hlib:nazwa>` (`find_hlib_file_path`/
  `hlib_extract_for_import` w `project.hcs` - szuka `.hlib` w
  `<projekt>/hlibs/`, generuje `region [...]` z `manifest.json` gdy brak
  zrodla). Jedyny brakujacy element: `virus build` (zaleznosci `bytes`/
  `bit` konczace sie na `.hlib`) tylko OSTRZEGAL "dopisz recznie" zamiast
  skopiowac plik do `<projekt>/hlibs/<nazwa>.hlib`, gdzie `get
  <hlib:nazwa>` juz by go znalazl. Naprawione (`virus/cmd/build.hcs`) -
  teraz to dziala od razu, bez zadnego recznego kroku.
* **`Set<T>`** dodany (`std::collections::HashSet`) - patrz tura 4 w
  SESSION_LOG.md.
* **Kolorowe diagnostyki + timing w `virus build`** - patrz tura 4 w
  SESSION_LOG.md.

Wciaz calkowicie nieruszone: Go FFI, `fast direct {backend}` (19
backendow), `ParseError`/numery linii w AST, LSP Etap 2b/2c
(hover/definition/completion/rename/formatting), `hack3rc/cmd/main.hcs`.

## Aktualizacja sesji 3

* **`ParseError`** - ZROBIONE (`Parser.errors`, `parse_checked()`,
  wpiete do `check`/`lint`/`build`/`emit-module`/LSP). Real, ale
  ograniczone: parser NADAL robi odzyskiwanie po bledzie (nie
  przerywa w miejscu) - to swiadoma decyzja (parytet z `rustc`), nie
  brak.
* **Numery linii w AST** - CZESCIOWO: bledy PARSERA maja teraz realny
  `line`/`col` (z `Token`). Diagnostyki TYPECHECKA (`check_program`)
  NADAL maja `line=0, col=0` na sztywno w kazdym miejscu
  `typecheck.hcs` - wymaga pola pozycji na KAZDYM wariancie `Expr`/
  `Stmt`, co dotyka tysiace miejsc dopasowania w kompilatorze na raz.
  NAJWIEKSZY pojedynczy pozostaly punkt, odlozony do sesji z dostepem
  do `cargo build` (zbyt ryzykowne bez mozliwosci kompilacji).
* **LSP** - zakresy diagnostyk realne dla bledow PARSERA (wczesniej
  zawsze `(0,0)-(0,1)`), nadal `(0,0)-(0,1)` dla typechecka (patrz
  wyzej - ten sam powod). Bledy parsera TERAZ w ogole trafiaja do LSP
  (wczesniej byly calkowicie gubione przez `lsp_parse_with_direct`
  uzywajace `parse()` zamiast `parse_checked()`).

Wciaz calkowicie nieruszone: Go FFI, `fast direct {backend}` (19
backendow), LSP hover/definition/completion/rename/formatting,
`hack3rc/cmd/main.hcs`.

## Aktualizacja sesji 4 - PIERWSZA realna weryfikacja przez `cargo build`

W tej sesji po raz pierwszy zainstalowano `rustc`/`cargo` i faktycznie
skompilowano `hackerc`, `virus` oraz programy testowe. Wyniki (pelne
szczegoly w SESSION_LOG.md, Tura 7):

* **`hackerc` i `virus` kompiluja sie i dzialaja** - self-hosting
  fixed-point zweryfikowany (dwie generacje transpilacji daja
  identyczny wynik).
* **`Set<T>`, `Dict.keys()/values()`, `ParseError`** - zweryfikowane
  DZIALAJACYM kodem (nie tylko czytaniem zrodla) - skompilowane I
  URUCHOMIONE, poprawne wyniki.
* Znalezione i naprawione 2 realne bledy kompilacji we wczesniejszych
  turach tej sesji (`tt.generic1` -> `tt.generic`, `VERSION.clone()` ->
  `VERSION.to_string()`) - niewidoczne bez `cargo build`.
* **`virus build --wasm`/playground - CZESCIOWO naprawione**: oryginalny
  zglaszany blad (`E0432 unresolved import`) jest naprawiony, ALE
  odkryto GLEBSZY problem: `include <work:...>` nie kanonizuje nazw
  modulow po sciezce pliku, wiec plik zaladowany i BEZPOSREDNIO (przez
  playground) i TRANSYTYWNIE (przez parser.hcs/typecheck.hcs, ktore
  playground tez importuje) dostaje DWIE rozne nazwy modulu = dwa
  niezgodne typy `Program`/`Diagnostic`. Dotyczy WYLACZNIE plikow
  uzywajacych `include <work:...>` na pliki, ktore SAME maja dalsze
  `include` (dzis: tylko `playground`). Najwiekszy, potwierdzony-przez-
  kompilator priorytet na kolejna sesje.

Wciaz calkowicie nieruszone: Go FFI, `fast direct {backend}` (19
backendow), LSP hover/definition/completion/rename/formatting,
`hack3rc/cmd/main.hcs`, numery linii w AST dla typechecka.

## Aktualizacja sesji 5

* **`get <kotlin:>` / `native {kotlin}`** - ZROBIONE i zweryfikowane
  end-to-end (docs/KOTLIN.md).
* **`include <work:...>` duplikacja modulow + `main` ambiguity** -
  ZROBIONE; `playground --library` kompiluje sie bez bledow. Do
  sprawdzenia u Ciebie: sam target `wasm32-unknown-unknown`.
* Wciaz nieruszone: Go FFI, `fast direct {backend}`, LSP
  hover/definition/completion, numery linii w AST dla typechecka,
  `hack3rc/cmd/main.hcs`.

## Aktualizacja sesji 6 (Tura 10) - Go FFI, fast direct (19 backendow), LSP hover/definition/completion, numery linii w typechecku

Wszystkie ponizsze zweryfikowane realna kompilacja (`rustc`/`cargo`
1.91) + w wiekszosci realnym URUCHOMIENIEM (nie tylko `cargo build`).

* **`get <go:>`/`native {go}`** - ZROBIONE, zweryfikowane end-to-end
  (`fmt`, `strings`, dwa bloki, petle). Statyczne linkowanie (Go
  wbudowany w binarke, `go build -buildmode=c-archive`). Moduly
  zewnetrzne (`get <go:sciezka::wersja>`) zaimplementowane, ale
  NIEPRZETESTOWANE (proxy.golang.org zablokowany w srodowisku pracy).
  Patrz docs/GO.md.
* **`fast direct {backend}` (19 backendow)** - ZROBIONE w wersji
  DYNAMICZNEJ (nie statycznej - statyczne linkowanie CPythona/PyPy to
  osobny, duzy projekt). 9 z 19 przetestowanych realnym uruchomieniem
  (numpy, pypy, asyncio, uvloop, trio, multiprocessing, cython,
  pythran, httpx) + poprawny czytelny blad przy braku modulu (numba).
  Patrz docs/FAST_DIRECT.md, "Stan implementacji".
* **LSP hover/definition/completion** - ZROBIONE (Etap 2b/2c),
  zweryfikowane przez bezposredni protokol JSON-RPC (nie w prawdziwym
  edytorze). Implementacja oparta na tekscie (skan linii), nie na
  AST - patrz nizej. Patrz docs/LSP.md.
* **Numery linii w AST dla diagnostyk typechecka** - ZROBIONE w wersji
  PRZYBLIZONEJ: `Checker`/`FnChecker` dostaly pole `source: Str`,
  wszystkie 6 miejsc tworzenia `Diagnostic` w typecheck.hcs licza teraz
  linie przez wyszukanie tekstowe charakterystycznego fragmentu
  (`find_line_of`) zamiast sztywnego `0, 0`. TO NIE SA prawdziwe pozycje
  z parsera (AST nadal ich nie ma - `Expr`/`Stmt` bez zmian) - przy
  wielu wystapieniach tego samego identyfikatora trafia PIERWSZE. Ten
  sam kompromis co `lsp_features.hcs`. Zweryfikowane: `E0001`/`W0002`/
  `W0001` na programie testowym wskazuja poprawne linie.

Jedyny pozostaly punkt z oryginalnej listy: **`hack3rc/cmd/main.hcs`**
(patrz docs/HACK3RC.md) - świadomie nieruszone, wymaga integracji z
Cranelift (crates `cranelift-*`), ktorej NIE dalo sie zweryfikowac w
tej sesji (proxy.golang.org byl zablokowany, kwestia dostepu do
crates.io dla tych konkretnych crate'ow niezweryfikowana, a pisanie
kilkuset linii wywolan API Cranelift bez petli kompiluj-sprawdz ma
wysokie ryzyko bledow).

## Aktualizacja sesji 6 - WSZYSTKIE pozostale punkty zrobione

Wszystkie 5 pozostalych punktow z listy zrobione w Turze 10 (pelne
szczegoly: SESSION_LOG.md):

* **`fast direct {backend}`** - ZROBIONE (dynamiczne, nie statyczne) -
  docs/FAST_DIRECT.md.
* **`get <go:>`/`native {go}`** - ZROBIONE, zweryfikowane end-to-end -
  docs/GO.md.
* **LSP hover/definition/completion** - ZROBIONE (oparte na tekscie,
  nie AST), zweryfikowane przez JSON-RPC - docs/LSP.md.
* **Numery linii w AST dla typechecka** - ZROBIONE jako PRZYBLIZENIE
  (przeszukanie tekstu, nie prawdziwe pozycje z parsera - pelne AST z
  pozycjami NADAL nie istnieje, to swiadomy kompromis). Przy okazji
  naprawiono realna dziure (`args`/`exit`/itd. nie byly w whiteliscie
  typechecka) i ZNALEZIONO (czesciowo naprawiono) regresje wydajnosci
  `hackerc build` na sobie samym (~20s -> 133s, WYMAGA DALSZEGO
  ZBADANIA w kolejnej sesji).
* **`hack3rc/cmd/main.hcs`** - ZROBIONE dla Zakresu 0.0.1 (prosta
  arytmetyka calkowita), zweryfikowane end-to-end przez prawdziwy
  Cranelift JIT - docs/HACK3RC.md (do zaktualizowania o wynik).

**Priorytet na kolejna sesje**: zdiagnozowac i naprawic regresje
wydajnosci `hackerc build` (Turze 10, punkt 4) - prawdopodobnie
nadliniowe skalowanie w `build_project`/`Discovery.resolve` przy
wiekszej liczbie plikow/instrukcji, ujawnione przez wzrost rozmiaru
wlasnego zrodla `hackerc` w tej sesji.

Nieruszone pozostaje: statyczne linkowanie dla `fast direct`, mosty
Rust-native dla backendow bez CPythona, prawdziwe (nie przyblizone)
numery linii w AST, pelny zakres jezyka w `hack3rc` (dzis: tylko
arytmetyka calkowita).

## Tura 11 — `hack3rc` 0.2 (kompilator AOT) i backend JavaScript

* **`hack3rc` — prawdziwy kompilator AOT** (docs/HACK3RC.md): natywny plik
  wykonywalny w kilkadziesiąt ms zamiast crate'a z JIT-em (≈ 2,5 min cargo
  na program). 0.1: `Int/Float/Bool`, funkcje, pętle, `log`. **0.2:**
  `struct`, `enum` z polami + `match`, `impl` (metody), `List<T>`,
  `Option<T>`, napisy (`+`, `==`, `as Str`), głębokie `.clone()`.
  Całość w HackerScript; API Cranelifta w jednym bloku `native {Rust}` w
  `backend.hcs`.
* **Model pamięci**: konserwatywny odśmiecacz mark-sweep w runtime
  (skanowanie stosu i rejestrów, wskaźniki wewnętrzne, deskryptory typów
  do `clone`). Testy w trybie „GC przy każdej alokacji”; pamięć stała
  (≈ 14 MB) przy ≈ 900 MB zaalokowanych w sumie.
* **Cross-kompilacja i wiele architektur**: `--target` (aarch64, riscv64,
  s390x, x86_64 + pełne triple), `--cc` z argumentami, `--runner`. Cały
  zestaw (31 testów) przechodzi na x86_64, aarch64, riscv64 i s390x
  (big-endian) pod QEMU. Obiekty Mach-O i COFF emitowane (bez linkowania).
* **`get <crates:nazwa::wersja+feature>`** (hackerc/cmd/project.hcs):
  features Cargo w składni `get` — odblokowało `all-arch` dla Cranelifta.
  Bootstrap: `hack3rc` buduje się teraz hackerc z tego repozytorium.
* **Weryfikacja różnicowa z `hackerc`**: 14 programów daje bajt w bajt to
  samo wyjście pod `hack3rc` i `hackerc`; plus testy matematyki, kodów
  wyjścia, paniki, stdin i błędów kompilacji — `hack3rc/tests/`.
* **Naprawiony błąd parsera `hackerc`**: pętle `while not
  self.check(Close, ...)` nie sprawdzały końca pliku, więc niedomknięty
  nawias na końcu pliku zapętlał parser w nieskończoność (OOM-kill).
* **Znaleziony, NIEnaprawiony błąd generatora Rust w `hackerc`**:
  `lista[i].pole += x` i `lista[i][j] += x` generują `lista[i].clone()...` i
  po cichu gubią zapis (modyfikują kopię). `hack3rc` ma poprawną semantykę
  (test `inplace`); do naprawy w `hackerc/cmd/codegen.hcs`.
* **`use <lang:javascript>`** (docs/JAVASCRIPT.md): backend
  `hackerc/cmd/javascript_backend.hcs` + runtime `js_runtime.hcs`;
  struct/enum/impl/match/Option/Result/`?`/async/await/spawn/kanały/Dict/
  listy; podkomenda `hackerc js`; testy `hackerc/tests/javascript/`.
* **`use <mode:website>`** (docs/WEBSITE.md): `hackerc build`/`hackerc
  website` generuje `index.html` + `app.js` z mostkiem DOM (zdarzenia,
  `http_get`, `localStorage`…); zweryfikowane w jsdom.
* **Następne kroki**: `Dict`/`Result`/`?` i metody `Str` w `hack3rc`,
  inline'owanie dostępu do list i szybsza ścieżka alokacji, `native
  {JavaScript}` w backendzie JS, `Int` jako BigInt (opcjonalnie), prawdziwe
  pozycje w AST dla diagnostyk, test strony w prawdziwej przeglądarce.
