# `lsp` — Language Server dla HackerScript

Status: **projekt (Faza 1 — specyfikacja)**, jeszcze nie
zaimplementowane w kodzie. Ten dokument opisuje kształt, w jakim
`lsp` zostanie dodane jako podkomenda zarówno `hackerc lsp`, jak i
`virus lsp` w kolejnej rundzie pracy (Faza 2, patrz
`docs/ROADMAP.md`).

## Dlaczego w dwóch miejscach

* `hackerc lsp` — serwer LSP dla pojedynczego pliku/bez kontekstu
  projektu (identyczny model jak `hackerc check <plik>` dziś: działa
  nawet poza katalogiem z `Virus.hk`).
* `virus lsp` — cienka nakładka, która woła dokładnie ten sam serwer
  co `hackerc lsp` (współdzielony kod przez
  `include <work:hackerc::lsp>`, ten sam wzorzec co już istniejące
  `virus/cmd/hackerc_bridge.hcs`), ale z pełnym kontekstem workspace
  (`Virus.hk`, `cache/`, zależności `get <work:...>`) — analogicznie
  do tego, jak edytory zwykle uruchamiają `rust-analyzer` per-projekt,
  nie per-plik.

Konfiguracja edytora (VS Code / Neovim / dowolny klient LSP): domyślnie
`virus lsp` w katalogu projektu; `hackerc lsp <plik>` jako fallback
dla plików `.hcs` otwartych poza jakimkolwiek `Virus.hk`.

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
* `textDocument/rename`
* `textDocument/formatting` — cienka nakładka na już istniejący
  `hackerc/cmd/formatter.hcs` (parytet z `virus fmt`)
* `textDocument/codeAction` — "quick fix" korzystający wprost z bazy
  `virus repair` (`docs/VIRUS.md`, "Plan: baza diagnostyk") — sugestia
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
* `virus/cmd/lsp.hcs` — nowy plik, analogiczny do
  `hackerc_bridge.hcs`: ustala `root` przez `require_project_root()`
  (już istniejące), potem woła `hackerc`owe `lsp_run()` z tym
  kontekstem przez `include <work:hackerc::lsp>` (ten sam mechanizm co
  `playground/`, patrz `docs/SYNTAX.md`, sekcja "`include
  <work:...>`" — realne statyczne linkowanie, nie duplikacja pliku).
* `virus/cmd/main.hcs` — nowa gałąź `cmd == "lsp"`, wpis w
  `cmd_usage()`.

## Czego ten dokument świadomie NIE rozstrzyga jeszcze

* Czy `completion` w Etapie 2c korzysta z pełnego re-typecheck przy
  każdym znaku, czy z debounce/inkrementalnego cache — decyzja
  wydajnościowa do zmierzenia dopiero na działającym Etapie 2a.
* Publikacja jako osobny plik binarny vs. podkomenda — na razie
  podkomenda (`hackerc lsp`/`virus lsp`), zgodnie z tym, jak działają
  `rust-analyzer` (osobny) vs. `gopls` (osobny) vs. `cargo`
  wbudowane narzędzia — HackerScript idzie tu bliżej modelu
  "podkomenda", żeby nie mnożyć artefaktów do zainstalowania.
