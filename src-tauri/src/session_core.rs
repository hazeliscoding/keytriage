// A test's events, stamped on the test's own clock. Nothing here touches Win32 or Tauri, so the
// same code runs in the capture callback and in tests fed a synthetic stream.
use std::collections::BTreeSet;
use std::time::Instant;

use keytriage_diagnostics::{self as diagnostics, BoardKind, Guide, Report};
use keytriage_input::Input;
use serde::{Deserialize, Serialize};

use crate::view::{GuideView, guide_view};

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

struct Recorder {
    start: Instant,
    held: BTreeSet<HeldKey>,
}

impl Recorder {
    fn new(start: Instant) -> Recorder {
        Recorder {
            start,
            held: BTreeSet::new(),
        }
    }

    fn micros(&self, at: Instant) -> u64 {
        at.saturating_duration_since(self.start).as_micros() as u64
    }

    fn entry(&mut self, input: Input) -> Entry {
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

// One test, as the capture callback and the commands see it: its clock, the ordered events the
// engine reads at the end, and the guided test with the keyboard and board it is diagnosed for.
// Nothing here derives Debug, because the entries are the typed text.
pub struct Core {
    recorder: Recorder,
    entries: Vec<diagnostics::Entry>,
    test: Option<(Guide, Vec<isize>, BoardKind)>,
}

impl Core {
    pub fn new(start: Instant, test: Option<(Guide, Vec<isize>, BoardKind)>) -> Core {
        Core {
            recorder: Recorder::new(start),
            entries: Vec::new(),
            test,
        }
    }

    // The view comes back only when the input changed it.
    pub fn input(&mut self, input: Input) -> (Entry, Option<GuideView>) {
        let entry = self.recorder.entry(input);
        let engine = to_engine(&entry);
        let view = self
            .test
            .as_mut()
            .and_then(|(guide, ..)| guide.entry(&engine).then(|| guide_view(guide)));
        self.entries.push(engine);
        (entry, view)
    }

    pub fn skip(&mut self, at: Instant) -> Option<GuideView> {
        let at_us = self.recorder.micros(at);
        let (guide, ..) = self.test.as_mut()?;
        guide.skip(at_us).then(|| guide_view(guide))
    }

    pub fn view(&self) -> Option<GuideView> {
        self.test.as_ref().map(|(guide, ..)| guide_view(guide))
    }

    pub fn guided(&self) -> bool {
        self.test.is_some()
    }

    // The ordered events and the Guide's rounds leave Core here and are dropped once the engine has
    // read them, so only the report outlives the test. A test without a plan has nothing to
    // diagnose, and keeps its events.
    pub fn finish(&mut self, at: Instant) -> Option<Report> {
        let (mut guide, keyboard, board) = self.test.take()?;
        let end_us = self.recorder.micros(at);
        let rounds = guide.finish(end_us);
        let entries = std::mem::take(&mut self.entries);
        Some(diagnostics::diagnose(&diagnostics::Session {
            entries: &entries,
            end_us,
            keyboard: &keyboard,
            rounds: &rounds,
            board,
        }))
    }
}

#[cfg(test)]
pub fn at(start: Instant, micros: u64) -> Instant {
    start + std::time::Duration::from_micros(micros)
}

// What the capture callback would have been handed for an engine entry, so tests can replay a
// synthetic stream through Core.
#[cfg(test)]
pub fn input(start: Instant, entry: &diagnostics::Entry) -> Input {
    match *entry {
        diagnostics::Entry::Key {
            scan,
            up,
            device,
            micros,
        } => Input::Key(keytriage_input::KeyEvent {
            scan,
            up,
            device,
            at: at(start, micros),
        }),
        diagnostics::Entry::Paused { micros, .. } => Input::Paused(at(start, micros)),
        diagnostics::Entry::Resumed { micros } => Input::Resumed(at(start, micros)),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use keytriage_diagnostics::fixture::guided_chatter;
    use keytriage_diagnostics::{Plan, RULES, Round};
    use keytriage_input::KeyEvent;

    use super::*;
    use crate::view::{KeyName, names, result};

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

    // ---- the guided test through Core ----

    const E: u16 = 0x12;
    const G: u16 = 0x22;
    const J: u16 = 0x24;

    fn press(start: Instant, scan: u16, up: bool, ms: u64) -> Input {
        Input::Key(KeyEvent {
            scan,
            up,
            device: 1,
            at: at(start, ms * 1_000),
        })
    }

    fn guided(
        keys: &[u16],
        rounds: u16,
        board: BoardKind,
    ) -> Option<(Guide, Vec<isize>, BoardKind)> {
        let plan = Plan {
            keys: keys.to_vec(),
            rounds,
            presses: 10,
        };
        Some((Guide::new(plan, &[1]).unwrap(), vec![1], board))
    }

    fn rounds(core: &mut Core, end_us: u64) -> Vec<Round> {
        let (guide, ..) = core.test.as_mut().unwrap();
        guide.finish(end_us)
    }

    #[test]
    fn core01_the_session_runs_the_same_test_as_the_engine() {
        let (plan, f) = guided_chatter();
        let start = Instant::now();
        let guide = Guide::new(plan.clone(), &f.keyboard).unwrap();
        let mut core = Core::new(start, Some((guide, f.keyboard.clone(), f.board)));
        let mut engine = Guide::new(plan, &f.keyboard).unwrap();
        let mut views = vec![core.view().unwrap()];
        let mut expected = vec![guide_view(&engine)];
        for e in &f.entries {
            let (entry, view) = core.input(input(start, e));
            assert!(to_engine(&entry) == *e);
            views.extend(view);
            if engine.entry(e) {
                expected.push(guide_view(&engine));
            }
        }
        assert!(core.entries == f.entries);
        assert!(views == expected);
        assert_eq!(rounds(&mut core, f.end_us), f.rounds);

        let first = &views[0];
        assert_eq!(
            (
                first.key,
                first.count,
                first.round,
                first.index,
                first.done,
                first.total
            ),
            (Some(G), 0, 0, 0, 0, 90)
        );
        let last = views.last().unwrap();
        assert_eq!((last.key, last.done), (None, 90));
        assert_eq!(last.tallies, [(E, 35), (G, 30), (J, 30)]);
        // Each counted press moves the count, and each view follows one change.
        assert!(views.len() > 90);
        assert!(views.windows(2).all(|w| w[0] != w[1]));
    }

    #[test]
    fn core02_a_pause_drops_the_open_round_and_the_resume_repeats_it() {
        let start = Instant::now();
        let mut core = Core::new(start, guided(&[E, G], 1, BoardKind::HotSwap));
        for (up, ms) in [
            (false, 1_000),
            (true, 1_100),
            (false, 1_400),
            (true, 1_500),
            (false, 1_800),
        ] {
            // A release changes the view only when it closes the round.
            assert_eq!(core.input(press(start, E, up, ms)).1.is_some(), !up);
        }
        assert_eq!(core.view().unwrap().count, 3);

        let (entry, view) = core.input(Input::Paused(at(start, 2_000_000)));
        assert_eq!(
            entry,
            Entry::Paused {
                micros: 2_000_000,
                interrupted: vec![HeldKey { device: 1, scan: E }]
            }
        );
        let view = view.expect("the open round was dropped");
        assert_eq!((view.key, view.count, view.done), (Some(E), 0, 0));
        assert!(view.tallies.is_empty());

        let (entry, view) = core.input(Input::Resumed(at(start, 3_000_000)));
        assert_eq!(entry, Entry::Resumed { micros: 3_000_000 });
        assert!(view.is_none());
        // The interrupted key's release after the resume answers nothing.
        assert!(core.input(press(start, E, true, 3_100)).1.is_none());
        let view = core.input(press(start, E, false, 3_400)).1.unwrap();
        assert_eq!(
            (view.key, view.count, view.round, view.index),
            (Some(E), 1, 0, 0)
        );
        assert_eq!(
            rounds(&mut core, 4_000_000),
            [Round {
                key: E,
                asked: 10,
                start_us: 3_000_000,
                end_us: 4_000_000
            }]
        );
    }

    #[test]
    fn core03_skip_moves_on_and_a_test_without_a_plan_only_records() {
        let start = Instant::now();
        let mut core = Core::new(start, guided(&[E, G], 1, BoardKind::Unknown));
        let view = core.skip(at(start, 5_000_000)).unwrap();
        assert_eq!(
            (view.key, view.index, view.count, view.done),
            (Some(G), 1, 0, 10)
        );
        let view = core.skip(at(start, 6_000_000)).unwrap();
        assert_eq!((view.key, view.done), (None, 20));
        assert!(core.skip(at(start, 7_000_000)).is_none());
        assert_eq!(
            rounds(&mut core, 8_000_000),
            [
                Round {
                    key: E,
                    asked: 10,
                    start_us: 0,
                    end_us: 5_000_000
                },
                Round {
                    key: G,
                    asked: 10,
                    start_us: 5_000_000,
                    end_us: 6_000_000
                }
            ]
        );

        let mut free = Core::new(start, None);
        assert!(free.view().is_none());
        let (entry, view) = free.input(press(start, E, false, 1_000));
        assert_eq!(
            entry,
            Entry::Key {
                scan: E,
                up: false,
                device: 1,
                micros: 1_000_000
            }
        );
        assert!(view.is_none());
        assert!(free.skip(at(start, 2_000_000)).is_none());
        assert!(free.test.is_none());
        assert_eq!(free.entries.len(), 1);
    }

    fn named(keys: &[(u16, &str)]) -> Vec<KeyName> {
        keys.iter()
            .map(|&(scan, name)| KeyName {
                scan,
                name: name.to_string(),
            })
            .collect()
    }

    #[test]
    fn res01_the_guided_chatter_run_ends_in_one_finding_on_e() {
        let (plan, f) = guided_chatter();
        let start = Instant::now();
        let guide = Guide::new(plan, &f.keyboard).unwrap();
        let mut core = Core::new(start, Some((guide, f.keyboard.clone(), f.board)));
        for e in &f.entries {
            core.input(input(start, e));
        }
        let report = core.finish(at(start, f.end_us)).unwrap();
        // The events and the Guide are gone, and the engine saw what the fixture's own run saw.
        assert!(core.entries.is_empty() && !core.guided());
        assert_eq!(report, f.diagnose());

        let labels = names(named(&[(G, "G"), (J, "J"), (E, "E")])).unwrap();
        let r = result(&report, &labels);
        assert_eq!(r.rules, RULES);
        assert_eq!(r.findings.len(), 1);
        let v = &r.findings[0];
        assert_eq!(
            (
                v.key,
                v.kind,
                v.confidence,
                v.title.as_str(),
                v.level.as_str(),
                v.strong
            ),
            (
                E,
                "chatter",
                "very-high",
                "Possible chatter",
                "Very high",
                true
            )
        );
        assert!(
            v.evidence[0].starts_with("6 of 30 presses sent an extra key-down"),
            "{}",
            v.evidence[0]
        );
        assert_eq!(
            v.causes,
            [
                "Switch contacts",
                "Hot-swap socket or solder joint",
                "Firmware debounce"
            ]
        );
        assert_eq!(
            v.next[0],
            "Say whether the keyboard is hot-swap, soldered or a laptop keyboard. The next steps \
             differ."
        );
        assert!(
            v.next[1].starts_with("Swap the E switch with the G switch"),
            "{}",
            v.next[1]
        );
        let gaps = v.gaps.as_ref().unwrap();
        let bars: Vec<(&str, u32)> = gaps.iter().map(|b| (b.label, b.count)).collect();
        let saved = report.saved();
        assert_eq!(
            bars.iter().map(|b| b.1).sum::<u32>(),
            saved.keys[&E].release_gap.total()
        );
        // The six extra key-downs came 5 ms after a release.
        assert_eq!(bars[..2], [("<4", 0), ("4–12", 6)]);

        let keys: Vec<(u16, u32)> = r.keys.iter().map(|k| (k.scan, k.count)).collect();
        assert_eq!(keys, [(E, 36), (G, 30), (J, 30)]);
        assert_eq!(r.clean.len(), 2);
        assert!(r.clean[0].starts_with("G: no extra key-downs in 30 presses"));
        assert!(r.clean[1].starts_with("J: no extra key-downs in 30 presses"));
        assert!(r.notes.is_empty());
        assert_eq!(
            r.resolution,
            "Timing resolution for this keyboard is 4 ms or finer."
        );

        // The page reads these names. The result holds no times.
        let json = serde_json::to_string(&r).unwrap();
        for part in [
            r#"{"rules":2,"findings":[{"key":18,"kind":"chatter","confidence":"very-high","title":"Possible chatter","level":"Very high","strong":true,"evidence":["6 of 30 presses"#,
            r#"],"causes":["Switch contacts","#,
            r#"],"next":["Say whether"#,
            r#"],"gaps":[{"label":"<4","count":0},{"label":"4–12","count":6},{"label":"12–20","count":0},"#,
            r#"{"label":"100+","count":29}]}],"notes":[],"clean":["G: no extra key-downs"#,
            r#"],"keys":[{"scan":18,"count":36},{"scan":34,"count":30},{"scan":36,"count":30}],"resolution":"Timing"#,
        ] {
            assert!(json.contains(part), "{part}");
        }
        for time in [r#""micros""#, r#"_us""#, r#""start"#, r#""end"#] {
            assert!(!json.contains(time), "{time}");
        }
    }

    #[test]
    fn res05_a_test_without_a_plan_has_nothing_to_diagnose() {
        let start = Instant::now();
        let mut free = Core::new(start, None);
        free.input(press(start, E, false, 1_000));
        assert!(!free.guided());
        assert!(free.finish(at(start, 2_000_000)).is_none());
        assert_eq!(free.entries.len(), 1);
    }
}
