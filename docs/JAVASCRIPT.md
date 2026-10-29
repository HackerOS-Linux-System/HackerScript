# `use <lang:javascript>` — HackerScript → JavaScript

Backend JavaScript (`hackerc/cmd/javascript_backend.hcs`, runtime w
`js_runtime.hcs`) tłumaczy program HackerScript na jeden plik JavaScript
(ES2020, bez zależności), uruchamiany w Node.js albo w przeglądarce
(patrz `docs/WEBSITE.md`).

## Użycie

```
hackerc js program.hcs [-o program.js]     # jawna podkomenda
node program.js
```

albo przez dyrektywę w pliku — wtedy `hackerc build` zmienia cel z
crate'a Rust na JavaScript:

```
use <lang:javascript>

fun main() [
    log("Czesc z JavaScriptu")
]
```
```
hackerc build program.hcs [-o program.js]
```

Dyrektywę rozpoznaje się w pliku wejściowym (aliasy: `lang:js`).
`include <nazwa>` w pliku wejściowym jest rozwijane (pliki siostrzane
`.hcs`). `lang:kotlin` zostaje osobnym, eksperymentalnym backendem.

## Mapowanie języka

| HackerScript | JavaScript |
|---|---|
| `Int`, `Float` | `number` |
| `Str`, `Bool` | `string`, `boolean` |
| `List<T>` | `Array` (indeksowanie sprawdzane: panic poza zakresem) |
| `Dict<K,V>` | `Map` (`insert`, `fetch`, `contains`, `remove`, `len`) |
| `struct` | `class` (konstruktor pozycyjny: `Point(1, 2)` → `new Point(1, 2)`) |
| `enum` | klasa z polami `$` (nazwa wariantu) i `v` (pola); warianty wołane jak funkcje |
| `impl` | metody na prototypie klasy (`self` → `this`) |
| `match` | łańcuch `if (x.$ === "Wariant")` z wiązaniem pól; `_` = `else` |
| `Option` / `Result` | `some`/`none`/`ok`/`err`, `?` przez wyjątek wczesnego powrotu |
| `async fun`, `await`, `spawn` | `async function`, `await`, `Promise` |
| `chan(n)`, `.send`, `.recv`, `.close` | `HksChan`; w kontekście `async` (async fun, operand `spawn`) `send`/`recv` dostają **automatyczny `await`** (0.6) |
| `native {JavaScript} [ ... ]` | surowy JS wklejony w osobnym zasięgu `{ ... }` (0.6) |
| `log`, `elog` | `console.log`, `console.error` |
| `expr as T` | `Number()`, `Math.trunc(Number())`, `String()` |
| `a / b` na `Int` | dzielenie całkowite (obcina, panic przy `b == 0`) — wybór po typie z `typeinfer` |
| `==` na listach/struct/enum | porównanie strukturalne (`hks_eq`) |
| `.clone()` | głębokie kopiowanie (`hks_clone`) |

Funkcje biblioteki std (`str_join`, `str_split`, `str_trim`,
`str_starts_with`, `str_ends_with`, `str_replace`, `str_contains`,
`str_to_int`, `str_to_float`, `str_index_of`, `math_*`) mają
odpowiedniki w runtime. W Node dostępne są `args()`, `exit()`,
`read_file`, `write_file`, `file_readable`, `now_ms`, `sleep`.
`get <npm:pakiet>` → `const pakiet = require("pakiet")` (tylko Node).

**`native {JavaScript} [ ... ]`** (0.6): treść bloku trafia dosłownie do wyniku, w
zasięgu `{ ... }`, i widzi zmienne HackerScript pod ich nazwami JS. Używaj go jako
INSTRUKCJI (nie jako wyrażenia). Wariant z importem pliku
`native {JavaScript} <ścieżka.js>` wczytuje plik względem pliku `.hcs`. Inny silnik
(`native {C++}` itd.) daje czytelny błąd. Bloki z plików dołączonych przez `include`
działają (indeksy są globalne dla całego programu).

**Automatyczny `await` na kanałach** (0.6): `kanal.send(x)` i `kanal.recv()` w kontekście
`async` same dostają `await`; ręczny `await` nadal jest poprawny. Nie dotyczy metod o
nazwach `send`/`recv` zdefiniowanych w twoim `impl`. Uwaga: dla obcego obiektu JS z
metodą `send` `await` jest nieszkodliwy (await na nie-Promise zwraca wartość), ale
dodaje jedno przejście pętli zdarzeń. Poza `async` (zwykła `fun`) `await` nie jest
dodawany — blokująca synchronizacja na kanale wymaga `async fun`.

**Dowolne API JavaScript** jest dostępne bez bloków `native`: nieznane
nazwy przechodzą przez bez zmian (`Math.floor(x)`, `document.title`,
`JSON.stringify(x)`).

Runtime jest dołączany kawałkami — tylko te, których używa program
(brak `async` w programie = brak kodu kanałów w wyniku). Cały program
trafia do IIFE, więc jego nazwy nie kolidują z globalnymi
przeglądarki.

## Różnice względem Rusta (backend `hackerc` → Rust)

- **`Int` to liczba zmiennoprzecinkowa (IEEE-754):** dokładna do 2^53.
  Rust (i64) zawija się przy przepełnieniu, JS traci precyzję — programy
  z haszowaniem na dużych liczbach dadzą inny wynik.
- panic: komunikat `hks panic: ...` na stderr, kod wyjścia 101 (Node)
- `.len()` na napisie liczy bajty UTF-8 jak Rust; `char_at`/`slice`
  operują na znakach Unicode / jednostkach UTF-16 (różnica tylko dla
  znaków spoza BMP)
- niewspierane (czytelny błąd, nie crash): `region [...]` (FFI),
  `direct`, `native` z silnikiem innym niż JavaScript, `get <...>` poza `std`/`core`/`npm`
- `for x in range(a, b)` jest obsługiwane jako rozszerzenie (w Rust
  `range` nie istnieje)

## Weryfikacja

Programy wspólne dla obu backendów dały identyczne wyjście pod `hackerc`
(Rust) i pod Node: `control`, `recursion`, `logic`, `scope`, `strings`
oraz `structs`, `enums`, `lists`, `options` (struct/impl/enum/match/
listy/Dict/Option/Result/`?`). Programy testowe nie są częścią tego repozytorium, więc to porównanie nie jest odtwarzalne
automatycznie (patrz `docs/ROADMAP.md`). Przykłady z `docs/js-examples/` (`native_js`, `channels`) są
budowane i uruchamiane pod Node przez `scripts/verify.sh`.
Programy z `Int` > 2^53 (`arith`, `algos`) różnią się celowo — patrz wyżej.
