# `lsp` — Language Server dla HackerScript

Status: **zaimplementowane w `hackerc lsp`** (diagnostyka, hover, definition,
completion, formatowanie, a od 0.6 także rename, references, documentSymbol —
patrz sekcje niżej). Dokument zaczyna się od pierwotnej specyfikacji
(Faza 1), którą zachowano jako opis założeń. `bit lsp` należy do `bit`
(poza tym repozytorium). Lista otwartych braków: `docs/ROADMAP.md`.

## Dlaczego w dwóch miejscach

* `hackerc lsp` — serwer LSP dla pojedynczego pliku/bez kontekstu
  projektu (identyczny model jak `hackerc check <plik>` dziś: działa
  nawet poza katalogiem z `Bit.hk`).
* `bit lsp` — cienka nakładka, która woła dokładnie ten sam serwer
  co `hackerc lsp` (współdzielony kod przez
  `include <work:hackerc::lsp>`, ten sam wzorzec co już istniejące
  `bit`), ale z pełnym kontekstem workspace
  (`Bit.hk`, `cache/`, zależności `get <work:...>`) — analogicznie
  do tego, jak edytory zwykle uruchamiają `rust-analyzer` per-projekt,
  nie per-plik.

Konfiguracja edytora (VS Code / Neovim / dowolny klient LSP): domyślnie
`bit lsp` w katalogu projektu; `hackerc lsp <plik>` jako fallback
dla plików `.hcs` otwartych poza jakimkolwiek `Bit.hk`.

## Transport i protokół

Standardowy **Language Server Protocol** (JSON-RPC 2.0) po **stdio** —
ten sam model transportu co `rust-analyzer`/`pyright`. Nagłówki
`Content-Length` + `\r\n\r\n` + treść JSON, zgodnie ze specyfikacją
LSP 3.17.

## Zakres funkcji — plan wdrożenia etapami

### Etap 2a (pierwszy działający serwer)
* `initialize` / `initialized` / `shutdown` / `exit`
* `textDocument/didOpen`, `didChange` (pełna synchronizacja treści —
  `TextDocumentSyncKind.Full`, najprostszy poprawny wariant na start;
  przyrostowa synchronizacja to możliwa optymalizacja później)
* `textDocument/publishDiagnostics` — **bezpośrednio ponowne użycie**
  `hackerc/cmd/diagnostics.hcs` + `typecheck.hcs`: ten sam pipeline co
  `hackerc check`, tylko wynik mapowany na `Diagnostic[]` LSP
  (`range`, `severity`, `code`, `message`) zamiast tekstu na stdout.
  To jest krok zależny od braku numerów linii w AST (patrz
  `docs/ROADMAP.md`, sekcja 1) — na Etapie 2a `range` będzie
  przybliżany (cały plik albo cała linia z komunikatu parsera, jeśli
  jest dostępna), z dokładnością rosnącą wraz z pracą nad numerami
  linii w AST.

### Etap 2b (podstawowa nawigacja)
* `textDocument/hover` — sygnatura funkcji/typ zmiennej pod kursorem,
  z tych samych danych co `typeinfer.hcs` już liczy dla codegen
* `textDocument/definition` — skok do deklaracji (`fun`/`struct`/
  `enum`/`let`) w obrębie pliku, potem w obrębie `include`/`get`
* `textDocument/documentSymbol` — lista `fun`/`struct`/`enum`/`impl`
  w pliku (structure outline edytora)

### Etap 2c (rozszerzone)
* `textDocument/completion` — nazwy z aktualnego zasięgu + funkcje
  `std`/`core` po `get <...> import <...>`
* `textDocument/references`
* `textDocument/rename` (zrobione w 0.5; działa na skanie tekstu, nie na AST — patrz ROADMAP)
* `textDocument/formatting` — cienka nakładka na już istniejący
  `hackerc/cmd/formatter.hcs` (parytet z `bit fmt`)
* `textDocument/codeAction` — "quick fix" korzystający wprost z bazy
  `bit repair` (plan po stronie `bit`) — sugestia
  z `RepairDiagnostic.suggestion` jako codeAction tam, gdzie to
  możliwe zamienić automatycznie

## Architektura w repo (planowana)

* `hackerc/cmd/lsp.hcs` — nowy plik: pętla stdio, (de)serializacja
  JSON-RPC (przez `get <std:json>`, już istniejący), dispatch metod,
  stan otwartych dokumentów (`Dict<Str, Str>` ścieżka→treść — tu
  akurat dzisiejszy brak iteracji po `Dict`, sekcja 1 ROADMAP, nie
  przeszkadza, bo dostęp jest zawsze po znanym kluczu-ścieżce).
* `hackerc/cmd/main.hcs` — nowa gałąź `cmd == "lsp"` obok istniejących
  (`build`/`check`/`fmt`/...), wołająca `lsp_run()` z `lsp.hcs`.
* `bit lsp` (menedżer pakietów `bit`, osobny projekt) wywołuje `hackerc lsp`
  w korzeniu projektu — serwer żyje w `hackerc/cmd/lsp.hcs`, a `bit`
  tylko ustala korzeń projektu i przekazuje stdio.

## Czego ten dokument świadomie NIE rozstrzyga jeszcze

* Czy `completion` w Etapie 2c korzysta z pełnego re-typecheck przy
  każdym znaku, czy z debounce/inkrementalnego cache — decyzja
  wydajnościowa do zmierzenia dopiero na działającym Etapie 2a.
* Publikacja jako osobny plik binarny vs. podkomenda — na razie
  podkomenda (`hackerc lsp`/`bit lsp`), zgodnie z tym, jak działają
  `rust-analyzer` (osobny) vs. `gopls` (osobny) vs. `cargo`
  wbudowane narzędzia — HackerScript idzie tu bliżej modelu
  "podkomenda", żeby nie mnożyć artefaktów do zainstalowania.

## Etap 2b/2c - hover, definition, completion

Status: **zaimplementowane, przetestowane przez surowy JSON-RPC po
stdio** (skrypt Python otwierający `hackerc lsp` jako proces i
wysyłający `initialize`/`didOpen`/`hover`/`definition`/`completion`).

Implementacja jest **oparta na tekście** (`hackerc/cmd/lsp_features.hcs`),
nie na AST z pozycjami (patrz ROADMAP, "numery linii w AST" — wciąż
nierozwiązane). Deklaracje (`fun`/`struct`/`enum`/`const`/`let`) są
wyszukiwane jako pierwsza pasująca linia zaczynająca się (po wcięciu,
opcjonalnym `pub `) od jednego z tych słów kluczowych + nazwy.

- **hover** — sygnatura `fun` (bez `[`) albo pełne ciało `struct`/`enum`
  (do `]`, max 30 linii); dla wbudowanych (`log`, `dict`, `now_ms`, ...)
  krótki opis; dla słów kluczowych — nazwa.
- **definition** — pierwsza deklaracja o pasującej nazwie w otwartym
  dokumencie; od 0.6, gdy jej tam nie ma, także w plikach dołączonych przez
  `include <nazwa>` leżących obok (czytanych z dysku). Brak wsparcia zakresów
  — jeśli dwie funkcje w różnych `impl` mają tę samą nazwę, trafia pierwsza;
  `get <work:...>` i `std` nie są przeszukiwane.
- **completion** — słowa kluczowe + wbudowane + wszystkie deklaracje z
  dokumentu, bez filtrowania po prefiksie (klient filtruje sam, zgodnie
  ze standardowym zachowaniem LSP).

Zweryfikowane: `initialize` deklaruje `hoverProvider`/`definitionProvider`/
`completionProvider`; dla programu z `fun add(a,b)` i wywołaniem
`add(1,2)` — hover zwraca poprawną sygnaturę, definition wskazuje
dokładną linię/kolumnę `add`, completion (52 pozycje) zawiera i `add`,
i wbudowane `log`.

**Nieprzetestowane w prawdziwym edytorze** (VS Code/Neovim) — tylko
protokół JSON-RPC bezpośrednio.

## 0.6 — rename, references, documentSymbol

Status: **zaimplementowane, sprawdzone surowym JSON-RPC po stdio**
(skrypt `scripts/lsp_check.py`, uruchamiany w `scripts/verify.sh`; to nadal NIE jest test w
prawdziwym edytorze).

Implementacja nadal skanuje tekst (`lsp_features.hcs`): słowo pod kursorem
jest dopasowywane jako CAŁE słowo w każdej linii, z pominięciem literałów
napisów i komentarzy `!!`/`!!!`.

- **rename** — `WorkspaceEdit` z `changes` dla otwartego dokumentu. Zwraca
  `null`, gdy pod kursorem nie ma słowa albo nowa nazwa nie jest poprawnym
  identyfikatorem (w tym gdy jest słowem kluczowym). **Bez analizy zasięgu:**
  dwie niezależne zmienne o tej samej nazwie w różnych funkcjach zostaną
  zmienione razem. Nie dotyka innych plików.
- **references** — lista `Location` wszystkich wystąpień słowa w dokumencie.
- **documentSymbol** — płaski spis `fun`/`struct`/`enum`/`const` (Function=12,
  Struct=23, Enum=10, Constant=14); zmienne `let` są pomijane.

Przy `initialize` serwer deklaruje `renameProvider`, `referencesProvider` i
`documentSymbolProvider`. Wcześniejsze wzmianki o `rename` jako „zrobionym w
0.5” były nieścisłe: w kodzie tej metody wtedy nie było.

**Nieprzetestowane w prawdziwym edytorze** — tak jak reszta serwera.

## 0.6 — zakresy (scope-aware definition / hover / rename / references)

Dotąd `rename` i `references` zmieniały **każde** wystąpienie słowa w dokumencie, a `definition`
brało pierwszą deklarację o tej nazwie. Od 0.6 serwer rozpoznaje zakresy — **nadal na skanie
tekstu, nie na AST** (AST nie niesie pozycji linia/kolumna; dodanie ich wymaga zmiany w całym
parserze, więc to jest świadomie poza zakresem).

Jak to działa: blok `fun ... [ ... ]` albo `impl Typ [ ... ]` to zakres; nagłówek kończy się `[`,
a blok kończy pierwsza linia zaczynająca się od `]` w tym samym wcięciu (kod po `hackerc fmt`
spełnia to zawsze). Dzięki temu:

* **Zmienna lokalna / parametr** (`let x`, `for x in`, `for k, x in`, wzorzec `Wariant(x) -> [`,
  parametr funkcji) jest szukana, zmieniana i wskazywana **tylko w swojej funkcji**; zmienna o tej
  samej nazwie w innej funkcji zostaje nietknięta.
* **Nazwa globalna** (funkcja, struktura) obejmuje cały plik, z pominięciem funkcji, które mają
  własną zmienną o tej nazwie (cieniowanie). `definition` szuka deklaracji tylko na poziomie
  pliku (nie łapie już cudzego `let`).
* **Metoda** `obiekt.nazwa(...)` jest przypisywana do typu odbiorcy: `self` → typ z otaczającego
  `impl`; zmienna → typ z `let v = Typ(...)`, `let v: Typ` albo parametru `v: Typ`. Dwie metody o
  tej samej nazwie w różnych `impl` są rozróżniane (`a.value()` → `impl A`, `b.value()` → `impl B`).

Ograniczenia: brak wnioskowania typów (wynik funkcji, pole struktury, element listy) — gdy typ
odbiorcy jest nieznany, działamy jak dawniej (wszystkie metody o tej nazwie); wzorce zagnieżdżone w
`match` (`Some(Foo(x))`) nie są rozpoznawane jako deklaracje; `rename`/`references` obejmują nadal
tylko otwarty dokument.

Naprawiony przy okazji błąd: diagnostyka parsera (`[hackerc-self] parser: ...`) trafiała na
**stdout**, co psuło ramkowanie protokołu, gdy edytowany dokument miał błąd składni; teraz idzie na
stderr.
