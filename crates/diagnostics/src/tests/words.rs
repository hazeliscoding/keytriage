use super::*;
use crate::fixture::label;

fn canned(board: BoardKind) -> Report {
    super::chatter_rounds([5, 5, 4], |s| {
        s.fragments(E, &[ms(5), ms(5), ms(100)]).wait(ms(200))
    })
    .board(board)
    .build()
    .diagnose()
}

#[test]
fn w01_the_canned_finding_renders() {
    let r = canned(BoardKind::Unknown);
    let lines = r.findings[0].lines(&label);
    assert_eq!(lines.headline, "possible chatter, very high confidence");
    assert_eq!(
        lines.evidence,
        [
            "14 of 100 presses sent an extra key-down (a rate of at least 8.5%)",
            "the extra key-downs came 5 ms after a release",
            "the extra presses lasted 5 ms",
            "reproduced in 3 of 3 rounds",
            "none of the 2 other tested keys showed it",
            "the keyboard's own debounce hides contact bounce shorter than its setting, often 5 ms",
        ]
    );
    assert_eq!(
        lines.causes,
        [
            "switch contacts",
            "hot-swap socket or solder joint",
            "firmware debounce"
        ]
    );
    assert_eq!(
        lines.next,
        [
            "Say whether the keyboard is hot-swap, soldered or a laptop keyboard. The next steps differ.",
            "Swap the E switch with the G switch and test both keys again. If the fault moves to G, \
             the switch is the likely cause. If it stays on E, look at the socket or the PCB.",
        ]
    );
    assert_eq!(
        NextTest::SwapSwitch {
            suspect: E,
            partner: None
        }
        .words(&label),
        "Swap the E switch with the switch of a key that tested clean, and test both keys again. \
         If the fault moves with the switch, the switch is the likely cause. If it stays on E, \
         look at the socket or the PCB."
    );
}

#[test]
fn w01b_a_hot_swap_board_leads_with_the_swap() {
    let r = canned(BoardKind::HotSwap);
    assert_eq!(
        r.findings[0].next_tests[0],
        NextTest::SwapSwitch {
            suspect: E,
            partner: Some(G)
        }
    );
}

fn everything() -> Vec<Report> {
    let mut out = vec![canned(BoardKind::Unknown)];
    for board in [
        BoardKind::HotSwap,
        BoardKind::Soldered,
        BoardKind::Laptop,
        BoardKind::Unknown,
    ] {
        out.push(canned(board));
        let mut s = Synth::new().board(board);
        for _ in 0..3 {
            s = s
                .round(G, 10, |s| s.taps(G, 10, HOLD, GAP))
                .round(E, 10, |s| s.wait(ms(4_000)));
        }
        out.push(run(s.round(G, 10, |s| s.taps(G, 10, HOLD, GAP))));
        out.push(run(Synth::new().board(board).down(R).wait(ms(3_000))));
    }
    out
}

#[test]
fn w02_every_rendered_string_is_hedged() {
    let mut seen = 0;
    for r in everything() {
        for f in &r.findings {
            let l = f.lines(&label);
            for text in std::iter::once(&l.headline)
                .chain(&l.evidence)
                .chain(&l.causes)
                .chain(&l.next)
            {
                assert!(hedged(text), "{text}");
                seen += 1;
            }
        }
        for n in &r.notes {
            assert!(hedged(&n.words(&label)));
        }
    }
    assert!(seen > 50);
    assert!(!hedged("the E switch is broken"));
    assert!(!hedged("E is dead"));
}

#[test]
fn w03_every_kind_is_possible() {
    for kind in [Kind::Chatter, Kind::Dead, Kind::Stuck] {
        assert!(kind.words().starts_with("possible "));
        for level in [
            Confidence::Low,
            Confidence::Medium,
            Confidence::High,
            Confidence::VeryHigh,
        ] {
            assert!(!criteria(kind, level).is_empty());
        }
    }
}

#[test]
fn w04_print_samples() {
    for r in everything().iter().take(4) {
        for f in &r.findings {
            let l = f.lines(&label);
            println!("{}  {}", label(f.key), l.headline);
            for e in &l.evidence {
                println!("    {e}");
            }
            for c in &l.causes {
                println!("  - {c}");
            }
            for n in &l.next {
                println!("  > {n}");
            }
        }
        for n in &r.notes {
            println!("  note: {}", n.words(&label));
        }
    }
}

// ---- found in review ----

fn chatter(borderline: u32, timing_unknown: u32, rate: u16, presses: u32) -> Finding {
    Finding {
        key: E,
        confidence: Confidence::Medium,
        evidence: Evidence::Chatter(ChatterEvidence {
            presses,
            affected: 2,
            extra_downs: 2,
            rate_floor_permille: rate,
            rounds: 2,
            rounds_affected: 2,
            gap: None,
            fragment: None,
            other_tested: 0,
            other_affected: 0,
            borderline,
            timing_unknown,
            poll: PollEstimate::Unknown,
            cap: None,
        }),
        causes: vec![],
        next_tests: vec![],
    }
}

#[test]
fn w04_counts_of_one_read_in_the_singular() {
    let lines = chatter(1, 1, 5, 100).lines(&label).evidence;
    assert!(lines.contains(
        &"1 more pair of presses came close to the limit and wasn't counted".to_string()
    ));
    assert!(
        lines.contains(&"1 interval read too close together to time wasn't counted".to_string())
    );
    let note = Note::NotAssessed {
        key: E,
        rounds: 2,
        why: Why::Paused,
    };
    assert!(note.words(&label).ends_with("paused for most of each."));
}

#[test]
fn w05_a_zero_bound_is_left_out() {
    let lines = chatter(0, 0, 0, 1000).lines(&label).evidence;
    assert_eq!(lines[0], "2 of 1000 presses sent an extra key-down");
}

#[test]
fn w06_one_press_with_several_extra_key_downs() {
    let note = Note::OneExtraDown {
        key: E,
        presses: 30,
        extra_downs: 3,
    };
    assert!(
        note.words(&label)
            .starts_with("E: 1 of 30 presses sent 3 extra key-downs.")
    );
}

#[test]
fn w07_stuck_medium_names_its_own_round() {
    assert!(criteria(Kind::Stuck, Confidence::Medium).contains("through its own round"));
}

// Every evidence branch and every note, rendered directly, so no wording escapes the check.
fn every_branch() -> Vec<String> {
    let span = |a, b| {
        Some(SpanMs {
            min_ms: a,
            max_ms: b,
        })
    };
    let mut evidence = Vec::new();
    for poll in [
        PollEstimate::Unknown,
        PollEstimate::Ms8,
        PollEstimate::Ms16OrSlower,
    ] {
        for (cap, n, others) in [
            (None, 1, (1, 0)),
            (Some(Cap::Systemic), 2, (3, 2)),
            (Some(Cap::CoarsePolling), 0, (0, 0)),
        ] {
            evidence.push(Evidence::Chatter(ChatterEvidence {
                presses: 100,
                affected: 4,
                extra_downs: 4 + n,
                rate_floor_permille: 15,
                rounds: 3,
                rounds_affected: 2,
                gap: span(5, 9),
                fragment: span(3, 3),
                other_tested: others.0,
                other_affected: others.1,
                borderline: n,
                timing_unknown: n,
                poll,
                cap,
            }));
        }
    }
    for (bracketed, control_rounds, other_presses_meanwhile) in
        [(true, 4, 1), (false, 1, 2), (false, 0, 0)]
    {
        evidence.push(Evidence::Dead(DeadEvidence {
            rounds: 3,
            silent_rounds: 3,
            asked_in_silent: 30,
            control_rounds,
            bracketed,
            other_presses_meanwhile,
        }));
    }
    for (still_down, n) in [(true, 0), (false, 1), (false, 2)] {
        evidence.push(Evidence::Stuck(StuckEvidence {
            held_ms: 4_000,
            still_down,
            episodes: 1,
            repeats: n,
            others_completed: n,
            own_prompts: n as u16,
            other_rounds_answered: n as u16,
        }));
    }
    let mut out = Vec::new();
    for evidence in evidence {
        for confidence in [
            Confidence::Low,
            Confidence::Medium,
            Confidence::High,
            Confidence::VeryHigh,
        ] {
            let lines = Finding {
                key: E,
                confidence,
                evidence,
                causes: vec![],
                next_tests: vec![],
            }
            .lines(&label);
            out.push(lines.headline);
            out.extend(lines.evidence);
        }
    }
    let mut notes = vec![
        Note::NoInputFromKeyboard,
        Note::Systemic { keys: 3 },
        Note::Clean {
            key: G,
            presses: 100,
            bound_permille: 30,
        },
        Note::OneExtraDown {
            key: E,
            presses: 100,
            extra_downs: 1,
        },
        Note::OneExtraDown {
            key: E,
            presses: 100,
            extra_downs: 3,
        },
        Note::Unprompted {
            key: R,
            affected: 2,
            presses: 2,
        },
        Note::DifferentCode {
            asked: BACKSLASH,
            got: ISO_BACKSLASH,
            rounds: 3,
        },
        Note::HeldAtPause {
            key: E,
            held_ms: 2_500,
        },
        Note::ReleasedWithNextKey {
            key: E,
            held_ms: 4_000,
        },
    ];
    for why in [
        Why::Paused,
        Why::Blocked,
        Why::Injected,
        Why::OtherKeyboard,
        Why::HeldDown,
    ] {
        for rounds in [1, 2] {
            notes.push(Note::NotAssessed {
                key: E,
                rounds,
                why,
            });
        }
    }
    out.extend(notes.iter().map(|n| n.words(&label)));
    out
}

#[test]
fn w08_every_branch_is_hedged() {
    let all = every_branch();
    assert!(all.len() > 100);
    for text in &all {
        assert!(hedged(text), "{text}");
    }
    for phrase in [
        "broken",
        "faulty",
        "defective",
        "failed",
        "is dead",
        "is bad",
    ] {
        assert!(!hedged(&format!("the E switch {phrase}")), "{phrase}");
    }
}

#[test]
fn w09_the_timing_resolution_in_words() {
    let words = [
        PollEstimate::Unknown,
        PollEstimate::No8Or16Ms,
        PollEstimate::Ms8,
        PollEstimate::Ms16OrSlower,
    ]
    .map(PollEstimate::words);
    assert_eq!(
        words,
        [
            "Timing resolution for this keyboard couldn't be measured in this test.",
            "This keyboard showed no 8 or 16 ms reporting schedule in this test.",
            "This keyboard reports about every 8 ms, so times are rounded to 8 ms.",
            "This keyboard reports every 16 ms or slower, so short extra presses can be missed.",
        ]
    );
    assert!(words.iter().all(|w| hedged(w)));
}

#[test]
fn w10_a_keyboard_on_neither_lattice_gets_no_resolution_it_lacks() {
    // Bluetooth LE links often report every 7.5, 10, 11.25 or 15 ms, which fit neither the 8 nor
    // the 16 ms lattice.
    for poll in [7_500, 10_000, 11_250, 15_000] {
        let taps = Synth::new().taps(G, 60, (ms(40), ms(130)), (ms(60), ms(250)));
        let estimate = run(taps.polled(poll)).aggregates.limits.poll;
        assert_eq!(estimate, PollEstimate::No8Or16Ms, "{poll}");
        assert_eq!(
            estimate.words(),
            "This keyboard showed no 8 or 16 ms reporting schedule in this test."
        );
    }
}
