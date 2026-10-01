# `using <wersja>` per plik — wsparcie wielu wersji jezyka w JEDNYM projekcie

Status: **zaimplementowane w tej sesji** (nieprzetestowane realnym `cargo build`
— brak toolchaina Rust w srodowisku pracy; przejrzane i uzasadnione czytaniem
kodu). Odpowiednik "edycji" z Rusta, ale **per plik**, nie per crate — i, w
odroznieniu od Rusta, faktycznie pobiera/uruchamia RÓŻNE binarki kompilatora
dla różnych plików tego samego builda.

## Jak to działa

1. Każdy plik `.hcs` MOŻE zaczynać się od `using <wersja>` (np. `using <0.5>`).
   Brak tej deklaracji = plik używa domyślnej wersji projektu
   (`Virus.hk` -> `[package] -> using => "0.4"`, domyślnie `0.4` jeśli pole
   nieobecne).
2. `virus build`:
   - skanuje CAŁY projekt (`virus/cmd/multiversion.hcs::scan_project_versions`)
     w poszukiwaniu wszystkich zadeklarowanych wersji,
   - łączy to z `Virus.hk -> [package] -> all-versions => [...]` (opcjonalne,
     jawne wyliczenie — przede wszystkim optymalizacja/dokumentacja, skan i tak
     wykrywa wszystko sam),
   - dla każdej znalezionej wersji woła `hackerc_ensure` (pobiera/buduje binarkę
     z cache, jak dotychczas robiono dla JEDNEJ wersji),
   - buduje mapę `wersja -> ścieżka binarki` i przekazuje ją do głównego
     wywołania `hackerc build` jako `--version-map wersja1=sciezka1;wersja2=sciezka2`.
3. `hackerc build` (dowolnej wersji, np. `0.4`), napotykając w Discovery plik
   deklarujący INNĄ wersję (np. `0.5`), której binarkę dostał w
   `--version-map`, **deleguje CAŁY plik** do tamtej binarki przez nowa
   podkomende `hackerc emit-module <plik> -o <plik.rs> --module-name <nazwa>`
   — ta inna wersja transpiluje plik SAMA (swoim własnym, ewentualnie innym
   parserem/typecheckerem), a wynikowy tekst Rust jest wklejany jako gotowy
   moduł do crate'a budowanego przez wersję domyślną.

## Dlaczego `using<...>` czyta się WYŁĄCZNIE przez `tokenize()`, nigdy `parse()`

Gdyby plik `using <0.5>` używał składni wprowadzonej w 0.5, a odczyt wersji
wymagałby pełnego `parse()` — hackerc 0.4 nie mógłby go nawet sparsować, żeby
DOWIEDZIEĆ się, że ma go zdelegować. To jajko i kura. Rozwiązanie: nagłówek
`using<...>` (i zasady tokenizacji/komentarzy) są **zamrożone na zawsze** —
jedyna część języka, która nigdy się nie zmienia między wersjami. Odczyt robi
`tokenize()` (lexer, stabilny) i ręcznie sprawdza pierwsze 2-3 tokeny.

## Czego NIE robi (świadome ograniczenia tej rundy)

- **Brak cross-file typecheck między wersjami.** Plik delegowany do innej
  wersji nie uczestniczy w `collect_project_signatures` tej domyślnej wersji
  (jego AST nie jest w ogóle parsowane przez domyślny `hackerc`) — jeśli
  odwołuje się do funkcji/structów z INNYCH plików projektu, to musi to zrobić
  przez mechanizmy, które i tak przechodzą przez granicę modułów Rust (`get
  <work:...>` do PLIKÓW WŁASNEJ wersji, albo `extern`/eksportowane publiczne
  `fun`/`struct` widoczne jako zwykły Rust) — nie przez sygnatury HackerScript
  współdzielone z resztą projektu.
- **`cmd_build_workspace_library_member`** (budowanie bibliotek-członków
  workspace) NIE przekazuje jeszcze `--version-map` — tylko główna ścieżka
  `cmd_build_run` (pojedynczy człon/projekt) to robi.
- **Brak testów end-to-end** — nie było możliwości uruchomienia `cargo
  build`/`hackerc` w tym środowisku (brak toolchaina). Logika była
  wyprowadzona z uważnego czytania istniejącego kodu (patrz komentarze `!!!`
  przy każdej zmianie), ale może zawierać błędy ujawniające się dopiero przy
  realnej kompilacji.

## Nowe/zmienione miejsca w kodzie

- `hackerc/cmd/project.hcs`: `peek_source_using_version`, `parse_version_map`,
  `Discovery.own_version`/`.version_binaries`, `DiscoveredFile.pre_transpiled_rust`,
  delegacja w `parse_and_add`, gałąź w pętli `build_project`.
- `hackerc/cmd/main.hcs`: `cmd_emit_module`, subkomenda `emit-module`, flaga
  `--version-map` dla `build`.
- `virus/cmd/multiversion.hcs` (NOWY): `scan_project_versions`,
  `ensure_all_versions`, `build_version_map_flag_value`,
  `peek_file_using_version_virus`, `find_hcs_files`.
- `virus/cmd/build.hcs`: `cmd_build_run` woła skan/ensure i przekazuje
  `version_map_str` do `hackerc_build_crate*`.
- `virus/cmd/hackerc_bridge.hcs`: `hackerc_build_crate*` przyjmują nowy
  parametr `version_map: Str`.
- `virus/cmd/manifest.hcs`: `PackageSection.all_versions`, parsowanie
  `-> all-versions => [...]`.
