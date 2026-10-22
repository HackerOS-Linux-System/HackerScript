# Wydajność transpilacji i cache budowania (0.6)

## Pomiary (hackerc transpiluje własne źródła, 1 rdzeń)

| | czas |
|---|---|
| 0.5 (stage0 z Releases) | ~2 min 51 s |
| 0.5 (zbudowany ze źródeł) | ~1 min 56 s |
| **0.6** | **~19–25 s** |
| 0.6, ponowny build bez zmian (cache) | ~3 s (odkrycie plików + hash) |
| mały projekt, trafienie w cache | kilka ms |

## Co przyspieszono

* `CodeGen::infer_type` nie buduje już `TypeEnv(sigs.clone(), vars.clone())` na każde wywołanie (głęboki
  klon tablicy sygnatur był ~50% czasu). Rdzeń to `infer_expr_type_sv(expr, sigs, vars)`;
  `infer_expr_type(expr, env)` zostało jako cienki wrapper. To samo w `typecheck.hcs` i backendzie JS.
* `s = s + x` dla `Str` → `s.push_str(..)` zamiast `format!("{}{}", s, x)` (kopiowało cały napis; O(n²)).
  Wzorzec rozpoznaje `CodeGen::try_gen_str_append`; wymaga znanego typu `Str` zmiennej.
* Lekser: `style_block_open` (materializuje cały `Vec<char>`) wołany tylko przy słowie `style`.

## Cache budowania

* Klucz: sól formatu + parametry (`entry`, `crate_name`, `--lib`, `bin_name`, wersja) + **hash treści każdego
  odkrytego pliku** (razem z `std`/`core`).
* Zapis: `<out>/.hks-cache/build.txt` — tylko po buildzie bez ostrzeżeń. Trafienie wymaga też, by
  `main.rs`, `Cargo.toml` i moduły nadal istniały.
* Pliki wynikowe są zapisywane tylko przy zmianie treści (`write_file_if_changed`), więc `cargo` nie
  przebudowuje niezmienionego.
* Wyłączenie: `HACKERC_NO_CACHE=1`. Czyszczenie: `rm -rf <out>/.hks-cache`.
* Zmiana formatu wyników transpilacji ⇒ podbij `HKS_CACHE_SALT` w `project.hcs`.
* Zmiana samego komentarza unieważnia cache (hashujemy surową treść).
