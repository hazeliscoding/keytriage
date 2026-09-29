// Set-1 make codes with the 0xE0 or 0xE1 prefix in the high byte. Positions, never characters.

pub const PAUSE: u16 = 0xE11D;
// Pause's second half shares Num Lock's code.
pub const NUM_LOCK: u16 = 0x0045;
// Windows wraps the navigation keys in these when Num Lock is on or Shift is held, and puts one
// before Print Screen. They are not keys.
pub const FAKE_LEFT_SHIFT: u16 = 0xE02A;
pub const FAKE_RIGHT_SHIFT: u16 = 0xE036;
pub const HANJA: u16 = 0x00F1;
pub const HANGUL: u16 = 0x00F2;
pub const LEFT_SHIFT: u16 = 0x002A;
pub const RIGHT_SHIFT: u16 = 0x0036;
pub const LEFT_WIN: u16 = 0xE05B;
pub const RIGHT_WIN: u16 = 0xE05C;
pub const PRINT_SCREEN: u16 = 0xE037;

const MODIFIERS: [u16; 8] = [
    LEFT_SHIFT,
    RIGHT_SHIFT,
    0x001D,
    0xE01D,
    0x0038,
    0xE038,
    LEFT_WIN,
    RIGHT_WIN,
];

// Keys with no release code, which can't be held and can't be stuck.
pub fn never_released(scan: u16) -> bool {
    matches!(scan, PAUSE | HANJA | HANGUL)
}

pub fn is_modifier(scan: u16) -> bool {
    MODIFIERS.contains(&scan)
}

// Windows answers these with a window of its own, which takes the foreground: the Start menu, the
// screen capture tool, and after five Shift presses the Sticky Keys prompt. A round of them would
// pause the test again and again.
pub fn os_owned(scan: u16) -> bool {
    matches!(
        scan,
        LEFT_WIN | RIGHT_WIN | PRINT_SCREEN | LEFT_SHIFT | RIGHT_SHIFT
    )
}

// The digit and letter rows, which have no stabilizer, so their switches swap most easily.
pub fn is_plain(scan: u16) -> bool {
    matches!(scan, 0x02..=0x0D | 0x10..=0x1B | 0x1E..=0x28 | 0x2C..=0x35)
}
