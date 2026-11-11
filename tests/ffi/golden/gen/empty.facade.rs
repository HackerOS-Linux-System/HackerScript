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
