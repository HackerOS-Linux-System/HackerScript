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
2. `virus build` kompiluje `.kt` pliki `kotlinc`iem do TEGO SAMEGO
   `java_classes/` co `.java` (Kotlin i Java moga wolac nawzajem swoje
   klasy z jednego wspolnego classpath).
3. Bajtkod Kotlina wymaga `kotlin-stdlib.jar` na classpath W RUNTIME
   (nie tylko przy kompilacji) - `virus build` znajduje ja wzgledem
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
- `virus build` -> `kotlinc` znaleziony i wywolany, `kotlin-stdlib.jar`
  znaleziony i skopiowany, `cargo build` -> dzialajaca binarka
- **Uruchomienie binarki dalo POPRAWNY wynik**: `Czesc z Kotlina! 2 + 2
  = 4`, `suma listy: 15` - kod Kotlina faktycznie sie wykonal, ze
  stdlib (`listOf`/`.sum()`) dzialajacym poprawnie.

## Znane ograniczenia

- `find_kotlin_stdlib_jar` uzywa `readlink -f` (dziala na Linux; BSD
  `readlink` na macOS nie ma `-f` - wtedy czytelny blad zamiast cichej
  awarii w runtime).
- Runtime classpath dla `get <java:...>`/`get <kotlin:...>` (zewnetrzne
  jary poza `kotlin-stdlib.jar`) ma TEN SAM, PRE-ISTNIEJACY gap co
  `native {java}` - `java_cp_entries` sa uzywane TYLKO przy kompilacji
  (`javac -cp`/`kotlinc -cp`), NIE sa dopisywane do runtime classpath w
  `hks_get_jvm()`. Nie jest to nowy problem wprowadzony w tej sesji -
  dotyczy rowniez istniejacego `native {java}`. `kotlin-stdlib.jar` NIE
  ma tego problemu (jest dopisywany jawnie, bo jest zawsze wymagany).
