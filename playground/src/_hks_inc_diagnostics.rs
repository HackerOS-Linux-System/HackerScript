#![allow(non_snake_case, unused_mut, dead_code)]

//! bootstrap/hackerc-self/diagnostics.hcs
//! 
//! Krok 2/N przepisania calego hackerc na HackerScript (patrz
//! docs/ROADMAP.md, "W TOKU"). Parytet z hackerc/hackerc/diagnostics.py
//! (84 linie) - formatowanie komunikatow bledow/ostrzezen w stylu
//! Rust/Elm (numer linii+kolumny + fragment kodu z karetka):
//! 
//! error[E0002]: brakuje 'end' z wartoscia w funkcji zwracajacej Int
//! --> cmd/main.hcs:12:5
//! |
//! 12 |     end
//! |     ^^^
//! 
//! Kompilowany dzis przez STAGE0 (Pythonowy hackerc) - patrz
//! bootstrap/README.md. Brak zaleznosci od innych plikow bootstrapu -
//! ten modul jest samodzielny (uzywany PRZEZ `typecheck.hcs`, nie
//! odwrotnie).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Diagnostic {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub line: i64,
    pub col: i64,
    pub length: i64,
    pub filename: String,
}

impl Diagnostic {
    pub fn new(severity: String, code: String, message: String, line: i64, col: i64, length: i64, filename: String) -> Self {
        Diagnostic { severity, code, message, line, col, length, filename }
    }
}

pub fn max_int(a: i64, b: i64) -> i64 {
    if (a > b) {
        return a;
    }
    return b;
}

pub fn min_int(a: i64, b: i64) -> i64 {
    if (a < b) {
        return a;
    }
    return b;
}

// Sklada `n` powtorzen jednoznakowego `c` w jeden Str - reczny
// odpowiednik Pythonowego `" " * width`/`"^" * length` (HackerScript
// nie ma operatora powtarzania Str, patrz "Ograniczenia").
pub fn repeat_char(c: &String, n: i64) -> String {
    let mut out: String = "".to_string();
    let mut i: i64 = 0;
    while (i < n) {
        out = format!("{}{}", out, c);
        i = (i + 1);
    }
    return out.to_string();
}

// Reczny odpowiednik `source.splitlines() or [""]` z diagnostics.py.
// Dzieli po `\n`, CICHO POMIJAJAC `\r` (parytet dla linii `\r\n` -
// oba znaki razem liczą się jako JEDNO zakonczenie linii, tak jak w
// Pythonowym splitlines() - ale patrz "Ograniczenia" co do SAMOTNEGO
// `\r` bez `\n`, ktory Python liczylby jako oddzielne zakonczenie
// linii, a ta wersja nie). Koncowa czesciowa linia jest dodawana
// TYLKO gdy jest niepusta ALBO gdy zrodlo jest calkowicie puste
// (parytet z `"".splitlines() == []`, a `[] or [""] == [""]`) - bez
// tego `"abc\n"` dostalaby fantomowa dodatkowa puste linie na
// koncu, ktorej Python nie ma.
// `split_lines` UZYWA `.slice()` (nie `cur = cur + c` znak-po-znaku) -
// budowanie stringa przez powtarzane `+` jest O(L^2) dla linii
// dlugosci L (kazde `+` w Rust kopiuje CALY dotychczasowy string).
// Dla duzego pliku (setki/tysiace linii) to sumowalo sie do bardzo
// zauwazalnego spowolnienia. `.slice(start, i)` jest O(dlugosc linii)
// (dzieki cache'owaniu `.char_at`/`.slice` na `source`, patrz
// lexer.hcs/`two_char_at`), wiec cala funkcja jest O(n) zamiast
// O(suma L_i^2). `\r` (stary styl linii Mac, patrz "Ograniczenia"
// ponizej) jest USUWANE z gotowej linii przez `strip_cr` (rzadka
// sciezka, dziala na JUZ WYCIETEJ linii, nie na calym zrodle).
// Bug wydajnosciowy znaleziony przy uzyciu skompilowanego stage1
// (samo-hostowanego hackerc) do zbudowania duzych plikow w tej sesji
// - patrz TEZ `render_many` nizej (druga polowa tego samego bledu:
// `split_lines` bylo wolane RAZ NA DIAGNOSTYKE zamiast raz).
pub fn strip_cr(s: &String) -> String {
    if !(str_contains_char(&s, &"\r".to_string())) {
        return s.to_string();
    }
    let mut out: String = "".to_string();
    let mut i: i64 = 0;
    let mut n = (s.len() as i64);
    while (i < n) {
        let mut c: String = (s.chars().nth(i as usize).map(|c| c.to_string()).unwrap_or_default());
        if (c.to_string() != "\r".to_string().to_string()) {
            out = format!("{}{}", out, c);
        }
        i = (i + 1);
    }
    return out.to_string();
}

pub fn str_contains_char(s: &String, target: &String) -> bool {
    let mut i: i64 = 0;
    let mut n = (s.len() as i64);
    while (i < n) {
        if ((s.chars().nth(i as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == target.to_string()) {
            return true;
        }
        i = (i + 1);
    }
    return false;
}

pub fn split_lines(source: &String) -> Vec<String> {
    let __hks_chars_source: Vec<char> = source.chars().collect();
    let mut lines: Vec<String> = vec![];
    let mut start: i64 = 0;
    let mut i: i64 = 0;
    let mut n = (source.len() as i64);
    while (i < n) {
        let mut c: String = (__hks_chars_source.get(i as usize).map(|c| c.to_string()).unwrap_or_default());
        if (c.to_string() == "\n".to_string().to_string()) {
            lines.push(strip_cr(&({ let __v = &__hks_chars_source; let __s = ((start) as usize).min(__v.len()); let __e = ((i) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() })));
            start = (i + 1);
        }
        i = (i + 1);
    }
    if ((start < n) || ((lines.len() as i64) == 0)) {
        lines.push(strip_cr(&({ let __v = &__hks_chars_source; let __s = ((start) as usize).min(__v.len()); let __e = ((n) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() })));
    }
    return lines;
}

// Renderuje JEDEN komunikat diagnostyczny - parytet z wolna funkcja
// `render(...)` w diagnostics.py. `code` moze byc pustym Str (`""`)
// - odpowiednik `code: str | None = None` w Pythonie (HackerScript w
// tej wersji bootstrapu nie ma `Option<Str>` na tyle wygodnego w
// uzyciu tutaj, wiec pusty string gra role "brak kodu", patrz
// "Ograniczenia").
pub fn render(source: &String, filename: &String, line: i64, col: i64, message: &String, code: &String, severity: &String, length: i64) -> String {
    return render_from_lines(&split_lines(&source), &filename, line, col, &message, &code, &severity, length).to_string();
}

// Rdzen `render` - przyjmuje JUZ POPODZIELONE linie zamiast surowego
// `source`, zeby `render_many` moglo wywolac `split_lines` RAZ dla
// calego pliku i podzielic sie wynikiem miedzy WSZYSTKIE diagnostyki,
// zamiast wolac `split_lines` (petla O(n) po calym zrodle) OSOBNO dla
// KAZDEJ diagnostyki - przy 50+ diagnostykach na duzym pliku to byl
// ogromny, latwy do uniknienia narzut. Bug wydajnosciowy znaleziony
// przy uzyciu skompilowanego stage1 (samo-hostowanego hackerc) do
// zbudowania duzych plikow w tej sesji - patrz TEZ komentarz przy
// `split_lines` (druga polowa tego samego bledu).
pub fn render_from_lines(lines: &Vec<String>, filename: &String, line: i64, col: i64, message: &String, code: &String, severity: &String, length: i64) -> String {
    let mut total = (lines.len() as i64);
    let mut line_idx: i64 = max_int(0, min_int((line - 1), (total - 1)));
    let mut src_line: String = "".to_string();
    if (total > 0) {
        src_line = lines[line_idx as usize].clone();
    }
    let mut gutter: String = (line).to_string();
    let mut gutter_width = (gutter.len() as i64);
    let mut pad: String = repeat_char(&" ".to_string(), gutter_width);
    let mut safe_col: i64 = max_int(1, col);
    let mut safe_length: i64 = max_int(1, length);
    let mut caret_line: String = format!("{}{}", repeat_char(&" ".to_string(), (safe_col - 1)), repeat_char(&"^".to_string(), safe_length));
    let mut tag: String = format!("{}{}", severity, "".to_string());
    if (code.to_string() != "".to_string().to_string()) {
        tag = format!("{}{}", format!("{}{}", format!("{}{}", severity, "[".to_string()), code), "]".to_string());
    }
    let mut out: String = format!("{}{}", format!("{}{}", format!("{}{}", tag, ": ".to_string()), message), "\n".to_string());
    out = format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", out, pad), " --> ".to_string()), filename), ":".to_string()), (line).to_string()), ":".to_string()), (safe_col).to_string()), "\n".to_string());
    out = format!("{}{}", format!("{}{}", out, pad), " |\n".to_string());
    out = format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", out, gutter), " | ".to_string()), src_line), "\n".to_string());
    out = format!("{}{}", format!("{}{}", format!("{}{}", out, pad), " | ".to_string()), caret_line);
    return out.to_string();
}

impl Diagnostic {
    /// Metoda-wygoda odpowiadajaca `Diagnostic.render(self, source)` w
    /// diagnostics.py - po prostu przekazuje wszystkie pola do wolnej
    /// funkcji `render`.
    pub fn render(&self, source: &String) -> String {
        return render(&source, &self.filename, self.line, self.col, &self.message, &self.code, &self.severity, self.length).to_string();
    }

}

// Renderuje LISTE diagnostyk, kazda oddzielona pustym wierszem -
// parytet z `render_many(source, filename, diagnostics)` w
// diagnostics.py. Nadpisuje `filename` na KAZDEJ diagnostyce PRZED
// renderowaniem (tak jak `d.filename = filename` w Pythonie) - dziala
// na LOKALNEJ KOPII kazdego elementu (`diagnostics[i]` klonuje
// non-Copy elementy, patrz codegen.py/`Index`), wiec NIE mutuje
// oryginalnej listy przekazanej przez wywolujacego - to jest ZAMIERZONA
// roznica wobec Pythona (ktory mutuje obiekty w miejscu, bo listy
// Pythona przechowuja referencje) i nie ma znaczenia dla wyniku, bo
// jedynym celem mutacji jest to, co trafia do renderowanego tekstu.
// `split_lines(source)` WOLANE RAZ (nie raz na diagnostyke, patrz
// `render_from_lines`).
pub fn render_many(source: &String, filename: &String, diagnostics: &Vec<Diagnostic>) -> String {
    let mut lines: Vec<String> = split_lines(&source);
    let mut out: String = "".to_string();
    let mut i: i64 = 0;
    let mut total = (diagnostics.len() as i64);
    while (i < total) {
        let mut d: Diagnostic = diagnostics[i as usize].clone();
        let mut block: String = render_from_lines(&lines.clone(), &filename.clone(), d.line, d.col, &d.message.clone(), &d.code.clone(), &d.severity.clone(), d.length);
        if (i > 0) {
            out = format!("{}{}", out, "\n\n".to_string());
        }
        out = format!("{}{}", out, block);
        i = (i + 1);
    }
    return out.to_string();
}

// Demonstracyjne uzycie - jedna diagnostyka na fikcyjnym 3-liniowym
// zrodle, wypisuje sformatowany blok (sprawdza `render`/metode
// `Diagnostic::render` na prawdziwych danych, bez I/O plikowego).
pub fn main() {
    let mut source: String = "fun main() [\n    end\n]\n".to_string();
    let mut d: Diagnostic = Diagnostic::new("error".to_string(), "E0002".to_string(), "brakuje 'end' z wartoscia w funkcji zwracajacej Int".to_string(), 2, 5, 3, "<hcs>".to_string());
    println!("{}", d.render(&source));
}

// ## Ograniczenia tej wersji (patrz docs/ROADMAP.md)
// 
// - `split_lines` traktuje SAMOTNE `\r` (bez towarzyszacego `\n`,
// stary styl linii Mac) jako CZESC tresci linii, NIE jako
// zakonczenie linii - Pythonowe `str.splitlines()` dzieli TEZ na
// samotnym `\r`. W praktyce zrodla `.hcs` w tym repo uzywaja `\n`
// lub `\r\n`, wiec ta roznica nie ma dzis znaczenia, ale jest
// udokumentowanym odstepstwem od Pythona.
// - `code: Str` uzywa PUSTEGO Str (`""`) jako "brak kodu" (parytet z
// `code: str | None = None`) - Diagnostic z PRAWDZIWYM kodem `""`
// (nigdy sie nie zdarza w praktyce - kody to "E0001" itp.) byloby
// nierozroznialne od "brak kodu"; brak realnego ryzyka, ale warto
// pamietac.
// - Brak konstruktora z wartosciami domyslnymi (Pythonowe
// `col: int = 1, length: int = 1, filename: str = "<hcs>"`) -
// HackerScript (w tej wersji bootstrapu) wymaga WSZYSTKICH 7 pol
// przy konstrukcji `Diagnostic(...)`, wiec kazde wywolanie w
// `typecheck.hcs` (kolejny krok) musi je podac explicite.
// - NIEPRZETESTOWANE na prawdziwym wejsciu w tym srodowisku (brak
// rustc) - zweryfikowane strukturalnie przez `hackerc check`/
// `build` i inspekcje wygenerowanego Rusta, patrz
// tests/test_hackerc.py.
