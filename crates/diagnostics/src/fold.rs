// Folds each key's own events into presses. Nothing here compares one key's timing with another's.
use std::collections::BTreeMap;

use crate::input::Device;
use crate::normalize::{Norm, Normalized};
use crate::params::REPEAT_MIN_DELAY_US;
use crate::report::Limits;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum End {
    Up { at: u64, handle: Device },
    Interrupted { at: u64 },
    Open,
}

pub(crate) struct Episode {
    pub seg: u32,
    pub down: u64,
    pub handle: Device,
    pub end: End,
    pub repeats: u32,
    pub duplicates: u32,
}

impl Episode {
    pub fn up(&self) -> Option<u64> {
        match self.end {
            End::Up { at, .. } => Some(at),
            _ => None,
        }
    }

    pub fn until(&self, end_us: u64) -> u64 {
        match self.end {
            End::Up { at, .. } | End::Interrupted { at } => at,
            End::Open => end_us.max(self.down),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum State {
    #[default]
    Idle,
    Held(usize),
    Absorbing,
}

#[derive(Default)]
pub(crate) struct Track {
    pub episodes: Vec<Episode>,
    state: State,
    pub downs: Vec<u64>,
    pub ups: u32,
    pub orphan_ups: u32,
    pub after_resume: u32,
    pub interrupted: u32,
}

pub(crate) struct Folded {
    pub tracks: BTreeMap<u16, Track>,
    pub paused: Vec<(u64, u64)>,
    pub overruns: Vec<u64>,
    pub foreign: Vec<(u16, u64)>,
    pub injected: Vec<u64>,
    pub limits: Limits,
    pub end_us: u64,
}

// An interval of one key: its length, and whether it was timed honestly.
#[derive(Clone, Copy)]
pub(crate) struct Interval {
    pub us: u64,
    pub countable: bool,
}

impl Folded {
    fn overrun_between(&self, from: u64, to: u64) -> bool {
        self.overruns.iter().any(|&o| from < o && o <= to)
    }

    pub fn hold(&self, e: &Episode) -> Option<Interval> {
        let End::Up { at, handle } = e.end else {
            return None;
        };
        let us = at.saturating_sub(e.down);
        Some(Interval {
            us,
            countable: handle == e.handle && !self.overrun_between(e.down, at),
        })
    }

    // From `a`'s release to `b`'s key-down, when both are in one segment.
    pub fn gap(&self, a: &Episode, b: &Episode) -> Option<Interval> {
        let End::Up { at, handle } = a.end else {
            return None;
        };
        if a.seg != b.seg {
            return None;
        }
        let us = b.down.saturating_sub(at);
        Some(Interval {
            us,
            countable: handle == b.handle && !self.overrun_between(at, b.down),
        })
    }

    pub fn paused_within(&self, from: u64, to: u64) -> u64 {
        self.paused
            .iter()
            .map(|&(a, b)| b.min(to).saturating_sub(a.max(from)))
            .sum()
    }
}

pub(crate) fn fold(norm: Normalized, end_us: u64) -> Folded {
    let mut tracks: BTreeMap<u16, Track> = BTreeMap::new();
    let mut paused = Vec::new();
    let mut pause_start: Option<u64> = None;
    let mut seg = 0u32;
    for event in norm.events {
        match event {
            Norm::Key {
                scan,
                up,
                at,
                handle,
            } => {
                let t = tracks.entry(scan).or_default();
                if up {
                    t.ups += 1;
                    match t.state {
                        State::Absorbing => {
                            t.state = State::Idle;
                            t.after_resume += 1;
                        }
                        State::Held(i) => {
                            t.episodes[i].end = End::Up { at, handle };
                            t.state = State::Idle;
                        }
                        State::Idle => t.orphan_ups += 1,
                    }
                } else {
                    t.downs.push(at);
                    match t.state {
                        State::Absorbing => t.after_resume += 1,
                        State::Held(i) => {
                            let e = &mut t.episodes[i];
                            if at.saturating_sub(e.down) < REPEAT_MIN_DELAY_US {
                                e.duplicates += 1;
                            } else {
                                e.repeats += 1;
                            }
                        }
                        State::Idle => {
                            t.episodes.push(Episode {
                                seg,
                                down: at,
                                handle,
                                end: End::Open,
                                repeats: 0,
                                duplicates: 0,
                            });
                            t.state = State::Held(t.episodes.len() - 1);
                        }
                    }
                }
            }
            Norm::Pause { at, interrupted } => {
                seg += 1;
                pause_start.get_or_insert(at);
                for t in tracks.values_mut() {
                    if let State::Held(i) = t.state {
                        t.episodes[i].end = End::Interrupted { at };
                        t.interrupted += 1;
                        t.state = State::Absorbing;
                    }
                }
                for scan in interrupted {
                    let t = tracks.entry(scan).or_default();
                    if t.state == State::Idle {
                        t.state = State::Absorbing;
                    }
                }
            }
            Norm::Resume { at } => {
                if let Some(start) = pause_start.take() {
                    paused.push((start, at.max(start)));
                }
            }
        }
    }
    if let Some(start) = pause_start {
        paused.push((start, end_us.max(start)));
    }
    Folded {
        tracks,
        paused,
        overruns: norm.overruns,
        foreign: norm.foreign,
        injected: norm.injected,
        limits: norm.limits,
        end_us,
    }
}
