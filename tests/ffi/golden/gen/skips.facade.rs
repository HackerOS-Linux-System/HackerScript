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
pub unsafe extern "C" fn hksffi_t_uses_bad(a0: i64) -> i64 {
    let r = uses_bad(hks_ref::<Bad>(a0));
    r
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_unit_ret(a0: i64) {
    unit_ret(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_no_params() -> i64 {
    let r = no_params();
    r
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_float_bool(a0: f64, a1: bool) -> bool {
    let r = float_bool(a0, a1);
    r
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Bad_free(a0: i64) {
    hks_free::<Gen<Bad>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Bad_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Gen<Bad>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Bad_get_v(a0: i64) -> i64 {
    hks_ptr(HksBorrow::<Bad>::hks_borrow(&mut hks_ref::<Gen<Bad>>(a0).v))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Bad_set_v(a0: i64, a1: i64) {
    hks_ref::<Gen<Bad>>(a0).v = HksFrom::hks_from(hks_ref::<Bad>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Bad_get_w(a0: i64) -> i64 {
    hks_ptr(HksBorrow::<Vec<Bad>>::hks_borrow(&mut hks_ref::<Gen<Bad>>(a0).w))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Bad_set_w(a0: i64, a1: i64) {
    hks_ref::<Gen<Bad>>(a0).w = HksFrom::hks_from(hks_ref::<Vec<Bad>>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Bad_new(a0: i64, a1: i64) -> i64 {
    { type T_ = Gen<Bad>; hks_box(T_ { v: HksFrom::hks_from(hks_ref::<Bad>(a0).clone()), w: HksFrom::hks_from(hks_ref::<Vec<Bad>>(a1).clone()) }) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Bad_get(a0: i64) -> i64 {
    let r = hks_ref::<Gen<Bad>>(a0).get();
    hks_box(r)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Bad_make(a0: i64) -> i64 {
    let r = Gen::<Bad>::make(hks_ref::<Bad>(a0).clone());
    hks_box(r)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bad_free(a0: i64) {
    hks_free::<Vec<Bad>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bad_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Vec<Bad>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bad_new() -> i64 {
    hks_box(Vec::<Bad>::new())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bad_len(a0: i64) -> i64 {
    hks_ref::<Vec<Bad>>(a0).len() as i64
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bad_clear(a0: i64) {
    hks_ref::<Vec<Bad>>(a0).clear();
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bad_push(a0: i64, a1: i64) {
    hks_ref::<Vec<Bad>>(a0).push(hks_ref::<Bad>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bad_get(a0: i64, a1: i64) -> i64 {
    match hks_ref::<Vec<Bad>>(a0).get_mut(a1 as usize) { Some(x) => hks_ptr(x), None => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bad_set(a0: i64, a1: i64, a2: i64) {
    if let Some(x) = hks_ref::<Vec<Bad>>(a0).get_mut(a1 as usize) { *x = hks_ref::<Bad>(a2).clone(); }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bad_insert(a0: i64, a1: i64, a2: i64) {
    let v = hks_ref::<Vec<Bad>>(a0); let i = (a1 as usize).min(v.len()); v.insert(i, hks_ref::<Bad>(a2).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bad_pop(a0: i64) -> i64 {
    match hks_ref::<Vec<Bad>>(a0).pop() { Some(x) => hks_box(x), None => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bad_remove(a0: i64, a1: i64) -> i64 {
    let v = hks_ref::<Vec<Bad>>(a0); if (a1 as usize) < v.len() { let x = v.remove(a1 as usize); hks_box(x) } else { 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_GenE_Str_free(a0: i64) {
    hks_free::<GenE<String>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_GenE_Str_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<GenE<String>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_GenE_Str_tag(a0: i64) -> i64 {
    match &*hks_ref::<GenE<String>>(a0) { GenE::Wrap(..) => 0, GenE::Pair2(..) => 1, GenE::None2 => 2 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_GenE_Str_Wrap(a0: *const std::os::raw::c_char) -> i64 {
    { type T_ = GenE<String>; hks_box(T_::Wrap(HksFrom::hks_from(hks_cstr(a0)))) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_GenE_Str_Wrap_get_0(a0: i64) -> *const std::os::raw::c_char {
    match hks_ref::<GenE<String>>(a0) { GenE::Wrap(x) => hks_ret_str((x).clone()), _ => hks_ret_str(String::new()) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_GenE_Str_Pair2(a0: *const std::os::raw::c_char, a1: *const std::os::raw::c_char) -> i64 {
    { type T_ = GenE<String>; hks_box(T_::Pair2(HksFrom::hks_from(hks_cstr(a0)), HksFrom::hks_from(hks_cstr(a1)))) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_GenE_Str_Pair2_get_0(a0: i64) -> *const std::os::raw::c_char {
    match hks_ref::<GenE<String>>(a0) { GenE::Pair2(x, _) => hks_ret_str((x).clone()), _ => hks_ret_str(String::new()) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_GenE_Str_Pair2_get_1(a0: i64) -> *const std::os::raw::c_char {
    match hks_ref::<GenE<String>>(a0) { GenE::Pair2(_, x) => hks_ret_str((x).clone()), _ => hks_ret_str(String::new()) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_GenE_Str_None2() -> i64 {
    { type T_ = GenE<String>; hks_box(T_::None2) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_GenE_Str_is_none(a0: i64) -> bool {
    let r = hks_ref::<GenE<String>>(a0).is_none();
    r
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Int_Str_free(a0: i64) {
    hks_free::<Gen2<i64, String>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Int_Str_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Gen2<i64, String>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Int_Str_get_a(a0: i64) -> i64 {
    (*&mut hks_ref::<Gen2<i64, String>>(a0).a).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Int_Str_set_a(a0: i64, a1: i64) {
    hks_ref::<Gen2<i64, String>>(a0).a = a1;
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Int_Str_get_b(a0: i64) -> *const std::os::raw::c_char {
    hks_ret_str((&mut hks_ref::<Gen2<i64, String>>(a0).b).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Int_Str_set_b(a0: i64, a1: *const std::os::raw::c_char) {
    hks_ref::<Gen2<i64, String>>(a0).b = HksFrom::hks_from(hks_cstr(a1));
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Int_Str_get_c(a0: i64) -> i64 {
    hks_box(HksOwn::<Option<i64>>::hks_own(&hks_ref::<Gen2<i64, String>>(a0).c))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Int_Str_set_c(a0: i64, a1: i64) {
    hks_ref::<Gen2<i64, String>>(a0).c = HksFrom::hks_from(hks_ref::<Option<i64>>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Int_Str_new(a0: i64, a1: *const std::os::raw::c_char, a2: i64) -> i64 {
    { type T_ = Gen2<i64, String>; hks_box(T_ { a: a0, b: HksFrom::hks_from(hks_cstr(a1)), c: HksFrom::hks_from(hks_ref::<Option<i64>>(a2).clone()) }) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Int_Str_swap(a0: i64) -> i64 {
    let r = hks_ref::<Gen2<i64, String>>(a0).swap();
    hks_box(r)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Str_Int_free(a0: i64) {
    hks_free::<Gen2<String, i64>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Str_Int_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Gen2<String, i64>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Str_Int_get_a(a0: i64) -> *const std::os::raw::c_char {
    hks_ret_str((&mut hks_ref::<Gen2<String, i64>>(a0).a).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Str_Int_set_a(a0: i64, a1: *const std::os::raw::c_char) {
    hks_ref::<Gen2<String, i64>>(a0).a = HksFrom::hks_from(hks_cstr(a1));
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Str_Int_get_b(a0: i64) -> i64 {
    (*&mut hks_ref::<Gen2<String, i64>>(a0).b).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Str_Int_set_b(a0: i64, a1: i64) {
    hks_ref::<Gen2<String, i64>>(a0).b = a1;
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Str_Int_get_c(a0: i64) -> i64 {
    hks_box(HksOwn::<Option<String>>::hks_own(&hks_ref::<Gen2<String, i64>>(a0).c))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Str_Int_set_c(a0: i64, a1: i64) {
    hks_ref::<Gen2<String, i64>>(a0).c = HksFrom::hks_from(hks_ref::<Option<String>>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Str_Int_new(a0: *const std::os::raw::c_char, a1: i64, a2: i64) -> i64 {
    { type T_ = Gen2<String, i64>; hks_box(T_ { a: HksFrom::hks_from(hks_cstr(a0)), b: a1, c: HksFrom::hks_from(hks_ref::<Option<String>>(a2).clone()) }) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen2_Str_Int_swap(a0: i64) -> i64 {
    let r = hks_ref::<Gen2<String, i64>>(a0).swap();
    hks_box(r)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Str_free(a0: i64) {
    hks_free::<Option<String>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Str_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Option<String>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Str_none() -> i64 {
    hks_box(None::<String>)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Str_some(a0: *const std::os::raw::c_char) -> i64 {
    hks_box(Some(hks_cstr(a0)))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Str_is_some(a0: i64) -> bool {
    hks_ref::<Option<String>>(a0).is_some()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Str_get(a0: i64) -> *const std::os::raw::c_char {
    match hks_ref::<Option<String>>(a0).as_mut() { Some(x) => hks_ret_str((x).clone()), None => hks_ret_str(String::new()) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Int_free(a0: i64) {
    hks_free::<Option<i64>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Int_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Option<i64>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Int_none() -> i64 {
    hks_box(None::<i64>)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Int_some(a0: i64) -> i64 {
    hks_box(Some(a0))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Int_is_some(a0: i64) -> bool {
    hks_ref::<Option<i64>>(a0).is_some()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Int_get(a0: i64) -> i64 {
    match hks_ref::<Option<i64>>(a0).as_mut() { Some(x) => (*x).clone(), None => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_free(a0: i64) {
    hks_free::<Gen<i64>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Gen<i64>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_get_v(a0: i64) -> i64 {
    (*&mut hks_ref::<Gen<i64>>(a0).v).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_set_v(a0: i64, a1: i64) {
    hks_ref::<Gen<i64>>(a0).v = a1;
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_get_w(a0: i64) -> i64 {
    hks_ptr(HksBorrow::<Vec<i64>>::hks_borrow(&mut hks_ref::<Gen<i64>>(a0).w))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_set_w(a0: i64, a1: i64) {
    hks_ref::<Gen<i64>>(a0).w = HksFrom::hks_from(hks_ref::<Vec<i64>>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_new(a0: i64, a1: i64) -> i64 {
    { type T_ = Gen<i64>; hks_box(T_ { v: a0, w: HksFrom::hks_from(hks_ref::<Vec<i64>>(a1).clone()) }) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_get(a0: i64) -> i64 {
    let r = hks_ref::<Gen<i64>>(a0).get();
    r
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Gen_Int_make(a0: i64) -> i64 {
    let r = Gen::<i64>::make(a0);
    hks_box(r)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Int_free(a0: i64) {
    hks_free::<Vec<i64>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Int_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Vec<i64>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Int_new() -> i64 {
    hks_box(Vec::<i64>::new())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Int_len(a0: i64) -> i64 {
    hks_ref::<Vec<i64>>(a0).len() as i64
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Int_clear(a0: i64) {
    hks_ref::<Vec<i64>>(a0).clear();
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Int_push(a0: i64, a1: i64) {
    hks_ref::<Vec<i64>>(a0).push(a1);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Int_get(a0: i64, a1: i64) -> i64 {
    match hks_ref::<Vec<i64>>(a0).get_mut(a1 as usize) { Some(x) => (*x).clone(), None => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Int_set(a0: i64, a1: i64, a2: i64) {
    if let Some(x) = hks_ref::<Vec<i64>>(a0).get_mut(a1 as usize) { *x = a2; }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Int_insert(a0: i64, a1: i64, a2: i64) {
    let v = hks_ref::<Vec<i64>>(a0); let i = (a1 as usize).min(v.len()); v.insert(i, a2);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Int_pop(a0: i64) -> i64 {
    match hks_ref::<Vec<i64>>(a0).pop() { Some(x) => x, None => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Int_remove(a0: i64, a1: i64) -> i64 {
    let v = hks_ref::<Vec<i64>>(a0); if (a1 as usize) < v.len() { let x = v.remove(a1 as usize); x } else { 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_BadE_free(a0: i64) {
    hks_free::<BadE>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_BadE_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<BadE>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_BadE_tag(a0: i64) -> i64 {
    match &*hks_ref::<BadE>(a0) { BadE::Good(..) => 0, BadE::Evil(..) => 1, BadE::Plain => 2 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_BadE_Good(a0: i64) -> i64 {
    { type T_ = BadE; hks_box(T_::Good(a0)) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_BadE_Good_get_0(a0: i64) -> i64 {
    match hks_ref::<BadE>(a0) { BadE::Good(x) => (*x).clone(), _ => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_BadE_Plain() -> i64 {
    { type T_ = BadE; hks_box(T_::Plain) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_BadE_go(a0: i64) -> i64 {
    let r = hks_block_on(hks_ref::<BadE>(a0).go());
    r
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Half_free(a0: i64) {
    hks_free::<Half>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Half_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Half>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Half_get_ok(a0: i64) -> i64 {
    (*&mut hks_ref::<Half>(a0).ok).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Half_set_ok(a0: i64, a1: i64) {
    hks_ref::<Half>(a0).ok = a1;
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Bad_free(a0: i64) {
    hks_free::<Bad>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Bad_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Bad>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Bad_get_y(a0: i64) -> i64 {
    (*&mut hks_ref::<Bad>(a0).y).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Bad_set_y(a0: i64, a1: i64) {
    hks_ref::<Bad>(a0).y = a1;
}
