// The only data that may be saved. Each key's numbers come from that key's own events, unordered.
// No timestamps, first or last times, sequence numbers, interval lists or key pairs.
use std::collections::{BTreeMap, BTreeSet};

use crate::chatter::Tally;
use crate::dead::DeadTally;
use crate::fold::{End, Folded};
use crate::input::Round;
use crate::params::{BINS, EDGES_MS, STUCK_US};
use crate::report::{Limits, Note, Report};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Histogram(pub [u32; BINS]);

impl Histogram {
    pub fn bin(us: u64) -> usize {
        EDGES_MS
            .iter()
            .position(|&edge| us < edge * 1000)
            .unwrap_or(BINS - 1)
    }

    pub fn add(&mut self, us: u64) {
        self.0[Self::bin(us)] += 1;
    }

    pub fn total(&self) -> u32 {
        self.0.iter().sum()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PromptTally {
    pub rounds: u16,
    pub asked: u32,
    pub presses: u32,
    pub affected: u32,
    pub extra_downs: u32,
    pub rounds_pressed: u16,
    pub rounds_affected: u16,
    pub assessed: u16,
    pub silent: u16,
    pub not_assessed: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyAggregate {
    pub downs: u32,
    pub ups: u32,
    pub episodes: u32,
    pub presses: u32,
    pub affected: u32,
    pub extra_downs: u32,
    pub repeats: u32,
    pub duplicates: u32,
    pub orphan_ups: u32,
    pub interrupted: u32,
    pub after_resume: u32,
    pub long_holds: u32,
    pub unreleased: u32,
    pub borderline: u32,
    pub timing_unknown: u32,
    pub hold: Histogram,
    pub release_gap: Histogram,
    pub interval: Histogram,
    pub prompted: Option<PromptTally>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Aggregates {
    pub rules: u16,
    pub limits: Limits,
    pub keys: BTreeMap<u16, KeyAggregate>,
}

pub(crate) fn build(
    f: &Folded,
    chatter: &BTreeMap<u16, Tally>,
    dead: &BTreeMap<u16, DeadTally>,
    rounds: &[Round],
    limits: Limits,
) -> Aggregates {
    let mut keys: BTreeMap<u16, KeyAggregate> = BTreeMap::new();
    for (&scan, t) in &f.tracks {
        let a = keys.entry(scan).or_default();
        a.downs = t.downs.len() as u32;
        a.ups = t.ups;
        a.episodes = t.episodes.len() as u32;
        a.orphan_ups = t.orphan_ups;
        a.interrupted = t.interrupted;
        a.after_resume = t.after_resume;
        for (i, e) in t.episodes.iter().enumerate() {
            a.repeats += e.repeats;
            a.duplicates += e.duplicates;
            if let Some(h) = f.hold(e) {
                a.hold.add(h.us);
                if h.us >= STUCK_US {
                    a.long_holds += 1;
                }
            }
            if e.end == End::Open && f.end_us.saturating_sub(e.down) >= STUCK_US {
                a.unreleased += 1;
            }
            if let Some(next) = t.episodes.get(i + 1).filter(|n| n.seg == e.seg) {
                a.interval.add(next.down.saturating_sub(e.down));
                if let Some(g) = f.gap(e, next) {
                    a.release_gap.add(g.us);
                }
            }
        }
        if let Some(c) = chatter.get(&scan) {
            a.presses = c.all.presses;
            a.affected = c.all.affected;
            a.extra_downs = c.all.extra;
            a.borderline = c.all.borderline;
            a.timing_unknown = c.all.unknown;
        }
    }
    let prompted: BTreeSet<u16> = rounds.iter().map(|r| r.key).collect();
    for key in prompted {
        let a = keys.entry(key).or_default();
        let c = chatter.get(&key);
        let d = dead.get(&key).copied().unwrap_or_default();
        a.prompted = Some(PromptTally {
            rounds: rounds.iter().filter(|r| r.key == key).count() as u16,
            asked: rounds
                .iter()
                .filter(|r| r.key == key)
                .map(|r| u32::from(r.asked))
                .sum(),
            presses: c.map_or(0, |c| c.prompted.presses),
            affected: c.map_or(0, |c| c.prompted.affected),
            extra_downs: c.map_or(0, |c| c.prompted.extra),
            rounds_pressed: c.map_or(0, |c| c.rounds_pressed),
            rounds_affected: c.map_or(0, |c| c.rounds_affected),
            assessed: d.assessed,
            silent: d.silent,
            not_assessed: d.not_assessed,
        });
    }
    let timing_unknown = keys.values().map(|a| a.timing_unknown).sum();
    Aggregates {
        rules: crate::RULES,
        limits: Limits {
            timing_unknown,
            ..limits
        },
        keys,
    }
}

impl Report {
    // What may be saved: prompted keys and keys a finding or note names. Free typing's per-key
    // counts would reveal which keys were typed, so they stay out.
    pub fn saved(&self) -> Aggregates {
        let mut named: BTreeSet<u16> = self.findings.iter().map(|f| f.key).collect();
        for note in &self.notes {
            match *note {
                Note::Clean { key, .. }
                | Note::OneExtraDown { key, .. }
                | Note::Unprompted { key, .. }
                | Note::NotAssessed { key, .. }
                | Note::HeldAtPause { key, .. }
                | Note::ReleasedWithNextKey { key, .. } => {
                    named.insert(key);
                }
                Note::DifferentCode { asked, got, .. } => {
                    named.insert(asked);
                    named.insert(got);
                }
                Note::NoInputFromKeyboard | Note::Systemic { .. } => {}
            }
        }
        let mut saved = self.aggregates.clone();
        saved
            .keys
            .retain(|k, a| a.prompted.is_some() || named.contains(k));
        saved
    }
}
