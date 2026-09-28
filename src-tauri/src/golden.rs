// The Done-when run, from capture to report. The engine's synthetic chatter stream goes through
// Core as the capture callback feeds it, and every payload the page gets, in order, is pinned in
// src/app/testing/guided-chatter.json, which the page's own test replays. A change to the engine's
// words, a payload's shape or the Guide shows up as a diff of that file, which KEYTRIAGE_BLESS=1
// rewrites once the change has been read.
use std::path::PathBuf;
use std::time::Instant;

use keytriage_diagnostics::fixture::{Fixture, guided_chatter};
use keytriage_diagnostics::params::END_WAIT_US;
use keytriage_diagnostics::{BoardKind, Confidence, Evidence, NextTest, Report};
use keytriage_input::Keyboard;
use serde::Serialize;
use serde_json::{Value, json};

use crate::export::{no_sequence, report_json};
use crate::session_core::{Core, at, input, rounds};
use crate::view::{self, KeyName, KeyboardGroup, PlanArgs, TestResult, groups, names, result};

const E: u16 = 0x12;
const G: u16 = 0x22;
const J: u16 = 0x24;

const FILE: &str = "../src/app/testing/guided-chatter.json";
const SOURCE: &str = "synthetic: crates/diagnostics fixture::guided_chatter";

// What the page sends: G, J and E on the Start screen's default board, and their drawn names.
const PLAN: &str =
    r#"{"keyboard":[1],"keys":[34,36,18],"rounds":3,"presses":10,"board":"hot-swap"}"#;
const LABELS: &str = r#"[{"scan":34,"name":"G"},{"scan":36,"name":"J"},{"scan":18,"name":"E"}]"#;

fn keyboards() -> Vec<KeyboardGroup> {
    groups(&[Keyboard {
        handle: 1,
        path: String::new(),
        vendor_id: Some(0),
        product_id: Some(1),
        manufacturer: None,
        product: Some("Synthetic keyboard".to_string()),
        container: None,
    }])
}

// Typed, so each payload keeps the field order it has on the wire.
#[derive(Serialize)]
struct Step<T> {
    event: &'static str,
    payload: T,
}

fn step(event: &'static str, payload: impl Serialize) -> String {
    serde_json::to_string(&Step { event, payload }).unwrap()
}

fn parsed(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

struct Run {
    fixture: Fixture,
    script: Vec<String>,
    core: Core,
    start: Instant,
}

// Core as start_test builds it from the page's plan, fed the stream as the capture callback is.
fn run() -> Run {
    let (engine_plan, fixture) = guided_chatter();
    let args: PlanArgs = serde_json::from_str(PLAN).unwrap();
    let test = view::plan(args).unwrap();
    assert_eq!(test.0.plan(), &engine_plan);
    let start = Instant::now();
    let mut core = Core::new(start, Some(test));
    let mut script = vec![step("test:guide", core.view().unwrap())];
    for e in &fixture.entries {
        let (entry, view) = core.input(input(start, e));
        script.push(step("test:event", entry));
        if let Some(view) = view {
            script.push(step("test:guide", view));
        }
    }
    Run {
        fixture,
        script,
        core,
        start,
    }
}

fn finish(run: &mut Run) -> (Report, TestResult) {
    let report = run.core.finish(at(run.start, run.fixture.end_us)).unwrap();
    let labels: Vec<KeyName> = serde_json::from_str(LABELS).unwrap();
    let result = result(&report, &names(labels).unwrap());
    (report, result)
}

fn indented(value: &impl Serialize) -> String {
    serde_json::to_string_pretty(value)
        .unwrap()
        .replace('\n', "\n  ")
}

// One line per emitted event, so a re-blessed file's diff reads event by event.
fn golden(script: &[String], result: &TestResult) -> String {
    let fields = [
        format!("\"source\": \"{SOURCE}\""),
        format!("\"plan\": {PLAN}"),
        format!("\"keyboards\": {}", indented(&keyboards())),
        format!("\"labels\": {LABELS}"),
        format!("\"script\": [\n    {}\n  ]", script.join(",\n    ")),
        format!("\"result\": {}", indented(result)),
    ];
    format!("{{\n  {}\n}}\n", fields.join(",\n  "))
}

#[test]
fn m3_done_guided_chatter() {
    // Taking the rounds ends the Guide, so the diagnosis comes from a second, identical run.
    let stamped = {
        let mut once = run();
        rounds(&mut once.core, once.fixture.end_us)
    };
    let mut run = run();
    assert_eq!(stamped, run.fixture.rounds);
    assert!(stamped.iter().all(|r| r.asked == 10));
    assert_eq!(
        stamped.iter().map(|r| r.key).collect::<Vec<u16>>(),
        [G, J, E, G, J, E, G, J, E]
    );
    let (report, result) = finish(&mut run);

    // The app's run is the engine's own run, on the board the Start screen preselects.
    let hot_swap = Fixture {
        board: BoardKind::HotSwap,
        ..guided_chatter().1
    };
    assert_eq!(report, hot_swap.diagnose());

    assert_eq!(report.findings.len(), 1);
    let finding = &report.findings[0];
    assert_eq!((finding.key, finding.confidence), (E, Confidence::VeryHigh));
    let Evidence::Chatter(c) = finding.evidence else {
        panic!("not chatter");
    };
    assert_eq!(
        (c.presses, c.affected, c.rounds, c.rounds_affected),
        (30, 6, 3, 3)
    );
    assert_eq!(
        finding.next_tests,
        [
            NextTest::SwapSwitch {
                suspect: E,
                partner: Some(G)
            },
            NextTest::CleanContacts { key: E },
            NextTest::RaiseDebounce,
        ]
    );

    let card = &result.findings[0];
    assert_eq!(
        (
            card.key,
            card.title.as_str(),
            card.level.as_str(),
            card.evidence[0].as_str()
        ),
        (
            E,
            "Possible chatter",
            "Very high",
            "6 of 30 presses sent an extra key-down (a rate of at least 9.5%)"
        )
    );
    assert!(card.next[0].starts_with("Swap the E switch with the G switch"));

    // The last view says the plan is done, and the page ends the test END_WAIT_US after it. The E
    // round's last chatter fragment follows it as events, within that wait.
    let script: Vec<Value> = run.script.iter().map(|s| parsed(s)).collect();
    let views: Vec<&Value> = script
        .iter()
        .filter(|s| s["event"] == "test:guide")
        .map(|s| &s["payload"])
        .collect();
    let last = views.last().unwrap();
    assert!(last["key"].is_null());
    assert_eq!((&last["done"], &last["total"]), (&json!(90), &json!(90)));
    assert_eq!(last["waitMs"], json!(END_WAIT_US / 1000));
    let prompts = &views[..views.len() - 1];
    assert!(
        prompts
            .iter()
            .all(|v| !v["key"].is_null() && v.get("waitMs").is_none())
    );
    let done = script
        .iter()
        .position(|s| s["event"] == "test:guide" && s["payload"]["key"].is_null())
        .unwrap();
    let micros = |s: &Value| s["payload"]["micros"].as_u64().unwrap();
    let closed = micros(&script[done - 1]);
    let trailing: Vec<u64> = script[done + 1..].iter().map(micros).collect();
    assert_eq!(trailing.len(), 2);
    assert!(trailing.iter().all(|&t| t - closed < END_WAIT_US));

    let export = report_json(&report.saved());
    assert_eq!(no_sequence(&export), Ok(()));
    for key in ["\"0012\"", "\"0022\"", "\"0024\""] {
        assert!(export.contains(key), "{key}");
    }
}

#[test]
fn golden_holds_only_the_synthetic_stream() {
    let mut run = run();
    let (_, result) = finish(&mut run);
    let text = golden(&run.script, &result);
    let file = parsed(&text);
    let script: Vec<Value> = run.script.iter().map(|s| parsed(s)).collect();
    assert_eq!(file["script"].as_array().unwrap(), &script);
    assert_eq!(file["plan"], parsed(PLAN));
    assert_eq!(file["labels"], parsed(LABELS));
    assert_eq!(file["source"], SOURCE);
    let mut keys = 0;
    for s in &script {
        let payload = &s["payload"];
        match s["event"].as_str() {
            Some("test:event") => match payload["kind"].as_str() {
                Some("key") => {
                    assert_eq!(payload["device"], 1);
                    keys += 1;
                }
                kind => panic!("{kind:?}"),
            },
            Some("test:guide") => assert!(payload.get("micros").is_none()),
            event => panic!("{event:?}"),
        }
    }
    assert_eq!(keys, run.fixture.entries.len());
}

// Only KEYTRIAGE_BLESS=1 rewrites the file, so a variable left set to anything else, such as 0,
// can't. CI compares and never rewrites, or a stale file would pass there.
fn bless(value: Option<&str>, ci: bool) -> bool {
    let asked = value == Some("1");
    assert!(
        !(asked && ci),
        "CI only compares {FILE}. Bless it locally and commit the diff."
    );
    asked
}

#[test]
fn bless_needs_exactly_1_and_never_runs_in_ci() {
    assert!(bless(Some("1"), false));
    for value in [None, Some("0"), Some(""), Some("true"), Some("1 ")] {
        assert!(!bless(value, false), "{value:?}");
        assert!(!bless(value, true), "{value:?}");
    }
    let in_ci = std::panic::catch_unwind(|| bless(Some("1"), true));
    assert!(in_ci.is_err());
}

#[test]
fn golden_is_current() {
    let mut run = run();
    let (_, result) = finish(&mut run);
    let text = golden(&run.script, &result);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FILE);
    let value = std::env::var("KEYTRIAGE_BLESS").ok();
    let ci = ["CI", "GITHUB_ACTIONS"]
        .iter()
        .any(|name| std::env::var_os(name).is_some());
    if bless(value.as_deref(), ci) {
        std::fs::write(&path, &text).unwrap();
    }
    let file = std::fs::read_to_string(&path).unwrap_or_default();
    if file != text {
        let same = file.lines().zip(text.lines()).take_while(|(a, b)| a == b);
        panic!(
            "{FILE} differs from this run from line {}. Read the change, then run the Rust tests \
             with KEYTRIAGE_BLESS=1 to rewrite it.",
            same.count() + 1
        );
    }
}
