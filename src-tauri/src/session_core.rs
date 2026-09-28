// A test's events, stamped on the test's own clock. Nothing here touches Win32 or Tauri, so the
// same code runs in the capture callback and in tests fed a synthetic stream.
use std::collections::BTreeSet;
use std::time::Instant;

use keytriage_diagnostics as diagnostics;
use keytriage_input::Input;
use serde::{Deserialize, Serialize};

// The `test:event` payload. The page, the debug echo and the scripts' checks all read this shape.
// Debug only in tests, because a printed list of these is the typed text.
#[cfg_attr(test, derive(Debug))]
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Entry {
    Key {
        scan: u16,
        up: bool,
        device: isize,
        // Microseconds since the test started, here and below.
        micros: u64,
    },
    // A key still down when the app lost the foreground may be released while it is away, which
    // capture never sees, so it is listed as interrupted rather than left to look stuck. If it is
    // still held at the resume, its release, and any repeats, arrive after Resumed.
    Paused {
        micros: u64,
        interrupted: Vec<HeldKey>,
    },
    Resumed {
        micros: u64,
    },
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HeldKey {
    pub device: isize,
    pub scan: u16,
}

pub struct Recorder {
    start: Instant,
    held: BTreeSet<HeldKey>,
}

impl Recorder {
    pub fn new(start: Instant) -> Recorder {
        Recorder {
            start,
            held: BTreeSet::new(),
        }
    }

    pub fn micros(&self, at: Instant) -> u64 {
        at.saturating_duration_since(self.start).as_micros() as u64
    }

    pub fn entry(&mut self, input: Input) -> Entry {
        match input {
            Input::Key(event) => {
                let key = HeldKey {
                    device: event.device,
                    scan: event.scan,
                };
                if event.up {
                    self.held.remove(&key);
                } else {
                    self.held.insert(key);
                }
                Entry::Key {
                    scan: event.scan,
                    up: event.up,
                    device: event.device,
                    micros: self.micros(event.at),
                }
            }
            Input::Paused(at) => Entry::Paused {
                micros: self.micros(at),
                interrupted: std::mem::take(&mut self.held).into_iter().collect(),
            },
            Input::Resumed(at) => Entry::Resumed {
                micros: self.micros(at),
            },
        }
    }
}

// Every field is named, so a field added to either type, or a variant added here, fails to compile
// until it is mapped. wire02's way back catches a variant added to the engine.
pub fn to_engine(entry: &Entry) -> diagnostics::Entry {
    match *entry {
        Entry::Key {
            scan,
            up,
            device,
            micros,
        } => diagnostics::Entry::Key {
            scan,
            up,
            device,
            micros,
        },
        Entry::Paused {
            micros,
            ref interrupted,
        } => diagnostics::Entry::Paused {
            micros,
            interrupted: interrupted
                .iter()
                .map(|&HeldKey { device, scan }| diagnostics::HeldKey { device, scan })
                .collect(),
        },
        Entry::Resumed { micros } => diagnostics::Entry::Resumed { micros },
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use keytriage_input::KeyEvent;

    use super::*;

    fn key(recorder: &Recorder, scan: u16, up: bool, micros: u64) -> Input {
        Input::Key(KeyEvent {
            scan,
            up,
            device: 7,
            at: recorder.start + Duration::from_micros(micros),
        })
    }

    #[test]
    fn records_time_since_the_test_started() {
        let mut r = Recorder::new(Instant::now());
        let input = key(&r, 0x1e, true, 1500);
        assert_eq!(
            r.entry(input),
            Entry::Key {
                scan: 0x1e,
                up: true,
                device: 7,
                micros: 1500
            }
        );
        let early = Input::Resumed(r.start - Duration::from_millis(1));
        assert_eq!(r.entry(early), Entry::Resumed { micros: 0 });
    }

    #[test]
    fn a_pause_lists_the_keys_still_down_as_interrupted() {
        let mut r = Recorder::new(Instant::now());
        for input in [
            key(&r, 0x1e, false, 10),
            key(&r, 0x1f, false, 20),
            key(&r, 0x1f, false, 30),
            key(&r, 0x1e, true, 40),
        ] {
            r.entry(input);
        }
        let paused = Input::Paused(r.start + Duration::from_micros(50));
        assert_eq!(
            r.entry(paused),
            Entry::Paused {
                micros: 50,
                interrupted: vec![HeldKey {
                    device: 7,
                    scan: 0x1f
                }]
            }
        );
        let again = Input::Paused(r.start + Duration::from_micros(60));
        assert_eq!(
            r.entry(again),
            Entry::Paused {
                micros: 60,
                interrupted: vec![]
            }
        );
    }

    fn samples() -> Vec<Entry> {
        vec![
            Entry::Key {
                scan: 0x12,
                up: false,
                device: 1,
                micros: 1500,
            },
            Entry::Key {
                scan: 0xE048,
                up: true,
                device: 65603,
                micros: 2_000_000,
            },
            Entry::Paused {
                micros: 50,
                interrupted: vec![
                    HeldKey {
                        device: 7,
                        scan: 0x1f,
                    },
                    HeldKey {
                        device: 0,
                        scan: 0x12,
                    },
                ],
            },
            Entry::Paused {
                micros: 0,
                interrupted: vec![],
            },
            Entry::Resumed { micros: 60 },
        ]
    }

    #[test]
    fn wire01_the_event_payload_is_pinned() {
        let json: Vec<String> = samples()
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect();
        assert_eq!(
            json,
            [
                r#"{"kind":"key","scan":18,"up":false,"device":1,"micros":1500}"#,
                r#"{"kind":"key","scan":57416,"up":true,"device":65603,"micros":2000000}"#,
                r#"{"kind":"paused","micros":50,"interrupted":[{"device":7,"scan":31},{"device":0,"scan":18}]}"#,
                r#"{"kind":"paused","micros":0,"interrupted":[]}"#,
                r#"{"kind":"resumed","micros":60}"#,
            ]
        );
        // The debug echo reads the payload back.
        for (entry, text) in samples().iter().zip(&json) {
            assert_eq!(&serde_json::from_str::<Entry>(text).unwrap(), entry);
        }
    }

    // The way back, for the test only. Both directions name every field and variant.
    fn from_engine(entry: &diagnostics::Entry) -> Entry {
        match *entry {
            diagnostics::Entry::Key {
                scan,
                up,
                device,
                micros,
            } => Entry::Key {
                scan,
                up,
                device,
                micros,
            },
            diagnostics::Entry::Paused {
                micros,
                ref interrupted,
            } => Entry::Paused {
                micros,
                interrupted: interrupted
                    .iter()
                    .map(|&diagnostics::HeldKey { device, scan }| HeldKey { device, scan })
                    .collect(),
            },
            diagnostics::Entry::Resumed { micros } => Entry::Resumed { micros },
        }
    }

    #[test]
    fn wire02_the_engine_gets_every_field() {
        for entry in samples() {
            assert_eq!(from_engine(&to_engine(&entry)), entry);
        }
        // The engine's types print nothing outside their crate, so these compare with ==.
        assert!(
            to_engine(&samples()[2])
                == diagnostics::Entry::Paused {
                    micros: 50,
                    interrupted: vec![
                        diagnostics::HeldKey {
                            device: 7,
                            scan: 0x1f
                        },
                        diagnostics::HeldKey {
                            device: 0,
                            scan: 0x12
                        },
                    ],
                }
        );
        assert!(
            to_engine(&samples()[1])
                == diagnostics::Entry::Key {
                    scan: 0xE048,
                    up: true,
                    device: 65603,
                    micros: 2_000_000,
                }
        );
    }
}
