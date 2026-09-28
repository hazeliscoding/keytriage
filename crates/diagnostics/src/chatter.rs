// Chatter: extra transitions that no finger makes. Only one key's own intervals are compared.
use std::collections::{BTreeMap, BTreeSet};

use crate::fold::{Episode, Folded};
use crate::input::{BoardKind, Round};
use crate::keys;
use crate::params::*;
use crate::report::*;
use crate::stats::{rule_of_three_permille, wilson_at_least, wilson_floor_permille};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Link {
    Linked { gap: u64, fragment: Option<u64> },
    Borderline,
    Unknown,
    Apart,
}

pub(crate) fn link(f: &Folded, a: &Episode, b: &Episode, tapped: bool) -> Link {
    let Some(gap) = f.gap(a, b) else {
        return Link::Apart;
    };
    if !gap.countable || gap.us < COALESCED_US {
        return Link::Unknown;
    }
    let holds: Vec<u64> = [f.hold(a), f.hold(b)]
        .into_iter()
        .flatten()
        .filter(|h| h.countable)
        .map(|h| h.us)
        .collect();
    if holds.iter().any(|&h| h < COALESCED_US) {
        return Link::Unknown;
    }
    let shortest = holds.iter().copied().min().filter(|_| !tapped);
    let within_reach = gap.us < REACH_US;
    if gap.us < SHORT_GAP_US || (within_reach && shortest.is_some_and(|s| s < SHORT_HOLD_US)) {
        return Link::Linked {
            gap: gap.us,
            fragment: shortest.filter(|&s| s < SHORT_HOLD_US),
        };
    }
    if gap.us < BORDERLINE_US || (within_reach && shortest.is_some_and(|s| s < BORDERLINE_US)) {
        return Link::Borderline;
    }
    Link::Apart
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Counts {
    pub presses: u32,
    pub affected: u32,
    pub extra: u32,
    pub borderline: u32,
    pub unknown: u32,
}

#[derive(Default)]
pub(crate) struct Tally {
    pub all: Counts,
    pub prompted: Counts,
    pub unprompted: Counts,
    pub rounds_pressed: u16,
    pub rounds_affected: u16,
    pub gap: Option<(u64, u64)>,
    pub fragment: Option<(u64, u64)>,
    pub tapped: bool,
    // For each episode, which of the key's own rounds its press began in. A press's chatter
    // belongs to it, even when it lands after the round has closed.
    pub round_of: Vec<Option<usize>>,
}

fn widen(range: &mut Option<(u64, u64)>, v: u64) {
    *range = Some(match *range {
        Some((lo, hi)) => (lo.min(v), hi.max(v)),
        None => (v, v),
    });
}

// A key whose presses are nearly all shorter than a finger's is being tapped by its firmware
// (tap-hold, macros, laptop hotkeys), so short presses say nothing about its contacts. A chattering
// key still has one finger-length hold per intended press, so heavy chatter never reaches this.
fn tapped(f: &Folded, eps: &[Episode]) -> bool {
    let holds: Vec<u64> = eps
        .iter()
        .filter_map(|e| f.hold(e))
        .filter(|h| h.countable)
        .map(|h| h.us)
        .collect();
    let short = holds.iter().filter(|&&h| h < SHORT_HOLD_US).count();
    holds.len() as u32 >= TAPPED_MIN_HOLDS && short * 10 >= holds.len() * 9
}

pub(crate) fn tally(f: &Folded, rounds: &[Round]) -> BTreeMap<u16, Tally> {
    let mut out = BTreeMap::new();
    for (&scan, track) in &f.tracks {
        if keys::never_released(scan) {
            continue;
        }
        let own: Vec<&Round> = rounds.iter().filter(|r| r.key == scan).collect();
        let prompted = |at: u64| own.iter().any(|r| r.contains(at));
        let eps = &track.episodes;
        let mut t = Tally {
            tapped: tapped(f, eps),
            round_of: vec![None; eps.len()],
            ..Tally::default()
        };
        let mut links: Vec<Link> = eps
            .windows(2)
            .map(|w| link(f, &w[0], &w[1], t.tapped))
            .collect();
        // A cluster is one intended press. A short phantom within reach of two finger-length
        // presses joins only the nearer one, so its reach link to the farther press is cut. Links
        // under the short gap stay, which keeps a dropout in the middle of a hold in one press.
        let long = |e: &Episode| {
            f.hold(e)
                .filter(|h| h.countable)
                .is_none_or(|h| h.us >= SHORT_HOLD_US)
        };
        let (mut seen_long, mut widest) = (false, None::<(usize, u64)>);
        for j in 0..eps.len() {
            if j > 0 {
                match links[j - 1] {
                    Link::Linked { gap, .. } => {
                        if gap >= SHORT_GAP_US && widest.is_none_or(|(_, w)| gap > w) {
                            widest = Some((j - 1, gap));
                        }
                    }
                    _ => {
                        seen_long = false;
                        widest = None;
                    }
                }
            }
            if long(&eps[j]) {
                if seen_long && let Some((cut, _)) = widest {
                    links[cut] = Link::Apart;
                }
                seen_long = true;
                widest = None;
            }
        }
        let mut pressed = vec![false; own.len()];
        let mut affected = vec![false; own.len()];
        let mut i = 0;
        while i < eps.len() {
            let (first, start) = (i, eps[i].down);
            let mut size = 1u32;
            let (mut gap, mut fragment) = (None, None);
            while i + 1 < eps.len() {
                let a = &eps[i];
                let counts = if prompted(a.down) {
                    &mut t.prompted
                } else {
                    &mut t.unprompted
                };
                match links[i] {
                    Link::Linked {
                        gap: g,
                        fragment: fr,
                    } => {
                        widen(&mut gap, g);
                        if let Some(fr) = fr {
                            widen(&mut fragment, fr);
                        }
                        size += 1;
                        i += 1;
                    }
                    Link::Borderline => {
                        counts.borderline += 1;
                        t.all.borderline += 1;
                        break;
                    }
                    Link::Unknown => {
                        counts.unknown += 1;
                        t.all.unknown += 1;
                        break;
                    }
                    Link::Apart => break,
                }
            }
            let hit = size > 1;
            t.round_of[first..=i].fill(own.iter().position(|r| r.contains(start)));
            for counts in [
                &mut t.all,
                if prompted(start) {
                    &mut t.prompted
                } else {
                    &mut t.unprompted
                },
            ] {
                counts.presses += 1;
                counts.extra += size - 1;
                counts.affected += u32::from(hit);
            }
            if prompted(start) {
                if hit {
                    if let Some((lo, hi)) = gap {
                        widen(&mut t.gap, lo);
                        widen(&mut t.gap, hi);
                    }
                    if let Some((lo, hi)) = fragment {
                        widen(&mut t.fragment, lo);
                        widen(&mut t.fragment, hi);
                    }
                }
                for (j, r) in own.iter().enumerate() {
                    if r.contains(start) {
                        pressed[j] = true;
                        affected[j] |= hit;
                    }
                }
            }
            i += 1;
        }
        t.rounds_pressed = pressed.iter().filter(|&&p| p).count() as u16;
        t.rounds_affected = affected.iter().filter(|&&a| a).count() as u16;
        out.insert(scan, t);
    }
    out
}

pub fn chatter_confidence(k: u32, n: u32, with: u16, of: u16) -> Option<Confidence> {
    if k < 2 {
        return None;
    }
    Some(
        if k >= 5 && of >= 3 && with == of && wilson_at_least(k, n, 50, 1000) {
            Confidence::VeryHigh
        } else if k >= 3 && with >= 2 && wilson_at_least(k, n, 20, 1000) {
            Confidence::High
        } else if (k >= 3 && wilson_at_least(k, n, 10, 1000)) || with >= 2 {
            Confidence::Medium
        } else {
            Confidence::Low
        },
    )
}

fn ms(us: u64) -> u32 {
    u32::try_from(us.saturating_add(500) / 1000).unwrap_or(u32::MAX)
}

fn span(range: Option<(u64, u64)>) -> Option<SpanMs> {
    range.map(|(lo, hi)| SpanMs {
        min_ms: ms(lo),
        max_ms: ms(hi),
    })
}

pub(crate) fn findings(
    tallies: &BTreeMap<u16, Tally>,
    rounds: &[Round],
    board: BoardKind,
    poll: PollEstimate,
    findings: &mut Vec<Finding>,
    notes: &mut Vec<Note>,
) {
    let prompted: BTreeSet<u16> = rounds.iter().map(|r| r.key).collect();
    let tested: BTreeSet<u16> = prompted
        .iter()
        .copied()
        .filter(|s| {
            tallies
                .get(s)
                .is_some_and(|t| t.prompted.presses >= TESTED_PRESSES)
        })
        .collect();
    // At 16 ms or slower a 30 ms tap reads as one poll, the same as a phantom. A key whose near
    // misses outnumber its hits is a fast tapper there, not evidence.
    let ambiguous = |t: &Tally| {
        poll == PollEstimate::Ms16OrSlower && t.prompted.borderline >= t.prompted.affected
    };
    let chattering: BTreeSet<u16> = prompted
        .iter()
        .copied()
        .filter(|s| {
            tallies
                .get(s)
                .is_some_and(|t| t.prompted.affected >= 2 && !ambiguous(t))
        })
        .collect();
    let systemic = chattering.len() >= SYSTEMIC_KEYS;
    if systemic {
        notes.push(Note::Systemic {
            keys: chattering.len() as u16,
        });
    }
    for (&key, t) in tallies {
        let p = t.prompted;
        if prompted.contains(&key) {
            match p.affected {
                0 if p.presses >= TESTED_PRESSES => notes.push(Note::Clean {
                    key,
                    presses: p.presses,
                    bound_permille: rule_of_three_permille(p.presses),
                }),
                1 => notes.push(Note::OneExtraDown {
                    key,
                    presses: p.presses,
                    extra_downs: p.extra,
                }),
                _ => {}
            }
        }
        if t.unprompted.extra >= 2 {
            notes.push(Note::Unprompted {
                key,
                affected: t.unprompted.affected,
                presses: t.unprompted.presses,
            });
        }
        let Some(mut confidence) =
            chatter_confidence(p.affected, p.presses, t.rounds_affected, t.rounds_pressed)
        else {
            continue;
        };
        if !prompted.contains(&key) || ambiguous(t) {
            continue;
        }
        let cap = if poll == PollEstimate::Ms16OrSlower {
            Some(Cap::CoarsePolling)
        } else if systemic {
            Some(Cap::Systemic)
        } else {
            None
        };
        if cap.is_some() {
            confidence = confidence.min(Confidence::Medium);
        }
        // Keys that chatter count as tested, so this line agrees with the systemic one.
        let others: BTreeSet<u16> = tested
            .union(&chattering)
            .copied()
            .filter(|&s| s != key)
            .collect();
        let evidence = ChatterEvidence {
            presses: p.presses,
            affected: p.affected,
            extra_downs: p.extra,
            rate_floor_permille: wilson_floor_permille(p.affected, p.presses),
            rounds: t.rounds_pressed,
            rounds_affected: t.rounds_affected,
            gap: span(t.gap),
            fragment: span(t.fragment),
            other_tested: others.len() as u16,
            other_affected: others.intersection(&chattering).count() as u16,
            borderline: p.borderline,
            timing_unknown: p.unknown,
            poll,
            cap,
        };
        let (mut causes, mut next) = match board {
            BoardKind::HotSwap => (
                vec![
                    Cause::SwitchContacts,
                    Cause::HotSwapSocket,
                    Cause::FirmwareDebounce,
                ],
                vec![
                    NextTest::SwapSwitch {
                        suspect: key,
                        partner: None,
                    },
                    NextTest::CleanContacts { key },
                    NextTest::RaiseDebounce,
                ],
            ),
            BoardKind::Soldered => (
                vec![
                    Cause::SwitchContacts,
                    Cause::SolderJoint,
                    Cause::FirmwareDebounce,
                ],
                vec![
                    NextTest::CleanContacts { key },
                    NextTest::RaiseDebounce,
                    NextTest::InspectSolderJoint { key },
                ],
            ),
            BoardKind::Laptop => (
                vec![
                    Cause::DebrisOrResidue,
                    Cause::DomeOrScissor,
                    Cause::FirmwareDebounce,
                ],
                vec![NextTest::InspectUnderKeycap { key }],
            ),
            BoardKind::Unknown => (
                vec![
                    Cause::SwitchContacts,
                    Cause::SocketOrSolderJoint,
                    Cause::FirmwareDebounce,
                ],
                vec![
                    NextTest::AskBoardKind,
                    NextTest::SwapSwitch {
                        suspect: key,
                        partner: None,
                    },
                ],
            ),
        };
        if systemic {
            causes.retain(|&c| c != Cause::FirmwareDebounce);
            causes.insert(0, Cause::FirmwareDebounce);
            next.retain(|&n| n != NextTest::RaiseDebounce);
            next.insert(0, NextTest::RaiseDebounce);
        }
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
            evidence: Evidence::Chatter(evidence),
            causes,
            next_tests: next,
        });
    }
}
