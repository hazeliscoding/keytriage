use std::collections::BTreeMap;

use super::*;
use crate::fixture::{Fixture, guided, guided_chatter, label};
use crate::params::PROMPT_MERGE_US;

const H: u16 = 0x23;

fn plan(keys: &[u16], rounds: u16, presses: u16) -> Plan {
    Plan {
        keys: keys.to_vec(),
        rounds,
        presses,
    }
}

// A Guide and the stream it has read so far, fed as the capture callback feeds it.
struct Run {
    guide: Guide,
    s: Synth,
    fed: usize,
    changed: Vec<bool>,
}

impl Run {
    fn new(keys: &[u16], rounds: u16, presses: u16) -> Run {
        Run::on(&[1], keys, rounds, presses)
    }

    fn on(keyboard: &[Device], keys: &[u16], rounds: u16, presses: u16) -> Run {
        Run {
            guide: Guide::new(plan(keys, rounds, presses), keyboard).unwrap(),
            s: Synth::new().keyboard(keyboard),
            fed: 0,
            changed: Vec::new(),
        }
    }

    fn then(mut self, f: impl FnOnce(Synth) -> Synth) -> Run {
        self.s = f(self.s);
        self.changed.clear();
        for e in &self.s.entries()[self.fed..] {
            self.changed.push(self.guide.entry(e));
        }
        self.fed = self.s.entries().len();
        self
    }

    fn prompt(&self) -> Prompt {
        self.guide.prompt().expect("a prompt")
    }

    fn count(&self) -> u16 {
        self.prompt().count
    }

    fn tally(&self, key: u16) -> u32 {
        self.guide.tallies().get(&key).copied().unwrap_or(0)
    }

    fn now(&self) -> u64 {
        self.s.now()
    }

    fn finish(mut self) -> (Vec<Round>, Fixture) {
        let end = self.s.now() + ms(200);
        let rounds = self.guide.finish(end);
        let mut f = self.s.end_now();
        f.end_us = end;
        f.rounds = rounds.clone();
        (rounds, f)
    }
}

fn answer(s: Synth, key: u16, n: u32) -> Synth {
    (0..n).fold(s, |s, _| normal(s, key))
}

// ---- counting ----

#[test]
fn g01_a_round_closes_at_the_release_of_its_last_counted_press() {
    let run = Run::new(&[E, G], 1, 10)
        .then(|s| answer(s, E, 9))
        .then(|s| s.down(E));
    assert_eq!((run.prompt().key, run.count()), (E, 10));
    let run = run.then(|s| s.wait(ms(90)).up(E));
    let release = run.now();
    assert_eq!(run.changed, [true]);
    assert_eq!((run.prompt().key, run.count()), (G, 0));
    let (rounds, _) = run.then(|s| answer(s.wait(ms(600)), G, 10)).finish();
    assert_eq!(
        rounds[0],
        Round {
            key: E,
            asked: 10,
            start_us: 0,
            end_us: release + 1
        }
    );
    assert_eq!(rounds[1].start_us, release + 1);
    assert_eq!(rounds.len(), 2);
}

#[test]
fn g02_a_key_down_close_to_a_release_adds_to_the_tally_only() {
    let run = Run::new(&[E], 1, 10)
        .then(|s| s.fragments(E, &[ms(80), ms(5), ms(100)]))
        .then(|s| s.wait(PROMPT_MERGE_US - 1).press(E, ms(60)));
    assert_eq!((run.count(), run.tally(E)), (1, 3));
    assert_eq!(run.changed, [true, false]);
    let run = run.then(|s| s.wait(PROMPT_MERGE_US).press(E, ms(60)));
    assert_eq!((run.count(), run.tally(E)), (2, 4));
    let run = run.then(|s| {
        s.wait(ms(200))
            .taps(E, 2, (ms(40), ms(100)), (ms(40), ms(80)))
    });
    assert_eq!((run.count(), run.tally(E)), (3, 6));
}

#[test]
fn g03_autorepeat_and_duplicates_count_toward_neither() {
    let run = Run::new(&[E], 1, 10)
        .then(|s| s.hold(E, ms(2_000), ms(500), ms(33)))
        .then(|s| {
            s.wait(ms(300))
                .down(E)
                .wait(ms(5))
                .down(E)
                .wait(ms(80))
                .up(E)
        });
    assert_eq!((run.count(), run.tally(E)), (2, 2));
}

#[test]
fn g04_injected_input_and_other_keyboards_never_count() {
    let run = Run::on(&[1, 2], &[E], 1, 10)
        .then(|s| s.on(0).press(E, ms(80)).wait(ms(300)))
        .then(|s| s.on(3).press(E, ms(80)).wait(ms(300)));
    assert_eq!((run.count(), run.tally(E)), (0, 0));
    assert!(run.changed.iter().all(|c| !c));
    // An injected key-down leaves nothing held, so the next real press still counts.
    let run = run
        .then(|s| s.on(0).down(E).wait(ms(300)))
        .then(|s| s.on(2).press(E, ms(80)).on(1));
    assert_eq!((run.count(), run.tally(E)), (1, 1));
    assert_eq!(
        Guide::new(plan(&[E], 1, 10), &[1, 0]).unwrap_err(),
        PlanError::ZeroHandle
    );
}

// ---- pause, skip and end ----

#[test]
fn g05_a_pause_drops_the_open_round() {
    let run = Run::new(&[E, G], 1, 10)
        .then(|s| answer(s, E, 4).fragments(E, &[ms(80), ms(5), ms(80)]))
        .then(|s| s.wait(ms(100)).pause(&[]));
    assert_eq!(run.changed, [true]);
    assert_eq!((run.prompt().key, run.count(), run.tally(E)), (E, 0, 0));
    assert!(run.guide.paused());
    let run = run.then(|s| s.wait(ms(3_000)).resume());
    let resumed = run.now();
    assert_eq!(run.changed, [false]);
    assert_eq!(
        run.prompt(),
        Prompt {
            key: E,
            count: 0,
            round: 0,
            index: 0
        }
    );
    let run = run.then(|s| answer(s, E, 3));
    assert_eq!((run.count(), run.tally(E)), (3, 3));
    let (rounds, _) = run
        .then(|s| answer(s, E, 7))
        .then(|s| answer(s.wait(ms(600)), G, 10))
        .finish();
    assert_eq!(rounds.len(), 2);
    assert_eq!((rounds[0].key, rounds[0].start_us), (E, resumed));
}

#[test]
fn g06_a_key_down_at_the_pause_stays_held_until_its_release() {
    let run = Run::new(&[E], 1, 10)
        .then(|s| s.down(E).wait(ms(50)).pause(&[E]).wait(ms(500)).resume())
        .then(|s| s.repeats(E, ms(500), ms(33), 5));
    assert_eq!((run.count(), run.tally(E)), (0, 0));
    let run = run.then(|s| s.wait(ms(20)).up(E).wait(ms(300)).press(E, ms(80)));
    assert_eq!((run.count(), run.tally(E)), (1, 1));

    // Released while the app was away: only the first press after the resume is lost.
    let run = Run::new(&[E], 1, 10)
        .then(|s| s.down(E).wait(ms(50)).pause(&[E]).wait(ms(500)).resume())
        .then(|s| s.wait(ms(400)).press(E, ms(80)));
    assert_eq!((run.count(), run.tally(E)), (0, 0));
    let run = run.then(|s| s.wait(ms(300)).press(E, ms(80)));
    assert_eq!((run.count(), run.tally(E)), (1, 1));

    // Keys on other keyboards are not held for this one.
    let run = Run::new(&[E], 1, 10)
        .then(|s| {
            s.raw(Entry::Paused {
                micros: 0,
                interrupted: vec![HeldKey { device: 7, scan: E }],
            })
            .wait(ms(500))
            .resume()
        })
        .then(|s| s.press(E, ms(80)));
    assert_eq!(run.count(), 1);
}

#[test]
fn g07_skip_keeps_the_round_while_running_and_only_moves_on_while_paused() {
    let mut run = Run::new(&[E, G, J], 1, 10).then(|s| answer(s, E, 3));
    let skipped = run.now();
    assert!(run.guide.skip(skipped));
    assert_eq!((run.prompt().key, run.count(), run.tally(E)), (G, 0, 3));
    let mut run = run.then(|s| s.wait(ms(800)).pause(&[]));
    let at = run.now();
    assert!(run.guide.skip(at));
    assert_eq!((run.prompt().key, run.prompt().index), (J, 2));
    assert!(run.guide.paused());
    let run = run.then(|s| s.wait(ms(2_000)).resume());
    let resumed = run.now();
    let (rounds, _) = run.then(|s| answer(s, J, 10)).finish();
    assert_eq!(
        rounds[0],
        Round {
            key: E,
            asked: 10,
            start_us: 0,
            end_us: skipped
        }
    );
    assert_eq!((rounds[1].key, rounds[1].start_us), (J, resumed));
    assert_eq!(rounds.len(), 2);
}

#[test]
fn g08_ending_or_pausing_before_a_press_leaves_no_silent_round() {
    let asked_then = |end: fn(Synth) -> Synth| {
        let run = Run::new(&[G, E], 1, 10).then(|s| answer(s, G, 10));
        assert_eq!(run.prompt().key, E);
        run.then(|s| end(s.wait(ms(4_000))))
    };
    let ends: [fn(Synth) -> Synth; 2] = [|s| s, |s| s.pause(&[]).wait(ms(1_000))];
    for end in ends {
        let (rounds, f) = asked_then(end).finish();
        assert_eq!(rounds.len(), 1);
        assert_clean(&f.diagnose());
    }
    // Positive control: the same wait kept as a round is a silent one.
    let mut run = asked_then(|s| s);
    let at = run.now();
    assert!(run.guide.skip(at));
    let (rounds, f) = run.finish();
    assert_eq!(rounds.len(), 2);
    only(&f.diagnose(), Kind::Dead, E);

    // An open round that counted a press is kept, closed at the end.
    let run = Run::new(&[G, E], 1, 10)
        .then(|s| answer(s, G, 10))
        .then(|s| s.wait(ms(600)).press(E, ms(90)).wait(ms(3_000)));
    let end = run.now() + ms(200);
    let (rounds, _) = run.finish();
    assert_eq!(
        (rounds[1].key, rounds[1].asked, rounds[1].end_us),
        (E, 10, end)
    );
}

#[test]
fn g09_a_test_that_begins_paused_opens_its_first_round_at_the_resume() {
    let run = Run::new(&[E], 1, 3).then(|s| s.pause(&[]).wait(ms(2_000)).resume());
    assert_eq!(run.changed, [false, false]);
    let (rounds, _) = run.then(|s| answer(s, E, 3)).finish();
    assert_eq!(rounds[0].start_us, ms(2_000));
}

#[test]
fn g10_progress_and_the_end_of_the_plan() {
    let run = Run::new(&[E, G], 2, 3);
    assert_eq!((run.guide.done(), run.guide.total()), (0, 12));
    let run = run.then(|s| answer(s, E, 2));
    assert_eq!(
        run.prompt(),
        Prompt {
            key: E,
            count: 2,
            round: 0,
            index: 0
        }
    );
    assert_eq!(run.guide.done(), 2);
    let run = run.then(|s| answer(s, E, 1));
    assert_eq!(
        run.prompt(),
        Prompt {
            key: G,
            count: 0,
            round: 0,
            index: 1
        }
    );
    assert_eq!(run.guide.done(), 3);
    let run = run.then(|s| answer(answer(s, G, 3), E, 3));
    assert_eq!((run.prompt().key, run.prompt().round), (G, 1));
    assert_eq!(run.guide.done(), 9);
    let run = run.then(|s| answer(s, G, 3));
    assert_eq!(run.guide.prompt(), None);
    assert_eq!(run.guide.done(), 12);
    let tallies = run.guide.tallies().clone();
    assert_eq!(tallies, BTreeMap::from([(E, 6), (G, 6)]));
    let mut run = run.then(|s| s.press(G, ms(5)).wait(ms(5)).press(G, ms(80)));
    assert!(run.changed.iter().all(|c| !c));
    let at = run.now();
    assert!(!run.guide.skip(at));
    assert_eq!(run.guide.tallies(), &tallies);
    assert_eq!(run.finish().0.len(), 4);
}

#[test]
fn g11_plans_that_cannot_run_are_refused() {
    let bad = |keys: &[u16], rounds, presses, keyboard: &[Device]| {
        Guide::new(plan(keys, rounds, presses), keyboard).unwrap_err()
    };
    assert_eq!(bad(&[], 3, 10, &[1]), PlanError::NoKeys);
    assert_eq!(bad(&[E, 0], 3, 10, &[1]), PlanError::BadKey);
    assert_eq!(bad(&[crate::keys::PAUSE], 3, 10, &[1]), PlanError::BadKey);
    assert_eq!(bad(&[E, G, E], 3, 10, &[1]), PlanError::RepeatedKey);
    assert_eq!(bad(&[E], 0, 10, &[1]), PlanError::Rounds);
    assert_eq!(bad(&[E], MAX_ROUNDS + 1, 10, &[1]), PlanError::Rounds);
    assert_eq!(bad(&[E], 3, 0, &[1]), PlanError::Presses);
    assert_eq!(bad(&[E], 3, MAX_PRESSES + 1, &[1]), PlanError::Presses);
    assert_eq!(bad(&[E], 3, 10, &[]), PlanError::NoKeyboard);
    assert_eq!(bad(&[E], 3, 10, &[4, 0]), PlanError::ZeroHandle);
    assert!(Guide::new(plan(&[E], MAX_ROUNDS, MAX_PRESSES), &[4, 5]).is_ok());
}

#[test]
fn g12_the_same_stream_gives_the_same_rounds_and_tallies() {
    let read = || {
        let (plan, f) = guided_chatter();
        let mut g = Guide::new(plan, &f.keyboard).unwrap();
        let changed: Vec<bool> = f.entries.iter().map(|e| g.entry(e)).collect();
        let tallies = g.tallies().clone();
        (changed, tallies, g.finish(f.end_us), f.rounds)
    };
    let (changed, tallies, rounds, fixture_rounds) = read();
    assert_eq!(rounds, fixture_rounds);
    assert_eq!(read(), (changed, tallies, rounds, fixture_rounds));
}

// ---- the guided runs through diagnose() ----

fn hedged_throughout(r: &Report) {
    for f in &r.findings {
        let l = f.lines(&label);
        for text in std::iter::once(&l.headline)
            .chain(&l.evidence)
            .chain(&l.causes)
            .chain(&l.next)
        {
            assert!(hedged(text), "{text}");
        }
    }
    for n in &r.notes {
        assert!(hedged(&n.words(&label)));
    }
}

// Each round on its own, so a round's prompted presses are the engine's count, not the Guide's.
fn prompted_in(f: &Fixture, r: &Round) -> PromptTally {
    let report = diagnose(&Session {
        rounds: &[*r],
        ..f.session()
    });
    report.aggregates.keys[&r.key].prompted.unwrap()
}

#[test]
fn gc01_the_guided_chatter_run() {
    let (plan, f) = guided_chatter();
    let r = f.diagnose();
    let finding = only(&r, Kind::Chatter, E);
    assert_eq!(finding.confidence, Confidence::VeryHigh);
    assert_eq!(
        chatter_of(&finding),
        ChatterEvidence {
            presses: 30,
            affected: 6,
            extra_downs: 6,
            rate_floor_permille: 95,
            rounds: 3,
            rounds_affected: 3,
            gap: Some(SpanMs {
                min_ms: 5,
                max_ms: 5
            }),
            fragment: Some(SpanMs {
                min_ms: 5,
                max_ms: 5
            }),
            other_tested: 2,
            other_affected: 0,
            borderline: 0,
            timing_unknown: 0,
            poll: PollEstimate::AtMost4Ms,
            cap: None,
        }
    );
    assert_eq!(
        finding.next_tests,
        [
            NextTest::AskBoardKind,
            NextTest::SwapSwitch {
                suspect: E,
                partner: Some(G)
            }
        ]
    );
    assert_eq!(f.rounds.len(), 9);
    for (i, round) in f.rounds.iter().enumerate() {
        assert_eq!((round.key, round.asked), (plan.keys[i % 3], 10));
        let p = prompted_in(&f, round);
        assert_eq!(p.presses, 10, "{round:?}");
        // The round closes at the 10th press's release, so its trailing chatter lands after the
        // end, and still joins that press.
        assert_eq!(p.affected, if round.key == E { 2 } else { 0 });
    }
    hedged_throughout(&r);

    let hot_swap = Fixture {
        board: BoardKind::HotSwap,
        ..f
    }
    .diagnose();
    assert_eq!(
        hot_swap.findings[0].next_tests,
        [
            NextTest::SwapSwitch {
                suspect: E,
                partner: Some(G)
            },
            NextTest::CleanContacts { key: E },
            NextTest::RaiseDebounce
        ]
    );
}

#[test]
fn gc01b_the_drawing_counts_every_key_down_it_saw() {
    let (plan, f) = guided_chatter();
    let mut g = Guide::new(plan, &f.keyboard).unwrap();
    for e in &f.entries {
        g.entry(e);
    }
    // The last round's trailing extra press arrives after the plan is done, and nothing moves then.
    assert_eq!(g.tallies(), &BTreeMap::from([(G, 30), (J, 30), (E, 35)]));
}

fn doubles(key: u16) -> impl FnMut(Synth, u16, u32) -> (Synth, bool) {
    move |s, k, _| {
        let s = if k == key {
            s.taps(k, 2, (ms(40), ms(100)), (ms(40), ms(80)))
                .wait(ms(250))
        } else {
            normal(s, k)
        };
        (s, false)
    }
}

#[test]
fn gc02_fast_double_presses_in_a_guided_run() {
    let mut answers = 0;
    let mut typist = doubles(E);
    let f = guided(plan(&[G, J, E], 3, 10), |s, k, n| {
        answers += u32::from(k == E);
        typist(s, k, n)
    });
    let r = f.diagnose();
    assert_clean(&r);
    hedged_throughout(&r);
    // Each double counts once, so each E prompt took 10 of them. The round closes at the release
    // of the 10th double's first press, so the round holds 19 key-downs and the 20th follows it.
    assert_eq!(answers, 30);
    for round in f.rounds.iter().filter(|r| r.key == E) {
        let downs = f
            .entries
            .iter()
            .filter(|e| matches!(**e, Entry::Key { scan: E, up: false, micros, .. } if round.contains(micros)))
            .count();
        assert_eq!(downs, 19);
        assert_eq!(prompted_in(&f, round).presses, 19);
    }
}

#[test]
fn gd01_a_skipped_key_in_a_guided_run() {
    let f = guided(plan(&[G, H, J], 3, 10), |s, k, _| {
        if k == H {
            (s.wait(ms(3_000)), true)
        } else {
            (normal(s, k), false)
        }
    });
    let r = f.diagnose();
    let finding = only(&r, Kind::Dead, H);
    assert_eq!(finding.confidence, Confidence::VeryHigh);
    assert_eq!(
        finding.evidence,
        Evidence::Dead(DeadEvidence {
            rounds: 3,
            silent_rounds: 3,
            asked_in_silent: 30,
            control_rounds: 6,
            bracketed: true,
            other_presses_meanwhile: 0,
        })
    );
    hedged_throughout(&r);
}
