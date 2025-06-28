use super::{
    mpv_byte_array, mpv_format_MPV_FORMAT_BYTE_ARRAY, mpv_format_MPV_FORMAT_DOUBLE, mpv_format_MPV_FORMAT_FLAG,
    mpv_format_MPV_FORMAT_INT64, mpv_format_MPV_FORMAT_NODE_ARRAY, mpv_format_MPV_FORMAT_NODE_MAP,
    mpv_format_MPV_FORMAT_NONE, mpv_format_MPV_FORMAT_STRING, mpv_node, mpv_node__bindgen_ty_1, mpv_node_list,
};
use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};
use std::ptr;
use std::slice;

#[derive(Debug, Clone)]
pub enum Node {
    None,
    String(String),
    Int(i64),
    Double(f64),
    Bool(bool),
    ByteArray(Vec<u8>),
    Array(Vec<Node>),
    Map(HashMap<String, Node>),
}

impl Default for Node {
    fn default() -> Self {
        Node::None
    }
}

pub fn from_mpv_node(node: &mut mpv_node) -> Node {
    match node.format {
        mpv_format_MPV_FORMAT_STRING => {
            Node::String(unsafe { CStr::from_ptr(node.u.string) }.to_string_lossy().into_owned())
        }
        mpv_format_MPV_FORMAT_INT64 => Node::Int(unsafe { node.u.int64 }),
        mpv_format_MPV_FORMAT_DOUBLE => Node::Double(unsafe { node.u.double_ }),
        mpv_format_MPV_FORMAT_FLAG => Node::Bool(unsafe { node.u.flag } != 0),
        mpv_format_MPV_FORMAT_NODE_ARRAY => {
            let list = unsafe { &*node.u.list };
            let values = unsafe { slice::from_raw_parts_mut(list.values, list.num as usize) };
            Node::Array(values.iter_mut().map(from_mpv_node).collect())
        }
        mpv_format_MPV_FORMAT_NODE_MAP => {
            let list = unsafe { &*node.u.list };
            let values = unsafe { slice::from_raw_parts_mut(list.values, list.num as usize) };
            let keys = unsafe { slice::from_raw_parts(list.keys, list.num as usize) };
            let map = keys
                .iter()
                .zip(values.iter_mut())
                .filter_map(|(&k, v)| {
                    unsafe { k.as_ref() }.map(|key_ptr| {
                        let key = unsafe { CStr::from_ptr(key_ptr) }.to_string_lossy().into_owned();
                        (key, from_mpv_node(v))
                    })
                })
                .collect();
            Node::Map(map)
        }
        mpv_format_MPV_FORMAT_BYTE_ARRAY => {
            let arr: &mpv_byte_array = unsafe { &*node.u.ba };
            let data = unsafe { slice::from_raw_parts(arr.data as *const u8, arr.size) };
            Node::ByteArray(data.to_vec())
        }
        _ => Node::None,
    }
}

pub fn to_mpv_node(node: &Node) -> *mut mpv_node {
    let mut mpv_node = Box::new(mpv_node {
        format: 0,
        u: mpv_node__bindgen_ty_1 { int64: 0 },
    });

    match node {
        Node::None => {
            mpv_node.format = mpv_format_MPV_FORMAT_NONE;
        }
        Node::String(s) => {
            mpv_node.format = mpv_format_MPV_FORMAT_STRING;
            let cstr = CString::new(s.as_str()).expect("CString::new failed");
            mpv_node.u.string = cstr.into_raw();
        }
        Node::Int(i) => {
            mpv_node.format = mpv_format_MPV_FORMAT_INT64;
            mpv_node.u.int64 = *i;
        }
        Node::Double(f) => {
            mpv_node.format = mpv_format_MPV_FORMAT_DOUBLE;
            mpv_node.u.double_ = *f;
        }
        Node::Bool(b) => {
            mpv_node.format = mpv_format_MPV_FORMAT_FLAG;
            mpv_node.u.flag = if *b { 1 } else { 0 };
        }
        Node::Array(arr) => {
            mpv_node.format = mpv_format_MPV_FORMAT_NODE_ARRAY;
            let values: Vec<_> = arr.iter().map(to_mpv_node).collect();
            let list = Box::new(mpv_node_list {
                num: values.len() as i32,
                values: Box::into_raw(values.into_boxed_slice()) as *mut mpv_node,
                keys: std::ptr::null_mut(),
            });
            mpv_node.u.list = Box::into_raw(list);
        }
        Node::Map(map) => {
            mpv_node.format = mpv_format_MPV_FORMAT_NODE_MAP;
            let (keys, values): (Vec<_>, Vec<_>) = map
                .iter()
                .map(|(k, v)| {
                    let ckey = CString::new(k.as_str()).expect("CString::new failed");
                    (ckey.into_raw(), to_mpv_node(v))
                })
                .unzip();

            let list = Box::new(mpv_node_list {
                num: keys.len() as i32,
                keys: Box::into_raw(keys.into_boxed_slice()) as *mut *mut c_char,
                values: Box::into_raw(values.into_boxed_slice()) as *mut mpv_node,
            });
            mpv_node.u.list = Box::into_raw(list);
        }
        Node::ByteArray(vec) => {
            mpv_node.format = mpv_format_MPV_FORMAT_BYTE_ARRAY;
            let ba = Box::new(mpv_byte_array {
                size: vec.len(),
                data: unsafe { libc::malloc(vec.len()) as *mut std::ffi::c_void },
            });
            if ba.data.is_null() {
                panic!("Failed to allocate memory for byte array");
            }
            unsafe {
                ptr::copy_nonoverlapping(vec.as_ptr(), ba.data as *mut u8, vec.len());
            }
            mpv_node.u.ba = Box::into_raw(ba);
        }
    }

    Box::into_raw(mpv_node)
}
