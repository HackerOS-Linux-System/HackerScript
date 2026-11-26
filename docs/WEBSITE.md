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
| `dom_on(el, zdarzenie, funkcja)` / `dom_on_event(...)` | obsługa zdarzeń — funkcję podaje się **po nazwie** (może być `async`); handler dostaje zdarzenie jako argument (`fun f(e)`), a `fun f()` bez argumentu nadal działa. Błędy w handlerze trafiają do `console.error` (i do nakładki w trybie `--dev`) |
| `page_title(t)`, `page_css(css)` | tytuł i dodatkowy CSS |
| `http_get(url)`, `http_post(url, body)` | `async`, zwracają `Result<Str, Str>` (użyj `await`); błąd statusu to `"HTTP 404"` (patrz niżej: `http_request`, `http_error_code`) |
| `storage_get(k)` → `Option<Str>`, `storage_set(k, v)` | `localStorage` — bezpieczne: gdy przeglądarka go blokuje (tryb prywatny, `file://`), dane trzymane są w pamięci strony |
| `set_timeout(f, ms)`, `set_interval(f, ms)`, `alert(t)`, `random()`, `now_ms()`, `sleep(ms)` | pomocnicze |

Reszta Web API jest dostępna wprost, bo nieznane nazwy przechodzą przez
bez zmian (`document.title`, `window.location`, `Math.floor(x)`,
`console.log`…). Stan trzymaj w zmiennych modułu (`let licznik = 0` na
poziomie pliku) — funkcje obsługi zdarzeń mogą je zmieniać.

## Routing (0.6)

Routing po `#hash` (działa także z `file://`, bez serwera): ścieżka `#/o-nas`
wywołuje zarejestrowaną funkcję i podmienia zawartość strony.

| Funkcja | Działanie |
|---|---|
| `route("/sciezka", funkcja)` | rejestruje handler (podany **po nazwie**, może być `async`); segment `:nazwa` łapie parametr, np. `"/user/:id"` |
| `route_not_found(funkcja)` | handler dla nieznanej ścieżki (domyślnie wypisuje `404: ścieżka`) |
| `route_param(nazwa)` → `Option<Str>` | parametr z ostatnio dopasowanej ścieżki |
| `route_current()` → `Str` | bieżąca ścieżka (bez `?zapytania`) |
| `route_outlet()` | element, do którego renderują handlery: `#route-outlet`, a gdy go nie ma — `#app` |
| `navigate("/sciezka")` | zmienia `location.hash` |

Przy każdej zmianie ścieżki router **czyści** `route_outlet()` i woła handler. Wspólny
układ (menu, stopka) zrób w `main()` poza elementem `#route-outlet`. Router startuje
automatycznie po zakończeniu `main()` (także `async fun main()`), tylko gdy
zarejestrowano co najmniej jedną trasę. Przykład: `docs/website-routing/app.hcs`.

Czego NIE ma: routingu po zwykłych ścieżkach (`/o-nas` bez `#`; wymagałoby serwera lub
`history.pushState` z konfiguracją hosta), renderowania po stronie serwera (SSR)
i zagnieżdżonych tras. Routing sprawdzono w **jsdom** (`scripts/verify_website.sh`),
nie w prawdziwej przeglądarce.

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

## Rozszerzenia mostka (ta runda)

**Zdarzenia.** `dom_on_event(el, "keydown", on_key)` z `fun on_key(e)`; pomocnicze:
`event_key(e)`, `event_code(e)`, `event_type(e)`, `event_prevent(e)` (= `preventDefault`), `event_stop(e)`,
`event_target(e)`, `event_current(e)`, `event_value(e)`, `event_checked(e)`, `event_mod(e, "ctrl"|"shift"|"alt"|"meta")`.
`dom_off(el, zdarzenie, funkcja)` zdejmuje handler (bez `funkcja` — wszystkie dla tego zdarzenia).
`dom_delegate(rodzic, selektor, zdarzenie, f)` — jeden handler dla wielu dzieci; `f(e, element)`.

**DOM.** `dom_class_remove`, `dom_class_toggle(el, k[, wymus])`, `dom_class_has`, `dom_remove(el)`,
`dom_get_attr(el, k)` → `Option<Str>`, `dom_remove_attr`, `dom_query_all(sel)`, `dom_focus`,
`dom_checked`/`dom_set_checked`, `clear_timeout(id)`, `clear_interval(id)`,
`html_escape(s)` — **używaj przed `dom_html`** z danymi od użytkownika (XSS).
`dom_clear(el)` czyści przez `textContent` (usuwa wszystkie dzieci i ich handlery razem z elementami).

**Nagłówek strony w czasie działania.** `page_lang`, `page_meta(nazwa, treść)`, `page_og(właściwość, treść)`,
`page_theme_color`, `page_favicon`. Dla SEO/podglądów linków używaj wersji statycznej (blok `head`, niżej) —
roboty nie wykonują JavaScriptu.

**Sieć.** `http_request(metoda, url, naglowki, body)` (`naglowki` to `dict()`), `http_post_json(url, tekst)`,
`http_json(url)` (sprawdza, że odpowiedź to poprawny JSON), `json_get(tekst, "a.b.0")` → `Option<Str>`,
`http_last_status()`, `http_last_body()`, `http_error_code(błąd)` → `Int` (np. 404; 0 = błąd sieci).
Błąd statusu ma stały format `"HTTP <kod>"`.

**Storage.** `storage_remove`, `storage_clear`, `storage_available()`; sesyjne: `session_get/set/remove/clear`.

**Router.** `route_query("klucz")` → `Option<Str>` (z `?klucz=wartość`), trasa z `*` na końcu (`/docs/*`) i `route_rest()`,
`before_route(f)` (`f(nowa, stara)` zwraca `false`, by anulować — adres wraca do poprzedniego), `on_leave(f)`,
zagnieżdżone trasy: `route_layout("/admin", f)` — `f` rysuje ramkę w `route_outlet()` i wywołuje
`route_set_outlet(element)`, by wskazać, gdzie trafia treść podtrasy (układy od zewnętrznego do wewnętrznego).
`nav_link("/sciezka", "Tekst")` / `nav_child(rodzic, ...)` — odnośnik z klasą `nav-link`, który dostaje klasę
`active` i `aria-current="page"` dla bieżącej trasy. `navigate_replace(sciezka)`.
**Tryb `history`:** `route_mode("history")` na początku `main()` — adresy bez `#` (`pushState`). Na `file://`
automatycznie wraca do trybu `#`. Hosting statyczny potrzebuje przekierowania, więc kompilator sam zapisuje
`404.html` (wariant GitHub Pages); jeśli strona stoi w podkatalogu domeny, podaj `--base /nazwa-repo`.

**Sygnały (reaktywność).**
```
let licznik = signal(0)
fun main() [
    let p = child(app_root(), "p", "")
    bind_text(p, licznik)               !! tekst aktualizuje się sam
    let b = child(app_root(), "button", "+1")
    dom_on(b, "click", plus)
]
fun plus() [ sig_set(licznik, sig_get(licznik) + 1) ]
```
`signal(v)`, `sig_get(s)` (w `effect` zapisuje zależność), `sig_peek(s)`, `sig_set(s, v)`, `sig_update(s, f)`,
`sig_push(s, x)`, `sig_remove_at(s, i)`, `effect(f)` (uruchamia od razu i po każdej zmianie odczytanych sygnałów),
`effect_stop(e)`, `computed(f)`, `sig_flush()`; powiązania: `bind_text`, `bind_attr(el, nazwa, s)`,
`bind_class(el, klasa, s)`, `bind_show(el, s)`, `bind_value(el, s)` (dwukierunkowo), `bind_checked`,
`bind_list(rodzic, s, render)` gdzie `render(element, indeks)` zwraca węzeł (renderuje listę od nowa przy zmianie — bez kluczy).
Aktualizacje są łączone (batch) i wykonują się w mikrozadaniu; `sig_flush()` wymusza natychmiast. Zależności
po `await` wewnątrz `effect` nie są śledzone. Cykl (efekt zmieniający własne zależności) jest przerywany po 1000
przebiegach z komunikatem. **Komponent** to zwykła funkcja zwracająca element: `fun karta(tytul) [ let k = el("div", ""); ...; end k ]`.

## Budowanie strony: `hackerc website`

```
hackerc website a.hcs [b.hcs ...] [-o katalog] [--minify] [--dev] [--watch] [--ssr] [--base /sciezka] [--assets katalog]
```
- **Wiele stron:** każde wejście → `<nazwa>.html` + `<nazwa>.js`; wejście o nazwie `index` (albo pierwsze) → `index.html` + `app.js`.
- **`use <site:lang=en>`** — atrybut `<html lang>` (domyślnie `pl`).
- **Blok `head { ... }`** (na początku linii, jak `style`): surowy HTML wstawiany do `<head>` — `<meta name="description" ...>`,
  `<link rel="icon" ...>`, `og:*`, `theme-color`. Klamry w środku muszą być zbalansowane.
- **CSS zakresowy `style(.karta) { ... }`:** każdy selektor dostaje przedrostek `.karta ` (`:root`/`body`/`&` oznaczają sam
  element zakresu; reguły w `@media`/`@supports` też; `@keyframes`/`@font-face` bez zmian).
- **Assety:** katalog `public/` obok pliku wejściowego jest kopiowany do korzenia wyniku, `assets/` → `assets/`, `--assets DIR` → `DIR/`.
- **`--minify`:** CSS (bez komentarzy i zbędnych spacji; napisy nietknięte) oraz JS (tylko usunięcie wcięć, pustych linii i
  linii-komentarzy; pliki z backtickiem/`native` zostają bez zmian). To NIE jest minifikator z przemianowywaniem nazw.
- **`--dev`:** nakładka z błędami na stronie (wyjątki z handlerów i `unhandledrejection`), oraz plik `__hks_version` do
  automatycznego przeładowania. **`--watch`:** przebudowuje po zmianie źródeł (odpytywanie co 0,5 s; nie obserwuje assetów)
  i włącza `--dev`; serwer uruchom osobno: `python3 -m http.server -d <katalog>`.
- **`--ssr` (0.7):** prerendering. Po zbudowaniu kompilator uruchamia Node + jsdom, wykonuje
  `app.js` i wstawia wyrenderowaną zawartość `#app` do HTML (`<div id="app" data-hks-ssr>`), więc roboty i
  podglądy linków widzą treść, a pierwszy ekran jest od razu. W trybie `route_mode("history")` powstaje
  dodatkowo `<trasa>/index.html` dla każdej trasy **statycznej** (bez `:param` i `*`); `--base` jest uwzględniany.
  Klient po starcie czyści prerenderowaną zawartość (`app_root()` usuwa `data-hks-ssr`) i renderuje od nowa —
  to prerendering, **nie** pełna hydratacja (jest krótki błysk przerenderowania, stan nie jest przejmowany).
  Wymaga `jsdom` (`npm i jsdom` obok projektu albo `npm i -g jsdom`); bez Node/jsdom budowanie kończy się
  ostrzeżeniem, a strona działa bez prerenderingu. Prerender wykonuje kod `main()` w jsdom, więc nie używaj
  w nim API niedostępnych poza przeglądarką bez zabezpieczenia (`fetch` do zasobów itp.).
- **Ostrzeżenia przy budowaniu:** nieznana funkcja o prefiksie mostka (`dom_tekst` → „czy chodziło o `dom_text`?”),
  literały całkowite > 2^53 (w JS `Int` to `float`), klasy użyte w kodzie bez selektora w blokach `style`.

`hackerc fmt` zachowuje bloki `style`/`head` dokładnie tak, jak je napisano (z nagłówkiem `style(.x)`), ale zbiera je
na końcu pliku.

## Weryfikacja

Blok `style { }` sprawdzono na przykładzie z `docs/website-style/` (CSS ze stringami i komentarzami
zawierającymi klamry, `@media`, kilka bloków, `fmt` idempotentny) oraz na prawdziwej stronie
(porównanie wstawionego CSS z oryginalnym arkuszem). Nie testowano w prawdziwej przeglądarce.

**Weryfikacja 0.7:** strona z `docs/website-app` została zbudowana kompilatorem 0.6 (zbudowanym ze źródeł) i
uruchomiona w jsdom (router w trybie `#`, `nav_link`, sygnały). `--ssr` sprawdza `tests/website/run.sh` (tryb
`history`: prerender tras statycznych, pominięcie tras z parametrem, brak dublowania po starcie klienta).
Nadal **nie** testowano w prawdziwej przeglądarce (Playwright) ani na urządzeniach mobilnych.

Przykładowa strona (struct + `impl`, lista, zmienne globalne, zdarzenia
`click`, pole `input`, `async fun`) została uruchomiona w **jsdom**
(symulacja przeglądarki w Node): sprawdzono tytuł, nagłówek, liczbę
przycisków, licznik po 3 kliknięciach, dodawanie elementów listy z
pola tekstowego i czyszczenie pola. Nie testowano w prawdziwej
przeglądarce ani na urządzeniach mobilnych.

## Dane i czytelnosc (0.7)

Tryb `website` **odrzuca** skladnie, ktora konczy sie plikami z liniami po dziesiatki tysiecy znakow (cale tresci jako
zagniezdzone `mkobj([...])` w jednej linii). To sa **bledy** - `hackerc build`/`hackerc website` konczy sie kodem 2:

| Kod | Regula |
|---|---|
| `E0801` | linia dluzsza niz 160 znakow |
| `E0802` | wiecej niz 5 poziomow `(`/`[` w jednej linii |
| `E0803` | dwa literaly napisowe sklejone plusem (`"a" + "b"`) - slowa kluczowe w napisach nie wymagaja obejsc (`"direct [ x ]"` dziala) |
| `E0804` | obiekt zapisany jako plaska lista par `mkobj(["klucz", wartosc, ...])` |

**Zamiast tego - tresc w plikach danych.** Pliki `data/*.json` (obok pliku wejsciowego albo w korzeniu projektu) sa
wstawiane do `app.js` i czytane w programie:

```
use <mode:website>

fun main() [
    match data_get("articles") [               ;; data/articles.json -> Option<Any>
        Some(list) -> [
            let i = 0
            while i < data_len(list) [         ;; dlugosc tablicy
                match data_at(list, i) [       ;; Option<Any>
                    Some(a) -> [
                        match data_field(a, "id") [   ;; pole obiektu -> Option<Any>
                            Some(id) -> [ child(app_root(), "p", id) ]
                            None -> [ ]
                        ]
                    ]
                    None -> [ ]
                ]
                i = i + 1
            ]
        ]
        None -> [ ]
    ]
]
```

Plik JSON musi byc poprawny (kompilator wstawia go doslownie). Dla tresci wielojezycznych: jeden plik na dziedzine
(`articles.json`) z kluczami `pl`/`en` w obiektach - latwo diffowac, formatowac i edytowac narzedziami do JSON.
Test: `tests/website/errors` (oczekiwane E0801/E0803/E0804) i `tests/website/data`.

## Ograniczenia

- SSR to prerendering (bez hydratacji, patrz `--ssr`); jedna strona na plik wejściowy (wiele podstron = wiele plików wejściowych w jednym wywołaniu `hackerc website`);
  `bind_list` renderuje listę od nowa (bez kluczy); `--minify` JS jest tylko liniowy
- `native {...}` i `get <npm:...>` niedostępne w trybie strony
- ograniczenia backendu JavaScript z `docs/JAVASCRIPT.md` (m.in.
  `Int` = liczba zmiennoprzecinkowa, kanały wymagają `await`)
