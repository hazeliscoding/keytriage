// What the page is sent and what it may ask for. Only these shapes cross to the page, so a device
// path or container ID never does.
use keytriage_diagnostics::{BoardKind, Guide, MAX_PRESSES, MAX_ROUNDS, Plan, PlanError};
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
    }
}

#[cfg(test)]
mod tests {
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
            r#"{"key":null,"asked":10,"count":0,"round":2,"rounds":3,"index":2,"keys":3,"done":90,"total":90,"tallies":[[34,3]]}"#
        );
        for text in [mid, end] {
            for time in ["micros", "_us", "start", "end", "700", "1300"] {
                assert!(!text.contains(time), "{time}");
            }
        }
    }
}
