// The exported report: `Report::saved()` under a fixed header, and nothing else. Every field of the
// engine's saved types is named here, so a field added to one fails to compile until it has had a
// privacy review.
use std::collections::BTreeMap;

use keytriage_diagnostics::params::{BINS, EDGES_MS};
use keytriage_diagnostics::{
    Aggregates, Histogram, KeyAggregate, Limits, PollEstimate, PromptTally,
};
use serde::Serialize;

const APP: &str = "keytriage";
const FORMAT: u16 = 1;
const NOTE: &str = "Per-key counts and histograms only. No ordered key events.";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportFile {
    app: &'static str,
    format: u16,
    note: &'static str,
    rules: u16,
    bin_edges_ms: &'static [u64],
    limits: LimitsFile,
    // Keys are positions as 4-digit hex, which sorts in scan order.
    keys: BTreeMap<String, KeyFile>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LimitsFile {
    poll: &'static str,
    injected: u32,
    other_devices: u32,
    unknown_codes: u32,
    fake_shifts: u32,
    overruns: u32,
    pauses: u32,
    timing_unknown: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyFile {
    downs: u32,
    ups: u32,
    episodes: u32,
    presses: u32,
    affected: u32,
    extra_downs: u32,
    repeats: u32,
    duplicates: u32,
    orphan_ups: u32,
    interrupted: u32,
    after_resume: u32,
    long_holds: u32,
    unreleased: u32,
    borderline: u32,
    timing_unknown: u32,
    hold: [u32; BINS],
    release_gap: [u32; BINS],
    interval: [u32; BINS],
    prompted: Option<PromptFile>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PromptFile {
    rounds: u16,
    asked: u32,
    presses: u32,
    affected: u32,
    extra_downs: u32,
    rounds_pressed: u16,
    rounds_affected: u16,
    assessed: u16,
    silent: u16,
    not_assessed: u16,
}

pub fn report_json(saved: &Aggregates) -> String {
    let Aggregates {
        rules,
        limits,
        keys,
    } = saved;
    let Limits {
        poll,
        injected,
        other_devices,
        unknown_codes,
        fake_shifts,
        overruns,
        pauses,
        timing_unknown,
    } = *limits;
    let file = ReportFile {
        app: APP,
        format: FORMAT,
        note: NOTE,
        rules: *rules,
        bin_edges_ms: &EDGES_MS,
        limits: LimitsFile {
            poll: poll_name(poll),
            injected,
            other_devices,
            unknown_codes,
            fake_shifts,
            overruns,
            pauses,
            timing_unknown,
        },
        keys: keys
            .iter()
            .map(|(&scan, key)| (format!("{scan:04X}"), key_file(key)))
            .collect(),
    };
    let mut text = serde_json::to_string_pretty(&file).expect("the report file holds only counts");
    text.push('\n');
    text
}

fn poll_name(poll: PollEstimate) -> &'static str {
    match poll {
        PollEstimate::Unknown => "unknown",
        PollEstimate::No8Or16Ms => "no-8-or-16-ms",
        PollEstimate::Ms8 => "8-ms",
        PollEstimate::Ms16OrSlower => "16-ms-or-slower",
    }
}

fn key_file(key: &KeyAggregate) -> KeyFile {
    let KeyAggregate {
        downs,
        ups,
        episodes,
        presses,
        affected,
        extra_downs,
        repeats,
        duplicates,
        orphan_ups,
        interrupted,
        after_resume,
        long_holds,
        unreleased,
        borderline,
        timing_unknown,
        hold: Histogram(hold),
        release_gap: Histogram(release_gap),
        interval: Histogram(interval),
        prompted,
    } = *key;
    KeyFile {
        downs,
        ups,
        episodes,
        presses,
        affected,
        extra_downs,
        repeats,
        duplicates,
        orphan_ups,
        interrupted,
        after_resume,
        long_holds,
        unreleased,
        borderline,
        timing_unknown,
        hold,
        release_gap,
        interval,
        prompted: prompted.map(prompt_file),
    }
}

fn prompt_file(tally: PromptTally) -> PromptFile {
    let PromptTally {
        rounds,
        asked,
        presses,
        affected,
        extra_downs,
        rounds_pressed,
        rounds_affected,
        assessed,
        silent,
        not_assessed,
    } = tally;
    PromptFile {
        rounds,
        asked,
        presses,
        affected,
        extra_downs,
        rounds_pressed,
        rounds_affected,
        assessed,
        silent,
        not_assessed,
    }
}

// The page suggests the name, and the Save dialog lets the user change it. Only a name of this
// shape is taken from the page, so no page text reaches the file or picks where it goes.
pub fn check_file_name(name: &str) -> Result<(), String> {
    const SHAPE: &str = "keytriage-####.##.##-####.json";
    let fits = name.len() == SHAPE.len()
        && name.bytes().zip(SHAPE.bytes()).all(|(c, s)| match s {
            b'#' => c.is_ascii_digit(),
            _ => c == s,
        });
    if fits {
        Ok(())
    } else {
        Err("A report's file name must read keytriage-YYYY.MM.DD-HHMM.json.".to_string())
    }
}

// The file may hold counts, fixed-length histograms and fixed words, and nothing that could carry
// an ordered list: arrays sit only at the bin edges and the histograms, strings only in three fixed
// fields, and every name is a known field or a key's position.
#[cfg(test)]
pub fn no_sequence(text: &str) -> Result<(), String> {
    use serde_json::{Map, Value};

    const TOP: [&str; 7] = [
        "app",
        "format",
        "note",
        "rules",
        "binEdgesMs",
        "limits",
        "keys",
    ];
    const LIMITS: [&str; 8] = [
        "poll",
        "injected",
        "otherDevices",
        "unknownCodes",
        "fakeShifts",
        "overruns",
        "pauses",
        "timingUnknown",
    ];
    const POLLS: [&str; 4] = ["unknown", "no-8-or-16-ms", "8-ms", "16-ms-or-slower"];
    const KEY: [&str; 19] = [
        "downs",
        "ups",
        "episodes",
        "presses",
        "affected",
        "extraDowns",
        "repeats",
        "duplicates",
        "orphanUps",
        "interrupted",
        "afterResume",
        "longHolds",
        "unreleased",
        "borderline",
        "timingUnknown",
        "hold",
        "releaseGap",
        "interval",
        "prompted",
    ];
    const HISTOGRAMS: [&str; 3] = ["hold", "releaseGap", "interval"];
    const PROMPTED: [&str; 10] = [
        "rounds",
        "asked",
        "presses",
        "affected",
        "extraDowns",
        "roundsPressed",
        "roundsAffected",
        "assessed",
        "silent",
        "notAssessed",
    ];

    fn object<'a>(
        v: &'a Value,
        at: &str,
        names: &[&str],
    ) -> Result<&'a Map<String, Value>, String> {
        let map = v.as_object().ok_or(format!("{at} is not an object"))?;
        match map.keys().find(|k| !names.contains(&k.as_str())) {
            Some(k) => Err(format!("{at}.{k} is not a saved field")),
            None => Ok(map),
        }
    }
    fn count(v: &Value, at: &str) -> Result<(), String> {
        v.as_u64().map(drop).ok_or(format!("{at} is not a count"))
    }
    fn word(v: &Value, at: &str, words: &[&str]) -> Result<(), String> {
        match v.as_str() {
            Some(w) if words.contains(&w) => Ok(()),
            _ => Err(format!("{at} is not one of its fixed words")),
        }
    }
    fn position(code: &str) -> bool {
        code.len() == 4
            && code
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
    }

    let file: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    for (name, v) in object(&file, "file", &TOP)? {
        match name.as_str() {
            "app" => word(v, name, &[APP])?,
            "note" => word(v, name, &[NOTE])?,
            "format" | "rules" => count(v, name)?,
            "binEdgesMs" if *v == serde_json::json!(EDGES_MS) => {}
            "binEdgesMs" => return Err("binEdgesMs is not the engine's bin edges".to_string()),
            "limits" => {
                for (n, v) in object(v, name, &LIMITS)? {
                    let at = format!("limits.{n}");
                    match n.as_str() {
                        "poll" => word(v, &at, &POLLS)?,
                        _ => count(v, &at)?,
                    }
                }
            }
            "keys" => {
                let keys = v.as_object().ok_or("keys is not an object")?;
                for (code, key) in keys {
                    if !position(code) {
                        return Err(format!("keys.{code} is not a key position"));
                    }
                    for (n, v) in object(key, &format!("keys.{code}"), &KEY)? {
                        let at = format!("keys.{code}.{n}");
                        if HISTOGRAMS.contains(&n.as_str()) {
                            match v.as_array() {
                                Some(bins) if bins.len() == BINS => {
                                    for bin in bins {
                                        count(bin, &at)?;
                                    }
                                }
                                _ => return Err(format!("{at} is not {BINS} bins")),
                            }
                        } else if n == "prompted" {
                            if !v.is_null() {
                                for (n, v) in object(v, &at, &PROMPTED)? {
                                    count(v, &format!("{at}.{n}"))?;
                                }
                            }
                        } else {
                            count(v, &at)?;
                        }
                    }
                }
            }
            _ => unreachable!("object() admits only the top-level fields"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use keytriage_diagnostics::fixture::{
        Fixture, GAP, HOLD, Synth, guided, guided_chatter, interleaved, ms, normal,
    };
    use keytriage_diagnostics::{Entry, Guide, Plan};
    use serde_json::{Value, json};

    use super::*;
    use crate::session_core::{Core, input};

    const E: u16 = 0x12;
    const K: u16 = 0x25;

    fn parsed(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn ns01_the_file_holds_no_list_or_free_text() {
        let (plan, f) = guided_chatter();
        let text = report_json(&f.diagnose().saved());
        assert_eq!(no_sequence(&text), Ok(()));
        assert!(text.ends_with("}\n"));

        // The ordered events, as the page got them.
        let start = Instant::now();
        let guide = Guide::new(plan, &f.keyboard).unwrap();
        let mut core = Core::new(start, Some((guide, f.keyboard.clone(), f.board)));
        let events: Vec<Value> = f
            .entries
            .iter()
            .map(|e| serde_json::to_value(core.input(input(start, e)).0).unwrap())
            .collect();
        let mut spliced = parsed(&text);
        spliced["events"] = Value::Array(events);
        let mut longer = parsed(&text);
        longer["keys"]["0012"]["hold"]
            .as_array_mut()
            .unwrap()
            .push(json!(0));
        let mut named = parsed(&text);
        named["keys"]["0012"]["name"] = json!("E");
        // A key's position is the one name the file doesn't fix, so a joined or typed one must fail.
        let renamed = |code: &str| {
            let mut file = parsed(&text);
            let keys = file["keys"].as_object_mut().unwrap();
            let key = keys.remove("0012").unwrap();
            keys.insert(code.to_string(), key);
            file
        };
        let with = |pointer: &str, value: Value| {
            let mut file = parsed(&text);
            *file.pointer_mut(pointer).unwrap() = value;
            file
        };
        for (planted, why) in [
            (spliced, "file.events is not a saved field"),
            (longer, "keys.0012.hold is not 14 bins"),
            (named, "keys.0012.name is not a saved field"),
            (renamed("0012,0022"), "keys.0012,0022 is not a key position"),
            (renamed("E"), "keys.E is not a key position"),
            (renamed("00e0"), "keys.00e0 is not a key position"),
            (
                with("/note", json!("E G J E")),
                "note is not one of its fixed words",
            ),
            (
                with("/app", json!("E G J E")),
                "app is not one of its fixed words",
            ),
            (
                with("/limits/poll", json!("E")),
                "limits.poll is not one of its fixed words",
            ),
            (
                with("/limits/pauses", json!("E")),
                "limits.pauses is not a count",
            ),
            (
                with("/keys/0012/downs", json!("E")),
                "keys.0012.downs is not a count",
            ),
            (
                with("/keys/0012/downs", json!(-1)),
                "keys.0012.downs is not a count",
            ),
            (
                with("/keys/0012/hold/3", json!("E")),
                "keys.0012.hold is not a count",
            ),
            (
                with("/keys/0012/prompted/presses", json!("E")),
                "keys.0012.prompted.presses is not a count",
            ),
            (with("/rules", json!("E")), "rules is not a count"),
            (
                with("/binEdgesMs/3", json!(21)),
                "binEdgesMs is not the engine's bin edges",
            ),
        ] {
            assert_eq!(
                no_sequence(&serde_json::to_string_pretty(&planted).unwrap()),
                Err(why.to_string())
            );
        }
    }

    fn same_for_every_interleaving(write: impl Fn(&Fixture) -> String) -> bool {
        let first = write(&interleaved(1));
        (2..=30).all(|seed| write(&interleaved(seed)) == first)
    }

    #[test]
    fn ns02_the_file_does_not_depend_on_how_keys_interleave() {
        assert!(same_for_every_interleaving(|f| report_json(
            &f.diagnose().aggregates
        )));
    }

    #[test]
    fn ns02_positive_control_catches_an_ordered_writer() {
        let ordered = |f: &Fixture| {
            let mut first_downs: Vec<u16> = Vec::new();
            for e in &f.entries {
                if let Entry::Key {
                    scan, up: false, ..
                } = *e
                    && !first_downs.contains(&scan)
                {
                    first_downs.push(scan);
                }
            }
            let mut text = report_json(&f.diagnose().aggregates);
            for scan in first_downs {
                text += &format!("{scan:04X}\n");
            }
            text
        };
        assert!(!same_for_every_interleaving(ordered));
    }

    #[test]
    fn ns03_free_typing_stays_out_of_the_file() {
        let report = Synth::new()
            .round(E, 12, |s| s.taps(E, 12, HOLD, GAP))
            .taps(K, 20, HOLD, GAP)
            .build()
            .diagnose();
        assert!(report.aggregates.keys.contains_key(&K));
        let text = report_json(&report.saved());
        assert!(text.contains(r#""0012""#));
        assert!(!text.contains("0025"));
        assert_eq!(no_sequence(&text), Ok(()));
    }

    // The app prompts every plain key, so free typing lands on prompted keys too. Presses of H
    // and U outside their own rounds, before H's first round and during other keys' rounds, must
    // leave the file as it was. The quiet run waits as long as the typing took.
    #[test]
    fn ns04_typing_on_prompted_keys_outside_their_rounds_stays_out_of_the_file() {
        const H: u16 = 0x23;
        const U: u16 = 0x16;
        let run = |typed: bool| {
            let tap = |s: Synth, key: u16| {
                if typed {
                    s.press(key, ms(100)).wait(ms(200))
                } else {
                    s.wait(ms(300))
                }
            };
            let plan = Plan {
                keys: vec![E, H, U],
                rounds: 3,
                presses: 10,
            };
            let f = guided(plan, |s, key, n| {
                let mut s = normal(s, key);
                if n == 3 {
                    match key {
                        E => s = tap(tap(tap(s, H), U), H),
                        U => s = tap(s, H),
                        _ => {}
                    }
                }
                (s, false)
            });
            report_json(&f.diagnose().saved())
        };
        let (quiet, typed) = (run(false), run(true));
        assert!(typed.contains(r#""0023""#) && typed.contains(r#""0016""#));
        assert_eq!(typed, quiet);
    }

    #[test]
    fn ex01_only_the_suggested_name_is_taken() {
        assert_eq!(check_file_name("keytriage-2026.09.28-1412.json"), Ok(()));
        for bad in [
            "",
            "keytriage-2026.09.28-1412.txt",
            "keytriage-2026.09.28-1412.json.exe",
            "keytriage-2026.09.28-141.json",
            "keytriage-2026-09-28-1412.json",
            r"..\keytriage-2026.09.28-1412.json",
            "../keytriage-2026.09.28-1412.json",
            r"C:\keytriage-2026.09.28-1412.json",
            "keytriage-2026.09.28-14/2.json",
            r"keytriage-2026.09.28-14\2.json",
            "keytriage-2026.09.28-..12.json",
            "Keytriage-2026.09.28-1412.json",
        ] {
            assert!(check_file_name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn ex02_every_saved_field_reaches_the_file_under_its_own_name() {
        let histogram = |base: u32| Histogram(std::array::from_fn(|i| base + i as u32));
        let key = KeyAggregate {
            downs: 1,
            ups: 2,
            episodes: 3,
            presses: 4,
            affected: 5,
            extra_downs: 6,
            repeats: 7,
            duplicates: 8,
            orphan_ups: 9,
            interrupted: 10,
            after_resume: 11,
            long_holds: 12,
            unreleased: 13,
            borderline: 14,
            timing_unknown: 15,
            hold: histogram(100),
            release_gap: histogram(200),
            interval: histogram(300),
            prompted: Some(PromptTally {
                rounds: 21,
                asked: 22,
                presses: 23,
                affected: 24,
                extra_downs: 25,
                rounds_pressed: 26,
                rounds_affected: 27,
                assessed: 28,
                silent: 29,
                not_assessed: 30,
            }),
        };
        let saved = Aggregates {
            rules: 2,
            limits: Limits {
                poll: PollEstimate::Ms8,
                injected: 41,
                other_devices: 42,
                unknown_codes: 43,
                fake_shifts: 44,
                overruns: 45,
                pauses: 46,
                timing_unknown: 47,
            },
            keys: [
                (0xE048, key),
                (
                    0x12,
                    KeyAggregate {
                        prompted: None,
                        ..Default::default()
                    },
                ),
            ]
            .into(),
        };
        let bins = |base: u32| (base..base + BINS as u32).collect::<Vec<u32>>();
        let zeros = vec![0; BINS];
        let text = report_json(&saved);
        assert!(text.starts_with("{\n  \"app\": \"keytriage\",\n  \"format\": 1,\n"));
        assert_eq!(
            parsed(&text),
            json!({
                "app": "keytriage",
                "format": 1,
                "note": "Per-key counts and histograms only. No ordered key events.",
                "rules": 2,
                "binEdgesMs": [1, 4, 12, 20, 28, 36, 60, 100, 164, 260, 516, 1028, 2052],
                "limits": {
                    "poll": "8-ms",
                    "injected": 41,
                    "otherDevices": 42,
                    "unknownCodes": 43,
                    "fakeShifts": 44,
                    "overruns": 45,
                    "pauses": 46,
                    "timingUnknown": 47
                },
                "keys": {
                    "0012": {
                        "downs": 0, "ups": 0, "episodes": 0, "presses": 0, "affected": 0,
                        "extraDowns": 0, "repeats": 0, "duplicates": 0, "orphanUps": 0,
                        "interrupted": 0, "afterResume": 0, "longHolds": 0, "unreleased": 0,
                        "borderline": 0, "timingUnknown": 0,
                        "hold": zeros, "releaseGap": zeros, "interval": zeros,
                        "prompted": null
                    },
                    "E048": {
                        "downs": 1, "ups": 2, "episodes": 3, "presses": 4, "affected": 5,
                        "extraDowns": 6, "repeats": 7, "duplicates": 8, "orphanUps": 9,
                        "interrupted": 10, "afterResume": 11, "longHolds": 12, "unreleased": 13,
                        "borderline": 14, "timingUnknown": 15,
                        "hold": bins(100), "releaseGap": bins(200), "interval": bins(300),
                        "prompted": {
                            "rounds": 21, "asked": 22, "presses": 23, "affected": 24,
                            "extraDowns": 25, "roundsPressed": 26, "roundsAffected": 27,
                            "assessed": 28, "silent": 29, "notAssessed": 30
                        }
                    }
                }
            })
        );
        // Keys come in scan order, E0-prefixed codes last.
        assert!(text.find("\"0012\"").unwrap() < text.find("\"E048\"").unwrap());
        assert_eq!(no_sequence(&text), Ok(()));
    }
}
