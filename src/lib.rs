mod error;
mod ffi;
mod format;

use error::{Error, Result};
use ffi::*;
use format::Format;

use std::ffi::{c_void, CStr, CString};
use std::fmt;
use std::time::Duration;

pub type RawHandle = *mut mpv_handle;

/// Client context used by the client API. Every client has its own private handle.
pub struct Handle(*mut mpv_handle);

/// Event sent before playback start of a file (before the file is loaded).
pub struct EventStartFile(*mut mpv_event_start_file);

/// Event sent due to `Handle::observe_property` or due to a response to `Handle::get_property_async`.
pub struct EventProperty(*mut mpv_event_property);

/// Event sent if a hook handler was registered with `Handle::hook_add`, and the
/// hook is invoked.
pub struct EventHook(*mut mpv_event_hook);

macro_rules! mpv_result {
    ($f:expr) => {
        unsafe {
            match $f {
                mpv_error::SUCCESS => Ok(()),
                e => Err(Error::new(e)),
            }
        }
    };
}

pub enum Event {
    None,
    Shutdown,
    LogMessage, // TODO mpv_event_log_message
    GetPropertyReply(EventProperty),
    SetPropertyReply,
    CommandReply, // TODO mpv_event_command
    StartFile(EventStartFile),
    EndFile, // TODO mpv_event_end_file
    FileLoaded,
    ClientMessage, // TODO mpv_event_client_message
    VideoReconfig,
    AudioReconfig,
    Seek,
    PlaybackRestart,
    PropertyChange(EventProperty),
    QueueOverflow,
    Hook(EventHook),
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::None => write!(f, "none"),
            Self::Shutdown => write!(f, "shutdown"),
            Self::LogMessage => write!(f, "log message"),
            Self::GetPropertyReply(ref event) => {
                write!(f, "get property reply [{}]", event.get_name())
            }
            Self::SetPropertyReply => write!(f, "set property reply"),
            Self::CommandReply => write!(f, "command reply"),
            Self::StartFile(ref event) => {
                write!(f, "start file [{}]", event.get_playlist_entry_id())
            }
            Self::EndFile => write!(f, "end file"),
            Self::FileLoaded => write!(f, "file loaded"),
            Self::ClientMessage => write!(f, "client message"),
            Self::VideoReconfig => write!(f, "video reconfig"),
            Self::AudioReconfig => write!(f, "audio reconfig"),
            Self::Seek => write!(f, "seek"),
            Self::PlaybackRestart => write!(f, "playback restart"),
            Self::PropertyChange(ref event) => write!(f, "property change [{}]", event.get_name()),
            Self::QueueOverflow => write!(f, "queue overflow"),
            Self::Hook(ref event) => write!(f, "hook [{}]", event.get_name()),
        }
    }
}

impl Handle {
    /// Wrap a raw mpv_handle
    /// The pointer must not be null
    pub fn from_ptr(ptr: RawHandle) -> Self {
        assert!(!ptr.is_null());
        Self(ptr)
    }

    fn upcast_event(event_id: mpv_event_id, data: *mut c_void) -> Event {
        match event_id {
            mpv_event_id::SHUTDOWN => Event::Shutdown,
            mpv_event_id::LOG_MESSAGE => Event::LogMessage,
            mpv_event_id::GET_PROPERTY_REPLY => Event::GetPropertyReply(EventProperty::from_raw(data)),
            mpv_event_id::SET_PROPERTY_REPLY => Event::SetPropertyReply,
            mpv_event_id::COMMAND_REPLY => Event::CommandReply,
            mpv_event_id::START_FILE => Event::StartFile(EventStartFile::from_raw(data)),
            mpv_event_id::END_FILE => Event::EndFile,
            mpv_event_id::FILE_LOADED => Event::FileLoaded,
            mpv_event_id::CLIENT_MESSAGE => Event::ClientMessage,
            mpv_event_id::VIDEO_RECONFIG => Event::VideoReconfig,
            mpv_event_id::AUDIO_RECONFIG => Event::AudioReconfig,
            mpv_event_id::SEEK => Event::Seek,
            mpv_event_id::PLAYBACK_RESTART => Event::PlaybackRestart,
            mpv_event_id::PROPERTY_CHANGE => Event::PropertyChange(EventProperty::from_raw(data)),
            mpv_event_id::QUEUE_OVERFLOW => Event::QueueOverflow,
            mpv_event_id::HOOK => Event::Hook(EventHook::from_raw(data)),
            _ => Event::None,
        }
    }

    /// Wait for the next event, or until the timeout expires, or if another thread
    /// makes a call to `mpv_wakeup()`. Passing 0 as timeout will never wait, and
    /// is suitable for polling.
    ///
    /// The internal event queue has a limited size (per client handle). If you
    /// don't empty the event queue quickly enough with `Handle::wait_event`, it will
    /// overflow and silently discard further events. If this happens, making
    /// asynchronous requests will fail as well (with MPV_ERROR_EVENT_QUEUE_FULL).
    ///
    /// Only one thread is allowed to call this on the same `Handle` at a time.
    /// The API won't complain if more than one thread calls this, but it will cause
    /// race conditions in the client when accessing the shared mpv_event struct.
    /// Note that most other API functions are not restricted by this, and no API
    /// function internally calls `mpv_wait_event()`. Additionally, concurrent calls
    /// to different handles are always safe.
    ///
    /// As long as the timeout is 0, this is safe to be called from mpv render API
    /// threads.
    pub fn wait_event(&self, timeout: f64) -> (u64, Result<Event>) {
        unsafe {
            let event = mpv_wait_event(self.0, timeout);

            if event.is_null() {
                // TODO is it possible ?
                let reply = 0;
                let event = Ok(Event::None);
                (reply, event)
            } else {
                let reply = (*event).reply_userdata;
                let event = match (*event).error {
                    mpv_error::SUCCESS => Ok(Self::upcast_event((*event).event_id, (*event).data)),
                    _ => Err(Error::new((*event).error)),
                };
                (reply, event)
            }
        }
    }

    /// Return the name of this client handle. Every client has its own unique
    /// name, which is mostly used for user interface purposes.
    pub fn client_name(&self) -> &str {
        unsafe { CStr::from_ptr(mpv_client_name(self.0)) }
            .to_str()
            .unwrap_or("unknown")
    }

    /// Send a command to the player. Commands are the same as those used in
    /// input.conf, except that this function takes parameters in a pre-split
    /// form.
    pub fn command(&self, args: &[String]) -> Result<()> {
        let c_args = args
            .iter()
            .map(|s| CString::new::<String>(s.into()).unwrap())
            .collect::<Vec<CString>>();
        let mut raw_args = c_args.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();
        raw_args.push(std::ptr::null::<i8>()); // Adding null at the end
        mpv_result!(mpv_command(self.0, raw_args.as_ptr()))
    }

    /// Display a message on the screen.
    pub fn osd_message(&self, text: String, duration: Duration) -> Result<()> {
        self.command(&["show-text".to_string(), text, duration.as_millis().to_string()])
    }

    pub fn set_property<T: Format>(&self, name: &str, data: T) -> Result<()> {
        let name = CString::new(name)?;
        data.to_mpv(|data| mpv_result!(mpv_set_property(self.0, name.as_ptr(), T::FORMAT, data)))
    }

    /// Read the value of the given property.
    ///
    /// If the format doesn't match with the internal format of the property, access
    /// usually will fail with `MPV_ERROR_PROPERTY_FORMAT`. In some cases, the data
    /// is automatically converted and access succeeds. For example, i64 is always
    /// converted to f64, and access using String usually invokes a string formatter.
    pub fn get_property<T: Format>(&self, name: &str) -> Result<T> {
        let name = CString::new(name)?;
        T::from_mpv(|data| mpv_result!(mpv_get_property(self.0, name.as_ptr(), T::FORMAT, data)))
    }

    pub fn observe_property<T: Format>(&self, reply_userdata: u64, name: &str) -> Result<()> {
        let name = CString::new(name)?;
        mpv_result!(mpv_observe_property(self.0, reply_userdata, name.as_ptr(), T::FORMAT))
    }

    /// Undo `Handle::observe_property`. This will remove all observed properties for
    /// which the given number was passed as reply_userdata to `Handle::observe_property`.
    ///
    /// Safe to be called from mpv render API threads.
    pub fn unobserve_property(&self, registered_reply_userdata: u64) -> Result<()> {
        mpv_result!(mpv_unobserve_property(self.0, registered_reply_userdata))
    }

    pub fn hook_add(&self, reply_userdata: u64, name: &str, priority: i32) -> Result<()> {
        let name = CString::new(name)?;
        mpv_result!(mpv_hook_add(self.0, reply_userdata, name.as_ptr(), priority))
    }

    pub fn hook_continue(&self, id: u64) -> Result<()> {
        mpv_result!(mpv_hook_continue(self.0, id))
    }
}

impl EventStartFile {
    /// Wrap a raw mpv_event_start_file
    /// The pointer must not be null
    fn from_raw(ptr: *mut c_void) -> Self {
        assert!(!ptr.is_null());
        Self(ptr as *mut mpv_event_start_file)
    }

    /// Playlist entry ID of the file being loaded now.
    pub fn get_playlist_entry_id(&self) -> u64 {
        unsafe { (*self.0).playlist_entry_id }
    }
}

impl EventProperty {
    /// Wrap a raw mpv_event_property
    /// The pointer must not be null
    fn from_raw(ptr: *mut c_void) -> Self {
        assert!(!ptr.is_null());
        Self(ptr as *mut mpv_event_property)
    }

    /// Name of the property.
    pub fn get_name(&self) -> &str {
        unsafe { CStr::from_ptr((*self.0).name) }.to_str().unwrap_or("unknown")
    }

    pub fn get_data<T: Format>(&self) -> Option<T> {
        unsafe {
            if (*self.0).format == T::FORMAT {
                T::from_raw((*self.0).data).ok()
            } else {
                None
            }
        }
    }
}

impl EventHook {
    /// Wrap a raw mpv_event_hook.
    /// The pointer must not be null
    fn from_raw(ptr: *mut c_void) -> Self {
        assert!(!ptr.is_null());
        Self(ptr as *mut mpv_event_hook)
    }

    /// The hook name as passed to `Handle::hook_add`.
    pub fn get_name(&self) -> &str {
        unsafe { CStr::from_ptr((*self.0).name) }.to_str().unwrap_or("unknown")
    }

    /// Internal ID that must be passed to `Handle::hook_continue`.
    pub fn get_id(&self) -> u64 {
        unsafe { (*self.0).id }
    }
}
