use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::fixture::{Fixture, interleaved};

fn order_free<T: PartialEq>(f: impl Fn(&Fixture) -> T) -> bool {
    let first = f(&interleaved(1));
    (2..30).all(|seed| f(&interleaved(seed)) == first)
}

#[test]
fn ag01_aggregates_do_not_depend_on_how_keys_interleave() {
    assert!(order_free(|f| f.diagnose().aggregates));
}

#[test]
fn ag01_positive_control_catches_a_cross_key_field() {
    let crossed = |f: &Fixture| {
        let mut held: BTreeSet<u16> = BTreeSet::new();
        let mut during: BTreeMap<u16, u32> = BTreeMap::new();
        for e in &f.entries {
            if let Entry::Key { scan, up, .. } = *e {
                if up {
                    held.remove(&scan);
                } else {
                    for h in &held {
                        if *h != scan {
                            *during.entry(*h).or_default() += 1;
                        }
                    }
                    held.insert(scan);
                }
            }
        }
        during
    };
    assert!(!order_free(crossed));
}

#[test]
fn ag02_saved_types_hold_only_counts_and_histograms() {
    // Naming every field makes a new one fail to compile here, so it gets a privacy review.
    let KeyAggregate {
        downs: _,
        ups: _,
        episodes: _,
        presses: _,
        affected: _,
        extra_downs: _,
        repeats: _,
        duplicates: _,
        orphan_ups: _,
        interrupted: _,
        after_resume: _,
        long_holds: _,
        unreleased: _,
        borderline: _,
        timing_unknown: _,
        hold: _,
        release_gap: _,
        interval: _,
        prompted: _,
    } = KeyAggregate::default();
    let PromptTally {
        rounds: _,
        asked: _,
        presses: _,
        affected: _,
        extra_downs: _,
        rounds_pressed: _,
        rounds_affected: _,
        assessed: _,
        silent: _,
        not_assessed: _,
    } = PromptTally::default();
    let Limits {
        poll: _,
        injected: _,
        other_devices: _,
        unknown_codes: _,
        fake_shifts: _,
        overruns: _,
        pauses: _,
        timing_unknown: _,
    } = Limits::default();
    let Aggregates {
        rules: _,
        limits: _,
        keys: _,
    } = Aggregates::default();
}

#[test]
fn ag03_bin_edges_sit_between_125_hz_ticks() {
    assert_ne!(Histogram::bin(19_999), Histogram::bin(20_000));
    for tick in 1..=300u64 {
        let m = tick * ms(8);
        assert_eq!(Histogram::bin(m - 999), Histogram::bin(m + 999), "{m}");
    }
    assert_eq!(Histogram::bin(999), 0);
    assert_eq!(Histogram::bin(ms(5_000)), crate::params::BINS - 1);
}

#[test]
fn ag04_saved_keeps_prompted_and_named_keys_only() {
    let s = Synth::new()
        .round(E, 12, |s| s.taps(E, 12, HOLD, GAP))
        .taps(K, 20, HOLD, GAP);
    let r = run(s);
    assert!(r.aggregates.keys.contains_key(&K));
    let saved = r.saved();
    assert!(saved.keys.contains_key(&E));
    assert!(!saved.keys.contains_key(&K));
}

#[test]
fn ag05_diagnosis_is_deterministic() {
    let f = super::chatter_rounds([5, 5, 4], |s| {
        s.fragments(E, &[ms(5), ms(5), ms(100)]).wait(ms(200))
    })
    .build();
    assert_eq!(f.diagnose(), f.diagnose());
    let mut reversed = Fixture {
        entries: f.entries.clone(),
        rounds: f.rounds.clone(),
        end_us: f.end_us,
        keyboard: f.keyboard.clone(),
        board: f.board,
    };
    reversed.rounds.reverse();
    assert_eq!(f.diagnose(), reversed.diagnose());
}

#[test]
fn ag06_histograms_of_a_small_stream() {
    let r = run(Synth::new()
        .press(E, ms(50))
        .wait(ms(30))
        .press(E, ms(10))
        .wait(ms(200))
        .press(E, ms(100)));
    let a = r.aggregates.keys[&E];
    let at = |bins: &[usize]| {
        let mut h = Histogram::default();
        for &b in bins {
            h.0[b] += 1;
        }
        h
    };
    assert_eq!(a.hold, at(&[6, 2, 8]));
    assert_eq!(a.release_gap, at(&[5, 9]));
    assert_eq!(a.interval, at(&[7, 9]));
}

#[test]
fn ag07_the_polling_estimate() {
    let taps = || Synth::new().taps(G, 60, (ms(40), ms(130)), (ms(60), ms(250)));
    assert_eq!(run(taps()).aggregates.limits.poll, PollEstimate::AtMost4Ms);
    assert_eq!(
        run(taps().polled(ms(8))).aggregates.limits.poll,
        PollEstimate::Ms8
    );
    assert_eq!(
        run(taps().polled(ms(16))).aggregates.limits.poll,
        PollEstimate::Ms16OrSlower
    );
    assert_eq!(
        run(Synth::new().taps(G, 5, HOLD, GAP))
            .aggregates
            .limits
            .poll,
        PollEstimate::Unknown
    );
}
