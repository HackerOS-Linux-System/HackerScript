use anyhow::{bail, Result};

use crate::progress;

struct Diagnostic {
    code: &'static str,
    title: &'static str,
    explanation: &'static str,
    suggestion: &'static str,
}

const DIAGNOSTICS: &[Diagnostic] = &[
    Diagnostic {
        code: "E0001",
        title: "niezamkniety blok [ ... ]",
        explanation: "Kazdy blok HackerScript otwarty przez '[' musi byc zamkniety przez ']'.",
        suggestion: "Sprawdz czy na koncu funkcji/if/while/for jest brakujacy ']'.",
    },
    Diagnostic {
        code: "E0002",
        title: "brak `end` na koncu funkcji zwracajacej wartosc",
        explanation: "Funkcja z typem zwracanym (-> Typ) powinna konczyc sie `end <wyrazenie>`.",
        suggestion: "Dodaj `end <wartosc>` przed zamknieciem bloku funkcji.",
    },
    Diagnostic {
        code: "W0001",
        title: "nieuzywana zmienna",
        explanation: "Zmienna zadeklarowana przez `let` nigdy nie jest odczytywana.",
        suggestion: "Usun deklaracje albo poprzedz nazwe podkreslnikiem `_nazwa` (docelowo wspierane).",
    },
    Diagnostic {
        code: "E0003",
        title: "nieznane zrodlo zaleznosci w `get <zrodlo:nazwa>`",
        explanation: "Obslugiwane zrodla to: pypi, crates, std, core.",
        suggestion: "Popraw prefiks zrodla w instrukcji `get`.",
    },
];

pub fn run(code: &str) -> Result<()> {
    let code_upper = code.to_uppercase();
    match DIAGNOSTICS.iter().find(|d| d.code == code_upper) {
        Some(d) => {
            progress::header(&format!("{} - {}", d.code, d.title));
            println!("{}", d.explanation);
            println!();
            println!("sugestia: {}", d.suggestion);
            Ok(())
        }
        None => {
            progress::error(&format!("nieznany kod diagnostyczny: {code}"));
            bail!(
                "brak wpisu dla {code} w bazie diagnostyk - jesli to swiezy blad z hackerc, \
                 zglos go, zeby dodac go do `virus repair` (docs/ROADMAP.md)"
            )
        }
    }
}
