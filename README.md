# MPV plugins in Rust

Bindings for libmpv client API that allow you to create plugins for MPV in Rust.

## Example

Here is an example for your `Cargo.toml`:

```toml
[package]
name = "mpv-plugin"
version = "0.1.0"
edition = "2021"

[lib]
name = "mpv_plugin"
crate-type = ["cdylib"]

[dependencies]
mpv-client = "0.2.0"
```

And then the code `src/lib.rs`:

```rust
use mpv_client::{Event, Handle, RawHandle};

#[no_mangle]
extern "C" fn mpv_open_cplugin(handle: RawHandle) -> std::os::raw::c_int {
  let mpv_handle = Handle::from_ptr(handle);
  
  println!("Hello world from Rust plugin {}!", mpv_handle.client_name());
  
  loop {
    match mpv_handle.wait_event(-1.) {
      Event::Shutdown => { return 0; },
      event => { println!("Got event: {}", event); },
    }
  }
}
```

You can find more examples in [`C`](https://github.com/mpv-player/mpv-examples/tree/master/cplugins) and [`Rust`](https://github.com/TheCactusVert/mpv-sponsorblock).
