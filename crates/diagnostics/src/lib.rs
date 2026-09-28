mod aggregate;
mod chatter;
mod dead;
mod fold;
mod guide;
mod input;
pub mod keys;
mod normalize;
pub mod params;
mod poll;
mod report;
mod stats;
mod stuck;
mod words;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixture;
#[cfg(test)]
mod tests;

use std::cmp::Reverse;

pub use aggregate::{Aggregates, Histogram, KeyAggregate, PromptTally};
pub use chatter::chatter_confidence;
pub use guide::{Guide, MAX_PRESSES, MAX_ROUNDS, Plan, PlanError, Prompt};
pub use input::{BoardKind, Device, Entry, HeldKey, Round, Session};

pub use report::*;
pub use stats::{rule_of_three_permille, wilson_at_least, wilson_floor_permille};
pub use words::{Label, Lines, code_label, criteria, hedged};

// Bumped whenever a threshold, bin edge or rule changes, so M4 only compares like with like.
pub const RULES: u16 = 2;

pub fn diagnose(session: &Session<'_>) -> Report {
    // A round with no length can't be answered, and would read as silent.
    let rounds: Vec<Round> = session
        .rounds
        .iter()
        .copied()
        .filter(|r| r.start_us < r.end_us)
        .collect();
    let folded = fold::fold(normalize::normalize(session), session.end_us);
    let poll = poll::estimate(&folded);
    let tallies = chatter::tally(&folded, &rounds);
    let mut findings = Vec::new();
    let mut notes = Vec::new();
    chatter::findings(
        &tallies,
        &rounds,
        session.board,
        poll,
        &mut findings,
        &mut notes,
    );
    stuck::findings(
        &folded,
        &rounds,
        session.board,
        poll,
        &mut findings,
        &mut notes,
    );
    let dead = dead::findings(&folded, &rounds, session.board, &mut findings, &mut notes);
    let limits = Limits {
        poll,
        ..folded.limits
    };
    let aggregates = aggregate::build(&folded, &tallies, &dead, &rounds, limits);

    let flagged: Vec<u16> = findings.iter().map(|f| f.key).collect();
    let partner = notes
        .iter()
        .filter_map(|n| match *n {
            Note::Clean { key, presses, .. } if !flagged.contains(&key) => Some((key, presses)),
            _ => None,
        })
        .max_by_key(|&(key, presses)| (keys::is_plain(key), presses, Reverse(key)))
        .map(|(key, _)| key);
    for f in &mut findings {
        for t in &mut f.next_tests {
            if let NextTest::SwapSwitch { partner: p, .. } = t {
                *p = partner;
            }
        }
    }
    findings.sort_by(|a, b| {
        b.confidence
            .cmp(&a.confidence)
            .then(a.key.cmp(&b.key))
            .then(a.kind().cmp(&b.kind()))
    });
    notes.sort();
    notes.dedup();
    Report {
        rules: RULES,
        findings,
        notes,
        aggregates,
    }
}
