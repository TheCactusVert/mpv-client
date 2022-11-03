mod error;
mod ffi;
mod format;

use error::{Error, Result};
use ffi::*;
use format::Format;

use std::ffi::{c_void, CStr, CString};
use std::fmt;
use std::time::Duration;

/// Raw client context.
pub type RawHandle = *mut mpv_handle;

/// Client context used by the client API. Every client has its own private handle.
pub struct Handle(*mut mpv_handle);

/// An enum representing the available events that can be received by
/// `Handle::wait_event`.
pub enum Event {
    /// Nothing happened. Happens on timeouts or sporadic wakeups.
    None,
    /// Happens when the player quits. The player enters a state where it tries
    /// to disconnect all clients.
    Shutdown,
    /// See `Handle::request_log_messages`.
    LogMessage, // TODO mpv_event_log_message
    /// Reply to a `Handle::get_property_async` request.
    /// See also `Property`.
    GetPropertyReply(Result<()>, u64, Property),
    /// Reply to a `Handle::set_property_async` request.
    /// (Unlike `GetPropertyReply`, `Property` is not used.)
    SetPropertyReply(Result<()>, u64),
    /// Reply to a `Handle::command_async` or mpv_command_node_async() request.
    /// See also `Command`.
    CommandReply(Result<()>, u64), // TODO mpv_event_command
    /// Notification before playback start of a file (before the file is loaded).
    /// See also `StartFile`.
    StartFile(StartFile),
    /// Notification after playback end (after the file was unloaded).
    /// See also `EndFile`.
    EndFile, // TODO mpv_event_end_file
    /// Notification when the file has been loaded (headers were read etc.), and
    /// decoding starts.
    FileLoaded,
    /// Triggered by the script-message input command. The command uses the
    /// first argument of the command as client name (see `Handle::client_name`) to
    /// dispatch the message, and passes along all arguments starting from the
    /// second argument as strings.
    /// See also `ClientMessage`.
    ClientMessage, // TODO mpv_event_client_message
    /// Happens after video changed in some way. This can happen on resolution
    /// changes, pixel format changes, or video filter changes. The event is
    /// sent after the video filters and the VO are reconfigured. Applications
    /// embedding a mpv window should listen to this event in order to resize
    /// the window if needed.
    /// Note that this event can happen sporadically, and you should check
    /// yourself whether the video parameters really changed before doing
    /// something expensive.
    VideoReconfig,
    /// Similar to `VideoReconfig`. This is relatively uninteresting,
    /// because there is no such thing as audio output embedding.
    AudioReconfig,
    /// Happens when a seek was initiated. Playback stops. Usually it will
    /// resume with `PlaybackRestart` as soon as the seek is finished.
    Seek,
    /// There was a discontinuity of some sort (like a seek), and playback
    /// was reinitialized. Usually happens on start of playback and after
    /// seeking. The main purpose is allowing the client to detect when a seek
    /// request is finished.
    PlaybackRestart,
    /// Event sent due to `mpv_observe_property()`.
    /// See also `Property`.
    PropertyChange(u64, Property),
    /// Happens if the internal per-mpv_handle ringbuffer overflows, and at
    /// least 1 event had to be dropped. This can happen if the client doesn't
    /// read the event queue quickly enough with `Handle::wait_event`, or if the
    /// client makes a very large number of asynchronous calls at once.
    ///
    /// Event delivery will continue normally once this event was returned
    /// (this forces the client to empty the queue completely).
    QueueOverflow,
    /// Triggered if a hook handler was registered with `Handle::hook_add`, and the
    /// hook is invoked. If you receive this, you must handle it, and continue
    /// the hook with `Handle::hook_continue`.
    /// See also `Hook`.
    Hook(u64, Hook),
}

/// Data associated with `Event::StartFile`.
pub struct StartFile(*mut mpv_event_start_file);

/// Data associated with `Event::GetPropertyReply` and `Event::PropertyChange`.
pub struct Property(*mut mpv_event_property);

/// Data associated with `Event::Hook`.
pub struct Hook(*mut mpv_event_hook);

macro_rules! mpv_result {
    ($f:expr) => {
        match $f {
            mpv_error::SUCCESS => Ok(()),
            e => Err(Error::new(e)),
        }
    };
}

impl Handle {
    /// Wrap a raw mpv_handle
    /// The pointer must not be null
    pub fn from_ptr(ptr: RawHandle) -> Self {
        assert!(!ptr.is_null());
        Self(ptr)
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
    pub fn wait_event(&self, timeout: f64) -> Event {
        unsafe { Event::from_ptr(mpv_wait_event(self.0, timeout)) }
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
    pub fn command<I, S>(&self, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let args: Vec<CString> = args.into_iter().map(|s| CString::new(s.as_ref()).unwrap()).collect();
        let mut raw_args: Vec<*const i8> = args.iter().map(|s| s.as_ptr()).collect();
        raw_args.push(std::ptr::null()); // Adding null at the end
        unsafe { mpv_result!(mpv_command(self.0, raw_args.as_ptr())) }
    }

    /// Same as `Handle::command`, but run the command asynchronously.
    ///
    /// Commands are executed asynchronously. You will receive a
    /// `CommandReply` event. This event will also have an
    /// error code set if running the command failed. For commands that
    /// return data, the data is put into mpv_event_command.result.
    ///
    /// The only case when you do not receive an event is when the function call
    /// itself fails. This happens only if parsing the command itself (or otherwise
    /// validating it) fails, i.e. the return code of the API call is not 0 or
    /// positive.
    ///
    /// Safe to be called from mpv render API threads.
    pub fn command_async<I, S>(&self, reply_userdata: u64, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let args: Vec<CString> = args.into_iter().map(|s| CString::new(s.as_ref()).unwrap()).collect();
        let mut raw_args: Vec<*const i8> = args.iter().map(|s| s.as_ptr()).collect();
        raw_args.push(std::ptr::null()); // Adding null at the end
        unsafe { mpv_result!(mpv_command_async(self.0, reply_userdata, raw_args.as_ptr())) }
    }

    /// Display a message on the screen.
    /// See `Handle::command`
    pub fn osd_message<S: AsRef<str>>(&self, text: S, duration: Duration) -> Result<()> {
        self.command(&["show-text", text.as_ref(), &duration.as_millis().to_string()])
    }

    /// Same as `Handle::osd_command`, but run the command asynchronously.
    /// See `Handle::command_async`
    pub fn osd_message_async<S: AsRef<str>>(&self, reply_userdata: u64, text: S, duration: Duration) -> Result<()> {
        self.command_async(
            reply_userdata,
            &["show-text", text.as_ref(), &duration.as_millis().to_string()],
        )
    }

    pub fn set_property<T: Format>(&self, name: &str, data: T) -> Result<()> {
        let name = CString::new(name)?;
        data.to_mpv(|data| unsafe { mpv_result!(mpv_set_property(self.0, name.as_ptr(), T::FORMAT, data)) })
    }

    /// Read the value of the given property.
    ///
    /// If the format doesn't match with the internal format of the property, access
    /// usually will fail with `MPV_ERROR_PROPERTY_FORMAT`. In some cases, the data
    /// is automatically converted and access succeeds. For example, i64 is always
    /// converted to f64, and access using String usually invokes a string formatter.
    pub fn get_property<T: Format>(&self, name: &str) -> Result<T> {
        let name = CString::new(name)?;
        T::from_mpv(|data| unsafe { mpv_result!(mpv_get_property(self.0, name.as_ptr(), T::FORMAT, data)) })
    }

    pub fn observe_property<T: Format>(&self, reply_userdata: u64, name: &str) -> Result<()> {
        let name = CString::new(name)?;
        unsafe { mpv_result!(mpv_observe_property(self.0, reply_userdata, name.as_ptr(), T::FORMAT)) }
    }

    /// Undo `Handle::observe_property`. This will remove all observed properties for
    /// which the given number was passed as reply_userdata to `Handle::observe_property`.
    ///
    /// Safe to be called from mpv render API threads.
    pub fn unobserve_property(&self, registered_reply_userdata: u64) -> Result<()> {
        unsafe { mpv_result!(mpv_unobserve_property(self.0, registered_reply_userdata)) }
    }

    pub fn hook_add(&self, reply_userdata: u64, name: &str, priority: i32) -> Result<()> {
        let name = CString::new(name)?;
        unsafe { mpv_result!(mpv_hook_add(self.0, reply_userdata, name.as_ptr(), priority)) }
    }

    pub fn hook_continue(&self, id: u64) -> Result<()> {
        unsafe { mpv_result!(mpv_hook_continue(self.0, id)) }
    }
}

impl Event {
    unsafe fn from_ptr(event: *const mpv_event) -> Event {
        match (*event).event_id {
            mpv_event_id::SHUTDOWN => Event::Shutdown,
            mpv_event_id::LOG_MESSAGE => Event::LogMessage,
            mpv_event_id::GET_PROPERTY_REPLY => Event::GetPropertyReply(
                mpv_result!((*event).error),
                (*event).reply_userdata,
                Property::from_ptr((*event).data),
            ),
            mpv_event_id::SET_PROPERTY_REPLY => {
                Event::SetPropertyReply(mpv_result!((*event).error), (*event).reply_userdata)
            }
            mpv_event_id::COMMAND_REPLY => Event::CommandReply(mpv_result!((*event).error), (*event).reply_userdata),
            mpv_event_id::START_FILE => Event::StartFile(StartFile::from_ptr((*event).data)),
            mpv_event_id::END_FILE => Event::EndFile,
            mpv_event_id::FILE_LOADED => Event::FileLoaded,
            mpv_event_id::CLIENT_MESSAGE => Event::ClientMessage,
            mpv_event_id::VIDEO_RECONFIG => Event::VideoReconfig,
            mpv_event_id::AUDIO_RECONFIG => Event::AudioReconfig,
            mpv_event_id::SEEK => Event::Seek,
            mpv_event_id::PLAYBACK_RESTART => Event::PlaybackRestart,
            mpv_event_id::PROPERTY_CHANGE => {
                Event::PropertyChange((*event).reply_userdata, Property::from_ptr((*event).data))
            }
            mpv_event_id::QUEUE_OVERFLOW => Event::QueueOverflow,
            mpv_event_id::HOOK => Event::Hook((*event).reply_userdata, Hook::from_ptr((*event).data)),
            _ => Event::None,
        }
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let event = match *self {
            Self::Shutdown => mpv_event_id::SHUTDOWN,
            Self::LogMessage => mpv_event_id::LOG_MESSAGE,
            Self::GetPropertyReply(..) => mpv_event_id::GET_PROPERTY_REPLY,
            Self::SetPropertyReply(..) => mpv_event_id::SET_PROPERTY_REPLY,
            Self::CommandReply(..) => mpv_event_id::COMMAND_REPLY,
            Self::StartFile(..) => mpv_event_id::START_FILE,
            Self::EndFile => mpv_event_id::END_FILE,
            Self::FileLoaded => mpv_event_id::FILE_LOADED,
            Self::ClientMessage => mpv_event_id::CLIENT_MESSAGE,
            Self::VideoReconfig => mpv_event_id::VIDEO_RECONFIG,
            Self::AudioReconfig => mpv_event_id::AUDIO_RECONFIG,
            Self::Seek => mpv_event_id::SEEK,
            Self::PlaybackRestart => mpv_event_id::PLAYBACK_RESTART,
            Self::PropertyChange(..) => mpv_event_id::PROPERTY_CHANGE,
            Self::QueueOverflow => mpv_event_id::QUEUE_OVERFLOW,
            Self::Hook(..) => mpv_event_id::HOOK,
            _ => mpv_event_id::NONE,
        };

        let name = unsafe {
            CStr::from_ptr(mpv_event_name(event))
                .to_str()
                .unwrap_or("unknown event")
        };
        write!(f, "{}", name)
    }
}

impl StartFile {
    /// Wrap a raw mpv_event_start_file
    /// The pointer must not be null
    fn from_ptr(ptr: *mut c_void) -> Self {
        assert!(!ptr.is_null());
        Self(ptr as *mut mpv_event_start_file)
    }

    /// Playlist entry ID of the file being loaded now.
    pub fn playlist_entry_id(&self) -> u64 {
        unsafe { (*self.0).playlist_entry_id }
    }
}

impl fmt::Display for StartFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.playlist_entry_id())
    }
}

impl Property {
    /// Wrap a raw mpv_event_property
    /// The pointer must not be null
    fn from_ptr(ptr: *mut c_void) -> Self {
        assert!(!ptr.is_null());
        Self(ptr as *mut mpv_event_property)
    }

    /// Name of the property.
    pub fn name(&self) -> &str {
        unsafe { CStr::from_ptr((*self.0).name) }.to_str().unwrap_or("unknown")
    }

    pub fn data<T: Format>(&self) -> Option<T> {
        unsafe {
            if (*self.0).format == T::FORMAT {
                T::from_ptr((*self.0).data).ok()
            } else {
                None
            }
        }
    }
}

impl fmt::Display for Property {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Hook {
    /// Wrap a raw mpv_event_hook.
    /// The pointer must not be null
    fn from_ptr(ptr: *mut c_void) -> Self {
        assert!(!ptr.is_null());
        Self(ptr as *mut mpv_event_hook)
    }

    /// The hook name as passed to `Handle::hook_add`.
    pub fn name(&self) -> &str {
        unsafe { CStr::from_ptr((*self.0).name) }.to_str().unwrap_or("unknown")
    }

    /// Internal ID that must be passed to `Handle::hook_continue`.
    pub fn id(&self) -> u64 {
        unsafe { (*self.0).id }
    }
}

impl fmt::Display for Hook {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
