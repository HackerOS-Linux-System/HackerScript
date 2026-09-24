# `fast direct {backend} [ ... ]` — przyspieszony Python, statycznie linkowany

Status: **projekt (Faza 1 — specyfikacja)**. Rozszerza istniejące
`direct [ ... ]` (patrz `docs/SYNTAX.md`) o wybór **backendu
wykonania Pythona** zamiast domyślnego CPythona przez PyO3.

## Model dzisiejszego `direct[...]` (punkt odniesienia)

Dziś jedyny wariant to `direct [ ... ]`: kod Python interpretowany w
runtime przez PyO3 (`auto-initialize`) — czyli binarka *dynamicznie*
zależy od `libpython` zainstalowanego w systemie, w którym się
uruchamia. Tylko wewnątrz `direct[...]` dostępne jest
`get <pypi:...>`.

## Cel `fast direct`

Dwie niezależne zmiany względem zwykłego `direct`:

1. **Wybór backendu wykonania** — zamiast zawsze CPythona, `{backend}`
   wskazuje który interpreter/JIT/biblioteka faktycznie wykonuje kod.
2. **Statyczne linkowanie** — w przeciwieństwie do dzisiejszego
   `direct`, cały interpreter wybrany przez `{backend}` (PyPy albo
   CPython, zależnie od backendu — patrz tabela niżej) ląduje
   **wewnątrz** finalnej binarki jako statycznie zlinkowany
   artefakt, tak samo jak `native {C++}` dziś linkuje statycznie
   `libmymath.a` (`docs/SYNTAX.md`, sekcja FFI). Jedyny wyjątek —
   patrz "Co zostaje niestatyczne" niżej.

## Składnia

```
fast direct {pypy} [
    ...kod Pythona, wykonywany przez statycznie zlinkowany PyPy...
]

fast direct {numba} [
    ...kod Pythona z funkcjami @numba.njit, JIT-owany...
]
```

`{backend}` to jeden z 19 identyfikatorów zdefiniowanych niżej.
Domyślny, "gołe" `fast direct [ ... ]` (bez `{backend}`) **nie
istnieje** — backend jest zawsze obowiązkowy, żeby uniknąć niejawnego
wyboru linkowanego wariantu (w przeciwieństwie do zwykłego `direct`,
gdzie backend "systemowy CPython" jest jeden i nie ma dwuznaczności).

## 19 backendów — kategorie i co każdy statycznie linkuje

| Backend | Kategoria | Co jest linkowane statycznie do binarki |
|---|---|---|
| `pypy` | ogólny interpreter | PyPy (RPython JIT) zamiast CPython |
| `numba` | JIT kompilacja | CPython + `numba` (LLVM JIT za kulisami numby pozostaje dynamiczny w samej numbie — patrz "Co zostaje niestatyczne") |
| `cython` | kompilacja do C | kod `.pyx` kompilowany build-time do `.c`→obiekt, linkowany jak `native {C++}` |
| `numpy` | obliczenia tablicowe | CPython + `numpy` (z jej skompilowanym `libnpymath`) |
| `polars` | dataframe (Rust-native) | `polars` ma silnik w Ruście — most bezpośrednio do `polars-rs`, bez przechodzenia przez CPython, gdy kod tego nie wymaga |
| `scipy` | obliczenia naukowe | CPython + `scipy` (LAPACK/BLAS statycznie, wariant `openblas-static`) |
| `asyncio` | pętla zdarzeń (stdlib) | CPython + `asyncio` z biblioteki standardowej |
| `uvloop` | szybka pętla zdarzeń | CPython + `uvloop` (libuv statycznie) |
| `multiprocessing` | równoległość procesowa | CPython + `multiprocessing`; proces potomny to wciąż ten sam statycznie zlinkowany interpreter we własnej binarce |
| `jax` | autodiff/XLA | CPython + `jax` (backend CPU-XLA; GPU patrz `cupy`) |
| `pythran` | kompilacja do C++ | j.w. co `cython`: adnotowany Python kompilowany build-time do C++, linkowany statycznie |
| `duckdb` | silnik SQL in-process | most bezpośredni do `libduckdb` statycznej (C API), CPython opcjonalny (tylko gdy kod miesza SQL z Pythonem) |
| `cupy` | tablice na GPU | CPython + `cupy`; **wymaga** dynamicznego CUDA runtime — patrz "Co zostaje niestatyczne" |
| `vaex` | dataframe leniwe/out-of-core | CPython + `vaex` |
| `numexpr` | wyrażenia wektorowe | CPython + `numexpr` |
| `trio` | strukturalna współbieżność | CPython + `trio` |
| `aiohttp` | klient/serwer HTTP async | CPython + `aiohttp` |
| `granian` | serwer ASGI/WSGI (Rust-native) | analogicznie do `polars` — rdzeń Rustowy, most bez pełnego CPythona tam, gdzie kod tego nie wymaga |
| `httpx` | klient HTTP sync/async | CPython + `httpx` |
| `ray` | rozproszona równoległość | CPython + `ray`; węzeł-koordynator (`raylet`) to jedyny element, który z natury Ray pozostaje osobnym procesem — patrz "Co zostaje niestatyczne" |

## Co zostaje niestatyczne (uczciwie, jak reszta tego bootstrapu)

Statyczne linkowanie "wszystkiego poza interpreterem" ma twarde
granice narzucone przez same biblioteki, nie przez `hackerc`:

* **`cupy`** — CUDA to własność NVIDIA; `libcudart`/sterownik GPU
  muszą być dynamicznymi bibliotekami systemowymi obecnymi w
  środowisku uruchomieniowym (analogicznie do tego, jak nawet
  natywne binarki CUDA w C++ nie linkują sterownika statycznie).
* **`ray`** — model rozproszony Ray z definicji uruchamia osobne
  procesy robocze (`raylet` + workery) komunikujące się przez sieć/
  IPC; "jedna statyczna binarka" dotyczy procesu-klienta wołającego
  Ray, nie całego klastra.
* **`numba`** — sama numba JIT-uje przez LLVM w runtime; LLVM wewnątrz
  numby jest linkowane tak, jak dystrybuuje je sam pakiet `numba`
  (dziś: statycznie w jej wheelach `manylinux`), więc dla typowego
  przypadku to i tak w pełni statyczne — zastrzeżenie dotyczy tylko
  niestandardowych/self-built kompilacji numby.

Wszystkie pozostałe 16 backendów (w tym cały interpreter — PyPy albo
CPython, zależnie od backendu) linkują się w pełni statycznie do
finalnej binarki.

## Semantyka wyboru interpretera

* `pypy`, `numba`, `numpy`, `scipy`, `asyncio`, `uvloop`,
  `multiprocessing`, `jax`, `duckdb` (tryb mieszany), `cupy`, `vaex`,
  `numexpr`, `trio`, `aiohttp`, `httpx` → **PyPy** statycznie
  zlinkowany jako bazowy interpreter (stąd nazwa "fast direct" —
  PyPy sam w sobie jest już szybszym CPythonem dzięki JIT).
* `cython`, `pythran` → kod jest **kompilowany build-time**, nie
  interpretowany — bazowy interpreter (PyPy) jest obecny tylko jako
  runtime dla ewentualnego kodu Python w tym samym bloku, który nie
  został objęty adnotacjami `cython`/`pythran`.
* `polars`, `granian`, `duckdb` (tryb czysty-SQL) → most bezpośrednio
  do silnika Rust/C, **bez** startowania interpretera Pythona w
  ogóle, gdy blok nie zawiera żadnego kodu Python spoza wywołań tej
  biblioteki (optymalizacja: unikamy kosztu startu PyPy tam, gdzie
  nie jest potrzebny).

`get <pypi:...>` wewnątrz `fast direct {backend}[...]` działa tak jak
dziś w `direct[...]`, z jednym dodatkiem: zależność jest rozwiązywana
i **zapisywana do cache builda jako źródło do statycznego
zlinkowania** (`virus build`), a nie tylko jako pakiet instalowany w
środowisku uruchomieniowym.

## Architektura w repo (planowana)

* `docs/SYNTAX.md` — nowa sekcja "`fast direct {backend}[...]`" obok
  istniejącej "`direct[...]`", z tabelą backendów skróconą do
  przykładów (pełna tabela zostaje tu, w `FAST_DIRECT.md`).
* `docs/GRAMMAR.md` — produkcja `fast_direct_block`/`fd_backend` (już
  dodana w tej rundzie, patrz sekcja 6 tego pliku).
* `hackerc/cmd/lexer.hcs` — tokenizacja `fast` `direct` `{` ident `}`
  jako nowej sekwencji słów kluczowych (analogicznie do `native
  {Silnik}`, który już dziś parsuje nawiasy klamrowe z identyfikatorem
  w środku — `fast direct {backend}` idzie tym samym torem co
  istniejący `native {...}`, nie wymaga nowego mechanizmu lexera).
* `hackerc/cmd/parser.hcs` — nowy węzeł AST `FastDirectBlock(backend:
  Str, body: List<Stmt>)`, równoległy do istniejącego `DirectBlock`.
* `hackerc/cmd/typecheck.hcs` — walidacja `{backend}` przeciw liście
  19 znanych identyfikatorów (błąd E-kod analogiczny do E0003 dla
  nieznanego źródła `get`).
* `hackerc/cmd/codegen.hcs` — generuje inicjalizację odpowiedniego
  środowiska wykonania (PyO3 z PyPy jako `sys.executable` statycznie
  wbudowanym / most Rust-native dla `polars`/`granian`/`duckdb`) —
  jedna funkcja `gen_fast_direct(backend, body)` z dispatchem po
  backendzie, każdy backend to osobna, mała gałąź kodogenu.
* `virus/cmd/build.hcs` — `build_wire_fast_direct_dependencies`,
  analogiczne do już istniejącego `build_wire_extern_dependencies`:
  dla wybranych backendów pobiera/buduje statyczny artefakt
  interpretera (PyPy) i dopisuje go do `build.rs`/`Cargo.toml` tego
  projektu, na wzór tego jak dziś działa `get <extern:...> use
  <static>`.

## Kolejność wdrożenia (Faza 3 → 4, patrz `docs/ROADMAP.md`)

Nie wszystkie 19 naraz — plan zakłada wdrożenie w grupach, każda z
realnym, przetestowanym przykładem w `docs/showcase`, zanim ruszy
kolejna:

1. **Grupa pilotażowa** (dowodzi całego mechanizmu end-to-end):
   `pypy` (bazowy interpreter, wszystko inne go używa) + `numpy`
   (najbardziej typowy przypadek użycia).
2. **Grupa "Rust-native mosty"** (inny mechanizm niż reszta —
   zero-lub-częściowy CPython): `polars`, `granian`, `duckdb`.
3. **Grupa "kompilacja build-time"**: `cython`, `pythran` (dzielą
   infrastrukturę z `native {C++}`).
4. **Grupa "współbieżność/async"**: `asyncio`, `uvloop`, `trio`,
   `multiprocessing`, `aiohttp`, `httpx`.
5. **Grupa "obliczenia naukowe"**: `scipy`, `jax`, `numexpr`, `vaex`.
6. **Grupa "specjalna"** (mają udokumentowane wyjątki od pełnej
   statyczności — patrz "Co zostaje niestatyczne"): `numba`, `cupy`,
   `ray`.
