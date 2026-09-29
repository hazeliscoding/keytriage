// The swap test: the suspect switch and its partner's trade sockets, and both keys are tested
// again. The judgment reads the retest's own report and the kept offer, which hold no times or
// order, and every word comes from words.rs.
use crate::guide::Plan;
use crate::input::BoardKind;
use crate::params::{SWAP_PRESSES, SWAP_ROUNDS, SWAP_UNCLEARED, SWAP_UNTESTED};
use crate::report::*;
use crate::stats::rule_of_three_permille;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Swap {
    pub suspect: u16,
    pub partner: u16,
    pub kind: Kind,
    pub before: Confidence,
    // The offered chatter finding's rate floor, and 0 for dead and stuck.
    pub floor_permille: u16,
    pub partner_untested: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Follows,
    Stays,
    Both,
    Gone,
    Unclear,
}

// Why a key without a finding still isn't cleared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gap {
    Untested,
    TooFew,
    OneExtraDown,
    // Extra key-downs that a keyboard reporting every 16 ms or slower can't tell from fast presses.
    CoarseTiming,
    NotAssessed,
    HeldLong,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Shows(Finding),
    Clear,
    Short(Gap),
}

// One key after the swap. `rounds` counts the rounds that are evidence for the swapped kind: rounds
// with a press for chatter and stuck, rounds with a key-down for dead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Side {
    pub key: u16,
    pub status: Status,
    pub presses: u32,
    pub rounds: u16,
    pub bound_permille: u16,
    pub not_assessed: u16,
    pub also: Vec<Finding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwapResult {
    pub swap: Swap,
    pub outcome: Outcome,
    pub confidence: Option<Confidence>,
    pub capped: bool,
    pub suspect: Side,
    pub partner: Side,
}

impl Swap {
    pub fn offer(report: &Report, board: BoardKind) -> Option<Swap> {
        // The chatter detector names a swap on an unknown board too, where the switches may not
        // pull out.
        if board != BoardKind::HotSwap {
            return None;
        }
        // The stuck detector reads every key, and the retest prompts the suspect. A key the main
        // test never prompted may act on the OS or the page, or sit on a stabilizer.
        let prompted = |key: u16| {
            report
                .aggregates
                .keys
                .get(&key)
                .is_some_and(|a| a.prompted.is_some())
        };
        let lent = |partner: Partner| match partner {
            Partner::Clean(key) => Some((key, false)),
            Partner::Untested(key) => Some((key, true)),
            Partner::Unnamed => None,
        };
        report.findings.iter().find_map(|f| {
            f.next_tests.iter().find_map(|t| match *t {
                NextTest::SwapSwitch { suspect, partner }
                    if suspect == f.key && prompted(suspect) =>
                {
                    lent(partner).map(|(partner, partner_untested)| Swap {
                        suspect,
                        partner,
                        kind: f.kind(),
                        before: f.confidence,
                        floor_permille: match f.evidence {
                            Evidence::Chatter(e) => e.rate_floor_permille,
                            Evidence::Dead(_) | Evidence::Stuck(_) => 0,
                        },
                        partner_untested,
                    })
                }
                _ => None,
            })
        })
    }

    pub fn plan(&self) -> Plan {
        Plan {
            keys: vec![self.suspect, self.partner],
            rounds: SWAP_ROUNDS,
            presses: SWAP_PRESSES,
        }
    }

    pub fn judge(&self, after: &Report) -> SwapResult {
        let suspect = self.side(after, self.suspect);
        let partner = self.side(after, self.partner);
        let shows = |s: &Side| match &s.status {
            Status::Shows(f) => Some(f.confidence),
            Status::Clear | Status::Short(_) => None,
        };
        let short = |s: &Side| matches!(s.status, Status::Short(_));
        // No surer than the test that raised the suspicion or the side that shows the fault now.
        // When the other side can't be cleared, the switch and the socket both stay possible.
        let located = |shown: Confidence, other: &Side| {
            let level = self.before.min(shown);
            if short(other) {
                (Some(level.min(SWAP_UNCLEARED)), true)
            } else {
                (Some(level), false)
            }
        };
        let (outcome, (mut confidence, capped)) = match (shows(&suspect), shows(&partner)) {
            (Some(a), Some(b)) => (Outcome::Both, (Some(self.before.min(a).min(b)), false)),
            (None, Some(b)) => (Outcome::Follows, located(b, &suspect)),
            (Some(a), None) => (Outcome::Stays, located(a, &partner)),
            // Neither key showing it fits a reseated contact and a fault that comes and goes
            // alike, so it is only Low.
            (None, None) if suspect.status == Status::Clear && partner.status == Status::Clear => {
                (Outcome::Gone, (Some(Confidence::Low), false))
            }
            // Too little evidence to rank carries no confidence, as a test with nothing tested.
            (None, None) => (Outcome::Unclear, (None, false)),
        };
        // A partner the first test never checked may carry a fault of its own.
        if self.partner_untested {
            confidence = confidence.map(|c| c.min(SWAP_UNTESTED));
        }
        SwapResult {
            swap: *self,
            outcome,
            confidence,
            capped,
            suspect,
            partner,
        }
    }

    fn side(&self, after: &Report, key: u16) -> Side {
        // The swap moves one switch to test one finding. Another kind on either key is a separate
        // fault with its own causes, so it is listed and doesn't decide.
        let (swapped, also): (Vec<&Finding>, Vec<&Finding>) = after
            .findings
            .iter()
            .filter(|f| f.key == key)
            .partition(|f| f.kind() == self.kind);
        let aggregate = after.aggregates.keys.get(&key).copied().unwrap_or_default();
        let tally = aggregate.prompted.unwrap_or_default();
        let answered = tally.assessed.saturating_sub(tally.silent);
        // A clean key clears its side only when its presses would very likely have shown the rate
        // the first test proved. Fewer can't tell a better contact from a fault that didn't show.
        let clean = after.notes.iter().any(|n| match *n {
            Note::Clean {
                key: k,
                bound_permille,
                ..
            } => k == key && bound_permille <= self.floor_permille,
            _ => false,
        });
        let held = aggregate.long_holds > 0
            || aggregate.unreleased > 0
            || after.notes.iter().any(|n| match *n {
                Note::HeldAtPause { key: k, .. } | Note::ReleasedWithNextKey { key: k, .. } => {
                    k == key
                }
                _ => false,
            });
        let status = match swapped.first() {
            Some(f) => Status::Shows((*f).clone()),
            None => match self.kind {
                Kind::Chatter if clean => Status::Clear,
                Kind::Chatter if tally.presses == 0 => Status::Short(Gap::Untested),
                Kind::Chatter => Status::Short(match tally.affected {
                    0 => Gap::TooFew,
                    1 => Gap::OneExtraDown,
                    _ => Gap::CoarseTiming,
                }),
                Kind::Dead if answered == 0 => Status::Short(Gap::Untested),
                Kind::Dead if tally.not_assessed > 0 => Status::Short(Gap::NotAssessed),
                Kind::Dead if answered < SWAP_ROUNDS || answered < tally.rounds => {
                    Status::Short(Gap::TooFew)
                }
                Kind::Dead => Status::Clear,
                Kind::Stuck if held => Status::Short(Gap::HeldLong),
                Kind::Stuck if tally.rounds_pressed == 0 => Status::Short(Gap::Untested),
                Kind::Stuck
                    if tally.rounds_pressed < SWAP_ROUNDS
                        || tally.rounds_pressed < tally.rounds =>
                {
                    Status::Short(Gap::TooFew)
                }
                Kind::Stuck => Status::Clear,
            },
        };
        Side {
            key,
            status,
            presses: tally.presses,
            rounds: match self.kind {
                Kind::Dead => answered,
                Kind::Chatter | Kind::Stuck => tally.rounds_pressed,
            },
            bound_permille: rule_of_three_permille(tally.presses),
            not_assessed: tally.not_assessed,
            also: also.into_iter().cloned().collect(),
        }
    }
}

impl SwapResult {
    pub fn tile(&self) -> Option<u16> {
        match self.outcome {
            Outcome::Follows => Some(self.swap.partner),
            Outcome::Stays | Outcome::Both => Some(self.swap.suspect),
            Outcome::Gone | Outcome::Unclear => None,
        }
    }

    // The keys that showed the fault after the swap, suspect first.
    pub fn flagged(&self) -> Vec<u16> {
        [&self.suspect, &self.partner]
            .into_iter()
            .filter(|s| matches!(s.status, Status::Shows(_)))
            .map(|s| s.key)
            .collect()
    }
}
