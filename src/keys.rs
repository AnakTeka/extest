use evdev::KeyCode;

/// Generate all possible keyboard keycodes (0-255)
/// This covers all standard and extended keyboard keys
pub fn all_keys() -> impl Iterator<Item = KeyCode> {
    (0u16..256).map(KeyCode::new)
}
