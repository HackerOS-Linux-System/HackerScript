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
pub unsafe extern "C" fn hksffi_t_make_cfg() -> i64 {
    let r = make_cfg();
    hks_box(r)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_take_tree(a0: i64) -> i64 {
    let r = take_tree(hks_ref::<Tree>(a0));
    r
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_depth(a0: i64) -> i64 {
    let r = hks_block_on(depth(hks_ref::<Tree>(a0)));
    r
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_ident_Int(a0: i64) -> i64 {
    let r = ident::<i64>(a0);
    r
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_ident_Cfg(a0: i64) -> i64 {
    let r = ident::<Cfg>(hks_ref::<Cfg>(a0).clone());
    hks_box(r)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_pair_swap_Int_Str(a0: i64) -> i64 {
    let r = pair_swap::<i64, String>(hks_ref::<Pair<i64, String>>(a0));
    hks_box(r)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_ch(a0: i64) -> i64 {
    let r = ch(hks_ref::<HksChan<String>>(a0).clone());
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
pub unsafe extern "C" fn hksffi_t_Chan_Str_free(a0: i64) {
    hks_free::<HksChan<String>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Chan_Str_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<HksChan<String>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Chan_Str_new(a0: i64) -> i64 {
    hks_box(<HksChan<String>>::new(a0))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Chan_Str_send(a0: i64, a1: *const std::os::raw::c_char) -> bool {
    hks_block_on(hks_ref::<HksChan<String>>(a0).send(hks_cstr(a1)))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Chan_Str_recv(a0: i64) -> i64 {
    hks_box(hks_block_on(hks_ref::<HksChan<String>>(a0).recv()))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Chan_Str_close(a0: i64) {
    hks_ref::<HksChan<String>>(a0).close();
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Str_Int_free(a0: i64) {
    hks_free::<Pair<String, i64>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Str_Int_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Pair<String, i64>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Str_Int_get_a(a0: i64) -> *const std::os::raw::c_char {
    hks_ret_str((&mut hks_ref::<Pair<String, i64>>(a0).a).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Str_Int_set_a(a0: i64, a1: *const std::os::raw::c_char) {
    hks_ref::<Pair<String, i64>>(a0).a = HksFrom::hks_from(hks_cstr(a1));
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Str_Int_get_b(a0: i64) -> i64 {
    (*&mut hks_ref::<Pair<String, i64>>(a0).b).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Str_Int_set_b(a0: i64, a1: i64) {
    hks_ref::<Pair<String, i64>>(a0).b = a1;
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Str_Int_new(a0: *const std::os::raw::c_char, a1: i64) -> i64 {
    { type T_ = Pair<String, i64>; hks_box(T_ { a: HksFrom::hks_from(hks_cstr(a0)), b: a1 }) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Int_Str_free(a0: i64) {
    hks_free::<Pair<i64, String>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Int_Str_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Pair<i64, String>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Int_Str_get_a(a0: i64) -> i64 {
    (*&mut hks_ref::<Pair<i64, String>>(a0).a).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Int_Str_set_a(a0: i64, a1: i64) {
    hks_ref::<Pair<i64, String>>(a0).a = a1;
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Int_Str_get_b(a0: i64) -> *const std::os::raw::c_char {
    hks_ret_str((&mut hks_ref::<Pair<i64, String>>(a0).b).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Int_Str_set_b(a0: i64, a1: *const std::os::raw::c_char) {
    hks_ref::<Pair<i64, String>>(a0).b = HksFrom::hks_from(hks_cstr(a1));
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Int_Str_new(a0: i64, a1: *const std::os::raw::c_char) -> i64 {
    { type T_ = Pair<i64, String>; hks_box(T_ { a: a0, b: HksFrom::hks_from(hks_cstr(a1)) }) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Int_Cfg_free(a0: i64) {
    hks_free::<Pair<i64, Cfg>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Int_Cfg_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Pair<i64, Cfg>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Int_Cfg_get_a(a0: i64) -> i64 {
    (*&mut hks_ref::<Pair<i64, Cfg>>(a0).a).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Int_Cfg_set_a(a0: i64, a1: i64) {
    hks_ref::<Pair<i64, Cfg>>(a0).a = a1;
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Int_Cfg_get_b(a0: i64) -> i64 {
    hks_ptr(HksBorrow::<Cfg>::hks_borrow(&mut hks_ref::<Pair<i64, Cfg>>(a0).b))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Int_Cfg_set_b(a0: i64, a1: i64) {
    hks_ref::<Pair<i64, Cfg>>(a0).b = HksFrom::hks_from(hks_ref::<Cfg>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Int_Cfg_new(a0: i64, a1: i64) -> i64 {
    { type T_ = Pair<i64, Cfg>; hks_box(T_ { a: a0, b: HksFrom::hks_from(hks_ref::<Cfg>(a1).clone()) }) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Float_Bool_free(a0: i64) {
    hks_free::<Pair<f64, bool>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Float_Bool_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Pair<f64, bool>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Float_Bool_get_a(a0: i64) -> f64 {
    (*&mut hks_ref::<Pair<f64, bool>>(a0).a).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Float_Bool_set_a(a0: i64, a1: f64) {
    hks_ref::<Pair<f64, bool>>(a0).a = a1;
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Float_Bool_get_b(a0: i64) -> bool {
    (*&mut hks_ref::<Pair<f64, bool>>(a0).b).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Float_Bool_set_b(a0: i64, a1: bool) {
    hks_ref::<Pair<f64, bool>>(a0).b = a1;
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Pair_Float_Bool_new(a0: f64, a1: bool) -> i64 {
    { type T_ = Pair<f64, bool>; hks_box(T_ { a: a0, b: a1 }) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Msg_free(a0: i64) {
    hks_free::<Msg>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Msg_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Msg>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Msg_tag(a0: i64) -> i64 {
    match &*hks_ref::<Msg>(a0) { Msg::Text(..) => 0, Msg::Num(..) => 1, Msg::Nest(..) => 2, Msg::Quit => 3 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Msg_Text(a0: *const std::os::raw::c_char) -> i64 {
    { type T_ = Msg; hks_box(T_::Text(HksFrom::hks_from(hks_cstr(a0)))) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Msg_Text_get_0(a0: i64) -> *const std::os::raw::c_char {
    match hks_ref::<Msg>(a0) { Msg::Text(x) => hks_ret_str((x).clone()), _ => hks_ret_str(String::new()) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Msg_Num(a0: i64, a1: f64) -> i64 {
    { type T_ = Msg; hks_box(T_::Num(a0, a1)) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Msg_Num_get_0(a0: i64) -> i64 {
    match hks_ref::<Msg>(a0) { Msg::Num(x, _) => (*x).clone(), _ => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Msg_Num_get_1(a0: i64) -> f64 {
    match hks_ref::<Msg>(a0) { Msg::Num(_, x) => (*x).clone(), _ => 0.0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Msg_Nest(a0: i64, a1: i64) -> i64 {
    { type T_ = Msg; hks_box(T_::Nest(HksFrom::hks_from(hks_ref::<Option<Tree>>(a0).clone()), HksFrom::hks_from(hks_ref::<Vec<Cfg>>(a1).clone()))) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Msg_Nest_get_0(a0: i64) -> i64 {
    match hks_ref::<Msg>(a0) { Msg::Nest(x, _) => hks_box(HksOwn::<Option<Tree>>::hks_own(&*x)), _ => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Msg_Nest_get_1(a0: i64) -> i64 {
    match hks_ref::<Msg>(a0) { Msg::Nest(_, x) => hks_ptr(HksBorrow::<Vec<Cfg>>::hks_borrow(x)), _ => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Msg_Quit() -> i64 {
    { type T_ = Msg; hks_box(T_::Quit) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Msg_describe(a0: i64) -> *const std::os::raw::c_char {
    let r = hks_ref::<Msg>(a0).describe();
    hks_ret_str(r)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Msg_quit() -> i64 {
    let r = Msg::quit();
    hks_box(r)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Cfg_free(a0: i64) {
    hks_free::<Vec<Cfg>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Cfg_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Vec<Cfg>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Cfg_new() -> i64 {
    hks_box(Vec::<Cfg>::new())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Cfg_len(a0: i64) -> i64 {
    hks_ref::<Vec<Cfg>>(a0).len() as i64
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Cfg_clear(a0: i64) {
    hks_ref::<Vec<Cfg>>(a0).clear();
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Cfg_push(a0: i64, a1: i64) {
    hks_ref::<Vec<Cfg>>(a0).push(hks_ref::<Cfg>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Cfg_get(a0: i64, a1: i64) -> i64 {
    match hks_ref::<Vec<Cfg>>(a0).get_mut(a1 as usize) { Some(x) => hks_ptr(x), None => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Cfg_set(a0: i64, a1: i64, a2: i64) {
    if let Some(x) = hks_ref::<Vec<Cfg>>(a0).get_mut(a1 as usize) { *x = hks_ref::<Cfg>(a2).clone(); }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Cfg_insert(a0: i64, a1: i64, a2: i64) {
    let v = hks_ref::<Vec<Cfg>>(a0); let i = (a1 as usize).min(v.len()); v.insert(i, hks_ref::<Cfg>(a2).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Cfg_pop(a0: i64) -> i64 {
    match hks_ref::<Vec<Cfg>>(a0).pop() { Some(x) => hks_box(x), None => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Cfg_remove(a0: i64, a1: i64) -> i64 {
    let v = hks_ref::<Vec<Cfg>>(a0); if (a1 as usize) < v.len() { let x = v.remove(a1 as usize); hks_box(x) } else { 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Tree_free(a0: i64) {
    hks_free::<Option<Tree>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Tree_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Option<Tree>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Tree_none() -> i64 {
    hks_box(None::<Tree>)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Tree_some(a0: i64) -> i64 {
    hks_box(Some(hks_ref::<Tree>(a0).clone()))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Tree_is_some(a0: i64) -> bool {
    hks_ref::<Option<Tree>>(a0).is_some()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_Tree_get(a0: i64) -> i64 {
    match hks_ref::<Option<Tree>>(a0).as_mut() { Some(x) => hks_ptr(x), None => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Tree_free(a0: i64) {
    hks_free::<Tree>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Tree_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Tree>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Tree_get_v(a0: i64) -> i64 {
    (*&mut hks_ref::<Tree>(a0).v).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Tree_set_v(a0: i64, a1: i64) {
    hks_ref::<Tree>(a0).v = a1;
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Tree_get_l(a0: i64) -> i64 {
    hks_box(HksOwn::<Option<Tree>>::hks_own(&hks_ref::<Tree>(a0).l))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Tree_set_l(a0: i64, a1: i64) {
    hks_ref::<Tree>(a0).l = HksFrom::hks_from(hks_ref::<Option<Tree>>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Tree_get_r(a0: i64) -> i64 {
    hks_box(HksOwn::<Option<Tree>>::hks_own(&hks_ref::<Tree>(a0).r))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Tree_set_r(a0: i64, a1: i64) {
    hks_ref::<Tree>(a0).r = HksFrom::hks_from(hks_ref::<Option<Tree>>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Tree_get_kids(a0: i64) -> i64 {
    hks_ptr(HksBorrow::<Vec<Tree>>::hks_borrow(&mut hks_ref::<Tree>(a0).kids))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Tree_set_kids(a0: i64, a1: i64) {
    hks_ref::<Tree>(a0).kids = HksFrom::hks_from(hks_ref::<Vec<Tree>>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Tree_new(a0: i64, a1: i64, a2: i64, a3: i64) -> i64 {
    { type T_ = Tree; hks_box(T_ { v: a0, l: HksFrom::hks_from(hks_ref::<Option<Tree>>(a1).clone()), r: HksFrom::hks_from(hks_ref::<Option<Tree>>(a2).clone()), kids: HksFrom::hks_from(hks_ref::<Vec<Tree>>(a3).clone()) }) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Tree_leaf(a0: i64) -> i64 {
    let r = Tree::leaf(a0);
    hks_box(r)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Tree_size(a0: i64) -> i64 {
    let r = hks_ref::<Tree>(a0).size();
    r
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Tree_add(a0: i64, a1: i64) -> i64 {
    let r = hks_block_on(hks_ref::<Tree>(a0).add(hks_ref::<Tree>(a1)));
    hks_box(r)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Tree_free(a0: i64) {
    hks_free::<Vec<Tree>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Tree_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Vec<Tree>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Tree_new() -> i64 {
    hks_box(Vec::<Tree>::new())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Tree_len(a0: i64) -> i64 {
    hks_ref::<Vec<Tree>>(a0).len() as i64
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Tree_clear(a0: i64) {
    hks_ref::<Vec<Tree>>(a0).clear();
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Tree_push(a0: i64, a1: i64) {
    hks_ref::<Vec<Tree>>(a0).push(hks_ref::<Tree>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Tree_get(a0: i64, a1: i64) -> i64 {
    match hks_ref::<Vec<Tree>>(a0).get_mut(a1 as usize) { Some(x) => hks_ptr(x), None => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Tree_set(a0: i64, a1: i64, a2: i64) {
    if let Some(x) = hks_ref::<Vec<Tree>>(a0).get_mut(a1 as usize) { *x = hks_ref::<Tree>(a2).clone(); }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Tree_insert(a0: i64, a1: i64, a2: i64) {
    let v = hks_ref::<Vec<Tree>>(a0); let i = (a1 as usize).min(v.len()); v.insert(i, hks_ref::<Tree>(a2).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Tree_pop(a0: i64) -> i64 {
    match hks_ref::<Vec<Tree>>(a0).pop() { Some(x) => hks_box(x), None => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Tree_remove(a0: i64, a1: i64) -> i64 {
    let v = hks_ref::<Vec<Tree>>(a0); if (a1 as usize) < v.len() { let x = v.remove(a1 as usize); hks_box(x) } else { 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_free(a0: i64) {
    hks_free::<Cfg>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Cfg>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_get_m(a0: i64) -> i64 {
    hks_ptr(HksBorrow::<std::collections::HashMap<String, Pair<i64, String>>>::hks_borrow(&mut hks_ref::<Cfg>(a0).m))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_set_m(a0: i64, a1: i64) {
    hks_ref::<Cfg>(a0).m = HksFrom::hks_from(hks_ref::<std::collections::HashMap<String, Pair<i64, String>>>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_get_c(a0: i64) -> i64 {
    hks_ptr(HksBorrow::<HksChan<String>>::hks_borrow(&mut hks_ref::<Cfg>(a0).c))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_set_c(a0: i64, a1: i64) {
    hks_ref::<Cfg>(a0).c = HksFrom::hks_from(hks_ref::<HksChan<String>>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_get_r(a0: i64) -> i64 {
    hks_ptr(HksBorrow::<Result<Vec<i64>, String>>::hks_borrow(&mut hks_ref::<Cfg>(a0).r))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_set_r(a0: i64, a1: i64) {
    hks_ref::<Cfg>(a0).r = HksFrom::hks_from(hks_ref::<Result<Vec<i64>, String>>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_get_o(a0: i64) -> i64 {
    hks_box(HksOwn::<Option<Vec<String>>>::hks_own(&hks_ref::<Cfg>(a0).o))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_set_o(a0: i64, a1: i64) {
    hks_ref::<Cfg>(a0).o = HksFrom::hks_from(hks_ref::<Option<Vec<String>>>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_get_s(a0: i64) -> i64 {
    hks_ptr(HksBorrow::<std::collections::HashSet<i64>>::hks_borrow(&mut hks_ref::<Cfg>(a0).s))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_set_s(a0: i64, a1: i64) {
    hks_ref::<Cfg>(a0).s = HksFrom::hks_from(hks_ref::<std::collections::HashSet<i64>>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_get_sb(a0: i64) -> i64 {
    hks_ptr(HksBorrow::<std::collections::HashSet<bool>>::hks_borrow(&mut hks_ref::<Cfg>(a0).sb))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_set_sb(a0: i64, a1: i64) {
    hks_ref::<Cfg>(a0).sb = HksFrom::hks_from(hks_ref::<std::collections::HashSet<bool>>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_get_f(a0: i64) -> f64 {
    (*&mut hks_ref::<Cfg>(a0).f).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_set_f(a0: i64, a1: f64) {
    hks_ref::<Cfg>(a0).f = a1;
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_get_ok(a0: i64) -> bool {
    (*&mut hks_ref::<Cfg>(a0).ok).clone()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_set_ok(a0: i64, a1: bool) {
    hks_ref::<Cfg>(a0).ok = a1;
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Cfg_new(a0: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64, a6: f64, a7: bool) -> i64 {
    { type T_ = Cfg; hks_box(T_ { m: HksFrom::hks_from(hks_ref::<std::collections::HashMap<String, Pair<i64, String>>>(a0).clone()), c: HksFrom::hks_from(hks_ref::<HksChan<String>>(a1).clone()), r: HksFrom::hks_from(hks_ref::<Result<Vec<i64>, String>>(a2).clone()), o: HksFrom::hks_from(hks_ref::<Option<Vec<String>>>(a3).clone()), s: HksFrom::hks_from(hks_ref::<std::collections::HashSet<i64>>(a4).clone()), sb: HksFrom::hks_from(hks_ref::<std::collections::HashSet<bool>>(a5).clone()), f: a6, ok: a7 }) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Bool_free(a0: i64) {
    hks_free::<std::collections::HashSet<bool>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Bool_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<std::collections::HashSet<bool>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Bool_new() -> i64 {
    hks_box(<std::collections::HashSet<bool>>::new())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Bool_len(a0: i64) -> i64 {
    hks_ref::<std::collections::HashSet<bool>>(a0).len() as i64
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Bool_clear(a0: i64) {
    hks_ref::<std::collections::HashSet<bool>>(a0).clear();
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Bool_insert(a0: i64, a1: bool) -> bool {
    hks_ref::<std::collections::HashSet<bool>>(a0).insert(a1)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Bool_contains(a0: i64, a1: bool) -> bool {
    hks_ref::<std::collections::HashSet<bool>>(a0).contains(&a1)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Bool_remove(a0: i64, a1: bool) -> bool {
    hks_ref::<std::collections::HashSet<bool>>(a0).remove(&a1)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Bool_items(a0: i64) -> i64 {
    hks_box(hks_ref::<std::collections::HashSet<bool>>(a0).iter().cloned().collect::<Vec<bool>>())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bool_free(a0: i64) {
    hks_free::<Vec<bool>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bool_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Vec<bool>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bool_new() -> i64 {
    hks_box(Vec::<bool>::new())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bool_len(a0: i64) -> i64 {
    hks_ref::<Vec<bool>>(a0).len() as i64
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bool_clear(a0: i64) {
    hks_ref::<Vec<bool>>(a0).clear();
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bool_push(a0: i64, a1: bool) {
    hks_ref::<Vec<bool>>(a0).push(a1);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bool_get(a0: i64, a1: i64) -> bool {
    match hks_ref::<Vec<bool>>(a0).get_mut(a1 as usize) { Some(x) => (*x).clone(), None => false }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bool_set(a0: i64, a1: i64, a2: bool) {
    if let Some(x) = hks_ref::<Vec<bool>>(a0).get_mut(a1 as usize) { *x = a2; }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bool_insert(a0: i64, a1: i64, a2: bool) {
    let v = hks_ref::<Vec<bool>>(a0); let i = (a1 as usize).min(v.len()); v.insert(i, a2);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bool_pop(a0: i64) -> bool {
    match hks_ref::<Vec<bool>>(a0).pop() { Some(x) => x, None => false }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Bool_remove(a0: i64, a1: i64) -> bool {
    let v = hks_ref::<Vec<bool>>(a0); if (a1 as usize) < v.len() { let x = v.remove(a1 as usize); x } else { false }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Int_free(a0: i64) {
    hks_free::<std::collections::HashSet<i64>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Int_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<std::collections::HashSet<i64>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Int_new() -> i64 {
    hks_box(<std::collections::HashSet<i64>>::new())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Int_len(a0: i64) -> i64 {
    hks_ref::<std::collections::HashSet<i64>>(a0).len() as i64
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Int_clear(a0: i64) {
    hks_ref::<std::collections::HashSet<i64>>(a0).clear();
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Int_insert(a0: i64, a1: i64) -> bool {
    hks_ref::<std::collections::HashSet<i64>>(a0).insert(a1)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Int_contains(a0: i64, a1: i64) -> bool {
    hks_ref::<std::collections::HashSet<i64>>(a0).contains(&a1)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Int_remove(a0: i64, a1: i64) -> bool {
    hks_ref::<std::collections::HashSet<i64>>(a0).remove(&a1)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Set_Int_items(a0: i64) -> i64 {
    hks_box(hks_ref::<std::collections::HashSet<i64>>(a0).iter().cloned().collect::<Vec<i64>>())
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
pub unsafe extern "C" fn hksffi_t_Option_List_Str_free(a0: i64) {
    hks_free::<Option<Vec<String>>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_List_Str_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Option<Vec<String>>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_List_Str_none() -> i64 {
    hks_box(None::<Vec<String>>)
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_List_Str_some(a0: i64) -> i64 {
    hks_box(Some(hks_ref::<Vec<String>>(a0).clone()))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_List_Str_is_some(a0: i64) -> bool {
    hks_ref::<Option<Vec<String>>>(a0).is_some()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Option_List_Str_get(a0: i64) -> i64 {
    match hks_ref::<Option<Vec<String>>>(a0).as_mut() { Some(x) => hks_ptr(x), None => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Str_free(a0: i64) {
    hks_free::<Vec<String>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Str_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Vec<String>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Str_new() -> i64 {
    hks_box(Vec::<String>::new())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Str_len(a0: i64) -> i64 {
    hks_ref::<Vec<String>>(a0).len() as i64
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Str_clear(a0: i64) {
    hks_ref::<Vec<String>>(a0).clear();
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Str_push(a0: i64, a1: *const std::os::raw::c_char) {
    hks_ref::<Vec<String>>(a0).push(hks_cstr(a1));
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Str_get(a0: i64, a1: i64) -> *const std::os::raw::c_char {
    match hks_ref::<Vec<String>>(a0).get_mut(a1 as usize) { Some(x) => hks_ret_str((x).clone()), None => hks_ret_str(String::new()) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Str_set(a0: i64, a1: i64, a2: *const std::os::raw::c_char) {
    if let Some(x) = hks_ref::<Vec<String>>(a0).get_mut(a1 as usize) { *x = hks_cstr(a2); }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Str_insert(a0: i64, a1: i64, a2: *const std::os::raw::c_char) {
    let v = hks_ref::<Vec<String>>(a0); let i = (a1 as usize).min(v.len()); v.insert(i, hks_cstr(a2));
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Str_pop(a0: i64) -> *const std::os::raw::c_char {
    match hks_ref::<Vec<String>>(a0).pop() { Some(x) => hks_ret_str(x), None => hks_ret_str(String::new()) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Str_remove(a0: i64, a1: i64) -> *const std::os::raw::c_char {
    let v = hks_ref::<Vec<String>>(a0); if (a1 as usize) < v.len() { let x = v.remove(a1 as usize); hks_ret_str(x) } else { hks_ret_str(String::new()) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Result_List_Int_Str_free(a0: i64) {
    hks_free::<Result<Vec<i64>, String>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Result_List_Int_Str_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Result<Vec<i64>, String>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Result_List_Int_Str_ok(a0: i64) -> i64 {
    hks_box(Ok::<Vec<i64>, String>(hks_ref::<Vec<i64>>(a0).clone()))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Result_List_Int_Str_err(a0: *const std::os::raw::c_char) -> i64 {
    hks_box(Err::<Vec<i64>, String>(hks_cstr(a0)))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Result_List_Int_Str_is_ok(a0: i64) -> bool {
    hks_ref::<Result<Vec<i64>, String>>(a0).is_ok()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Result_List_Int_Str_ok_value(a0: i64) -> i64 {
    match hks_ref::<Result<Vec<i64>, String>>(a0) { Ok(x) => hks_ptr(x), Err(_) => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Result_List_Int_Str_err_value(a0: i64) -> *const std::os::raw::c_char {
    match hks_ref::<Result<Vec<i64>, String>>(a0) { Err(x) => hks_ret_str((x).clone()), Ok(_) => hks_ret_str(String::new()) }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Dict_Str_Pair_Int_Str_free(a0: i64) {
    hks_free::<std::collections::HashMap<String, Pair<i64, String>>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Dict_Str_Pair_Int_Str_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<std::collections::HashMap<String, Pair<i64, String>>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Dict_Str_Pair_Int_Str_new() -> i64 {
    hks_box(<std::collections::HashMap<String, Pair<i64, String>>>::new())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Dict_Str_Pair_Int_Str_len(a0: i64) -> i64 {
    hks_ref::<std::collections::HashMap<String, Pair<i64, String>>>(a0).len() as i64
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Dict_Str_Pair_Int_Str_clear(a0: i64) {
    hks_ref::<std::collections::HashMap<String, Pair<i64, String>>>(a0).clear();
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Dict_Str_Pair_Int_Str_insert(a0: i64, a1: *const std::os::raw::c_char, a2: i64) {
    hks_ref::<std::collections::HashMap<String, Pair<i64, String>>>(a0).insert(hks_cstr(a1), hks_ref::<Pair<i64, String>>(a2).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Dict_Str_Pair_Int_Str_contains(a0: i64, a1: *const std::os::raw::c_char) -> bool {
    hks_ref::<std::collections::HashMap<String, Pair<i64, String>>>(a0).contains_key(&hks_cstr(a1))
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Dict_Str_Pair_Int_Str_remove(a0: i64, a1: *const std::os::raw::c_char) -> bool {
    hks_ref::<std::collections::HashMap<String, Pair<i64, String>>>(a0).remove(&hks_cstr(a1)).is_some()
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Dict_Str_Pair_Int_Str_get(a0: i64, a1: *const std::os::raw::c_char) -> i64 {
    match hks_ref::<std::collections::HashMap<String, Pair<i64, String>>>(a0).get_mut(&hks_cstr(a1)) { Some(x) => hks_ptr(x), None => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Dict_Str_Pair_Int_Str_keys(a0: i64) -> i64 {
    hks_box(hks_ref::<std::collections::HashMap<String, Pair<i64, String>>>(a0).keys().cloned().collect::<Vec<String>>())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_Dict_Str_Pair_Int_Str_values(a0: i64) -> i64 {
    hks_box(hks_ref::<std::collections::HashMap<String, Pair<i64, String>>>(a0).values().cloned().collect::<Vec<Pair<i64, String>>>())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Pair_Int_Str_free(a0: i64) {
    hks_free::<Vec<Pair<i64, String>>>(a0);
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Pair_Int_Str_clone(a0: i64) -> i64 {
    hks_box(hks_ref::<Vec<Pair<i64, String>>>(a0).clone())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Pair_Int_Str_new() -> i64 {
    hks_box(Vec::<Pair<i64, String>>::new())
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Pair_Int_Str_len(a0: i64) -> i64 {
    hks_ref::<Vec<Pair<i64, String>>>(a0).len() as i64
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Pair_Int_Str_clear(a0: i64) {
    hks_ref::<Vec<Pair<i64, String>>>(a0).clear();
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Pair_Int_Str_push(a0: i64, a1: i64) {
    hks_ref::<Vec<Pair<i64, String>>>(a0).push(hks_ref::<Pair<i64, String>>(a1).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Pair_Int_Str_get(a0: i64, a1: i64) -> i64 {
    match hks_ref::<Vec<Pair<i64, String>>>(a0).get_mut(a1 as usize) { Some(x) => hks_ptr(x), None => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Pair_Int_Str_set(a0: i64, a1: i64, a2: i64) {
    if let Some(x) = hks_ref::<Vec<Pair<i64, String>>>(a0).get_mut(a1 as usize) { *x = hks_ref::<Pair<i64, String>>(a2).clone(); }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Pair_Int_Str_insert(a0: i64, a1: i64, a2: i64) {
    let v = hks_ref::<Vec<Pair<i64, String>>>(a0); let i = (a1 as usize).min(v.len()); v.insert(i, hks_ref::<Pair<i64, String>>(a2).clone());
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Pair_Int_Str_pop(a0: i64) -> i64 {
    match hks_ref::<Vec<Pair<i64, String>>>(a0).pop() { Some(x) => hks_box(x), None => 0 }
}
#[no_mangle]
pub unsafe extern "C" fn hksffi_t_List_Pair_Int_Str_remove(a0: i64, a1: i64) -> i64 {
    let v = hks_ref::<Vec<Pair<i64, String>>>(a0); if (a1 as usize) < v.len() { let x = v.remove(a1 as usize); hks_box(x) } else { 0 }
}
