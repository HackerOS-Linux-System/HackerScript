mod cache;
mod commands;
mod hackerc_bridge;
mod manifest;
mod progress;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use commands::build::{BuildOptions, Target};

#[derive(Parser)]
#[command(
    name = "virus",
    version,
    about = "Manager pakietow i narzedzie budowania dla HackerScript"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Tlumaczy i buduje projekt (domyslnie: .AppImage)
    Build {
        /// Zbuduj tylko ten plik .hcs (pomija reszte projektu)
        file: Option<PathBuf>,
        /// Produkcyjna, samodzielna binarka
        #[arg(long)]
        release: bool,
        /// Buduj do .so / .rlib / .a
        #[arg(long)]
        library: bool,
        /// Buduj do WebAssembly
        #[arg(long)]
        wasm: bool,
        /// Buduj do .jar
        #[arg(long)]
        jar: bool,
    },
    /// Tworzy / odswieza katalog cache/
    Cache,
    /// Sprawdza poprawnosc kodu bez budowania
    Check,
    /// Probuje wyjasnic/naprawic blad lub warning po jego kodzie (np. E0001)
    Repair { code: String },
    /// Usuwa caly katalog cache/
    Clean,
    /// Instaluje zaleznosc: `virus install pypi rich`
    Install {
        source: String,
        name: String,
        #[arg(long)]
        version: Option<String>,
    },
    /// Usuwa zaleznosc: `virus remove pypi rich`
    Remove { source: String, name: String },
    /// Formatuje kod .hcs (deleguje do `hackerc fmt`)
    Fmt {
        file: Option<PathBuf>,
        #[arg(long)]
        check: bool,
    },
    /// Pokazuje tylko warningi (deleguje do `hackerc lint`)
    Lint { file: Option<PathBuf> },
    /// Tworzy szkielet nowego projektu HackerScript w biezacym katalogu
    Init {
        #[arg(long)]
        name: Option<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let cwd = env::current_dir().expect("nie mozna odczytac biezacego katalogu");

    let result = match cli.command {
        Commands::Init { name } => commands::init::run(&cwd, name),
        Commands::Build { file, release, library, wasm, jar } => {
            with_project_root(&cwd, |root| {
                let target = if release {
                    Target::Release
                } else if library {
                    Target::Library
                } else if wasm {
                    Target::Wasm
                } else if jar {
                    Target::Jar
                } else {
                    Target::AppImage
                };
                commands::build::run(root, BuildOptions { target, single_file: file })
            })
        }
        Commands::Cache => with_project_root(&cwd, commands::cache_cmd::run),
        Commands::Check => with_project_root(&cwd, commands::check::run),
        Commands::Repair { code } => commands::repair::run(&code),
        Commands::Clean => with_project_root(&cwd, commands::clean::run),
        Commands::Install { source, name, version } => {
            with_project_root(&cwd, |root| commands::install::run(root, &source, &name, version.as_deref()))
        }
        Commands::Remove { source, name } => {
            with_project_root(&cwd, |root| commands::remove::run(root, &source, &name))
        }
        Commands::Fmt { file, check } => {
            with_project_root(&cwd, |root| commands::fmt::run(root, file.as_deref(), check))
        }
        Commands::Lint { file } => {
            with_project_root(&cwd, |root| commands::lint::run(root, file.as_deref()))
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            progress::error(&format!("{e:#}"));
            ExitCode::FAILURE
        }
    }
}

fn with_project_root<F>(cwd: &std::path::Path, f: F) -> anyhow::Result<()>
where
    F: FnOnce(&std::path::Path) -> anyhow::Result<()>,
{
    match cache::find_project_root(cwd) {
        Some(root) => f(&root),
        None => anyhow::bail!(
            "nie znaleziono Virus.hk (ani w biezacym katalogu, ani w nadrzednych) - \
             uzyj `virus init` aby utworzyc nowy projekt"
        ),
    }
}
