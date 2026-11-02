# Roadmap HackerScript 0.6 - otwarte punkty

Jedna, aktualna lista tego, co jeszcze nie jest zrobione albo nie jest
zweryfikowane. To NIE jest dziennik zmian: rzeczy zrobione sa opisane w
README (sekcja "Historia wersji") i w dokumentach poszczegolnych funkcji.
Numeracja wersji pozostaje **0.6**.

Oznaczenia: **[brak]** nie zaimplementowane · **[niezweryfikowane]**
zaimplementowane, ale nie sprawdzone realnym uruchomieniem · **[czesciowe]**
dziala w ograniczonym zakresie.

## Weryfikacja repozytorium

Automatycznie sprawdza `scripts/verify.sh` (uruchamiany w CI): `hackerc
check` na wszystkich `.hcs`, budowa i uruchomienie `docs/std-examples/*`
z porownaniem wyjscia, przyklady `hack3rc` (gdy ustawione `HACK3RC`) i -
w CI - samohostowanie.

* [czesciowe] Zestaw testow: `tests/run.sh` (diagnostyki `tests/errors/*` z
  oczekiwaniem `!! expect: KOD@LINIA` + budowa i uruchomienie programow z
  `tests/cases`, `docs/hack3rc-examples`, `docs/std-examples`) oraz
  `hack3rc/tests/run.sh` (te same programy przez hack3rc = test roznicowy +
  `hack3rc/tests/errors`). Brak jeszcze: testow bledow o kodach `H0xxx`
  (hack3rc), programow struct/enum/match z `Option`/`Result` i `?`, GC pod
  `HKS_GC_STRESS=1`.
* [brak] Testy serwera LSP w prawdziwym edytorze (dotychczas tylko surowy
  JSON-RPC).

## Kompilator `hackerc`

* [czesciowe] Pozycje w AST: parser zapisuje w `Program` tabele pozycji WSZYSTKICH
  instrukcji (`lines`/`cols`/`tags`, kolejnosc pre-order, obok drzewa - warianty
  `Stmt` bez zmian). Typechecker uzywa jej dla naglowkow funkcji/metod i `let`
  (E0002, E0005, E0011, W0001; kolumna 1-based), ale tylko gdy
  `positions_consistent` potwierdzi zgodnosc z drzewem; inaczej wraca do
  wyszukiwania tekstowego za kursorem. `HKS_DEBUG_POSITIONS=1` + `scripts/verify.sh`
  wymagaja braku `W0900` na kazdym pliku repozytorium. Nadal: wywolania
  (E0001/W0002) to k-te wystapienie `nazwa(` w funkcji (heurystyka), pozycje
  wyrazen i identyfikatorow nie sa w AST, E0003 szuka pierwszego `get <zrodlo:`,
  a LSP (`rename`/`references`/`hover`) dziala na skanie tekstu.
* [x] (0.6) Linie po wielolinijkowym bloku `direct`/`fast direct`/`native`/`$ ... $`
  nie przesuwaja sie juz (ekstrakcja dopisuje newline'y), a ekstraktory
  `direct`/`fast direct`/`native` pomijaja napisy i komentarze (wczesniej napis
  zawierajacy slowo kluczowe i nawias, np. w `formatter.hcs`, wycinal kod).
* [czesciowe] Funkcje asocjacyjne (`Typ.nazwa(args)` -> `Typ::nazwa`) i wartosci
  domyslne parametrow funkcji dzialaja w `hackerc` od 0.6 (sprawdzone testem
  roznicowym z `hack3rc`). Znany konflikt: nazwa `new` jest zajeta przez
  automatyczny konstruktor struktury (rustc: E0592) - uzyj innej nazwy.
* [czesciowe] Generyki: `let w: Wrapper<Int> = Wrapper(5)` i wnioskowanie bez
  adnotacji dzialaja (typecheck + `let` bez zbednego typu), funkcje i `impl`
  generyczne dostaja ograniczenie `T: Clone` (indeksowanie elementow `T`).
  Test: `tests/cases/generics.hcs`. Brak: ograniczen trait'ow zdefiniowanych
  przez uzytkownika, jawnych argumentow typu przy wywolaniu (`first<Int>(..)`).
* [brak] Typ krotki (`(Int, Str)`, `let (a, b) = ...`) - nowy wariant `Expr` i
  `TypeRef` we wszystkich backendach.
* [czesciowe] Podzial duzych plikow: z `codegen.hcs` wydzielono emiter sandboxa
  (`codegen_sandbox.hcs`, czysta funkcja zwracajaca linie Rusta - nie wymaga
  struktury `CodeGen`). Reszta (~5350 linii) i `project.hcs` (~2800) bez zmian.
  Dalszy podzial wymaga najpierw wyciagniecia `struct CodeGen` i wspolnych
  funkcji do pliku bazowego, bo `include` nie obsluguje cyklicznych zaleznosci.
* [niezweryfikowane] Czas samohostowania po dodaniu cache budowania -
  zmierzony regres (~20 s -> ~133 s) mogl nie zniknac na wszystkich
  maszynach.
* [brak] Warunkowa kompilacja `use <os:...>` per plik (dzis tylko poziom
  projektu).
* [czesciowe] RAII/domkniecia: w jezyku nadal brak. Dla trybu raw terminala
  problem rozwiazany w bibliotece (`std:term`: termios + automatyczne
  przywracanie przy wyjsciu, panice i SIGINT/SIGTERM/SIGHUP; test w PTY
  `docs/std-examples/term_raw.pty.py`). Ogolny RAII (`defer`/`Drop`) - otwarte.
* [czesciowe] Crate z myslnikiem w nazwie, ktorego biblioteka nazywa sie
  inaczej (np. `md-5` -> `md5`): `get <crates:...>` emituje `use md_5::*`
  i nie kompiluje sie. Obejscie: uzywac crate'ow, ktorych nazwa pakietu
  = nazwa biblioteki (`md5`).
* [brak] Skladnia zgody na uruchomienie bez piaskownicy w kodzie zrodlowym
  (`$ polecenie $ allow_unsandboxed`) - dzis tylko zmienna `HKS_SANDBOX`.

## Sandbox `$ ... $`

Patrz `docs/SANDBOX.md`.

* [czesciowe] Kaskada metod izolacji sprawdzona recznie: jako root
  (izolacja dziala, zapis do `/` zablokowany, widoczny tylko `lo`), jako
  uzytkownik bez uprawnien (user namespace) oraz przy wymuszonym bledzie
  `unshare` (tryby `strict` - panic bez wykonania polecenia, `best-effort`
  i `off` - ostrzezenie i uruchomienie). **Nie sprawdzono** poziomu 3
  (`bwrap` nie byl zainstalowany) ani maszyny z wylaczonymi user namespaces
  na poziomie jadra. Zautomatyzowany test: `tests/sandbox.sh` (w `scripts/verify.sh`).
* [czesciowe] Remount RO obejmuje tylko `/` (nie osobne montowania).
* [x] (0.6) Filtr seccomp (denylista ~27 wywolan + kontrola architektury i x32,
  `no_new_privs`) dla metod 1 i 2 oraz limity zasobow `HKS_SANDBOX_CPU`/
  `_MEM_MB`/`_NPROC`/`_FSIZE_MB`/`_NOFILE`; test `tests/sandbox.sh` (pomija sie
  bez izolacji w srodowisku). Szczegoly: `docs/SANDBOX.md`.
* [brak] Izolacja PID/IPC, filtr seccomp dla `bwrap`, allowlista zamiast
  denylisty, Windows/macOS.

## FFI i backendy

* [brak] `fast direct {backend}`: statyczne linkowanie CPythona/PyPy.
  Dzis wylacznie wersja dynamiczna.
* [czesciowe] `fast direct`: test dymny (`scripts/verify_fast_direct.sh`) przeszedl dla
  17 z 20 backendow. Niesprawdzone: `pypy` (brak binarki), `cupy` (GPU), `vaex`
  (nie udalo sie zainstalowac). Test dymny sprawdza tylko, ze backend sie uruchamia.
* [niezweryfikowane] Moduly zewnetrzne Go (`get <go:...::wersja>`): w srodowisku
  pracy nie bylo `go` ani dostepu do `proxy.golang.org`, wiec nie dalo sie ich uruchomic.
* [czesciowe] Backend JavaScript: `native {JavaScript}` i automatyczny `await`
  na `send`/`recv` (w kontekscie `async`) dzialaja (przyklady pod Node). Brak:
  `native` jako wyrazenie (tylko instrukcja), inne silniki `native`.
* [czesciowe] Tryb `website`: routing po `#hash` (`route`, `route_param`,
  `navigate`...) dziala w jsdom. Brak: routingu po zwyklych sciezkach
  (history API), SSR, zagniezdzonych tras; nie sprawdzono w prawdziwej
  przegladarce.
* [czesciowe] Cel `wasm32` dla playgroundu: buduje sie i `check_source` dziala
  w Node (`scripts/verify_wasm.sh`) - po naprawie `tokio` (`rt-multi-thread`
  nie istnieje na wasm32) i runtime async. Strona webowa playgroundu nie jest
  czescia repozytorium; nie sprawdzono w przegladarce.
* [czesciowe] Backend Kotlin (`use <lang:kotlin>`): zakres opisany w
  `docs/KOTLIN.md`; przyklady kompiluja sie `kotlinc` i uruchamiaja. Brak
  `enum`, `impl`, `?`/`Result`, `native`. Kanaly maja typ `HksChan<Any?>`.
* [czesciowe] Inline `native {C++}`: `hackerc build` sprawdza teraz z gory
  obecnosc kompilatora C++ (`CXX`, `c++`, `g++`, `clang++`, `cl`;
  pominiecie: `HKS_SKIP_CXX_CHECK=1`). Sprawdzone: z kompilatorem, bez
  kompilatora (blad, kod 1), z `HKS_SKIP_CXX_CHECK=1` i z nieistniejacym
  `CXX`. Pelny przebieg z `bit build` nie byl sprawdzany.
* [czesciowe] `hks_get_jvm()` jest emitowane raz na plik `.hcs`; bloki
  `native {java}` w roznych plikach tego samego projektu moga probowac
  utworzyc druga JVM.

## Serwer LSP (`hackerc lsp`)

Patrz `docs/LSP.md`.

* [czesciowe] `hover`, `definition`, `rename`, `references` rozpoznaja zakresy
  (0.6): zmienna lokalna/parametr tylko w swojej funkcji, metoda wg typu
  odbiorcy (`self`, `let v = Typ(...)`, `let v: Typ`, parametr `v: Typ`) -
  ale nadal na skanie tekstu, NIE na AST (AST nie ma pozycji linia/kolumna;
  dodanie ich to zmiana w calym parserze). Typ odbiorcy z wywolania funkcji
  lub pola nie jest wnioskowany (wtedy dzialanie jak dawniej: wszystkie
  metody o tej nazwie). `completion` bez zmian.
* [czesciowe] `rename` i `references` obejmuja otwarty dokument, nie caly
  workspace.
* [czesciowe] Formatowanie dziala tylko dla dokumentow bez bledow skladni.
* [niezweryfikowane] Brak testow w prawdziwym edytorze.

## `hack3rc` (kompilator AOT)

Patrz `docs/HACK3RC.md`. Obslugiwane: `Result<T, E>` i `?`, `Option`,
struct/enum/impl, listy. **Nieobslugiwane** (czytelny blad `H0xxx`):

* [brak] `Dict` (w tym `for k, v in d.items()`, kod H0151) - wymaga tablicy
  mieszajacej w runtime C, typow `Dict<K, V>` w loweringu i integracji z GC
* [brak] interpolacja napisow
* [brak] `async`/kanaly, `native`/`direct`, `get`/`include`
* [brak] wartosci domyslne parametrow METOD (dla funkcji dziala od 0.6)
* [brak] `for` po czymkolwiek poza lista
* [brak] `log()` dla struct/enum
* [czesciowe] `List.sort()` dla `List<Int>`, `List<Float>`, `List<Str>` (0.6);
  inne typy elementow -> H0475
* [czesciowe] Rozbieznosc z `hackerc`: `hack3rc` przyjmuje metody `Str`
  (`to_upper`, `to_lower`, `contains`...), dla ktorych `hackerc` generuje
  Rust, ktory sie nie kompiluje (sprawdzone dla `to_upper()` i
  `contains("...")`). Zalozenie "program przyjety przez hack3rc dziala tak
  samo pod hackerc" nie zachodzi dla tych metod - w przenosnym kodzie
  uzywaj funkcji z `std:string`.
* [czesciowe] GC jednowatkowy; zbyt gleboka rekurencja konczy sie SIGSEGV
* [brak] inline'owanie dostepu do list i szybsza sciezka alokacji
  (glowne zrodlo roznicy wydajnosci wzgledem Rusta)
* [czesciowe] Cele: obiekty Mach-O (macOS) i COFF (Windows) sa emitowane
  i sprawdzany jest tylko naglowek - **nie sa linkowane ani uruchamiane**.
  Runtime C zaklada POSIX/libc.
* [czesciowe] Przyklady uruchamiane pod QEMU (aarch64, riscv64, s390x)
  byly sprawdzone na wczesniejszej wersji zestawu, nie w tym repozytorium.

## Biblioteka standardowa

* [czesciowe] `std:http`: brak klienta strumieniujacego (cale cialo naraz).
* [czesciowe] `std:term`: rozmiar przez `ioctl(TIOCGWINSZ)` i prawdziwy tryb raw
  (termios + RAII, `term_raw_enter`/`term_raw_leave`/`term_read_byte`),
  sprawdzone w PTY (Linux). Brak: Windows, wejscia nieblokujacego/timeoutu,
  parsowania sekwencji klawiszy (strzalki to surowe bajty 27,91,A..D).
* [czesciowe] `std:cybersecurity`: dostepne skroty (SHA-1/2/3, BLAKE2, MD5,
  CRC32), HMAC, PBKDF2, OsRng, base64, porownania w stalym czasie oraz (0.6)
  szyfrowanie AEAD `cipher` (ChaCha20-Poly1305, AES-256-GCM, losowy nonce,
  testy z wektorami z niezaleznej implementacji) i `random`.
  Brak: podpisow (Ed25519 poza `hlib`), KDF
  pamieciochlonnych (argon2/scrypt), HKDF.
* [czesciowe] `std:process`: uchwyty z odczytem linii na zywo i zapisem
  do stdin (zrobione). Brak: odczytu surowych bajtow (tylko linie), grup
  procesow, limitow czasu (`timeout`) wbudowanych w `run_*`.
* [czesciowe] `std:hk`: edycja z zachowaniem formatu dla kluczy pierwszego
  poziomu (`hk_set_value`, `hk_remove_key`, `hk_add_dependency`). `bit`
  nadal edytuje `Bit.hk` wlasnym kodem tekstowym - przejscie na `std:hk`
  zalezy od `bit`, ktory jest poza tym repozytorium.

## Menedzer pakietow `bit`

Osobny projekt (poza tym repozytorium). Po stronie HackerScript zostaja:
flagi `hackerc build` (`-o`, `--crate-name`, `--library`, `--bin-name`,
`--version-map`, `--libs-root`, `--bootstrap-root`, `--bit-root`),
rozwiazywanie `get <bit:...>` (`BIT_LIBS`, `cache/libs/<nazwa>/`,
`~/.hackeros/libs/<nazwa>/current/`) i format manifestu `Bit.hk`.

## Plan wielosesyjny (numeracja pozostaje 0.6)

Kolejnosc wg dzwigni i ryzyka; kazda sesja konczy sie testami w
`scripts/verify.sh` i wpisem w `CHANGELOG.md`. Status: [x] zrobione,
[~] w toku, [ ] przed nami.

* [x] **S1** Testy i skrypty z CI, pozycje diagnostyk wg kursora, `List<Float>.sort()`,
  `std:cybersecurity::cipher`/`random`, `std:term` (ioctl + raw/RAII), poprawki dokumentacji `fast direct`.
* [x] **S2** `direct`/`fast direct`: `std:python` (bledy jako `Result`, wymiana wartosci,
  wspolny stan z `direct`), traceback Pythona w komunikacie bledu bloku, benchmark
  `scripts/bench_direct.sh` (wyniki w `docs/FAST_DIRECT.md`), klasyfikacja backendow
  (zawezenie tylko w dokumentacji - aliasy zostaja dla zgodnosci). Poprawka regresji
  zdublowanych zaleznosci Cargo (`libc`/`pyo3`...). Nadal: `direct` jako instrukcja nie
  zwraca wartosci i panikuje przy bledzie; `{pypy}` bez wymiany danych.
* [~] **S3** Pozycje instrukcji z parsera w `Program` + typecheck (patrz wyzej), naprawa przesuniec
  linii po blokach i ekstraktorow. Otwarte: pozycje wyrazen/identyfikatorow, LSP na AST.
* [~] **S4** Generyki z adnotacja (`Wrapper<Int>`) - gotowe w zakresie opisanym wyzej. Otwarte: krotki.
* [ ] **S5** `hack3rc`: `Dict` (tablica mieszajaca w runtime C + lowering), `for` po nie-listach,
  test roznicowy z hackerc (hack3rc nie byl budowany lokalnie). Uwaga: interpolacji napisow
  NIE MA w jezyku (ani w `hackerc`, ani w `SYNTAX.md`) - to nowa funkcja jezyka, nie uzupelnienie hack3rc.
* [~] **S6** Wydzielono `codegen_sandbox.hcs`. Otwarte: plik bazowy z `CodeGen`, podzial
  emiterow `native`/`direct`, `project.hcs`; czas samohostowania niezmierzony (tu: build bez optymalizacji).
* [x] **S7** Sandbox `$ ... $`: seccomp, limity zasobow, `tests/sandbox.sh`, `docs/SANDBOX.md`.
