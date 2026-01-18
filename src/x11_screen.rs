use std::ffi::c_int;
use std::ptr;

#[repr(C)]
struct Display {
    _data: [u8; 0],
}

#[repr(C)]
struct Screen {
    _data: [u8; 0],
}

#[link(name = "X11")]
extern "C" {
    fn XOpenDisplay(display_name: *const i8) -> *mut Display;
    fn XCloseDisplay(display: *mut Display) -> c_int;
    fn XDefaultScreen(display: *mut Display) -> c_int;
    fn XDisplayWidth(display: *mut Display, screen_number: c_int) -> c_int;
    fn XDisplayHeight(display: *mut Display, screen_number: c_int) -> c_int;
}

#[derive(Copy, Clone, Default)]
pub struct DisplaySize {
    pub width: i32,
    pub height: i32,
}

/// Returns the screen dimensions from X11
#[must_use]
pub fn get_axes_range() -> DisplaySize {
    unsafe {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            eprintln!("extest: failed to open X11 display, using default 1920x1080");
            return DisplaySize {
                width: 1920,
                height: 1080,
            };
        }

        let screen = XDefaultScreen(display);
        let width = XDisplayWidth(display, screen);
        let height = XDisplayHeight(display, screen);

        XCloseDisplay(display);

        println!("extest: detected X11 display with width {width} and height {height}");

        DisplaySize { width, height }
    }
}
