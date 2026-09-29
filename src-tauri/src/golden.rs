// The Done-when runs, from capture to report. The engine's synthetic streams go through Core as the
// capture callback feeds them, and every payload the page gets, in order, is pinned: the chatter
// run in src/app/testing/guided-chatter.json, and the swap test that follows it in
// src/app/testing/swap.json. The page's own tests replay both. A change to the engine's words, a
// payload's shape or the Guide shows up as a diff of a file, which KEYTRIAGE_BLESS=1 rewrites once
// the change has been read.
use std::path::PathBuf;
use std::time::Instant;

use keytriage_diagnostics::fixture::{Fixture, guided_chatter, swap_chatter};
use keytriage_diagnostics::params::{END_WAIT_US, SWAP_PRESSES, SWAP_ROUNDS};
use keytriage_diagnostics::{
    BoardKind, Confidence, Entry, Evidence, Kind, NextTest, Partner, Swap,
};
use keytriage_input::Keyboard;
use serde::Serialize;
use serde_json::{Value, json};

use crate::export::{no_sequence, report_json};
use crate::session_core::{Core, Ended, at, input, rounds};
use crate::view::{self, KeyName, KeyboardGroup, PlanArgs, TestResult, groups, names};

const E: u16 = 0x12;
const G: u16 = 0x22;
const J: u16 = 0x24;

const FILE: &str = "../src/app/testing/guided-chatter.json";
const SOURCE: &str = "synthetic: crates/diagnostics fixture::guided_chatter";
const SWAP_FILE: &str = "../src/app/testing/swap.json";
const SWAP_SOURCE: &str =
    "synthetic: crates/diagnostics fixture::swap_chatter after fixture::guided_chatter";

// What the page sends: G, J and E on the Start screen's default board, and their drawn names.
const PLAN: &str =
    r#"{"keyboard":[1],"keys":[34,36,18],"rounds":3,"presses":10,"board":"hot-swap"}"#;
const LABELS: &str = r#"[{"scan":34,"name":"G"},{"scan":36,"name":"J"},{"scan":18,"name":"E"}]"#;

// The synthetic keyboard as Windows would list it. The page gets its groups, and the path stays in
// Rust, where the swap test finds the keyboard again by it.
fn listed() -> Vec<Keyboard> {
    vec![Keyboard {
        handle: 1,
        path: r"\\?\HID#VID_0000&PID_0001#synthetic".to_string(),
        vendor_id: Some(0),
        product_id: Some(1),
        manufacturer: None,
        product: Some("Synthetic keyboard".to_string()),
        container: None,
    }]
}

fn keyboards() -> Vec<KeyboardGroup> {
    groups(&listed())
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

// Every payload a started test emits, in order: its first view, then each event and any view the
// event changed.
fn feed(core: &mut Core, start: Instant, entries: &[Entry]) -> Vec<String> {
    let mut script = vec![step("test:guide", core.view().unwrap())];
    for e in entries {
        let (entry, view) = core.input(input(start, e));
        script.push(step("test:event", entry));
        if let Some(view) = view {
            script.push(step("test:guide", view));
        }
    }
    script
}

// Core as start_test builds it from the page's plan, fed the stream as the capture callback is.
fn run() -> Run {
    let (engine_plan, fixture) = guided_chatter();
    let args: PlanArgs = serde_json::from_str(PLAN).unwrap();
    let test = view::plan(args).unwrap();
    assert_eq!(test.0.plan(), &engine_plan);
    let start = Instant::now();
    let mut core = Core::new(start, Some(test));
    let script = feed(&mut core, start, &fixture.entries);
    Run {
        fixture,
        script,
        core,
        start,
    }
}

// As end_test ends it.
fn finish(run: &mut Run) -> Ended {
    let labels: Vec<KeyName> = serde_json::from_str(LABELS).unwrap();
    let names = names(labels).unwrap();
    run.core
        .end(at(run.start, run.fixture.end_us), &names)
        .unwrap()
}

// A swap test after the main run, before it ends.
struct Retest {
    offer: Swap,
    handles: Vec<isize>,
    run: Run,
}

// As start_swap_test builds it: the offer the main run left, the keyboard found again by the paths
// start_test kept, and the engine's plan for that offer. The `faulty` keys chatter.
fn swap_run(faulty: &[u16]) -> Retest {
    let offer = finish(&mut run()).offer.expect("a swap offer");
    let args: PlanArgs = serde_json::from_str(PLAN).unwrap();
    let paths = view::still_listed(&args.keyboard, &listed()).unwrap();
    let handles = view::refind(&paths, &listed()).unwrap();
    let test = view::swap_plan(&offer, handles.clone()).unwrap();
    let fixture = swap_chatter(offer.plan(), faulty);
    assert_eq!(test.0.plan(), &offer.plan());
    assert_eq!((&test.1, test.2), (&fixture.keyboard, fixture.board));
    let start = Instant::now();
    let mut core = Core::swap(start, test, offer);
    let script = feed(&mut core, start, &fixture.entries);
    Retest {
        offer,
        handles,
        run: Run {
            fixture,
            script,
            core,
            start,
        },
    }
}

// The swap test's two Done-when runs, each ended.
fn judged() -> [(&'static str, Retest, Ended); 2] {
    [("follows", G), ("stays", E)].map(|(name, faulty)| {
        let mut retest = swap_run(&[faulty]);
        let ended = finish(&mut retest.run);
        (name, retest, ended)
    })
}

// Pretty JSON whose lines after the first sit `depth` levels in.
fn indented(value: &impl Serialize, depth: usize) -> String {
    serde_json::to_string_pretty(value)
        .unwrap()
        .replace('\n', &format!("\n{}", "  ".repeat(depth)))
}

fn object(fields: &[String], depth: usize) -> String {
    let pad = "  ".repeat(depth);
    format!(
        "{{\n{pad}  {}\n{pad}}}",
        fields.join(&format!(",\n{pad}  "))
    )
}

// One line per emitted event, so a re-blessed file's diff reads event by event.
fn lines(script: &[String], depth: usize) -> String {
    let pad = "  ".repeat(depth);
    format!("[\n{pad}  {}\n{pad}]", script.join(&format!(",\n{pad}  ")))
}

fn golden(script: &[String], result: &TestResult) -> String {
    let fields = [
        format!("\"source\": \"{SOURCE}\""),
        format!("\"plan\": {PLAN}"),
        format!("\"keyboards\": {}", indented(&keyboards(), 1)),
        format!("\"labels\": {LABELS}"),
        format!("\"script\": {}", lines(script, 1)),
        format!("\"result\": {}", indented(result, 1)),
    ];
    object(&fields, 0) + "\n"
}

// The page replays each run after guided-chatter.json's, and start_swap_test answers `handles`.
fn swap_golden(handles: &[isize], runs: &[(&str, &[String], &TestResult)]) -> String {
    let runs: Vec<String> = runs
        .iter()
        .map(|(name, script, result)| {
            let fields = [
                format!("\"script\": {}", lines(script, 3)),
                format!("\"result\": {}", indented(result, 3)),
            ];
            format!("\"{name}\": {}", object(&fields, 2))
        })
        .collect();
    let fields = [
        format!("\"source\": \"{SWAP_SOURCE}\""),
        format!("\"handles\": {}", json!(handles)),
        format!("\"runs\": {}", object(&runs, 1)),
    ];
    object(&fields, 0) + "\n"
}

fn swap_file() -> String {
    let runs = judged();
    let handles = &runs[0].1.handles;
    let files: Vec<(&str, &[String], &TestResult)> = runs
        .iter()
        .map(|(name, retest, ended)| (*name, &retest.run.script[..], &ended.result))
        .collect();
    swap_golden(handles, &files)
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
    let Ended {
        report,
        result,
        offer,
    } = finish(&mut run);

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
                partner: Partner::Clean(G)
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

    // Rust keeps the offer for the retest, and the page gets only its words.
    assert_eq!(
        offer,
        Some(Swap {
            suspect: E,
            partner: G,
            kind: Kind::Chatter,
            before: Confidence::VeryHigh,
            floor_permille: 95,
            partner_untested: false,
        })
    );
    let swap = result.swap.as_ref().expect("a swap offer");
    assert_eq!((swap.suspect, swap.partner), (E, G));
    assert!(
        swap.title
            .starts_with("Move the E switch into the G socket"),
        "{}",
        swap.title
    );
    assert_eq!(swap.steps.len(), 4);
    assert_eq!(swap.note, "Swap test. Both keys, 3 rounds.");
    assert!(result.outcome.is_none());

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
fn m4_done_swap_follows_and_stays() {
    for (faulty, outcome, title, diagnosis, keys) in [
        (
            G,
            "follows",
            "The fault moved with the switch.",
            "The E switch now sits in the G socket, and the fault appeared there.",
            [(E, 90), (G, 108)],
        ),
        (
            E,
            "stays",
            "The fault stayed on E.",
            "A known-good switch in the E socket shows the same fault.",
            [(E, 108), (G, 90)],
        ),
    ] {
        // Taking the rounds ends the Guide, so the judgment comes from a second, identical run.
        let stamped = {
            let mut once = swap_run(&[faulty]).run;
            rounds(&mut once.core, once.fixture.end_us)
        };
        let Retest {
            offer,
            handles,
            mut run,
        } = swap_run(&[faulty]);
        assert_eq!((offer.suspect, offer.partner), (E, G));
        assert_eq!(handles, [1]);
        // The suspect first, then its known-good partner, in every round.
        assert_eq!(stamped, run.fixture.rounds);
        assert!(stamped.iter().all(|r| r.asked == SWAP_PRESSES));
        assert_eq!(
            stamped.iter().map(|r| r.key).collect::<Vec<u16>>(),
            [E, G].repeat(usize::from(SWAP_ROUNDS))
        );
        let script: Vec<Value> = run.script.iter().map(|s| parsed(s)).collect();
        let views: Vec<&Value> = script
            .iter()
            .filter(|s| s["event"] == "test:guide")
            .map(|s| &s["payload"])
            .collect();
        let (first, last) = (views[0], views[views.len() - 1]);
        assert_eq!(
            (
                &first["key"],
                &first["asked"],
                &first["rounds"],
                &first["keys"]
            ),
            (&json!(E), &json!(30), &json!(3), &json!(2))
        );
        assert!(last["key"].is_null());
        assert_eq!((&last["done"], &last["total"]), (&json!(180), &json!(180)));
        assert_eq!(last["waitMs"], json!(END_WAIT_US / 1000));

        let Ended {
            report,
            result,
            offer: kept,
        } = finish(&mut run);
        // The app's retest is the engine's own run of the offer's plan.
        assert_eq!(report, swap_chatter(offer.plan(), &[faulty]).diagnose());
        assert!(kept.is_none() && result.swap.is_none(), "{outcome}");
        assert!(result.findings.is_empty() && result.notes.is_empty() && result.clean.is_empty());
        let o = result.outcome.as_ref().expect("a judgment");
        assert_eq!(
            (
                o.outcome,
                o.tile,
                o.flagged.as_slice(),
                o.title.as_str(),
                o.confidence,
                o.level.as_deref(),
                o.strong
            ),
            (
                outcome,
                Some(faulty),
                &[faulty][..],
                title,
                Some("very-high"),
                Some("Very high"),
                true
            )
        );
        assert!(o.diagnosis.starts_with(diagnosis), "{}", o.diagnosis);
        let counts: Vec<(u16, u32)> = result.keys.iter().map(|k| (k.scan, k.count)).collect();
        assert_eq!(counts, keys);

        // The file Export writes after the swap is the retest's own saved report.
        let export = report_json(&report.saved());
        assert_eq!(no_sequence(&export), Ok(()));
        let file = parsed(&export);
        let positions: Vec<&String> = file["keys"].as_object().unwrap().keys().collect();
        assert_eq!(positions, ["0012", "0022"]);
    }
}

#[test]
fn golden_holds_only_the_synthetic_stream() {
    let mut run = run();
    let Ended { result, .. } = finish(&mut run);
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

#[test]
fn swap_golden_holds_only_the_synthetic_stream() {
    let runs = judged();
    let text = swap_file();
    let file = parsed(&text);
    assert_eq!(file["source"], SWAP_SOURCE);
    assert_eq!(file["handles"], json!([1]));
    assert_eq!(file["runs"].as_object().unwrap().len(), 2);
    for (name, retest, ended) in &runs {
        let run = &file["runs"][name];
        let script: Vec<Value> = retest.run.script.iter().map(|s| parsed(s)).collect();
        assert_eq!(run["script"].as_array().unwrap(), &script);
        assert_eq!(run["result"], serde_json::to_value(&ended.result).unwrap());
        let mut keys = 0;
        for s in &script {
            let payload = &s["payload"];
            match s["event"].as_str() {
                Some("test:event") => match payload["kind"].as_str() {
                    Some("key") => {
                        assert_eq!(payload["device"], 1);
                        assert!([json!(E), json!(G)].contains(&payload["scan"]), "{payload}");
                        keys += 1;
                    }
                    kind => panic!("{kind:?}"),
                },
                Some("test:guide") => assert!(payload.get("micros").is_none()),
                event => panic!("{event:?}"),
            }
        }
        assert_eq!(keys, retest.run.fixture.entries.len());
    }
    // The path the retest found the keyboard by stays in Rust. The first check shows the needle is
    // in it.
    assert!(listed()[0].path.contains("HID#"));
    assert!(!text.contains("HID#"));
}

// Only KEYTRIAGE_BLESS=1 rewrites a file, so a variable left set to anything else, such as 0,
// can't. CI compares and never rewrites, or a stale file would pass there.
fn bless(file: &str, value: Option<&str>, ci: bool) -> bool {
    let asked = value == Some("1");
    assert!(
        !(asked && ci),
        "CI only compares {file}. Bless it locally and commit the diff."
    );
    asked
}

#[test]
fn bless_needs_exactly_1_and_never_runs_in_ci() {
    assert!(bless(FILE, Some("1"), false));
    for value in [None, Some("0"), Some(""), Some("true"), Some("1 ")] {
        assert!(!bless(FILE, value, false), "{value:?}");
        assert!(!bless(FILE, value, true), "{value:?}");
    }
    let in_ci = std::panic::catch_unwind(|| bless(SWAP_FILE, Some("1"), true));
    let message = in_ci.unwrap_err().downcast::<String>().unwrap();
    assert!(message.contains(SWAP_FILE), "{message}");
}

// Compares `text` with the golden file, after rewriting the file when asked to bless it.
fn current(file: &str, text: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file);
    let value = std::env::var("KEYTRIAGE_BLESS").ok();
    let ci = ["CI", "GITHUB_ACTIONS"]
        .iter()
        .any(|name| std::env::var_os(name).is_some());
    if bless(file, value.as_deref(), ci) {
        std::fs::write(&path, text).unwrap();
    }
    let found = std::fs::read_to_string(&path).unwrap_or_default();
    if found != text {
        let same = found.lines().zip(text.lines()).take_while(|(a, b)| a == b);
        panic!(
            "{file} differs from this run from line {}. Read the change, then run the Rust tests \
             with KEYTRIAGE_BLESS=1 to rewrite it.",
            same.count() + 1
        );
    }
}

#[test]
fn golden_is_current() {
    let mut run = run();
    let Ended { result, .. } = finish(&mut run);
    current(FILE, &golden(&run.script, &result));
}

#[test]
fn swap_golden_is_current() {
    current(SWAP_FILE, &swap_file());
}

// README.md says its sample is the finding the app shows for this run, so each of the finding's
// lines must appear in it. The sample wraps long lines, so a line break reads as a space.
#[test]
fn readme_quotes_the_golden_finding() {
    let mut run = run();
    let Ended { result, .. } = finish(&mut run);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../README.md");
    let readme = std::fs::read_to_string(path).unwrap();
    let sample = readme.split("```text").nth(1).unwrap();
    let words = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let sample = words(sample.split("```").next().unwrap());
    let finding = &result.findings[0];
    let lines = [&finding.title]
        .into_iter()
        .chain(&finding.evidence)
        .chain(&finding.causes)
        .chain(&finding.next);
    for line in lines {
        assert!(
            sample.contains(&words(line)),
            "README.md's sample lacks: {line}"
        );
    }
}
