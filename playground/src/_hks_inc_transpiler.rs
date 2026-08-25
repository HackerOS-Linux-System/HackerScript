#![allow(non_snake_case, unused_mut, dead_code)]

//! bootstrap/hackerc-self/transpiler.hcs
//! 
//! Krok 7/N przepisania calego hackerc na HackerScript (patrz
//! bootstrap/README.md). Parytet z hackerc/hackerc/transpiler.py (148
//! linii) - publiczne API "zrodlo .hcs -> kod Rust (+ needs_pyo3)":
//! 
//! let wynik = transpile_source_full(zrodlo, "<hcs>", "module", pusta_extra())
//! 
//! `__direct__(0)` to CZYSTY kod Pythona wstawiany do wygenerowanego
//! Rusta jako string (PyO3 `Python::with_gil`). Wyciagany jest TU, na
//! etapie preprocessingu (przed tokenizacja HackerScript), zeby
//! `lexer.hcs`/`parser.hcs` nie musialy wiedziec nic o skladni Pythona -
//! dokladnie jak w oryginalnym `_extract_direct_blocks`.
//! 
//! Kompilowany dzis przez STAGE0 (Pythonowy hackerc) - patrz
//! bootstrap/README.md. Zaleznosci: `ast_nodes.hcs` (Stmt/Program),
//! `lexer.hcs`+`parser.hcs` (`parse`), `typeinfer.hcs` (`Signatures`/
//! `collect_signatures`), `codegen.hcs` (reszta pipeline'u generacji).
//! 
//! ## Roznica architektoniczna vs transpiler.py (WAZNE)
//! 
//! Pythonowy `transpile_source_full` propaguje bledy parsera/kodegenu
//! jako wyjatki `TranspileError` (z `line`/`col`). Ten bootstrap NIE
//! MOZE tego zrobic wiernie: `parser.hcs::parse()` sam w sobie NIE MA
//! `Result`/wyjatkow - w razie nieoczekiwanego tokenu robi `log(...)` i
//! probuje kontynuowac (best-effort), patrz "Ograniczenia" w
//! parser.hcs. To jest ograniczenie WYZEJ w stosie (parser.hcs, nie
//! ten plik) - `transpile_source_full` w tej wersji zwraca WIEC
//! zawsze `TranspileResult` (bez `Result`/bledu) - parytet z
//! "sciezka sukcesu" Pythonowej wersji. Diagnostyka bledow parsera to
//! przyszly krok (podpiecie `diagnostics.hcs` pod `parser.hcs`, patrz
//! `docs/ROADMAP.md` - juz udokumentowane w parser.hcs).
use crate::_hks_inc_ast_nodes::*;
use crate::_hks_inc_parser::*;
use crate::_hks_inc_typeinfer::*;
use crate::_hks_inc_codegen::*;
// Wynik transpilacji - parytet z `@dataclass TranspileResult`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TranspileResult {
    pub rust_code: String,
    pub needs_pyo3: bool,
}

impl TranspileResult {
    pub fn new(rust_code: String, needs_pyo3: bool) -> Self {
        TranspileResult { rust_code, needs_pyo3 }
    }
}

// Sygnatury/mut-maps "z zewnatrz" (z INNYCH plikow .hcs, zaimportowanych
// przez `get <std/core/selfhost:...>`) - parytet z parametrami
// `extra_functions`/`extra_structs`/`extra_enums`/`extra_mut_params`/
// `extra_methods`/`extra_method_mut_params` w `transpile_source_full`
// Pythona. HackerScript nie ma iteracji po `Dict` (brak `.keys()`/
// `.items()`, patrz "Ograniczenia" w typeinfer.hcs) - kazdy Dict tu
// MUSI wiec byc towarzyszony wlasna lista nazw-kluczy (`*_names`),
// zeby `merge_signatures_with_extra` mogl "przejrzec" jego zawartosc.
// Wypelnia to `project.hcs` (`collect_project_signatures`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExtraSignatures {
    pub functions: std::collections::HashMap<String, Stmt>,
    pub function_names: Vec<String>,
    pub structs: std::collections::HashMap<String, Stmt>,
    pub struct_names: Vec<String>,
    pub enums: std::collections::HashMap<String, Stmt>,
    pub enum_names: Vec<String>,
    pub methods: std::collections::HashMap<String, Stmt>,
    pub method_names: Vec<String>,
    pub mut_params: std::collections::HashMap<String, bool>,
    pub method_mut_params: std::collections::HashMap<String, bool>,
}

impl ExtraSignatures {
    pub fn new(functions: std::collections::HashMap<String, Stmt>, function_names: Vec<String>, structs: std::collections::HashMap<String, Stmt>, struct_names: Vec<String>, enums: std::collections::HashMap<String, Stmt>, enum_names: Vec<String>, methods: std::collections::HashMap<String, Stmt>, method_names: Vec<String>, mut_params: std::collections::HashMap<String, bool>, method_mut_params: std::collections::HashMap<String, bool>) -> Self {
        ExtraSignatures { functions, function_names, structs, struct_names, enums, enum_names, methods, method_names, mut_params, method_mut_params }
    }
}

// `ExtraSignatures` pusty - dla wywolan `transpile_source_full` na
// pojedynczym pliku, bez kontekstu calego projektu (parytet z
// wywolaniem Pythona z domyslnymi `extra_* = None`).
pub fn empty_extra_signatures() -> ExtraSignatures {
    let mut functions: std::collections::HashMap<String, Stmt> = std::collections::HashMap::new();
    let mut structs: std::collections::HashMap<String, Stmt> = std::collections::HashMap::new();
    let mut enums: std::collections::HashMap<String, Stmt> = std::collections::HashMap::new();
    let mut methods: std::collections::HashMap<String, Stmt> = std::collections::HashMap::new();
    let mut mut_params: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let mut method_mut_params: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    return ExtraSignatures::new((functions).clone(), vec![], (structs).clone(), vec![], (enums).clone(), vec![], (methods).clone(), vec![], (mut_params).clone(), (method_mut_params).clone());
}

// Dopisuje do `owner` mapowanie wariant -> nazwa enuma dla JEDNEGO
// `EnumDecl` (parytet z petla po `enum_decl.variants` w
// `_build_variant_registry` z codegen.py - tu wydzielone, bo ten
// plik musi to policzyc TEZ dla enumow pochodzacych z `extra.enums`,
// ktore `collect_signatures` (dzialajace tylko na LOKALNYM programie)
// nie widzi).
pub fn collect_variant_owner_for_enum(e: &Stmt, owner: &mut std::collections::HashMap<String, String>) {
    match e {
        Stmt::EnumDecl(name, variants, type_params) => {
            let mut k: i64 = 0;
            let mut vn = (variants.len() as i64);
            while (k < vn) {
                owner.insert(variants[k as usize].name.clone(), name.clone());
                k = (k + 1);
            }
        }
        _ => {
        }
    }
}

// Laczy `local` (Signatures policzone TYLKO dla tego pliku przez
// `collect_signatures`) z `extra` (globalny rejestr z reszty
// projektu) - LOKALNE wpisy maja PIERWSZENSTWO (parytet z Pythonowym
// `{**extra_functions, **gen.sigs.functions}` - drugi operand
// nadpisuje pierwszy w skladni `{**a, **b}`, wiec lokalne wygrywaja).
// Iteruje `extra.*_names` (patrz `ExtraSignatures` wyzej) zamiast
// iterowac Dict bezposrednio - ten bootstrap nie ma iteracji po Dict.
pub fn merge_signatures_with_extra(local: &Signatures, extra: &ExtraSignatures) -> Signatures {
    let mut functions = local.functions.clone();
    let mut function_names = local.function_names.clone();
    let mut fi: i64 = 0;
    let mut fn_n = (extra.function_names.len() as i64);
    while (fi < fn_n) {
        let mut name: String = extra.function_names[fi as usize].clone();
        match local.functions.get(name.as_str()).cloned() {
            Some(_v) => {
            }
            None => {
                match extra.functions.get(name.as_str()).cloned() {
                    Some(v) => {
                        functions.insert(name.clone(), v);
                        function_names.push((name).to_string());
                    }
                    None => {
                    }
                }
            }
        }
        fi = (fi + 1);
    }
    let mut structs = local.structs.clone();
    let mut struct_names = local.struct_names.clone();
    let mut si: i64 = 0;
    let mut st_n = (extra.struct_names.len() as i64);
    while (si < st_n) {
        let mut name: String = extra.struct_names[si as usize].clone();
        match local.structs.get(name.as_str()).cloned() {
            Some(_v) => {
            }
            None => {
                match extra.structs.get(name.as_str()).cloned() {
                    Some(v) => {
                        structs.insert(name.clone(), v);
                        struct_names.push((name).to_string());
                    }
                    None => {
                    }
                }
            }
        }
        si = (si + 1);
    }
    let mut enums = local.enums.clone();
    let mut enum_names = local.enum_names.clone();
    let mut variant_owner = local.variant_owner.clone();
    let mut ei: i64 = 0;
    let mut en_n = (extra.enum_names.len() as i64);
    while (ei < en_n) {
        let mut name: String = extra.enum_names[ei as usize].clone();
        match local.enums.get(name.as_str()).cloned() {
            Some(_v) => {
            }
            None => {
                match extra.enums.get(name.as_str()).cloned() {
                    Some(v) => {
                        enums.insert(name.clone(), v.clone());
                        enum_names.push((name).to_string());
                        collect_variant_owner_for_enum(&v, &mut variant_owner);
                    }
                    None => {
                    }
                }
            }
        }
        ei = (ei + 1);
    }
    let mut methods = local.methods.clone();
    let mut mi: i64 = 0;
    let mut m_n = (extra.method_names.len() as i64);
    while (mi < m_n) {
        let mut name: String = extra.method_names[mi as usize].clone();
        match local.methods.get(name.as_str()).cloned() {
            Some(_v) => {
            }
            None => {
                match extra.methods.get(name.as_str()).cloned() {
                    Some(v) => {
                        methods.insert(name.clone(), v);
                    }
                    None => {
                    }
                }
            }
        }
        mi = (mi + 1);
    }
    return Signatures::new(functions, structs, enums, methods, variant_owner, struct_names, enum_names, function_names);
}

// Laczy `local_mut_params` (policzone przez `compute_mut_params` TYLKO
// dla tego pliku) z `extra.mut_params` - lokalne wygrywaja, parytet z
// `{**extra_mut_params, **gen.mut_params}` w `generate()` (codegen.py).
pub fn merge_mut_params_with_extra(local_mut: &std::collections::HashMap<String, bool>, extra: &ExtraSignatures) -> std::collections::HashMap<String, bool> {
    let mut out = local_mut.clone();
    let mut i: i64 = 0;
    let mut n = (extra.function_names.len() as i64);
    while (i < n) {
        let mut name: String = extra.function_names[i as usize].clone();
        match local_mut.get(name.as_str()).cloned() {
            Some(_v) => {
            }
            None => {
                match extra.mut_params.get(name.as_str()).cloned() {
                    Some(v) => {
                        out.insert((name).to_string(), v);
                    }
                    None => {
                    }
                }
            }
        }
        i = (i + 1);
    }
    return out;
}

// Sklada `direct_blocks` (Dict<Int-jako-Str, Str> zebrane przez
// `extract_direct_blocks` nizej) na `Dict<Str, Str>` wymagany przez
// `codegen.hcs::new_codegen` - `idx` jest juz `Str` w tym module
// (patrz `extract_direct_blocks`), wiec to zwykle "przepisanie 1:1",
// wydzielone jako wlasna funkcja dla czytelnosci wywolan nizej.
pub fn direct_blocks_to_dict(blocks: &Vec<String>) -> std::collections::HashMap<String, String> {
    let mut out: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut i: i64 = 0;
    let mut n = (blocks.len() as i64);
    while (i < n) {
        out.insert((i).to_string(), blocks[i as usize].clone().clone());
        i = (i + 1);
    }
    return out;
}

pub fn is_direct_ident_char(c: &String) -> bool {
    if ((c.to_string() >= "a".to_string().to_string()) && (c.to_string() <= "z".to_string().to_string())) {
        return true;
    }
    if ((c.to_string() >= "A".to_string().to_string()) && (c.to_string() <= "Z".to_string().to_string())) {
        return true;
    }
    if ((c.to_string() >= "0".to_string().to_string()) && (c.to_string() <= "9".to_string().to_string())) {
        return true;
    }
    if (c.to_string() == "_".to_string().to_string()) {
        return true;
    }
    return false;
}

// Reczny odpowiednik minimalnego wciecia wspolnego dla wszystkich
// NIEPUSTYCH linii (parytet z `textwrap.dedent` - UPROSZCZONY: liczy
// DLUGOSC wiodacych spacji/tabow, nie porownuje same znaki litera po
// literze jak prawdziwy `textwrap.dedent` - wystarczajace dla kodu
// `.hcs`, ktory zawsze uzywa spacji konsekwentnie, patrz
// "Ograniczenia" na koncu pliku).
pub fn common_leading_whitespace(lines: &Vec<String>) -> i64 {
    let mut min_indent: i64 = -(1);
    let mut i: i64 = 0;
    let mut n = (lines.len() as i64);
    while (i < n) {
        let mut line: String = lines[i as usize].clone();
        let mut llen = (line.len() as i64);
        let mut blank: bool = true;
        let mut j: i64 = 0;
        while (j < llen) {
            let mut c: String = (line.chars().nth(j as usize).map(|c| c.to_string()).unwrap_or_default());
            if !(((c.to_string() == " ".to_string().to_string()) || (c.to_string() == "\t".to_string().to_string()))) {
                blank = false;
            }
            j = (j + 1);
        }
        if !(blank) {
            let mut indent: i64 = 0;
            let mut k: i64 = 0;
            while (k < llen) {
                let mut c: String = (line.chars().nth(k as usize).map(|c| c.to_string()).unwrap_or_default());
                if ((c.to_string() == " ".to_string().to_string()) || (c.to_string() == "\t".to_string().to_string())) {
                    indent = (indent + 1);
                } else {
                    k = llen;
                }
                k = (k + 1);
            }
            if ((min_indent == -(1)) || (indent < min_indent)) {
                min_indent = indent;
            }
        }
        i = (i + 1);
    }
    if (min_indent == -(1)) {
        return 0;
    }
    return min_indent;
}

pub fn dedent_text(raw: &String) -> String {
    let mut lines: Vec<String> = split_lines_local(&raw);
    let mut indent: i64 = common_leading_whitespace(&lines);
    let mut out_lines: Vec<String> = vec![];
    let mut i: i64 = 0;
    let mut n = (lines.len() as i64);
    while (i < n) {
        let mut line: String = lines[i as usize].clone();
        let mut llen = (line.len() as i64);
        if (indent >= llen) {
            out_lines.push("".to_string());
        } else {
            out_lines.push((line.chars().skip(indent as usize).take(((llen) - (indent)) as usize).collect::<String>()));
        }
        i = (i + 1);
    }
    let mut joined: String = "".to_string();
    let mut j: i64 = 0;
    let mut jn = (out_lines.len() as i64);
    while (j < jn) {
        if (j > 0) {
            joined = format!("{}{}", joined, "\n".to_string());
        }
        joined = format!("{}{}", joined, out_lines[j as usize].clone());
        j = (j + 1);
    }
    return strip_newlines(&joined).to_string();
}

// Dzieli po `\n` (BEZ pomijania `\r`, w odroznieniu od
// `diagnostics.hcs::split_lines` - tresc `__direct__(1)` to Python,
// gdzie `\r` wewnatrz linii jest istotny/rzadki, wiec ten plik nie
// ryzykuje go cicho gubic tak jak `diagnostics.hcs` robi to celowo
// dla zrodla `.hcs`).
pub fn split_lines_local(source: &String) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    let mut cur: String = "".to_string();
    let mut i: i64 = 0;
    let mut n = (source.len() as i64);
    while (i < n) {
        let mut c: String = (source.chars().nth(i as usize).map(|c| c.to_string()).unwrap_or_default());
        if (c.to_string() == "\n".to_string().to_string()) {
            lines.push((cur).to_string());
            cur = "".to_string();
        } else {
            cur = format!("{}{}", cur, c);
        }
        i = (i + 1);
    }
    lines.push((cur).to_string());
    return lines;
}

// Parytet z Pythonowym `.strip("\n")` - usuwa TYLKO wiodace/koncowe
// znaki `\n` (nie inne bialе znaki, w odroznieniu od goleg `.strip()`).
pub fn strip_newlines(s: &String) -> String {
    let __hks_chars_s: Vec<char> = s.chars().collect();
    let mut n = (s.len() as i64);
    let mut start: i64 = 0;
    while ((start < n) && ((__hks_chars_s.get(start as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "\n".to_string().to_string())) {
        start = (start + 1);
    }
    let mut end_i = n;
    while ((end_i > start) && ((__hks_chars_s.get((end_i - 1) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "\n".to_string().to_string())) {
        end_i = (end_i - 1);
    }
    return ({ let __v = &__hks_chars_s; let __s = ((start) as usize).min(__v.len()); let __e = ((end_i) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() }).to_string();
}

// Wynik `extract_direct_blocks` - HackerScript nie ma generycznych
// tupli, wiec `(Str, Dict<Int,Str>)` z Pythona staje sie wlasnym
// struct (parytet z `ElifArm`/`Edge` gdzie indziej w bootstrapie).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DirectExtraction {
    pub stripped_source: String,
    pub blocks: Vec<String>,
}

impl DirectExtraction {
    pub fn new(stripped_source: String, blocks: Vec<String>) -> Self {
        DirectExtraction { stripped_source, blocks }
    }
}

// Zamienia kazdy blok `__direct__(2)` na wyrazenie `__direct__(N)` i
// zwraca (nowe_zrodlo, lista_surowych_blokow_w_kolejnosci) - parytet
// z `_extract_direct_blocks` (transpiler.py). `blocks[N]` odpowiada
// Pythonowemu `blocks[N]` (indeksy 0..len-1 w kolejnosci wystapienia).
// 
// Reczne skanowanie zamiast `re.compile(r"\bdirect\b").search` -
// sprawdza znak przed/po dopasowaniu przez `is_direct_ident_char`
// (parytet granicy slowa `\b` dla `[a-zA-Z0-9_]`).
pub fn extract_direct_blocks(source: &String) -> DirectExtraction {
    let __hks_chars_source: Vec<char> = source.chars().collect();
    let mut blocks: Vec<String> = vec![];
    let mut out: String = "".to_string();
    let mut i: i64 = 0;
    let mut n = (source.len() as i64);
    while (i < n) {
        let mut found_at: i64 = -(1);
        let mut scan: i64 = i;
        while ((scan < n) && (found_at == -(1))) {
            if (((scan + 6) <= n) && (({ let __v = &__hks_chars_source; let __s = ((scan) as usize).min(__v.len()); let __e = (((scan + 6)) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() }).to_string() == "direct".to_string().to_string())) {
                let mut before_ok: bool = ((scan == 0) || !(is_direct_ident_char(&(__hks_chars_source.get((scan - 1) as usize).map(|c| c.to_string()).unwrap_or_default()))));
                let mut after_ok: bool = (((scan + 6) >= n) || !(is_direct_ident_char(&(__hks_chars_source.get((scan + 6) as usize).map(|c| c.to_string()).unwrap_or_default()))));
                if (before_ok && after_ok) {
                    found_at = scan;
                }
            }
            scan = (scan + 1);
        }
        if (found_at == -(1)) {
            out = format!("{}{}", out, ({ let __v = &__hks_chars_source; let __s = ((i) as usize).min(__v.len()); let __e = ((n) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() }));
            i = n;
        } else {
            let mut start: i64 = found_at;
            out = format!("{}{}", out, ({ let __v = &__hks_chars_source; let __s = ((i) as usize).min(__v.len()); let __e = ((start) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() }));
            let mut j: i64 = (start + 6);
            while ((j < n) && (((__hks_chars_source.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == " ".to_string().to_string()) || ((__hks_chars_source.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "\t".to_string().to_string()))) {
                j = (j + 1);
            }
            if ((j >= n) || ((__hks_chars_source.get(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() != "[".to_string().to_string())) {
                out = format!("{}{}", out, ({ let __v = &__hks_chars_source; let __s = ((start) as usize).min(__v.len()); let __e = ((j) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() }));
                i = j;
            } else {
                let mut depth: i64 = 0;
                let mut k: i64 = j;
                let mut closed_at: i64 = -(1);
                while ((k < n) && (closed_at == -(1))) {
                    let mut c: String = (__hks_chars_source.get(k as usize).map(|c| c.to_string()).unwrap_or_default());
                    if (c.to_string() == "[".to_string().to_string()) {
                        depth = (depth + 1);
                    } else if (c.to_string() == "]".to_string().to_string()) {
                        depth = (depth - 1);
                        if (depth == 0) {
                            closed_at = k;
                        }
                    }
                    k = (k + 1);
                }
                if (closed_at == -(1)) {
                    println!("{}", "[transpiler.hcs] niezamkniety blok __direct__(3)".to_string());
                    i = n;
                } else {
                    let mut raw: String = ({ let __v = &__hks_chars_source; let __s = (((j + 1)) as usize).min(__v.len()); let __e = ((closed_at) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() });
                    let mut dedented: String = dedent_text(&raw);
                    let mut idx = (blocks.len() as i64);
                    blocks.push((dedented).to_string());
                    out = format!("{}{}", format!("{}{}", format!("{}{}", out, "__direct__(".to_string()), (idx).to_string()), ")".to_string());
                    i = (closed_at + 1);
                }
            }
        }
    }
    return DirectExtraction::new((out).to_string(), (blocks).clone());
}

// Rdzen transpilacji - parytet z `transpile_source_full` (patrz
// uwaga na gorze pliku co do bledow: brak `Result`, zawsze sciezka
// sukcesu). `extra` domyslnie `empty_extra_signatures()` dla
// pojedynczego pliku bez kontekstu projektu.
pub fn transpile_program_with_extra(prog: &Program, direct_blocks: &Vec<String>, extra: &ExtraSignatures) -> TranspileResult {
    let mut local_sigs = collect_signatures(&prog.clone());
    let mut sigs = merge_signatures_with_extra(&local_sigs, &extra);
    let mut local_mut: std::collections::HashMap<String, bool> = compute_mut_params(&prog.clone());
    let mut mut_params: std::collections::HashMap<String, bool> = merge_mut_params_with_extra(&local_mut, &extra);
    let mut method_mut_params: std::collections::HashMap<String, bool> = compute_method_mut_params(&prog.clone(), &extra.method_mut_params.clone());
    let mut info = build_recursion_info(&sigs.clone());
    let mut varity: std::collections::HashMap<String, i64> = build_variant_arity(&sigs.clone());
    let mut ndstructs: std::collections::HashMap<String, bool> = compute_no_default_structs(&sigs.clone(), &info.boxed_fields.clone());
    let mut direct_dict: std::collections::HashMap<String, String> = direct_blocks_to_dict(&direct_blocks);
    let mut gen = new_codegen(&sigs, &info.boxed_fields, &mut_params, &method_mut_params, &varity, &ndstructs, &direct_dict);
    let mut i: i64 = 0;
    let mut n = (prog.body.len() as i64);
    while (i < n) {
        gen.gen_toplevel(&prog.body[i as usize].clone().clone());
        i = (i + 1);
    }
    let mut header: Vec<String> = vec!["// Plik wygenerowany automatycznie przez hackerc (HackerScript -> Rust).".to_string(), "// NIE EDYTUJ RECZNIE - edytuj zrodlo .hcs i uruchom 'virus build' ponownie.".to_string(), "#![allow(non_snake_case, unused_mut, dead_code)]".to_string()];
    if gen.needs_pyo3 {
        header.push("use pyo3::prelude::*;".to_string());
    }
    header.push("".to_string());
    let mut rust_lines: Vec<String> = vec![];
    let mut hi: i64 = 0;
    let mut hn = (header.len() as i64);
    while (hi < hn) {
        rust_lines.push(header[hi as usize].clone().clone());
        hi = (hi + 1);
    }
    let mut oi: i64 = 0;
    let mut on = (gen.output.len() as i64);
    while (oi < on) {
        rust_lines.push(gen.output[oi as usize].clone().clone());
        oi = (oi + 1);
    }
    let mut rust_code: String = "".to_string();
    let mut ri: i64 = 0;
    let mut rn = (rust_lines.len() as i64);
    while (ri < rn) {
        rust_code = format!("{}{}", rust_code, rust_lines[ri as usize].clone());
        if (ri < (rn - 1)) {
            rust_code = format!("{}{}", rust_code, "\n".to_string());
        }
        ri = (ri + 1);
    }
    rust_code = format!("{}{}", rust_code, "\n".to_string());
    return TranspileResult::new((rust_code).to_string(), gen.needs_pyo3);
}

// Transpiluje zrodlo `.hcs` -> `TranspileResult` (kod Rust + info czy
// potrzebne PyO3) - parytet z `transpile_source_full`. `module_name`
// jest dzis przyjmowany dla zgodnosci sygnatury z Pythonem, ale
// (tak samo jak w `codegen.hcs::gen_program`) NIE trafia jeszcze do
// naglowka wygenerowanego pliku - laczenie wielu plikow/nazewnictwo
// modulow to zadanie `project.hcs`, ktore uzywa go osobno przy
// zapisie `src/{flat_name}.rs`.
pub fn transpile_source_full(source: &String, filename: &String, module_name: &String, extra: &ExtraSignatures) -> TranspileResult {
    let mut extraction: DirectExtraction = extract_direct_blocks(&source);
    let mut prog = parse(&extraction.stripped_source);
    return transpile_program_with_extra(&prog, &extraction.blocks, &extra);
}

// Skrot: zwraca tylko kod Rust (bez `needs_pyo3`) - parytet z
// `transpile_source(source, filename)` (Python, domyslny `extra=None`).
pub fn transpile_source(source: &String, filename: &String) -> String {
    let mut result: TranspileResult = transpile_source_full(&source, &filename, &"module".to_string(), &empty_extra_signatures());
    return result.rust_code.clone();
}

// Czyta `src_path`, transpiluje, zapisuje `.rs` do `out_path` - parytet
// z `transpile_file`. Uzywa wbudowanych `read_file`/`write_file`
// (patrz `libs/std/lib/io.hcs`) zamiast `pathlib.Path.read_text`/
// `write_text` - `mkdir(parents=True)` z Pythona NIE MA odpowiednika
// (brak prymitywu tworzenia katalogow w tym bootstrapie, patrz
// "Ograniczenia" ponizej i `project.hcs`) wiec `out_path` musi
// wskazywac na JUZ ISTNIEJACY katalog.
pub fn transpile_file(src_path: &String, out_path: &String, module_name: &String) -> Result<TranspileResult, String> {
    let mut read_result: Result<String, String> = std::fs::read_to_string(&src_path).map_err(|e| e.to_string());
    match read_result {
        Ok(source) => {
            let mut result: TranspileResult = transpile_source_full(&source, &src_path, &module_name, &empty_extra_signatures());
            let mut write_result: Result<(), String> = std::fs::write(&out_path, result.rust_code.clone()).map_err(|e| e.to_string());
            match write_result {
                Ok(_v) => {
                    return Ok((result).clone());
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Err(e) => {
            return Err(e);
        }
    }
}

// Demonstracyjne uzycie - transpiluje maly program zawierajacy
// `__direct__(4)`, sprawdza ze `__direct__(0)` zostal poprawnie
// wyciagniety i ze `needs_pyo3` jest ustawione.
pub fn main() {
    let mut src: String = "fun main() [\n    __direct__(5)\n    log(\"hello from rust\")\n]\n".to_string();
    let mut extra: ExtraSignatures = empty_extra_signatures();
    let mut result: TranspileResult = transpile_source_full(&src, &"<demo>".to_string(), &"module".to_string(), &extra);
    println!("{} {}", "needs_pyo3:".to_string(), result.needs_pyo3);
    println!("{} {}", "dlugosc wygenerowanego Rusta:".to_string(), (result.rust_code.len() as i64));
}

// ## Ograniczenia tej wersji (patrz docs/ROADMAP.md)
// 
// - Brak `Result`/wyjatkow dla bledow PARSERA (patrz uwaga na gorze
// pliku) - `parser.hcs::parse()` sam w sobie best-effort/`log`-uje.
// `transpile_file` zwraca `Result` TYLKO dla bledow I/O
// (`read_file`/`write_file`), nie dla bledow skladni - inny zakres
// niz Pythonowy `TranspileError`, ktory pokrywa oba.
// - `dedent_text` liczy wciecie PO DLUGOSCI wiodacych spacji/tabow,
// nie po identycznosci znak-po-znaku jak prawdziwy
// `textwrap.dedent` (patrz komentarz przy `common_leading_whitespace`)
// - wystarczajace dla `.hcs`, ktory zawsze wciska spacjami
// konsekwentnie, ale technicznie inny algorytm niz Python dla
// mieszanych spacje/taby.
// - `module_name` jest przyjmowany, ale nie trafia jeszcze do
// wygenerowanego naglowka (parytet z dzisiejszym stanem
// `codegen.hcs::gen_program`, patrz tam) - `project.hcs` uzywa go
// TYLKO do nazwania pliku wyjsciowego `src/{module_name}.rs`.
// - NIEPRZETESTOWANE na prawdziwym wejsciu w tym srodowisku bez
// pelnego `rustc`/`cargo` w oryginalnym stage0 - w TEJ sesji
// zweryfikowane strukturalnie (`hackerc check`) i, gdzie to mozliwe,
// przez faktyczne `cargo build` na wygenerowanym kodzie (patrz
// bootstrap/README.md, sekcja o tej sesji).
