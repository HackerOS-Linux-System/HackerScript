# Roadmap HackerScript - otwarte punkty

Jedna, aktualna lista tego, co jeszcze nie jest zrobione albo nie jest zweryfikowane. To NIE jest
dziennik zmian. Numer wydania w `Bit.hk` pozostaje **0.6**; funkcje tej rundy są w komentarzach
źródeł oznaczone „0.7”.

Oznaczenia: **[brak]** nie zaimplementowane · **[niezweryfikowane]** zaimplementowane, ale nie
sprawdzone realnym uruchomieniem · **[częściowe]** działa w ograniczonym zakresie.

## Jak weryfikować

Kompilator buduje się ze źródeł łańcuchem stage0 (wydanie `hackerc` z GitHub Releases, najnowsze
dostępne to v0.5) → `hackerc` → `hackerc` (samohostowanie; wynik stage N i N+1 to identyczny Rust).

* `tests/run_lang.sh` - programy z `tests/lang/*` (krotki, f-stringi, traity, domknięcia, `defer`/`Drop`,
  biblioteka standardowa, sandbox): budowa, `cargo build`, porównanie wyjścia (`HACKERC=ścieżka`).
* `tests/website/run.sh` - `hackerc website --ssr` w jsdom (wymaga Node i jsdom).
* Wektory testowe z RFC: HKDF (5869), scrypt (7914), Ed25519 (8032).

## Zrobione i zweryfikowane w tej rundzie (skrót)

Samohostowanie (naprawa 3 brakujących adnotacji typów w źródłach), krotki + destrukturyzacja,
interpolacja `f"..."`, jawne argumenty typu `f<T>(..)`, ograniczenia generyków, `trait` +
`impl Trait for`, domknięcia i typ `Fn`, `defer`, `Drop`, konstruktor `hks_new` (koniec konfliktu z
`new`), aliasy nazw bibliotek crate'ów (`md-5`→`md5`), `$ ... $ allow_unsandboxed`, sandbox:
PID/IPC/UTS, osobny `/proc`, RO dla wszystkich montowań, seccomp także dla `bwrap`, `HKS_SANDBOX_SKIP`,
`hackerc website --ssr`, `std:cybersecurity::kdf` (HKDF, scrypt, Argon2id), `::sign` (Ed25519),
`std:process` (`run_capture_timeout`, `run_capture_bytes`, `proc_spawn_group`, `proc_kill_group`),
`std:term` (`term_parse_key`, `term_read_key` - sprawdzone przez pseudo-terminal), `std:http`
(strumieniowanie `http_stream_*`), LSP `rename`/`references` w workspace.

## Język i `hackerc`

* [częściowe] Nowe konstrukcje: formatter (`hackerc fmt`) rozwija `f"..."` do konkatenacji
  (`"" + ... + (x as Str)`) zamiast zachować zapis; `let (a, b) = ...` wypisuje jako `let __tupN = ...`
  + elementy. Backendy JS (tablice, `t.0`→`t[0]`, domknięcia jednowyrażeniowe) i Kotlin (`Pair`/`Triple`,
  jednowyrażeniowe lambdy) kompilują się, ale nie mają testów uruchomieniowych; domknięcia blokowe i
  `trait`/`Drop`/`defer` w JS/Kotlin nie są obsługiwane.
* [brak] Metody traitów biorą tylko `&self`; typ `Fn(...)` nie działa w polach struktur; `Drop` wykonuje się
  dla każdej kopii (kompilator klonuje wartości).
* [brak] Pozycje (linia/kolumna) dla wyrażeń i identyfikatorów w AST. Pozycje mają tylko instrukcje, więc
  diagnostyki wywołań (`E0001`/`W0002`) nadal opierają się na heurystyce „k-te wystąpienie `nazwa(`”.
  Instrukcje wewnątrz domknięć są poza tabelą pozycji (typecheck wyłącza wtedy dokładne pozycje).
* [brak] Podział dużych plików: `codegen.hcs` (~5600 linii) i `project.hcs` (~2800) wymagają wyciągnięcia
  `struct CodeGen` do pliku bazowego (brak cyklicznych `include`).
* [brak] Warunkowa kompilacja `use <os:...>` na poziomie pliku (jest na poziomie projektu).
* [niezweryfikowane] Czas samohostowania po dodaniu cache (wcześniej ~20 s → ~133 s); nie mierzono w tej rundzie.
* [brak] Interpolacja: format (`{x:>5}`), cudzysłów wewnątrz `{...}`.
* [brak] Wnioskowanie typów dla zmiennych z `match`-owych wzorców (np. `Ok(h) -> h.slice(..)` wymaga
  `let hs: Str = h.clone()`); to samo ograniczenie dotyka `hackerc`.

## Backendy i FFI

* [brak] Kotlin: `enum`, `impl`, `?`/`Result`, `native`.
* [częściowe] JavaScript: `native` tylko jako instrukcja, `Int` = liczba zmiennoprzecinkowa.
* [niezweryfikowane] Go: moduły zewnętrzne nigdy nie uruchomione; bloki bez parametrów i wyników.
* [brak] `fast direct`: statyczne linkowanie CPythona/PyPy; nieprzetestowane `pypy`, `cupy`, `vaex`;
  `direct` jako instrukcja nie zwraca wartości.
* [brak] Java/JVM: możliwa druga instancja JVM przy `native {java}` w wielu plikach.
* [częściowe] Tryb `website`: **są** trasy zagnieżdżone (`route_layout`), tryb `history` i sygnały, a od
  0.7 prerendering `--ssr`. Brak: pełnej hydratacji (klient renderuje od nowa), kluczy w `bind_list`,
  prerenderingu tras z parametrami, testów w prawdziwej przeglądarce.
* [niezweryfikowane] Playground wasm32 w przeglądarce.
* [brak] Wiele wersji: cross-file typecheck; `--version-map` nie jest przekazywany przy bibliotekach workspace.

## Sandbox `$ ... $`

* [częściowe] Dziś: PID/IPC/UTS, osobny `/proc`, RO dla montowań, seccomp (denylista) na każdej
  metodzie. Brak: allowlisty syscalli, Windows/macOS; `/dev` zostaje zapisywalny.
* [niezweryfikowane] Maszyna bez user namespaces na poziomie jądra (zasymulowana tylko przez
  `HKS_SANDBOX_SKIP`).

## LSP

* [brak] Analiza na AST (dziś skan tekstu); wnioskowanie typu odbiorcy z wyniku funkcji/pola.
* [częściowe] `rename`/`references`: od 0.7 w całym workspace dla symboli najwyższego poziomu
  (sprawdzone przez protokół LSP, nie w edytorze); w innych plikach dopasowanie jest tekstowe, a
  zmienne lokalne i metody nadal tylko w pliku.
* [częściowe] Formatowanie tylko dla kodu bez błędów składni; brak testów w prawdziwym edytorze.
  Serwer nie zna jeszcze składni 0.7 (`trait`, `defer`, `fn(...)`, `f"..."`) w podpowiedziach.

## Biblioteka standardowa

* [brak] `std:term`: Windows (nowe funkcje klawiszy używają `poll(2)`, jak reszta modułu `libc`).
* [częściowe] `std:http`: jest strumieniowanie odpowiedzi (`http_stream_*`); brak strumieniowania
  *żądania* i własnych nagłówków w `http_stream_open`.
* [częściowe] `std:process`: grupy procesów i limit czasu tylko na Uniksie.
* [brak] `std:cybersecurity`: algorytmy poza podanymi (np. ECDSA, X25519, AEAD ze strumieniowaniem).
* [brak] `std:hk`: `bit` nadal edytuje `Bit.hk` własnym kodem (nie przez `std:hk`).

## Testy

* [częściowe] `tests/run_lang.sh` i `tests/website/run.sh` (patrz wyżej). Brak: testów diagnostyk dla
  nowych konstrukcji, testów JS/Kotlin dla krotek i domknięć, testów LSP.
