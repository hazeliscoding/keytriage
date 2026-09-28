// Times are microseconds. The chatter limits (20, 36 and 100 ms) and every bin edge after the first
// are 4 mod 8, so a 125 Hz keyboard's readings (multiples of 8 ms, plus read jitter under 1 ms)
// never straddle one. Changing any limit or bin edge here means bumping `RULES`.
use crate::report::Confidence;

// Each transition needs its own HID report, and full speed polls at most every 1 ms, so readings
// closer than this were bunched by the input queue. Raw Input carries no hardware timestamp.
pub const COALESCED_US: u64 = 1_000;
// Phantom presses after firmware debounce last about 5 ms (QMK eager), 5 to 20 ms (QMK defer, a
// dropout), 9 to 15 ms (laptop firmware), or one or two polls at 125 Hz. Deliberate holds are
// under 17 ms only 0.1% of the time.
pub const SHORT_HOLD_US: u64 = 20_000;
// Chatter's release-to-press gap is about 10 ms, and 5 to 20 ms through defer debounce. The
// fastest one-finger repeat still leaves gaps of about 30 ms.
pub const SHORT_GAP_US: u64 = 20_000;
// Covers laptop firmware's 30 ms release lock and a 30 ms human gap read at 125 Hz. Such pairs are
// counted and shown, never counted as chatter.
pub const BORDERLINE_US: u64 = 36_000;
// Release chatter lands 22 to 90 ms after the release, so a short press counts only when another
// press of the same key is this close.
pub const REACH_US: u64 = 100_000;
// Release chatter lands up to 90 ms after a release, so a key-down this soon after the same key's
// release may be chatter. It shows on the drawing but never moves a guided prompt on. A fast
// deliberate double press then needs one more press, which never mislabels a fault.
pub const PROMPT_MERGE_US: u64 = REACH_US;
// The plan's last round closes at a release, and that press's chatter can land up to 90 ms later,
// so capture stays open this long after the plan's last view. The margin covers the view's trip to
// the page and the input still queued behind it.
pub const END_WAIT_US: u64 = REACH_US + 50_000;
// Windows' shortest repeat delay, 250 ms, less 20% tolerance. A second key-down closer than this
// is a duplicate, not a repeat.
pub const REPEAT_MIN_DELAY_US: u64 = 200_000;
// Pause arrives as 0xE11D and 0x45 from one translation, microseconds apart. No finger presses Num
// Lock that soon after.
pub const PAUSE_TAIL_US: u64 = 2_000;
// The keyboard driver sends releases before presses from one report, so a lost release arrives in
// the same burst as the next key's press.
pub const NEXT_KEY_US: u64 = 1_000;
// Twice the longest repeat delay (1 s), and about 17 times a typical hold.
pub const STUCK_US: u64 = 2_000_000;
// Three finished presses of other keys mean the hand was elsewhere. One or two can be a chord.
pub const BUSY_PRESSES: u32 = 3;
// A silent round means something only when it asked for at least this many presses. One missed
// prompt isn't a dead key.
pub const SILENT_MIN_ASKED: u16 = 3;
// With this many prompted presses and none affected, the rule of three bounds the rate at 30%.
pub const TESTED_PRESSES: u32 = 10;
// Tap-hold keys and macros send each tap as a 1 to 8 ms press. Telling them apart takes a sample.
pub const TAPPED_MIN_HOLDS: u32 = 10;
// Several keys chattering at once points past a single switch, to debounce or the whole board.
pub const SYSTEMIC_KEYS: usize = 3;
// A 1000 Hz keyboard's readings land near the 8 ms lattice a quarter of the time, so 16 of 20 (the
// 80% fit) happen by chance less than once in a million. At 5 samples it would be 1.6%.
pub const POLL_MIN_SAMPLES: usize = 20;
// A reading under 1 ms is within tolerance of 0, a multiple of every lattice, so it fits any rate.
// The extra millisecond is margin for read jitter on two events from one report.
pub const POLL_MIN_US: u64 = 2_000;
// Events are timed when the app reads them, under 1 ms after their report, so a 125 Hz reading
// lands within 1 ms of a multiple of 8 ms. A wider band would let more 1000 Hz readings fit.
pub const POLL_TOLERANCE_US: u64 = 1_000;
// At 1000 Hz about a quarter of samples fall near the 8 ms lattice by chance; at 125 Hz, nearly all.
pub const POLL_FIT_PERCENT: usize = 80;
// At 30 presses per round a 5% rate shows in 79% of rounds, and 90 clean presses bound the rate at
// 3.4%.
pub const RETEST_ROUNDS: u16 = 3;
pub const RETEST_PRESSES: u16 = 30;
// 90 clean presses bound a key's rate at 3.4%, which is at or under the floor of a High chatter
// finding from 30 presses (3 of 30 floors at 3.4%) and of every Very high one, so a key that stays
// clean after the swap counts as evidence.
pub const SWAP_ROUNDS: u16 = RETEST_ROUNDS;
pub const SWAP_PRESSES: u16 = RETEST_PRESSES;
// A side left with too little evidence to clear it leaves both the switch and the socket possible.
pub const SWAP_UNCLEARED: Confidence = Confidence::Medium;

// Half-open [lo, hi) bins; bin 0 is too close to time. The limits above (1, 20, 36 and 100 ms) are
// edges, so a saved report can recount them.
pub const EDGES_MS: [u64; 13] = [1, 4, 12, 20, 28, 36, 60, 100, 164, 260, 516, 1028, 2052];
pub const BINS: usize = EDGES_MS.len() + 1;
