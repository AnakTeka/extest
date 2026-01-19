mod keys;
mod x11_screen;
use x11_screen::get_axes_range;

use evdev::{
    uinput::VirtualDevice, AbsInfo, AbsoluteAxisCode, AttributeSet, BusType, EventType, InputEvent,
    InputId, KeyCode, RelativeAxisCode, UinputAbsSetup,
};
use once_cell::sync::Lazy;
use std::ffi::{c_int, c_uint, c_ulong, c_void, CStr};
use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};

// Track last cursor position for converting absolute to relative motion
static LAST_X: AtomicI32 = AtomicI32::new(-1);
static LAST_Y: AtomicI32 = AtomicI32::new(-1);

// Check if XTest passthrough should be disabled (for Moonlight compatibility)
static NO_XTEST: Lazy<bool> = Lazy::new(|| {
    let enabled = std::env::var("EXTEST_NO_XTEST").is_ok();
    if enabled {
        eprintln!("extest: EXTEST_NO_XTEST mode - mouse motion via uinput only (for Moonlight)");
    }
    enabled
});

// Opaque type
#[repr(C)]
pub struct Display {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

// dlsym constants
const RTLD_NEXT: *mut c_void = -1isize as *mut c_void;

#[link(name = "dl")]
extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const i8) -> *mut c_void;
}

// Function pointer types for real XTest functions
type XTestFakeMotionEventFn = unsafe extern "C" fn(*mut Display, c_int, c_int, c_int, c_ulong) -> c_int;
type XTestFakeRelativeMotionEventFn = unsafe extern "C" fn(*mut Display, c_int, c_int, c_ulong) -> c_int;

// Get real XTest functions via dlsym
static REAL_XTEST_MOTION: Lazy<XTestFakeMotionEventFn> = Lazy::new(|| unsafe {
    let sym = dlsym(RTLD_NEXT, b"XTestFakeMotionEvent\0".as_ptr() as *const i8);
    if sym.is_null() {
        panic!("Failed to find real XTestFakeMotionEvent");
    }
    std::mem::transmute(sym)
});

static REAL_XTEST_RELATIVE_MOTION: Lazy<XTestFakeRelativeMotionEventFn> = Lazy::new(|| unsafe {
    let sym = dlsym(RTLD_NEXT, b"XTestFakeRelativeMotionEvent\0".as_ptr() as *const i8);
    if sym.is_null() {
        panic!("Failed to find real XTestFakeRelativeMotionEvent");
    }
    std::mem::transmute(sym)
});

static DEVICE: Lazy<Mutex<VirtualDevice>> = Lazy::new(|| {
    let size = get_axes_range();
    eprintln!("extest: keyboard/buttons via uinput, mouse motion via both uinput+XTest");
    Mutex::new(
        VirtualDevice::builder()
            .unwrap()
            .name("extest fake device")
            .input_id(InputId::new(BusType::BUS_VIRTUAL, 0xe17e, 0x5700, 1))
            .with_keys(&AttributeSet::from_iter(
                [
                    KeyCode::BTN_LEFT,
                    KeyCode::BTN_RIGHT,
                    KeyCode::BTN_MIDDLE,
                    KeyCode::BTN_EXTRA,
                    KeyCode::BTN_SIDE,
                ]
                .into_iter()
                .chain(keys::all_keys()),
            ))
            .unwrap()
            .with_relative_axes(&AttributeSet::from_iter([
                RelativeAxisCode::REL_X,
                RelativeAxisCode::REL_Y,
                RelativeAxisCode::REL_WHEEL,
            ]))
            .unwrap()
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_X,
                AbsInfo::new(0, 0, size.width, 0, 0, 1),
            ))
            .unwrap()
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_Y,
                AbsInfo::new(0, 0, size.height, 0, 0, 1),
            ))
            .unwrap()
            .build()
            .unwrap(),
    )
});

#[no_mangle]
pub extern "C" fn XTestFakeKeyEvent(
    _: *mut Display,
    keycode: c_uint,
    is_press: bool,
    _: c_ulong,
) -> c_int {
    let mut dev = DEVICE.lock().unwrap();

    // Seems that X11 keycodes are just 8 + linux keycode - https://wiki.archlinux.org/title/Keyboard_input#Identifying_keycodes
    let key = match keycode {
        156 => KeyCode::KEY_TAB, // I have no idea where this comes from
        keycode => KeyCode::new((keycode - 8) as u16),
    };

    #[cfg(debug_assertions)]
    println!("emitting keycode {key:?}");

    dev.emit(&[InputEvent::new_now(
        EventType::KEY.0,
        key.0,
        is_press as i32,
    )])
    .unwrap();
    1
}

#[repr(u8)]
enum MouseButtons {
    LeftClick = 1,
    MiddleClick = 2,
    RightClick = 3,
    ScrollUp = 4,
    ScrollDown = 5,
    Side = 8,
    Extra = 9,
}

impl TryFrom<u32> for MouseButtons {
    type Error = u32;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        use MouseButtons::*;
        match value {
            1 => Ok(LeftClick),
            2 => Ok(MiddleClick),
            3 => Ok(RightClick),
            4 => Ok(ScrollUp),
            5 => Ok(ScrollDown),
            8 => Ok(Side),
            9 => Ok(Extra),
            other => Err(other),
        }
    }
}

#[no_mangle]
pub extern "C" fn XTestFakeButtonEvent(
    _: *mut Display,
    button: c_uint,
    is_press: bool,
    _: c_ulong,
) -> c_int {
    let mut dev = DEVICE.lock().unwrap();
    // values determined via xev
    let key = match button.try_into() {
        Ok(MouseButtons::LeftClick) => KeyCode::BTN_LEFT,
        Ok(MouseButtons::MiddleClick) => KeyCode::BTN_MIDDLE,
        Ok(MouseButtons::RightClick) => KeyCode::BTN_RIGHT,
        Ok(MouseButtons::Side) => KeyCode::BTN_SIDE,
        Ok(MouseButtons::Extra) => KeyCode::BTN_EXTRA,
        Ok(MouseButtons::ScrollUp | MouseButtons::ScrollDown) => {
            // These are sent with is_press true and is_press false like the other buttons,
            // but we only care about is_press because an "unpressed" scroll event doesn't make
            // sense. Why are these considered "buttons" anyway?
            if is_press {
                let value = match button.try_into() {
                    Ok(MouseButtons::ScrollUp) => 1,
                    Ok(MouseButtons::ScrollDown) => -1,
                    _ => unreachable!(),
                };
                dev.emit(&[InputEvent::new_now(
                    EventType::RELATIVE.0,
                    RelativeAxisCode::REL_WHEEL.0,
                    value,
                )])
                .unwrap();
            }
            return 1;
        }
        Err(other) => {
            println!("WARNING: received unknown keycode {other}");
            return 1;
        }
    };

    #[cfg(debug_assertions)]
    println!("emitting mouse button {key:?}");
    dev.emit(&[InputEvent::new_now(
        EventType::KEY.0,
        key.0,
        is_press as i32,
    )])
    .unwrap();
    1
}

// Mouse motion - send to both uinput (for Moonlight) and XTest (for X11 cursor)
#[no_mangle]
pub extern "C" fn XTestFakeRelativeMotionEvent(
    display: *mut Display,
    x: c_int,
    y: c_int,
    delay: c_ulong,
) -> c_int {
    // Send to uinput for apps like Moonlight that capture raw input
    // Only send reasonable deltas (filter out large warp events from Deskflow)
    const MAX_DELTA: c_int = 127;
    if x.abs() <= MAX_DELTA && y.abs() <= MAX_DELTA {
        let mut dev = DEVICE.lock().unwrap();
        let events = [
            InputEvent::new_now(EventType::RELATIVE.0, RelativeAxisCode::REL_X.0, x),
            InputEvent::new_now(EventType::RELATIVE.0, RelativeAxisCode::REL_Y.0, y),
        ];
        let _ = dev.emit(&events);
    }
    // Pass through to real XTest for X11 cursor movement (unless NO_XTEST mode)
    if !*NO_XTEST {
        unsafe { REAL_XTEST_RELATIVE_MOTION(display, x, y, delay) };
    }
    1
}

// Mouse motion - send to both uinput (for Moonlight) and XTest (for X11 cursor)
#[no_mangle]
pub extern "C" fn XTestFakeMotionEvent(
    display: *mut Display,
    screen_number: c_int,
    x: c_int,
    y: c_int,
    delay: c_ulong,
) -> c_int {
    // Convert absolute to relative motion for uinput (Moonlight expects relative)
    {
        let last_x = LAST_X.load(Ordering::Relaxed);
        let last_y = LAST_Y.load(Ordering::Relaxed);

        // Only send relative motion if we have a previous position
        // Filter out large deltas (warp events)
        const MAX_DELTA: c_int = 127;
        if last_x >= 0 && last_y >= 0 {
            let dx = x - last_x;
            let dy = y - last_y;

            if (dx != 0 || dy != 0) && dx.abs() <= MAX_DELTA && dy.abs() <= MAX_DELTA {
                let mut dev = DEVICE.lock().unwrap();
                let events = [
                    InputEvent::new_now(EventType::RELATIVE.0, RelativeAxisCode::REL_X.0, dx),
                    InputEvent::new_now(EventType::RELATIVE.0, RelativeAxisCode::REL_Y.0, dy),
                ];
                let _ = dev.emit(&events);
            }
        }

        // Update last position
        LAST_X.store(x, Ordering::Relaxed);
        LAST_Y.store(y, Ordering::Relaxed);
    }
    // Pass through to real XTest for X11 cursor movement (unless NO_XTEST mode)
    if !*NO_XTEST {
        unsafe { REAL_XTEST_MOTION(display, screen_number, x, y, delay) };
    }
    1
}
