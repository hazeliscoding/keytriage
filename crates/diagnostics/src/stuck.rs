// Stuck key: a key-down with no key-up, or one held down through its own round, when a finger
// would have let go.
use crate::fold::{End, Folded};
use crate::input::{BoardKind, Round};
use crate::keys;
use crate::params::*;
use crate::report::*;

struct Candidate {
    hold: u64,
    still_down: bool,
    repeats: u32,
    others: u32,
    own: bool,
    busy: bool,
    other_rounds: u16,
}

impl Candidate {
    fn lines(&self) -> u32 {
        u32::from(self.busy) + u32::from(self.own) + u32::from(self.other_rounds > 0)
    }
}

fn overlap(a: (u64, u64), b: (u64, u64)) -> u64 {
    a.1.min(b.1).saturating_sub(a.0.max(b.0))
}

pub(crate) fn findings(
    f: &Folded,
    rounds: &[Round],
    board: BoardKind,
    poll: PollEstimate,
    findings: &mut Vec<Finding>,
    notes: &mut Vec<Note>,
) {
    let answered = |r: &Round| {
        f.tracks
            .get(&r.key)
            .is_some_and(|t| t.downs.iter().any(|&d| r.contains(d)))
    };
    for (&key, track) in &f.tracks {
        if keys::never_released(key) {
            continue;
        }
        let mut candidates = Vec::new();
        let mut at_pause = 0u64;
        let mut rode = Vec::new();
        for e in &track.episodes {
            if let End::Interrupted { at } = e.end {
                at_pause = at_pause.max(at.saturating_sub(e.down));
                continue;
            }
            let until = e.until(f.end_us);
            let hold = until.saturating_sub(e.down);
            if hold < STUCK_US {
                continue;
            }
            if let Some(up) = e.up()
                && e.repeats > 0
                && f.tracks.iter().any(|(&s, t)| {
                    s != key
                        && t.episodes
                            .iter()
                            .any(|o| o.down >= up && o.down - up <= NEXT_KEY_US)
                })
            {
                rode.push(hold);
            }
            let own_overlap: u64 = rounds
                .iter()
                .filter(|r| r.key == key && r.asked >= SILENT_MIN_ASKED)
                .map(|r| overlap((e.down, until), (r.start_us, r.end_us)))
                .sum();
            let own = own_overlap >= STUCK_US;
            let still_down = e.end == End::Open;
            if !still_down && !own {
                continue;
            }
            let others = f
                .tracks
                .iter()
                .filter(|(s, _)| **s != key)
                .flat_map(|(_, t)| &t.episodes)
                .filter(|o| o.down > e.down && o.up().is_some_and(|u| u < until))
                .count() as u32;
            let other_rounds = rounds
                .iter()
                .filter(|r| {
                    r.key != key && r.start_us >= e.down && r.end_us <= until && answered(r)
                })
                .count() as u16;
            candidates.push(Candidate {
                hold,
                still_down,
                repeats: e.repeats,
                others,
                own,
                busy: !keys::is_modifier(key) && others >= BUSY_PRESSES,
                other_rounds,
            });
        }
        // At 8 ms or slower a finger can release one key and press the next within one report,
        // which reads the same as a lost release, so one hold isn't enough there.
        let needed = if poll >= PollEstimate::Ms8 { 2 } else { 1 };
        if rode.len() >= needed {
            notes.extend(rode.iter().map(|&hold| Note::ReleasedWithNextKey {
                key,
                held_ms: (hold / 1000) as u32,
            }));
        }
        if at_pause >= STUCK_US {
            notes.push(Note::HeldAtPause {
                key,
                held_ms: (at_pause / 1000) as u32,
            });
        }
        if candidates.is_empty() {
            continue;
        }
        let owned = candidates.iter().filter(|c| c.own).count();
        let agreeing = candidates.iter().filter(|c| c.lines() > 0).count();
        let most = candidates.iter().map(Candidate::lines).max().unwrap_or(0);
        let confidence = if owned >= 2 {
            Confidence::VeryHigh
        } else if most >= 2 || agreeing >= 2 {
            Confidence::High
        } else if most == 1 {
            Confidence::Medium
        } else {
            Confidence::Low
        };
        // The length and the state describe one hold: the open one when there is one.
        let still_down = candidates.iter().any(|c| c.still_down);
        let held = candidates
            .iter()
            .filter(|c| c.still_down == still_down)
            .map(|c| c.hold)
            .max()
            .unwrap_or(0);
        let evidence = StuckEvidence {
            held_ms: (held / 1000) as u32,
            still_down,
            episodes: candidates.len() as u16,
            repeats: candidates.iter().map(|c| c.repeats).sum(),
            others_completed: candidates.iter().map(|c| c.others).sum(),
            own_prompts: owned as u16,
            other_rounds_answered: candidates.iter().map(|c| c.other_rounds).sum(),
        };
        let causes = match board {
            BoardKind::Laptop => vec![
                Cause::DebrisOrResidue,
                Cause::DomeOrScissor,
                Cause::LostRelease,
                Cause::HostSoftware,
            ],
            _ => vec![
                Cause::StemOrKeycap,
                Cause::SwitchContacts,
                Cause::LostRelease,
                Cause::HostSoftware,
            ],
        };
        let mut next = vec![NextTest::InspectUnderKeycap { key }];
        if board == BoardKind::HotSwap {
            next.push(NextTest::SwapSwitch {
                suspect: key,
                partner: None,
            });
        }
        next.push(NextTest::CheckConnection);
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
            evidence: Evidence::Stuck(evidence),
            causes,
            next_tests: next,
        });
    }
}
