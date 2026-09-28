// What the page is sent and what it may ask for. Only these shapes cross to the page, so a device
// path or container ID never does.
use std::collections::BTreeMap;

use keytriage_diagnostics::params::{EDGES_MS, END_WAIT_US};
use keytriage_diagnostics::{
    BoardKind, Confidence, Guide, Histogram, Kind, MAX_PRESSES, MAX_ROUNDS, Note, Plan, PlanError,
    Report, code_label,
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
// plan on them would wait on its first prompt for good.
pub fn still_listed(keyboard: &[isize], listed: &[Keyboard]) -> Result<(), String> {
    if keyboard
        .iter()
        .all(|&handle| listed.iter().any(|k| k.handle == handle))
    {
        Ok(())
    } else {
        Err(RECONNECTED.to_string())
    }
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

// Keys the page didn't name read as their code, "key 0012".
pub fn result(report: &Report, names: &BTreeMap<u16, String>) -> TestResult {
    let label = |scan: u16| {
        names
            .get(&scan)
            .cloned()
            .unwrap_or_else(|| code_label(scan))
    };
    let saved = report.saved();
    let findings = report
        .findings
        .iter()
        .map(|f| {
            let lines = f.lines(&label);
            let all = |lines: &[String]| lines.iter().map(|l| sentence(l)).collect();
            FindingView {
                key: f.key,
                kind: match f.kind() {
                    Kind::Chatter => "chatter",
                    Kind::Dead => "dead",
                    Kind::Stuck => "stuck",
                },
                confidence: match f.confidence {
                    Confidence::Low => "low",
                    Confidence::Medium => "medium",
                    Confidence::High => "high",
                    Confidence::VeryHigh => "very-high",
                },
                title: sentence(f.kind().words()),
                level: sentence(f.confidence.words()),
                strong: f.confidence >= Confidence::High,
                evidence: all(&lines.evidence),
                causes: all(&lines.causes),
                next: all(&lines.next),
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
    }
}

#[cfg(test)]
mod tests {
    use keytriage_diagnostics::fixture::{
        GAP, HOLD, Synth, guided, guided_chatter, interleaved, ms, normal,
    };
    use keytriage_diagnostics::hedged;
    use keytriage_diagnostics::params::BINS;

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
    const G: u16 = 0x22;
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
        assert_eq!(still_listed(&[65603, 65605], &listed), Ok(()));
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
        all
    }

    #[test]
    fn res03_every_rendered_line_keeps_to_evidence_and_likelihood() {
        const H: u16 = 0x23;
        let dead = guided(
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
        );
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
        let labels = names(vec![name(E, "E"), name(G, "G"), name(H, "H")]).unwrap();
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
