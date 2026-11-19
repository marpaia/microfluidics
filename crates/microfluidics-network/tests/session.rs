//! The session: what it says to a viewer, and what it does when asked.

// A test stops on the first thing that is not as expected, in its helpers as in its cases.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;

use microfluidics_core::Quantity;
use microfluidics_core::model::LayerSpec;
use microfluidics_layout::{BuildError, Builder, Elaborated, Origin, Overrides, Point};
use microfluidics_network::{Host, serve};

/// One strip on a 10 mm chip, its length a parameter that may be tuned.
fn strip(overrides: &Overrides) -> Result<Elaborated, BuildError> {
    let origin = Origin::new("strip.py", 1);
    let mut design = Builder::design("strip", overrides);
    design.slab([10e-3, 10e-3, 3e-3], &origin)?;
    design.glass([75e-3, 25e-3, 1e-3], &origin)?;
    let flow = LayerSpec {
        name: "flow".to_owned(),
        height: 20e-6,
        gds: 1,
    };
    design.layer(flow, &origin)?;
    let range = Some((Quantity::length(1e-3), Quantity::length(5e-3)));
    let length = design.param("length", Quantity::length(3.3e-3), range, false, &origin)?;
    design.tune("length", &origin)?;
    let (from, to) = (Point::new(1e-3, 1e-3), Point::new(1e-3 + length.si, 1.1e-3));
    design.rectangle("strip", "flow", from, to, &origin)?;
    design.finish()
}

/// The strip as a session holds it, counting how often it is read again.
#[derive(Default)]
struct Held {
    reads: usize,
    /// The fault every build gives, when the design is to fail.
    fault: Option<BuildError>,
}

impl Host for Held {
    fn reload(&mut self) -> Result<(), BuildError> {
        self.reads += 1;
        Ok(())
    }

    fn build(&self, overrides: &Overrides) -> Result<Elaborated, BuildError> {
        match &self.fault {
            Some(fault) => Err(fault.clone()),
            None => strip(overrides),
        }
    }

    fn run(&mut self, command: &str) -> (bool, String) {
        (command != "nonsense", format!("the host ran `{command}`"))
    }

    fn idle(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A directory holding a design's file, and that file.
fn placed() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let file = dir.path().canonicalize().unwrap().join("strip.py");
    fs::write(&file, "").unwrap();
    (dir, file)
}

/// Everything a session on `file` says when it is asked `asked`, in order, and then ends.
fn said(host: &mut Held, file: &Path, asked: &[&str]) -> Vec<Value> {
    let source = Cursor::new(asked.join("\n").into_bytes());
    let mut sink = Vec::new();
    serve(file, host, source, &mut sink).unwrap();
    let text = String::from_utf8(sink).unwrap();
    text.lines().map(|line| serde_json::from_str(line).unwrap()).collect()
}

/// What each message is: its type, and for a status its state.
fn kinds(messages: &[Value]) -> Vec<String> {
    let kind = |message: &Value| match message["state"].as_str() {
        Some(state) => format!("status {state}"),
        None => message["type"].as_str().unwrap().to_owned(),
    };
    messages.iter().map(kind).collect()
}

const BUILT: [&str; 4] = ["status building", "scene", "diagnostics", "status ready"];
const TRIAL: &str = r#"{"type": "set_overrides", "values": {"length": "3.4mm"}}"#;

#[test]
fn a_session_begins_by_building_the_design_it_has_read_once() {
    let (_dir, file) = placed();
    let mut host = Held::default();
    let messages = said(&mut host, &file, &[]);
    assert_eq!(kinds(&messages), BUILT);
    assert_eq!(host.reads, 1);
    let scene = &messages[1];
    assert_eq!(
        (&scene["scene"]["design"], &scene["trial"]),
        (&json!("strip"), &json!(false))
    );
    assert_eq!(messages[3]["passes"], scene["scene"]["passes"]);
}

#[test]
fn trial_values_are_built_with_and_nothing_is_written() {
    let (dir, file) = placed();
    let mut host = Held::default();
    let messages = said(&mut host, &file, &[TRIAL]);
    assert_eq!(kinds(&messages), [BUILT, BUILT].concat());
    assert_eq!(host.reads, 2);
    assert_eq!(
        (&messages[1]["trial"], &messages[5]["trial"]),
        (&json!(false), &json!(true))
    );
    let tuned = |scene: &Value| scene["scene"]["tunables"][0]["value"].as_f64().unwrap();
    assert!((tuned(&messages[1]) - 3300.0).abs() < 1e-6 && (tuned(&messages[5]) - 3400.0).abs() < 1e-6);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn kept_values_are_written_beside_the_design_and_are_no_longer_a_trial() {
    let (_dir, file) = placed();
    let mut host = Held::default();
    let messages = said(&mut host, &file, &[TRIAL, r#"{"type": "keep"}"#]);
    assert_eq!(kinds(&messages), [BUILT, BUILT, BUILT].concat());
    assert_eq!(messages[9]["trial"], json!(false));
    assert!((messages[9]["scene"]["tunables"][0]["value"].as_f64().unwrap() - 3400.0).abs() < 1e-6);
    let kept = fs::read_to_string(file.with_extension("tuned.toml")).unwrap();
    let kept = Overrides::parse("kept", &kept).unwrap();
    assert_eq!(kept.0.len(), 1);
    assert!((kept.0["length"].si - 3.4e-3).abs() < 1e-12);
}

#[test]
fn a_build_that_fails_says_where_and_the_session_goes_on() {
    let (_dir, file) = placed();
    let place = file.display().to_string();
    let mut host = Held {
        fault: Some(BuildError::at(&Origin::new(&place, 7), "the pad does not fit")),
        ..Held::default()
    };
    let messages = said(&mut host, &file, &[r#"{"type": "run", "command": "nonsense"}"#]);
    assert_eq!(
        kinds(&messages),
        ["status building", "status failed", "diagnostics", "result"]
    );
    let failed = &messages[1];
    assert_eq!(
        (&failed["message"], &failed["file"], &failed["line"]),
        (&json!("the pad does not fit"), &json!(place), &json!(7))
    );
    let found = &messages[2]["files"][&place];
    assert_eq!(found.as_array().unwrap().len(), 1);
    assert_eq!(
        (
            &found[0]["line"],
            &found[0]["severity"],
            &found[0]["check"],
            &found[0]["group"]
        ),
        (&json!(7), &json!("error"), &json!("build"), &json!("integrity"))
    );
    assert_eq!(messages[3]["ok"], json!(false));
}

#[test]
fn what_is_not_a_message_is_refused_and_what_was_found_stays() {
    let (_dir, file) = placed();
    let mut host = Held::default();
    let messages = said(&mut host, &file, &[r#"{"type": "bogus"}"#, "not a message"]);
    assert_eq!(
        kinds(&messages),
        [&BUILT[..], &["status failed", "status failed"]].concat()
    );
    assert_eq!(
        messages[4]["message"],
        json!("`bogus` is not something a session is asked")
    );
    assert!(messages[5]["message"].as_str().unwrap().starts_with("not a message: "));
    assert_eq!(host.reads, 1);
}

#[test]
fn a_command_is_answered_by_the_host_and_closing_builds_the_design_again() {
    let (_dir, file) = placed();
    let mut host = Held::default();
    let run = |command: &str| json!({ "type": "run", "command": command }).to_string();
    let asked = [TRIAL.to_owned(), run("check"), run("nonsense"), run("close")];
    let asked: Vec<&str> = asked.iter().map(String::as_str).collect();
    let messages = said(&mut host, &file, &asked);
    assert_eq!(
        kinds(&messages),
        [&BUILT[..], &BUILT[..], &["result", "result", "result"], &BUILT[..]].concat()
    );
    assert_eq!(
        (&messages[8]["command"], &messages[8]["ok"], &messages[8]["text"]),
        (&json!("check"), &json!(true), &json!("the host ran `check`"))
    );
    assert_eq!(messages[9]["ok"], json!(false));
    // What closing tuned is beside the design, so the trial values give way to it.
    assert_eq!(
        (&messages[5]["trial"], &messages[12]["trial"]),
        (&json!(true), &json!(false))
    );
}
