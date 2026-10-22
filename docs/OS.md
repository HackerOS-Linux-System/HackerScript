# `use <os:...>` — system docelowy (0.6)

```
use <os:linux>
use <os:windows>
use <os:macos>
use <os:ios>
use <os:android>
```

Dyrektywa na poziomie projektu (jak `use <arc:...>`, `use <crosscompile>`). Można podać kilka — projekt
deklaruje wtedy, że wspiera **wszystkie** wymienione systemy.

## Co robi `hackerc`

1. **Walidacja** — nieznana wartość daje ostrzeżenie (`znane: linux, windows, macos, ios, android`).
2. **Guard w wygenerowanym kodzie** — do `main.rs` trafia
   `#[cfg(not(any(target_os = "...", ...)))] compile_error!(...)`. Build dla systemu spoza listy
   kończy się czytelnym błędem kompilacji Rusta.
3. **Sidecar `.hks-use.txt`** — linie `os<TAB>linux` itd. Gdy zadeklarowano **dokładnie jeden** system,
   dochodzi wskazówka `target<TAB><triple>` (architektura z `use <arc:...>`; domyślnie `x86_64`,
   a dla `ios`/`android` — `aarch64`).
4. **`target_os()`** — wbudowana funkcja zwracająca system docelowy programu
   (`"linux" | "windows" | "macos" | "ios" | "android"`), także przy cross-kompilacji.
   W backendzie JS: nazwa platformy Node albo `"web"`.
5. **`get <std:os>`** — predykaty: `os_is_linux/windows/macos/ios/android`, `os_is_unix`, `os_is_mobile`,
   `os_path_separator`, `os_exe_suffix`, `os_shared_lib_ext`.

## Mapowanie na triple (wskazówka dla `bit`)

| os | x86_64 | aarch64 |
|---|---|---|
| linux | `x86_64-unknown-linux-gnu` | `aarch64-unknown-linux-gnu` |
| windows | `x86_64-pc-windows-gnu` | `aarch64-pc-windows-msvc` |
| macos | `x86_64-apple-darwin` | `aarch64-apple-darwin` |
| ios | `x86_64-apple-ios` | `aarch64-apple-ios` |
| android | `x86_64-linux-android` | `aarch64-linux-android` |

(`arc:armv7` + android → `armv7-linux-androideabi`.)

## Ograniczenia

* Deklaracja jest **na poziomie projektu** — brak warunkowej kompilacji pojedynczych plików.
* `hackerc` tylko zapisuje wskazówkę `target`; wybór `--target` w `cargo` należy do `bit`.
* `hack3rc` akceptuje dyrektywę, ale kieruje się własnym `--target`.
