use super::*;

fn answered(s: Synth, key: u16) -> Synth {
    s.round(key, 10, |s| s.taps(key, 10, HOLD, GAP))
}

fn silent(s: Synth, key: u16) -> Synth {
    s.round(key, 10, |s| s.wait(ms(4_000)))
}

fn level(s: Synth) -> Confidence {
    only(&run(s), Kind::Dead, E).confidence
}

// ---- faults ----

#[test]
fn df01_silent_between_answered_controls() {
    let mut s = Synth::new();
    for _ in 0..3 {
        s = silent(answered(s, G), E);
    }
    let r = run(answered(s, G));
    let f = only(&r, Kind::Dead, E);
    assert_eq!(f.confidence, Confidence::VeryHigh);
    assert_eq!(
        f.evidence,
        Evidence::Dead(DeadEvidence {
            rounds: 3,
            silent_rounds: 3,
            asked_in_silent: 30,
            control_rounds: 4,
            bracketed: true,
            other_presses_meanwhile: 0,
        })
    );
}

#[test]
fn df02_one_silent_round_between_controls() {
    assert_eq!(
        level(answered(silent(answered(Synth::new(), G), E), G)),
        Confidence::Medium
    );
}

#[test]
fn df03_silent_rounds_then_controls() {
    let s = answered(answered(silent(silent(Synth::new(), E), E), G), G);
    assert_eq!(level(s), Confidence::High);
}

#[test]
fn df04_silent_in_one_of_three() {
    let mut s = answered(Synth::new(), G);
    for quiet in [false, true, false] {
        s = if quiet { silent(s, E) } else { answered(s, E) };
        s = answered(s, G);
    }
    assert_eq!(level(s), Confidence::Low);
}

#[test]
fn df05_silent_in_two_of_three() {
    let mut s = answered(Synth::new(), G);
    for quiet in [true, false, true] {
        s = if quiet { silent(s, E) } else { answered(s, E) };
        s = answered(s, G);
    }
    assert_eq!(level(s), Confidence::Medium);
}

#[test]
fn df06_silent_twice_without_controls() {
    let s = silent(
        silent(Synth::new().press(SPACE, ms(90)).wait(ms(500)), E),
        E,
    );
    assert_eq!(level(s), Confidence::Medium);
}

#[test]
fn df07_silent_once_without_controls() {
    let s = silent(Synth::new().press(SPACE, ms(90)).wait(ms(500)), E);
    assert_eq!(level(s), Confidence::Low);
}

// ---- clean and not assessed ----

#[test]
fn dc01_every_round_answered() {
    let mut s = Synth::new();
    for _ in 0..3 {
        s = answered(answered(s, G), E);
    }
    assert_clean(&run(s));
}

#[test]
fn dc02_nothing_from_the_keyboard() {
    let r = run(silent(silent(Synth::new(), E), E));
    assert_clean(&r);
    assert_eq!(r.notes, vec![Note::NoInputFromKeyboard]);
}

#[test]
fn dc03_the_key_arrives_from_another_keyboard() {
    let s = answered(Synth::new(), G).round(E, 10, |s| s.on(9).taps(E, 10, HOLD, GAP).on(1));
    let r = run(answered(s, G));
    assert_clean(&r);
    assert!(r.notes.contains(&Note::NotAssessed {
        key: E,
        rounds: 1,
        why: Why::OtherKeyboard
    }));
}

#[test]
fn dc04_another_code_arrives_instead() {
    let mut s = answered(Synth::new(), G);
    for _ in 0..3 {
        s = s.round(BACKSLASH, 10, |s| s.taps(ISO_BACKSLASH, 10, HOLD, GAP));
        s = answered(s, G);
    }
    let r = run(s);
    assert_clean(&r);
    assert!(r.notes.contains(&Note::DifferentCode {
        asked: BACKSLASH,
        got: ISO_BACKSLASH,
        rounds: 3
    }));
}

#[test]
fn dc05_two_other_keys_held() {
    let s = answered(Synth::new(), G).round(E, 10, |s| {
        s.down(LSHIFT)
            .down(LCTRL)
            .wait(ms(4_000))
            .up(LCTRL)
            .up(LSHIFT)
    });
    let r = run(answered(s, G));
    assert_clean(&r);
    assert!(r.notes.contains(&Note::NotAssessed {
        key: E,
        rounds: 1,
        why: Why::Blocked
    }));
}

#[test]
fn dc06_a_rollover_overrun_inside_the_round() {
    let s = answered(Synth::new(), G).round(E, 10, |s| {
        s.wait(ms(2_000)).press(0x00FF, ms(1)).wait(ms(2_000))
    });
    let r = run(answered(s, G));
    assert_clean(&r);
    assert!(r.notes.contains(&Note::NotAssessed {
        key: E,
        rounds: 1,
        why: Why::Blocked
    }));
}

#[test]
fn dc07_a_round_spent_paused() {
    let s = answered(Synth::new(), G).round(E, 10, |s| s.pause(&[]).wait(ms(4_000)).resume());
    let r = run(answered(s, G));
    assert_clean(&r);
    assert!(r.notes.contains(&Note::NotAssessed {
        key: E,
        rounds: 1,
        why: Why::Paused
    }));
}

#[test]
fn dc08_a_key_held_into_its_round_is_stuck_not_dead() {
    let s = answered(Synth::new(), G).down(E).wait(ms(1_000));
    let s = answered(silent(s, E), G);
    let r = run(s);
    let f = only(&r, Kind::Stuck, E);
    assert_eq!(f.confidence, Confidence::High);
    assert!(r.notes.contains(&Note::NotAssessed {
        key: E,
        rounds: 1,
        why: Why::HeldDown
    }));
}

#[test]
fn dc09_software_sent_keys_during_the_round() {
    let s = answered(Synth::new(), G).round(E, 10, |s| {
        s.wait(ms(1_000))
            .on(0)
            .press(R, ms(20))
            .on(1)
            .wait(ms(3_000))
    });
    let r = run(answered(s, G));
    assert_clean(&r);
    assert!(r.notes.contains(&Note::NotAssessed {
        key: E,
        rounds: 1,
        why: Why::Injected
    }));
}

#[test]
fn dc10_a_single_paced_prompt_missed() {
    let s = answered(Synth::new(), G).round(E, 1, |s| s.wait(ms(2_000)));
    let r = run(answered(s, G));
    assert_clean(&r);
    assert!(
        !r.notes
            .iter()
            .any(|n| matches!(n, Note::NotAssessed { .. }))
    );
}

#[test]
fn dc05c_positive_control_with_one_key_held() {
    let s = answered(Synth::new(), G).round(E, 10, |s| s.down(LSHIFT).wait(ms(4_000)).up(LSHIFT));
    let r = run(answered(s, G));
    assert_eq!(only(&r, Kind::Dead, E).confidence, Confidence::Medium);
}

#[test]
fn dc06c_positive_control_without_the_overrun() {
    let s = answered(Synth::new(), G).round(E, 10, |s| s.wait(ms(4_001)));
    assert_eq!(level(answered(s, G)), Confidence::Medium);
}

// ---- found in review ----

#[test]
fn df08_a_brief_chord_does_not_void_a_silent_round() {
    let mut s = Synth::new();
    for _ in 0..3 {
        s = answered(s, G).round(E, 10, |s| {
            s.wait(ms(2_000))
                .down(LSHIFT)
                .wait(ms(60))
                .press(R, ms(90))
                .wait(ms(40))
                .up(LSHIFT)
                .wait(ms(2_000))
        });
    }
    assert_eq!(level(answered(s, G)), Confidence::VeryHigh);
}

#[test]
fn dc11_rounds_without_length_are_ignored() {
    let mut fixture = answered(Synth::new(), G).build();
    fixture.rounds.push(Round {
        key: E,
        asked: 10,
        start_us: ms(5_000),
        end_us: ms(4_000),
    });
    fixture.rounds.push(Round {
        key: E,
        asked: 10,
        start_us: ms(3_000),
        end_us: ms(3_000),
    });
    let r = fixture.diagnose();
    assert!(r.findings.iter().all(|f| f.key != E), "{:#?}", r.findings);
}

// A key chosen alone that sends nothing can't be told from a keyboard that sends nothing, so it
// reads as no input, and the picker asks for a neighbor too.
#[test]
fn dc12_one_chosen_key_alone_reads_as_no_input() {
    let skipped = |keys: &[u16]| {
        let plan = Plan {
            keys: keys.to_vec(),
            rounds: 3,
            presses: 30,
        };
        crate::fixture::guided(plan, |s, key, _| {
            if key == E {
                (s.wait(ms(3_000)), true)
            } else {
                (normal(s, key), false)
            }
        })
        .diagnose()
    };
    let r = skipped(&[E]);
    assert_clean(&r);
    assert_eq!(r.notes, vec![Note::NoInputFromKeyboard]);

    // Positive control: a second key that answers shows the keyboard was there.
    let r = skipped(&[E, R]);
    assert_eq!(only(&r, Kind::Dead, E).confidence, Confidence::High);
}
