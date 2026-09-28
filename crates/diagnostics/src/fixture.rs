// Synthetic event streams for fixtures. Every value comes from numbers here, never from typing.
use std::collections::BTreeMap;

use crate::guide::{Guide, Plan};
use crate::input::{BoardKind, Device, Entry, HeldKey, Round, Session};
use crate::report::Report;

pub const fn ms(n: u64) -> u64 {
    n * 1_000
}

// A deliberate press: a finger-length hold, then a gap no chatter reaches.
pub const HOLD: (u64, u64) = (ms(80), ms(130));
pub const GAP: (u64, u64) = (ms(150), ms(250));

pub fn normal(s: Synth, scan: u16) -> Synth {
    s.taps(scan, 1, HOLD, GAP)
}

#[derive(Clone)]
pub struct Synth {
    entries: Vec<Entry>,
    rounds: Vec<Round>,
    now: u64,
    device: Device,
    keyboard: Vec<Device>,
    board: BoardKind,
    seed: u64,
}

pub struct Fixture {
    pub entries: Vec<Entry>,
    pub rounds: Vec<Round>,
    pub end_us: u64,
    pub keyboard: Vec<Device>,
    pub board: BoardKind,
}

impl Fixture {
    pub fn session(&self) -> Session<'_> {
        Session {
            entries: &self.entries,
            end_us: self.end_us,
            keyboard: &self.keyboard,
            rounds: &self.rounds,
            board: self.board,
        }
    }

    pub fn diagnose(&self) -> Report {
        crate::diagnose(&self.session())
    }
}

impl Default for Synth {
    fn default() -> Self {
        Self::new()
    }
}

impl Synth {
    pub fn new() -> Self {
        Synth {
            entries: Vec::new(),
            rounds: Vec::new(),
            now: 0,
            device: 1,
            keyboard: vec![1],
            board: BoardKind::Unknown,
            seed: 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub fn board(mut self, board: BoardKind) -> Self {
        self.board = board;
        self
    }

    pub fn keyboard(mut self, handles: &[Device]) -> Self {
        self.keyboard = handles.to_vec();
        self
    }

    pub fn on(mut self, device: Device) -> Self {
        self.device = device;
        self
    }

    pub fn now(&self) -> u64 {
        self.now
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn wait(mut self, us: u64) -> Self {
        self.now += us;
        self
    }

    pub fn down(mut self, scan: u16) -> Self {
        self.key(scan, false);
        self
    }

    pub fn up(mut self, scan: u16) -> Self {
        self.key(scan, true);
        self
    }

    pub fn raw(mut self, entry: Entry) -> Self {
        self.entries.push(entry);
        self
    }

    // One press; the clock stops at its release.
    pub fn press(self, scan: u16, hold: u64) -> Self {
        self.down(scan).wait(hold).up(scan)
    }

    // Alternating hold and gap lengths, starting and ending with a hold: [5 ms, 5 ms, 100 ms] is a
    // 5 ms press, a 5 ms release, then a 100 ms press.
    pub fn fragments(mut self, scan: u16, spans: &[u64]) -> Self {
        for (i, &span) in spans.iter().enumerate() {
            self = if i % 2 == 0 {
                self.press(scan, span)
            } else {
                self.wait(span)
            };
        }
        self
    }

    // n deliberate presses, each hold and the gap after it drawn from the given ranges.
    pub fn taps(mut self, scan: u16, n: u32, hold: (u64, u64), gap: (u64, u64)) -> Self {
        for _ in 0..n {
            let h = self.draw(hold);
            let g = self.draw(gap);
            self = self.press(scan, h).wait(g);
        }
        self
    }

    // A held key with autorepeat: a down, repeats from `delay` every `period`, one up at `len`.
    pub fn hold(mut self, scan: u16, len: u64, delay: u64, period: u64) -> Self {
        let start = self.now;
        self = self.down(scan);
        let mut t = delay;
        while t < len {
            self.now = start + t;
            self = self.down(scan);
            t += period;
        }
        self.now = start + len;
        self.up(scan)
    }

    // Autorepeat downs only, for a key that is already down.
    pub fn repeats(mut self, scan: u16, delay: u64, period: u64, count: u32) -> Self {
        for i in 0..count {
            self = self.wait(if i == 0 { delay } else { period }).down(scan);
        }
        self
    }

    pub fn pause(mut self, interrupted: &[u16]) -> Self {
        let device = self.device;
        self.entries.push(Entry::Paused {
            micros: self.now,
            interrupted: interrupted
                .iter()
                .map(|&scan| HeldKey { device, scan })
                .collect(),
        });
        self
    }

    pub fn resume(mut self) -> Self {
        self.entries.push(Entry::Resumed { micros: self.now });
        self
    }

    // A guided round: 300 ms of lead-in, the body, 300 ms of lead-out, then 500 ms to the next.
    pub fn round(mut self, key: u16, asked: u16, body: impl FnOnce(Self) -> Self) -> Self {
        let start = self.now;
        self = body(self.wait(ms(300))).wait(ms(300));
        self.rounds.push(Round {
            key,
            asked,
            start_us: start,
            end_us: self.now,
        });
        self.wait(ms(500))
    }

    // A keyboard polled every `poll` that sends every transition in its own report (QMK does):
    // each key event moves to the next report, a key's next transition to a later one, and events
    // that share a report are read 30 us apart. Not for streams with pauses.
    pub fn polled(mut self, poll: u64) -> Self {
        let mut last: BTreeMap<(Device, u16), u64> = BTreeMap::new();
        for e in &mut self.entries {
            if let Entry::Key {
                scan,
                device,
                micros,
                ..
            } = e
            {
                let mut slot = micros.div_ceil(poll) * poll;
                if let Some(&prev) = last.get(&(*device, *scan)) {
                    slot = slot.max(prev + poll);
                }
                last.insert((*device, *scan), slot);
                *micros = slot;
            }
        }
        self.entries.sort_by_key(Entry::micros);
        let mut prev = None;
        let mut shared = 0;
        for e in &mut self.entries {
            if let Entry::Key { micros, .. } = e {
                let slot = *micros;
                shared = if prev == Some(slot) { shared + 1 } else { 0 };
                prev = Some(slot);
                *micros = slot + 30 * shared;
            }
        }
        self
    }

    // A keyboard polled every `poll` whose firmware overwrites a pending report: a press and its
    // release that fall between two polls never reach the host.
    pub fn polled_lossy(mut self, poll: u64) -> Self {
        let mut out: Vec<Option<Entry>> = Vec::new();
        let mut last: BTreeMap<(Device, u16), usize> = BTreeMap::new();
        for e in std::mem::take(&mut self.entries) {
            let Entry::Key {
                scan,
                up,
                device,
                micros,
            } = e
            else {
                out.push(Some(e));
                continue;
            };
            let slot = micros.div_ceil(poll) * poll;
            if let Some(&i) = last.get(&(device, scan))
                && let Some(Entry::Key {
                    up: before,
                    micros: at,
                    ..
                }) = out[i]
                && at == slot
                && before != up
            {
                out[i] = None;
                last.remove(&(device, scan));
                continue;
            }
            last.insert((device, scan), out.len());
            out.push(Some(Entry::Key {
                scan,
                up,
                device,
                micros: slot,
            }));
        }
        self.entries = out.into_iter().flatten().collect();
        self
    }

    pub fn build(self) -> Fixture {
        let end = self.now + ms(200);
        self.finish(end)
    }

    pub fn end_now(self) -> Fixture {
        let end = self.now;
        self.finish(end)
    }

    fn finish(self, end_us: u64) -> Fixture {
        let mut rounds = self.rounds;
        rounds.sort_by_key(|r| r.start_us);
        Fixture {
            entries: self.entries,
            rounds,
            end_us,
            keyboard: self.keyboard,
            board: self.board,
        }
    }

    fn key(&mut self, scan: u16, up: bool) {
        self.entries.push(Entry::Key {
            scan,
            up,
            device: self.device,
            micros: self.now,
        });
    }

    // xorshift64: the same numbers on every machine and run.
    fn draw(&mut self, (lo, hi): (u64, u64)) -> u64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        lo + self.seed % (hi - lo + 1)
    }
}

// Each key's own timeline, shifted as a whole by a seeded offset. Only how keys interleave changes.
pub fn interleaved(seed: u64) -> Fixture {
    const E: u16 = 0x12;
    const K: u16 = 0x25;
    const F: u16 = 0x21;
    let parts = [
        {
            let mut s = Synth::new();
            for p in 0..40 {
                s = if p % 7 == 2 {
                    s.fragments(E, &[ms(5), ms(5), ms(100)]).wait(ms(200))
                } else {
                    normal(s, E)
                };
            }
            s
        },
        Synth::new().taps(K, 40, (ms(40), ms(120)), (ms(40), ms(250))),
        Synth::new().taps(F, 40, (ms(5), ms(120)), (ms(10), ms(250))),
    ];
    let mut x = seed | 1;
    let mut all = Vec::new();
    for part in parts {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let offset = x % ms(5_000);
        for e in part.end_now().entries {
            if let Entry::Key {
                scan,
                up,
                device,
                micros,
            } = e
            {
                all.push(Entry::Key {
                    scan,
                    up,
                    device,
                    micros: micros + offset,
                });
            }
        }
    }
    all.sort_by_key(Entry::micros);
    let end_us = all.last().map_or(0, Entry::micros) + ms(100);
    Fixture {
        entries: all,
        rounds: Vec::new(),
        end_us,
        keyboard: vec![1],
        board: BoardKind::Unknown,
    }
}

// A guided test on keyboard [1]. `typist` answers the prompted key once per call, told which
// answer to that prompt this is (from 1), and returns true to skip the key. A real Guide reads the
// events as the capture callback would, so the rounds are the ones the app would stamp.
pub fn guided(plan: Plan, mut typist: impl FnMut(Synth, u16, u32) -> (Synth, bool)) -> Fixture {
    let mut s = Synth::new();
    let mut guide = Guide::new(plan, &s.keyboard).expect("a valid plan");
    let mut fed = 0;
    while let Some(prompt) = guide.prompt() {
        let step = (prompt.round, prompt.index);
        // Time to read the prompt.
        s = s.wait(ms(600));
        for n in 1.. {
            assert!(n <= 1_000, "the prompt for {:04X} never moved", prompt.key);
            let skip;
            (s, skip) = typist(s, prompt.key, n);
            for e in &s.entries[fed..] {
                guide.entry(e);
            }
            fed = s.entries.len();
            if skip {
                guide.skip(s.now);
            }
            if guide.prompt().is_none_or(|p| (p.round, p.index) != step) {
                break;
            }
        }
    }
    let end_us = s.entries.last().map_or(s.now, Entry::micros) + ms(200);
    let rounds = guide.finish(end_us);
    Fixture {
        entries: s.entries,
        rounds,
        end_us,
        keyboard: s.keyboard,
        board: s.board,
    }
}

// The Done-when run: G, J and E, 3 rounds of 10. E's 5th and 10th answers in every round split
// into a 5 ms press and a 100 ms one, 5 ms apart.
pub fn guided_chatter() -> (Plan, Fixture) {
    const E: u16 = 0x12;
    const G: u16 = 0x22;
    const J: u16 = 0x24;
    let plan = Plan {
        keys: vec![G, J, E],
        rounds: 3,
        presses: 10,
    };
    let fixture = guided(plan.clone(), |s, key, n| {
        let s = if key == E && matches!(n, 5 | 10) {
            s.fragments(E, &[ms(5), ms(5), ms(100)]).wait(ms(200))
        } else {
            normal(s, key)
        };
        (s, false)
    });
    (plan, fixture)
}

// A swap retest on a hot-swap board. Every 5th answer of a key in `faulty` in each round is
// guided_chatter's fault, at its rate of 1 in 5.
pub fn swap_chatter(plan: Plan, faulty: &[u16]) -> Fixture {
    let fixture = guided(plan, |s, key, n| {
        let s = if faulty.contains(&key) && n % 5 == 0 {
            s.fragments(key, &[ms(5), ms(5), ms(100)]).wait(ms(200))
        } else {
            normal(s, key)
        };
        (s, false)
    });
    Fixture {
        board: BoardKind::HotSwap,
        ..fixture
    }
}

// Drawn-keyboard names for the positions the fixtures use.
pub fn label(scan: u16) -> String {
    match scan {
        0x12 => "E".into(),
        0x13 => "R".into(),
        0x21 => "F".into(),
        0x22 => "G".into(),
        0x23 => "H".into(),
        0x24 => "J".into(),
        _ => crate::words::code_label(scan),
    }
}
