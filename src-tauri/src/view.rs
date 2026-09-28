// What the page is sent and what it may ask for. Only these shapes cross to the page, so a device
// path or container ID never does.
use std::collections::BTreeMap;

use keytriage_diagnostics::params::{EDGES_MS, END_WAIT_US};
use keytriage_diagnostics::{
    BoardKind, Confidence, Guide, Histogram, Kind, Label, MAX_PRESSES, MAX_ROUNDS, Note, Outcome,
    OutcomeLines, Plan, PlanError, Report, Swap, SwapLines, SwapResult, code_label,
};
use keytriage_input::Keyboard;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardGroup {
    pub name: String,
    pub id: Option<String>,
    pub built_in: bool,
    pub entries: Vec<KeyboardEntry>,
}

#[derive(Serialize)]
pub struct KeyboardEntry {
    pub handle: isize,
    pub name: String,
    pub id: Option<String>,
}

// Windows gives everything built into the computer the null container
// {00000000-0000-0000-ffff-ffffffffffff}, so it can't tell a laptop's keyboard from its hotkeys.
const BUILT_IN: u128 = u64::MAX as u128;

// One keyboard shows up once per HID collection. An external keyboard's collections share a
// container, so they form one group, and picking any entry tests all of them.
pub fn groups(keyboards: &[Keyboard]) -> Vec<KeyboardGroup> {
    let mut groups: Vec<(Option<u128>, KeyboardGroup)> = Vec::new();
    let mut built_in: Option<KeyboardGroup> = None;
    for keyboard in keyboards {
        let entry = KeyboardEntry {
            handle: keyboard.handle,
            name: keyboard.name(),
            id: id(keyboard),
        };
        let into = match keyboard.container {
            Some(BUILT_IN) => built_in.get_or_insert_with(|| group(&entry, true)),
            // Without a container nothing says two entries are one device.
            container => {
                let found = container.and_then(|c| groups.iter().position(|(g, _)| *g == Some(c)));
                let i = found.unwrap_or_else(|| {
                    groups.push((container, group(&entry, false)));
                    groups.len() - 1
                });
                &mut groups[i].1
            }
        };
        into.entries.push(entry);
    }
    groups.into_iter().map(|(_, g)| g).chain(built_in).collect()
}

fn group(first: &KeyboardEntry, built_in: bool) -> KeyboardGroup {
    KeyboardGroup {
        name: first.name.clone(),
        id: first.id.clone(),
        built_in,
        entries: Vec::new(),
    }
}

fn id(keyboard: &Keyboard) -> Option<String> {
    Some(format!(
        "{:04X}:{:04X}",
        keyboard.vendor_id?, keyboard.product_id?
    ))
}

#[derive(Deserialize)]
pub struct PlanArgs {
    pub keyboard: Vec<isize>,
    pub keys: Vec<u16>,
    pub rounds: u16,
    pub presses: u16,
    pub board: Option<BoardArg>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BoardArg {
    HotSwap,
    Soldered,
    Laptop,
}

// The `test:guide` payload. It says where the test is and how many key-downs each key has sent, and
// holds no times.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideView {
    pub key: Option<u16>,
    pub asked: u16,
    pub count: u16,
    pub round: u16,
    pub rounds: u16,
    pub index: u16,
    pub keys: u16,
    pub done: u32,
    pub total: u32,
    pub tallies: Vec<(u16, u32)>,
    // Only once the plan is done: how long the page waits before it ends the test.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_ms: Option<u32>,
}

// Past these a request is a mistake, not a keyboard: one keyboard shows up as a handful of HID
// collections, and a full-size ISO layout has 105 keys.
const MAX_HANDLES: usize = 64;
const MAX_KEYS: usize = 128;

// Everything is checked before a running test is stopped, so a refused plan leaves it running.
pub fn plan(args: PlanArgs) -> Result<(Guide, Vec<isize>, BoardKind), String> {
    if args.keyboard.len() > MAX_HANDLES {
        return Err(format!(
            "A keyboard can't have more than {MAX_HANDLES} entries."
        ));
    }
    if args.keys.len() > MAX_KEYS {
        return Err(format!("A test can't prompt more than {MAX_KEYS} keys."));
    }
    let board = match args.board {
        None => BoardKind::Unknown,
        Some(BoardArg::HotSwap) => BoardKind::HotSwap,
        Some(BoardArg::Soldered) => BoardKind::Soldered,
        Some(BoardArg::Laptop) => BoardKind::Laptop,
    };
    let plan = Plan {
        keys: args.keys,
        rounds: args.rounds,
        presses: args.presses,
    };
    let guide = Guide::new(plan, &args.keyboard).map_err(refusal)?;
    Ok((guide, args.keyboard, board))
}

const RECONNECTED: &str =
    "This keyboard was unplugged or reconnected. Pick it again on the Start screen.";

// Windows gives a keyboard new handles when it reconnects, and no event carries the old ones, so a
// plan on them would wait on its first prompt for good. The paths let a swap test find the same
// keyboard again, and stay in Rust.
pub fn still_listed(keyboard: &[isize], listed: &[Keyboard]) -> Result<Vec<String>, String> {
    keyboard
        .iter()
        .map(|&handle| {
            listed
                .iter()
                .find(|k| k.handle == handle)
                .map(|k| k.path.clone())
                .ok_or_else(|| RECONNECTED.to_string())
        })
        .collect()
}

// Names no path, so a refusal the debug echo prints can't leak one.
const UNPLUGGED: &str =
    "The keyboard isn't listed. Plug it back into the port it used, then try again.";

// The user pulls switches between the two tests, and may unplug the keyboard to do it. It comes back
// on new handles, but on the same port with the same device path. A handle is never matched on its
// own, because Windows may have given an old one to another device.
pub fn refind(paths: &[String], listed: &[Keyboard]) -> Result<Vec<isize>, String> {
    paths
        .iter()
        .map(|path| {
            listed
                .iter()
                .find(|k| k.path == *path)
                .map(|k| k.handle)
                .ok_or_else(|| UNPLUGGED.to_string())
        })
        .collect()
}

// The retest is the engine's plan for the kept offer, checked as a plan from the page is. Only a
// hot-swap board gets an offer.
pub fn swap_plan(
    swap: &Swap,
    keyboard: Vec<isize>,
) -> Result<(Guide, Vec<isize>, BoardKind), String> {
    let Plan {
        keys,
        rounds,
        presses,
    } = swap.plan();
    plan(PlanArgs {
        keyboard,
        keys,
        rounds,
        presses,
        board: Some(BoardArg::HotSwap),
    })
}

fn refusal(error: PlanError) -> String {
    match error {
        PlanError::NoKeys => "The test has no keys to prompt.".to_string(),
        PlanError::BadKey => "The test names a key that can't be prompted.".to_string(),
        PlanError::RepeatedKey => "The test names a key twice.".to_string(),
        PlanError::Rounds => format!("A test runs 1 to {MAX_ROUNDS} rounds."),
        PlanError::Presses => format!("A round asks for 1 to {MAX_PRESSES} presses."),
        PlanError::NoKeyboard => "No keyboard was picked.".to_string(),
        PlanError::ZeroHandle => "Handle 0 is injected input, not a keyboard.".to_string(),
    }
}

// Once the plan is done there is no open round: key is None, count is 0, and round and index stay
// on the last step.
pub fn guide_view(guide: &Guide) -> GuideView {
    let plan = guide.plan();
    let keys = plan.keys.len() as u16;
    let prompt = guide.prompt();
    GuideView {
        key: prompt.map(|p| p.key),
        asked: plan.presses,
        count: prompt.map_or(0, |p| p.count),
        round: prompt.map_or(plan.rounds - 1, |p| p.round),
        rounds: plan.rounds,
        index: prompt.map_or(keys - 1, |p| p.index),
        keys,
        done: guide.done(),
        total: guide.total(),
        tallies: guide.tallies().iter().map(|(&k, &n)| (k, n)).collect(),
        wait_ms: prompt.is_none().then_some((END_WAIT_US / 1000) as u32),
    }
}

// A drawn key's name, which only words the findings for the page. Names never reach a file.
#[derive(Deserialize)]
pub struct KeyName {
    pub scan: u16,
    pub name: String,
}

// A drawn keyboard names about a hundred keys in a word or two each, so more is a mistake.
const MAX_NAMES: usize = 256;
const MAX_NAME_CHARS: usize = 24;

pub fn names(labels: Vec<KeyName>) -> Result<BTreeMap<u16, String>, String> {
    if labels.len() > MAX_NAMES {
        return Err(format!("A layout can't name more than {MAX_NAMES} keys."));
    }
    let mut names = BTreeMap::new();
    for KeyName { scan, name } in labels {
        if name.trim().is_empty() {
            return Err("A key name can't be empty.".to_string());
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(format!(
                "A key name can't be longer than {MAX_NAME_CHARS} characters."
            ));
        }
        if name.chars().any(char::is_control) {
            return Err("A key name can't hold control characters.".to_string());
        }
        names.insert(scan, name);
    }
    Ok(names)
}

// The `end_test` result: the engine's findings and notes in words, with no times.
#[derive(Serialize)]
pub struct TestResult {
    pub rules: u16,
    pub findings: Vec<FindingView>,
    pub notes: Vec<String>,
    pub clean: Vec<String>,
    pub keys: Vec<KeyCount>,
    pub resolution: String,
    pub swap: Option<SwapView>,
    pub outcome: Option<OutcomeView>,
}

// The swap a main test offers, in the engine's words. The page chooses no keys.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapView {
    pub suspect: u16,
    pub partner: u16,
    pub title: String,
    pub known_good: String,
    pub steps: Vec<String>,
    pub means: String,
    pub note: String,
}

// A swap test's judgment. Unclear carries no confidence, so it shows no badge.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutcomeView {
    pub outcome: &'static str,
    pub tile: Option<u16>,
    pub flagged: Vec<u16>,
    pub title: String,
    pub confidence: Option<&'static str>,
    pub level: Option<String>,
    pub strong: bool,
    pub evidence: Vec<String>,
    pub diagnosis: String,
    pub next: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingView {
    pub key: u16,
    pub kind: &'static str,
    pub confidence: &'static str,
    pub title: String,
    pub level: String,
    pub strong: bool,
    pub evidence: Vec<String>,
    pub causes: Vec<String>,
    pub next: Vec<String>,
    pub gaps: Option<Vec<Bar>>,
}

#[derive(Serialize)]
pub struct Bar {
    pub label: &'static str,
    pub count: u32,
}

#[derive(Serialize)]
pub struct KeyCount {
    pub scan: u16,
    pub count: u32,
}

// The chatter card's bars, each closed at one of the engine's bin edges, in ms, so a bar never
// splits a bin.
const GAP_BARS: [(&str, u64); 6] = [
    ("<4", 4),
    ("4–12", 12),
    ("12–20", 20),
    ("20–36", 36),
    ("36–100", 100),
    ("100+", u64::MAX),
];

fn gap_bars(gaps: &Histogram) -> Vec<Bar> {
    let mut bars: Vec<Bar> = GAP_BARS
        .iter()
        .map(|&(label, _)| Bar { label, count: 0 })
        .collect();
    for (bin, &count) in gaps.0.iter().enumerate() {
        let upper = EDGES_MS.get(bin).copied().unwrap_or(u64::MAX);
        let bar = GAP_BARS
            .iter()
            .position(|&(_, edge)| upper <= edge)
            .unwrap_or(GAP_BARS.len() - 1);
        bars[bar].count += count;
    }
    bars
}

// The engine's words are lowercase so that they can sit inside a sentence. Here each one starts
// its own line.
fn sentence(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Chatter => "chatter",
        Kind::Dead => "dead",
        Kind::Stuck => "stuck",
    }
}

fn level_name(level: Confidence) -> &'static str {
    match level {
        Confidence::Low => "low",
        Confidence::Medium => "medium",
        Confidence::High => "high",
        Confidence::VeryHigh => "very-high",
    }
}

fn sentences(lines: &[String]) -> Vec<String> {
    lines.iter().map(|l| sentence(l)).collect()
}

// Keys the page didn't name read as their code, "key 0012".
fn labeller(names: &BTreeMap<u16, String>) -> impl Fn(u16) -> String + '_ {
    |scan| {
        names
            .get(&scan)
            .cloned()
            .unwrap_or_else(|| code_label(scan))
    }
}

pub fn result(report: &Report, names: &BTreeMap<u16, String>) -> TestResult {
    let label = labeller(names);
    let saved = report.saved();
    let findings = report
        .findings
        .iter()
        .map(|f| {
            let lines = f.lines(&label);
            FindingView {
                key: f.key,
                kind: kind_name(f.kind()),
                confidence: level_name(f.confidence),
                title: sentence(f.kind().words()),
                level: sentence(f.confidence.words()),
                strong: f.confidence >= Confidence::High,
                evidence: sentences(&lines.evidence),
                causes: sentences(&lines.causes),
                next: sentences(&lines.next),
                gaps: match f.kind() {
                    Kind::Chatter => saved.keys.get(&f.key).map(|a| gap_bars(&a.release_gap)),
                    Kind::Dead | Kind::Stuck => None,
                },
            }
        })
        .collect();
    let (clean, notes): (Vec<&Note>, Vec<&Note>) = report
        .notes
        .iter()
        .partition(|n| matches!(n, Note::Clean { .. }));
    let words = |notes: Vec<&Note>| notes.iter().map(|n| sentence(&n.words(&label))).collect();
    TestResult {
        rules: report.rules,
        findings,
        notes: words(notes),
        clean: words(clean),
        // Counts for the drawing: every key-down each prompted key sent in its own rounds.
        keys: saved
            .keys
            .iter()
            .filter_map(|(&scan, a)| {
                a.prompted.map(|p| KeyCount {
                    scan,
                    count: p.presses + p.extra_downs,
                })
            })
            .collect(),
        resolution: report.aggregates.limits.poll.words().to_string(),
        swap: None,
        outcome: None,
    }
}

// A main test may offer the swap, which Rust keeps for the retest. A swap test ends in its judgment,
// with none of its own findings or notes: their steps would name a swap already made.
pub fn ended(
    report: &Report,
    board: BoardKind,
    retest: Option<&Swap>,
    names: &BTreeMap<u16, String>,
) -> (TestResult, Option<Swap>) {
    let label = labeller(names);
    match retest {
        None => {
            let offer = Swap::offer(report, board);
            let swap = offer.map(|s| swap_view(&s, &label));
            (
                TestResult {
                    swap,
                    ..result(report, names)
                },
                offer,
            )
        }
        Some(retest) => {
            let outcome = outcome_view(&retest.judge(report), &label);
            (
                TestResult {
                    findings: Vec::new(),
                    notes: Vec::new(),
                    clean: Vec::new(),
                    outcome: Some(outcome),
                    ..result(report, names)
                },
                None,
            )
        }
    }
}

// Both views name every field of the engine's lines, so a line the engine adds fails to compile
// until it reaches the page.
fn swap_view(swap: &Swap, label: Label) -> SwapView {
    let SwapLines {
        title,
        known_good,
        steps,
        means,
        note,
    } = swap.lines(label);
    SwapView {
        suspect: swap.suspect,
        partner: swap.partner,
        title: sentence(&title),
        known_good: sentence(&known_good),
        steps: sentences(&steps),
        means: sentence(&means),
        note: sentence(&note),
    }
}

fn outcome_view(judged: &SwapResult, label: Label) -> OutcomeView {
    let OutcomeLines {
        title,
        evidence,
        diagnosis,
        next,
    } = judged.lines(label);
    OutcomeView {
        outcome: match judged.outcome {
            Outcome::Follows => "follows",
            Outcome::Stays => "stays",
            Outcome::Both => "both",
            Outcome::Gone => "gone",
            Outcome::Unclear => "unclear",
        },
        tile: judged.tile(),
        flagged: judged.flagged(),
        title: sentence(&title),
        confidence: judged.confidence.map(level_name),
        level: judged.confidence.map(|c| sentence(c.words())),
        strong: judged.confidence.is_some_and(|c| c >= Confidence::High),
        evidence: sentences(&evidence),
        diagnosis: sentence(&diagnosis),
        next: sentences(&next),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use keytriage_diagnostics::fixture::{
        Fixture, GAP, HOLD, Synth, guided, guided_chatter, interleaved, ms, normal, swap_chatter,
    };
    use keytriage_diagnostics::hedged;
    use keytriage_diagnostics::params::BINS;
    use serde_json::{Value, json};

    use super::*;

    fn keyboard(
        handle: isize,
        product: &str,
        ids: Option<(u16, u16)>,
        container: Option<u128>,
    ) -> Keyboard {
        Keyboard {
            handle,
            path: format!(r"\\?\HID#VID_3434&PID_0220&MI_00#7&1a2b3c4d&0&0000#{handle}"),
            vendor_id: ids.map(|i| i.0),
            product_id: ids.map(|i| i.1),
            manufacturer: None,
            product: Some(product.to_string()),
            container,
        }
    }

    const K2: u128 = 0x1234_5678_9abc_def0_1234_5678_9abc_def0;
    const OTHER_K2: u128 = 0x0fed_cba9_8765_4321_0fed_cba9_8765_4321;
    const K2_IDS: Option<(u16, u16)> = Some((0x3434, 0x0220));

    fn handles(group: &KeyboardGroup) -> Vec<isize> {
        group.entries.iter().map(|e| e.handle).collect()
    }

    #[test]
    fn kb01_collections_that_share_a_container_form_one_group() {
        let all = groups(&[
            keyboard(11, "Keychron K2", K2_IDS, Some(K2)),
            keyboard(12, "Keychron K2, consumer control", K2_IDS, Some(K2)),
            keyboard(13, "Keychron K2, system control", K2_IDS, Some(K2)),
        ]);
        assert_eq!(all.len(), 1);
        assert_eq!(handles(&all[0]), [11, 12, 13]);
        assert_eq!(all[0].name, "Keychron K2");
        assert!(!all[0].built_in);
    }

    #[test]
    fn kb02_identical_keyboards_stay_apart() {
        let all = groups(&[
            keyboard(11, "Keychron K2", K2_IDS, Some(K2)),
            keyboard(21, "Keychron K2", K2_IDS, Some(OTHER_K2)),
            keyboard(12, "Keychron K2", K2_IDS, Some(K2)),
        ]);
        let found: Vec<Vec<isize>> = all.iter().map(handles).collect();
        assert_eq!(found, [vec![11, 12], vec![21]]);
    }

    #[test]
    fn kb03_everything_built_in_forms_one_group_listed_last() {
        let all = groups(&[
            keyboard(31, "HID Keyboard Device", None, Some(BUILT_IN)),
            keyboard(11, "Keychron K2", K2_IDS, Some(K2)),
            keyboard(32, "Standard PS/2 Keyboard", None, Some(BUILT_IN)),
        ]);
        let found: Vec<(Vec<isize>, bool)> = all.iter().map(|g| (handles(g), g.built_in)).collect();
        assert_eq!(found, [(vec![11], false), (vec![31, 32], true)]);
        assert_eq!(all[1].name, "HID Keyboard Device");
    }

    #[test]
    fn kb04_a_keyboard_with_no_container_stands_alone() {
        let all = groups(&[
            keyboard(41, "USB Keyboard", None, None),
            keyboard(42, "USB Keyboard", None, None),
        ]);
        let found: Vec<Vec<isize>> = all.iter().map(handles).collect();
        assert_eq!(found, [vec![41], vec![42]]);
    }

    #[test]
    fn kb05_ids_read_as_uppercase_hex() {
        let mut ps2 = keyboard(51, "Standard PS/2 Keyboard", None, Some(BUILT_IN));
        ps2.product = None;
        let half = keyboard(52, "Half", Some((0x05ac, 0x024f)), None);
        let half = Keyboard {
            product_id: None,
            ..half
        };
        let all = groups(&[
            keyboard(11, "Galaxy80", Some((0x05ac, 0x024f)), Some(K2)),
            half,
            ps2,
        ]);
        assert_eq!(all[0].id.as_deref(), Some("05AC:024F"));
        assert_eq!(all[0].entries[0].id.as_deref(), Some("05AC:024F"));
        assert_eq!(all[1].id, None);
        assert_eq!(
            (all[2].id.as_deref(), all[2].name.as_str()),
            (None, "Keyboard")
        );
        assert_eq!(all[2].entries[0].id, None);
    }

    #[test]
    fn kb06_the_page_gets_names_ids_and_handles_only() {
        let all = groups(&[
            keyboard(65603, "Keychron K2", K2_IDS, Some(K2)),
            keyboard(65605, "Keychron K2, consumer control", K2_IDS, Some(K2)),
            keyboard(31, "HID Keyboard Device", None, Some(BUILT_IN)),
        ]);
        let json = serde_json::to_string(&all).unwrap();
        assert_eq!(
            json,
            concat!(
                r#"[{"name":"Keychron K2","id":"3434:0220","builtIn":false,"entries":["#,
                r#"{"handle":65603,"name":"Keychron K2","id":"3434:0220"},"#,
                r#"{"handle":65605,"name":"Keychron K2, consumer control","id":"3434:0220"}]},"#,
                r#"{"name":"HID Keyboard Device","id":null,"builtIn":true,"entries":["#,
                r#"{"handle":31,"name":"HID Keyboard Device","id":null}]}]"#,
            )
        );
        for leak in ["path", "container", r"\\?\", "HID#", "1a2b3c4d"] {
            assert!(!json.contains(leak), "{leak}");
        }
    }

    const E: u16 = 0x12;
    const F: u16 = 0x21;
    const G: u16 = 0x22;
    const H: u16 = 0x23;
    const J: u16 = 0x24;

    fn args(keyboard: &[isize], keys: &[u16], rounds: u16, presses: u16) -> PlanArgs {
        PlanArgs {
            keyboard: keyboard.to_vec(),
            keys: keys.to_vec(),
            rounds,
            presses,
            board: None,
        }
    }

    // The Guide prints nothing outside its crate, so neither can a Result holding one.
    fn refused(args: PlanArgs) -> String {
        match plan(args) {
            Ok(_) => panic!("the plan was accepted"),
            Err(e) => e,
        }
    }

    #[test]
    fn plan01_the_page_can_ask_only_for_a_test_that_fits() {
        let handles: Vec<isize> = (1..=65).collect();
        let keys: Vec<u16> = (1..=129).collect();
        for (bad, why) in [
            (args(&[], &[E], 3, 10), "No keyboard was picked."),
            (
                args(&[1, 0], &[E], 3, 10),
                "Handle 0 is injected input, not a keyboard.",
            ),
            (
                args(&handles, &[E], 3, 10),
                "A keyboard can't have more than 64 entries.",
            ),
            (args(&[1], &[], 3, 10), "The test has no keys to prompt."),
            (args(&[1], &[E, G, E], 3, 10), "The test names a key twice."),
            (
                args(&[1], &[E, 0], 3, 10),
                "The test names a key that can't be prompted.",
            ),
            (
                args(&[1], &keys, 3, 10),
                "A test can't prompt more than 128 keys.",
            ),
            (args(&[1], &[E], 0, 10), "A test runs 1 to 10 rounds."),
            (args(&[1], &[E], 11, 10), "A test runs 1 to 10 rounds."),
            (args(&[1], &[E], 3, 0), "A round asks for 1 to 100 presses."),
            (
                args(&[1], &[E], 3, 101),
                "A round asks for 1 to 100 presses.",
            ),
        ] {
            assert_eq!(refused(bad), why);
        }
        for fits in [
            args(&handles[..64], &[E], 1, 1),
            args(&[1], &keys[..128], 10, 100),
        ] {
            assert!(plan(fits).is_ok());
        }
    }

    #[test]
    fn plan02_the_board_reaches_the_diagnosis() {
        let json =
            r#"{"keyboard":[65603,65605],"keys":[34,36,18],"rounds":3,"presses":10,"board":"#;
        for (board, kind) in [
            ("null", BoardKind::Unknown),
            (r#""hot-swap""#, BoardKind::HotSwap),
            (r#""soldered""#, BoardKind::Soldered),
            (r#""laptop""#, BoardKind::Laptop),
        ] {
            let args: PlanArgs = serde_json::from_str(&format!("{json}{board}}}")).unwrap();
            let Ok((guide, keyboard, got)) = plan(args) else {
                panic!("refused");
            };
            assert_eq!(
                (guide.plan(), keyboard, got),
                (
                    &Plan {
                        keys: vec![G, J, E],
                        rounds: 3,
                        presses: 10
                    },
                    vec![65603, 65605],
                    kind
                )
            );
        }
        let unknown = format!("{json}\"hotswap\"}}");
        assert!(serde_json::from_str::<PlanArgs>(&unknown).is_err());
    }

    #[test]
    fn plan03_a_keyboard_whose_handles_are_gone_is_refused() {
        let listed = [
            keyboard(65603, "Keychron K2", K2_IDS, Some(K2)),
            keyboard(65605, "Keychron K2, consumer control", K2_IDS, Some(K2)),
        ];
        let paths: Vec<String> = listed.iter().map(|k| k.path.clone()).collect();
        assert_eq!(still_listed(&[65603, 65605], &listed), Ok(paths.clone()));
        assert_eq!(still_listed(&[65605], &listed), Ok(vec![paths[1].clone()]));
        // Replugged, it came back as 65611 and 65613.
        for stale in [&[65603, 65611][..], &[65609][..]] {
            assert_eq!(
                still_listed(stale, &listed),
                Err(
                    "This keyboard was unplugged or reconnected. Pick it again on the Start \
                     screen."
                        .to_string()
                )
            );
        }
    }

    fn key(scan: u16, up: bool, ms: u64) -> keytriage_diagnostics::Entry {
        keytriage_diagnostics::Entry::Key {
            scan,
            up,
            device: 1,
            micros: ms * 1_000,
        }
    }

    #[test]
    fn guide_json01_the_view_is_pinned_and_holds_no_times() {
        let three = Plan {
            keys: vec![G, J, E],
            rounds: 3,
            presses: 10,
        };
        let mut guide = Guide::new(three, &[1]).unwrap();
        let json = |guide: &Guide| serde_json::to_string(&guide_view(guide)).unwrap();
        assert_eq!(
            json(&guide),
            r#"{"key":34,"asked":10,"count":0,"round":0,"rounds":3,"index":0,"keys":3,"done":0,"total":90,"tallies":[]}"#
        );
        for e in [
            key(G, false, 700),
            key(G, true, 790),
            key(G, false, 795),
            key(G, true, 890),
            key(G, false, 1_200),
            key(G, true, 1_300),
        ] {
            guide.entry(&e);
        }
        let mid = json(&guide);
        assert_eq!(
            mid,
            r#"{"key":34,"asked":10,"count":2,"round":0,"rounds":3,"index":0,"keys":3,"done":2,"total":90,"tallies":[[34,3]]}"#
        );
        for at in 2..=10 {
            guide.skip(at * 1_000_000);
        }
        let end = json(&guide);
        assert_eq!(
            end,
            r#"{"key":null,"asked":10,"count":0,"round":2,"rounds":3,"index":2,"keys":3,"done":90,"total":90,"tallies":[[34,3]],"waitMs":150}"#
        );
        for text in [mid, end] {
            for time in ["micros", "_us", "start", "end", "700", "1300"] {
                assert!(!text.contains(time), "{time}");
            }
        }
    }

    fn name(scan: u16, name: &str) -> KeyName {
        KeyName {
            scan,
            name: name.to_string(),
        }
    }

    #[test]
    fn res02_keys_without_a_name_read_as_their_code() {
        let (_, f) = guided_chatter();
        let r = result(&f.diagnose(), &BTreeMap::new());
        let next = &r.findings[0].next;
        assert!(
            next.iter()
                .any(|n| n.starts_with("Swap the key 0012 switch with the key 0022 switch")),
            "{next:?}"
        );
        assert!(r.clean[0].starts_with("Key 0022: no extra key-downs"));
        assert!(r.clean[1].starts_with("Key 0024: no extra key-downs"));
    }

    fn texts(r: &TestResult) -> Vec<String> {
        let mut all = vec![r.resolution.clone()];
        for f in &r.findings {
            all.extend([f.title.clone(), f.level.clone()]);
            all.extend(f.evidence.iter().chain(&f.causes).chain(&f.next).cloned());
        }
        all.extend(r.notes.iter().chain(&r.clean).cloned());
        if let Some(s) = &r.swap {
            all.extend([&s.title, &s.known_good, &s.means, &s.note].map(String::clone));
            all.extend(s.steps.iter().cloned());
        }
        if let Some(o) = &r.outcome {
            all.extend([&o.title, &o.diagnosis].map(String::clone));
            all.extend(o.level.iter().chain(&o.evidence).chain(&o.next).cloned());
        }
        all
    }

    // H skipped after 3 s in each of its rounds, between answered rounds of G and J.
    fn dead_run() -> Fixture {
        guided(
            Plan {
                keys: vec![G, H, J],
                rounds: 3,
                presses: 10,
            },
            |s, k, _| {
                if k == H {
                    (s.wait(ms(3_000)), true)
                } else {
                    (normal(s, k), false)
                }
            },
        )
    }

    // E's first answer is held for 3 s with autorepeat.
    fn stuck_run() -> Fixture {
        let mut first = true;
        guided(
            Plan {
                keys: vec![G, J, E],
                rounds: 3,
                presses: 10,
            },
            move |s, k, _| {
                if k == E && first {
                    first = false;
                    (s.hold(E, ms(3_000), ms(500), ms(33)).wait(ms(200)), false)
                } else {
                    (normal(s, k), false)
                }
            },
        )
    }

    // The swap test of `swap` over `rounds` rounds, in which the `faulty` keys show its kind of
    // fault.
    fn retest(swap: &Swap, faulty: &[u16], rounds: u16) -> Report {
        let kind = swap.kind;
        let mut held = BTreeSet::new();
        let plan = Plan {
            rounds,
            ..swap.plan()
        };
        let f = guided(plan, |s, k, n| {
            if !faulty.contains(&k) {
                return (normal(s, k), false);
            }
            match kind {
                Kind::Chatter if n % 5 == 0 => (
                    s.fragments(k, &[ms(5), ms(5), ms(100)]).wait(ms(200)),
                    false,
                ),
                Kind::Chatter => (normal(s, k), false),
                // A press of another key shows someone was at the keyboard, so two silent keys
                // still read as dead.
                Kind::Dead => (s.press(F, ms(90)).wait(ms(3_000)), true),
                Kind::Stuck if held.insert(k) => {
                    (s.hold(k, ms(3_000), ms(500), ms(33)).wait(ms(200)), false)
                }
                Kind::Stuck => (normal(s, k), false),
            }
        });
        on(BoardKind::HotSwap, f).diagnose()
    }

    fn on(board: BoardKind, f: Fixture) -> Fixture {
        Fixture { board, ..f }
    }

    // guided_chatter on the Start screen's default board offers E with G.
    fn offered() -> Swap {
        let report = on(BoardKind::HotSwap, guided_chatter().1).diagnose();
        Swap::offer(&report, BoardKind::HotSwap).expect("an offer")
    }

    fn labels() -> BTreeMap<u16, String> {
        names(vec![name(E, "E"), name(G, "G"), name(H, "H")]).unwrap()
    }

    #[test]
    fn res03_every_rendered_line_keeps_to_evidence_and_likelihood() {
        let dead = dead_run();
        let mut stuck = Synth::new();
        for _ in 0..2 {
            stuck = stuck.round(E, 10, |s| s.hold(E, ms(6_000), ms(500), ms(33)));
        }
        let paused = Synth::new().round(E, 10, |s| {
            s.taps(E, 4, HOLD, GAP)
                .down(G)
                .wait(ms(3_000))
                .pause(&[G])
                .wait(ms(20_000))
                .resume()
        });
        let reports = [
            guided_chatter().1.diagnose(),
            dead.diagnose(),
            stuck.build().diagnose(),
            paused.build().diagnose(),
            interleaved(7).diagnose(),
        ];
        let labels = labels();
        let mut kinds = Vec::new();
        let mut notes = 0;
        for report in &reports {
            for named in [&labels, &BTreeMap::new()] {
                let r = result(report, named);
                kinds.extend(r.findings.iter().map(|f| f.kind));
                notes += r.notes.len();
                for text in texts(&r) {
                    assert!(hedged(&text), "{text}");
                    assert!(!text.starts_with(char::is_lowercase), "{text}");
                }
            }
        }
        for kind in ["chatter", "dead", "stuck"] {
            assert!(kinds.contains(&kind), "{kind}");
        }
        assert!(notes > 0);

        // The swap's words, for each kind the engine offers it and each outcome.
        let mut outcomes = BTreeSet::new();
        for (main, kind) in [
            (guided_chatter().1, Kind::Chatter),
            (dead_run(), Kind::Dead),
            (stuck_run(), Kind::Stuck),
        ] {
            let report = on(BoardKind::HotSwap, main).diagnose();
            let swap = Swap::offer(&report, BoardKind::HotSwap).expect("an offer");
            assert_eq!(swap.kind, kind);
            let (a, b, full) = (swap.suspect, swap.partner, swap.plan().rounds);
            for (faulty, rounds, expected) in [
                (&[b][..], full, "follows"),
                (&[a][..], full, "stays"),
                (&[a, b][..], full, "both"),
                (&[][..], full, "gone"),
                (&[][..], 1, "unclear"),
            ] {
                let after = retest(&swap, faulty, rounds);
                for named in [&labels, &BTreeMap::new()] {
                    let (offer, _) = ended(&report, BoardKind::HotSwap, None, named);
                    let (judged, _) = ended(&after, BoardKind::HotSwap, Some(&swap), named);
                    assert!(offer.swap.is_some());
                    let outcome = judged.outcome.as_ref().expect("a judgment").outcome;
                    assert_eq!(outcome, expected, "{kind:?}");
                    outcomes.insert(outcome);
                    for text in texts(&offer).into_iter().chain(texts(&judged)) {
                        assert!(hedged(&text), "{text}");
                        assert!(!text.starts_with(char::is_lowercase), "{text}");
                    }
                }
            }
        }
        assert_eq!(outcomes.len(), 5);
    }

    #[test]
    fn swap01_the_offer_reaches_the_page_only_on_a_hot_swap_board() {
        let named = labels();
        for board in [BoardKind::Soldered, BoardKind::Laptop, BoardKind::Unknown] {
            let report = on(board, guided_chatter().1).diagnose();
            let (r, offer) = ended(&report, board, None, &named);
            assert!(offer.is_none() && r.swap.is_none(), "{board:?}");
            assert_eq!(r.findings.len(), 1);
            let json = serde_json::to_string(&r).unwrap();
            assert!(
                json.ends_with(r#""swap":null,"outcome":null}"#),
                "{board:?}"
            );
        }
        // Positive control: the same stream on a hot-swap board.
        let report = on(BoardKind::HotSwap, guided_chatter().1).diagnose();
        let (r, offer) = ended(&report, BoardKind::HotSwap, None, &named);
        assert_eq!(offer, Some(offered()));
        let json = serde_json::to_string(&r).unwrap();
        assert!(
            json.contains(r#""swap":{"suspect":18,"partner":34,"title":"Move the E switch"#),
            "{json}"
        );
        assert!(json.ends_with(r#""outcome":null}"#));
    }

    #[test]
    fn swap02_the_swap_plan_is_the_engines() {
        let swap = offered();
        let Ok((guide, keyboard, board)) = swap_plan(&swap, vec![1]) else {
            panic!("refused");
        };
        assert_eq!(
            (guide.plan(), keyboard, board),
            (&swap.plan(), vec![1], BoardKind::HotSwap)
        );
        assert_eq!(
            swap.plan(),
            Plan {
                keys: vec![E, G],
                rounds: 3,
                presses: 30
            }
        );
        let refused = |keyboard: Vec<isize>| match swap_plan(&swap, keyboard) {
            Ok(_) => panic!("the plan was accepted"),
            Err(e) => e,
        };
        assert_eq!(
            refused(vec![0]),
            "Handle 0 is injected input, not a keyboard."
        );
        assert_eq!(
            refused((1..=65).collect()),
            "A keyboard can't have more than 64 entries."
        );
    }

    // Keys that would hold a time, and a device path as JSON writes it.
    const LEAKS: [&str; 6] = [
        r#""micros""#,
        r#"_us""#,
        r#""start"#,
        r#""end"#,
        "HID#",
        r"\\\\?\\",
    ];

    fn leaks(json: &str) -> Vec<&'static str> {
        LEAKS.into_iter().filter(|l| json.contains(l)).collect()
    }

    #[test]
    fn swap03_swap_views_hold_no_times_or_paths() {
        let swap = offered();
        let main = on(BoardKind::HotSwap, guided_chatter().1).diagnose();
        let (offer, _) = ended(&main, BoardKind::HotSwap, None, &labels());
        let after = swap_chatter(swap.plan(), &[G]).diagnose();
        let (judged, _) = ended(&after, BoardKind::HotSwap, Some(&swap), &labels());
        assert!(offer.swap.is_some() && judged.outcome.is_some());
        for r in [&offer, &judged] {
            assert_eq!(leaks(&serde_json::to_string(r).unwrap()), [""; 0]);
        }
        // Positive controls: a planted time and a planted path are caught.
        let mut planted: Value = serde_json::to_value(&judged).unwrap();
        planted["outcome"]["micros"] = json!(1_500);
        assert_eq!(leaks(&planted.to_string()), [r#""micros""#]);
        let mut planted: Value = serde_json::to_value(&offer).unwrap();
        planted["swap"]["title"] = json!(r"\\?\HID#VID_05AC&PID_024F&MI_00#8&2d5c1f1a&0&0000");
        assert_eq!(leaks(&planted.to_string()), ["HID#", r"\\\\?\\"]);
    }

    #[test]
    fn swap04_a_retest_ends_in_its_judgment_and_offers_nothing_more() {
        let swap = offered();
        let after = swap_chatter(swap.plan(), &[G]).diagnose();
        // The retest's own report would offer G with E, a swap already made.
        assert!(Swap::offer(&after, BoardKind::HotSwap).is_some());
        let (r, offer) = ended(&after, BoardKind::HotSwap, Some(&swap), &labels());
        assert_eq!(offer, None);
        assert!(r.swap.is_none());
        assert!(r.findings.is_empty() && r.notes.is_empty() && r.clean.is_empty());
        let o = r.outcome.as_ref().expect("a judgment");
        assert_eq!(
            (
                o.outcome,
                o.tile,
                o.flagged.as_slice(),
                o.confidence,
                o.level.as_deref(),
                o.strong
            ),
            (
                "follows",
                Some(G),
                &[G][..],
                Some("very-high"),
                Some("Very high"),
                true
            )
        );
        assert_eq!(o.title, "The fault moved with the switch.");
        assert_eq!(
            o.evidence[1],
            "G: 18 of 90 presses sent an extra key-down (a rate of at least 13%)"
        );
        // The drawing still gets the retest's counts.
        let keys: Vec<(u16, u32)> = r.keys.iter().map(|k| (k.scan, k.count)).collect();
        assert_eq!(keys, [(E, 90), (G, 108)]);
        assert_eq!(r.resolution, after.aggregates.limits.poll.words());

        // Too short to judge: no confidence and no badge.
        let (r, _) = ended(
            &retest(&swap, &[], 1),
            BoardKind::HotSwap,
            Some(&swap),
            &labels(),
        );
        let o = r.outcome.as_ref().expect("a judgment");
        assert_eq!(
            (o.outcome, o.confidence, o.level.as_deref(), o.strong),
            ("unclear", None, None, false)
        );
        let json = serde_json::to_string(o).unwrap();
        assert!(
            json.contains(r#""confidence":null,"level":null,"strong":false"#),
            "{json}"
        );
    }

    fn at_path(handle: isize, path: &str) -> Keyboard {
        Keyboard {
            path: path.to_string(),
            ..keyboard(handle, "Galaxy80", Some((0x05ac, 0x024f)), Some(K2))
        }
    }

    #[test]
    fn refind01_the_same_device_is_found_again_by_its_path() {
        const MAIN: &str = r"\\?\HID#VID_05AC&PID_024F&MI_00#8&2d5c1f1a&0&0000#{884b96c3}";
        const CONSUMER: &str =
            r"\\?\HID#VID_05AC&PID_024F&MI_01&Col01#8&13b2a5e4&0&0000#{884b96c3}";
        const OTHER: &str = r"\\?\HID#VID_046D&PID_C31C&MI_00#8&3f2e9d01&0&0000#{884b96c3}";
        let kept = [MAIN.to_string(), CONSUMER.to_string()];
        let same = [at_path(11, MAIN), at_path(12, CONSUMER)];
        assert_eq!(refind(&kept, &same), Ok(vec![11, 12]));
        // Replugged on the same port: new handles, listed in another order.
        let replugged = [at_path(22, CONSUMER), at_path(21, MAIN)];
        assert_eq!(refind(&kept, &replugged), Ok(vec![21, 22]));
        // Another keyboard now holds an old handle, which is never taken.
        let reused = [at_path(11, OTHER), at_path(21, MAIN), at_path(22, CONSUMER)];
        assert_eq!(refind(&kept, &reused), Ok(vec![21, 22]));
        for listed in [
            &[at_path(11, OTHER), at_path(12, CONSUMER)][..],
            &[at_path(21, MAIN)][..],
            &[][..],
        ] {
            assert_eq!(
                refind(&kept, listed),
                Err(
                    "The keyboard isn't listed. Plug it back into the port it used, then try \
                     again."
                        .to_string()
                )
            );
        }
    }

    fn refusal(labels: Vec<KeyName>) -> String {
        match names(labels) {
            Ok(_) => panic!("the names were accepted"),
            Err(e) => e,
        }
    }

    #[test]
    fn res04_names_must_fit_a_drawn_key() {
        let many: Vec<KeyName> = (0..257).map(|scan| name(scan, "Key")).collect();
        assert_eq!(refusal(many), "A layout can't name more than 256 keys.");
        let most: Vec<KeyName> = (0..256).map(|scan| name(scan, "Key")).collect();
        assert_eq!(names(most).unwrap().len(), 256);
        for (bad, why) in [
            ("", "A key name can't be empty."),
            ("  ", "A key name can't be empty."),
            (
                "Twenty-five characters...",
                "A key name can't be longer than 24 characters.",
            ),
            ("E\n", "A key name can't hold control characters."),
            ("E\u{7f}", "A key name can't hold control characters."),
        ] {
            assert_eq!(refusal(vec![name(G, "G"), name(E, bad)]), why);
        }
        // Characters are counted, not bytes.
        let wide = "Ä".repeat(24);
        assert_eq!(names(vec![name(E, &wide)]).unwrap()[&E], wide);
    }

    #[test]
    fn bars01_each_bar_is_a_run_of_whole_bins() {
        for &(_, edge) in &GAP_BARS[..GAP_BARS.len() - 1] {
            assert!(EDGES_MS.contains(&edge), "{edge}");
        }
        let bars = gap_bars(&Histogram([1; BINS]));
        let counts: Vec<(&str, u32)> = bars.iter().map(|b| (b.label, b.count)).collect();
        assert_eq!(
            counts,
            [
                ("<4", 2),
                ("4–12", 1),
                ("12–20", 1),
                ("20–36", 2),
                ("36–100", 2),
                ("100+", 6)
            ]
        );
    }
}
