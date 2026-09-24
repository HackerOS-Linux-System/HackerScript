# Składnia HackerScript

Ten plik jest źródłem prawdy o składni HackerScript (`.hcs`), pisanym
ręcznie na podstawie faktycznej implementacji w `hackerc/cmd/lexer.hcs`
i `hackerc/cmd/parser.hcs` — w razie rozbieżności wygrywa kod
parsera, a ten dokument traktuj jako mapę, nie specyfikację formalną
(gramatyka `hackerc` nie jest dziś opisana w BNF/EBNF — patrz
`docs/ROADMAP.md`).

Wcześniej ten plik był referencjonowany z dziesiątek komentarzy `!!!`
w całym kodzie (`lexer.hcs`, `libs/core/lib/mod.hcs`, i inne), ale
fizycznie nie istniał — to jest jego pierwsza wersja.

## Komentarze

| Składnia          | Znaczenie                                                            |
|--------------------|-----------------------------------------------------------------------|
| `!! tekst`         | Zwykły komentarz liniowy.                                             |
| `!!! tekst`        | Komentarz dokumentacyjny (`DocComment`) — jako samodzielna instrukcja staje się `ExprStmt(StringLit(..., true))`, parytet z docstringiem. |
| `!> ... <!`        | Komentarz blokowy (wieloliniowy), usuwany przed tokenizacją — nowe linie w środku są zachowywane (żeby numery linii się nie przesuwały). |

## Literały i typy podstawowe

```
123          !! Number (Int/Float - bez rozróżnienia na etapie lexera)
"tekst"      !! Str, wspiera \n \t \r \\ \" \' \0 \e (ESC/0x1B)
true / false !! Bool
null         !! NullLit
[1, 2, 3]    !! ListLit -> List<T>
```

Typy wbudowane najczęściej spotykane w adnotacjach: `Int`, `Float`,
`Str`, `Bool`, `List<T>`, `Dict<K, V>`, `Option<T>`, `Result<T, E>`.

## Zmienne

```
let x = 5
let y: Int = 10
const PI: Float = 3.14159
```

`const` na najwyższym poziomie generuje stałą Rust (wartość MUSI być
literałem — patrz `codegen.hcs::gen_const`).

## Funkcje

```
fun add(a: Int, b: Int) -> Int [
    end a + b
]

pub fun greet(name: Str) [
    log("Hej,", name)
]
```

`end wyrażenie` to instrukcja powrotu (odpowiednik `return`; `return`
też jest słowem kluczowym i działa tak samo — `end` jest formą
preferowaną w tym repozytorium). `pub` przed `fun`/wewnątrz `impl`
oznacza `pub fn` w wygenerowanym Ruście.

## Sterowanie przepływem

```
if cond [
    ...
] elif other_cond [
    ...
] else [
    ...
]

while cond [
    ...
]

for item in iterable [
    ...
]

break
continue
```

## `struct` / `enum` / `match` / `impl`

```
struct Point [
    x: Int,
    y: Int
]

enum Shape [
    Circle(Float),
    Rectangle(Float, Float),
    Empty
]

fun area(s: Shape) -> Float [
    match s [
        Circle(r) -> [
            end 3.14159 * r * r
        ]
        Rectangle(w, h) -> [
            end w * h
        ]
        _ -> [
            end 0.0
        ]
    ]
]

impl Point [
    fun length(self) -> Float [
        end (self.x * self.x + self.y * self.y)
    ]
]
```

## System modułów

```
include <sciezka/bez/rozszerzenia>          !! `mod`/glob-import z pliku siostrzanego

get <std:io>                                !! biblioteka standardowa
get <std:string> import <str_join::str_trim> !! import wybranych nazw
get <core:memory::arena>                    !! `core`, z podmodułem po `::`
get <crates:serde::1.0>                     !! prawdziwa zależność Cargo
get <pypi:rich>                             !! dostępne tylko w direct[ ... ]
get <npm:left-pad> / get <jsr:@std/path>    !! JS/TS (w budowie, patrz ROADMAP)
get <vira:nazwa>                            !! biblioteka Vira typu "git" (.hcs)
get <work:parser>                           !! czlonek workspace "parser" (0.4, cale lib/mod.hcs)
get <work:parser::ast>                      !! ...jego podmodul lib/ast.hcs
```

Zrodła obsługiwane dziś przez `get <źródło:nazwa[::wersja]>`: `std`,
`core`, `selfhost` (tylko `include`, blokowane w `get`), `virus`,
`vira`, `work`, `hlib`, `bytes`, `bit`, `crates`, `pypi`, `npm`, `jsr`
— oraz, od **0.3**, `extern`, `c`, `cpp` (patrz sekcja FFI niżej).

### `get <work:członek[::plik]>` — import z workspace (nowość 0.4)

Odpowiednik Rustowego `use nazwa_membera::modul::*;` dla dowolnego
członka `[workspace] -> members` z `Virus.hk` — nie tylko `core`/`std`
(które mają własne, krótsze aliasy `get <core:...>`/`get <std:...>`,
działające identycznie i nadal zalecane dla nich dwóch).

```
!! w dowolnym pliku .hcs, gdziekolwiek w workspace:
get <work:parser>                    !! => <workspace_root>/parser/lib/mod.hcs
get <work:parser::ast>               !! => <workspace_root>/parser/lib/ast.hcs
get <work:parser> import <parse_ast> !! import wybranych nazw, jak przy std/core
```

**Kiedy członek jest importowalny w ten sposób.** Każdy członek
workspace ma jeden z dwóch kształtów (albo oba naraz), rozpoznawany
wyłącznie po zawartości jego własnego katalogu — **bez** żadnego pola
w `Virus.hk` (sekcja `[build]` z takim polem została usunięta w 0.4,
patrz niżej):

| Kształt        | Wymagany plik      | Budowany przez     | Importowalny przez        |
|-----------------|---------------------|---------------------|-----------------------------|
| "binarka"       | `cmd/main.hcs` (`fun main()`) | `virus build`       | nie (uruchamiany, nie importowany) |
| "biblioteka"    | `lib/mod.hcs`       | nie (nie ma `cmd/`) | `get <work:nazwa[::plik]>` |

`hackerc`/`virus` same są dziś "binarkami" (mają tylko `cmd/main.hcs`),
`libs/core`/`libs/std` są "bibliotekami" (mają tylko `lib/mod.hcs`).
Nic nie stoi na przeszkodzie, żeby przyszły członek miał **oba**
naraz — byłby wtedy jednocześnie samodzielnym narzędziem i biblioteką
dla reszty workspace.

Rozwiązywanie ścieżki (`hackerc/cmd/project.hcs::find_workspace_root`)
idzie w górę drzewa katalogów od pliku, w którym jest napisany `get
<work:...>`, szukając najbliższego przodka mającego
`<nazwa_membera>/lib` jako katalog — działa więc identycznie
niezależnie od tego, z którego miejsca w workspace go użyjesz.

### `include <work:członek[::plik]>` — statyczne linkowanie bez kopiowania (nowość 0.4)

Drugi kształt zwykłego `include <ścieżka>` (który wciąż działa bez
zmian, względem katalogu bieżącego pliku). Ten łączy dwie rzeczy:
semantykę `include` (plik jest scalany **bez prefiksu**, jak Rustowe
`mod` — w przeciwieństwie do `get`, który tworzy nazwany, osobny
podmoduł) z rozwiązywaniem ścieżki `get <work:...>` (względem korzenia
workspace, nie bieżącego katalogu).

```
!! playground/lib/mod.hcs:
include <work:hackerc::ast_nodes>   !! => hackerc/cmd/ast_nodes.hcs, scalony bez prefiksu
include <work:hackerc::lexer>
include <work:hackerc::parser>
include <work:hackerc::typecheck>
include <work:hackerc::diagnostics>

!! wszystkie funkcje z tych plikow (parse, check_program, ...) sa juz
!! dostepne bez prefiksu, dokladnie tak jak przy zwyklym
!! `include <lexer>` w obrebie tego samego katalogu:
fun check_source(src: Str) -> Str [
    let toks = tokenize(src)
    ...
]
```

**Po co to istnieje, skoro jest już `get <work:...>`.** Oba w
generowanym Rust kończą się identycznie: `use crate::<flat>::*;`
(`gen_include`/`gen_get_import` w `codegen.hcs`) — jedyna różnica to
brak `import <wybrane_nazwy>` przy `include` (zawsze pełny glob) i
inny prefiks generowanej nazwy modułu (`_hks_inc_` zamiast `_hks_`, by
nigdy nie kolidować z `get`). To jest właśnie **prawdziwe statyczne
linkowanie**: kod `hackerc`a (parser, typecheck, diagnostyki...) trafia
skompilowany **wprost do crate'a** `playground` (patrz sekcja
"`@wasm_export`" niżej) — zero kopiowania plików, zero duplikowania
źródła między `hackerc/cmd/` a `playground/lib/` — dokładnie jak
wtedy, gdy `mod` w Rust wskazuje na plik spoza własnego katalogu
(`#[path = "..."]`), tylko bez potrzeby takiej adnotacji.

Rozwiązywanie ścieżki: `hackerc/cmd/project.hcs::resolve_include_path`
rozpoznaje prefiks `"work:"`, po czym używa dokładnie tej samej pary
funkcji co `get <work:...>` (`find_workspace_root` +
`work_module_file_path`) — szuka w górę drzewa katalogów, zaczynając
od pliku z `include`, aż znajdzie `<przodek>/<członek>/lib`.

### `@wasm_export` i `virus build --wasm` (nowość 0.4)

Marker tekstowy (na wzór istniejącego `@hot_reload`) przed
`fun nazwa(...) [ ... ]` na najwyższym poziomie pliku — oznacza
funkcję do wyeksportowania z crate'a do JavaScript przez
[`wasm-bindgen`](https://rustwasm.github.io/wasm-bindgen/):

```
@wasm_export
fun check_source(source: Str) -> Str [
    ...
]
```

Skutek w wygenerowanym Ruście: `#[wasm_bindgen]` tuż przed `pub fn`
(`codegen.hcs::gen_fun`) + `use wasm_bindgen::prelude::*;` w nagłówku
crate'a. Jak przy `@hot_reload`, ekstrakcja dzieje się na surowym
tekście PRZED tokenizacją (`transpiler.hcs::
extract_wasm_export_markers`) — AST (`FunDecl`) pozostaje całkowicie
nieświadome markera. **Ograniczenie** (to samo, co realny
`wasm-bindgen` ma zawsze — nie dodatkowe od nas): funkcje generyczne
nie są wspierane — kompilują się normalnie, ale bez atrybutu, z
ostrzeżeniem jako komentarz w wygenerowanym Ruście.

Gdy co najmniej jedna funkcja w projekcie jest oznaczona
`@wasm_export`, `hackerc build`/`virus build` generuje `Cargo.toml` w
kształcie `[lib]` (`crate-type = ["cdylib", "rlib"]`, `path =
"src/lib.rs"`) zamiast zwykłego `[[bin]]` (`src/main.rs`) —
automatycznie, bez żadnej flagi (patrz `project.hcs::cargo_toml_text`).

`virus build --wasm` (`TargetWasm` w `virus/cmd/build.hcs`) kompiluje
ten crate na `wasm32-unknown-unknown` (`cargo build --target
wasm32-unknown-unknown --release`), po czym — jeśli `wasm-bindgen`
(CLI, **osobny** program od crate'a/cargo, instalowany przez `cargo
install wasm-bindgen-cli`) jest zainstalowany — uruchamia go, żeby
wygenerować glue `.js`/`.d.ts` + finalny, przetworzony `..._bg.wasm`
gotowy pod `import` w przeglądarce. Wersja CLI **musi** zgadzać się z
wersją `wasm-bindgen` w `Cargo.toml` (`"0.2"`) — niedopasowanie kończy
się twardym błędem CLI, to ograniczenie całego ekosystemu
`wasm-bindgen`, nie coś specyficznego dla `virus`. Brak CLI nie jest
błędem krytycznym: surowy `.wasm` (bez glue) i tak trafia do
`cache/build/`, z ostrzeżeniem jak go dokończyć ręcznie.

Zobacz `playground/` (`playground/lib/mod.hcs` + `playground/web/`) -
działający przykład: `check_source` oznaczone `@wasm_export`, budowane
z `virus build --wasm` uruchomionym w `playground/`.



Do 0.3 `Virus.hk` mógł mieć sekcję `[build] -> entry => <ścieżka>`,
nadpisującą, który plik jest punktem wejścia. Od **0.4** ta sekcja
została **całkowicie usunięta** — plik wejściowy dowolnego budowalnego
członka/projektu to zawsze, bez wyjątku, `cmd/main.hcs` (dokładny
odpowiednik tego, jak Cargo samo znajduje `src/main.rs`, bez żadnego
pola w `Cargo.toml`). Jeśli w starym `Virus.hk` sekcja `[build]` nadal
występuje, jest po prostu ignorowana (nieznane sekcje nie są błędem).

`using <wersja>` na początku pliku (albo `[package] using` w
`Virus.hk`) deklaruje wymaganą wersję kompilatora `hackerc`.

## `direct[ ... ]` — surowy Python

```
direct [
    print("Wykonywane przez PyO3, wewnątrz binarki Rust.")
]
```

Interpretowany w runtime (nie kompilowany) przez wbudowany interpreter
Pythona (PyO3, `auto-initialize`). Tylko tu dostępne są `get <pypi:...>`.

## `native {Silnik} [ ... ]` — inne języki

```
native {JavaScript} [
    console.log("Wykonywane przez QuickJS (rquickjs), w runtime.")
]

native {C++} [
    // Kompilowane BUILD-TIME (crate `cc`), nie interpretowane -
    // patrz sekcja FFI niżej.
]
```

Wariant `native {JavaScript} <ścieżka/do/pliku.js>` importuje cały
plik; `native {JavaScript} <ścieżka.js> [ ... ]` importuje plik I
dokleja kod inline PO nim.

## `manual[ ... ]`

```
manual [
    ...
]
```

Blok emitowany jako `unsafe { ... }` w wygenerowanym Ruście.

## `gc:use::tryb`

```
gc:use::arena
```

Pragma sterująca strategią alokacji dla bieżącego zakresu — patrz
`libs/core/lib/memory/`.

## `$ polecenie $` — Zero-Cost Sandbox

```
$ ls -la $
```

Generuje `std::process::Command` z izolacją sieci/mount-namespace
(`unshare`) w procesie potomnym — patrz komentarz w `codegen.hcs` przy
`gen_sandbox`.

## `@hot_reload`

Znacznik nad `fun`, który wydziela ciało funkcji do osobnego,
dynamicznie przeładowywanego crate'a (`libloading`) — użyteczne przy
szybkiej iteracji bez pełnej rekompilacji binarki głównej.

---

## FFI: `extern` / `region` / `c` / `cpp` (0.3)

> **Zmiana względem `< 0.3`:** stara, jedno-funkcyjna składnia
> `extern "biblioteka" fun nazwa(params) -> Typ` **została usunięta**.
> `extern` przestał być słowem kluczowym języka. Poniższa składnia ją
> zastępuje.

### `get <extern:ścieżka> use <static|dynamic>`

Deklaruje bibliotekę natywną (plik `.a`/`.so`/`.lib` itp.) do
podlinkowania, oraz tryb linkowania:

```
get <extern:libs/native/libmymath.a> use <static>
get <extern:libmysystemlib> use <dynamic>
```

* `use <static>` → linkowanie statyczne (`kind = "static"` w
  `#[link(...)]`, `cargo:rustc-link-lib=static=...` w `build.rs`).
* `use <dynamic>` (albo pominięte `use <...>`) → linkowanie
  dynamiczne (domyślne zachowanie linkera, `dylib`).

Sama deklaracja `get <extern:...>` **niczego nie linkuje** — mówi
kompilatorowi "następny `region [ ... ]` w tym pliku należy do tej
biblioteki, w tym trybie". Samo linkowanie (znalezienie/skopiowanie
pliku, dopisanie `cargo:rustc-link-*` do `build.rs`) robi `virus
build` (patrz `virus/cmd/build.hcs::build_wire_extern_dependencies`).

### `region [ ... ]`

Zaraz po `get <extern:...> use <...>`, blok `region` zawiera dowolnie
wiele **samych sygnatur** (bez ciała) funkcji z tej biblioteki:

```
get <extern:libs/native/libmymath.a> use <static>
region [
    fun add(a: Int, b: Int) -> Int
    fun mul(a: Int, b: Int) -> Int
    fun greet(name: Str)
]

fun main() [
    log("2 + 3 =", add(2, 3))
]
```

Po sparsowaniu `region` funkcje zadeklarowane w środku są wołane
**normalnie**, jak każda inna funkcja HackerScript — `region` tylko
je deklaruje, nie trzeba nic dodatkowo importować przy wywołaniu.
Wygenerowany Rust to jeden zbiorczy blok:

```rust
#[link(name = "mymath", kind = "static")]
extern "C" {
    pub fn add(a: i64, b: i64) -> i64;
    pub fn mul(a: i64, b: i64) -> i64;
    pub fn greet(name: &str);
}
```

`region` bez poprzedzającego `get <extern:...>` w tym samym pliku
kompiluje się (best-effort, zgodnie z resztą tego bootstrapu — patrz
"Ograniczenia" w `docs/ROADMAP.md`), ale z zaślepioną nazwą biblioteki
i ostrzeżeniem w wygenerowanym kodzie.

### `get <c:nazwa>` / `get <cpp:nazwa>`

Deklaruje zależność od biblioteki C/C++ linkowanej po nazwie (dziś:
jako systemowa biblioteka dynamiczna, wyszukiwana domyślnymi ścieżkami
linkera — patrz "Ograniczenia" niżej):

```
get <c:zlib>
get <cpp:fmt>
```

Sygnatury funkcji z takiej biblioteki wciąż deklarujesz osobno przez
`get <extern:nazwa> use <...>` + `region [ ... ]` (zwykle
`use <static>` lub `use <dynamic>`, zależnie jak biblioteka jest
dostarczona w Twoim środowisku).

### `native {C++} [ ... ]`

W odróżnieniu od `native {JavaScript}` (interpretowane w runtime przez
QuickJS), `native {C++}` jest **kompilowane build-time**: kod C++ w
środku jest zawijany jako ciało `extern "C" void
__hks_native_cpp_block_N()`, kompilowany przez `virus build` (crate
`cc`, `.cpp(true)`) i linkowany statycznie do binarki:

```
fun main() [
    native {C++} [
        #include <cstdio>
        std::printf("Cześć z C++, skompilowane build-time.\n");
    ]
]
```

Wymaga kompilatora C++ (g++/clang++) w środowisku, w którym uruchamiasz
`virus build` — dokładnie tak, jak `native {JavaScript}` wymaga
kompilatora C (bo `rquickjs-sys` kompiluje C QuickJS przez `cc`).

### Ograniczenia (uczciwie, jak reszta tego bootstrapu)

* `get <c:...>`/`get <cpp:...>` (BEZ `use <...>`) wciąż linkują
  wyłącznie **systemową** bibliotekę po nazwie. Kompilowanie WŁASNEGO
  źródła `.c`/`.cpp` z projektu jest od tej rundy możliwe — ale przez
  osobny mechanizm: sekcja `[native_sources]` w `Virus.hk` (patrz
  `docs/VIRUS.md`), nie przez `get <c:...>`/`get <cpp:...>`.
* ~~Sygnatury zadeklarowane w `region [ ... ]` nie są dziś wpuszczane
  do `typecheck.hcs`/`typeinfer.hcs`~~ **Naprawione**: od tej rundy
  `collect_signatures` (`hackerc/cmd/typeinfer.hcs`) wpisuje każdą
  `ExternSig` z `region [ ... ]` do tej samej tabeli `functions`, co
  zwykłe `fun` — literówka w nazwie/liczbie argumentów wywołania
  funkcji z `region` daje teraz błąd `hackerc` (arity check w
  `check_call`), zamiast dopiero błędu `rustc` na wygenerowanym
  kodzie.
* `region` nie sprawdza w żaden sposób, że poprzedzający go `get
  <extern:...>` rzeczywiście istnieje w tym samym pliku (best-effort
  parser, patrz wyżej).
