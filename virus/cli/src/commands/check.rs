use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use walkdir::WalkDir;

use crate::cache::CacheDirs;
use crate::hackerc_bridge;
use crate::manifest;
use crate::progress;

pub fn run(project_root: &Path) -> Result<()> {
    let cache = CacheDirs::for_project(project_root);
    cache.ensure_all()?;

    let manifest = manifest::load(project_root)?;
    let version = manifest
        .package
        .using
        .clone()
        .unwrap_or_else(|| hackerc_bridge::DEFAULT_HACKERC_VERSION.to_string());
    let hackerc_bin = hackerc_bridge::resolve(&cache, &version)
        .context("hackerc niedostepny - uruchom najpierw `virus cache`")?;

    let sources: Vec<_> = WalkDir::new(manifest::source_dir(project_root))
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map(|x| x == "hcs").unwrap_or(false))
        .map(|e| e.path().to_path_buf())
        .collect();

    let mut errors = 0usize;
    for src in &sources {
        let status = Command::new(&hackerc_bin)
            .arg("check")
            .arg(src)
            .status()
            .context("nie mozna uruchomic hackerc check")?;
        if !status.success() {
            errors += 1;
        }
    }

    if errors > 0 {
        bail!("`virus check` znalazl bledy w {errors}/{} plik(ow)", sources.len());
    }
    progress::success(&format!("sprawdzono {} plik(ow), 0 bledow", sources.len()));
    Ok(())
}
