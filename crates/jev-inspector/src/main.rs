mod answer_value;
mod compare;
mod demo;
mod detail_states;
#[cfg(test)]
mod detail_states_tests;
mod detail_view;
mod diagram;
mod feed;
mod icons;
mod json_tree;
mod json_view;
mod motion;
mod overview;
mod query;
mod request_scroll;
#[cfg(test)]
mod scroll_focus_tests;
mod session;
mod setup_view;
mod showcase;
#[cfg(test)]
mod showcase_tests;
mod ui;
#[cfg(test)]
mod workflow_tests;
mod workspace;
#[cfg(test)]
mod workspace_tests;
mod workspace_view;
use workspace_view::view;

use jev_protocol::{Exchange, Outcome, Snapshot, Summary};
use rune::{App, AppSpec, Color, Cx, El, TextEdit, Tx, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit, TableState};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub struct Mailbox {
    pub update: Option<Result<feed::Update, String>>,
    pub selected: Option<u64>,
    pub stop: bool,
    pub refresh: bool,
}

struct State {
    connection_file: PathBuf,
    mailbox: Arc<Mutex<Mailbox>>,
    snapshot: Snapshot,
    visible: Vec<Summary>,
    selected: Option<u64>,
    detail: Option<Exchange>,
    detail_error: Option<String>,
    request_text: json_view::JsonDocument,
    response_text: json_view::JsonDocument,
    filter: TextEdit,
    table: TableState,
    errors_only: bool,
    dark: bool,
    tab: u8,
    diagram: rune_diagram::DiagramState,
    diagram_question: String,
    diagram_questions_open: bool,
    connected: bool,
    connection_error: Option<String>,
    demo: bool,
    offline_fixture: bool,
    showcase: Option<showcase::Playback>,
    workspace: workspace::Workspace,
}

impl Default for State {
    fn default() -> Self {
        Self {
            connection_file: ".jev/connection.json".into(),
            mailbox: Arc::default(),
            snapshot: Snapshot::default(),
            visible: vec![],
            selected: None,
            detail: None,
            detail_error: None,
            request_text: json_view::JsonDocument::default(),
            response_text: json_view::JsonDocument::default(),
            filter: TextEdit::default(),
            table: TableState::new().key("requests"),
            errors_only: false,
            dark: true,
            tab: 0,
            diagram: rune_diagram::DiagramState::default(),
            diagram_question: String::new(),
            diagram_questions_open: false,
            connected: false,
            connection_error: None,
            demo: false,
            offline_fixture: false,
            showcase: None,
            workspace: workspace::Workspace::default(),
        }
    }
}

impl State {
    fn select_tab(&mut self, tab: u8) {
        self.tab = tab;
        self.diagram_questions_open = false;
        self.request_text.schedule_reveal();
        self.response_text.schedule_reveal();
    }
    fn filter_rows(&mut self) {
        let query = query::Query::parse(&self.filter.text);
        self.workspace.query_error = query.as_ref().err().cloned();
        let query = match query {
            Ok(query) => {
                self.workspace.applied_query = self.filter.text.clone();
                query
            }
            Err(_) => query::Query::parse(&self.workspace.applied_query).expect("last valid query"),
        };
        self.visible = self
            .snapshot
            .requests
            .iter()
            .filter(|s| {
                s.id > self.workspace.clear_before
                    && (!self.errors_only || failed(s))
                    && self.workspace.matches(s)
                    && query.matches(s)
            })
            .cloned()
            .collect();
        let sort = self.table.sort().map(|(c, d)| (c.to_owned(), d));
        self.visible.sort_by(|a, b| {
            let (column, direction) = sort
                .as_ref()
                .map(|(c, d)| (c.as_str(), *d))
                .unwrap_or(("request", rune_kit::SortDir::Descending));
            let order = match column {
                "time" => a
                    .duration_ms
                    .unwrap_or(-1.0)
                    .total_cmp(&b.duration_ms.unwrap_or(-1.0)),
                "status" => a.status.cmp(&b.status),
                "model" => a.model.cmp(&b.model),
                "started" => a.started_at_ms.cmp(&b.started_at_ms),
                "size" => {
                    (a.request_bytes + a.response_bytes).cmp(&(b.request_bytes + b.response_bytes))
                }
                _ => a.id.cmp(&b.id),
            };
            direction.apply(order).then_with(|| b.id.cmp(&a.id))
        });
        if self.selected.is_some_and(|id| {
            !self
                .snapshot
                .requests
                .iter()
                .any(|row| row.id == id && row.id > self.workspace.clear_before)
        }) {
            self.selected = None;
            self.detail = None;
            self.detail_error = None;
            self.table.clear_selection();
            self.mailbox.lock().expect("inspector mailbox").selected = None;
        }
        if (self.selected.is_none() || self.workspace.follow)
            && let Some(first) = self.visible.first()
        {
            self.select(first.id);
        }
    }
    fn select(&mut self, id: u64) {
        showcase::stop(self);
        if self.selected != Some(id) {
            self.request_text.schedule_reveal();
            self.response_text.schedule_reveal();
            self.request_text.copied = false;
            self.response_text.copied = false;
            self.diagram.reset();
            self.diagram_question.clear();
            self.diagram_questions_open = false;
            self.detail = None;
            self.detail_error = None;
        }
        self.selected = Some(id);
        self.table.select(id.to_string());
        self.mailbox.lock().expect("inspector mailbox").selected = Some(id);
        if let Some(detail) = self.workspace.cache.get(&id).cloned() {
            self.set_detail(detail);
        } else if self.demo && self.showcase.is_none() && self.workspace.session_name.is_none() {
            self.set_detail(demo::exchange(id));
        } else if self.showcase.is_some() && self.workspace.session_name.is_none() {
            self.detail_error = Some(
                "This example's capture is unavailable. Reopen the showcase to restore it.".into(),
            );
        } else if self.workspace.session_name.is_some() {
            self.detail_error = Some(
                "This session contains a summary only. The capture was unavailable when saved."
                    .into(),
            );
        }
    }
    fn set_detail(&mut self, detail: Exchange) {
        self.workspace.comparison.update(&detail);
        self.request_text.update(&detail.request);
        self.response_text.update(&detail.response);
        self.workspace
            .cache
            .insert(detail.summary.id, detail.clone());
        self.detail = Some(detail);
        self.detail_error = None;
    }
}

fn failed(s: &Summary) -> bool {
    s.status.is_some_and(|s| s >= 400)
        || matches!(
            s.outcome,
            Outcome::Disconnected | Outcome::Rejected | Outcome::UpstreamError
        )
}

fn setup(state: &mut State, waker: rune::Waker) {
    state.workspace.waker = Some(waker.clone());
    if !state.demo && !state.offline_fixture {
        feed::start(state.connection_file.clone(), state.mailbox.clone(), waker);
    }
}

fn poll(state: &mut State, now: f64, tx: &mut Tx) -> bool {
    let changed = state.poll_job() | showcase::tick(state, now, tx);
    let update = state
        .mailbox
        .lock()
        .expect("inspector mailbox")
        .update
        .take();
    match update {
        Some(Ok(update)) => {
            state.connected = true;
            state.connection_error = None;
            if state.workspace.session_name.is_some() {
                return changed;
            }
            let same_instance = state.snapshot.instance_id == update.snapshot.instance_id;
            if state.workspace.paused {
                state.workspace.pending = Some(update.snapshot);
            } else {
                state.apply_snapshot(update.snapshot);
            }
            if let Some((id, detail)) = update.detail
                && state.selected == Some(id)
                && (same_instance || !state.workspace.paused)
            {
                match detail {
                    Ok(detail) => state.set_detail(detail),
                    Err(message) => {
                        state.detail_error = Some(message);
                        if !state.workspace.cache.contains_key(&id) {
                            state.detail = None;
                        }
                    }
                }
            }
            true
        }
        Some(Err(message)) => {
            let connection_changed =
                state.connected || state.connection_error.as_ref() != Some(&message);
            state.connected = false;
            state.connection_error = Some(message);
            changed || connection_changed
        }
        None => changed,
    }
}

fn clip(value: &str, limit: usize) -> String {
    let mut text: String = value.chars().take(limit).collect();
    if value.chars().count() > limit {
        text.push_str(" …");
    }
    text
}

fn label(value: impl Into<String>, size: f32, color: Color) -> El {
    text(value)
        .font(rune::TextStyle::new(size, 450.0))
        .color(color)
}

fn status(s: &Summary) -> String {
    match s.outcome {
        Outcome::Pending => "In flight".into(),
        Outcome::Disconnected => "Detached".into(),
        Outcome::UpstreamError => "Transport".into(),
        _ => s
            .status
            .map(|n| n.to_string())
            .unwrap_or_else(|| "Rejected".into()),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut state = State::default();
    let mut rune_args = vec![];
    let mut fixture = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--connection-file" => {
                let Some(path) = args.next() else {
                    eprintln!("--connection-file requires a path");
                    std::process::exit(2);
                };
                state.connection_file = path.into();
            }
            "--demo" => demo::populate(&mut state),
            "--showcase" => showcase::populate(&mut state),
            "--tour" => {
                showcase::populate(&mut state);
                state.showcase.as_mut().unwrap().playing = true;
            }
            "--fixture" => {
                fixture = Some(args.next().unwrap_or_else(|| {
                    eprintln!("--fixture requires a scenario name");
                    std::process::exit(2);
                }));
            }
            "--diagram" => state.tab = 3,
            "--help" | "-h" => {
                println!(
                    "Jev inspector\n  --connection-file PATH  Connection written by jev-bridge (default .jev/connection.json)\n  --demo                  Synthetic, offline UI fixture\n  --showcase              14 varied offline examples\n  --tour                  Open examples and cycle layouts automatically\n  --diagram               Start on the Diagram tab\nRune QA flags are also accepted, including --snap FILE and --tree FILE."
                );
                return;
            }
            _ => rune_args.push(arg),
        }
    }
    if let Some(name) = fixture
        && let Err(error) = demo::scenario(&mut state, &name)
    {
        eprintln!("{error}");
        std::process::exit(2);
    }
    let mailbox = state.mailbox.clone();
    if !state.demo && !state.offline_fixture {
        state.workspace.prefs = rune::prefs::Prefs::open("dev.jev.inspector");
        state.dark = state.workspace.prefs.get("dark").unwrap_or(true);
        state.workspace.compact = state.workspace.prefs.get("compact").unwrap_or(false);
        state.workspace.list_width = state
            .workspace
            .prefs
            .get::<f32>("list_width")
            .filter(|v| v.is_finite())
            .unwrap_or(550.0)
            .clamp(440.0, 800.0);
    }
    let app = App::new(AppSpec {
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(workspace::on_key),
        min_size: Some((1100.0, 700.0)),
        families: ("Inter", "ui-monospace"),
        ..AppSpec::new("Jev inspector", (1440.0, 900.0), State::default, view)
    })
    .init(move || state)
    .args(rune_args)
    .tab_visits_controls(true)
    .chrome(rune::Chrome {
        full_size_content: false,
        ..rune::Chrome::default()
    });
    let result = app.try_run();
    mailbox.lock().expect("inspector mailbox").stop = true;
    if let Err(error) = result {
        eprintln!("Inspector window could not start: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filters_match_errors_and_selection_survives_filter_changes() {
        let mut state = State::default();
        demo::populate(&mut state);
        state.errors_only = true;
        state.filter_rows();
        assert!(state.visible.iter().all(failed));
        assert!(!state.visible.is_empty());
        state.filter = TextEdit::new("429");
        state.filter_rows();
        assert_eq!(state.visible.len(), 1);
        let id = state.visible[0].id;
        state.select(id);
        state.filter = TextEdit::default();
        state.filter_rows();
        assert_eq!(state.selected, Some(id));
    }
    #[test]
    fn rune_controls_change_tabs_and_filter_actual_state() {
        let mut state = State::default();
        demo::populate(&mut state);
        let mut rt = rune::Runtime::new(
            state,
            view,
            (1240.0, 820.0),
            rune::Env {
                system_dark: true,
                scale: 1.0,
            },
        );
        rt.frame(0.0);
        let (x, y) = rt.find("tab-1").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.02);
        assert_eq!(rt.state.tab, 1);
        let (x, y) = rt.find("errors-filter").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.02);
        assert!(rt.state.errors_only);
        assert!(rt.state.visible.iter().all(failed));
    }
}
