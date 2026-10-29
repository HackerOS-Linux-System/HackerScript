# `get <kotlin:...>` i `native {kotlin} [...]`

Status: **zaimplementowane i ZWERYFIKOWANE end-to-end** (realny `rustc`/
`cargo`/`kotlinc`, nie tylko czytanie kodu) w tej sesji.

## Jak dziala

`native {kotlin} [ ... ]` dziala DOKLADNIE jak `native {java} [ ... ]` -
ta sama embedded JVM (przez crate `jni`, `hks_get_jvm()` w
wygenerowanym Ruscie), bo Kotlin kompiluje sie do TEGO SAMEGO bajtkodu
JVM co Java. Roznice:

1. Surowy kod trafia do pliku `HksNativeKotlinN.kt` jako **funkcja
   najwyzszego poziomu** `fun run() { ... }` (nie metoda klasy - w
   Kotlinie funkcje nie musza byc w klasie). `kotlinc` kompiluje taki
   plik do klasy `HksNativeKotlinNKt` (konwencja "nazwa pliku + Kt").
2. `bit build` kompiluje `.kt` pliki `kotlinc`iem do TEGO SAMEGO
   `java_classes/` co `.java` (Kotlin i Java moga wolac nawzajem swoje
   klasy z jednego wspolnego classpath).
3. Bajtkod Kotlina wymaga `kotlin-stdlib.jar` na classpath W RUNTIME
   (nie tylko przy kompilacji) - `bit build` znajduje ja wzgledem
   polozenia samego `kotlinc` (`<KOTLIN_HOME>/lib/kotlin-stdlib.jar` -
   stala konwencja dystrybucji Kotlina, dziala niezaleznie od tego czy
   `kotlinc` pochodzi z apt, SDKMAN czy oficjalnego archiwum), kopiuje
   ja do `kotlin_classes/kotlin-stdlib.jar` OBOK finalnej binarki, a
   wygenerowany Rust dopisuje ja do `-Djava.class.path` (patrz
   `hks_get_jvm()` w transpiler.hcs).

`get <kotlin:sciezka.jar>` dziala jak `get <java:sciezka.jar>` -
dopisuje wpis do WSPOLNEGO classpath (`java_classpath_entries`), uzywany
i przy `javac -cp`, i przy `kotlinc -cp`.

## Zweryfikowane w tej sesji (realna kompilacja + uruchomienie)

Program testowy z `native {kotlin} [ println(...); listOf(1,2,3,4,5).sum() ]`:
- `hackerc build` -> poprawny `.kt` + wywolanie JNI w Ruscie
- `bit build` -> `kotlinc` znaleziony i wywolany, `kotlin-stdlib.jar`
  znaleziony i skopiowany, `cargo build` -> dzialajaca binarka
- **Uruchomienie binarki dalo POPRAWNY wynik**: `Czesc z Kotlina! 2 + 2
  = 4`, `suma listy: 15` - kod Kotlina faktycznie sie wykonal, ze
  stdlib (`listOf`/`.sum()`) dzialajacym poprawnie.

## Znane ograniczenia

- `find_kotlin_stdlib_jar` uzywa `readlink -f` (dziala na Linux; BSD
  `readlink` na macOS nie ma `-f` - wtedy czytelny blad zamiast cichej
  awarii w runtime).
- Runtime classpath dla `get <java:...>`/`get <kotlin:...>` - **naprawione w
  0.6**: wygenerowany Rust dopisuje teraz kazdy wpis do `-Djava.class.path`
  (dwa kandydaci: sciezka dokladnie jak w zrodle oraz `java_classes/<nazwa
  pliku>` obok binarki; JVM ignoruje nieistniejace wpisy). Zweryfikowane
  transpilacja i `cargo check`, NIE uruchomieniem z JVM. Zeby drugi kandydat
  mial efekt, `bit build` musi kopiowac jary do `java_classes/` (kod `bit`,
  poza tym repozytorium).

# Backend `use <lang:kotlin>` (`hackerc kotlin`) — zakres

To jest **osobna rzecz** niż `native {kotlin}` wyżej: cały program HackerScript jest
tłumaczony na `Main.kt` (zamiast na Rust). Backend jest **eksperymentalny** i obejmuje
podzbiór języka. Poniższa lista pochodzi z kodu `kotlin_backend.hcs` i z kompilacji
wyniku prawdziwym `kotlinc` (konstrukcje oznaczone ✔ zostały przetłumaczone, skompilowane
i uruchomione).

## Obsługiwane

| Konstrukcja | Wynik w Kotlinie | |
|---|---|---|
| `fun` (także `async fun`, bez `suspend`) | `fun` | ✔ |
| `let` / `const` z wartością początkową | `var` / `val` | ✔ |
| przypisania, `+ - * / %`, porównania, `and`/`or`/`not` | operatory Kotlina | ✔ |
| `if` / `elif` / `else`, `while`, `for x in lista` | to samo | ✔ |
| `return`/`end`, `break`, `continue` | to samo | ✔ |
| `struct` | `data class` (pola `var`) | ✔ |
| `List<T>`: literał, `.push`, `.len()`, `lista[i]` | `MutableList`, `.add`, `hksLen`, `[i.toInt()]` | ✔ |
| `Option<T>`: `some`/`none`, `match` z `Some(x)`/`None` | typ nullable `T?` | ✔ |
| `Dict<K, V>`: `dict()`, `.insert`, `.fetch`, `.contains`, `.len()` (0.6) | `MutableMap` | ✔ |
| `log(a, b, ...)`, `elog(...)` | `println(...)` / `System.err.println(...)` (wiele argumentów łączone spacją — 0.6) | ✔ |
| `x as Int/Float/Str` | `.toLong()` / `.toDouble()` / `.toString()` | ✔ |
| `chan(n)`, `.send`, `.recv`, `spawn`, `await` | `HksChan<Any?>`, `CompletableFuture` | częściowo (patrz niżej) |

## Odrzucane z czytelnym błędem (`lang:kotlin: ... nieobsługiwane`)

`enum` · `impl` (metody) · operator `?` i `Result` · `let x` bez wartości początkowej ·
`match` na wariantach innych niż `Some`/`None` · `native` / `direct` · `manual` ·
`region [ ... ]` (FFI) · `get <...>` (importy).

Program zawierający konstrukcję spoza listy **nie generuje częściowego wyniku** —
`hackerc kotlin` kończy się błędem.

## Różnice względem backendu Rust (świadome / znane)

* `Int` → `Long`, `Float` → `Double`. `Float` wypisywany przez `log` ma zawsze kropkę
  (`3.0`), Rust wypisuje `3`.
* `.len()` na napisie to `String.length` (jednostki UTF-16); w Ruście — bajty UTF-8.
  Różnica tylko dla znaków spoza ASCII.
* Dzielenie przez zero rzuca `ArithmeticException` (w Ruście — panic z innym komunikatem).
* Kanały: typ elementu to zawsze `Any?` (Kotlin nie wnioskuje `HksChan<T>` z `chan(1)`,
  a `HksChan<Long>` i `HksChan<Any>` są niezgodne), więc odebraną wartość trzeba w
  Kotlinie rzutować. Kompilację programu z kanałami sprawdzono wyłącznie dla przykładu
  bez użycia odebranej wartości.
* Nie ma garbage-collectora ani `Drop` z Rusta — to zwykły GC JVM.

## Weryfikacja

`scripts/verify.sh` buduje i uruchamia `docs/kotlin-examples/*.hcs` (gdy w systemie jest
`kotlinc` i `java`) i porównuje wyjście z `*.out`.
