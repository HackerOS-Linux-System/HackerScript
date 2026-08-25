#![allow(non_snake_case, unused_mut, dead_code)]

mod _hks_inc_ast_nodes;
mod _hks_inc_parser;
mod _hks_inc_typecheck;
mod _hks_inc_diagnostics;
mod _hks_inc_transpiler;
mod _hks_inc_lexer;
mod _hks_inc_typeinfer;
mod _hks_inc_codegen;

use crate::_hks_inc_ast_nodes::*;
use crate::_hks_inc_diagnostics::*;
use crate::_hks_inc_parser::*;
use crate::_hks_inc_transpiler::*;
use crate::_hks_inc_typecheck::*;

use std::mem;

// Kazdy `_hks_inc_*.hcs` ma WLASNE `main()` (self-test, uruchamiany
// tylko gdy dany plik jest budowany jako oddzielny plik wykonywalny -
// patrz bootstrap/README.md) - `use ...::*` (wyzej) importuje je
// WSZYSTKIE naraz, co dawaloby "ambiguous glob" przy kazdej probie
// uzycia golej nazwy `main`. W ORYGINALNYM wygenerowanym `main.rs`
// (binarce) nie byl to problem, bo `main.rs` definiuje WLASNY `pub fn
// main()` w korzeniu skrzyni, ktory PRZESLANIA importy glob (lokalna
// definicja ma pierwszenstwo w Rust) - to samo robimy tutaj (funkcja
// nigdy nie jest wywolywana jako punkt wejscia crate typu `lib`/
// `cdylib` - istnieje WYLACZNIE po to, zeby przeslonic ambiguity).
pub fn main() {}

// ---- od tego miejsca w dol: 1:1 skopiowane z main.rs wygenerowanego
// przez `hackerc build bootstrap/hackerc-self/playground.hcs` (patrz
// README.md w tym katalogu - jak odtworzyc/zaktualizowac) --------------

pub fn parse_source(source: &String) -> Program {
    let mut extraction = extract_direct_blocks(&source);
    return parse(&extraction.stripped_source);
}

pub fn check_source_inner(source: &String) -> String {
    let mut program = parse_source(&source);
    let mut diags: Vec<Diagnostic> = check_program(&program, &vec![]);
    if (diags.len() as i64) == 0 {
        return "OK (0 warning(ow))".to_string();
    }
    let mut report: String =
        render_many(&source, &"playground.hcs".to_string(), &diags.clone());
    let mut errors: i64 = 0;
    let mut i: i64 = 0;
    let mut n = diags.len() as i64;
    while i < n {
        if diags[i as usize].clone().severity.to_string() == "error".to_string() {
            errors += 1;
        }
        i += 1;
    }
    if errors > 0 {
        return format!(
            "{}\n\n{} blad(ow), {} warning(ow)",
            report,
            errors,
            (diags.len() as i64) - errors
        );
    }
    format!("{}\n\nOK ({} warning(ow))", report, diags.len() as i64)
}

fn check_source(source: &str) -> String {
    // Zabezpieczenie PRZED znanym bugiem w recovery parsera dla
    // niektorych skrajnie zdeformowanych wejsc (petla w obsludze bledu
    // skladni potrafi nie konczyc sie - patrz README.md, sekcja
    // "Znane ograniczenia") - twardy limit dlugosci wejscia jako
    // najprostsza, bezpieczna dla przegladarki ochrona (zamiast
    // ryzykowac zawieszenie karty).
    const MAX_LEN: usize = 200_000;
    if source.len() > MAX_LEN {
        return format!(
            "playground: kod zrodlowy za dlugi (max {MAX_LEN} bajtow, dostano {})",
            source.len()
        );
    }
    check_source_inner(&source.to_string())
}

// ---- koniec czesci skopiowanej z main.rs -----------------------------

// ======================================================================
// Surowy interfejs C ABI dla JS (bez wasm-bindgen) - patrz
// playground/wasm-glue.js dla strony JS tego kontraktu.
//
// Protokol:
//   1. JS wola `wasm_alloc(len)` -> dostaje wskaznik `ptr` do bufora
//      `len` bajtow w pamieci linear WASM (`memory.buffer`).
//   2. JS zapisuje UTF-8 kodu zrodlowego pod `ptr` (przez
//      `Uint8Array` widok na `memory.buffer`).
//   3. JS wola `wasm_check(ptr, len)` -> dostaje POJEDYNCZY `i64`
//      zapakowany jako `(wynik_ptr << 32) | wynik_len` (oba
//      NIEUJEMNE i < 2^32 dla realistycznych rozmiarow raportu -
//      wystarczajace, unika alokowania osobnego bufora "out
//      params" po stronie JS).
//   4. JS odczytuje `wynik_len` bajtow UTF-8 spod `wynik_ptr`,
//      dekoduje przez `TextDecoder`.
//   5. JS wola `wasm_dealloc(wynik_ptr, wynik_len)` (i ewentualnie
//      `wasm_dealloc(ptr, len)` dla wejscia, jesli nie uzywa go juz
//      wiecej) - ZWALNIA pamiec (Rust `String`/`Vec<u8>` inaczej by
//      przeciekala - `mem::forget` ponizej celowo oddaje wlasnosc
//      buforow stronie JS na czas trwania wywolania).
// ======================================================================

/// Alokuje bufor `len` bajtow w pamieci linear WASM i zwraca wskaznik
/// do niego. Wywolywane z JS PRZED zapisem kodu zrodlowego.
#[no_mangle]
pub extern "C" fn wasm_alloc(len: usize) -> *mut u8 {
    let mut buf: Vec<u8> = Vec::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    mem::forget(buf); // wlasnosc przechodzi na JS do `wasm_dealloc`
    ptr
}

/// Zwalnia bufor zaalokowany przez `wasm_alloc` (albo zwrocony przez
/// `wasm_check` jako wynik) - MUSI byc wywolane z DOKLADNIE tym samym
/// `len`, jakiego uzyto przy alokacji (Rust `Vec::from_raw_parts`
/// wymaga zgodnej pojemnosci/dlugosci, zeby poprawnie zwolnic pamiec).
#[no_mangle]
pub extern "C" fn wasm_dealloc(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

/// Sprawdza kod zrodlowy HackerScript zapisany PRZEZ JS pod `ptr`
/// (`len` bajtow, UTF-8) i zwraca zapakowany `(wynik_ptr <<
/// 32) | wynik_len` wskazujacy na UTF-8 raport diagnostyk (JS go
/// dekoduje, potem MUSI zwolnic przez `wasm_dealloc`).
///
/// # Safety (dla wywolujacego z JS)
/// `ptr` musi wskazywac na `len` poprawnie zainicjalizowanych bajtow
/// (zapisanych po `wasm_alloc(len)`) - odpowiedzialnosc JS, tak jak
/// przy kazdym recznym C ABI.
#[no_mangle]
pub extern "C" fn wasm_check(ptr: *const u8, len: usize) -> u64 {
    let source_bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let source = match std::str::from_utf8(source_bytes) {
        Ok(s) => s,
        Err(_) => "playground: kod zrodlowy nie jest poprawnym UTF-8",
    };
    let report = check_source(source);
    let report_bytes = report.into_bytes();
    let out_len = report_bytes.len();
    let out_ptr = wasm_alloc(out_len);
    unsafe {
        std::ptr::copy_nonoverlapping(report_bytes.as_ptr(), out_ptr, out_len);
    }
    ((out_ptr as u64) << 32) | (out_len as u64)
}

/// Wersja hackerc, ktorej logike checkera uzywa ten playground -
/// STALA liczba (bez alokacji) - JS moze ja odczytac wprost.
#[no_mangle]
pub extern "C" fn wasm_hackerc_version_major() -> u32 {
    0
}
#[no_mangle]
pub extern "C" fn wasm_hackerc_version_minor() -> u32 {
    0
}
#[no_mangle]
pub extern "C" fn wasm_hackerc_version_patch() -> u32 {
    1
}
