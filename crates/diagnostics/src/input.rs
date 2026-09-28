pub type Device = isize;

// Mirrors src-tauri's session_core::Entry field for field. It derives Debug only in tests, because
// a formatted list of these is the typed text.
#[cfg_attr(test, derive(Debug))]
#[derive(Clone, PartialEq, Eq)]
pub enum Entry {
    Key {
        scan: u16,
        up: bool,
        device: Device,
        micros: u64,
    },
    Paused {
        micros: u64,
        interrupted: Vec<HeldKey>,
    },
    Resumed {
        micros: u64,
    },
}

impl Entry {
    pub fn micros(&self) -> u64 {
        match *self {
            Entry::Key { micros, .. }
            | Entry::Paused { micros, .. }
            | Entry::Resumed { micros } => micros,
        }
    }
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HeldKey {
    pub device: Device,
    pub scan: u16,
}

// One guided prompt: "press `key` `asked` times", shown over [start_us, end_us).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Round {
    pub key: u16,
    pub asked: u16,
    pub start_us: u64,
    pub end_us: u64,
}

impl Round {
    pub fn contains(&self, at: u64) -> bool {
        self.start_us <= at && at < self.end_us
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum BoardKind {
    #[default]
    Unknown,
    HotSwap,
    Soldered,
    Laptop,
}

pub struct Session<'a> {
    pub entries: &'a [Entry],
    pub end_us: u64,
    // Every Raw Input handle of the keyboard under test, one per HID collection. Empty accepts
    // every handle except 0.
    pub keyboard: &'a [Device],
    pub rounds: &'a [Round],
    pub board: BoardKind,
}
