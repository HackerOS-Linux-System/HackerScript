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
```

Zrodła obsługiwane dziś przez `get <źródło:nazwa[::wersja]>`: `std`,
`core`, `selfhost` (tylko `include`, blokowane w `get`), `virus`,
`vira`, `crates`, `pypi`, `npm`, `jsr` — oraz, od **0.3**, `extern`,
`c`, `cpp` (patrz sekcja FFI niżej).

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

* `get <c:...>`/`get <cpp:...>` linkują dziś **systemową** bibliotekę
  po nazwie. Kompilowanie WŁASNEGO źródła `.c`/`.cpp` z projektu
  (np. `native/<nazwa>/*.c`) przez `cc::Build` zamiast tego — **nie
  jest jeszcze zrobione** (patrz `docs/ROADMAP.md`).
* Sygnatury zadeklarowane w `region [ ... ]` nie są dziś wpuszczane do
  `typecheck.hcs`/`typeinfer.hcs` (ten sam gap co miało stare
  `extern "lib" fun ...` przed 0.3) — literówka w nazwie/typie
  parametru nie da błędu kompilatora HackerScript, tylko błąd `rustc`
  na wygenerowanym kodzie.
* `region` nie sprawdza w żaden sposób, że poprzedzający go `get
  <extern:...>` rzeczywiście istnieje w tym samym pliku (best-effort
  parser, patrz wyżej).
