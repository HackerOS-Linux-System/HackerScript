use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub const CACHE_DIR_NAME: &str = "cache";

#[derive(Debug, Clone)]
pub struct CacheDirs {
    pub root: PathBuf,      // <projekt>/cache
    pub libs: PathBuf,      // <projekt>/cache/libs
    pub source: PathBuf,    // <projekt>/cache/source
    pub env: PathBuf,       // <projekt>/cache/env
    pub build: PathBuf,     // <projekt>/cache/build
}

impl CacheDirs {
    pub fn for_project(project_root: &Path) -> Self {
        let root = project_root.join(CACHE_DIR_NAME);
        Self {
            libs: root.join("libs"),
            source: root.join("source"),
            env: root.join("env"),
            build: root.join("build"),
            root,
        }
    }

    /// Tworzy caly katalog `cache/` (i podkatalogi) jesli nie istnieje.
    /// Odpowiednik `virus cache`.
    pub fn ensure_all(&self) -> Result<()> {
        for dir in [&self.root, &self.libs, &self.source, &self.env, &self.build] {
            fs::create_dir_all(dir)
                .with_context(|| format!("nie mozna utworzyc katalogu {}", dir.display()))?;
        }
        Ok(())
    }

    pub fn clean(&self) -> Result<()> {
        if self.root.exists() {
            fs::remove_dir_all(&self.root)
                .with_context(|| format!("nie mozna usunac {}", self.root.display()))?;
        }
        Ok(())
    }

    /// Sciezka do lokalnego cache konkretnej zaleznosci, np.
    /// cache/libs/pypi/rich/ lub cache/libs/crates/colorize/
    pub fn lib_path(&self, source: &str, name: &str) -> PathBuf {
        self.libs.join(source).join(name)
    }

    /// Sciezka gdzie powinien wladowac sie pobrany `hackerc` dla danej wersji.
    pub fn hackerc_binary(&self, version: &str) -> PathBuf {
        self.env.join(format!("hackerc-{version}")).join("hackerc")
    }
}

/// Szuka `Virus.hk` zaczynajac od `start` i idac w gore drzewa katalogow
/// (tak jak `cargo` szuka `Cargo.toml`).
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start.to_path_buf());
    while let Some(d) = dir {
        if d.join("Virus.hk").is_file() {
            return Some(d);
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    None
}
