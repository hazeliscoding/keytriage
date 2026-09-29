// Dead key: a prompted key that produced nothing, while the keyboard and the user were there.
use std::collections::BTreeMap;

use crate::fold::Folded;
use crate::input::{BoardKind, Round};
use crate::params::{RETEST_PRESSES, RETEST_ROUNDS, SILENT_MIN_ASKED};
use crate::report::*;

#[derive(Clone, Copy, Default)]
pub(crate) struct DeadTally {
    pub assessed: u16,
    pub silent: u16,
    pub not_assessed: u16,
}

fn downs_in(f: &Folded, scan: u16, r: &Round) -> u32 {
    f.tracks.get(&scan).map_or(0, |t| {
        t.downs.iter().filter(|&&d| r.contains(d)).count() as u32
    })
}

// Whether two or more other keys were down together for more than half the round, when the
// keyboard's rollover limit could have blocked the prompted key by design. A brief overlap, such as
// Shift with a letter or a rolled pair, can't have blocked a whole round of presses.
fn crowded(f: &Folded, key: u16, r: &Round) -> bool {
    let mut edges: Vec<(u64, i32)> = Vec::new();
    for (&scan, t) in &f.tracks {
        if scan == key || crate::keys::never_released(scan) {
            continue;
        }
        for e in &t.episodes {
            let (from, to) = (e.down.max(r.start_us), e.until(f.end_us).min(r.end_us));
            if from < to {
                edges.push((from, 1));
                edges.push((to, -1));
            }
        }
    }
    edges.sort();
    let (mut held, mut since, mut crowded_us) = (0, 0, 0u64);
    for &(at, d) in &edges {
        if held >= 2 {
            crowded_us += at - since;
        }
        held += d;
        since = at;
    }
    crowded_us * 2 > r.end_us.saturating_sub(r.start_us)
}

fn held_at(f: &Folded, key: u16, at: u64) -> bool {
    f.tracks.get(&key).is_some_and(|t| {
        t.episodes
            .iter()
            .any(|e| e.down < at && e.until(f.end_us) > at)
    })
}

pub(crate) fn findings(
    f: &Folded,
    rounds: &[Round],
    board: BoardKind,
    findings: &mut Vec<Finding>,
    notes: &mut Vec<Note>,
) -> BTreeMap<u16, DeadTally> {
    let mut tallies = BTreeMap::new();
    let answered = |r: &Round| downs_in(f, r.key, r) > 0;
    let mut prompted: Vec<u16> = rounds.iter().map(|r| r.key).collect();
    prompted.sort();
    prompted.dedup();
    let mut no_input = false;
    for key in prompted {
        let mut tally = DeadTally::default();
        let mut voids: BTreeMap<Why, u16> = BTreeMap::new();
        let mut different: BTreeMap<u16, u16> = BTreeMap::new();
        let mut silent: Vec<&Round> = Vec::new();
        for r in rounds
            .iter()
            .filter(|r| r.key == key && r.asked >= SILENT_MIN_ASKED)
        {
            if answered(r) {
                tally.assessed += 1;
                continue;
            }
            let why = if f.paused_within(r.start_us, r.end_us) * 2
                > r.end_us.saturating_sub(r.start_us)
            {
                Some(Why::Paused)
            } else if held_at(f, key, r.start_us) {
                Some(Why::HeldDown)
            } else if f.foreign.iter().any(|&(s, t)| s == key && r.contains(t)) {
                Some(Why::OtherKeyboard)
            } else if f.injected.iter().any(|&t| r.contains(t)) {
                Some(Why::Injected)
            } else if f.overruns.iter().any(|&t| r.contains(t)) || crowded(f, key, r) {
                Some(Why::Blocked)
            } else {
                None
            };
            if let Some(why) = why {
                *voids.entry(why).or_default() += 1;
                tally.not_assessed += 1;
                continue;
            }
            let mut presses: Vec<(u32, u16)> = f
                .tracks
                .iter()
                .filter(|(s, _)| **s != key)
                .map(|(&s, t)| {
                    (
                        t.episodes.iter().filter(|e| r.contains(e.down)).count() as u32,
                        s,
                    )
                })
                .filter(|&(n, _)| n > 0)
                .collect();
            presses.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            let need = 2u32.max(u32::from(r.asked) / 2);
            if let Some(&(n, s)) = presses.first()
                && n >= need
                && presses.get(1).is_none_or(|&(m, _)| m < n)
            {
                *different.entry(s).or_default() += 1;
                tally.not_assessed += 1;
                continue;
            }
            tally.assessed += 1;
            tally.silent += 1;
            silent.push(r);
        }
        for (why, rounds) in voids {
            notes.push(Note::NotAssessed { key, rounds, why });
        }
        if let Some((&got, &n)) = different
            .iter()
            .max_by_key(|(s, n)| (**n, std::cmp::Reverse(**s)))
        {
            notes.push(Note::DifferentCode {
                asked: key,
                got,
                rounds: n,
            });
        }
        tallies.insert(key, tally);
        if silent.is_empty() {
            continue;
        }
        let controls: Vec<&Round> = rounds
            .iter()
            .filter(|r| r.key != key && answered(r))
            .collect();
        let live = !controls.is_empty()
            || f.tracks
                .iter()
                .any(|(&s, t)| s != key && !t.downs.is_empty());
        if !live {
            no_input = true;
            continue;
        }
        let bracketed = silent.iter().all(|s| {
            controls.iter().any(|c| c.end_us <= s.start_us)
                && controls.iter().any(|c| c.start_us >= s.end_us)
        });
        let (s, all) = (tally.silent, tally.assessed);
        let control = !controls.is_empty();
        let confidence = if s == all && s >= 3 && bracketed {
            Confidence::VeryHigh
        } else if s == all && s >= 2 && control {
            Confidence::High
        } else if (s == all && s >= 2) || (s >= 2 && control) || (s == all && bracketed) {
            Confidence::Medium
        } else {
            Confidence::Low
        };
        let meanwhile = f
            .tracks
            .iter()
            .filter(|(sc, _)| **sc != key)
            .flat_map(|(_, t)| &t.episodes)
            .filter(|e| silent.iter().any(|r| r.contains(e.down)))
            .count() as u32;
        let evidence = DeadEvidence {
            rounds: all,
            silent_rounds: s,
            asked_in_silent: silent.iter().map(|r| u32::from(r.asked)).sum(),
            control_rounds: controls.len() as u16,
            bracketed,
            other_presses_meanwhile: meanwhile,
        };
        let (causes, mut next) = match board {
            BoardKind::HotSwap => (
                vec![
                    Cause::SwitchSeating,
                    Cause::HotSwapSocket,
                    Cause::SwitchContacts,
                    Cause::KeymapOrRemap,
                ],
                vec![
                    NextTest::ReseatSwitch { key },
                    NextTest::BridgeSocket { key },
                    NextTest::SwapSwitch {
                        suspect: key,
                        partner: Partner::Unnamed,
                    },
                    NextTest::CheckKeymap { key },
                ],
            ),
            BoardKind::Soldered => (
                vec![
                    Cause::SolderJoint,
                    Cause::SwitchContacts,
                    Cause::DiodeOrTrace,
                    Cause::KeymapOrRemap,
                ],
                vec![
                    NextTest::CheckKeymap { key },
                    NextTest::InspectSolderJoint { key },
                ],
            ),
            BoardKind::Laptop => (
                vec![
                    Cause::DebrisOrResidue,
                    Cause::DomeOrScissor,
                    Cause::KeymapOrRemap,
                ],
                vec![
                    NextTest::InspectUnderKeycap { key },
                    NextTest::CheckKeymap { key },
                ],
            ),
            BoardKind::Unknown => (
                vec![
                    Cause::SwitchContacts,
                    Cause::SocketOrSolderJoint,
                    Cause::DiodeOrTrace,
                    Cause::KeymapOrRemap,
                ],
                vec![NextTest::AskBoardKind, NextTest::CheckKeymap { key }],
            ),
        };
        if confidence < Confidence::High {
            next.insert(
                0,
                NextTest::TestAgain {
                    key,
                    rounds: RETEST_ROUNDS,
                    presses: RETEST_PRESSES,
                },
            );
        }
        findings.push(Finding {
            key,
            confidence,
            evidence: Evidence::Dead(evidence),
            causes,
            next_tests: next,
        });
    }
    if no_input {
        notes.push(Note::NoInputFromKeyboard);
    }
    tallies
}
