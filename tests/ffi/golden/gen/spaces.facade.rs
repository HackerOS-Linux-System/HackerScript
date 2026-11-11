#![allow(dead_code, unused_imports, unused_mut, unused_variables, non_snake_case, unused_unsafe, clippy::all)]
use crate::*;
use std::os::raw::c_char;

thread_local! {
    static HKS_RET: std::cell::RefCell<std::ffi::CString> = std::cell::RefCell::new(std::ffi::CString::default());
}

unsafe fn hks_cstr(p: *const c_char) -> String {
    if p.is_null() { String::new() } else { std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned() }
}

fn hks_ret_str(s: String) -> *const c_char {
    let c = std::ffi::CString::new(s.replace('\0', "")).unwrap_or_default();
    HKS_RET.with(|r| { *r.borrow_mut() = c; r.borrow().as_ptr() })
}

fn hks_box<T>(v: T) -> i64 { std::boxed::Box::into_raw(std::boxed::Box::new(v)) as i64 }
fn hks_ptr<T>(r: &mut T) -> i64 { r as *mut T as i64 }
unsafe fn hks_ref<'a, T>(h: i64) -> &'a mut T { &mut *(h as *mut T) }
unsafe fn hks_free<T>(h: i64) { if h != 0 { drop(std::boxed::Box::from_raw(h as *mut T)); } }

// pola rekurencyjne transpiler owija w Box / Option<Box>; te cechy ukrywaja roznice
trait HksBorrow<T> { fn hks_borrow(&mut self) -> &mut T; }
impl<T> HksBorrow<T> for T { fn hks_borrow(&mut self) -> &mut T { self } }
impl<T> HksBorrow<T> for std::boxed::Box<T> { fn hks_borrow(&mut self) -> &mut T { &mut **self } }
trait HksFrom<T> { fn hks_from(v: T) -> Self; }
impl<T> HksFrom<T> for T { fn hks_from(v: T) -> T { v } }
impl<T> HksFrom<T> for std::boxed::Box<T> { fn hks_from(v: T) -> std::boxed::Box<T> { std::boxed::Box::new(v) } }
impl<T> HksFrom<Option<T>> for Option<std::boxed::Box<T>> { fn hks_from(v: Option<T>) -> Option<std::boxed::Box<T>> { v.map(std::boxed::Box::new) } }
trait HksOwn<T> { fn hks_own(&self) -> T; }
impl<T: Clone> HksOwn<T> for T { fn hks_own(&self) -> T { self.clone() } }
impl<T: Clone> HksOwn<T> for std::boxed::Box<T> { fn hks_own(&self) -> T { (**self).clone() } }
impl<T: Clone> HksOwn<Option<T>> for Option<std::boxed::Box<T>> { fn hks_own(&self) -> Option<T> { self.as_ref().map(|b| (**b).clone()) } }

#[no_mangle]
pub unsafe extern "C" fn hksffi_t_g_Int_Str(a0: i64, a1: *const std::os::raw::c_char) -> i64 {
    let r = g::<i64, String>(a0, hks_cstr(a1));
    r
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Str_Str_free(a0: i64) {
    hks_free::<Gen<String, String>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Str_Str_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Gen<String, String>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Str_Str_get_x(a0: i64) -> *const std::os::raw::c_char {
    hks_ret_str((&mut hks_ref::<Gen<String, String>>(a0).x).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Str_Str_set_x(a0: i64, a1: *const std::os::raw::c_char) {
    hks_ref::<Gen<String, String>>(a0).x = HksFrom::hks_from(hks_cstr(a1));
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Str_Str_get_y(a0: i64) -> *const std::os::raw::c_char {
    hks_ret_str((&mut hks_ref::<Gen<String, String>>(a0).y).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Str_Str_set_y(a0: i64, a1: *const std::os::raw::c_char) {
    hks_ref::<Gen<String, String>>(a0).y = HksFrom::hks_from(hks_cstr(a1));
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Str_Str_new(a0: *const std::os::raw::c_char, a1: *const std::os::raw::c_char) -> i64 {
    { type T_ = Gen<String, String>; hks_box(T_ { x: HksFrom::hks_from(hks_cstr(a0)), y: HksFrom::hks_from(hks_cstr(a1)) }) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_Str_free(a0: i64) {
    hks_free::<Gen<i64, String>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_Str_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Gen<i64, String>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_Str_get_x(a0: i64) -> i64 {
    (*&mut hks_ref::<Gen<i64, String>>(a0).x).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_Str_set_x(a0: i64, a1: i64) {
    hks_ref::<Gen<i64, String>>(a0).x = a1;
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_Str_get_y(a0: i64) -> *const std::os::raw::c_char {
    hks_ret_str((&mut hks_ref::<Gen<i64, String>>(a0).y).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_Str_set_y(a0: i64, a1: *const std::os::raw::c_char) {
    hks_ref::<Gen<i64, String>>(a0).y = HksFrom::hks_from(hks_cstr(a1));
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_Str_new(a0: i64, a1: *const std::os::raw::c_char) -> i64 {
    { type T_ = Gen<i64, String>; hks_box(T_ { x: a0, y: HksFrom::hks_from(hks_cstr(a1)) }) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_E1_free(a0: i64) {
    hks_free::<E1>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_E1_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<E1>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_E1_tag(a0: i64) -> i64 {
    match &*hks_ref::<E1>(a0) { E1:: A(..) => 0, E1:: B => 1 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_E1_ A(a0: i64, a1: *const std::os::raw::c_char) -> i64 {
    { type T_ = E1; hks_box(T_:: A(a0, HksFrom::hks_from(hks_cstr(a1)))) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_E1_ A_get_0(a0: i64) -> i64 {
    match hks_ref::<E1>(a0) { E1:: A(x, _) => (*x).clone(), _ => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_E1_ A_get_1(a0: i64) -> *const std::os::raw::c_char {
    match hks_ref::<E1>(a0) { E1:: A(_, x) => hks_ret_str((x).clone()), _ => hks_ret_str(String::new()) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_E1_ B() -> i64 {
    { type T_ = E1; hks_box(T_:: B) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Sp_free(a0: i64) {
    hks_free::<Sp>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Sp_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Sp>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Sp_get_a(a0: i64) -> i64 {
    (*&mut hks_ref::<Sp>(a0).a).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Sp_set_a(a0: i64, a1: i64) {
    hks_ref::<Sp>(a0).a = a1;
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Sp_get_ b (a0: i64) -> *const std::os::raw::c_char {
    hks_ret_str((&mut hks_ref::<Sp>(a0). b ).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Sp_set_ b (a0: i64, a1: *const std::os::raw::c_char) {
    hks_ref::<Sp>(a0). b  = HksFrom::hks_from(hks_cstr(a1));
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Sp_new(a0: i64, a1: *const std::os::raw::c_char) -> i64 {
    { type T_ = Sp; hks_box(T_ { a: a0,  b : HksFrom::hks_from(hks_cstr(a1)) }) }
}
