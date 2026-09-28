use super::*;

fn level(s: crate::fixture::Fixture) -> Confidence {
    only(&s.diagnose(), Kind::Stuck, E).confidence
}

// ---- faults ----

#[test]
fn sf01_stuck_while_other_keys_are_typed() {
    let s = Synth::new()
        .down(E)
        .repeats(E, ms(500), ms(33), 30)
        .wait(ms(100))
        .press(F, ms(90))
        .wait(ms(150))
        .press(G, ms(90))
        .wait(ms(150))
        .press(J, ms(90));
    let now = s.now();
    let s = s.wait(ms(8_000) - now);
    assert_eq!(level(s.end_now()), Confidence::Medium);
}

#[test]
fn sf02_stuck_through_its_own_round() {
    let s = Synth::new().round(E, 10, |s| s.down(E).repeats(E, ms(500), ms(33), 170));
    assert_eq!(level(s.build()), Confidence::Medium);
}

#[test]
fn sf03_stuck_while_another_key_answers_its_round() {
    let s = Synth::new()
        .down(E)
        .wait(ms(500))
        .round(G, 10, |s| s.taps(G, 10, HOLD, GAP))
        .wait(ms(8_000));
    let r = s.build().diagnose();
    let f = only(&r, Kind::Stuck, E);
    assert_eq!(f.confidence, Confidence::High);
    let Evidence::Stuck(e) = f.evidence else {
        panic!()
    };
    assert!(e.still_down && e.others_completed == 10 && e.other_rounds_answered == 1);
}

#[test]
fn sf04_down_alone_at_the_end() {
    let s = Synth::new().down(E).wait(ms(3_000));
    assert_eq!(level(s.build()), Confidence::Low);
}

#[test]
fn sf05_stuck_through_two_of_its_own_rounds() {
    let mut s = Synth::new();
    for _ in 0..2 {
        s = s.round(E, 10, |s| s.hold(E, ms(6_000), ms(500), ms(33)));
    }
    let r = s.build().diagnose();
    let f = only(&r, Kind::Stuck, E);
    assert_eq!(f.confidence, Confidence::VeryHigh);
    assert!(
        !r.notes
            .iter()
            .any(|n| matches!(n, Note::ReleasedWithNextKey { .. }))
    );
}

// ---- clean ----

#[test]
fn sc01_an_autorepeat_hold() {
    let s = Synth::new().round(E, 12, |s| {
        s.taps(E, 5, HOLD, GAP)
            .hold(E, ms(1_500), ms(500), ms(33))
            .wait(ms(200))
            .taps(E, 5, HOLD, GAP)
    });
    assert_clean(&run(s));
}

#[test]
fn sc02_a_deliberate_long_hold() {
    let r = run(Synth::new().hold(SPACE, ms(5_000), ms(500), ms(33)));
    assert_clean(&r);
    assert_eq!(r.aggregates.keys[&SPACE].long_holds, 1);
}

#[test]
fn sc03_interrupted_and_never_released() {
    let s = Synth::new()
        .taps(G, 12, HOLD, GAP)
        .down(E)
        .wait(ms(500))
        .pause(&[E])
        .wait(ms(9_000))
        .resume();
    let r = run(s);
    assert_clean(&r);
    assert!(r.notes.is_empty(), "{:?}", r.notes);
}

#[test]
fn sc04_interrupted_then_released_after_the_resume() {
    let s = Synth::new()
        .down(E)
        .wait(ms(100))
        .pause(&[E])
        .wait(ms(3_000))
        .resume()
        .wait(ms(5))
        .down(E)
        .wait(ms(33))
        .down(E)
        .wait(ms(7))
        .up(E);
    assert_clean(&run(s));
}

#[test]
fn sc05_the_pause_key() {
    let with_ups = Synth::new()
        .down(0xE11D)
        .wait(20)
        .down(0x45)
        .wait(20)
        .up(0xE11D)
        .wait(20)
        .up(0x45)
        .wait(ms(3_000));
    assert_clean(&run(with_ups));
    let without = Synth::new()
        .down(0xE11D)
        .wait(20)
        .down(0x45)
        .wait(ms(3_000));
    assert_clean(&run(without));
}

#[test]
fn sc06_a_make_only_key() {
    assert_clean(&run(Synth::new().down(0x00F2).wait(ms(3_000))));
}

#[test]
fn sc07_a_key_down_as_the_test_ends() {
    let s = Synth::new().taps(G, 5, HOLD, GAP).down(ENTER).wait(ms(5));
    assert_clean(&s.end_now().diagnose());
}

#[test]
fn sc08_released_on_another_handle() {
    let s = Synth::new()
        .keyboard(&[1, 2])
        .down(E)
        .wait(ms(300))
        .on(2)
        .up(E)
        .on(1)
        .wait(ms(3_000));
    assert_clean(&run(s));
}

#[test]
fn sc09_the_test_ends_while_paused() {
    let s = Synth::new()
        .down(E)
        .down(G)
        .wait(ms(100))
        .pause(&[E, G])
        .wait(ms(5_000));
    assert_clean(&run(s));
}

#[test]
fn sc10_stuck_on_another_keyboard() {
    let s = Synth::new()
        .taps(G, 5, HOLD, GAP)
        .on(9)
        .down(E)
        .wait(ms(5_000));
    assert_clean(&run(s));
}

#[test]
fn sc11_shift_held_while_typing() {
    let s = Synth::new()
        .down(LSHIFT)
        .taps(G, 20, HOLD, (ms(100), ms(200)))
        .up(LSHIFT);
    assert_clean(&run(s));
}

// ---- notes ----

#[test]
fn sn01_a_release_that_rides_on_the_next_key() {
    let s = Synth::new()
        .taps(G, 5, HOLD, GAP)
        .hold(E, ms(4_000), ms(500), ms(33))
        .wait(100)
        .press(R, ms(100));
    let r = run(s);
    assert_clean(&r);
    assert!(r.notes.contains(&Note::ReleasedWithNextKey {
        key: E,
        held_ms: 4_000
    }));
}

#[test]
fn sn02_held_long_before_a_pause() {
    let s = Synth::new()
        .down(E)
        .wait(ms(2_500))
        .pause(&[E])
        .wait(ms(1_000))
        .resume();
    let r = run(s);
    assert_clean(&r);
    assert_eq!(
        r.notes,
        vec![Note::HeldAtPause {
            key: E,
            held_ms: 2_500
        }]
    );
}

// ---- found in review ----

#[test]
fn sf06_the_evidence_describes_the_open_hold() {
    let s = Synth::new()
        .round(E, 10, |s| s.hold(E, ms(6_000), ms(500), ms(33)))
        .wait(ms(1_000))
        .down(E)
        .wait(ms(2_500));
    let f = only(&s.end_now().diagnose(), Kind::Stuck, E);
    let Evidence::Stuck(e) = f.evidence else {
        panic!("not stuck: {f:#?}");
    };
    assert_eq!((e.held_ms, e.still_down), (2_500, true));
    assert_eq!(
        f.lines(&crate::fixture::label).evidence[0],
        "down for 2.5 s with no release by the end of the test"
    );
}

// Backspace held 3 s, then R pressed 3 ms after its release, which at 8 ms polling shares a report.
fn held_then_next_key(s: Synth) -> Synth {
    let tick = s.now() % ms(8);
    s.wait(ms(8) - tick + ms(1))
        .hold(0x0E, ms(3_000), ms(500), ms(33))
        .wait(ms(3))
        .press(R, ms(100))
        .wait(ms(300))
}

#[test]
fn sn03_one_same_report_release_at_8_ms_is_not_a_lost_release() {
    let s = held_then_next_key(Synth::new().taps(G, 30, HOLD, GAP));
    let r = run(s.taps(G, 10, HOLD, GAP).polled(ms(8)));
    assert_clean(&r);
    assert!(
        !r.notes
            .iter()
            .any(|n| matches!(n, Note::ReleasedWithNextKey { .. })),
        "{:#?}",
        r.notes
    );
}

#[test]
fn sn04_two_same_report_releases_at_8_ms_are_noted() {
    let s = held_then_next_key(held_then_next_key(Synth::new().taps(G, 30, HOLD, GAP)));
    let r = run(s.taps(G, 10, HOLD, GAP).polled(ms(8)));
    assert!(
        r.notes.contains(&Note::ReleasedWithNextKey {
            key: 0x0E,
            held_ms: 3_000
        }),
        "{:#?}",
        r.notes
    );
}
