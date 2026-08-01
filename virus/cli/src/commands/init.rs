use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::progress;

const DEFAULT_MAIN_HCS: &str = r#"! Witaj w HackerScript!

fun main() [
    log("Hello, HackerScript!")
    end
]
"#;

pub fn run(target_dir: &Path, name: Option<String>) -> Result<()> {
    if target_dir.join("Virus.hk").exists() {
        bail!("Virus.hk juz istnieje w {}", target_dir.display());
    }

    let pkg_name = name.unwrap_or_else(|| {
        target_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("hackerscript-project")
            .to_string()
    });

    let cmd_dir: PathBuf = target_dir.join("cmd");
    fs::create_dir_all(&cmd_dir)
        .with_context(|| format!("nie mozna utworzyc {}", cmd_dir.display()))?;

    fs::write(cmd_dir.join("main.hcs"), DEFAULT_MAIN_HCS)?;

    let manifest = format!(
        r#"[package]
name = "{pkg_name}"
version = "0.0.1"
using = "1.2"
edition = "2026"

[dependencies]
# "pypi:rich" = "*"

[build]
entry = "cmd/main.hcs"
"#
    );
    fs::write(target_dir.join("Virus.hk"), manifest)?;

    progress::success(&format!("utworzono nowy projekt HackerScript: {pkg_name}"));
    Ok(())
}
