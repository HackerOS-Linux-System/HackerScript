use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use walkdir::WalkDir;

use crate::cache::CacheDirs;
use crate::hackerc_bridge;
use crate::manifest::{self, Manifest};
use crate::progress::{self, Progress};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    AppImage,
    Release,
    Library,
    Wasm,
    Jar,
}

pub struct BuildOptions {
    pub target: Target,
    /// Jesli podano `virus build <plik.hcs>` - transpiluj/zbuduj TYLKO ten plik,
    /// z pominieciem reszty projektu (szybka petla iteracyjna).
    pub single_file: Option<PathBuf>,
}

pub fn run(project_root: &Path, opts: BuildOptions) -> Result<()> {
    let cache = CacheDirs::for_project(project_root);
    cache.ensure_all()?;

    let manifest = manifest::load(project_root)?;
    let version = manifest
        .package
        .using
        .clone()
        .unwrap_or_else(|| hackerc_bridge::DEFAULT_HACKERC_VERSION.to_string());

    progress::header(&format!(
        "Budowanie {} v{} ({})",
        manifest.package.name,
        manifest.package.version,
        target_name(opts.target)
    ));

    let hackerc_bin = ensure_hackerc(&cache, &version)?;

    let sources = match &opts.single_file {
        Some(f) => vec![project_root.join(f)],
        None => collect_sources(project_root),
    };
    if sources.is_empty() {
        bail!("nie znaleziono zadnych plikow .hcs w cmd/");
    }

    install_missing_dependencies(&cache, &manifest)?;
    transpile_all(&cache, &hackerc_bin, project_root, &sources)?;

    match opts.target {
        Target::AppImage => build_appimage(&cache, &manifest)?,
        Target::Release => build_release_binary(&cache, &manifest)?,
        Target::Library => build_library(&cache, &manifest)?,
        Target::Wasm => build_wasm(&cache, &manifest)?,
        Target::Jar => build_jar(&cache, &manifest)?,
    }

    Ok(())
}

fn target_name(t: Target) -> &'static str {
    match t {
        Target::AppImage => "AppImage",
        Target::Release => "release",
        Target::Library => "library (.so/.rlib/.a)",
        Target::Wasm => "wasm",
        Target::Jar => "jar",
    }
}

fn ensure_hackerc(cache: &CacheDirs, version: &str) -> Result<PathBuf> {
    match hackerc_bridge::resolve(cache, version) {
        Ok(path) => Ok(path),
        Err(_) => {
            progress::info(&format!("pobieranie hackerc {version}..."));
            hackerc_bridge::download(cache, version)
                .context("nie udalo sie pobrac hackerc - sprawdz polaczenie sieciowe albo ustaw HACKERC_BIN")
        }
    }
}

fn collect_sources(project_root: &Path) -> Vec<PathBuf> {
    let cmd_dir = manifest::source_dir(project_root);
    WalkDir::new(cmd_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map(|x| x == "hcs").unwrap_or(false))
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn install_missing_dependencies(cache: &CacheDirs, manifest: &Manifest) -> Result<()> {
    for dep in &manifest.dependencies {
        let dest = cache.lib_path(&dep.source.to_string(), &dep.name);
        if dest.exists() {
            continue;
        }
        progress::warn(&format!(
            "zaleznosc {}:{} nie jest jeszcze pobrana - uruchom `virus install {} {}`",
            dep.source, dep.name, dep.source, dep.name
        ));
    }
    Ok(())
}

fn transpile_all(
    cache: &CacheDirs,
    hackerc_bin: &Path,
    project_root: &Path,
    sources: &[PathBuf],
) -> Result<()> {
    let bar = Progress::new(sources.len() as u64);
    for (i, src) in sources.iter().enumerate() {
        let rel = src.strip_prefix(project_root).unwrap_or(src);
        bar.step(format!("transpilacja: {}", rel.display()));
        bar.set(i as u64);

        let out_rel = rel.with_extension("py");
        let out = cache.source.join(out_rel);
        hackerc_bridge::transpile_file(hackerc_bin, src, &out)
            .with_context(|| format!("blad transpilacji {}", src.display()))?;

        // Jesli plik zawieral `native fun`, hackerc zapisal obok
        // `<stem>_native/` (Cargo.toml + src/lib.rs, patrz
        // hackerc/hackerc/native_codegen.py) - skompiluj go teraz.
        let native_dir = out.parent().unwrap().join(format!(
            "{}_native",
            out.file_stem().and_then(|s| s.to_str()).unwrap_or("mod")
        ));
        if native_dir.join("Cargo.toml").exists() {
            bar.step(format!("kompilacja native (Rust): {}", rel.display()));
            let so_path = hackerc_bridge::compile_native(&native_dir, &out)
                .with_context(|| format!("blad kompilacji native fun z {}", src.display()))?;
            progress::info(&format!("skompilowano native -> {}", so_path.display()));
        }
    }
    bar.finish_with(format!("przetlumaczono {} plik(ow)", sources.len()));
    Ok(())
}

fn python_available() -> Option<String> {
    for candidate in ["python3", "python"] {
        if which::which(candidate).is_ok() {
            return Some(candidate.to_string());
        }
    }
    None
}

fn main_py(cache: &CacheDirs, manifest: &Manifest) -> PathBuf {
    // `cmd/main.hcs` (albo cokolwiek wskazane w [build] entry) -> odpowiadajacy
    // mu plik .py wewnatrz cache/source/, zachowujac te sama sciezke wzgledna.
    let entry_rel = PathBuf::from(&manifest.build.entry).with_extension("py");
    cache.source.join(entry_rel)
}

/// Domyslny target: prosty, przenosny bundle uruchamialny (docelowo prawdziwy
/// .AppImage przez `appimagetool`, jesli jest dostepny w systemie).
fn build_appimage(cache: &CacheDirs, manifest: &Manifest) -> Result<()> {
    let bar = Progress::new(1);
    bar.step("pakowanie AppImage");

    if which::which("appimagetool").is_ok() {
        // TODO(bootstrap 0.0.1): zbudowac prawdziwa strukture AppDir
        // (AppRun, .desktop, ikona) i wywolac `appimagetool AppDir`.
        // Patrz docs/ROADMAP.md.
        bar.fail("budowanie prawdziwego .AppImage nie jest jeszcze zaimplementowane");
        bail!("`virus build` (AppImage) wymaga jeszcze implementacji pakowania AppDir - patrz docs/ROADMAP.md");
    }

    let out = cache.build.join(format!("{}.AppImage", manifest.package.name));
    std::fs::copy(main_py(cache, manifest), &out).context("nie mozna skopiowac artefaktu")?;
    bar.finish_with(format!("zapisano {} (tymczasowy placeholder, nie prawdziwy AppImage)", out.display()));
    progress::warn("to jest placeholder - prawdziwe pakowanie AppImage jest w docs/ROADMAP.md");
    Ok(())
}

/// `--release`: samodzielna binarka (bez wymaganego interpretera Pythona
/// obok) - poprzez PyInstaller, jesli dostepny.
fn build_release_binary(cache: &CacheDirs, manifest: &Manifest) -> Result<()> {
    let bar = Progress::new(1);
    bar.step("budowanie binarki release");

    let Some(py) = python_available() else {
        bail!("nie znaleziono python3/python w PATH - wymagany do zbudowania release");
    };

    let has_pyinstaller = Command::new(&py)
        .args(["-m", "PyInstaller", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_pyinstaller {
        bar.fail("PyInstaller niedostepny");
        bail!(
            "`virus build --release` wymaga PyInstaller (`{} -m pip install pyinstaller`) \
             - docelowo bedzie to pobierane automatycznie do cache/env/, patrz docs/ROADMAP.md",
            py
        );
    }

    let status = Command::new(&py)
        .args([
            "-m",
            "PyInstaller",
            "--onefile",
            "--distpath",
        ])
        .arg(&cache.build)
        .arg("--name")
        .arg(&manifest.package.name)
        .arg(main_py(cache, manifest))
        .status()
        .context("nie mozna uruchomic PyInstaller")?;

    if !status.success() {
        bar.fail("PyInstaller zwrocil blad");
        bail!("budowanie release nie powiodlo sie");
    }

    bar.finish_with("zbudowano binarke release");
    Ok(())
}

fn build_library(cache: &CacheDirs, manifest: &Manifest) -> Result<()> {
    let _ = (cache, manifest);
    progress::warn("`virus build --library` (.so/.rlib/.a) nie jest jeszcze zaimplementowane");
    bail!("brak implementacji - patrz docs/ROADMAP.md (\"budowanie bibliotek\")");
}

fn build_wasm(cache: &CacheDirs, manifest: &Manifest) -> Result<()> {
    let _ = (cache, manifest);
    progress::warn("`virus build --wasm` nie jest jeszcze zaimplementowane");
    bail!("brak implementacji - patrz docs/ROADMAP.md (\"target wasm\")");
}

fn build_jar(cache: &CacheDirs, manifest: &Manifest) -> Result<()> {
    let _ = (cache, manifest);
    progress::warn("`virus build --jar` nie jest jeszcze zaimplementowane");
    bail!("brak implementacji - patrz docs/ROADMAP.md (\"target jar\")");
}
