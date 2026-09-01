(def stage (os/getenv "ZPM_PACKAGE_STAGE_DIR"))

(defn fail [msg]
  (eprint "recipe.janet: " msg)
  (os/exit 1))

(defn run [cmd]
  # `os/shell` zwraca kod wyjścia polecenia (jak C-owe system()) --
  # zero == sukces.
  (def code (os/shell cmd))
  (unless (zero? code)
    (fail (string "'" cmd "' zakończone kodem " code))))

(defn try-run [cmd]
  # Jak `run`, ale nie przerywa recipe przy niepowodzeniu -- zwraca
  # true/false. Do kroków, które są "najlepszym wysiłkiem" (kolejne
  # etapy łańcucha bootstrapu, z fallbackiem na końcu).
  (zero? (os/shell cmd)))

(defn shell-out [cmd]
  # Uruchamia polecenie i zwraca [ok stdout-przycięte].
  (def proc (os/spawn ["/bin/sh" "-c" cmd] :p {:out :pipe}))
  (def out (:read (proc :out) :all))
  (def code (:wait proc))
  [(zero? code) (string/trimr (or out ""))])

(defn have? [tool]
  (zero? (os/shell (string "command -v " tool " >/dev/null 2>&1"))))

(defn root? []
  (zero? (os/shell "test \"$(id -u)\" = 0")))

(defn sudo- []
  (if (root?) "" (if (have? "sudo") "sudo " "")))

(defn ensure-dir [path]
  (try (os/mkdir path) ([_] nil)))

# ---------------------------------------------------------------------
# Auto-instalacja brakujących narzędzi -- wykrywa menedżer pakietów
# (apt/dnf/pacman/zypper/apk/brew), nie tylko apt/Debian.
# ---------------------------------------------------------------------

(defn detect-pm []
  (cond
    (have? "apt-get") :apt
    (have? "dnf") :dnf
    (have? "pacman") :pacman
    (have? "zypper") :zypper
    (have? "apk") :apk
    (have? "brew") :brew
    :none))

(defn pm-install [pkgs-by-pm]
  (def pm (detect-pm))
  (def pkgs (get pkgs-by-pm pm))
  (if (not pkgs)
    false
    (let [sudo (sudo-)]
      (case pm
        :apt (try-run (string sudo "apt-get update && " sudo "env DEBIAN_FRONTEND=noninteractive apt-get install -y " pkgs))
        :dnf (try-run (string sudo "dnf install -y " pkgs))
        :pacman (try-run (string sudo "pacman -Sy --noconfirm " pkgs))
        :zypper (try-run (string sudo "zypper --non-interactive install " pkgs))
        :apk (try-run (string sudo "apk add --no-cache " pkgs))
        :brew (try-run (string "brew install " pkgs))
        false))))

(defn ensure-cargo []
  (unless (have? "cargo")
    (eprint "recipe.janet: brak 'cargo' -- próbuję zainstalować (" (detect-pm) ")...")
    (unless (pm-install {:apt "cargo" :dnf "cargo" :pacman "rust" :zypper "cargo" :apk "cargo" :brew "rust"})
      (eprint "recipe.janet: menedżer pakietów nie ma 'cargo' -- próbuję rustup (oficjalny instalator)...")
      (try-run "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable")
      (def cargo-bin-dir (string (os/getenv "HOME") "/.cargo/bin"))
      (when (os/stat (string cargo-bin-dir "/cargo") :mode)
        (os/setenv "PATH" (string cargo-bin-dir ":" (os/getenv "PATH"))))))
  (unless (have? "cargo")
    (fail "nie udało się zapewnić 'cargo' -- zainstaluj Rust ręcznie (rustup) i uruchom ponownie")))

# packaging/zenit/recipe.janet leży dwa poziomy pod korzeniem repo
# (packaging/zenit -> packaging -> <root>) -- zpk zawsze ustawia cwd
# recipe na katalog z zpk.build, więc korzeń repo liczymy względem
# (os/cwd), niezależnie skąd faktycznie wywołano `zpk build`.
(def repo-root (string (os/cwd) "/../.."))

(def work-dir (string (os/cwd) "/build"))
(ensure-dir work-dir)

# ---------------------------------------------------------------------
# HackerScript jest w trakcie przechodzenia na pełne samo-hostowanie:
# `hackerc` i `virus` są dziś napisane w samym HackerScript (.hcs), więc
# żeby je skompilować, potrzeba JUŻ DZIAŁAJĄCEGO hackerc ("stage0"),
# pobieranego z GitHub Releases -- dokładnie ten sam mechanizm i te same
# zasady wyboru wersji co .github/workflows/build.yml (job
# resolve-versions) i virus/cmd/hackerc_bridge.hcs. Odtwarzamy tu
# CAŁY ten łańcuch, żeby recipe budowała to samo, co oficjalne CI:
#
#   stage0 (pobrany, wersja N-1) -> transpiluje hackerc/cmd/cli.hcs
#     -> cargo build --release -> `hackerc` (wersja N, świeżo zbudowany)
#   świeży hackerc (NIE stage0!) -> transpiluje virus/cmd/main.hcs
#     -> cargo build --release -> `virus` (wersja N)
# ---------------------------------------------------------------------

(defn read-version []
  # "-> using => X.Y" w [package] głównego Virus.hk -- to samo pole,
  # które czyta stage0/CI.
  (def [ok out] (shell-out (string "grep -E '^[[:space:]]*->[[:space:]]*using[[:space:]]*=>' "
                                    repo-root "/Virus.hk | head -n1 "
                                    "| sed -E 's/^[[:space:]]*->[[:space:]]*using[[:space:]]*=>[[:space:]]*//' "
                                    "| tr -d '[:space:]'")))
  (if (and ok (> (length out) 0)) out nil))

(defn stage0-version [version]
  # Ta sama zasada co resolve-versions w build.yml: 0.1 -> 0.0.1
  # (specjalny przypadek -- pierwsze samo-hostujące się wydanie),
  # 0.N (N>1) -> 0.(N-1).
  (cond
    (= version "0.1") "0.0.1"
    (do
      (def parts (string/split "." version))
      (if (not= (length parts) 2)
        nil
        (let [major (scan-number (parts 0))
              minor (scan-number (parts 1))]
          (if (or (not major) (not minor) (<= minor 0))
            nil
            (string major "." (- minor 1))))))))

(def version (read-version))
(unless version
  (fail "nie udało się odczytać '-> using => ...' z Virus.hk"))

(when (= version "0.0.1")
  (fail (string "wersja docelowa to 0.0.1 -- to historyczny korzeń bootstrapu (Python/PyInstaller, patrz README.adoc), "
                "nie ma dla niego stage0 ani łańcucha samo-hostującego się -- ta recipe go nie obsługuje")))

(def stage0 (stage0-version version))
(unless stage0
  (fail (string "nie udało się wyliczyć wersji stage0 dla '" version "'")))

(def prebuilt-dir (os/getenv "ZPK_PACKAGING_PREBUILT_BIN_DIR"))

(defn download-stage0 [dest]
  # Nazwa assetu "hackerc" (bez przyrostka platformy) to konwencja
  # DOCELOWA, wymagana przez build.yml (patrz komentarz w tym pliku
  # o hackerc_asset_name) -- ale .github/workflows/release.yml, które
  # faktycznie publikuje wydania, jest dziś (wg własnego komentarza w
  # repo) NIEZAKTUALIZOWANE i wciąż publikuje starą, PyInstallerową
  # nazwę z przyrostkiem platformy. Próbujemy obu, w tej kolejności.
  (def primary (string "https://github.com/HackerOS-Linux-System/HackerScript/releases/download/v" stage0 "/hackerc"))
  (def legacy (string "https://github.com/HackerOS-Linux-System/HackerScript/releases/download/v" stage0 "/hackerc-linux-x86_64"))
  (if (try-run (string "curl -fsSL -o " dest " " primary))
    true
    (do
      (eprint "recipe.janet: brak assetu 'hackerc' przy v" stage0 " -- próbuję starszej nazwy 'hackerc-linux-x86_64'...")
      (try-run (string "curl -fsSL -o " dest " " legacy)))))

(defn self-hosted-build []
  # Zwraca [hackerc-path virus-path] przy sukcesie całego łańcucha,
  # albo nil, jeśli którykolwiek etap zawiedzie -- wywołujący ma wtedy
  # fallback (patrz niżej).
  (ensure-cargo)
  (def stage0-bin (string work-dir "/stage0-hackerc"))
  (def hackerc-out (string repo-root "/hackerc/cmd/target-hcs"))
  (def virus-out (string repo-root "/virus/cmd/target-hcs"))
  (def hackerc-bin (string hackerc-out "/target/release/hackerc"))
  (def virus-bin (string virus-out "/target/release/virus"))
  (if
    (and
      (download-stage0 stage0-bin)
      (try-run (string "chmod +x " stage0-bin))
      (do
        # `--version` bywa nieobsługiwane przez stage0 (patrz build.yml:
        # "stage0 nie obsluguje --version - kontynuuje") -- to tylko log
        # sanity-check, wynik celowo nie wpływa na dalszy łańcuch.
        (try-run (string stage0-bin " --version"))
        true)
      # Etap 1: stage0 transpiluje hackerc/cmd/cli.hcs.
      (try-run (string "cd " repo-root " && " stage0-bin
                        " build hackerc/cmd/cli.hcs -o hackerc/cmd/target-hcs --crate-name hackerc"))
      (try-run (string "cd " hackerc-out " && cargo build --release"))
      (os/stat hackerc-bin :mode)
      # Etap 2: świeżo zbudowany hackerc (NIE stage0) transpiluje virus/cmd/main.hcs.
      (try-run (string "cd " repo-root " && " hackerc-bin
                        " build virus/cmd/main.hcs -o virus/cmd/target-hcs --crate-name virus"))
      (try-run (string "cd " virus-out " && cargo build --release"))
      (os/stat virus-bin :mode))
    [hackerc-bin virus-bin]
    nil))

(def binaries
  (if (and prebuilt-dir (> (length prebuilt-dir) 0))
    # CI/operator już zbudowało/pobrało binarki wcześniej w tym samym
    # biegu -- nie buduj drugi raz.
    [(string prebuilt-dir "/hackerc") (string prebuilt-dir "/virus")]
    (let [built (self-hosted-build)]
      (if built
        built
        (do
          # -----------------------------------------------------
          # Pełny łańcuch stage0 (jak w build.yml) nie powiódł się --
          # sam projekt opisuje ten etap jako niestabilny/przejściowy
          # (patrz komentarz na górze <root>/Virus.hk). Zamiast
          # failować całą recipe, spadamy do tego, co realnie działa
          # dziś: gotowa binarka `virus` z oficjalnego wydania v0.1
          # -- dokładnie to, co robi scripts/install.sh. `hackerc`
          # osobno nie jest wtedy dostępny (nie ma osobnego release'u
          # tej binarki w tej ścieżce) -- pakujemy samo `virus`.
          # -----------------------------------------------------
          (eprint "recipe.janet: uwaga -- samo-hostujący się build (stage0 -> hackerc -> virus) nie powiódł się -- spadam do gotowej binarki 'virus' z oficjalnego wydania v0.1 (jak scripts/install.sh)")
          (def fallback-bin (string work-dir "/virus-prebuilt"))
          (unless (try-run (string "curl -fsSL -o " fallback-bin
                                    " https://github.com/HackerOS-Linux-System/HackerScript/releases/download/v0.1/virus"))
            (fail "ani samo-hostujący się build, ani pobranie oficjalnej binarki 'virus' (v0.1) się nie powiodło"))
          (try-run (string "chmod +x " fallback-bin))
          [nil fallback-bin])))))

(def hackerc-path (binaries 0))
(def virus-path (binaries 1))

(def bin-dir (string stage "/usr/bin"))
(ensure-dir stage)
(ensure-dir (string stage "/usr"))
(ensure-dir bin-dir)

(defn stage-binary [name path]
  (when path
    (unless (os/stat path :mode)
      (fail (string "nie znaleziono zbudowanej/pobranej binarki '" name "': " path)))
    (def dest (string bin-dir "/" name))
    (spit dest (slurp path))
    (run (string "chmod +x " dest))))

(stage-binary "hackerc" hackerc-path)
(stage-binary "virus" virus-path)

(unless (or hackerc-path virus-path)
  (fail "nie zbudowano/pobrano żadnej binarki -- nic do zapakowania"))
