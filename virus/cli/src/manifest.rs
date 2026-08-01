use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
pub use hk_parser::{DepSource, Dependency, Manifest};

pub const MANIFEST_FILE: &str = "Virus.hk";

pub fn load(project_root: &Path) -> Result<Manifest> {
    let path = project_root.join(MANIFEST_FILE);
    hk_parser::parse_file(&path).with_context(|| format!("nie mozna wczytac {}", path.display()))
}

/// Zwraca sciezke wejsciowa (plik startowy .hcs) wzgledem korzenia projektu.
pub fn entry_path(project_root: &Path, manifest: &Manifest) -> PathBuf {
    project_root.join(&manifest.build.entry)
}

/// Katalog zrodel uzytkownika - zawsze `cmd/` (nazwa zarezerwowana, nie do
/// zmiany przez uzytkownika - patrz spec HackerScript).
pub fn source_dir(project_root: &Path) -> PathBuf {
    project_root.join("cmd")
}
