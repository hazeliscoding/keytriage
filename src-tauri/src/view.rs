// What the page is sent and what it may ask for. Only these shapes cross to the page, so a device
// path or container ID never does.
use keytriage_input::Keyboard;
use serde::Serialize;

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
}
