use super::*;
use crate::fixture::{Fixture, guided, guided_chatter};

fn plan(keys: &[u16], rounds: u16, presses: u16) -> Plan {
    Plan {
        keys: keys.to_vec(),
        rounds,
        presses,
    }
}

fn hot_swap(f: Fixture) -> Fixture {
    Fixture {
        board: BoardKind::HotSwap,
        ..f
    }
}

// guided_chatter's fault: a 5 ms press and a 100 ms one, 5 ms apart.
fn fault(s: Synth, key: u16) -> Synth {
    s.fragments(key, &[ms(5), ms(5), ms(100)]).wait(ms(200))
}

// gc02's typist: two deliberate presses in quick succession, which is never chatter.
fn double(s: Synth, key: u16) -> Synth {
    s.taps(key, 2, (ms(40), ms(100)), (ms(40), ms(80)))
        .wait(ms(250))
}

// Every `every`th answer of a key in `chatters` is the fault, every answer of a key in `doubles` is
// a double press, and every other answer is a normal press.
fn typist<'a>(
    chatters: &'a [u16],
    every: u32,
    doubles: &'a [u16],
) -> impl FnMut(Synth, u16, u32) -> (Synth, bool) + 'a {
    move |s, key, n| {
        let s = if chatters.contains(&key) && n % every == 0 {
            fault(s, key)
        } else if doubles.contains(&key) {
            double(s, key)
        } else {
            normal(s, key)
        };
        (s, false)
    }
}

fn partner(f: &Finding) -> Option<u16> {
    f.next_tests
        .iter()
        .find_map(|t| match *t {
            NextTest::SwapSwitch { suspect, partner } => {
                assert_eq!(suspect, f.key);
                Some(partner)
            }
            _ => None,
        })
        .expect("a swap step")
}

fn clean_presses(r: &Report, key: u16) -> Option<u32> {
    r.notes.iter().find_map(|n| match *n {
        Note::Clean {
            key: k, presses, ..
        } if k == key => Some(presses),
        _ => None,
    })
}

// ---- the known-good partner ----

#[test]
fn sw01_no_clean_key_leaves_the_partner_unnamed() {
    let r = hot_swap(guided(plan(&[G, E], 3, 3), typist(&[E], 2, &[]))).diagnose();
    let f = only(&r, Kind::Chatter, E);
    assert_eq!(clean_presses(&r, G), None);
    assert_eq!(partner(&f), None);

    // Positive control: one more press a round takes G past TESTED_PRESSES, and it is named.
    let r = hot_swap(guided(plan(&[G, E], 3, 4), typist(&[E], 2, &[]))).diagnose();
    let f = only(&r, Kind::Chatter, E);
    assert_eq!(clean_presses(&r, G), Some(12));
    assert_eq!(partner(&f), Some(G));
}

#[test]
fn sw02_a_flagged_key_is_never_the_partner() {
    // R holds through its first round, which is a stuck finding. Its double presses are clean and
    // outnumber G's, so only its finding keeps it from being the partner.
    let mut first = true;
    let mut rest = typist(&[E], 5, &[R]);
    let f = guided(plan(&[G, R, E], 3, 10), |s, key, n| {
        if key == R && first {
            first = false;
            (s.hold(R, ms(3_000), ms(500), ms(33)).wait(ms(200)), false)
        } else {
            rest(s, key, n)
        }
    });
    let r = hot_swap(f).diagnose();
    assert_eq!(r.findings.len(), 2, "{:#?}", r.findings);
    assert!(
        r.findings
            .iter()
            .any(|f| (f.kind(), f.key) == (Kind::Stuck, R))
    );
    assert!(clean_presses(&r, R) > clean_presses(&r, G));
    assert_eq!(clean_presses(&r, G), Some(30));
    for f in &r.findings {
        assert_eq!(partner(f), Some(G), "{f:#?}");
    }
}

#[test]
fn sw03_a_plain_key_is_preferred() {
    let r = hot_swap(guided(
        plan(&[G, SPACE, E], 3, 10),
        typist(&[E], 5, &[SPACE]),
    ))
    .diagnose();
    let f = only(&r, Kind::Chatter, E);
    assert!(Guide::new(plan(&[SPACE], 1, 1), &[1]).is_ok());
    assert!(!crate::keys::is_plain(SPACE));
    assert!(clean_presses(&r, SPACE) > clean_presses(&r, G));
    assert_eq!(partner(&f), Some(G));
}

#[test]
fn sw04_more_clean_presses_win_then_the_lower_scan_code() {
    let r = hot_swap(guided(plan(&[G, J, E], 3, 10), typist(&[E], 5, &[J]))).diagnose();
    assert!(clean_presses(&r, J) > clean_presses(&r, G));
    assert_eq!(partner(&only(&r, Kind::Chatter, E)), Some(J));

    let r = hot_swap(guided_chatter().1).diagnose();
    assert_eq!(clean_presses(&r, G), Some(30));
    assert_eq!(clean_presses(&r, J), Some(30));
    assert_eq!(partner(&only(&r, Kind::Chatter, E)), Some(G));

    // The lower scan code wins the tie, not the key prompted first.
    let r = hot_swap(guided(plan(&[J, G, E], 3, 10), typist(&[E], 5, &[]))).diagnose();
    assert_eq!(partner(&only(&r, Kind::Chatter, E)), Some(G));
}

#[test]
fn sw05_every_finding_names_the_same_partner() {
    let r = hot_swap(guided(plan(&[G, R, E], 3, 10), typist(&[E, R], 5, &[]))).diagnose();
    let keys: Vec<(Kind, u16)> = r.findings.iter().map(|f| (f.kind(), f.key)).collect();
    assert_eq!(keys, [(Kind::Chatter, E), (Kind::Chatter, R)]);
    for f in &r.findings {
        assert_eq!(partner(f), Some(G), "{f:#?}");
    }
}
