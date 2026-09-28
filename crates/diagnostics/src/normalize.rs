// Drops what isn't a key press on the keyboard under test before anything is timed.
use std::collections::{BTreeMap, BTreeSet};

use crate::input::{Device, Entry, Session};
use crate::keys;
use crate::report::Limits;

pub(crate) enum Norm {
    Key {
        scan: u16,
        up: bool,
        at: u64,
        handle: Device,
    },
    Pause {
        at: u64,
        interrupted: Vec<u16>,
    },
    Resume {
        at: u64,
    },
}

pub(crate) struct Normalized {
    pub events: Vec<Norm>,
    pub overruns: Vec<u64>,
    pub foreign: Vec<(u16, u64)>,
    pub injected: Vec<u64>,
    pub limits: Limits,
}

pub(crate) fn normalize(session: &Session) -> Normalized {
    let mine = |d: Device| d != 0 && (session.keyboard.is_empty() || session.keyboard.contains(&d));
    let mut out = Normalized {
        events: Vec::new(),
        overruns: Vec::new(),
        foreign: Vec::new(),
        injected: Vec::new(),
        limits: Limits::default(),
    };
    let mut pause_head: BTreeMap<Device, (bool, u64)> = BTreeMap::new();
    // What is down after the filters above, so a pause lists only keys this stream holds. The
    // session lists every code it saw go down, including the ones dropped here.
    let mut down: BTreeSet<(Device, u16)> = BTreeSet::new();
    for entry in session.entries {
        match *entry {
            Entry::Key {
                scan,
                up,
                device,
                micros,
            } => {
                // SendInput and user-mode remappers arrive with no device, and can send any code.
                if device == 0 {
                    out.limits.injected += 1;
                    out.injected.push(micros);
                    continue;
                }
                if !mine(device) {
                    out.limits.other_devices += 1;
                    if !up {
                        out.foreign.push((scan, micros));
                    }
                    continue;
                }
                let low = scan & 0xFF;
                // Media and other keys without a make code arrive as 0.
                if low == 0 {
                    out.limits.unknown_codes += 1;
                    continue;
                }
                // HID ErrorRollOver: too many keys at once. It says the keyboard hit its limit, not
                // that a key was pressed.
                if low == 0xFF {
                    if !up {
                        out.limits.overruns += 1;
                        out.overruns.push(micros);
                    }
                    continue;
                }
                if scan == keys::FAKE_LEFT_SHIFT || scan == keys::FAKE_RIGHT_SHIFT {
                    out.limits.fake_shifts += 1;
                    continue;
                }
                // Pause arrives as 0xE11D followed at once by Num Lock's code. The second half is
                // dropped so it doesn't count as a Num Lock press.
                let head = pause_head.remove(&device);
                if scan == keys::NUM_LOCK
                    && let Some((head_up, at)) = head
                    && head_up == up
                    && micros >= at
                    && micros - at <= crate::params::PAUSE_TAIL_US
                {
                    continue;
                }
                if scan == keys::PAUSE {
                    pause_head.insert(device, (up, micros));
                }
                if up {
                    down.remove(&(device, scan));
                } else {
                    down.insert((device, scan));
                }
                out.events.push(Norm::Key {
                    scan,
                    up,
                    at: micros,
                    handle: device,
                });
            }
            Entry::Paused {
                micros,
                ref interrupted,
            } => {
                out.limits.pauses += 1;
                pause_head.clear();
                out.events.push(Norm::Pause {
                    at: micros,
                    interrupted: interrupted
                        .iter()
                        .filter(|h| mine(h.device) && down.contains(&(h.device, h.scan)))
                        .map(|h| h.scan)
                        .collect(),
                });
            }
            Entry::Resumed { micros } => out.events.push(Norm::Resume { at: micros }),
        }
    }
    out
}
