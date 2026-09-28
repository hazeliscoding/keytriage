// The only data that may be saved. Each key's numbers come from that key's own events, unordered.
// No timestamps, first or last times, sequence numbers, interval lists or key pairs.
use std::collections::{BTreeMap, BTreeSet};

use crate::chatter::Tally;
use crate::dead::DeadTally;
use crate::fold::{End, Folded};
use crate::input::Round;
use crate::params::{BINS, EDGES_MS, STUCK_US};
use crate::report::{Finding, Kind, Limits, Note, Report};

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

// The keys a note names, each with whether its evidence can lie outside that key's own rounds.
fn named(note: &Note) -> [Option<(u16, bool)>; 2] {
    match *note {
        Note::Clean { key, .. }
        | Note::OneExtraDown { key, .. }
        | Note::NotAssessed { key, .. } => [Some((key, false)), None],
        Note::Unprompted { key, .. }
        | Note::HeldAtPause { key, .. }
        | Note::ReleasedWithNextKey { key, .. } => [Some((key, true)), None],
        Note::DifferentCode { asked, got, .. } => [Some((asked, false)), Some((got, true))],
        Note::NoInputFromKeyboard | Note::Systemic { .. } => [None, None],
    }
}

// Keys named for evidence that can lie outside their own rounds: a stuck key, a key held at a
// pause or released with the next key, chatter in free typing, and a key that answered another
// key's prompt. They keep the whole test's numbers, as keys never prompted do.
pub(crate) fn beyond_rounds(findings: &[Finding], notes: &[Note]) -> BTreeSet<u16> {
    let stuck = findings
        .iter()
        .filter(|f| f.kind() == Kind::Stuck)
        .map(|f| f.key);
    let noted = notes
        .iter()
        .flat_map(named)
        .flatten()
        .filter(|&(_, beyond)| beyond)
        .map(|(key, _)| key);
    stuck.chain(noted).collect()
}

pub(crate) fn build(
    f: &Folded,
    chatter: &BTreeMap<u16, Tally>,
    dead: &BTreeMap<u16, DeadTally>,
    rounds: &[Round],
    beyond: &BTreeSet<u16>,
    limits: Limits,
) -> Aggregates {
    let prompted: BTreeSet<u16> = rounds.iter().map(|r| r.key).collect();
    let mut keys: BTreeMap<u16, KeyAggregate> = BTreeMap::new();
    for (&scan, t) in &f.tracks {
        let a = keys.entry(scan).or_default();
        let c = chatter.get(&scan);
        // The app prompts every plain key, and outside its rounds the user may be typing, so a
        // prompted key counts only the presses that began in its own rounds, with their chatter.
        let own = prompted.contains(&scan) && !beyond.contains(&scan);
        let round_of = |i: usize| match (own, c) {
            (false, _) => Some(0),
            (true, Some(c)) => c.round_of[i],
            (true, None) => None,
        };
        let inside = |at: u64| !own || rounds.iter().any(|r| r.key == scan && r.contains(at));
        for (i, e) in t.episodes.iter().enumerate() {
            let Some(round) = round_of(i) else {
                continue;
            };
            a.episodes += 1;
            a.downs += 1 + e.repeats + e.duplicates;
            a.repeats += e.repeats;
            a.duplicates += e.duplicates;
            match e.end {
                End::Up { .. } => a.ups += 1,
                End::Interrupted { .. } => a.interrupted += 1,
                End::Open => {}
            }
            if let Some(h) = f.hold(e) {
                a.hold.add(h.us);
                if h.us >= STUCK_US {
                    a.long_holds += 1;
                }
            }
            if e.end == End::Open && f.end_us.saturating_sub(e.down) >= STUCK_US {
                a.unreleased += 1;
            }
            // A pair within one round, so no interval spans time the user spent on other keys.
            if let Some(next) = t
                .episodes
                .get(i + 1)
                .filter(|n| n.seg == e.seg && round_of(i + 1) == Some(round))
            {
                a.interval.add(next.down.saturating_sub(e.down));
                if let Some(g) = f.gap(e, next) {
                    a.release_gap.add(g.us);
                }
            }
        }
        a.orphan_ups = t.orphan_ups.iter().filter(|&&at| inside(at)).count() as u32;
        a.ups += a.orphan_ups;
        for &(_, up) in t.after_resume.iter().filter(|&&(at, _)| inside(at)) {
            a.after_resume += 1;
            if up {
                a.ups += 1;
            } else {
                a.downs += 1;
            }
        }
        if let Some(c) = c {
            let counts = if own { c.prompted } else { c.all };
            a.presses = counts.presses;
            a.affected = counts.affected;
            a.extra_downs = counts.extra;
            a.borderline = counts.borderline;
            a.timing_unknown = counts.unknown;
        }
    }
    for &key in &prompted {
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
    // One count for the whole test, which names no key.
    let timing_unknown = chatter.values().map(|c| c.all.unknown).sum();
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
    // counts would reveal which keys were typed, so they stay out: a key never prompted is kept
    // only when named, and a prompted key's numbers already hold only its own rounds (see build).
    pub fn saved(&self) -> Aggregates {
        let findings = self.findings.iter().map(|f| f.key);
        let notes = self
            .notes
            .iter()
            .flat_map(named)
            .flatten()
            .map(|(key, _)| key);
        let named: BTreeSet<u16> = findings.chain(notes).collect();
        let mut saved = self.aggregates.clone();
        saved
            .keys
            .retain(|k, a| a.prompted.is_some() || named.contains(k));
        saved
    }
}
