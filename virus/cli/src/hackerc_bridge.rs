use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

use crate::cache::CacheDirs;

pub const DEFAULT_HACKERC_VERSION: &str = "1.2";

/// Nazwa assetu wydania dopasowana do biezacej platformy - MUSI byc
/// zgodna z nazewnictwem uzywanym w .github/workflows/release.yml.
fn asset_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "hackerc-windows-x86_64.exe"
    } else if cfg!(target_os = "macos") {
        "hackerc-macos-x86_64"
    } else {
        "hackerc-linux-x86_64"
    }
}

fn release_url(version: &str) -> String {
    format!(
        "https://github.com/HackerOS-Linux-System/HackerScript/releases/download/v{version}/{}",
        asset_name()
    )
}

/// Znajduje binarke `hackerc` do uzycia, w kolejnosci:
/// 1. zmienna srodowiskowa `HACKERC_BIN`
/// 2. `cache/env/hackerc-<wersja>/hackerc` (pobrana wczesniej)
/// 3. `hackerc` w PATH (np. zainstalowany przez `pip install -e ./hackerc`)
pub fn resolve(cache: &CacheDirs, version: &str) -> Result<PathBuf> {
    if let Ok(path) = std::env::var("HACKERC_BIN") {
        return Ok(PathBuf::from(path));
    }

    let cached = cache.hackerc_binary(version);
    if cached.exists() {
        return Ok(cached);
    }

    if let Ok(found) = which::which("hackerc") {
        return Ok(found);
    }

    bail!(
        "nie znaleziono hackerc (wersja {version}). Uruchom `virus cache` aby pobrac \
         narzedzia, ustaw HACKERC_BIN, lub zainstaluj `hackerc` z PATH \
         (patrz {})",
        release_url(version)
    )
}

/// Pobiera hackerc z GitHub Releases do `cache/env/hackerc-<wersja>/hackerc`.
pub fn download(cache: &CacheDirs, version: &str) -> Result<PathBuf> {
    let dest_dir = cache.env.join(format!("hackerc-{version}"));
    std::fs::create_dir_all(&dest_dir)
        .with_context(|| format!("nie mozna utworzyc {}", dest_dir.display()))?;
    let dest = dest_dir.join("hackerc");

    let url = release_url(version);
    let mut resp = reqwest::blocking::get(&url)
        .with_context(|| format!("nie mozna pobrac {url}"))?
        .error_for_status()
        .with_context(|| format!("serwer zwrocil blad dla {url}"))?;

    let mut file = std::fs::File::create(&dest)
        .with_context(|| format!("nie mozna zapisac {}", dest.display()))?;
    std::io::copy(&mut resp, &mut file).context("blad zapisu pobranego pliku hackerc")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms)?;
    }

    Ok(dest)
}

/// Transpiluje pojedynczy plik `.hcs` do `.py` uzywajac `hackerc`.
pub fn transpile_file(hackerc_bin: &Path, src: &Path, out: &Path) -> Result<()> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let status = Command::new(hackerc_bin)
        .arg(src)
        .arg("-o")
        .arg(out)
        .status()
        .with_context(|| format!("nie mozna uruchomic {}", hackerc_bin.display()))?;

    if !status.success() {
        return Err(anyhow!(
            "hackerc zwrocil kod bledu {:?} dla {}",
            status.code(),
            src.display()
        ));
    }
    Ok(())
}

/// `virus check` - tlumaczy do pamieci/tymczasowego pliku tylko po to, zeby
/// zweryfikowac poprawnosc skladniowa (bez zapisywania trwalych artefaktow).
pub fn check_file(hackerc_bin: &Path, src: &Path) -> Result<()> {
    let tmp = std::env::temp_dir().join(format!(
        "virus-check-{}.py",
        src.file_stem().and_then(|s| s.to_str()).unwrap_or("out")
    ));
    let result = transpile_file(hackerc_bin, src, &tmp);
    let _ = std::fs::remove_file(&tmp);
    result
}

/// Kompiluje crate wygenerowany przez hackerc dla `native fun`
/// (`<stem>_native/` obok pliku .py, zawiera juz gotowy `Cargo.toml` +
/// `src/lib.rs` z bindingami PyO3 - patrz hackerc/hackerc/native_codegen.py).
///
/// UWAGA co do niezaleznosci od cargo/pip: `virus` NIE zarzadza tu
/// zaleznosciami jako menedzer pakietow (to robi samo, patrz
/// commands/install.rs, bez `pip`/`cargo`) - tutaj `cargo`/`rustc` sa
/// uzyte jako TOOLCHAIN kompilujacy WYGENEROWANY kod Rust, dokladnie tak
/// jak `rustc` jest toolchainem dla samego `cargo`. Bez realnego
/// kompilatora Rust na maszynie `native fun` nie da sie skompilowac -
/// to nieuniknione, taka jest natura kompilacji do natywnego kodu.
///
/// Zwraca sciezke pliku .so/.pyd/.dylib gotowego do zaimportowania przez
/// Python (skopiowanego/przemianowanego obok `out_py`).
pub fn compile_native(native_dir: &Path, out_py: &Path) -> Result<PathBuf> {
    if which::which("cargo").is_err() {
        bail!(
            "'native fun' wymaga zainstalowanego Rust (cargo) do skompilowania {} - \
             patrz https://rustup.rs",
            native_dir.display()
        );
    }

    let manifest_path = native_dir.join("Cargo.toml");
    let status = Command::new("cargo")
        .args(["build", "--release", "--manifest-path"])
        .arg(&manifest_path)
        .status()
        .with_context(|| format!("nie mozna uruchomic 'cargo build' dla {}", native_dir.display()))?;
    if !status.success() {
        bail!("kompilacja native fun ({}) nie powiodla sie", native_dir.display());
    }

    // Nazwa crate'a = nazwa katalogu native_dir (patrz native_codegen.py:
    // generate_native_cargo_toml uzywa "{package_name}_native").
    let crate_name = native_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("nieprawidlowa nazwa katalogu native: {}", native_dir.display()))?;

    let target_release = native_dir.join("target").join("release");
    let (built_name, py_ext) = if cfg!(target_os = "windows") {
        (format!("{crate_name}.dll"), "pyd")
    } else if cfg!(target_os = "macos") {
        (format!("lib{crate_name}.dylib"), "so")
    } else {
        (format!("lib{crate_name}.so"), "so")
    };

    let built_path = target_release.join(&built_name);
    if !built_path.exists() {
        bail!(
            "kompilacja native fun zakonczyla sie sukcesem, ale nie znaleziono {} - \
             sprawdz nazwe crate'a w {}",
            built_path.display(),
            manifest_path.display()
        );
    }

    let dest = out_py
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{crate_name}.{py_ext}"));
    std::fs::copy(&built_path, &dest)
        .with_context(|| format!("nie mozna skopiowac {} -> {}", built_path.display(), dest.display()))?;

    Ok(dest)
}
