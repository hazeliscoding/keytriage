use super::*;

fn eager(s: Synth) -> Synth {
    s.fragments(E, &[ms(5), ms(5), ms(100)]).wait(ms(200))
}

fn release(s: Synth) -> Synth {
    s.fragments(E, &[ms(100), ms(40), ms(4)]).wait(ms(200))
}

fn dropout(s: Synth) -> Synth {
    s.fragments(E, &[ms(60), ms(10), ms(80)]).wait(ms(200))
}

fn laptop(s: Synth) -> Synth {
    s.fragments(E, &[ms(100), ms(34), ms(10)]).wait(ms(200))
}

fn defer(s: Synth) -> Synth {
    s.fragments(E, &[ms(12), ms(12), ms(90)]).wait(ms(200))
}

fn intermittent(s: Synth) -> Synth {
    s.fragments(E, &[ms(80), ms(8), ms(6), ms(8), ms(5), ms(8), ms(80)])
        .wait(ms(200))
}

fn level(s: Synth) -> Confidence {
    only(&run(s), Kind::Chatter, E).confidence
}

// ---- faults ----

#[test]
fn cf01_eager_debounce_is_the_canned_finding() {
    let r = run(chatter_rounds([5, 5, 4], eager));
    let f = only(&r, Kind::Chatter, E);
    assert_eq!(f.confidence, Confidence::VeryHigh);
    let e = chatter_of(&f);
    assert_eq!((e.affected, e.presses, e.extra_downs), (14, 100, 14));
    assert_eq!(e.rate_floor_permille, 85);
    assert_eq!((e.rounds_affected, e.rounds), (3, 3));
    assert_eq!((e.other_tested, e.other_affected), (2, 0));
    assert_eq!(
        e.gap,
        Some(SpanMs {
            min_ms: 5,
            max_ms: 5
        })
    );
    assert_eq!(
        e.fragment,
        Some(SpanMs {
            min_ms: 5,
            max_ms: 5
        })
    );
    assert_eq!(e.borderline, 0);
}

#[test]
fn cf02_eager_debounce_at_125_hz() {
    let r = run(chatter_rounds([5, 5, 4], eager).polled(ms(8)));
    let f = only(&r, Kind::Chatter, E);
    assert_eq!(f.confidence, Confidence::VeryHigh);
    assert_eq!(r.aggregates.limits.poll, PollEstimate::Ms8);
    assert_eq!(chatter_of(&f).affected, 14);
}

#[test]
fn cf03_sub_poll_pulses_can_vanish() {
    let r = run(chatter_rounds([5, 5, 4], eager).polled_lossy(ms(8)));
    let affected = r
        .findings
        .iter()
        .map(|f| chatter_of(f).affected)
        .next()
        .unwrap_or(0);
    println!(
        "cf03 {:?} affected {affected}",
        r.findings.first().map(|f| f.confidence)
    );
    assert!(affected < 14);
    assert!(
        r.findings
            .iter()
            .all(|f| f.kind() == Kind::Chatter && f.key == E)
    );
}

#[test]
fn cf04_release_chatter() {
    assert_eq!(level(chatter_rounds([2, 2, 2], release)), Confidence::High);
}

#[test]
fn cf05_mid_hold_dropout_pins_the_5_percent_edge() {
    assert_eq!(level(chatter_rounds([3, 3, 3], dropout)), Confidence::High);
    assert_eq!(
        level(chatter_rounds([4, 3, 3], dropout)),
        Confidence::VeryHigh
    );
}

#[test]
fn cf06_laptop_firmware_phantom() {
    let r = run(chatter_rounds([3, 2, 3], laptop).board(BoardKind::Laptop));
    let f = only(&r, Kind::Chatter, E);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(f.causes[0], Cause::DebrisOrResidue);
}

#[test]
fn cf07_defer_chatter() {
    assert_eq!(level(chatter_rounds([2, 2, 2], defer)), Confidence::High);
}

#[test]
fn cf08_intermittent_contact_counts_one_press() {
    let r = run(chatter_rounds([2, 2, 1], intermittent));
    let f = only(&r, Kind::Chatter, E);
    let e = chatter_of(&f);
    assert_eq!((e.affected, e.presses, e.extra_downs), (5, 100, 15));
    assert_eq!(f.confidence, Confidence::High);
}

#[test]
fn cf09_a_low_rate() {
    assert_eq!(level(chatter_rounds([2, 0, 0], eager)), Confidence::Low);
    assert_eq!(level(chatter_rounds([1, 1, 0], eager)), Confidence::Medium);
}

#[test]
fn cf10_one_round_only() {
    assert_eq!(level(chatter_rounds([0, 5, 0], eager)), Confidence::Medium);
}

#[test]
fn cf11_two_poll_fragments_at_125_hz() {
    let s = Synth::new()
        .round(E, 30, |mut s| {
            for p in 0..30 {
                s = if p % 10 == 5 {
                    s.fragments(E, &[ms(16), ms(16), ms(80)]).wait(ms(200))
                } else {
                    normal(s, E)
                };
            }
            s
        })
        .polled(ms(8));
    let r = run(s);
    let f = only(&r, Kind::Chatter, E);
    assert_eq!(f.confidence, Confidence::Medium);
    assert_eq!(chatter_of(&f).affected, 3);
}

#[test]
fn cf12_many_keys_point_past_the_switch() {
    let mut s = Synth::new();
    for key in [E, R, T, Y] {
        for bad in [2u32, 1] {
            s = s.round(key, 15, |mut s| {
                for p in 0..15 {
                    s = if p % 5 == 2 && p / 5 < bad {
                        s.fragments(key, &[ms(60), ms(10), ms(80)]).wait(ms(200))
                    } else {
                        normal(s, key)
                    };
                }
                s
            });
        }
    }
    let r = run(s);
    assert_eq!(r.findings.len(), 4);
    for f in &r.findings {
        assert_eq!(f.confidence, Confidence::Medium);
        assert_eq!(chatter_of(f).cap, Some(Cap::Systemic));
        assert_eq!(f.causes[0], Cause::FirmwareDebounce);
        assert_eq!(f.next_tests[1], NextTest::RaiseDebounce);
    }
    assert!(r.notes.contains(&Note::Systemic { keys: 4 }));
}

#[test]
fn cf13_polling_at_16_ms_caps_at_medium() {
    let r = run(chatter_rounds([5, 5, 4], eager).polled(ms(16)));
    let f = only(&r, Kind::Chatter, E);
    assert_eq!(f.confidence, Confidence::Medium);
    assert_eq!(chatter_of(&f).cap, Some(Cap::CoarsePolling));
    assert_eq!(r.aggregates.limits.poll, PollEstimate::Ms16OrSlower);
}

#[test]
fn cf14_a_late_short_phantom() {
    let s = Synth::new()
        .round(E, 20, |mut s| {
            for p in 0..20 {
                s = if p == 3 || p == 14 {
                    s.fragments(E, &[ms(100), ms(34), ms(12)]).wait(ms(200))
                } else {
                    normal(s, E)
                };
            }
            s
        })
        .round(E, 20, |mut s| {
            for p in 0..20 {
                s = if p == 7 || p == 15 {
                    s.fragments(E, &[ms(88), ms(22), ms(4)]).wait(ms(200))
                } else {
                    normal(s, E)
                };
            }
            s
        });
    let f = only(&run(s), Kind::Chatter, E);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(
        chatter_of(&f).gap,
        Some(SpanMs {
            min_ms: 22,
            max_ms: 34
        })
    );
}

#[test]
fn cf15_a_free_test_gives_notes_only() {
    let mut s = Synth::new();
    for p in 0..100 {
        s = if p % 10 == 3 { eager(s) } else { normal(s, E) };
    }
    let r = run(s);
    assert_clean(&r);
    assert!(r.notes.contains(&Note::Unprompted {
        key: E,
        affected: 10,
        presses: 100
    }));
}

// ---- clean ----

fn steady() -> Synth {
    let taps = |key| move |s: Synth| s.taps(key, 30, (ms(60), ms(110)), (ms(90), ms(250)));
    Synth::new()
        .round(E, 30, taps(E))
        .round(F, 30, taps(F))
        .round(E, 30, taps(E))
}

#[test]
fn cc01_steady_presses() {
    let r = run(steady());
    assert_clean(&r);
    assert!(r.notes.contains(&Note::Clean {
        key: E,
        presses: 60,
        bound_permille: 50
    }));
}

#[test]
fn cc02_steady_presses_at_125_hz() {
    assert_clean(&run(steady().polled(ms(8))));
}

fn doubles() -> Synth {
    let mut s = Synth::new();
    for _ in 0..3 {
        s = s.round(E, 30, |mut s| {
            for _ in 0..15 {
                s = s
                    .taps(E, 2, (ms(40), ms(100)), (ms(40), ms(80)))
                    .wait(ms(250));
            }
            s
        });
    }
    s
}

#[test]
fn cc03_fast_deliberate_double_presses() {
    let r = run(doubles());
    assert_clean(&r);
    assert_eq!(r.aggregates.keys[&E].presses, 90);
    assert_eq!(r.aggregates.keys[&E].borderline, 0);
}

fn fastest() -> Synth {
    Synth::new().round(E, 30, |s| s.taps(E, 30, (ms(30), ms(35)), (ms(30), ms(35))))
}

#[test]
fn cc04_the_fastest_one_finger_repeats() {
    let r = run(fastest());
    assert_clean(&r);
    assert!(r.aggregates.keys[&E].borderline > 0);
}

#[test]
fn cc05_the_fastest_repeats_at_125_hz() {
    let r = run(fastest().polled(ms(8)));
    assert_clean(&r);
    println!("cc05 borderline {}", r.aggregates.keys[&E].borderline);
}

#[test]
fn cc06_double_presses_at_125_hz() {
    assert_clean(&run(doubles().polled(ms(8))));
}

#[test]
fn cc07_triple_taps() {
    let s = Synth::new().round(E, 60, |mut s| {
        for _ in 0..20 {
            s = s
                .fragments(E, &[ms(35), ms(35), ms(35), ms(35), ms(35)])
                .wait(ms(400));
        }
        s
    });
    assert_clean(&run(s));
}

#[test]
fn cc08_autorepeat() {
    let s = Synth::new().round(E, 12, |s| {
        s.taps(E, 5, HOLD, GAP)
            .hold(E, ms(1_500), ms(500), ms(33))
            .wait(ms(300))
            .hold(E, ms(1_900), ms(250), ms(27))
            .wait(ms(300))
            .taps(E, 5, HOLD, GAP)
    });
    let r = run(s);
    assert_clean(&r);
    assert!(r.aggregates.keys[&E].repeats > 40);
}

#[test]
fn cc09_rollover_typing() {
    let mut s = Synth::new();
    for _ in 0..40 {
        s = s
            .down(G)
            .wait(ms(60))
            .down(J)
            .wait(ms(40))
            .up(G)
            .wait(ms(30))
            .down(K)
            .wait(ms(70))
            .up(J)
            .wait(ms(20))
            .up(K)
            .wait(ms(90));
    }
    let r = run(s);
    assert_clean(&r);
    assert!(r.notes.is_empty(), "{:?}", r.notes);
}

#[test]
fn cc10_keys_the_firmware_taps() {
    let s = Synth::new()
        .round(F, 30, |s| s.taps(F, 30, (ms(1), ms(8)), (ms(60), ms(250))))
        .round(G, 30, |s| s.taps(G, 30, (ms(8), ms(8)), (ms(150), ms(250))))
        .polled(ms(1));
    assert_clean(&run(s));
}

#[test]
fn cc11_one_light_tap() {
    let s = Synth::new().round(E, 21, |s| {
        s.taps(E, 10, HOLD, GAP)
            .press(E, ms(15))
            .wait(ms(180))
            .taps(E, 10, HOLD, GAP)
    });
    let r = run(s);
    assert_clean(&r);
    assert!(matches!(
        r.notes[..],
        [Note::Clean {
            key: E,
            presses: 21,
            ..
        }]
    ));
}

#[test]
fn cc12_one_light_tap_inside_a_double_press() {
    let s = Synth::new().round(E, 100, |mut s| {
        for p in 0..100 {
            s = if p == 50 {
                s.fragments(E, &[ms(90), ms(50), ms(15)]).wait(ms(200))
            } else {
                normal(s, E)
            };
        }
        s
    });
    let r = run(s);
    assert_clean(&r);
    assert!(r.notes.contains(&Note::OneExtraDown {
        key: E,
        presses: 100,
        extra_downs: 1
    }));
}

#[test]
fn cc13_reads_bunched_by_a_stall() {
    let s = Synth::new().round(E, 22, |s| {
        s.taps(E, 10, HOLD, GAP)
            .down(E)
            .wait(ms(90))
            .up(E)
            .wait(30)
            .down(E)
            .wait(40)
            .up(E)
            .wait(ms(200))
            .taps(E, 10, HOLD, GAP)
    });
    let r = run(s);
    assert_clean(&r);
    assert_eq!(r.aggregates.keys[&E].timing_unknown, 2);
}

#[test]
fn cc14_an_interrupted_key() {
    let s = Synth::new().round(E, 12, |s| {
        s.taps(E, 5, HOLD, GAP)
            .down(E)
            .wait(ms(100))
            .pause(&[E])
            .wait(ms(5_000))
            .resume()
            .wait(ms(10))
            .down(E)
            .wait(ms(33))
            .down(E)
            .wait(ms(3))
            .up(E)
            .wait(ms(12))
            .taps(E, 5, HOLD, GAP)
    });
    let r = run(s);
    assert_clean(&r);
    let a = r.aggregates.keys[&E];
    assert_eq!((a.interrupted, a.after_resume), (1, 3));
}

#[test]
fn cc15_a_key_held_when_capture_starts() {
    let s = Synth::new().round(ENTER, 6, |s| {
        s.down(ENTER)
            .wait(ms(33))
            .down(ENTER)
            .wait(ms(4))
            .up(ENTER)
            .wait(ms(300))
            .taps(ENTER, 5, HOLD, GAP)
    });
    assert_clean(&run(s));
}

#[test]
fn cc16_injected_chatter() {
    let s = Synth::new().round(E, 30, |mut s| {
        for p in 0..30 {
            s = normal(s, E);
            if p % 5 == 0 {
                s = s
                    .on(0)
                    .fragments(E, &[ms(5), ms(5), ms(100)])
                    .on(1)
                    .wait(ms(200));
            }
        }
        s
    });
    let r = run(s);
    assert_clean(&r);
    assert!(r.aggregates.limits.injected > 0);
}

#[test]
fn cc17_a_rollover_overrun_between_release_and_press() {
    let s = Synth::new().round(E, 23, |mut s| {
        for _ in 0..3 {
            s = s
                .taps(E, 5, HOLD, GAP)
                .press(E, ms(60))
                .wait(ms(2))
                .press(0x00FF, ms(1))
                .wait(ms(3))
                .press(E, ms(80))
                .wait(ms(200));
        }
        s.taps(E, 5, HOLD, GAP)
    });
    let r = run(s);
    assert_clean(&r);
    assert!(r.aggregates.keys[&E].timing_unknown >= 3);
}

#[test]
fn cc18_one_key_on_two_handles() {
    let s = Synth::new().keyboard(&[1, 2]).round(E, 23, |mut s| {
        for _ in 0..3 {
            s = s
                .taps(E, 5, HOLD, GAP)
                .down(E)
                .wait(150)
                .on(2)
                .down(E)
                .wait(ms(80))
                .up(E)
                .on(1)
                .wait(ms(5))
                .press(E, ms(80))
                .wait(ms(200));
        }
        s.taps(E, 5, HOLD, GAP)
    });
    let r = run(s);
    assert_clean(&r);
    assert_eq!(r.aggregates.keys[&E].duplicates, 3);
}

#[test]
fn cc19_grazed_keys_outside_their_rounds() {
    let mut s = Synth::new().round(E, 20, |s| s.taps(E, 20, HOLD, GAP));
    for _ in 0..2 {
        s = s
            .press(R, ms(12))
            .wait(ms(80))
            .press(R, ms(12))
            .wait(ms(1_000));
    }
    let r = run(s);
    assert_clean(&r);
    assert!(r.notes.contains(&Note::Unprompted {
        key: R,
        affected: 2,
        presses: 2
    }));
}

#[test]
fn cc20_chatter_on_another_keyboard() {
    let s = Synth::new().round(E, 30, |mut s| {
        for p in 0..30 {
            s = normal(s, E);
            if p % 3 == 0 {
                s = s
                    .on(9)
                    .fragments(E, &[ms(5), ms(5), ms(100)])
                    .on(1)
                    .wait(ms(200));
            }
        }
        s
    });
    let r = run(s);
    assert_clean(&r);
    assert!(r.aggregates.limits.other_devices > 0);
}

#[test]
fn cc21_malformed_streams_do_not_panic() {
    let s = Synth::new()
        .up(E)
        .resume()
        .raw(Entry::Key {
            scan: E,
            up: false,
            device: 1,
            micros: 50_000,
        })
        .raw(Entry::Key {
            scan: E,
            up: true,
            device: 1,
            micros: 10_000,
        })
        .raw(Entry::Key {
            scan: E,
            up: false,
            device: 1,
            micros: 9_000,
        })
        .raw(Entry::Resumed { micros: 1 })
        .pause(&[E, G]);
    let r = s.end_now().diagnose();
    assert_clean(&r);
}

#[test]
fn cc22_a_test_that_starts_paused() {
    let s = Synth::new()
        .pause(&[])
        .wait(ms(2_000))
        .resume()
        .round(E, 30, |s| s.taps(E, 30, HOLD, GAP));
    assert_clean(&run(s));
}

fn overrun_between(marker: bool) -> Synth {
    Synth::new().round(E, 23, |mut s| {
        for _ in 0..3 {
            s = s.taps(E, 5, HOLD, GAP).press(E, ms(60)).wait(ms(2));
            s = if marker {
                s.press(0x00FF, ms(1))
            } else {
                s.wait(ms(1))
            };
            s = s.wait(ms(3)).press(E, ms(80)).wait(ms(200));
        }
        s.taps(E, 5, HOLD, GAP)
    })
}

#[test]
fn cc17c_positive_control_without_the_overrun_marker() {
    assert_clean(&run(overrun_between(true)));
    only(&run(overrun_between(false)), Kind::Chatter, E);
}

#[test]
fn cc18c_positive_control_on_one_handle() {
    let s = Synth::new().keyboard(&[1, 2]).round(E, 23, |mut s| {
        for _ in 0..3 {
            s = s
                .taps(E, 5, HOLD, GAP)
                .down(E)
                .wait(150)
                .down(E)
                .wait(ms(80))
                .up(E)
                .wait(ms(5))
                .press(E, ms(80))
                .wait(ms(200));
        }
        s.taps(E, 5, HOLD, GAP)
    });
    only(&run(s), Kind::Chatter, E);
}

// ---- found in review ----

// Three 5 ms release phantoms on 40% of presses: more short holds than finger holds, but not the
// near-total share a firmware-tapped key shows.
#[test]
fn cf16_heavy_release_chatter_is_not_taken_for_firmware_taps() {
    let mut s = controls(Synth::new());
    for _ in 0..3 {
        s = s.round(E, 30, |mut s| {
            for p in 0..30 {
                s = if p % 5 < 2 {
                    s.fragments(E, &[ms(100), ms(30), ms(5), ms(25), ms(5), ms(25), ms(5)])
                        .wait(ms(200))
                } else {
                    normal(s, E)
                };
            }
            s
        });
    }
    let r = run(s);
    let f = only(&r, Kind::Chatter, E);
    assert_eq!(f.confidence, Confidence::VeryHigh);
    assert_eq!(chatter_of(&f).affected, 36);
    assert!(
        !r.notes
            .iter()
            .any(|n| matches!(n, Note::Clean { key: E, .. }))
    );
}

// Same-key presses at the fastest human pace (75 ms holds, 145 ms apart), with a 4 ms release
// phantom 36 ms before the next press.
fn fast_with_phantoms(every: u32) -> Synth {
    let mut s = controls(Synth::new());
    for _ in 0..3 {
        s = s.round(E, 30, |mut s| {
            for p in 0..30 {
                s = if p % every == 0 {
                    s.fragments(E, &[ms(75), ms(30), ms(4)]).wait(ms(36))
                } else {
                    s.press(E, ms(75)).wait(ms(70))
                };
            }
            s
        });
    }
    s
}

#[test]
fn cf17_a_release_phantom_joins_only_its_own_press() {
    let e = chatter_of(&only(&run(fast_with_phantoms(10)), Kind::Chatter, E));
    assert_eq!((e.presses, e.affected, e.extra_downs), (90, 9, 9));
    assert_eq!(
        e.gap,
        Some(SpanMs {
            min_ms: 30,
            max_ms: 30
        })
    );

    let f = only(&run(fast_with_phantoms(1)), Kind::Chatter, E);
    let e = chatter_of(&f);
    assert_eq!((e.presses, e.affected, e.extra_downs), (90, 90, 90));
    assert_eq!(f.confidence, Confidence::VeryHigh);
}

// Taps of 30 to 35 ms at 16 ms polling read as one or two polls, the same as a phantom.
fn fast_taps(s: Synth, key: u16) -> Synth {
    s.round(key, 30, |s| {
        s.taps(key, 30, (ms(30), ms(35)), (ms(30), ms(35)))
    })
}

#[test]
fn cc23_fast_taps_at_16_ms_are_not_evidence() {
    let mut s = Synth::new().round(G, 12, |s| s.taps(G, 12, HOLD, GAP));
    for _ in 0..3 {
        s = fast_taps(s, E).round(G, 12, |s| s.taps(G, 12, HOLD, GAP));
    }
    assert_clean(&run(s.polled(ms(16))));

    let mut s = Synth::new();
    for _ in 0..3 {
        for key in [E, R, T] {
            s = fast_taps(s, key);
        }
    }
    let r = run(s.polled(ms(16)));
    assert_clean(&r);
    assert!(!r.notes.iter().any(|n| matches!(n, Note::Systemic { .. })));
}

#[test]
fn cf18_the_comparison_line_agrees_with_the_systemic_one() {
    let mut s = Synth::new().round(G, 12, |s| s.taps(G, 12, HOLD, GAP));
    for key in [E, R, T] {
        s = s.round(key, 6, |mut s| {
            for p in 0..6 {
                s = if p % 3 == 1 {
                    s.fragments(key, &[ms(5), ms(5), ms(100)]).wait(ms(200))
                } else {
                    normal(s, key)
                };
            }
            s
        });
    }
    let r = run(s);
    assert_eq!(r.findings.len(), 3, "{:#?}", r.findings);
    for f in &r.findings {
        let e = chatter_of(f);
        assert_eq!((e.other_tested, e.other_affected), (3, 2));
    }
}
