use std::path::Path;

use anyhow::{Context, Result};

use crate::cache::CacheDirs;
use crate::manifest::MANIFEST_FILE;
use crate::progress;

pub fn run(project_root: &Path, source: &str, name: &str) -> Result<()> {
    let cache = CacheDirs::for_project(project_root);
    let dest = cache.lib_path(source, name);
    if dest.exists() {
        std::fs::remove_dir_all(&dest)
            .with_context(|| format!("nie mozna usunac {}", dest.display()))?;
    }

    let path = project_root.join(MANIFEST_FILE);
    let text = std::fs::read_to_string(&path)?;
    let key = format!("\"{source}:{name}\"");
    let new_text: String = text
        .lines()
        .filter(|line| !line.trim_start().starts_with(&key))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(&path, new_text)?;

    progress::success(&format!("usunieto {source}:{name}"));
    Ok(())
}
