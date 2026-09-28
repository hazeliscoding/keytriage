mod aggregates;
mod chatter;
mod dead;
mod guide;
mod normalize;
mod stats;
mod stuck;
mod swap;
mod words;

pub use crate::fixture::{GAP, HOLD, normal};
use crate::fixture::{Synth, ms};
use crate::*;

pub const E: u16 = 0x12;
pub const R: u16 = 0x13;
pub const T: u16 = 0x14;
pub const Y: u16 = 0x15;
pub const F: u16 = 0x21;
pub const G: u16 = 0x22;
pub const J: u16 = 0x24;
pub const K: u16 = 0x25;
pub const SPACE: u16 = 0x39;
pub const ENTER: u16 = 0x1C;
pub const LSHIFT: u16 = 0x2A;
pub const LCTRL: u16 = 0x1D;
pub const INSERT: u16 = 0xE052;
pub const BACKSLASH: u16 = 0x2B;
pub const ISO_BACKSLASH: u16 = 0x56;

pub fn run(s: Synth) -> Report {
    s.build().diagnose()
}

pub fn assert_clean(r: &Report) {
    assert!(
        r.findings.is_empty(),
        "unexpected findings: {:#?}",
        r.findings
    );
}

pub fn only(r: &Report, kind: Kind, key: u16) -> Finding {
    assert_eq!(r.findings.len(), 1, "{:#?}", r.findings);
    let f = r.findings[0].clone();
    assert_eq!((f.kind(), f.key), (kind, key), "{f:#?}");
    f
}

pub fn chatter_of(f: &Finding) -> ChatterEvidence {
    match f.evidence {
        Evidence::Chatter(e) => e,
        _ => panic!("not chatter: {f:#?}"),
    }
}

// Answered rounds of G and J, so two other keys count as tested.
pub fn controls(s: Synth) -> Synth {
    s.round(G, 12, |s| s.taps(G, 12, HOLD, GAP))
        .round(J, 12, |s| s.taps(J, 12, HOLD, GAP))
}

// Controls, then 100 presses of E over rounds of 34, 33 and 33, with bad[i] of round i faulty.
pub fn chatter_rounds(bad: [u32; 3], fault: fn(Synth) -> Synth) -> Synth {
    let mut s = controls(Synth::new());
    for (i, &b) in bad.iter().enumerate() {
        let n: u32 = if i == 0 { 34 } else { 33 };
        s = s.round(E, n as u16, |mut s| {
            for p in 0..n {
                s = if p % 6 == 1 && p / 6 < b {
                    fault(s)
                } else {
                    normal(s, E)
                };
            }
            s
        });
    }
    s
}
