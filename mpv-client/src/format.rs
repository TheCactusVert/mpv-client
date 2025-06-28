use super::Result;
use super::{mpv_format_MPV_FORMAT_NONE, mpv_free, mpv_free_node_contents, mpv_node, mpv_node__bindgen_ty_1};

use std::ffi::{c_char, c_int, c_void, CStr, CString};

use super::node::{from_mpv_node, to_mpv_node, Node};

pub trait Format: Sized + Default {
    const MPV_FORMAT: u32;
    fn from_ptr(ptr: *const c_void) -> Result<Self>;
    fn to_mpv<F: Fn(*mut c_void) -> Result<()>>(self, fun: F) -> Result<()>;
    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self>;
}

impl Format for String {
    const MPV_FORMAT: u32 = 1;

    fn from_ptr(ptr: *const c_void) -> Result<Self> {
        let ptr = ptr as *const *const c_char;
        Ok(unsafe { CStr::from_ptr(*ptr) }.to_str()?.to_string())
    }

    fn to_mpv<F: Fn(*mut c_void) -> Result<()>>(self, fun: F) -> Result<()> {
        let str = CString::new::<String>(self.into())?;
        fun(&str.as_ptr() as *const *const c_char as *mut c_void)
    }

    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self> {
        let mut ptr: *mut c_char = std::ptr::null_mut();
        fun(&mut ptr as *mut _ as *mut c_void).and_then(|()| unsafe {
            let str = CStr::from_ptr(ptr);
            let str = str.to_str().map(|s| s.to_owned());
            mpv_free(ptr as *mut c_void);
            Ok(str?)
        })
    }
}

impl Format for bool {
    const MPV_FORMAT: u32 = 3;

    fn from_ptr(ptr: *const c_void) -> Result<Self> {
        Ok(unsafe { *(ptr as *const c_int) != 0 })
    }

    fn to_mpv<F: Fn(*mut c_void) -> Result<()>>(self, fun: F) -> Result<()> {
        let data = self as c_int;
        fun(&data as *const _ as *mut c_void)
    }

    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self> {
        let mut data = Self::default() as c_int;
        fun(&mut data as *mut _ as *mut c_void).map(|()| data != 0)
    }
}

impl Format for i64 {
    const MPV_FORMAT: u32 = 4;

    fn from_ptr(ptr: *const c_void) -> Result<Self> {
        Ok(unsafe { *(ptr as *const Self) })
    }

    fn to_mpv<F: Fn(*mut c_void) -> Result<()>>(self, fun: F) -> Result<()> {
        fun(&self as *const _ as *mut c_void)
    }

    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self> {
        let mut data = Self::default();
        fun(&mut data as *mut _ as *mut c_void).map(|()| data)
    }
}

impl Format for f64 {
    const MPV_FORMAT: u32 = 5;

    fn from_ptr(ptr: *const c_void) -> Result<Self> {
        Ok(unsafe { *(ptr as *const Self) })
    }

    fn to_mpv<F: Fn(*mut c_void) -> Result<()>>(self, fun: F) -> Result<()> {
        fun(&self as *const _ as *mut c_void)
    }

    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self> {
        let mut data = Self::default();
        fun(&mut data as *mut _ as *mut c_void).map(|()| data)
    }
}

impl Format for Node {
    const MPV_FORMAT: u32 = 6;

    fn from_ptr(ptr: *const c_void) -> Result<Self> {
        if ptr.is_null() {
            return Ok(Node::None);
        }

        let node = unsafe { &mut *(ptr as *mut mpv_node) };
        let result = from_mpv_node(node);
        unsafe { mpv_free_node_contents(node) };
        Ok(result)
    }

    fn to_mpv<F: Fn(*mut c_void) -> Result<()>>(self, fun: F) -> Result<()> {
        let mpv_node_ptr = to_mpv_node(&self);
        let res = fun(mpv_node_ptr as *mut c_void);
        unsafe { mpv_free_node_contents(mpv_node_ptr) };
        res
    }

    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self> {
        let mut node = mpv_node {
            format: mpv_format_MPV_FORMAT_NONE,
            u: mpv_node__bindgen_ty_1 { int64: 0 },
        };

        fun(&mut node as *mut _ as *mut c_void)?;
        let result = from_mpv_node(&mut node);
        unsafe { mpv_free_node_contents(&mut node) };
        Ok(result)
    }
}
