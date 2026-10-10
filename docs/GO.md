# `get <go:...>` i `native {go} [...]`

Status: **zaimplementowane, zweryfikowane end-to-end dla stdlib Go**
(Tura 10, realny `go 1.22` + `rustc`/`cargo`).

## Jak działa

* Wszystkie bloki `native {go} [ ... ]` projektu trafiają jako ciała
  funkcji `//export hks_native_go_N` do JEDNEGO pliku
  `<crate>/go/hks_go.go` (+ `go.mod`), zapisywanego przez `hackerc build`.
  Jeden plik, bo Go nie pozwala zlinkować dwóch archiwów `c-archive` z
  osobnymi runtime'ami.
* `bit build` woła `go build -buildmode=c-archive -o libhksgo.a .`,
  a `build.rs` linkuje je **statycznie** (`link-lib=static=hksgo` +
  `pthread`). Runtime Go jest w binarce — Go NIE jest potrzebny w
  runtime, tylko przy budowaniu.
* `get <go:pakiet>` dodaje `import "pakiet"` do pliku Go
  (`get <go:fmt>`, `get <go:strings>`). Zewnętrzne moduły:
  `get <go:github.com/google/uuid::v1.6.0>` → `go get <ścieżka>@<wersja>`
  przed budową (wpis `go_dep` w `.hks-extern-links.txt`).
* Wywołanie z Rusta: `extern "C" { fn hks_native_go_N(); }` — jak `{C++}`.

```
get <go:fmt>
fun main() [
    native {go} [
        fmt.Println("Czesc z Go!", 6*7)
    ]
]
```

## Przetestowane

`fmt`, `strings`, dwa bloki w jednym programie, pętle/zmienne lokalne;
wynik poprawny (`Czesc z Go! 42`, `HACKERSCRIPT`, `suma 1..100 = 5050`).

## Ograniczenia

* **Moduły zewnętrzne nieprzetestowane** — środowisko testowe blokuje
  `proxy.golang.org`; ścieżka `go get` jest zaimplementowana, ale nie
  wykonana realnie.
* Bloki nie przyjmują/nie zwracają parametrów (jak `{C++}`); komunikacja
  tylko przez efekty uboczne (stdout, pliki).
* Niewykorzystany import Go jest błędem kompilacji Go — deklaruj tylko
  te pakiety, których blok używa.
* Wymaga `go` + `gcc` (cgo) przy budowaniu. Linkowanie statyczne
  zwiększa binarkę o kilka MB (runtime Go).
