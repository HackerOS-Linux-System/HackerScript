# hack3rc — kompilator AOT HackerScript (Cranelift)

`hack3rc` kompiluje **podzbiór HackerScript do natywnych plików
wykonywalnych** przez [Cranelift](https://cranelift.dev), dla wielu
architektur. W odróżnieniu od `hackerc` nie transpiluje do Rusta i nie
wywołuje `cargo` — kompilacja programu trwa dziesiątki milisekund.

Cały kod źródłowy `hack3rc` to **HackerScript** (pliki `.hcs`; zero
plików `.rs`/`.c` w repozytorium):

| Plik | Rola |
|---|---|
| `hack3rc/cmd/main.hcs` | CLI i driver (argumenty, cele, linkowanie, `--run`) |
| `hack3rc/cmd/lower.hcs` | analiza semantyczna + typy + lowering AST → IR (czysty HackerScript) |
| `hack3rc/cmd/backend.hcs` | backend Cranelift: IR → plik obiektowy |
| `hack3rc/cmd/runtime.hcs` | runtime (tekst C linkowany z programem): I/O, napisy, listy, **odśmiecacz pamięci** |
| `docs/hack3rc-examples/` | przykłady z oczekiwanym wyjściem (`*.hcs` + `*.out`), uruchamiane przez `scripts/verify.sh` |

Frontend (lexer, parser, AST, diagnostyki) jest **współdzielony z `hackerc`**
(`include <work:hackerc::...>`), więc `hack3rc` czyta dokładnie ten sam język.

> **Granica „100% HackerScript”.** HackerScript nie ma składni ścieżek
> (`modul::Typ`), więc API crate'ów `cranelift-*` da się wywołać tylko
> przez wbudowany blok `native {Rust} [ ... ]`. Ten blok siedzi
> **wewnątrz `backend.hcs`** i robi jedno: zamienia IR na obiekt
> Cranelift. Runtime (~150 linii C) jest tekstem w `runtime.hcs`,
> zapisywanym i kompilowanym przez `cc` przy linkowaniu (to samo `cc` jest
> potrzebne do linkowania).

## Budowanie

Potrzebny `hackerc` **z obsługą features w `get <crates:...>`** (wersja z tego
repozytorium; dostarczona binarka v0.4 jej nie ma) i toolchain Rust ≥ 1.82:

```
# 1. (jednorazowo) zbuduj hackerc z tego repozytorium dowolnym hackerc >= 0.4
hackerc build hackerc/cmd/main.hcs -o /tmp/hackerc-crate --crate-name hackerc
(cd /tmp/hackerc-crate && cargo build --release)

# 2. zbuduj hack3rc nowym hackerc
/tmp/hackerc-crate/target/release/hackerc build hack3rc/cmd/main.hcs \
    -o /tmp/hack3rc-crate --crate-name hack3rc
(cd /tmp/hack3rc-crate && cargo build --release)    # target/release/hack3rc
```

Pierwszy build kompiluje Cranelift ze wszystkimi architekturami (kilka minut).

## Użycie

```
hack3rc program.hcs                    # -> ./program (natywny plik wykonywalny)
hack3rc program.hcs -o out             # własna nazwa wyniku
hack3rc program.hcs --run              # zbuduj i uruchom (kod wyjścia = kod programu)
hack3rc program.hcs --check            # tylko analiza, bez generowania kodu
hack3rc program.hcs --emit obj|ir      # tylko obiekt / IR
hack3rc program.hcs -O0|-O1|-O2        # optymalizacja Cranelift (domyślnie -O1)
hack3rc program.hcs --dump-clif        # Cranelift IR na stderr
hack3rc program.hcs --keep-temps       # zachowaj <wynik>.hks-tmp (prog.o, rt.c)

# cross-kompilacja
hack3rc program.hcs --target aarch64 -o prog-arm        # aarch64 | riscv64 | s390x | x86_64
hack3rc program.hcs --target aarch64 --run              # uruchom pod QEMU
hack3rc program.hcs --target aarch64 --cc "zig cc -target aarch64-linux-gnu"
hack3rc program.hcs --target x86_64-apple-darwin --emit obj
```

Kod wyjścia `hack3rc`: `0` sukces, `1` błąd kompilacji/linkowania. Z `--run`
zwracany jest kod wyjścia programu.

### Cele (architektury)

Cranelift generuje kod dla x86_64, aarch64, riscv64 i s390x (tylko cele
64-bitowe). `--target` przyjmuje skrót (`aarch64`, `riscv64`, `s390x`,
`x86_64`) albo pełny triple. Dla celu innego niż host potrzebny jest
kompilator C tego celu: domyślnie `<arch>-linux-gnu-gcc` (pakiety
`gcc-<arch>-linux-gnu`), albo `--cc` z dowolnym poleceniem (np. `clang
--target=...`, `zig cc -target ...`); `--run` używa `qemu-<arch> -L
/usr/<arch>-linux-gnu` albo `--runner`.

**Stan weryfikacji (uczciwie).** W tym repozytorium automatycznie
sprawdzany jest tylko x86_64 (natywnie) na przykładach z
`docs/hack3rc-examples/` (`scripts/verify.sh`, krok w CI). Wcześniejsze
uruchomienia pod QEMU — aarch64, riscv64, s390x (big-endian) — dotyczyły
większego, nieistniejącego tu zestawu testów i **nie zostały powtórzone**
w tej wersji; traktuj je jako niezweryfikowane. **Tylko obiekt** (sprawdzony
nagłówek pliku, bez linkowania ani uruchomienia): macOS
`x86_64-apple-darwin`/`aarch64-apple-darwin` (Mach-O) i Windows
`x86_64-pc-windows-msvc` (COFF). Obiekty te są emitowane, ale **nie są
linkowane**: pełny `exe` wymaga własnego `--cc` i nie był testowany (runtime
C zakłada POSIX/libc).

## Zakres języka

Zasada: **program przyjęty przez `hack3rc` jest też poprawnym programem
HackerScript** i daje to samo wyjście pod `hackerc`. **Wyjątek (sprawdzone):**
metody `Str` `.to_upper()` i `.contains(...)` są przyjmowane przez `hack3rc`,
ale `hackerc` generuje dla nich Rust, który się nie kompiluje — w przenosnym
kodzie używaj funkcji z `std:string`. Czego `hackerc` nie
przyjmuje (np. przypisanie do parametru), tego nie przyjmie też `hack3rc`.

Typy: `Int` (i64), `Float` (f64), `Bool`, `Str`, `List<T>`, `Option<T>`, `Result<T, E>`,
własne `struct` i `enum` (także rekurencyjne).

Obsługiwane:

- funkcje (typowane parametry i wynik, rekurencja, `fun main()` bez wartości)
- `struct` (konstruktor `Nazwa(a, b)`, pola, w tym zagnieżdżone), `enum` z
  polami (warianty wołane jak funkcje: `Circle(2.0)`, `Empty`), `impl` z
  metodami (`self` pierwszy) **i funkcjami asocjacyjnymi** (0.6: metoda bez `self`, wołana
  `Typ.nazwa(args)`; nazwa `new` jest zajęta przez konstruktor; `get` nie może być nazwą)
- wartości domyślne parametrów **funkcji** (0.6): `fun f(a: Int, b: Int = 5)` — tylko literał
  (liczba, napis, bool, liczba ujemna), tylko na końcu listy (H0201/H0205), podstawiane
  w miejscu wywołania; `hackerc` robi to samo (typecheck + codegen)
- `match` na `enum` i `Option` (instrukcja; wiązanie pól, `_`, kontrola
  wyczerpywania — brak wariantu to błąd)
- listy: literał, `xs[i]` (odczyt/zapis, także `xs[i].pole += x`,
  `g[i][j] = ...`), `.len()`, `.push(x)`, `.pop()` → `Option<T>`, `.clear()`,
  `.reverse()`, `.sort()` (tylko `List<Int>`), `.contains(x)` (`List<Int/Bool/Str>`),
  `.insert(i, x)`, `.remove(i)` → element, `.clone()`, `for x in lista`
- `Option`: `some(x)`, `none()` (typ z kontekstu: `let o: Option<Int> = none()`,
  `end none()`), `.is_some()`, `.is_none()`
- `Str`: literały, `+`, `==`, `!=`, `.len()` (bajty), `.is_empty()`, `.contains(s)`,
  `.starts_with(s)`, `.ends_with(s)`, `.find(s)` (indeks bajtu albo -1), `.to_upper()`,
  `.to_lower()`, `.trim()`, `.repeat(n)`, `x as Str` (Int/Float/Bool)
- `.clone()` — **głębokie kopiowanie** struktur, enumów, list, Option
- `let`, przypisania `=`, `+=`, `-=`, `*=`, `/=` (też na polach i elementach),
  `if/elif/else`, `while`, `break`, `continue`, `end`
- operatory: `+ - * / %`, `== != < <= > >=`, `and or not`, unarny `-`; brak
  niejawnych konwersji, rzutowania `as`: `Int↔Float`, `Bool→Int`, `→Str`
- `log(a, b, ...)` — `Int/Float/Bool/Str`
- wbudowane: `sqrt sin cos tan exp ln floor ceil fabs`, `pow`, `abs`,
  `read_int()`, `exit(Int)`

`Result<T, E>` i operator `?` są obsługiwane (sekcja „Result<T, E> i operator `?`” niżej).

Nieobsługiwane (czytelny błąd `H0xxx`, nie crash): `Dict` (także `for k, v in
d.items()`, H0151), `for` po czymkolwiek poza listą, interpolacja napisów,
`get`/`include`, `native`/`direct`, async/kanały, wartości domyślne parametrów METOD,
`log()` dla struct/enum. `sort` działa dla `List<Int>`, `List<Float>` (NaN na końcu;
w `hackerc` `Vec<f64>::sort` się nie kompiluje) i `List<Str>` (bajtowo). Pełna lista otwartych
punktów: `docs/ROADMAP.md`.

### Semantyka i różnice względem `hackerc`

- struct/enum/lista/Option to **referencje do obiektów na stercie**; `let b =
  a` przenosi (jak w Rust), `.clone()` kopiuje głęboko
- arytmetyka `Int` zawija się; dzielenie/reszta przez zero i indeks poza
  zakresem listy → komunikat na stderr i **kod wyjścia 101**
- `lista[i].pole += x` i `lista[i][j] += x` **modyfikują element w miejscu**.
  Znany błąd `hackerc`: generuje tu `.clone()` i po cichu gubił zapis — naprawione w `hackerc` (`gen_lvalue`, test
  `inplace`)
- `Float → Int` jest nasycające (NaN → 0); `log` drukuje floaty najkrótszą
  reprezentacją jak Rust; bardzo duże/małe floaty drukują się wykładniczo
  (`1e+21`), Rust drukuje pełne rozwinięcie
- kod wyjścia procesu: `0` albo wartość z `exit(n)`

## Model pamięci i odśmiecacz

Obiekty (`struct`, `enum`, lista, `Option`, napisy dynamiczne) trafiają na
stertę zarządzaną przez **konserwatywny odśmiecacz mark-sweep** (w
`runtime.hcs`): skanuje stos i rejestry (przez `setjmp`) oraz zawartość
obiektów, rozpoznaje też wskaźniki wewnętrzne, nie przenosi obiektów, więc
kod Cranelifta nie potrzebuje map stosu. GC startuje po zaalokowaniu 8 MB
i podwaja próg względem żywych danych. Każdy obiekt ma nagłówek
`{rozmiar, deskryptor, mark}`; deskryptor (`(data ...)` w IR: rodzaj,
liczba słów, maska wskaźników) mówi `hks_clone`, które pola kopiować.

Diagnostyka: `HKS_GC_STRESS=1` (odśmiecanie przy **każdej** alokacji),
`HKS_GC_MIN_KB=<n>` (próg startu). Zestaw testów uruchamia wszystkie
programy z obiektami także w trybie stresowym (`nazwa+gc` w `TESTS`).

Ograniczenia: GC jest jednowątkowy; rekurencja zbyt głęboka (stos C/Cranelift,
w tym głębokie `clone` struktur rekurencyjnych) kończy się SIGSEGV.

## Diagnostyki

Błędy mają kod, linię i kolumnę (np. `error[H0073]: argument 2 (konstruktor
P): oczekiwano Int, jest Float`). Błędy składni pochodzą z parsera `hackerc`
(`E0100`).

| Kody | Znaczenie |
|---|---|
| H0010 | literał liczbowy |
| H0020 | niezdefiniowana zmienna |
| H0030–H0031, H0060–H0064 | typy operandów / nieobsługiwane operatory |
| H0040–H0041 | rzutowania |
| H0050, H0053 | `null`, `?` |
| H0070–H0074 | wywołania (nieznana funkcja, liczba i typy argumentów) |
| H0080, H0090 | `log` / warunek nie-Bool |
| H0100–H0104 | `let` |
| H0110–H0113 | przypisania (cel, typ, operator, parametr niemutowalny) |
| H0120–H0122, H0160 | `end` |
| H0130, H0140, H0150 | `break/continue`, nieobsługiwane instrukcje, `for` |
| H0200–H0204, H0210 | sygnatury i elementy modułu |
| H0220–H0222 | `main` |
| H0300–H0302, H0310–H0312, H0320–H0323 | typy, struct/enum, `impl` |
| H0412–H0413, H0420–H0424 | `none()`/`some()`, `match` |
| H0430–H0431, H0440, H0450–H0451, H0460–H0462, H0470–H0478 | listy, warianty, indeksy, pola, metody |

## Architektura

```
źródło .hcs ─ lexer/parser (hackerc) ─▶ AST
            ─ lower.hcs: typy + lowering ─▶ IR (S-wyrażenia)
            ─ backend.hcs (Cranelift) ─▶ prog.o
            ─ <cc> prog.o rt.c -lm ─▶ plik wykonywalny
```

Typy w IR to maszynowe znaczniki: `i` (i64), `f` (f64), `b` (bool), `p`/`s`
(wskaźnik), `u` (brak).

```
(data NAZWA w ...)                         ; słowa i64 w kolejności bajtów celu
(extern NAZWA RET (T ...))
(fn NAZWA RET ((ID T) ...) INSTR ...)
INSTR := (let ID T E) | (set ID E) | (expr E) | (ret E) | (retv)
       | (store T ADRES OFFSET WARTOSC)    ; WARTOSC liczona PRZED ADRES
       | (if E (INSTR...) (INSTR...)) | (while E (INSTR...) (INSTR...))
       | (break) | (continue)
E     := (i N) | (f X) | (b 0|1) | (s "tekst") | (v ID) | (daddr NAZWA)
       | (load T ADRES OFFSET) | (seq (INSTR...) E)
       | (bin add|sub|mul|div|rem T A B) | (cmp eq|ne|lt|le|gt|ge T A B)
       | (and A B) | (or A B) | (not A) | (neg T A) | (cast T1 T2 A)
       | (call NAZWA A ...)
```

Układ obiektów: struct = pola w kolejnych 8-bajtowych slotach; enum/Option =
`[tag][pola...]` (tag = indeks wariantu; Option: `None`=0, `Some`=1); lista
= `{len, cap, data*}`. `--emit ir` pokazuje IR programu.

## Weryfikacja

```
HACKERC=... HACK3RC=<ścieżka-do-hack3rc> bash scripts/verify.sh   # krok 3
```

Krok 3 `scripts/verify.sh` kompiluje każdy `docs/hack3rc-examples/*.hcs` przez
`hack3rc <plik> --run` i porównuje wyjście bajt po bajcie z `<nazwa>.out`.
Zestaw jest **mały** (kilka programów) i nie zastępuje testu różnicowego
względem `hackerc`. Zmienna `HKS_GC_STRESS=1` jest dziedziczona przez
uruchamiane programy (odśmiecanie przy każdej alokacji) — można ją ustawić
ręcznie przy uruchamianiu skryptu. Cross-architekturowe uruchomienia trzeba
robić ręcznie (`--target <arch> --run`).

## Wydajność (pomiary jednej, hałaśliwej maszyny — nie benchmark)

- kompilacja programu: ≈ 0,15 s łącznie z linkowaniem i kompilacją runtime'u
  (wobec ≈ 8,7 s dla `hackerc` + `cargo`)
- obliczenia (`fib(40)` + liczby pierwsze do 2 000 000): 1,7–3 razy wolniej
  niż Rust z LLVM (`opt-level = 3`) — zależnie od pomiaru
- pętla alokująca (4 mln × struct + lista + napis): ≈ 5 razy wolniej niż Rust;
  pamięć stała (≈ 14 MB szczytu mimo ≈ 900 MB zaalokowanych w sumie)

## Ograniczenia i plan

- kompilator C (`cc`/cross-gcc/clang/zig cc) jest wymagany do linkowania
  i do kompilacji runtime'u; tylko cele 64-bitowe
- brak `Dict`, async, interpolacji i domyślnych wartości parametrów metod;
  `for` tylko po listach; `sort` tylko dla `List<Int>`, `List<Float>`, `List<Str>`
- diagnostyki mają linię i kolumnę znalezione przeszukiwaniem tekstu
  (AST nie niesie pozycji) — przybliżenie
- dostęp do list idzie przez wywołania runtime (bez wstawiania inline) —
  główne źródło różnicy wydajności; GC konserwatywny (możliwe fałszywe
  zatrzymania pamięci)
- wywołanie API Cranelifta bez bloku `native` wymagałoby składni `::`
  w parserze HackerScript


## Result<T, E> i operator `?` (0.5)

Typ `Result<T, E>` (obiekt na stercie: tag 0 = Ok, 1 = Err, payload w slowie 1; GC skanuje payload
zgodnie z deskryptorem wariantu). Konstruktory `Ok(x)`/`Err(e)` (tez malymi literami) wymagaja znanego
typu docelowego: adnotacja `let r: Result<Int, Str> = Ok(1)`, `end Ok(..)` w funkcji zwracajacej Result,
argument funkcji. `match` na Result (`Ok(v) -> ...`, `Err(e) -> ...`, wyczerpujacy albo z `_`),
metody `is_ok()`/`is_err()`.

Operator `?`: na `Result<T, E>` w funkcji zwracajacej `Result<_, E>` (ten sam typ bledu, bez konwersji)
przy `Err` zwraca z funkcji ten sam `Err`, w przeciwnym razie daje `T`. Na `Option<T>` w funkcji
zwracajacej `Option<_>` przy `None` zwraca `none()`. Bledy: H0414/H0415 (konstruktor bez typu / zly typ),
H0054/H0055/H0056 (`?` poza pasujaca funkcja / niezgodny typ bledu), H0474 (nieznana metoda Result).

Przyklad: `docs/hack3rc-examples/result_option.hcs`. Wczesniejsze testy obciazeniowe (payload
strukturalny, 20000 iteracji, `HKS_GC_STRESS=1`) nie sa czescia tego repozytorium.

Nadal nieobslugiwane: `Dict`, interpolacja stringow, async/kanaly, `get`/`include`, `native`.
