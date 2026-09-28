use super::*;

#[test]
fn n01_pause_is_one_key_and_num_lock_stays_num_lock() {
    let s = Synth::new()
        .taps(0x45, 3, HOLD, GAP)
        .down(0xE11D)
        .wait(20)
        .down(0x45)
        .wait(20)
        .up(0xE11D)
        .wait(20)
        .up(0x45)
        .wait(ms(300))
        .taps(0x45, 3, HOLD, GAP);
    let r = run(s);
    assert_clean(&r);
    assert_eq!(r.aggregates.keys[&0x45].downs, 6);
    assert_eq!(r.aggregates.keys[&0xE11D].downs, 1);
}

#[test]
fn n02_pause_without_releases() {
    let r = run(Synth::new()
        .down(0xE11D)
        .wait(20)
        .down(0x45)
        .wait(ms(3_000)));
    assert_clean(&r);
    assert!(!r.aggregates.keys.contains_key(&0x45));
}

#[test]
fn n03_fake_shifts_are_dropped() {
    let s = Synth::new()
        .down(0xE02A)
        .wait(20)
        .down(INSERT)
        .wait(ms(90))
        .up(INSERT)
        .wait(20)
        .up(0xE02A)
        .wait(ms(200))
        .up(0xE036);
    let r = run(s);
    assert_clean(&r);
    assert_eq!(r.aggregates.limits.fake_shifts, 3);
    assert!(!r.aggregates.keys.contains_key(&0xE02A));
    assert!(!r.aggregates.keys.contains_key(&0xE036));
    assert_eq!(r.aggregates.keys[&INSERT].episodes, 1);
}

#[test]
fn n04_an_overrun_is_a_marker() {
    let r = run(Synth::new().press(0x00FF, ms(1)));
    assert_eq!(r.aggregates.limits.overruns, 1);
    assert!(r.aggregates.keys.is_empty());
}

#[test]
fn n05_injected_foreign_and_unknown_codes() {
    let s = Synth::new()
        .on(0)
        .press(E, ms(50))
        .on(9)
        .press(E, ms(50))
        .on(1)
        .press(0x0000, ms(50))
        .press(0xE000, ms(50));
    let r = run(s);
    let l = r.aggregates.limits;
    assert_eq!((l.injected, l.other_devices, l.unknown_codes), (2, 2, 4));
    assert!(r.aggregates.keys.is_empty());
}

#[test]
fn n06_num_lock_after_pause_but_not_with_it() {
    let s = Synth::new()
        .down(0xE11D)
        .wait(ms(5))
        .down(0x45)
        .wait(ms(80))
        .up(0x45);
    let r = run(s);
    assert_eq!(r.aggregates.keys[&0x45].downs, 1);
}

#[test]
fn n07_interrupted_keys_of_other_keyboards_are_ignored() {
    let s = Synth::new()
        .on(9)
        .pause(&[E])
        .on(1)
        .wait(ms(100))
        .resume()
        .press(E, ms(80));
    let r = run(s);
    assert_eq!(r.aggregates.keys[&E].episodes, 1);
}

// ---- found in review ----

#[test]
fn n08_a_pause_lists_only_keys_the_stream_holds() {
    let r = run(Synth::new()
        .down(keys::FAKE_LEFT_SHIFT)
        .wait(20)
        .down(INSERT)
        .wait(ms(500))
        .pause(&[keys::FAKE_LEFT_SHIFT, INSERT])
        .wait(ms(1_000))
        .resume()
        .up(INSERT)
        .up(keys::FAKE_LEFT_SHIFT));
    assert!(!r.aggregates.keys.contains_key(&keys::FAKE_LEFT_SHIFT));

    let s = Synth::new()
        .down(keys::PAUSE)
        .wait(20)
        .down(keys::NUM_LOCK)
        .wait(ms(1_000))
        .pause(&[keys::NUM_LOCK, keys::PAUSE])
        .wait(ms(1_000))
        .resume()
        .wait(ms(300));
    let s = (0..5).fold(s, |s, _| normal(s, keys::NUM_LOCK));
    let num_lock = run(s).aggregates.keys[&keys::NUM_LOCK];
    assert_eq!((num_lock.episodes, num_lock.after_resume), (5, 0));
}
