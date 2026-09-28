use super::*;
use crate::fixture::{Fixture, guided, guided_chatter, label, swap_chatter};
use crate::params::{SWAP_PRESSES, SWAP_ROUNDS};

const H: u16 = 0x23;

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

// ---- boards without a swap ----

// gd01's run: H skipped after 3 s in each of its rounds, between answered rounds of G and J.
fn dead_run() -> Fixture {
    guided(plan(&[G, H, J], 3, 10), |s, key, _| {
        if key == H {
            (s.wait(ms(3_000)), true)
        } else {
            (normal(s, key), false)
        }
    })
}

// E's first answer in its first round is held for 3 s with autorepeat.
fn stuck_run() -> Fixture {
    let mut first = true;
    guided(plan(&[G, J, E], 3, 10), move |s, key, _| {
        if key == E && first {
            first = false;
            (s.hold(E, ms(3_000), ms(500), ms(33)).wait(ms(200)), false)
        } else {
            (normal(s, key), false)
        }
    })
}

// The steps and causes that need a switch pulled from its socket.
fn pulls_a_switch(f: &Finding) -> Vec<&'static str> {
    let steps = f.next_tests.iter().filter_map(|t| match t {
        NextTest::SwapSwitch { .. } => Some("swap"),
        NextTest::ReseatSwitch { .. } => Some("reseat"),
        NextTest::BridgeSocket { .. } => Some("bridge"),
        _ => None,
    });
    let causes = f.causes.iter().filter_map(|c| match c {
        Cause::HotSwapSocket => Some("socket"),
        Cause::SwitchSeating => Some("seating"),
        _ => None,
    });
    steps.chain(causes).collect()
}

type Build = fn() -> Fixture;

#[test]
fn sw06_soldered_and_laptop_boards_get_no_swap_step() {
    let runs: [(Kind, u16, Build); 3] = [
        (Kind::Chatter, E, || guided_chatter().1),
        (Kind::Dead, H, dead_run),
        (Kind::Stuck, E, stuck_run),
    ];
    let mut on_hot_swap = std::collections::BTreeSet::new();
    for (kind, key, run) in runs {
        for board in [BoardKind::Soldered, BoardKind::Laptop] {
            let r = Fixture { board, ..run() }.diagnose();
            let f = only(&r, kind, key);
            assert!(!f.next_tests.is_empty(), "{board:?} {f:#?}");
            assert_eq!(pulls_a_switch(&f), [""; 0], "{board:?} {kind:?}");
        }
        // Positive control: the same run on a hot-swap board carries the swap.
        let r = hot_swap(run()).diagnose();
        let f = only(&r, kind, key);
        assert_eq!(partner(&f), Some(G), "{kind:?}");
        on_hot_swap.extend(pulls_a_switch(&f));
    }
    assert_eq!(
        on_hot_swap.into_iter().collect::<Vec<_>>(),
        ["bridge", "reseat", "seating", "socket", "swap"]
    );
}

// ---- the partner and other notes ----

// G in rounds of 10 and J in rounds of 20, both clean, and E's guided chatter, on a hot-swap board.
// J has the most clean presses, so it is the partner unless something else counts against it.
fn partner_after(extra: impl FnOnce(Synth) -> Synth) -> Report {
    let mut s = Synth::new().board(BoardKind::HotSwap);
    for _ in 0..3 {
        s = s
            .round(G, 10, |s| s.taps(G, 10, HOLD, GAP))
            .round(J, 20, |s| s.taps(J, 20, HOLD, GAP))
            .round(E, 10, |s| {
                (1..=10).fold(s, |s, n| {
                    if n % 5 == 0 {
                        fault(s, E)
                    } else {
                        normal(s, E)
                    }
                })
            });
    }
    extra(s).build().diagnose()
}

type Extra = fn(Synth) -> Synth;

#[test]
fn sw07_the_partner_skips_a_key_another_note_names() {
    let r = partner_after(|s| s);
    assert_eq!(partner(&only(&r, Kind::Chatter, E)), Some(J));
    assert_eq!(clean_presses(&r, J), Some(60));

    let cases: [(&str, Extra); 6] = [
        ("Unprompted", |s| fault(fault(s, J), J)),
        ("ReleasedWithNextKey", |s| {
            s.down(J)
                .repeats(J, ms(500), ms(33), 60)
                .wait(ms(20))
                .up(J)
                .press(F, ms(90))
        }),
        ("HeldAtPause", |s| {
            s.down(J).wait(ms(2_500)).pause(&[J]).wait(ms(500)).resume()
        }),
        ("NotAssessed", |s| {
            s.round(J, 10, |s| s.pause(&[]).wait(ms(4_000)).resume())
        }),
        ("DifferentCode", |s| {
            s.round(J, 10, |s| s.taps(F, 10, HOLD, GAP))
        }),
        ("DifferentCode", |s| {
            s.round(K, 10, |s| s.taps(J, 10, HOLD, GAP))
        }),
    ];
    for (variant, extra) in cases {
        let r = partner_after(extra);
        let f = only(&r, Kind::Chatter, E);
        assert!(
            r.notes.iter().any(|n| format!("{n:?}").starts_with(variant)
                && crate::aggregate::named(n)
                    .iter()
                    .flatten()
                    .any(|&(key, _)| key == J)),
            "{variant}: {:#?}",
            r.notes
        );
        assert!(clean_presses(&r, J) > clean_presses(&r, G), "{variant}");
        assert_eq!(partner(&f), Some(G), "{variant}");
    }
}

// ---- the offer ----

// guided_chatter on the Start screen's default board: one Very high chatter finding on E.
fn offered() -> Swap {
    Swap::offer(&hot_swap(guided_chatter().1).diagnose(), BoardKind::HotSwap).expect("an offer")
}

#[test]
fn sw08_the_offer_is_hot_swap_only_and_needs_a_partner() {
    let unknown = guided_chatter().1.diagnose();
    assert!(
        unknown.findings[0]
            .next_tests
            .contains(&NextTest::SwapSwitch {
                suspect: E,
                partner: Some(G)
            })
    );
    for board in [BoardKind::Unknown, BoardKind::Soldered, BoardKind::Laptop] {
        let r = Fixture {
            board,
            ..guided_chatter().1
        }
        .diagnose();
        assert_eq!(Swap::offer(&r, board), None, "{board:?}");
    }
    // Positive control: the unknown board's report offers a swap once the board is hot-swap.
    assert!(Swap::offer(&unknown, BoardKind::HotSwap).is_some());

    let clean = hot_swap(guided(plan(&[G, J, E], 3, 10), typist(&[], 5, &[]))).diagnose();
    assert_clean(&clean);
    assert_eq!(Swap::offer(&clean, BoardKind::HotSwap), None);
    let unnamed = hot_swap(guided(plan(&[G, E], 3, 3), typist(&[E], 2, &[]))).diagnose();
    assert_eq!(partner(&only(&unnamed, Kind::Chatter, E)), None);
    assert_eq!(Swap::offer(&unnamed, BoardKind::HotSwap), None);

    // R chatters in every round and E in two, so R comes first although E's code is lower.
    let mut answers = 0;
    let two = hot_swap(guided(plan(&[G, R, E], 3, 10), |s, key, n| {
        let s = match key {
            R if n % 5 == 0 => fault(s, R),
            E => {
                answers += 1;
                if matches!(answers, 5 | 15) {
                    fault(s, E)
                } else {
                    normal(s, E)
                }
            }
            _ => normal(s, key),
        };
        (s, false)
    }))
    .diagnose();
    let order: Vec<(u16, Confidence)> =
        two.findings.iter().map(|f| (f.key, f.confidence)).collect();
    assert_eq!(order, [(R, Confidence::VeryHigh), (E, Confidence::Medium)]);
    assert_eq!(partner(&two.findings[1]), Some(G));
    assert_eq!(
        Swap::offer(&two, BoardKind::HotSwap).map(|s| (s.suspect, s.partner)),
        Some((R, G))
    );

    let offer = offered();
    assert_eq!(
        offer,
        Swap {
            suspect: E,
            partner: G,
            kind: Kind::Chatter,
            before: Confidence::VeryHigh,
            floor_permille: 95,
        }
    );
    assert_eq!(offer.plan(), plan(&[E, G], SWAP_ROUNDS, SWAP_PRESSES));
    assert!(Guide::new(offer.plan(), &[1]).is_ok());
    // SWAP_ROUNDS' reason: a clean key's bound clears a High finding from 30 presses.
    assert_eq!(
        rule_of_three_permille(u32::from(SWAP_ROUNDS * SWAP_PRESSES)),
        34
    );
    assert_eq!(wilson_floor_permille(3, 30), 34);
}

// ---- the Done-when, at the engine ----

fn follows_stream() -> Report {
    swap_chatter(offered().plan(), &[G]).diagnose()
}

fn stays_stream() -> Report {
    swap_chatter(offered().plan(), &[E]).diagnose()
}

const E_CLEAR: &str =
    "E: no extra key-downs in 90 presses, so a rate above 3.4% would very likely have shown";
const G_CLEAR: &str =
    "G: no extra key-downs in 90 presses, so a rate above 3.4% would very likely have shown";

#[test]
fn m4_done_fault_follows_the_switch() {
    let offer = offered();
    let f = swap_chatter(offer.plan(), &[G]);
    // A real Guide stamped the rounds, suspect first.
    let keys: Vec<(u16, u16)> = f.rounds.iter().map(|r| (r.key, r.asked)).collect();
    assert_eq!(keys, [(E, 30), (G, 30), (E, 30), (G, 30), (E, 30), (G, 30)]);
    let r = offer.judge(&f.diagnose());
    assert_eq!(
        (r.outcome, r.confidence, r.capped),
        (Outcome::Follows, Some(Confidence::VeryHigh), false)
    );
    assert_eq!((r.tile(), r.flagged()), (Some(G), vec![G]));
    assert_eq!(r.suspect.status, Status::Clear);
    assert!(r.suspect.bound_permille <= offer.floor_permille);
    assert_eq!((r.suspect.presses, r.suspect.bound_permille), (90, 34));
    let Status::Shows(g) = &r.partner.status else {
        panic!("{:#?}", r.partner)
    };
    let e = chatter_of(g);
    assert_eq!((e.affected, e.presses, e.rounds_affected), (18, 90, 3));
    assert_eq!(
        r.lines(&label),
        OutcomeLines {
            title: "The fault moved with the switch.".into(),
            evidence: vec![
                E_CLEAR.into(),
                "G: 18 of 90 presses sent an extra key-down (a rate of at least 13%)".into(),
            ],
            diagnosis: "The E switch now sits in the G socket, and the fault appeared there. The \
                        switch is the most likely cause. The E socket and the PCB behaved \
                        normally with a known-good switch."
                .into(),
            next: vec![
                "Replace the switch that came from E, now in the G socket, with a switch of the \
                 same model. Then test G again."
                    .into(),
                "Blow out the G switch with the key held down, or work contact cleaner into it \
                 while pressing it many times. Then test G again."
                    .into(),
            ],
        }
    );
    // Control: the fault left on E doesn't read as moved.
    assert_ne!(offer.judge(&stays_stream()).outcome, Outcome::Follows);
}

#[test]
fn m4_done_fault_stays() {
    let offer = offered();
    let r = offer.judge(&stays_stream());
    assert_eq!(
        (r.outcome, r.confidence, r.capped),
        (Outcome::Stays, Some(Confidence::VeryHigh), false)
    );
    assert_eq!((r.tile(), r.flagged()), (Some(E), vec![E]));
    assert_eq!(r.partner.status, Status::Clear);
    assert_eq!(
        r.lines(&label),
        OutcomeLines {
            title: "The fault stayed on E.".into(),
            evidence: vec![
                "E: 18 of 90 presses sent an extra key-down (a rate of at least 13%)".into(),
                G_CLEAR.into(),
            ],
            diagnosis: "A known-good switch in the E socket shows the same fault. The socket, the \
                        solder joints under it, or the matrix trace is the most likely cause. The \
                        original switch is probably fine."
                .into(),
            next: vec![
                "Inspect the E socket for a loose pin and the joints under it for a crack. \
                 Reflowing the socket pins is a small job. If the board is under warranty, this \
                 result is what the vendor needs."
                    .into(),
            ],
        }
    );
    // Control: the fault moved to G doesn't read as staying.
    assert_ne!(offer.judge(&follows_stream()).outcome, Outcome::Stays);
}

#[test]
fn sw09_the_judgment_reads_which_key_shows_it() {
    let offer = offered();
    let turned = Swap {
        suspect: G,
        partner: E,
        ..offer
    };
    let r = turned.judge(&follows_stream());
    assert_eq!((r.outcome, r.tile()), (Outcome::Stays, Some(G)));
    let r = turned.judge(&stays_stream());
    assert_eq!((r.outcome, r.tile()), (Outcome::Follows, Some(E)));
}

#[test]
fn sw10_the_fault_on_both_keys_is_both_never_stays() {
    let offer = offered();
    let both = swap_chatter(offer.plan(), &[E, G]).diagnose();
    let r = offer.judge(&both);
    assert_eq!(
        (r.outcome, r.confidence, r.tile(), r.flagged()),
        (
            Outcome::Both,
            Some(Confidence::VeryHigh),
            Some(E),
            vec![E, G]
        )
    );
    let l = r.lines(&label);
    assert_eq!(l.title, "The fault showed on both keys.");
    assert_eq!(
        l.diagnosis,
        "The fault showed on E with the known-good switch and on G with the E switch, so this \
         swap can't tell the switch from the socket. That points past a single switch, to \
         firmware debounce or the keyboard as a whole."
    );
    assert_eq!(l.next, [NextTest::RaiseDebounce.words(&label)]);
    // The weakest of the three tests decides.
    let high = Swap {
        before: Confidence::High,
        ..offer
    };
    assert_eq!(high.judge(&both).confidence, Some(Confidence::High));
}

// Words that would say a part is mended, which a swap test can't show.
pub(super) fn claims_a_cure(text: &str) -> bool {
    let lower = text.to_lowercase();
    ["fixed", "cured", "resolved", "solved", "repaired"]
        .iter()
        .any(|w| lower.contains(w))
}

fn every_line(l: &OutcomeLines) -> Vec<String> {
    let mut out = vec![l.title.clone(), l.diagnosis.clone()];
    out.extend(l.evidence.iter().cloned());
    out.extend(l.next.iter().cloned());
    out
}

#[test]
fn sw11_neither_key_showing_it_is_gone_at_low() {
    let offer = offered();
    let r = offer.judge(&swap_chatter(offer.plan(), &[]).diagnose());
    assert_eq!(
        (r.outcome, r.confidence, r.tile(), r.flagged()),
        (Outcome::Gone, Some(Confidence::Low), None, vec![])
    );
    assert_eq!(
        r.lines(&label),
        OutcomeLines {
            title: "Neither key showed the fault.".into(),
            evidence: vec![E_CLEAR.into(), G_CLEAR.into()],
            diagnosis: "Both keys registered normally after the swap. Reseating the switches may \
                        have cleared a poor contact, or the fault comes and goes and didn't show \
                        in this test."
                .into(),
            next: vec![
                "Use the keyboard for a day. If the fault returns on E, repeat this swap test. If \
                 it returns on G, the switch that came from E is the likely cause."
                    .into(),
            ],
        }
    );
    let unclear = offer.judge(&ended_early());
    assert_eq!(unclear.outcome, Outcome::Unclear);
    for text in every_line(&r.lines(&label))
        .iter()
        .chain(&every_line(&unclear.lines(&label)))
    {
        assert!(!claims_a_cure(text), "{text}");
    }
    assert!(claims_a_cure("The fault is fixed."));
}

#[test]
fn sw12_fast_deliberate_double_presses_on_both_keys_read_as_gone() {
    let offer = offered();
    let after = hot_swap(guided(offer.plan(), typist(&[], 5, &[E, G]))).diagnose();
    assert_clean(&after);
    let r = offer.judge(&after);
    assert_eq!(r.outcome, Outcome::Gone);
    assert!(r.suspect.presses > 90 && r.partner.presses > 90);
}

// The gone stream, ended after the suspect's first round.
fn ended_early() -> Report {
    first_round_of(&swap_chatter(offered().plan(), &[]))
}

fn first_round_of(f: &Fixture) -> Report {
    let end = f.rounds[1].start_us;
    let entries: Vec<Entry> = f
        .entries
        .iter()
        .filter(|e| e.micros() < end)
        .cloned()
        .collect();
    diagnose(&Session {
        entries: &entries,
        end_us: end,
        rounds: &f.rounds[..1],
        ..f.session()
    })
}

#[test]
fn sw13_a_swap_test_ended_early_is_unclear() {
    let offer = offered();
    let r = offer.judge(&ended_early());
    assert_eq!(r.suspect.status, Status::Short(Gap::TooFew));
    assert_eq!(
        (
            r.suspect.presses,
            r.suspect.bound_permille,
            offer.floor_permille
        ),
        (30, 100, 95)
    );
    assert_eq!(r.partner.status, Status::Short(Gap::Untested));
    assert_eq!((r.outcome, r.confidence), (Outcome::Unclear, None));
    let l = r.lines(&label);
    assert_eq!(l.title, "This swap test can't place the fault.");
    assert_eq!(
        l.evidence,
        [
            "E: no extra key-downs in 30 presses, too few to rule out the first test's rate of at \
             least 9.5%",
            "G: no key-down arrived in its own rounds, so it can't be compared",
        ]
    );
    assert_eq!(
        l.diagnosis,
        "This swap test gave too little evidence on E and G to compare with the first test, so \
         the first test's finding still stands."
    );
}

#[test]
fn sw14_one_extra_key_down_on_the_clean_side_is_unclear() {
    let offer = offered();
    let mut once = true;
    let after = hot_swap(guided(offer.plan(), |s, key, n| {
        if key == G && n == 5 && once {
            once = false;
            (fault(s, G), false)
        } else {
            (normal(s, key), false)
        }
    }))
    .diagnose();
    assert_clean(&after);
    assert!(after.notes.contains(&Note::OneExtraDown {
        key: G,
        presses: 90,
        extra_downs: 1
    }));
    let r = offer.judge(&after);
    assert_eq!(r.suspect.status, Status::Clear);
    assert_eq!(r.partner.status, Status::Short(Gap::OneExtraDown));
    assert_eq!((r.outcome, r.confidence), (Outcome::Unclear, None));
    assert_eq!(
        r.lines(&label).evidence[1],
        "G: 1 of 90 presses sent an extra key-down, one short of a finding"
    );
}

#[test]
fn sw15_a_short_retest_caps_the_result_at_medium() {
    let offer = offered();
    let short = Plan {
        rounds: 2,
        presses: 15,
        ..offer.plan()
    };
    let after = swap_chatter(short, &[G]).diagnose();
    let g = only(&after, Kind::Chatter, G);
    let e = chatter_of(&g);
    assert_eq!(
        (
            g.confidence,
            e.affected,
            e.presses,
            e.rounds_affected,
            e.rounds
        ),
        (Confidence::High, 6, 30, 2, 2)
    );
    let r = offer.judge(&after);
    assert_eq!(r.suspect.status, Status::Short(Gap::TooFew));
    assert_eq!(r.suspect.bound_permille, 100);
    assert_eq!(
        (r.outcome, r.confidence, r.capped),
        (Outcome::Follows, Some(Confidence::Medium), true)
    );
    assert!(r.lines(&label).diagnosis.ends_with(
        "The E socket gave too little evidence with the known-good switch to clear it, so it \
         remains possible too."
    ));
    // Control: a first test that proved no more than E's bound lets E clear, and the cap goes.
    let weaker = Swap {
        floor_permille: 100,
        ..offer
    };
    let r = weaker.judge(&after);
    assert_eq!(
        (r.outcome, r.confidence, r.capped),
        (Outcome::Follows, Some(Confidence::High), false)
    );
}

fn skipping(offer: &Swap, skipped: u16) -> Report {
    hot_swap(guided(offer.plan(), move |s, key, _| {
        if key == skipped {
            (s.wait(ms(3_000)), true)
        } else {
            (normal(s, key), false)
        }
    }))
    .diagnose()
}

#[test]
fn sw16_a_dead_key_follows_or_stays() {
    let offer = Swap::offer(&hot_swap(dead_run()).diagnose(), BoardKind::HotSwap).unwrap();
    assert_eq!(
        offer,
        Swap {
            suspect: H,
            partner: G,
            kind: Kind::Dead,
            before: Confidence::VeryHigh,
            floor_permille: 0,
        }
    );
    assert!(offer.lines(&label).known_good.ends_with(
        "A key that sends nothing doesn't move the prompt on, so press Skip this key once you \
         have pressed it 30 times."
    ));

    // G's last round has no later round to bracket it, so its finding is High.
    let r = offer.judge(&skipping(&offer, G));
    assert_eq!(
        (r.outcome, r.confidence, r.capped, r.tile()),
        (Outcome::Follows, Some(Confidence::High), false, Some(G))
    );
    assert_eq!(
        r.lines(&label).evidence,
        [
            "H: a key-down in each of its 3 rounds",
            "G: no key-down in 3 of 3 rounds (90 presses asked)",
        ]
    );

    let r = offer.judge(&skipping(&offer, H));
    assert_eq!(
        (r.outcome, r.confidence, r.capped, r.tile()),
        (Outcome::Stays, Some(Confidence::High), false, Some(H))
    );
    let l = r.lines(&label);
    assert!(
        l.diagnosis
            .contains("A keymap, layer or Windows remap for H would also stay with the key."),
        "{}",
        l.diagnosis
    );
    assert_eq!(l.next[0], NextTest::CheckKeymap { key: H }.words(&label));
}

// The first `holds` answers of `held` are each held for 3 s, in its first round.
fn holding(offer: &Swap, held: u16, holds: u32) -> Report {
    let mut left = holds;
    hot_swap(guided(offer.plan(), move |s, key, _| {
        if key == held && left > 0 {
            left -= 1;
            (s.hold(key, ms(3_000), ms(500), ms(33)).wait(ms(200)), false)
        } else {
            (normal(s, key), false)
        }
    }))
    .diagnose()
}

#[test]
fn sw17_a_stuck_key_follows_or_is_gone() {
    let offer = Swap::offer(&hot_swap(stuck_run()).diagnose(), BoardKind::HotSwap).unwrap();
    assert_eq!(
        offer,
        Swap {
            suspect: E,
            partner: G,
            kind: Kind::Stuck,
            before: Confidence::Medium,
            floor_permille: 0,
        }
    );
    let after = holding(&offer, G, 1);
    let g = only(&after, Kind::Stuck, G);
    let r = offer.judge(&after);
    assert_eq!(
        (r.outcome, r.confidence, r.capped),
        (
            Outcome::Follows,
            Some(offer.before.min(g.confidence)),
            false
        )
    );
    // Two held presses in its own round make G's finding Very high, and the result still rests
    // on the first test's Medium.
    let after = holding(&offer, G, 2);
    assert_eq!(
        only(&after, Kind::Stuck, G).confidence,
        Confidence::VeryHigh
    );
    assert_eq!(offer.judge(&after).confidence, Some(Confidence::Medium));
    assert_eq!(r.suspect.status, Status::Clear);
    assert_eq!(
        r.lines(&label).evidence[0],
        "E: every press in its 3 rounds released within 2 s"
    );

    let clean = hot_swap(guided(offer.plan(), typist(&[], 5, &[]))).diagnose();
    let r = offer.judge(&clean);
    assert_eq!(
        (r.outcome, r.confidence),
        (Outcome::Gone, Some(Confidence::Low))
    );
}

#[test]
fn sw18_another_kind_after_the_swap_is_listed_not_judged() {
    let offer = offered();
    let after = skipping(&offer, G);
    let dead = only(&after, Kind::Dead, G);
    assert_eq!(dead.confidence, Confidence::High);
    let r = offer.judge(&after);
    assert_eq!(r.partner.status, Status::Short(Gap::Untested));
    assert_eq!(r.partner.also, [dead]);
    assert_eq!((r.outcome, r.confidence), (Outcome::Unclear, None));
    let l = r.lines(&label);
    assert_eq!(
        l.evidence,
        [
            E_CLEAR,
            "G: no key-down arrived in its own rounds, so it can't be compared",
            "G: also possible dead key, high confidence",
        ]
    );
    assert_eq!(
        l.diagnosis,
        "This swap test gave too little evidence on G to compare with the first test, so the \
         first test's finding still stands."
    );
    // Positive control: the same stream judged for a dead key reads as moved.
    let dead_swap = Swap {
        kind: Kind::Dead,
        floor_permille: 0,
        ..offer
    };
    assert_eq!(dead_swap.judge(&after).outcome, Outcome::Follows);
}

#[test]
fn sw19_every_other_gap_leaves_its_side_uncleared() {
    let dead = Swap::offer(&hot_swap(dead_run()).diagnose(), BoardKind::HotSwap).unwrap();
    let stuck = Swap::offer(&hot_swap(stuck_run()).diagnose(), BoardKind::HotSwap).unwrap();
    let evidence = |swap: &Swap, after: &Report| {
        let r = swap.judge(after);
        assert_eq!((r.outcome, r.confidence), (Outcome::Unclear, None));
        r.lines(&label).evidence
    };

    // Software typed during G's first round, which it never answered, so that round isn't counted.
    let mut first = true;
    let injected = hot_swap(guided(dead.plan(), |s, key, _| {
        if key == G && first {
            first = false;
            (s.on(0).press(F, ms(80)).on(1).wait(ms(3_000)), true)
        } else {
            (normal(s, key), false)
        }
    }))
    .diagnose();
    assert_eq!(
        evidence(&dead, &injected),
        [
            "H: a key-down in each of its 3 rounds",
            "G: 1 of its rounds wasn't counted, so it can't be cleared",
        ]
    );

    let dead_early = first_round_of(&hot_swap(guided(dead.plan(), typist(&[], 5, &[]))));
    assert_eq!(
        evidence(&dead, &dead_early),
        [
            "H: tested in 1 of 3 rounds, too few to compare",
            "G: no key-down arrived in its own rounds, so it can't be compared",
        ]
    );

    // G was down 2.5 s when the test paused, which is never counted as stuck.
    let mut first = true;
    let paused = hot_swap(guided(stuck.plan(), |s, key, _| {
        if key == G && first {
            first = false;
            let s = s.down(G).wait(ms(2_500)).pause(&[G]).wait(ms(500)).resume();
            (s.wait(ms(300)).up(G).wait(ms(300)), false)
        } else {
            (normal(s, key), false)
        }
    }))
    .diagnose();
    assert_eq!(
        evidence(&stuck, &paused),
        [
            "E: every press in its 3 rounds released within 2 s",
            "G: a press stayed down 2 s or more without a finding, so it can't be cleared",
        ]
    );

    let stuck_early = first_round_of(&hot_swap(guided(stuck.plan(), typist(&[], 5, &[]))));
    assert_eq!(
        evidence(&stuck, &stuck_early),
        [
            "E: tested in 1 of 3 rounds, too few to compare",
            "G: no key-down arrived in its own rounds, so it can't be compared",
        ]
    );

    // At 16 ms or slower, extra key-downs with as many near misses give no finding. The report is
    // edited to that state, since the judgment reads only the report.
    let offer = offered();
    let mut coarse = swap_chatter(offer.plan(), &[]).diagnose();
    coarse
        .notes
        .retain(|n| !matches!(*n, Note::Clean { key, .. } if key == G));
    if let Some(p) = coarse
        .aggregates
        .keys
        .get_mut(&G)
        .and_then(|a| a.prompted.as_mut())
    {
        p.affected = 3;
    }
    assert_eq!(
        evidence(&offer, &coarse)[1],
        "G: extra key-downs in 90 presses that a keyboard reporting every 16 ms or slower can't \
         tell from fast presses"
    );
}

// A key goes down just before J's first answer and its release is lost, so it is still down at the
// end. Only G, J and E are prompted.
fn left_down(key: u16) -> Report {
    let mut first = true;
    hot_swap(guided(plan(&[G, J, E], 3, 10), move |s, k, _| {
        let s = if k == J && first {
            first = false;
            s.down(key).wait(ms(200))
        } else {
            s
        };
        (normal(s, k), false)
    }))
    .diagnose()
}

#[test]
fn sw20_a_key_the_main_test_never_prompted_gets_no_offer() {
    const ESC: u16 = 0x01;
    const F1: u16 = 0x3B;
    const LWIN: u16 = 0xE05B;
    for key in [SPACE, ESC, F1, LWIN, LSHIFT] {
        let r = left_down(key);
        let f = only(&r, Kind::Stuck, key);
        assert!(f.confidence >= Confidence::Medium, "{key:04X} {f:#?}");
        // The card keeps its swap step. Only the guided retest, which would prompt the key, is
        // held back.
        assert_eq!(partner(&f), Some(E), "{key:04X}");
        assert_eq!(r.aggregates.keys[&key].prompted, None, "{key:04X}");
        assert!(Guide::new(plan(&[key], 1, 1), &[1]).is_ok(), "{key:04X}");
        assert_eq!(Swap::offer(&r, BoardKind::HotSwap), None, "{key:04X}");
    }

    // Positive control: stuck_run's E was prompted, and gets its offer.
    let r = hot_swap(stuck_run()).diagnose();
    assert!(r.aggregates.keys[&E].prompted.is_some());
    assert_eq!(
        Swap::offer(&r, BoardKind::HotSwap).map(|s| (s.suspect, s.kind)),
        Some((E, Kind::Stuck))
    );

    // An unprompted finding ranked first doesn't hide a prompted one after it.
    let mut first = true;
    let mut held = true;
    let r = hot_swap(guided(plan(&[G, J, E], 3, 10), move |s, k, _| {
        let s = if k == J && held {
            held = false;
            s.down(SPACE).wait(ms(200))
        } else {
            s
        };
        if k == E && first {
            first = false;
            (s.hold(E, ms(3_000), ms(500), ms(33)).wait(ms(200)), false)
        } else {
            (normal(s, k), false)
        }
    }))
    .diagnose();
    let order: Vec<(u16, Kind)> = r.findings.iter().map(|f| (f.key, f.kind())).collect();
    assert_eq!(order, [(SPACE, Kind::Stuck), (E, Kind::Stuck)]);
    assert_eq!(
        Swap::offer(&r, BoardKind::HotSwap).map(|s| (s.suspect, s.partner)),
        Some((E, G))
    );
}
