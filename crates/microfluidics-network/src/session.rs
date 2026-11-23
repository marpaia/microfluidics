//! A session: a design held for a viewer, and built again as it changes.
//!
//! A session speaks JSON, a message to a line. It says `status` (building, ready, or failed
//! with where), `scene`, `diagnostics` (each finding at the line of the design that made what
//! it is about) and `result` (what a command gave). It is asked `rebuild`, `set_overrides`
//! (trial values for tunables; nothing is written), `keep` (the trial values are written beside
//! the design), `run` (a command by name) and `shutdown`.
//!
//! The session knows nothing of how a design is written. Its [`Host`] reads the design again,
//! builds it, and runs commands on it. The files beside the design whose writing builds it again
//! are those a design is made of: Python and TOML.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{self, Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use serde_json::{Value, json};

use microfluidics_core::Status;
use microfluidics_layout::{BuildError, Elaborated, Origin, Overrides};

use crate::project::{Project, ProjectError};
use crate::scene::{Scene, scene};

/// How often the files beside the design are looked at for a change.
const POLL: Duration = Duration::from_millis(400);

/// The extensions of the files beside the design whose writing builds it again.
const WATCHED: [&str; 2] = ["py", "toml"];

/// What holds a design for a session: how it is read, built and asked about.
pub trait Host {
    /// Read the design's source again, before it is built. A fault here is a build that failed.
    fn reload(&mut self) -> Result<(), BuildError>;

    /// Build the design with `overrides` replacing parameters by path.
    fn build(&self, overrides: &Overrides) -> Result<Elaborated, BuildError>;

    /// Run a command on the design: whether it passed, and what it said.
    fn run(&mut self, command: &str) -> (bool, String);

    /// Called whenever the session has waited and nothing was asked.
    fn idle(&mut self) -> io::Result<()>;
}

fn send(sink: &mut dyn Write, value: Value) -> io::Result<()> {
    serde_json::to_writer(&mut *sink, &value)?;
    writeln!(sink)?;
    sink.flush()
}

/// A file that is there, as its path from the root of the file system with links followed; a
/// file that is not, as it was given.
fn found(file: &Path) -> PathBuf {
    file.canonicalize().unwrap_or_else(|_| file.to_owned())
}

fn locate(origin: &mut Origin) {
    origin.file = found(Path::new(&origin.file)).display().to_string();
}

/// Each finding of a scene, by the file and line of the design that made what it is about. A
/// finding with no place of its own, and a check that did not run, are put at the top of the
/// design.
fn diagnostics(scene: &Scene, file: &Path) -> Value {
    let mut files: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for finding in &scene.findings {
        let origin = finding
            .origin
            .clone()
            .unwrap_or_else(|| Origin::new(&file.display().to_string(), 1));
        let severity = if finding.waived.is_some() {
            "information"
        } else {
            "error"
        };
        files.entry(origin.file).or_default().push(json!({
            "line": origin.line.max(1), "severity": severity, "message": finding.message,
            "check": finding.check, "group": finding.group, "rule": finding.rule, "paths": finding.paths,
        }));
    }
    for check in &scene.checks {
        if check.status == Status::NotRun {
            files.entry(file.display().to_string()).or_default().push(json!({
                "line": 1, "severity": "warning",
                "message": format!("{}: {}", check.id, check.note.as_deref().unwrap_or("did not run")),
                "check": check.id, "group": check.group, "rule": check.id, "paths": [],
            }));
        }
    }
    json!({ "type": "diagnostics", "files": files })
}

/// Trial values as a viewer sends them: each tunable's path, and its value written with its unit.
fn overrides(values: &Value) -> Result<Overrides, String> {
    let values = values.as_object().ok_or("trial values must be an object")?;
    let mut text = String::new();
    for (path, value) in values {
        let value = value.as_str().ok_or("a trial value must be a string with its unit")?;
        text.push_str(&format!("{} = {}\n", json!(path), json!(value)));
    }
    Overrides::parse("trial values", &text)
}

/// One design, the trial values a viewer has set, and the files beside it as last seen.
struct Session<'h> {
    file: PathBuf,
    host: &'h mut dyn Host,
    trial: Overrides,
    /// When each watched file was last written, as the last build found them.
    seen: BTreeMap<PathBuf, SystemTime>,
}

impl Session<'_> {
    /// When each watched file beside the design was last written.
    fn watched(&self) -> BTreeMap<PathBuf, SystemTime> {
        let mut found = BTreeMap::new();
        if let Some(directory) = self.file.parent().and_then(|dir| fs::read_dir(dir).ok()) {
            for entry in directory.flatten() {
                let path = entry.path();
                let watched = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| WATCHED.contains(&extension));
                if watched && let Ok(written) = fs::metadata(&path).and_then(|metadata| metadata.modified()) {
                    found.insert(path, written);
                }
            }
        }
        found
    }

    /// A fault as its message, and the file and line it is shown at: its own, or the top of the
    /// design.
    fn located(&self, error: &ProjectError) -> (String, String, u32) {
        match &error.origin {
            Some(origin) => (
                error.message.clone(),
                found(Path::new(&origin.file)).display().to_string(),
                origin.line.max(1),
            ),
            None => (error.message.clone(), self.file.display().to_string(), 1),
        }
    }

    /// The design read again, built with the trial values over what is kept beside it, and
    /// verified: its scene, with every place in its source as a path from the root.
    fn drawn(&mut self) -> Result<Scene, ProjectError> {
        self.host.reload()?;
        let host = &*self.host;
        let build = |overrides: &Overrides| host.build(overrides);
        let mut project = Project::beside(&self.file, &build)?;
        project.overrides.0.extend(self.trial.0.clone());
        let verified = project.verify()?;
        let mut scene = scene(&verified);
        for origin in scene.origins.values_mut() {
            locate(origin);
        }
        for origin in scene.findings.iter_mut().filter_map(|finding| finding.origin.as_mut()) {
            locate(origin);
        }
        for origin in scene
            .instances
            .iter_mut()
            .filter_map(|instance| instance.origin.as_mut())
        {
            locate(origin);
        }
        Ok(scene)
    }

    /// Build the scene and say what was found. A fault is said, and what was shown stays.
    fn build(&mut self, sink: &mut dyn Write) -> io::Result<()> {
        self.seen = self.watched();
        send(sink, json!({ "type": "status", "state": "building" }))?;
        let started = Instant::now();
        match self.drawn() {
            Ok(scene) => {
                send(
                    sink,
                    json!({ "type": "scene", "scene": scene, "trial": !self.trial.0.is_empty() }),
                )?;
                send(sink, diagnostics(&scene, &self.file))?;
                send(
                    sink,
                    json!({
                        "type": "status", "state": "ready", "passes": scene.passes,
                        "problems": scene.problems, "build_ms": started.elapsed().as_millis(),
                    }),
                )
            }
            Err(error) => {
                let (message, file, line) = self.located(&error);
                send(
                    sink,
                    json!({ "type": "status", "state": "failed", "message": message, "file": file, "line": line }),
                )?;
                send(
                    sink,
                    json!({ "type": "diagnostics", "files": { file: [{
                        "line": line, "severity": "error", "message": message,
                        "check": "build", "group": "integrity", "rule": "build", "paths": [],
                    }]}}),
                )
            }
        }
    }

    /// Say that something asked of the session is not something it does. What was shown, and
    /// what was found in the design, stay.
    fn refused(&self, sink: &mut dyn Write, message: &str) -> io::Result<()> {
        send(
            sink,
            json!({ "type": "status", "state": "failed", "message": message, "file": self.file, "line": 1 }),
        )
    }

    /// Run a command on the design and say what it gave.
    fn run(&mut self, command: &str, sink: &mut dyn Write) -> io::Result<()> {
        let (ok, text) = self.host.run(command);
        send(
            sink,
            json!({ "type": "result", "command": command, "ok": ok, "text": text }),
        )?;
        if command == "close" {
            // What was tuned is now beside the design; trial values give way to it.
            self.trial = Overrides::default();
            self.build(sink)?;
        }
        Ok(())
    }

    /// Write the trial values beside the design, and show the design as it is now kept.
    fn keep(&mut self, sink: &mut dyn Write) -> io::Result<()> {
        if !self.trial.0.is_empty() {
            let host = &*self.host;
            let build = |overrides: &Overrides| host.build(overrides);
            let kept = Project::beside(&self.file, &build).and_then(|mut project| project.keep(&self.trial));
            if let Err(error) = kept {
                let (message, file, line) = self.located(&error);
                let text = format!("{file}:{line}: {message}");
                return send(
                    sink,
                    json!({ "type": "result", "command": "keep", "ok": false, "text": text }),
                );
            }
            self.trial = Overrides::default();
        }
        self.build(sink)
    }

    /// Do what a viewer asks; `false` when it asks the session to end.
    fn handle(&mut self, message: &Value, sink: &mut dyn Write) -> io::Result<bool> {
        match message["type"].as_str().unwrap_or("") {
            "shutdown" => return Ok(false),
            "rebuild" => self.build(sink)?,
            "set_overrides" => match overrides(&message["values"]) {
                Ok(trial) => {
                    self.trial = trial;
                    self.build(sink)?;
                }
                Err(error) => self.refused(sink, &error)?,
            },
            "keep" => self.keep(sink)?,
            "run" => self.run(message["command"].as_str().unwrap_or(""), sink)?,
            kind => self.refused(sink, &format!("`{kind}` is not something a session is asked"))?,
        }
        Ok(true)
    }
}

/// Hold the design in `file` for a viewer until `source` ends or it asks the session to end.
/// What a viewer asks comes from `source`, a message to a line; what the session finds goes to
/// `sink`.
pub fn serve(
    file: &Path,
    host: &mut dyn Host,
    source: impl BufRead + Send + 'static,
    sink: &mut dyn Write,
) -> io::Result<()> {
    let mut session = Session {
        file: file.canonicalize().or_else(|_| path::absolute(file))?,
        host,
        trial: Overrides::default(),
        seen: BTreeMap::new(),
    };
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for line in source.lines() {
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    session.build(sink)?;
    loop {
        match receiver.recv_timeout(POLL) {
            Ok(line) => {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<Value>(&line) {
                    Ok(message) => {
                        if !session.handle(&message, sink)? {
                            return Ok(());
                        }
                    }
                    Err(error) => session.refused(sink, &format!("not a message: {error}"))?,
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                session.host.idle()?;
                // Nothing was asked: build again if a file was written.
                if session.watched() != session.seen {
                    session.build(sink)?;
                }
            }
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}
