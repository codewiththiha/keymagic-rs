//! Tester page: type with the active keyboard and see what it produces.

use super::Notice;
use crate::model::{TesterText, TypedKey};
use crate::state::{Host, lock};
use crate::widgets;
use egui::{Align, Context, Event, Key, Layout, RichText, Ui};
use keymagic_engine::{Action, Engine, KeyEvent, Modifiers, TraceStep};
use keymagic_host::Session;
use keymagic_types::SourceSpan;
use std::sync::{Arc, Mutex};

/// View state of the Tester page: its session and typed text.
#[derive(Default)]
pub struct View {
    compiles: u64,
    keyboard: Option<String>,
    session: Option<Session>,
    probe: Option<Engine>,
    source: String,
    spans: Vec<Option<SourceSpan>>,
    last: Option<Probe>,
    typed: String,
    text: TesterText,
    notice: Notice,
}

impl View {
    /// The committed text, for tests and status display.
    #[must_use]
    pub const fn text(&self) -> &TesterText {
        &self.text
    }

    /// Draws the output area and sends typed keys to the session.
    pub fn show(&mut self, ui: &mut Ui, host: &Mutex<Host>) {
        self.sync_session(host);
        self.capture(ui, host);
        ui.horizontal(|ui| {
            let status = self.keyboard.as_ref().map_or_else(
                || "No keyboard is active. Choose one on the Keyboards page.".to_owned(),
                |id| format!("Keyboard: {id}"),
            );
            ui.label(widgets::muted(ui, &status));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::button(ui, "Clear").clicked() {
                    self.clear();
                }
            });
        });
        self.notice.show(ui);
        ui.add_space(8.0);
        let mut record_clicked = false;
        ui.columns(2, |columns| {
            let [typing, pane] = columns else {
                return;
            };
            widgets::card(typing, |ui| {
                ui.set_min_size(egui::vec2(ui.available_width(), 240.0));
                ui.label(RichText::new(self.text.committed()).size(28.0));
                ui.label(widgets::muted(ui, self.text.preedit()).size(28.0));
            });
            widgets::card(pane, |ui| {
                record_clicked = self.details(ui);
            });
        });
        if record_clicked {
            self.record_case(host);
        }
        ui.add_space(8.0);
        ui.label(widgets::muted(
            ui,
            "Click the page and type. Keys go to the active keyboard.",
        ));
    }

    /// Reloads the session when the active keyboard changes.
    fn sync_session(&mut self, host: &Mutex<Host>) {
        let (active, compiles) = {
            let guard = lock(host);
            (guard.config().keyboards.active.clone(), guard.compiles())
        };
        if active == self.keyboard && compiles == self.compiles {
            return;
        }
        self.compiles = compiles;
        self.keyboard.clone_from(&active);
        self.session = None;
        self.probe = None;
        self.last = None;
        self.typed.clear();
        self.text.clear();
        let Some(id) = active else {
            return;
        };
        let opened = open_session(&lock(host), &id);
        match opened {
            Ok(opened) => {
                self.session = Some(opened.session);
                self.probe = Some(opened.probe);
                self.source = opened.source;
                self.spans = opened.spans;
            }
            Err(error) => self.notice.set(format!("Could not load {id}: {error}")),
        }
    }

    fn capture(&mut self, ui: &Ui, host: &Mutex<Host>) {
        if ui.ctx().memory(|memory| memory.focused().is_some()) {
            return;
        }
        for key in typed_keys(ui.ctx()) {
            let Some(session) = self.session.as_mut() else {
                return;
            };
            let event = key.event();
            let actions = session.key(event);
            if let Some(probe) = self.probe.as_mut() {
                self.last = Some(probe_key(probe, event, &self.source, &self.spans));
            }
            match key {
                TypedKey::Char(ch) => self.typed.push(ch),
                TypedKey::Backspace => {
                    self.typed.pop();
                }
                TypedKey::Enter => {}
            }
            if let Some(id) = self.text.apply(&actions, key.fallback()) {
                self.switch(host, &id);
            }
        }
    }

    fn clear(&mut self) {
        self.text.clear();
        self.typed.clear();
        self.last = None;
        if let Some(probe) = self.probe.as_mut() {
            probe.reset();
        }
        if let Some(session) = self.session.as_mut() {
            session.sync("");
        }
    }

    /// Draws the diagnostics pane; returns true when Record case is clicked.
    fn details(&self, ui: &mut Ui) -> bool {
        let probe = self.last.as_ref();
        detail(ui, "Key", probe.map_or("—", |p| p.key.as_str()));
        detail(
            ui,
            "Handled",
            probe.map_or("—", |p| {
                if p.handled {
                    "Yes"
                } else {
                    "No, passed through"
                }
            }),
        );
        detail(ui, "Output", probe.map_or("—", |p| p.output.as_str()));
        detail(ui, "Composing", probe.map_or("—", |p| p.composing.as_str()));
        detail(ui, "Modifiers", probe.map_or("—", |p| p.modifiers.as_str()));
        let captures = probe.map_or_else(|| "—".to_owned(), |p| capture_list(&p.captures));
        detail(ui, "Captures", &captures);
        let rule = probe.and_then(|p| p.rule.as_ref());
        detail(
            ui,
            "Rule",
            &rule.map_or_else(|| "No rule matched".to_owned(), rule_label),
        );
        let source_line = rule.map_or_else(
            || "—".to_owned(),
            |hit| {
                hit.text
                    .clone()
                    .unwrap_or_else(|| "No source line".to_owned())
            },
        );
        detail(ui, "Source", &source_line);
        ui.add_space(8.0);
        widgets::button(ui, "Record case").clicked()
    }

    fn record_case(&mut self, host: &Mutex<Host>) {
        let Some(id) = self.keyboard.clone() else {
            return;
        };
        let expected = format!("{}{}", self.text.committed(), self.text.preedit());
        let result = lock(host).record_case(&id, &self.typed, &expected);
        self.notice.set(match result {
            Ok(path) => format!("Recorded a case in {}.", path.display()),
            Err(error) => format!("Could not record the case: {error}"),
        });
    }

    fn switch(&mut self, host: &Mutex<Host>, id: &str) {
        let result = lock(host).switch_keyboard(id);
        if let Err(error) = result {
            self.notice.set(format!(
                "Switched to {id}, but settings were not saved: {error}"
            ));
        }
    }
}

/// Engines, source and rule spans for one keyboard.
struct Opened {
    session: Session,
    probe: Engine,
    source: String,
    spans: Vec<Option<SourceSpan>>,
}

/// What the last key did on the active keyboard.
struct Probe {
    key: String,
    handled: bool,
    output: String,
    composing: String,
    rule: Option<RuleHit>,
    captures: Vec<String>,
    modifiers: String,
}

/// The primary rule behind the last output.
struct RuleHit {
    rank: usize,
    line: Option<usize>,
    text: Option<String>,
}

/// The rank of the rule and, when known, its source line.
fn rule_label(hit: &RuleHit) -> String {
    hit.line.map_or_else(
        || format!("#{}", hit.rank),
        |line| format!("#{} at line {line}", hit.rank),
    )
}

fn open_session(host: &Host, id: &str) -> Result<Opened, String> {
    let keyboard = host.load_keyboard(id).map_err(|error| error.to_string())?;
    let spans = keyboard.rules.iter().map(|rule| rule.source).collect();
    let keyboard = Arc::new(keyboard);
    let engine = Engine::new(Arc::clone(&keyboard)).map_err(|error| error.message.to_owned())?;
    let probe = Engine::new(keyboard).map_err(|error| error.message.to_owned())?;
    let mut session = Session::new(engine, host.config());
    session.focus_in("KeyMagic Tester");
    let source = host.source(id).ok().flatten().unwrap_or_default();
    Ok(Opened {
        session,
        probe,
        source,
        spans,
    })
}

fn probe_key(
    engine: &mut Engine,
    event: KeyEvent,
    source: &str,
    spans: &[Option<SourceSpan>],
) -> Probe {
    let output = engine.process(event);
    let applied = output.trace.as_ref().and_then(|trace| {
        trace.steps.iter().find_map(|step| match step {
            TraceStep::Applied {
                rule,
                rank,
                recursive: false,
                captures,
                ..
            } => Some((*rule, *rank, captures.clone())),
            _ => None,
        })
    });
    let captures = applied
        .as_ref()
        .map_or_else(Vec::new, |(_, _, captured)| captured.clone());
    let rule = applied.map(|(index, rank, _)| {
        let span = spans.get(index).copied().flatten();
        let text = span.map(|span| {
            source
                .get(span.start..span.end)
                .unwrap_or_default()
                .trim()
                .to_owned()
        });
        let line = span.map(|span| {
            source
                .get(..span.start)
                .map_or(1, |head| head.matches('\n').count() + 1)
        });
        RuleHit { rank, line, text }
    });
    Probe {
        key: event
            .ch
            .map_or_else(|| "Special key".to_owned(), |ch| ch.to_string()),
        handled: output.consumed,
        output: describe(&output.action),
        composing: output.composing,
        rule,
        captures,
        modifiers: held(event.mods),
    }
}

/// The modifier keys held for one key press, or `None`.
fn held(mods: Modifiers) -> String {
    let names: Vec<&str> = [
        (mods.shift, "Shift"),
        (mods.ctrl, "Ctrl"),
        (mods.alt, "Alt"),
        (mods.caps_lock, "Caps Lock"),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .map(|(_, name)| name)
    .collect();
    if names.is_empty() {
        "None".to_owned()
    } else {
        names.join(", ")
    }
}

/// Each capture with its position, such as `$1 = k`, or `None`.
fn capture_list(captures: &[String]) -> String {
    if captures.is_empty() {
        return "None".to_owned();
    }
    captures
        .iter()
        .enumerate()
        .map(|(index, text)| format!("${} = {text}", index + 1))
        .collect::<Vec<_>>()
        .join(", ")
}

fn describe(action: &Action) -> String {
    match action {
        Action::None => "No change".to_owned(),
        Action::Insert(text) => format!("Insert {text}"),
        Action::Delete(count) => format!("Delete {count}"),
        Action::Replace { delete, insert } => format!("Replace {delete} with {insert}"),
    }
}

fn detail(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.add_sized([96.0, 18.0], egui::Label::new(widgets::muted(ui, label)));
        ui.label(RichText::new(value).monospace());
    });
}

/// The printable characters, Backspace and Enter typed this frame.
fn typed_keys(ctx: &Context) -> Vec<TypedKey> {
    ctx.input(|input| {
        // Shortcuts such as Ctrl+1 send no text; keep Alt for AltGr.
        let shortcut = input.modifiers.command && !input.modifiers.alt;
        let mut keys = Vec::new();
        for event in &input.events {
            match event {
                Event::Text(text) if !shortcut => keys.extend(text.chars().map(TypedKey::Char)),
                Event::Key {
                    key: Key::Backspace,
                    pressed: true,
                    ..
                } => keys.push(TypedKey::Backspace),
                Event::Key {
                    key: Key::Enter,
                    pressed: true,
                    ..
                } => keys.push(TypedKey::Enter),
                _ => {}
            }
        }
        keys
    })
}
