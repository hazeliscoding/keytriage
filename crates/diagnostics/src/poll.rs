// How often the keyboard seems to report, from same-key intervals only. At 125 Hz every interval
// sits on the 8 ms lattice; at 1000 Hz about a quarter of human intervals land near it by chance.
use crate::fold::Folded;
use crate::params::*;
use crate::report::PollEstimate;

fn near(us: u64, lattice: u64) -> bool {
    let r = us % lattice;
    r <= POLL_TOLERANCE_US || lattice - r <= POLL_TOLERANCE_US
}

pub(crate) fn estimate(f: &Folded) -> PollEstimate {
    let mut samples = Vec::new();
    for t in f.tracks.values() {
        for (i, e) in t.episodes.iter().enumerate() {
            samples.extend(f.hold(e).filter(|h| h.countable).map(|h| h.us));
            if let Some(next) = t.episodes.get(i + 1) {
                samples.extend(f.gap(e, next).filter(|g| g.countable).map(|g| g.us));
            }
        }
    }
    samples.retain(|&us| us >= POLL_MIN_US);
    if samples.len() < POLL_MIN_SAMPLES {
        return PollEstimate::Unknown;
    }
    let share =
        |lattice| samples.iter().filter(|&&us| near(us, lattice)).count() * 100 / samples.len();
    if share(16_000) >= POLL_FIT_PERCENT {
        PollEstimate::Ms16OrSlower
    } else if share(8_000) >= POLL_FIT_PERCENT {
        PollEstimate::Ms8
    } else {
        PollEstimate::AtMost4Ms
    }
}
