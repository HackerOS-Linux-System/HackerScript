#![allow(non_snake_case, unused_mut, dead_code)]

//! bootstrap/hackerc-self/lexer.hcs
//! 
//! Lekser samo-hostowanego hackerc - PELNA wersja, parytet z
//! hackerc/hackerc/lexer.py (322 linie). Kompilowany dzis przez
//! STAGE0 (Pythonowy hackerc) - patrz bootstrap/README.md.
//! 
//! Rozroznia: `!!` (DocComment, do konca linii), `!` (LineComment, do
//! konca linii, gdy NIE nastepuje `=` - inaczej to operator `!=`),
//! `!= ... =!` (komentarz wieloliniowy - usuwany PRZED tokenizacja,
//! patrz `strip_multiline_comments`, zachowujac liczbe linii przez
//! wstawienie tej samej liczby `\n` w miejsce usunietej tresci),
//! stringi z escape'ami (`\n`, `\t`, `\r`, `\\`, `\"`, `\'`, `\0`),
//! liczby (surowy tekst, `.` dopuszczony - rozroznienie Int/Float
//! zostaje po stronie parsera/codegen, tak jak w oryginale),
//! identyfikatory/slowa kluczowe, `?` jako WLASNY TokKind (Question,
//! nie Op - parytet z `TokKind.QUESTION` w lexer.py), flaga `tight`
//! na tokenie `Open` (`[` bez bialej spacji przed nim - odroznia
//! `xs[i]` (indeksowanie, postfiks) od nowego bloku `[ ... ]` -
//! patrz "Ograniczenia" nizej co do tego, GDZIE ta flaga jest juz
//! faktycznie wykorzystywana).
#[derive(Debug, Clone, PartialEq)]
pub enum TokKind {
    Newline,
    Open,
    Close,
    LParen,
    RParen,
    LAngle,
    RAngle,
    Colon,
    DColon,
    Comma,
    Op,
    Number,
    StrLit,
    Ident,
    Keyword,
    DocComment,
    LineComment,
    Question,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokKind,
    pub value: String,
    pub line: i64,
    pub col: i64,
    pub tight: bool,
}

impl Token {
    pub fn new(kind: TokKind, value: String, line: i64, col: i64, tight: bool) -> Self {
        Token { kind, value, line, col, tight }
    }
}

pub fn is_digit(c: &String) -> bool {
    return ((c.to_string() >= "0".to_string().to_string()) && (c.to_string() <= "9".to_string().to_string()));
}

pub fn is_alpha(c: &String) -> bool {
    return ((((c.to_string() >= "a".to_string().to_string()) && (c.to_string() <= "z".to_string().to_string())) || ((c.to_string() >= "A".to_string().to_string()) && (c.to_string() <= "Z".to_string().to_string()))) || (c.to_string() == "_".to_string().to_string()));
}

pub fn is_alnum(c: &String) -> bool {
    return (is_alpha(&c) || is_digit(&c));
}

// Lista slow kluczowych jezyka - musi byc trzymana w zgodzie z
// hackerc/hackerc/lexer.py::KEYWORDS. Bez Set-literalu w tej wersji
// uzywamy zwyklej petli po List<Str> (patrz "Ograniczenia" - O(n) na
// kazde sprawdzenie, akceptowalne dla malych plikow zrodlowych .hcs).
pub fn is_keyword(w: &String) -> bool {
    let mut kws: Vec<String> = vec!["fun".to_string(), "let".to_string(), "const".to_string(), "if".to_string(), "else".to_string(), "elif".to_string(), "while".to_string(), "for".to_string(), "in".to_string(), "return".to_string(), "end".to_string(), "get".to_string(), "import".to_string(), "using".to_string(), "direct".to_string(), "manual".to_string(), "true".to_string(), "false".to_string(), "null".to_string(), "struct".to_string(), "enum".to_string(), "match".to_string(), "break".to_string(), "continue".to_string(), "gc".to_string(), "pub".to_string(), "self".to_string(), "and".to_string(), "or".to_string(), "not".to_string(), "extern".to_string(), "as".to_string(), "impl".to_string(), "include".to_string()];
    let mut i: i64 = 0;
    while (i < (kws.len() as i64)) {
        if (kws[i as usize].clone().to_string() == w.to_string()) {
            return true;
        }
        i = (i + 1);
    }
    return false;
}

pub fn is_multi_op(s: &String) -> bool {
    return ((((((((((((s.to_string() == "==".to_string().to_string()) || (s.to_string() == "!=".to_string().to_string())) || (s.to_string() == "<=".to_string().to_string())) || (s.to_string() == ">=".to_string().to_string())) || (s.to_string() == "->".to_string().to_string())) || (s.to_string() == "::".to_string().to_string())) || (s.to_string() == "&&".to_string().to_string())) || (s.to_string() == "||".to_string().to_string())) || (s.to_string() == "+=".to_string().to_string())) || (s.to_string() == "-=".to_string().to_string())) || (s.to_string() == "*=".to_string().to_string())) || (s.to_string() == "/=".to_string().to_string()));
}

pub fn is_op_char(c: &String) -> bool {
    return ((((((((((((c.to_string() == "+".to_string().to_string()) || (c.to_string() == "-".to_string().to_string())) || (c.to_string() == "*".to_string().to_string())) || (c.to_string() == "/".to_string().to_string())) || (c.to_string() == "%".to_string().to_string())) || (c.to_string() == "=".to_string().to_string())) || (c.to_string() == "!".to_string().to_string())) || (c.to_string() == ".".to_string().to_string())) || (c.to_string() == "&".to_string().to_string())) || (c.to_string() == "|".to_string().to_string())) || (c.to_string() == "^".to_string().to_string())) || (c.to_string() == "~".to_string().to_string()));
}

// Znaki, po ktorych `!=` MOZE byc operatorem nierownosci (koniec
// wyrazenia po lewej: litera/cyfra/`_`/`)`/`]`/cudzyslow) - parytet
// z `_EXPR_END_CHARS` w lexer.py. Uzywane tylko przez
// `looks_like_operator_before`, ktore odrozniamy od `strip_multiline_comments`.
pub fn is_expr_end_char(c: &String) -> bool {
    return ((((is_alnum(&c) || (c.to_string() == ")".to_string().to_string())) || (c.to_string() == "]".to_string().to_string())) || (c.to_string() == "\"".to_string().to_string())) || (c.to_string() == "'".to_string().to_string()));
}

// Dwuznakowy podglad od pozycji `i` (albo pojedynczy znak, gdy `i` to
// ostatnia pozycja w zrodle) - uzywane do rozpoznawania operatorow
// wieloznakowych (`==`, `->`, `::`, ...) bez wychodzenia poza `n`.
// Reczny `trim` (usuwa spacje/taby/`\r` z obu koncow) - napisany na
// `char_at`/`slice`, BEZ uzycia Rustowego `.trim()` (ktory zwraca
// `&str`, nie `String` - podstawienie go pod `Str` skompilowaloby
// sie do niezgodnosci typow w wygenerowanym Rust, niewykrywalnej
// przez dzisiejszy Pythonowy `typecheck.py`, ktory nie sledzi tego
// konkretnego metod-callu - patrz "Ograniczenia").
pub fn trim_str(s: &String) -> String {
    let __hks_chars_s: Vec<char> = s.chars().collect();
    let mut n = (s.len() as i64);
    let mut start: i64 = 0;
    while ((start < n) && ((((__hks_chars_s.get(start as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == " ".to_string().to_string()) || ((__hks_chars_s.get(start as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "\t".to_string().to_string())) || ((__hks_chars_s.get(start as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "\r".to_string().to_string()))) {
        start = (start + 1);
    }
    let mut stop = n;
    while ((stop > start) && ((((__hks_chars_s.get((stop - 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == " ".to_string().to_string()) || ((__hks_chars_s.get((stop - 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "\t".to_string().to_string())) || ((__hks_chars_s.get((stop - 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "\r".to_string().to_string()))) {
        stop = (stop - 1);
    }
    return ({ let __v = &__hks_chars_s; let __s = ((start) as usize).min(__v.len()); let __e = ((stop) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() }).to_string();
}

// `two_char_at` (pobieranie 2 znakow na pozycji `i`) BYLO tu, ale
// zostalo USUNIETE - wywolywanie GO jako osobnej funkcji z ciasnej
// petli `tokenize`/`strip_multiline_comments` bylo prawdziwym zrodlem
// O(n^2) w calym bootstrapie (WIEKSZYM niz brak cache'owania
// `.char_at`/`.slice`, ktore naprawiono wczesniej w tej samej sesji):
// kazde wywolanie `two_char_at(source, i, n)` materializowalo WLASNA,
// SWIEZA kopie `Vec<char>` z CALEGO `source` (bo `char_indexed_str_params`
// widzi 2+ uzycia `.char_at`/`.slice` na `source` W CIELE `two_char_at`
// i include cache'owanie TAM) - dla pliku ktory sam siebie kompiluje
// (np. `codegen.hcs`, ~137KB), wywolywane TYSIACE razy w petli
// `tokenize`, dawalo to O(n) PRZY KAZDYM WYWOLANIU zamiast O(1).
// Cache'owanie dziala TYLKO W OBREBIE JEDNEJ funkcji - nie przenosi
// sie przez granice wywolania. Naprawa: kazde miejsce wywolania
// `two_char_at(source, i, n) == "XY"` zastapione WPROST przez
// `i + 1 < n and source.char_at(i) == "X" and source.char_at(i + 1) == "Y"`
// (dwa BEZPOSREDNIE `.char_at()` na JUZ zcache'owanej zmiennej
// WEWNATRZ tokenize/strip_multiline_comments, oba O(1)) - patrz
// bootstrap/README.md za pelny opis. Bug znaleziony przy uzyciu
// skompilowanego stage1 (samo-hostowanego hackerc) do zbudowania
// DUZYCH plikow samego siebie w tej sesji.
// `looks_like_operator_before` (cofanie sie od `pos` przez spacje/taby
// i sprawdzanie czy poprzedni niebialy znak konczy wyrazenie, patrz
// `is_expr_end_char`) BYLO tu jako osobna funkcja - USUNIETE i
// wcielone WPROST do `strip_multiline_comments` (jedynego wywolujacego)
// z tego samego powodu co `two_char_at` wyzej: wywolywanie oddzielnej
// funkcji przyjmujacej `source: Str` z ciasnej petli niweczylo
// cache'owanie `.char_at`/`.slice` (dziala TYLKO w obrebie jednej
// funkcji) - patrz duzy komentarz przy `two_char_at` i
// bootstrap/README.md.
// Usuwa komentarze `!= ... =!` (wieloliniowe), zamieniajac cala ich
// tresc na TA SAMA liczbe znakow `\n` (zeby numery linii sie nie
// przesunely dla reszty tokenizacji) - parytet ze
// `strip_comments` w lexer.py. Stringi i komentarze
// `!`/`!!` (do konca linii) sa kopiowane 1:1 BEZ interpretacji PRZED
// testem `!=` - bug znaleziony w POPRZEDNICH sesjach (patrz
// docs/ROADMAP.md): `!=` WEWNATRZ stringa albo wewnatrz TRESCI
// takiego komentarza nie moze otwierac komentarza wieloliniowego.
pub fn strip_multiline_comments(source: &String) -> String {
    let __hks_chars_source: Vec<char> = source.chars().collect();
    let mut out: String = "".to_string();
    let mut i: i64 = 0;
    let mut n = (source.len() as i64);
    while (i < n) {
        let mut c: String = (__hks_chars_source.get(i as usize).map(|c| c.to_string()).unwrap_or_default());
        let mut is_double_eq: bool = ((((i + 1) < n) && ((__hks_chars_source.get(i as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "!".to_string().to_string())) && ((__hks_chars_source.get((i + 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "=".to_string().to_string()));
        let mut op_before: bool = false;
        if is_double_eq {
            let mut __k: i64 = (i - 1);
            while ((__k >= 0) && (((__hks_chars_source.get(__k as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == " ".to_string().to_string()) || ((__hks_chars_source.get(__k as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "\t".to_string().to_string()))) {
                __k = (__k - 1);
            }
            if (__k >= 0) {
                op_before = is_expr_end_char(&(__hks_chars_source.get(__k as usize).map(|c| c.to_string()).unwrap_or_default()));
            }
        }
        if ((c.to_string() == "\"".to_string().to_string()) || (c.to_string() == "'".to_string().to_string())) {
            let mut quote: String = (c).to_string();
            let mut j: i64 = (i + 1);
            while (((j < n) && ((__hks_chars_source.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() != quote.to_string())) && ((__hks_chars_source.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() != "\n".to_string().to_string())) {
                if (((__hks_chars_source.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "\\".to_string().to_string()) && ((j + 1) < n)) {
                    j = (j + 2);
                } else {
                    j = (j + 1);
                }
            }
            if ((j < n) && ((__hks_chars_source.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == quote.to_string())) {
                j = (j + 1);
            }
            out = format!("{}{}", out, ({ let __v = &__hks_chars_source; let __s = ((i) as usize).min(__v.len()); let __e = ((j) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() }));
            i = j;
        } else if ((c.to_string() == "!".to_string().to_string()) && !(is_double_eq)) {
            let mut j: i64 = i;
            while ((j < n) && ((__hks_chars_source.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() != "\n".to_string().to_string())) {
                j = (j + 1);
            }
            out = format!("{}{}", out, ({ let __v = &__hks_chars_source; let __s = ((i) as usize).min(__v.len()); let __e = ((j) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() }));
            i = j;
        } else if (is_double_eq && !(op_before)) {
            let mut j: i64 = (i + 2);
            let mut found: bool = false;
            while ((j < n) && !(found)) {
                if ((((j + 1) < n) && ((__hks_chars_source.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "=".to_string().to_string())) && ((__hks_chars_source.get((j + 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "!".to_string().to_string())) {
                    found = true;
                } else {
                    j = (j + 1);
                }
            }
            let mut block: String = ({ let __v = &__hks_chars_source; let __s = ((i) as usize).min(__v.len()); let __e = ((j) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() });
            let mut k: i64 = 0;
            while (k < (block.len() as i64)) {
                if ((block.chars().nth(k as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "\n".to_string().to_string()) {
                    out = format!("{}{}", out, "\n".to_string());
                }
                k = (k + 1);
            }
            i = (j + 2);
        } else {
            out = format!("{}{}", out, c);
            i = (i + 1);
        }
    }
    return out.to_string();
}

// Zamienia jeden znak escape'u (bez `\`) na jego realna wartosc, np.
// `n` -> nowa linia. `""` (pusty Str) sygnalizuje "nierozpoznany
// escape" - wywolujacy (`tokenize`) w tym przypadku zachowuje OBA
// znaki doslownie (`\` + znak), tak jak `_STRING_ESCAPES.get(esc)`
// zwracajace `None` w lexer.py.
pub fn resolve_escape(c: &String) -> String {
    if (c.to_string() == "n".to_string().to_string()) {
        return "\n".to_string();
    }
    if (c.to_string() == "t".to_string().to_string()) {
        return "\t".to_string();
    }
    if (c.to_string() == "r".to_string().to_string()) {
        return "\r".to_string();
    }
    if (c.to_string() == "\\".to_string().to_string()) {
        return "\\".to_string();
    }
    if (c.to_string() == "\"".to_string().to_string()) {
        return "\"".to_string();
    }
    if (c.to_string() == "'".to_string().to_string()) {
        return "'".to_string();
    }
    if (c.to_string() == "0".to_string().to_string()) {
        return " ".to_string();
    }
    return "".to_string();
}

// Tokenizuje zrodlo HackerScript. Zwraca liste tokenow zakonczona
// zawsze tokenem Eof. Woa `strip_multiline_comments` jako pierwszy
// krok (parytet z `tokenize(source)` w lexer.py, ktore rowniez
// zaczyna od `strip_comments(source)`).
pub fn tokenize(source: &String) -> Vec<Token> {
    let mut src: String = strip_multiline_comments(&source);
    let __hks_chars_src: Vec<char> = src.chars().collect();
    let mut tokens: Vec<Token> = vec![];
    let mut i: i64 = 0;
    let mut n = (src.len() as i64);
    let mut line: i64 = 1;
    let mut line_start: i64 = 0;
    while (i < n) {
        let mut col: i64 = ((i - line_start) + 1);
        let mut c: String = (__hks_chars_src.get(i as usize).map(|c| c.to_string()).unwrap_or_default());
        let mut prev_is_space: bool = ((i > 0) && (((((__hks_chars_src.get((i - 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == " ".to_string().to_string()) || ((__hks_chars_src.get((i - 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "\t".to_string().to_string())) || ((__hks_chars_src.get((i - 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "\n".to_string().to_string())) || ((__hks_chars_src.get((i - 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "\r".to_string().to_string())));
        let mut is_tight: bool = ((i > 0) && !(prev_is_space));
        if (c.to_string() == "\n".to_string().to_string()) {
            tokens.push(Token::new(TokKind::Newline, (c).to_string(), line, col, false));
            i = (i + 1);
            line = (line + 1);
            line_start = i;
        } else if (((c.to_string() == " ".to_string().to_string()) || (c.to_string() == "\t".to_string().to_string())) || (c.to_string() == "\r".to_string().to_string())) {
            i = (i + 1);
        } else if ((((c.to_string() == "!".to_string().to_string()) && ((i + 1) < n)) && ((__hks_chars_src.get(i as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "!".to_string().to_string())) && ((__hks_chars_src.get((i + 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "!".to_string().to_string())) {
            let mut j: i64 = (i + 2);
            while ((j < n) && ((__hks_chars_src.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() != "\n".to_string().to_string())) {
                j = (j + 1);
            }
            tokens.push(Token::new(TokKind::DocComment, trim_str(&({ let __v = &__hks_chars_src; let __s = (((i + 2)) as usize).min(__v.len()); let __e = ((j) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() })), line, col, false));
            i = j;
        } else if ((c.to_string() == "!".to_string().to_string()) && !(((((i + 1) < n) && ((__hks_chars_src.get(i as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "!".to_string().to_string())) && ((__hks_chars_src.get((i + 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "=".to_string().to_string())))) {
            let mut j: i64 = (i + 1);
            while ((j < n) && ((__hks_chars_src.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() != "\n".to_string().to_string())) {
                j = (j + 1);
            }
            tokens.push(Token::new(TokKind::LineComment, trim_str(&({ let __v = &__hks_chars_src; let __s = (((i + 1)) as usize).min(__v.len()); let __e = ((j) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() })), line, col, false));
            i = j;
        } else if is_digit(&c) {
            let mut j: i64 = i;
            while ((j < n) && (is_digit(&(__hks_chars_src.get(j as usize).map(|c| c.to_string()).unwrap_or_default())) || ((__hks_chars_src.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == ".".to_string().to_string()))) {
                j = (j + 1);
            }
            tokens.push(Token::new(TokKind::Number, ({ let __v = &__hks_chars_src; let __s = ((i) as usize).min(__v.len()); let __e = ((j) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() }), line, col, false));
            i = j;
        } else if ((c.to_string() == "\"".to_string().to_string()) || (c.to_string() == "'".to_string().to_string())) {
            let mut quote: String = (c).to_string();
            let mut j: i64 = (i + 1);
            let mut buf: String = "".to_string();
            while (((j < n) && ((__hks_chars_src.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() != quote.to_string())) && ((__hks_chars_src.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() != "\n".to_string().to_string())) {
                if (((__hks_chars_src.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "\\".to_string().to_string()) && ((j + 1) < n)) {
                    let mut esc: String = (__hks_chars_src.get((j + 1) as usize).map(|c| c.to_string()).unwrap_or_default());
                    let mut resolved: String = resolve_escape(&esc);
                    if (resolved.to_string() == "".to_string().to_string()) {
                        buf = format!("{}{}", format!("{}{}", buf, "\\".to_string()), esc);
                    } else {
                        buf = format!("{}{}", buf, resolved);
                    }
                    j = (j + 2);
                } else {
                    buf = format!("{}{}", buf, (__hks_chars_src.get(j as usize).map(|c| c.to_string()).unwrap_or_default()));
                    j = (j + 1);
                }
            }
            if ((j < n) && ((__hks_chars_src.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == quote.to_string())) {
                j = (j + 1);
            }
            tokens.push(Token::new(TokKind::StrLit, (buf).to_string(), line, col, false));
            i = j;
        } else if is_alpha(&c) {
            let mut j: i64 = i;
            while ((j < n) && is_alnum(&(__hks_chars_src.get(j as usize).map(|c| c.to_string()).unwrap_or_default()))) {
                j = (j + 1);
            }
            let mut text: String = ({ let __v = &__hks_chars_src; let __s = ((i) as usize).min(__v.len()); let __e = ((j) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() });
            if is_keyword(&text) {
                tokens.push(Token::new(TokKind::Keyword, (text).to_string(), line, col, false));
            } else {
                tokens.push(Token::new(TokKind::Ident, (text).to_string(), line, col, false));
            }
            i = j;
        } else if (c.to_string() == "[".to_string().to_string()) {
            tokens.push(Token::new(TokKind::Open, (c).to_string(), line, col, is_tight));
            i = (i + 1);
        } else if (c.to_string() == "]".to_string().to_string()) {
            tokens.push(Token::new(TokKind::Close, (c).to_string(), line, col, false));
            i = (i + 1);
        } else if (c.to_string() == "(".to_string().to_string()) {
            tokens.push(Token::new(TokKind::LParen, (c).to_string(), line, col, false));
            i = (i + 1);
        } else if (c.to_string() == ")".to_string().to_string()) {
            tokens.push(Token::new(TokKind::RParen, (c).to_string(), line, col, false));
            i = (i + 1);
        } else if (c.to_string() == ",".to_string().to_string()) {
            tokens.push(Token::new(TokKind::Comma, (c).to_string(), line, col, false));
            i = (i + 1);
        } else if ((((i + 1) < n) && ((__hks_chars_src.get(i as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == ":".to_string().to_string())) && ((__hks_chars_src.get((i + 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == ":".to_string().to_string())) {
            tokens.push(Token::new(TokKind::DColon, "::".to_string(), line, col, false));
            i = (i + 2);
        } else if (c.to_string() == ":".to_string().to_string()) {
            tokens.push(Token::new(TokKind::Colon, (c).to_string(), line, col, false));
            i = (i + 1);
        } else if ((((i + 1) < n) && ((__hks_chars_src.get(i as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "<".to_string().to_string())) && ((__hks_chars_src.get((i + 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "=".to_string().to_string())) {
            tokens.push(Token::new(TokKind::Op, "<=".to_string(), line, col, false));
            i = (i + 2);
        } else if (c.to_string() == "<".to_string().to_string()) {
            tokens.push(Token::new(TokKind::LAngle, (c).to_string(), line, col, false));
            i = (i + 1);
        } else if ((((i + 1) < n) && ((__hks_chars_src.get(i as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == ">".to_string().to_string())) && ((__hks_chars_src.get((i + 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "=".to_string().to_string())) {
            tokens.push(Token::new(TokKind::Op, ">=".to_string(), line, col, false));
            i = (i + 2);
        } else if (c.to_string() == ">".to_string().to_string()) {
            tokens.push(Token::new(TokKind::RAngle, (c).to_string(), line, col, false));
            i = (i + 1);
        } else if (c.to_string() == "?".to_string().to_string()) {
            tokens.push(Token::new(TokKind::Question, (c).to_string(), line, col, false));
            i = (i + 1);
        } else if is_op_char(&c) {
            let mut two: String = (c).to_string();
            if ((i + 1) < n) {
                two = format!("{}{}", c, (__hks_chars_src.get((i + 1) as usize).map(|c| c.to_string()).unwrap_or_default()));
            }
            if is_multi_op(&two) {
                tokens.push(Token::new(TokKind::Op, (two).to_string(), line, col, false));
                i = (i + 2);
            } else {
                tokens.push(Token::new(TokKind::Op, (c).to_string(), line, col, false));
                i = (i + 1);
            }
        } else {
            i = (i + 1);
        }
    }
    tokens.push(Token::new(TokKind::Eof, "".to_string(), line, ((i - line_start) + 1), false));
    return tokens;
}

// Demonstracyjne uzycie - tokenizuje fragment HackerScript zawierajacy
// string z escape'em, `!!`/`!` komentarz, `?` i operator `!=`,
// zaszyty w kodzie (bez I/O plikowego), i wypisuje liczbe tokenow.
pub fn main() {
    let mut sample: String = "!! doc\n! line\nfun main() [\n    let x = 1 + 2\n    let s = \"a\\nb\"\n    if x != 3 [\n        log(x?)\n    ]\n]\n".to_string();
    let mut tokens: Vec<Token> = tokenize(&sample);
    println!("{} {}", "tokens:".to_string(), (tokens.len() as i64));
}

// ## Ograniczenia tej wersji (patrz docs/ROADMAP.md)
// 
// - Liczby: brak jawnego rozroznienia Int/Float na tym etapie (jak w
// oryginalnym lexer.py - to robi parser na podstawie obecnosci '.').
// - `is_keyword`-owe sprawdzenie to petla O(n) zamiast HashSet - dla
// malych plikow .hcs bez znaczenia wydajnosciowego.
// - Nieznane znaki sa CICHO pomijane zamiast zglaszac blad leksykalny
// (`LexError` w Pythonowym lexer.py) - samo-hostowany bootstrap nie
// ma jeszcze reprezentacji bledow z komunikatem+linia+kolumna poza
// `log`, to czeka na `diagnostics.hcs` (nastepny krok).
// - Flaga `tight` na `Token::Open` jest juz OBLICZANA poprawnie (brak
// bialej spacji miedzy poprzednim znakiem a `[`), ale NIC W TYM
// PLIKU jeszcze z niej nie korzysta - to zadanie parsera
// (`parse_postfix` w kolejnym kroku), ktory musi odrozniac
// `xs[i]` (tight=true, indeksowanie) od `if cond [ ... ]`
// (tight=false, nowy blok).
// - NIEPRZETESTOWANE na prawdziwym wejsciu w tym srodowisku (brak
// rustc) - zweryfikowane strukturalnie przez `hackerc check` i
// inspekcje wygenerowanego Rusta, patrz tests/test_hackerc.py.
