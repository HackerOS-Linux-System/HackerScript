use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use walkdir::WalkDir;

use crate::cache::CacheDirs;
use crate::hackerc_bridge;
use crate::manifest;
use crate::progress;

pub fn run(project_root: &Path, file: Option<&Path>, check: bool) -> Result<()> {
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

    let mut unformatted = 0usize;
    for src in &sources {
        let mut cmd = Command::new(&hackerc_bin);
        cmd.arg("fmt").arg(src);
        if check {
            cmd.arg("--check");
        }
        let status = cmd.status().context("nie mozna uruchomic hackerc fmt")?;
        if !status.success() {
            unformatted += 1;
        }
    }

    if check && unformatted > 0 {
        bail!("{unformatted} plik(ow) niesformatowanych (uruchom `virus fmt` bez --check)");
    }
    progress::success(&format!("virus fmt: sprawdzono/sformatowano {} plik(ow)", sources.len()));
    Ok(())
}
