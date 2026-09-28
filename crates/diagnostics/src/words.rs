// The fallback English. Findings describe evidence and likelihood; none says a part is broken.
use crate::report::*;

pub type Label<'a> = &'a dyn Fn(u16) -> String;

// A position's name when the caller has no drawn keyboard: "key 0012".
pub fn code_label(scan: u16) -> String {
    format!("key {scan:04X}")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lines {
    pub headline: String,
    pub evidence: Vec<String>,
    pub causes: Vec<String>,
    pub next: Vec<String>,
}

impl Confidence {
    pub fn words(self) -> &'static str {
        match self {
            Confidence::Low => "low",
            Confidence::Medium => "medium",
            Confidence::High => "high",
            Confidence::VeryHigh => "very high",
        }
    }
}

impl Kind {
    pub fn words(self) -> &'static str {
        match self {
            Kind::Chatter => "possible chatter",
            Kind::Dead => "possible dead key",
            Kind::Stuck => "possible stuck key",
        }
    }
}

impl PollEstimate {
    pub fn words(self) -> &'static str {
        match self {
            PollEstimate::Unknown => {
                "Timing resolution for this keyboard couldn't be measured in this test."
            }
            // The estimate only rules out the 8 and 16 ms lattices. A 10 ms keyboard lands here too.
            PollEstimate::No8Or16Ms => {
                "This keyboard showed no 8 or 16 ms reporting schedule in this test."
            }
            PollEstimate::Ms8 => {
                "This keyboard reports about every 8 ms, so times are rounded to 8 ms."
            }
            PollEstimate::Ms16OrSlower => {
                "This keyboard reports every 16 ms or slower, so short extra presses can be missed."
            }
        }
    }
}

impl Cause {
    pub fn words(self) -> &'static str {
        match self {
            Cause::SwitchContacts => "switch contacts",
            Cause::HotSwapSocket => "hot-swap socket",
            Cause::SolderJoint => "solder joint",
            Cause::SocketOrSolderJoint => "hot-swap socket or solder joint",
            Cause::SwitchSeating => "a bent or loose switch pin",
            Cause::DiodeOrTrace => "a diode or trace on the PCB",
            Cause::FirmwareDebounce => "firmware debounce",
            Cause::KeymapOrRemap => "a keymap, layer or Windows remap",
            Cause::DebrisOrResidue => "debris or dried liquid under the key",
            Cause::DomeOrScissor => "the rubber dome or scissor mechanism",
            Cause::StemOrKeycap => "a stem or keycap that sticks",
            Cause::LostRelease => "a lost release report (wireless link, cable or port)",
            Cause::HostSoftware => {
                "software holding the key (a vendor driver, remote desktop, or Sticky Keys)"
            }
        }
    }
}

impl NextTest {
    pub fn words(&self, label: Label) -> String {
        match *self {
            NextTest::TestAgain {
                key,
                rounds,
                presses,
            } => format!(
                "Test {k} again: {rounds} rounds of {presses} presses, with rounds of another key in between.",
                k = label(key)
            ),
            NextTest::AskBoardKind => "Say whether the keyboard is hot-swap, soldered or a laptop \
                keyboard. The next steps differ."
                .to_string(),
            NextTest::SwapSwitch {
                suspect,
                partner: Some(partner),
            } => format!(
                "Swap the {s} switch with the {p} switch and test both keys again. If the fault \
                 moves to {p}, the switch is the cause. If it stays on {s}, look at the socket or \
                 the PCB.",
                s = label(suspect),
                p = label(partner)
            ),
            NextTest::SwapSwitch {
                suspect,
                partner: None,
            } => format!(
                "Swap the {s} switch with the switch of a key that tested clean, and test both \
                 keys again. If the fault moves with the switch, the switch is the cause. If it \
                 stays on {s}, look at the socket or the PCB.",
                s = label(suspect)
            ),
            NextTest::ReseatSwitch { key } => format!(
                "Pull the {k} switch, check that its two pins are straight, and press it back in \
                 firmly. Then test {k} again.",
                k = label(key)
            ),
            NextTest::BridgeSocket { key } => format!(
                "Pull the {k} switch and bridge its two socket contacts with tweezers during a \
                 test. If {k} registers, the board reaches the socket, and the switch is the \
                 likely cause.",
                k = label(key)
            ),
            NextTest::CleanContacts { key } => format!(
                "Blow out the {k} switch with the key held down, or work contact cleaner into it \
                 while pressing it many times. Then test {k} again.",
                k = label(key)
            ),
            NextTest::RaiseDebounce => "If the keyboard's firmware lets you, raise its debounce \
                time to 10 ms, then 15 ms, and test again."
                .to_string(),
            NextTest::InspectUnderKeycap { key } => format!(
                "Take the {k} keycap off, check that the key moves freely and springs back, and \
                 clean under it. Then test {k} again.",
                k = label(key)
            ),
            NextTest::InspectSolderJoint { key } => format!(
                "Look at the {k} switch's solder joints on the back of the board for cracks or \
                 dull joints, and reflow them if you can. Then test {k} again.",
                k = label(key)
            ),
            NextTest::CheckKeymap { key } => format!(
                "Check what {k} sends in the keyboard's keymap (VIA, Vial or the vendor's \
                 software) and in any Windows remapping tool.",
                k = label(key)
            ),
            NextTest::CheckConnection => "Try another USB port or cable, or recharge and re-pair \
                a wireless keyboard, and test again."
                .to_string(),
        }
    }
}

fn plural(n: u32, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn range(s: SpanMs) -> String {
    if s.min_ms == s.max_ms {
        format!("{} ms", s.min_ms)
    } else {
        format!("{} to {} ms", s.min_ms, s.max_ms)
    }
}

fn percent(permille: u16) -> String {
    if permille.is_multiple_of(10) {
        format!("{}%", permille / 10)
    } else {
        format!("{}.{}%", permille / 10, permille % 10)
    }
}

fn seconds(ms: u32) -> String {
    format!("{}.{} s", ms / 1000, ms % 1000 / 100)
}

fn chatter_lines(e: &ChatterEvidence) -> Vec<String> {
    let mut out = Vec::new();
    // A bound that rounds down to 0% says nothing, so the line leaves it out.
    let rate = match e.rate_floor_permille {
        0 => String::new(),
        p => format!(" (a rate of at least {})", percent(p)),
    };
    out.push(if e.extra_downs == e.affected {
        format!(
            "{} of {} sent an extra key-down{rate}",
            e.affected,
            plural(e.presses, "press", "presses")
        )
    } else {
        format!(
            "{} of {} sent extra key-downs, {} in all{rate}",
            e.affected,
            plural(e.presses, "press", "presses"),
            e.extra_downs
        )
    });
    if let Some(gap) = e.gap {
        out.push(format!(
            "the extra key-downs came {} after a release",
            range(gap)
        ));
    }
    if let Some(fragment) = e.fragment {
        out.push(format!("the extra presses lasted {}", range(fragment)));
    }
    if e.rounds > 0 {
        out.push(format!(
            "reproduced in {} of {} rounds",
            e.rounds_affected, e.rounds
        ));
    }
    match (e.other_tested, e.other_affected) {
        (0, _) => {}
        (1, 0) => out.push("the other tested key didn't show it".to_string()),
        (t, 0) => out.push(format!("none of the {t} other tested keys showed it")),
        (t, a) => out.push(format!("{a} of the {t} other tested keys showed it too")),
    }
    match e.poll {
        PollEstimate::Ms8 => out.push(
            "this keyboard seems to report every 8 ms, so times are rounded to 8 ms".to_string(),
        ),
        PollEstimate::Ms16OrSlower => out.push(
            "this keyboard seems to report every 16 ms or slower, so short extra presses can be \
             missed, and confidence is capped at medium"
                .to_string(),
        ),
        PollEstimate::Unknown | PollEstimate::No8Or16Ms => {}
    }
    if e.cap == Some(Cap::Systemic) {
        out.push(
            "several tested keys showed it, which points past a single switch, so confidence is \
             capped at medium"
                .to_string(),
        );
    }
    if e.borderline > 0 {
        out.push(format!(
            "{} came close to the limit and {} counted",
            plural(
                e.borderline,
                "more pair of presses",
                "more pairs of presses"
            ),
            if e.borderline == 1 {
                "wasn't"
            } else {
                "weren't"
            }
        ));
    }
    if e.timing_unknown > 0 {
        out.push(format!(
            "{} read too close together to time {} counted",
            plural(e.timing_unknown, "interval", "intervals"),
            if e.timing_unknown == 1 {
                "wasn't"
            } else {
                "weren't"
            }
        ));
    }
    out.push(
        "the keyboard's own debounce hides contact bounce shorter than its setting, often 5 ms"
            .to_string(),
    );
    out
}

fn dead_lines(e: &DeadEvidence) -> Vec<String> {
    let mut out = vec![format!(
        "no key-down in {} of {} rounds ({} asked)",
        e.silent_rounds,
        e.rounds,
        plural(e.asked_in_silent, "press", "presses")
    )];
    out.push(if e.bracketed {
        "other keys answered their rounds before and after each silent round".to_string()
    } else if e.control_rounds > 0 {
        format!(
            "other keys answered {} in the same test",
            plural(u32::from(e.control_rounds), "round", "rounds")
        )
    } else {
        "other keys on this keyboard sent key-downs during the test".to_string()
    });
    if e.other_presses_meanwhile > 0 {
        out.push(format!(
            "{} of other keys arrived during the silent rounds",
            plural(e.other_presses_meanwhile, "press", "presses")
        ));
    }
    out
}

fn stuck_lines(e: &StuckEvidence) -> Vec<String> {
    let mut out = vec![if e.still_down {
        format!(
            "down for {} with no release by the end of the test",
            seconds(e.held_ms)
        )
    } else {
        format!("down for up to {} before its release", seconds(e.held_ms))
    }];
    if e.repeats > 0 {
        out.push(format!(
            "{} while it was down",
            plural(e.repeats, "autorepeat key-down", "autorepeat key-downs")
        ));
    }
    match e.own_prompts {
        0 => {}
        1 => out.push("it stayed down through a round that asked for separate presses".to_string()),
        n => out.push(format!(
            "it stayed down through {n} rounds that asked for separate presses"
        )),
    }
    if e.others_completed > 0 {
        out.push(format!(
            "{} of other keys began and ended while it was down",
            plural(e.others_completed, "press", "presses")
        ));
    }
    if e.other_rounds_answered > 0 {
        out.push(format!(
            "{} of other keys answered while it was down",
            plural(u32::from(e.other_rounds_answered), "round", "rounds")
        ));
    }
    out
}

impl Finding {
    pub fn headline(&self) -> String {
        format!(
            "{}, {} confidence",
            self.kind().words(),
            self.confidence.words()
        )
    }

    pub fn lines(&self, label: Label) -> Lines {
        Lines {
            headline: self.headline(),
            evidence: match &self.evidence {
                Evidence::Chatter(e) => chatter_lines(e),
                Evidence::Dead(e) => dead_lines(e),
                Evidence::Stuck(e) => stuck_lines(e),
            },
            causes: self.causes.iter().map(|c| c.words().to_string()).collect(),
            next: self.next_tests.iter().map(|t| t.words(label)).collect(),
        }
    }
}

impl Note {
    pub fn words(&self, label: Label) -> String {
        match *self {
            Note::NoInputFromKeyboard => "Nothing arrived from this keyboard during the test. \
                Check that the right keyboard is picked."
                .to_string(),
            Note::Systemic { keys } => format!(
                "{keys} tested keys showed extra key-downs. That points past single switches, to \
                 firmware debounce or the keyboard as a whole."
            ),
            Note::Clean {
                key,
                presses,
                bound_permille,
            } => format!(
                "{}: no extra key-downs in {} presses. A rate above {} would very likely have \
                 shown.",
                label(key),
                presses,
                percent(bound_permille)
            ),
            Note::OneExtraDown {
                key,
                presses,
                extra_downs,
            } => format!(
                "{}: 1 of {} presses sent {}. One event isn't enough for a finding.",
                label(key),
                presses,
                if extra_downs > 1 {
                    format!("{extra_downs} extra key-downs")
                } else {
                    "an extra key-down".to_string()
                }
            ),
            Note::Unprompted {
                key,
                affected,
                presses,
            } => format!(
                "{k}: {affected} of {presses} presses outside its own rounds sent extra key-downs. \
                 Only presses in its own rounds count, so test {k} in its own rounds.",
                k = label(key)
            ),
            Note::DifferentCode { asked, got, rounds } => format!(
                "{a} was asked for in {r}, and {g} arrived instead. A keymap, a layer, a Windows \
                 remap or the ANSI or ISO layout choice may send {g} for this key.",
                a = label(asked),
                g = label(got),
                r = plural(u32::from(rounds), "round", "rounds")
            ),
            Note::NotAssessed { key, rounds, why } => {
                let it = if rounds == 1 { "it" } else { "each" };
                let reason = match why {
                    Why::Paused => &format!("the test was paused for most of {it}"),
                    Why::Blocked => {
                        "other keys were held, or the keyboard reported too many keys at once, so \
                         its rollover limit could have blocked the key"
                    }
                    Why::Injected => &format!(
                        "software sent key presses during {it}, and a remapper can hide a key"
                    ),
                    Why::OtherKeyboard => "the key arrived from another keyboard",
                    Why::HeldDown => "the key was already held down",
                };
                format!(
                    "{} of {} {} counted: {reason}.",
                    plural(u32::from(rounds), "round", "rounds"),
                    label(key),
                    if rounds == 1 { "wasn't" } else { "weren't" }
                )
            }
            Note::HeldAtPause { key, held_ms } => format!(
                "{} had been down for {} when the test paused. A key down at a pause isn't \
                 counted as stuck.",
                label(key),
                seconds(held_ms)
            ),
            Note::ReleasedWithNextKey { key, held_ms } => format!(
                "{} stayed down for {}, and its release arrived with the next key's press. That \
                 can come from a lost release report on a wireless link, a cable or a port.",
                label(key),
                seconds(held_ms)
            ),
        }
    }
}

// Whether a rendered string keeps to evidence and likelihood.
pub fn hedged(text: &str) -> bool {
    let lower = text.to_lowercase();
    ![
        "broken",
        "faulty",
        "defective",
        "failed",
        "is dead",
        "is bad",
    ]
    .iter()
    .any(|w| lower.contains(w))
}

// What each level takes, in words a report can show beside a finding.
pub fn criteria(kind: Kind, level: Confidence) -> &'static str {
    use Confidence::*;
    match (kind, level) {
        (Kind::Chatter, Low) => "2 or more presses sent an extra key-down",
        (Kind::Chatter, Medium) => {
            "3 or more at a rate of at least 1%, or 2 or more in 2 or more rounds"
        }
        (Kind::Chatter, High) => "3 or more at a rate of at least 2%, in 2 or more rounds",
        (Kind::Chatter, VeryHigh) => {
            "5 or more at a rate of at least 5%, in every round of 3 or more"
        }
        (Kind::Dead, Low) => "no key-down in at least 1 round",
        (Kind::Dead, Medium) => {
            "no key-down in every round of 2 or more, or in 2 or more rounds while other keys \
             answered, or in its only round with other keys answering before and after"
        }
        (Kind::Dead, High) => {
            "no key-down in every round of 2 or more while other keys answered theirs"
        }
        (Kind::Dead, VeryHigh) => {
            "no key-down in every round of 3 or more, with other keys answering before and after each"
        }
        (Kind::Stuck, Low) => "down 2 s or more with no release by the end of the test",
        (Kind::Stuck, Medium) => {
            "down 2 s or more through its own round asking for presses, or with no release by the end \
             and one sign: 3 or more other presses meanwhile (not for a modifier), or another key's \
             round answered meanwhile"
        }
        (Kind::Stuck, High) => "two of those signs, or one of them in 2 separate holds",
        (Kind::Stuck, VeryHigh) => {
            "down through a round asking for its presses, in 2 separate holds"
        }
    }
}
