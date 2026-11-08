//! YYHistory shares YYEarth's Rust / WebGL2 / Direct3D11 platform layer.
#[macro_use]
mod lang;
#[cfg(all(windows, not(all(test, feature = "headless"))))]
#[path = "win/app.rs"]
mod app;
#[cfg(all(windows, not(all(test, feature = "headless"))))]
#[path = "win/crashlog.rs"]
mod crashlog;
mod game;
#[cfg(any(not(windows), all(test, feature = "headless")))]
mod gfx;
#[cfg(all(windows, not(all(test, feature = "headless"))))]
#[path = "win/gfx.rs"]
mod gfx;
mod history;
#[cfg(any(not(windows), all(test, feature = "headless")))]
pub mod platform;
#[cfg(all(windows, not(all(test, feature = "headless"))))]
#[path = "win/platform.rs"]
pub mod platform;
mod scene;
mod speech;
#[cfg(any(not(windows), all(test, feature = "headless")))]
mod web;
#[cfg(all(windows, not(all(test, feature = "headless"))))]
pub fn desktop_main() {
    app::main();
}

#[derive(Clone, Copy, Debug)]
pub enum Ev {
    MouseMove(f32, f32),
    Down(u8, f32, f32),
    Up(u8, f32, f32),
    Wheel(f32),
    Key(u32, bool),
    Char(char),
    Resize(u32, u32),
    Compact(bool),
    Touch(bool),
    Scroll(f32),
}
