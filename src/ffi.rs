use std::ffi::{c_char, c_double, c_int, c_longlong, c_ulonglong, c_void};

#[repr(i32)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum mpv_error {
    SUCCESS = 0,
    EVENT_QUEUE_FULL = -1,
    NOMEM = -2,
    UNINITIALIZED = -3,
    INVALID_PARAMETER = -4,
    OPTION_NOT_FOUND = -5,
    OPTION_FORMAT = -6,
    OPTION_ERROR = -7,
    PROPERTY_NOT_FOUND = -8,
    PROPERTY_FORMAT = -9,
    PROPERTY_UNAVAILABLE = -10,
    PROPERTY_ERROR = -11,
    COMMAND = -12,
    LOADING_FAILED = -13,
    AO_INIT_FAILED = -14,
    VO_INIT_FAILED = -15,
    NOTHING_TO_PLAY = -16,
    UNKNOWN_FORMAT = -17,
    UNSUPPORTED = -18,
    NOT_IMPLEMENTED = -19,
    GENERIC = -20,
}

#[repr(i32)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, PartialEq)]
pub enum mpv_event_id {
    NONE = 0,
    SHUTDOWN = 1,
    LOG_MESSAGE = 2,
    GET_PROPERTY_REPLY = 3,
    SET_PROPERTY_REPLY = 4,
    COMMAND_REPLY = 5,
    START_FILE = 6,
    END_FILE = 7,
    FILE_LOADED = 8,
    IDLE = 11, // Deprecated
    TICK = 14, // Deprecated
    CLIENT_MESSAGE = 16,
    VIDEO_RECONFIG = 17,
    AUDIO_RECONFIG = 18,
    SEEK = 20,
    PLAYBACK_RESTART = 21,
    PROPERTY_CHANGE = 22,
    QUEUE_OVERFLOW = 24,
    HOOK = 25,
}

#[repr(i32)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, PartialEq)]
pub enum mpv_log_level {
    MPV_LOG_LEVEL_NONE = 0,
    MPV_LOG_LEVEL_FATAL = 10,
    MPV_LOG_LEVEL_ERROR = 20,
    MPV_LOG_LEVEL_WARN = 30,
    MPV_LOG_LEVEL_INFO = 40,
    MPV_LOG_LEVEL_V = 50,
    MPV_LOG_LEVEL_DEBUG = 60,
    MPV_LOG_LEVEL_TRACE = 70,
}

#[repr(i32)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, PartialEq)]
pub enum mpv_end_file_reason {
    MPV_END_FILE_REASON_EOF = 0,
    MPV_END_FILE_REASON_STOP = 2,
    MPV_END_FILE_REASON_QUIT = 3,
    MPV_END_FILE_REASON_ERROR = 4,
    MPV_END_FILE_REASON_REDIRECT = 5,
}

/// Raw client context.
#[allow(non_camel_case_types)]
pub type mpv_handle = c_void;

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mpv_event_property {
    pub name: *const c_char,
    pub format: i32,
    pub data: *mut c_void,
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mpv_event_log_message {
    pub prefix: *const c_char,
    pub level: *const c_char,
    pub text: *const c_char,
    pub log_level: mpv_log_level,
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mpv_event_start_file {
    pub playlist_entry_id: c_ulonglong,
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mpv_event_end_file {
    pub reason: mpv_end_file_reason,
    pub error: c_int,
    pub playlist_entry_id: c_ulonglong,
    pub playlist_insert_id: c_ulonglong,
    pub playlist_insert_num_entries: c_int,
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mpv_event_client_message {
    pub num_args: c_int,
    pub args: *const *const c_char,
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mpv_event_hook {
    pub name: *const c_char,
    pub id: c_ulonglong,
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mpv_event {
    pub event_id: mpv_event_id,
    pub error: mpv_error,
    pub reply_userdata: c_ulonglong,
    pub data: *mut c_void,
}

extern "C" {
    pub fn mpv_error_string(error: mpv_error) -> *const c_char;
    pub fn mpv_free(data: *mut c_void);
    pub fn mpv_client_name(ctx: *mut mpv_handle) -> *const c_char;
    pub fn mpv_client_id(ctx: *mut mpv_handle) -> c_longlong;
    pub fn mpv_create() -> *mut mpv_handle;
    pub fn mpv_initialize(ctx: *mut mpv_handle) -> mpv_error;
    pub fn mpv_destroy(ctx: *mut mpv_handle);
    //pub fn mpv_terminate_destroy(ctx: *mut mpv_handle);
    pub fn mpv_create_client(ctx: *mut mpv_handle, name: *const c_char) -> *mut mpv_handle;
    pub fn mpv_create_weak_client(ctx: *mut mpv_handle, name: *const c_char) -> *mut mpv_handle;
    //pub fn mpv_load_config_file(ctx: *mut mpv_handle, filename: *const c_char) -> mpv_error;
    pub fn mpv_command(ctx: *mut mpv_handle, args: *const *const c_char) -> mpv_error;
    pub fn mpv_command_async(
        ctx: *mut mpv_handle,
        reply_userdata: c_ulonglong,
        args: *const *const c_char,
    ) -> mpv_error;
    pub fn mpv_set_property(ctx: *mut mpv_handle, name: *const c_char, format: c_int, data: *const c_void)
        -> mpv_error;
    pub fn mpv_get_property(ctx: *mut mpv_handle, name: *const c_char, format: c_int, data: *mut c_void) -> mpv_error;
    pub fn mpv_observe_property(
        mpv: *mut mpv_handle,
        reply_userdata: c_ulonglong,
        name: *const c_char,
        format: c_int,
    ) -> mpv_error;
    pub fn mpv_unobserve_property(mpv: *mut mpv_handle, registered_reply_userdata: c_ulonglong) -> mpv_error;
    pub fn mpv_event_name(event: mpv_event_id) -> *const c_char;
    pub fn mpv_wait_event(ctx: *mut mpv_handle, timeout: c_double) -> *mut mpv_event;
    pub fn mpv_hook_add(
        ctx: *mut mpv_handle,
        reply_userdata: c_ulonglong,
        name: *const c_char,
        priority: c_int,
    ) -> mpv_error;
    pub fn mpv_hook_continue(ctx: *mut mpv_handle, id: c_ulonglong) -> mpv_error;
}
