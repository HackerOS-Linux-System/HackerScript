use std::io::Cursor;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::cache::CacheDirs;
use crate::manifest::{self, MANIFEST_FILE};
use crate::progress::{self, Progress};

pub fn run(project_root: &Path, source: &str, name: &str, version: Option<&str>) -> Result<()> {
    let cache = CacheDirs::for_project(project_root);
    cache.ensure_all()?;

    let bar = Progress::new(1);
    bar.step(format!("pobieranie {source}:{name}"));

    let dest = cache.lib_path(source, name);
    std::fs::create_dir_all(&dest)?;

    match source {
        "pypi" => install_pypi(&dest, name, version)?,
        "crates" => install_crates(&dest, name, version)?,
        "std" | "core" => {
            bar.finish_with(format!(
                "{source}:{name} jest czescia dystrybucji HackerScript (libs/{source}) - nic do pobrania"
            ));
            return Ok(());
        }
        other => bail!("nieznane zrodlo zaleznosci {other:?} (oczekiwano pypi/crates/std/core)"),
    }

    bar.finish_with(format!("zainstalowano {source}:{name} -> {}", dest.display()));
    update_manifest(project_root, source, name, version)?;
    Ok(())
}

// ---- pypi: bezposrednio przez PyPI JSON API, bez `pip` --------------------

#[derive(Debug, Deserialize)]
struct PypiResponse {
    urls: Vec<PypiFile>,
}

#[derive(Debug, Deserialize, Clone)]
struct PypiFile {
    filename: String,
    url: String,
    packagetype: String,
}

fn install_pypi(dest: &Path, name: &str, version: Option<&str>) -> Result<()> {
    let api_url = match version {
        Some(v) => format!("https://pypi.org/pypi/{name}/{v}/json"),
        None => format!("https://pypi.org/pypi/{name}/json"),
    };

    let resp: PypiResponse = reqwest::blocking::get(&api_url)
        .with_context(|| format!("nie mozna polaczyc sie z PyPI ({api_url})"))?
        .error_for_status()
        .with_context(|| format!("PyPI zwrocilo blad dla {name} (zla nazwa/wersja?)"))?
        .json()
        .context("nie mozna sparsowac odpowiedzi PyPI JSON API")?;

    if resp.urls.is_empty() {
        bail!("PyPI nie ma zadnych plikow do pobrania dla {name}");
    }

    // Preferuj czyste "universal wheel" (py3-none-any) - najlatwiejsze do
    // rozpakowania bez kompilacji. W przeciwnym razie bierz sdist.
    let chosen = resp
        .urls
        .iter()
        .find(|f| f.packagetype == "bdist_wheel" && f.filename.contains("py3-none-any"))
        .or_else(|| resp.urls.iter().find(|f| f.packagetype == "bdist_wheel"))
        .or_else(|| resp.urls.iter().find(|f| f.packagetype == "sdist"))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("brak wspieranego formatu paczki dla {name} (ani wheel, ani sdist)"))?;

    let bytes = reqwest::blocking::get(&chosen.url)
        .with_context(|| format!("nie mozna pobrac {}", chosen.url))?
        .bytes()
        .context("blad odczytu pobranych bajtow")?;

    if chosen.filename.ends_with(".whl") {
        extract_zip(&bytes, dest)?;
    } else if chosen.filename.ends_with(".tar.gz") {
        extract_tar_gz(&bytes, dest)?;
    } else {
        // np. .zip sdist - obsluz jak zip
        extract_zip(&bytes, dest)?;
    }

    Ok(())
}

fn extract_zip(bytes: &[u8], dest: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("nie mozna otworzyc archiwum .whl/.zip")?;
    archive.extract(dest).context("nie mozna rozpakowac archiwum .whl/.zip")?;
    Ok(())
}

fn extract_tar_gz(bytes: &[u8], dest: &Path) -> Result<()> {
    let decoder = flate2::read::GzDecoder::new(Cursor::new(bytes));
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(dest).context("nie mozna rozpakowac archiwum .tar.gz")?;
    Ok(())
}

// ---- crates: bezposrednio przez crates.io API, bez `cargo` ----------------

#[derive(Debug, Deserialize)]
struct CratesVersionsResponse {
    versions: Vec<CratesVersion>,
}

#[derive(Debug, Deserialize)]
struct CratesVersion {
    num: String,
    dl_path: String,
}

fn install_crates(dest: &Path, name: &str, version: Option<&str>) -> Result<()> {
    let api_url = format!("https://crates.io/api/v1/crates/{name}");
    let resp: CratesVersionsResponse = reqwest::blocking::get(&api_url)
        .with_context(|| format!("nie mozna polaczyc sie z crates.io ({api_url})"))?
        .error_for_status()
        .with_context(|| format!("crates.io zwrocilo blad dla {name} (zla nazwa?)"))?
        .json()
        .context("nie mozna sparsowac odpowiedzi crates.io API")?;

    let chosen = match version {
        Some(v) => resp
            .versions
            .iter()
            .find(|ver| ver.num == v)
            .ok_or_else(|| anyhow::anyhow!("crates.io: brak wersji {v} dla {name}"))?,
        None => resp
            .versions
            .first()
            .ok_or_else(|| anyhow::anyhow!("crates.io: {name} nie ma zadnych wersji"))?,
    };

    let dl_url = format!("https://crates.io{}", chosen.dl_path);
    let bytes = reqwest::blocking::get(&dl_url)
        .with_context(|| format!("nie mozna pobrac {dl_url}"))?
        .bytes()
        .context("blad odczytu pobranego .crate")?;

    // Plik .crate to zwykly .tar.gz - rozpakuj tak samo jak sdist z PyPI.
    extract_tar_gz(&bytes, dest)?;
    Ok(())
}

fn update_manifest(project_root: &Path, source: &str, name: &str, version: Option<&str>) -> Result<()> {
    let path = project_root.join(MANIFEST_FILE);
    let mut text = std::fs::read_to_string(&path)?;
    let key = format!("\"{source}:{name}\"");
    let ver = version.unwrap_or("*");

    if text.contains(&key) {
        return Ok(()); // juz obecna - nie duplikujemy
    }
    if !text.contains("[dependencies]") {
        text.push_str("\n[dependencies]\n");
    }
    let insertion = format!("{key} = \"{ver}\"\n");
    if let Some(pos) = text.find("[dependencies]") {
        let line_end = text[pos..].find('\n').map(|i| pos + i + 1).unwrap_or(text.len());
        text.insert_str(line_end, &insertion);
    } else {
        text.push_str(&insertion);
    }
    std::fs::write(&path, text)?;
    let _ = manifest::load(project_root)?; // walidacja po zapisie
    Ok(())
}
