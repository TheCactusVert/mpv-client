use super::ffi::mpv_free;
use super::Result;

use std::ffi::{c_char, c_int, c_void, CStr, CString};

pub trait Format: Sized + Default {
    const MPV_FORMAT: i32;
    fn from_ptr(ptr: *const c_void) -> Result<Self>;
    fn to_mpv<F: Fn(*const c_void) -> Result<()>>(self, fun: F) -> Result<()>;
    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self>;
}

impl Format for String {
    const MPV_FORMAT: i32 = 1;

    fn from_ptr(ptr: *const c_void) -> Result<Self> {
        let ptr = ptr as *const *const i8;
        Ok(unsafe { CStr::from_ptr(*ptr) }.to_str()?.to_string())
    }

    fn to_mpv<F: Fn(*const c_void) -> Result<()>>(self, fun: F) -> Result<()> {
        let str = CString::new::<String>(self.into())?;
        fun(&str.as_ptr() as *const *const i8 as *const c_void)
    }

    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self> {
        let mut ptr: *mut c_char = std::ptr::null_mut();
        fun(&mut ptr as *mut _ as *mut c_void)?;
        unsafe {
            let str = CStr::from_ptr(ptr as *mut i8);
            let str = str.to_str().map(|s| s.to_owned());
            mpv_free(ptr as *mut c_void);
            Ok(str?)
        }
    }
}

impl Format for bool {
    const MPV_FORMAT: i32 = 3;

    fn from_ptr(ptr: *const c_void) -> Result<Self> {
        Ok(unsafe { *(ptr as *const c_int) != 0 })
    }

    fn to_mpv<F: Fn(*const c_void) -> Result<()>>(self, fun: F) -> Result<()> {
        let data = self as c_int;
        fun(&data as *const _ as *const c_void)
    }

    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self> {
        let mut data = Self::default() as c_int;
        fun(&mut data as *mut _ as *mut c_void)?;
        Ok(data != 0)
    }
}

impl Format for i64 {
    const MPV_FORMAT: i32 = 4;

    fn from_ptr(ptr: *const c_void) -> Result<Self> {
        Ok(unsafe { *(ptr as *const Self) })
    }

    fn to_mpv<F: Fn(*const c_void) -> Result<()>>(self, fun: F) -> Result<()> {
        fun(&self as *const _ as *const c_void)
    }

    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self> {
        let mut data = Self::default();
        fun(&mut data as *mut _ as *mut c_void)?;
        Ok(data)
    }
}

impl Format for f64 {
    const MPV_FORMAT: i32 = 5;

    fn from_ptr(ptr: *const c_void) -> Result<Self> {
        Ok(unsafe { *(ptr as *const Self) })
    }

    fn to_mpv<F: Fn(*const c_void) -> Result<()>>(self, fun: F) -> Result<()> {
        fun(&self as *const _ as *const c_void)
    }

    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self> {
        let mut data = Self::default();
        fun(&mut data as *mut _ as *mut c_void)?;
        Ok(data)
    }
}
