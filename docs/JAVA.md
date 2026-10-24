# Java w HackerScript: `get <java:...>` i `native {java}`

`native {java} [ ... ]` osadza kod Javy w programie HackerScript. Blok jest kompilowany do
syntetycznej klasy `HksNativeJavaN` (`javac`), a w czasie działania binarki uruchamiany w
osadzonej JVM (`hks_get_jvm()`, crate `jni` 0.21 z cechą `invocation`, zależność zwykła, nie
build-dependency).

* `get <java:sciezka.jar>` dopisuje jar do classpath: przy kompilacji (`javac -cp`) i w
  runtime (`-Djava.class.path`; runtime od 0.6 - wczesniej wpis trafial tylko na classpath
  kompilacji, patrz `docs/KOTLIN.md`). To **ścieżka do pliku**, nie nazwa biblioteki systemowej, i
  samo `get` nie tworzy żadnych sygnatur - do wywołań służy wyłącznie `native {java}`.
* Wpisy classpath mogą się powtarzać; `bit build` je odduplikowuje.
* Kotlin używa tego samego mechanizmu (patrz `KOTLIN.md`): ten sam bajtkod, inny kompilator.

Ograniczenia: wymaga JDK przy budowaniu i JVM przy uruchamianiu; kod jest zbudowany tylko
wtedy, gdy plik faktycznie zawiera blok `native {java}` (inaczej `jni` nie trafia do `Cargo.toml`).
