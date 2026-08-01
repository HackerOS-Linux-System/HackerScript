use console::style;
use indicatif::{ProgressBar, ProgressStyle};

pub struct Progress {
    bar: ProgressBar,
}

impl Progress {
    /// Tworzy nowy pasek postepu z `total` krokami.
    pub fn new(total: u64) -> Self {
        let bar = ProgressBar::new(total);
        bar.set_style(
            ProgressStyle::with_template("{prefix:>12.bold.cyan} [{bar:32.green/black}] {percent:>3}%  {msg}")
                .unwrap()
                .progress_chars("/-"),
        );
        bar.set_prefix("virus");
        Progress { bar }
    }

    /// Ustawia komunikat statusu widoczny obok paska (np. "transpilacja").
    pub fn step(&self, message: impl Into<String>) {
        self.bar.set_message(message.into());
    }

    pub fn inc(&self, delta: u64) {
        self.bar.inc(delta);
    }

    pub fn set(&self, pos: u64) {
        self.bar.set_position(pos);
    }

    pub fn finish_with(&self, message: impl Into<String>) {
        self.bar.finish_with_message(message.into());
    }

    pub fn fail(&self, message: impl Into<String>) {
        self.bar.abandon_with_message(format!("{} {}", style("blad:").red().bold(), message.into()));
    }
}

/// Krotki naglowek etapu, np. "==> Budowanie (release)".
pub fn header(text: &str) {
    println!("{} {}", style("==>").bold().green(), style(text).bold());
}

pub fn info(text: &str) {
    println!("{} {}", style("info:").blue().bold(), text);
}

pub fn warn(text: &str) {
    println!("{} {}", style("uwaga:").yellow().bold(), text);
}

pub fn error(text: &str) {
    eprintln!("{} {}", style("blad:").red().bold(), text);
}

pub fn success(text: &str) {
    println!("{} {}", style("ok:").green().bold(), text);
}
