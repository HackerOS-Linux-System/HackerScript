# `hack3rc` — plan (SZKIELET, implementacja niezaczeta)

Status: **manifest + rejestracja w workspace zrobione w tej sesji**
(`hack3rc/Virus.hk`, `hack3rc/Virus.hk` w `[workspace] -> members`). Kod
(`hack3rc/cmd/main.hcs`) **NIE zostal napisany** — celowo: bez dzialajacego
toolchaina Rust w srodowisku pracy tej sesji nie bylo mozliwosci
skompilowac/zweryfikowac choc raz wygenerowanego kodu wywolujacego
Cranelift, a ten obszar (budowanie funkcji IR blok-po-bloku przez
`cranelift-frontend::FunctionBuilder`) jest na tyle niskopoziomowy, ze pisanie
go "na slepo", bez petli kompiluj-sprawdz, ma wysokie ryzyko subtelnych
bledow ktore wygladalyby na dzialajace, a nie sa.

## Architektura (niezmieniona wzgledem planu)

- `hack3rc` NIE ma wlasnego lexera/parsera/AST/typecheckera. Reużywa
  frontendu `hackerc` w calosci przez `include <work:hackerc::lexer>` /
  `<work:hackerc::ast_nodes>` / `<work:hackerc::parser>` (ten sam mechanizm
  co `playground/cmd/main.hcs`, dzialajacy dopiero po naprawie
  `find_workspace_root`/`work_module_file_path` w tej sesji).
- Jedyna nowa czesc to backend: zamiast `hackerc/cmd/codegen.hcs` (emitujacy
  TEKST Rusta), `hack3rc` ma miec `hack3rc/cmd/cranelift_codegen.hcs`
  emitujacy wywolania API `cranelift-jit`/`cranelift-object` (przez
  `native {Rust} [ ... ]`, tak jak inne bloki natywne w tym jezyku).

## Proponowany "Zakres 0.0.1" (nastepna sesja)

Waski, ale kompletny pionowy przekroj: JEDNA funkcja `fun main() -> Int [ ... ]`
z:
- literałami calkowitymi,
- `let` z typem `Int`,
- operatorami `+ - * /`,
- `end <wyrazenie>` (return).

Wystarczajace, zeby zbudowac REALNY, dzialajacy `cranelift-jit` pipeline
(stworz `JITModule`, zdefiniuj funkcje, przetlumacz AST na `iconst`/`iadd`/
`isub`/`imul`/`sdiv`/zmienne przez `Variable`, `finalize_definitions`,
wywolaj przez wskaznik funkcji, wypisz wynik) — bez ryzyka utkniecia w
nieograniczonym zakresie calego jezyka.

## Co NIE jest jeszcze zaprojektowane

- Linkowanie `cranelift-object` wyjscia (`.o`) z reszta projektu (stringi,
  structy, wywolania `get <std:...>` itd.) — wymaga wlasnego ABI/calling
  convention zgodnego z tym, co generuje `hackerc/cmd/codegen.hcs` dla Rusta,
  zeby dwa swiaty (transpilowany-do-Rusta kod i JIT-owany-przez-Cranelift
  kod) mogly sie wolac nawzajem. To duzo wiekszy temat niz "Zakres 0.0.1".
- Strategia testowania bez dostepu do `cargo`/`rustc` w kolejnych sesjach —
  jesli srodowisko pracy w przyszlej sesji rowniez nie bedzie mialo
  toolchaina, warto rozwazyc: (a) pisanie kodu z WIEKSZA ostroznoscia i
  mniejszymi krokami, kazdy jawnie oznaczony jako niezweryfikowany, (b)
  poproszenie uzytkownika o wklejenie wyniku `cargo build`/`cargo test` z
  jego wlasnej maszyny miedzy sesjami, analogicznie do tego, jak w TEJ sesji
  uzytkownik wkleil log bledu `wasm-bindgen`, co pozwolilo na precyzyjna,
  zweryfikowana-przez-czytanie-kodu naprawe.

## Zakres 0.0.1 - ZROBIONE i zweryfikowane end-to-end (Tura 10)

Status zmieniony z "SZKIELET, implementacja niezaczeta" na
**zaimplementowane i dzialajace**. `hack3rc/cmd/main.hcs` parsuje
`fun main() -> Int [ ... ]` (podzbior: `let`/`end`, liczby calkowite,
identyfikatory, `+ - * /`), generuje program Rust z Cranelift JIT.

Zweryfikowane realnie: program `let x = 5  let y = x * 2 + 1  end y - 3`
-> `hack3rc prog.hcs -o out` -> `cd out && cargo run` -> wynik **`8`**
(poprawny). Szablon Cranelift (budowa IR ze stosowej listy instrukcji,
JIT, wywolanie) napisany recznie i zweryfikowany OSOBNO (skompilowany +
uruchomiony) PRZED wbudowaniem w generator - `hack3rc` nigdy go nie
modyfikuje, dokleja tylko literal `vec![...]`.

Napotkane problemy (i naprawy), przydatne dla kolejnych rozszerzen
zakresu:
- Lexer HackerScript przerywa string literal na doslownym znaku nowej
  linii - dlugi, wieloliniowy szablon Rust musi byc zapisany jako
  `List<Str>` linii + `str_join(..., "\n")`, NIE jeden literal.
- `List<Str>` jako parametr-akumulator przekazywany rekurencyjnie
  (`fun f(out: List<Str>)`, wywolywane jako `f(out)` z wnetrza innego
  wywolania `f`) daje w generowanym Ruscie `error[E0596]: cannot borrow
  as mutable` - kodegen nie robi automatycznego "reborrow". Zamiast
  tego: zwracac `Option<List<Str>>` z kazdej funkcji i laczyc listy u
  wywolujacego.
- `match` bez wariantu `_` (wildcard) na `enum` z wiecej niz
  wymienionymi wariantami daje niewyczerpujace dopasowanie w Rust -
  zawsze dodawac `_ -> [ ... ]` gdy nie obslugujemy wszystkich
  wariantow (np. `Stmt` ma dziesiatki wariantow, Zakres 0.0.1 obsluguje
  tylko 2).
- `args()` (codegen.hcs) JUZ pomija nazwe programu
  (`std::env::args().skip(1)`) - `argv[0]` to PIERWSZY prawdziwy
  argument.

## Nastepne kroki (rozszerzenie zakresu)

Naturalny nastepny krok: `if`/`while` (rozgalezienia w Cranelift IR -
`brif`/bloki), `Bool` (typ `I8` w Cranelift), wywolania funkcji
uzytkownika (wiecej niz jedna funkcja w pliku - wymaga tablicy funkcji
Cranelift, nie tylko jednej `hks_main`). String/struct to znacznie
wiekszy skok (wymaga ABI zgodnego z tym, co generuje
`hackerc/cmd/codegen.hcs` dla Rusta, zeby dwa swiaty mogly sie wolac
nawzajem - patrz oryginalny plan w tym dokumencie).
