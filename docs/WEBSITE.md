# `use <mode:website>` — HackerScript dla stron

Tryb `website` kompiluje program HackerScript do **statycznej strony**:
`index.html` + `app.js` (ten sam backend co `lang:javascript`, plus
mostek DOM). Nie ma serwera ani bundlera — wynik otwiera się w
przeglądarce (także z `file://`).

```
use <mode:website>

fun on_click() [
    dom_text(dom_get("out"), "kliknieto")
]

fun main() [
    page_title("Moja strona")
    let root = app_root()
    child(root, "h1", "Czesc!")
    let btn = child(root, "button", "Kliknij")
    let out = child(root, "p", "")
    dom_attr(out, "id", "out")
    dom_on(btn, "click", on_click)
]
```
```
hackerc build strona.hcs -o dist        # dist/index.html + dist/app.js
hackerc website strona.hcs -o dist      # to samo, jawna podkomenda
```

`use <mode:website>` implikuje backend JavaScript (`use <lang:javascript>`
można dopisać jawnie). Bez `-o` wynik trafia do `<nazwa>-site/`.
`main()` uruchamia się po załadowaniu DOM; `async fun main()` też działa.

## Mostek DOM (runtime strony)

| Funkcja | Działanie |
|---|---|
| `app_root()` | element `#app` (kontener strony) |
| `el(tag, tekst)` | nowy element (z tekstem) |
| `child(rodzic, tag, tekst)` | tworzy element, dołącza do rodzica i zwraca go |
| `dom_get(id)`, `dom_query(selektor)` | wyszukiwanie |
| `dom_create(tag)`, `dom_append(rodzic, dziecko)`, `dom_clear(el)` | struktura |
| `dom_text(el, t)`, `dom_html(el, html)` | zawartość |
| `dom_attr(el, nazwa, wartosc)`, `dom_style(el, wlasciwosc, wartosc)`, `dom_class(el, klasa)` | atrybuty i style |
| `dom_value(el)`, `dom_set_value(el, v)` | pola formularzy |
| `dom_on(el, zdarzenie, funkcja)` | obsługa zdarzeń — funkcję podaje się **po nazwie** (może być `async`) |
| `page_title(t)`, `page_css(css)` | tytuł i dodatkowy CSS |
| `http_get(url)`, `http_post(url, body)` | `async`, zwracają `Result<Str, Str>` (użyj `await`) |
| `storage_get(k)` → `Option<Str>`, `storage_set(k, v)` | `localStorage` |
| `set_timeout(f, ms)`, `set_interval(f, ms)`, `alert(t)`, `random()`, `now_ms()`, `sleep(ms)` | pomocnicze |

Reszta Web API jest dostępna wprost, bo nieznane nazwy przechodzą przez
bez zmian (`document.title`, `window.location`, `Math.floor(x)`,
`console.log`…). Stan trzymaj w zmiennych modułu (`let licznik = 0` na
poziomie pliku) — funkcje obsługi zdarzeń mogą je zmieniać.

## Style: blok `style { ... }`

Stronę stylujesz **bez osobnego pliku `.css`** — blokiem `style { ... }` prosto w `.hcs`.
Wewnątrz jest zwykły CSS (reguły, `@media`, `@keyframes`, `@import`, zmienne `--x`,
komentarze `/* */`):

```
use <mode:website>

style {
    :root { --akcent: #38c8ff; }
    body  { margin: 0; background: #08040d; color: white; }
    .karta:hover { border-color: var(--akcent); }
    @media (max-width: 760px) { .karta { padding: 12px; } }
}

fun main() [
    let k = child(app_root(), "div", "Czesc!")
    dom_attr(k, "class", "karta")
]
```

Zasady:

- `style {` musi stać **na początku linii** (przed nim tylko białe znaki). `style {` w stringu albo
  w komentarzu `!!` / `!> <!` nie otwiera bloku.
- Blok kończy pasujące `}`; klamry w stringach CSS (`content: "}"`) i w komentarzach `/* } */` nie
  liczą się do zagnieżdżenia. Nie zamknięty blok jest zwykłym błędem składni.
- W środku obowiązuje składnia CSS, nie HackerScript — komentarze piszesz jako `/* ... */`
  (`!!` byłoby tekstem CSS).
- Bloków może być wiele, także w plikach wciągniętych przez `include <...>`. Kolejność
  w kaskadzie: najpierw style plików z `include`, na końcu style pliku wejściowego.
- Kompilator wstawia CSS do **statycznego** `<style>` w `index.html`, **po** domyślnym stylu
  szkieletu (`body`, `#app` — dlatego możesz go nadpisać, np. `#app { max-width: none; }`).
  Nic nie jest wstrzykiwane z JavaScriptu, więc nie ma błysku niestylowanej strony.
- Dla `lexera`, `check`, `build` i `lsp` blok jest niewidzialny (jak komentarz).
  `hackerc fmt` zachowuje treść CSS bez zmian i przenosi bloki (połączone w jeden) na koniec pliku.
- Poza `mode:website` bloki są ignorowane (`hackerc js` ostrzega).

Dynamicznie (w czasie działania) nadal możesz użyć `dom_style`, `dom_class`, `page_css(css)`.

## Weryfikacja

Blok `style { }` sprawdzono na przykładzie z `docs/website-style/` (CSS ze stringami i komentarzami
zawierającymi klamry, `@media`, kilka bloków, `fmt` idempotentny) oraz na prawdziwej stronie
(porównanie wstawionego CSS z oryginalnym arkuszem). Nie testowano w prawdziwej przeglądarce.

Przykładowa strona (struct + `impl`, lista, zmienne globalne, zdarzenia
`click`, pole `input`, `async fun`) została uruchomiona w **jsdom**
(symulacja przeglądarki w Node): sprawdzono tytuł, nagłówek, liczbę
przycisków, licznik po 3 kliknięciach, dodawanie elementów listy z
pola tekstowego i czyszczenie pola. Nie testowano w prawdziwej
przeglądarce ani na urządzeniach mobilnych.

## Ograniczenia

- tylko `index.html` + `app.js` (brak routingu, SSR); jedna strona na plik wejściowy
  (wiele podstron = wiele wejść `hackerc build`, każde do własnego katalogu `-o`);
  styl przez blok `style { ... }`, `page_css(...)`/`dom_style(...)` albo własny `<link>`
  dopisany do wygenerowanego `index.html`
- `native {...}` i `get <npm:...>` niedostępne w trybie strony
- ograniczenia backendu JavaScript z `docs/JAVASCRIPT.md` (m.in.
  `Int` = liczba zmiennoprzecinkowa, kanały wymagają `await`)
