use std::path::Path;

use anyhow::Result;

use crate::cache::CacheDirs;
use crate::progress;

pub fn run(project_root: &Path) -> Result<()> {
    let cache = CacheDirs::for_project(project_root);
    cache.clean()?;
    progress::success("wyczyszczono cache/");
    Ok(())
}
