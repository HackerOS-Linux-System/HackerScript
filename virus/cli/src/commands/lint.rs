use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::cache::CacheDirs;
use crate::hackerc_bridge;
use crate::manifest;

pub fn run(project_root: &Path, file: Option<&Path>) -> Result<()> {
    let cache = CacheDirs::for_project(project_root);
    let version = manifest::load(project_root)
        .ok()
        .and_then(|m| m.package.using)
        .unwrap_or_else(|| hackerc_bridge::DEFAULT_HACKERC_VERSION.to_string());
    let hackerc_bin = hackerc_bridge::resolve(&cache, &version)
        .context("hackerc niedostepny - uruchom najpierw `virus cache`")?;

    let sources: Vec<_> = match file {
        Some(f) => vec![project_root.join(f)],
        None => WalkDir::new(manifest::source_dir(project_root))
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().map(|x| x == "hcs").unwrap_or(false))
            .map(|e| e.path().to_path_buf())
            .collect(),
    };

    for src in &sources {
        Command::new(&hackerc_bin)
            .arg("lint")
            .arg(src)
            .status()
            .context("nie mozna uruchomic hackerc lint")?;
    }
    Ok(())
}
