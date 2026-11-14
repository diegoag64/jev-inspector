use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, Clone, Copy, PartialEq)]
pub enum Filter {
    #[default]
    All,
    Success,
    Pending,
    Starred,
}
pub struct Workspace {
    pub paused: bool,
    pub follow: bool,
    pub follow_scroll_pending: bool,
    pub arrivals: motion::Arrivals,
    pub pending: Option<Snapshot>,
    pub clear_before: u64,
    pub filter: Filter,
    pub starred: BTreeSet<u64>,
    pub cache: BTreeMap<u64, Exchange>,
    pub query_error: Option<String>,
    pub applied_query: String,
    pub probability_limits: BTreeMap<(u64, String), usize>,
    pub setup_help: bool,
    pub notice: Option<(String, bool)>,
    pub help: bool,
    pub export: bool,
    pub compact: bool,
    pub detail_expanded: bool,
    pub session_name: Option<String>,
    pub last_received: bool,
    pub waker: Option<rune::Waker>,
    pub job: Option<Arc<session::Job>>,
    pub prefs: rune::prefs::Prefs,
    pub comparison: compare::Comparison,
    pub list_width: f32,
    pub window_width: f32,
    pub window_height: f32,
}
impl Default for Workspace {
    fn default() -> Self {
        Self {
            paused: false,
            follow: false,
            follow_scroll_pending: false,
            arrivals: motion::Arrivals::default(),
            pending: None,
            clear_before: 0,
            filter: Filter::All,
            starred: BTreeSet::new(),
            cache: BTreeMap::new(),
            query_error: None,
            applied_query: String::new(),
            probability_limits: BTreeMap::new(),
            setup_help: false,
            notice: None,
            help: false,
            export: false,
            compact: true,
            detail_expanded: false,
            session_name: None,
            last_received: false,
            waker: None,
            job: None,
            prefs: rune::prefs::Prefs::memory(),
            comparison: compare::Comparison::default(),
            list_width: 550.0,
            window_width: 1440.0,
            window_height: 900.0,
        }
    }
}
impl Workspace {
    pub fn matches(&self, r: &Summary) -> bool {
        match self.filter {
            Filter::All => true,
            Filter::Success => r.status.is_some_and(|n| (200..300).contains(&n)) && !failed(r),
            Filter::Pending => r.outcome == Outcome::Pending,
            Filter::Starred => self.starred.contains(&r.id),
        }
    }
}
impl State {
    pub fn toggle_expanded(&mut self) {
        self.workspace.detail_expanded = !self.workspace.detail_expanded;
        self.diagram.request_fit();
    }
    pub fn apply_snapshot(&mut self, snapshot: Snapshot) {
        self.workspace.arrivals.record(&self.snapshot, &snapshot);
        if self.snapshot.instance_id != snapshot.instance_id {
            if !self.snapshot.instance_id.is_empty() {
                self.workspace.notice = Some(("Relay restarted. Previous stars and comparison baseline were cleared. Save a session to keep an investigation across restarts.".into(), false));
            }
            self.workspace.comparison = compare::Comparison::default();
            self.workspace.probability_limits.clear();
            self.selected = None;
            self.detail = None;
            self.detail_error = None;
            self.workspace.clear_before = 0;
            self.workspace.starred.clear();
            self.workspace.cache.clear();
            self.table.clear_selection();
            self.mailbox.lock().unwrap().selected = None;
        } else if self
            .selected
            .is_some_and(|id| !snapshot.requests.iter().any(|r| r.id == id))
        {
            self.workspace.notice = Some(("The selected exchange is no longer retained by the relay. Save a session to keep older captures.".into(), false));
        }
        self.workspace
            .cache
            .retain(|id, _| snapshot.requests.iter().any(|r| r.id == *id));
        self.workspace
            .starred
            .retain(|id| snapshot.requests.iter().any(|r| r.id == *id));
        self.workspace
            .probability_limits
            .retain(|(id, _), _| snapshot.requests.iter().any(|r| r.id == *id));
        self.snapshot = snapshot;
        self.filter_rows();
    }
    pub fn toggle_pause(&mut self) {
        if self.workspace.session_name.is_some() || self.demo {
            return;
        }
        self.workspace.paused = !self.workspace.paused;
        if !self.workspace.paused
            && let Some(snapshot) = self.workspace.pending.take()
        {
            self.apply_snapshot(snapshot);
            self.mailbox.lock().unwrap().refresh = true;
        }
    }
    pub fn follow_scroll(&mut self, tx: &mut Tx) {
        if std::mem::take(&mut self.workspace.follow_scroll_pending)
            && self.workspace.follow
            && !self.visible.is_empty()
        {
            tx.scroll_to_instant("requests/viewport", 0.0, 0.0);
        }
    }
    pub fn clear_view(&mut self) {
        self.workspace.clear_before = self
            .snapshot
            .requests
            .iter()
            .map(|r| r.id)
            .max()
            .unwrap_or(0);
        self.workspace.follow = false;
        self.filter_rows();
    }
    pub fn reset_filters(&mut self) {
        self.filter = TextEdit::default();
        self.errors_only = false;
        self.workspace.filter = Filter::All;
        self.filter_rows();
    }
    pub fn star(&mut self) {
        if let Some(id) = self.selected {
            if !self.workspace.starred.remove(&id) {
                self.workspace.starred.insert(id);
            }
            self.filter_rows();
        }
    }
    pub fn navigate(&mut self, delta: isize, tx: &mut Tx) {
        let index = self
            .visible
            .iter()
            .position(|r| Some(r.id) == self.selected);
        let next = index.map_or_else(
            || {
                if delta < 0 {
                    self.visible.len().saturating_sub(1)
                } else {
                    0
                }
            },
            |index| {
                index
                    .saturating_add_signed(delta)
                    .min(self.visible.len().saturating_sub(1))
            },
        );
        if let Some(id) = self.visible.get(next).map(|r| r.id) {
            self.workspace.follow = false;
            self.select(id);
            self.table.reveal(id.to_string(), tx);
        }
    }
    pub fn toggle_theme(&mut self) {
        self.dark = !self.dark;
        if let Err(e) = self.workspace.prefs.set("dark", &self.dark) {
            self.workspace.notice = Some((e.to_string(), true));
        }
    }
    pub fn toggle_density(&mut self) {
        self.workspace.compact = !self.workspace.compact;
        if let Err(e) = self.workspace.prefs.set("compact", &self.workspace.compact) {
            self.workspace.notice = Some((e.to_string(), true));
        }
    }
    pub fn poll_job(&mut self) -> bool {
        let result = self
            .workspace
            .job
            .as_ref()
            .and_then(|j| j.result.lock().unwrap().take());
        let Some(result) = result else {
            return false;
        };
        self.workspace.job = None;
        match result {
            Ok(session::ResultValue::Saved(path, missing, last_received)) => {
                self.workspace.notice = Some((
                    format!(
                        "Saved {}{}{}",
                        if last_received {
                            "last-received snapshot: "
                        } else {
                            ""
                        },
                        path.display(),
                        if missing > 0 {
                            format!(
                                " · {missing} exchanges have summaries only (captures unavailable)"
                            )
                        } else {
                            String::new()
                        }
                    ),
                    false,
                ));
            }
            Ok(session::ResultValue::Loaded(session, path)) => {
                self.workspace.arrivals.clear();
                self.workspace.last_received = session.last_received;
                self.workspace.comparison = compare::Comparison::default();
                self.workspace.probability_limits.clear();
                self.workspace.pending = None;
                self.workspace.paused = false;
                self.workspace.follow = false;
                self.workspace.clear_before = 0;
                self.workspace.cache = session.exchanges;
                self.workspace.starred = session.starred;
                self.workspace.session_name = Some(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into(),
                );
                self.snapshot = session.snapshot;
                self.selected = None;
                self.detail = None;
                self.detail_error = None;
                self.table.clear_selection();
                self.reset_filters();
                self.workspace.notice = Some((
                    format!(
                        "Opened {}{} requests · {} summaries without captures",
                        if session.last_received {
                            "last-received snapshot: "
                        } else {
                            ""
                        },
                        self.snapshot.requests.len(),
                        session.missing.len()
                    ),
                    false,
                ));
            }
            Ok(session::ResultValue::Cancelled) => {
                self.workspace.notice =
                    Some(("Export cancelled. No file was written.".into(), false))
            }
            Err(error) => self.workspace.notice = Some((error, true)),
        }
        true
    }
    pub fn return_live(&mut self) {
        self.workspace.arrivals.clear();
        self.workspace.comparison = compare::Comparison::default();
        self.workspace.probability_limits.clear();
        self.workspace.session_name = None;
        self.workspace.last_received = false;
        self.workspace.cache.clear();
        self.workspace.starred.clear();
        self.workspace.clear_before = 0;
        self.workspace.pending = None;
        self.snapshot = Snapshot::default();
        self.selected = None;
        self.detail = None;
        self.table.clear_selection();
        self.mailbox.lock().unwrap().selected = None;
        self.reset_filters();
        // Force the poller's revision check to refresh even if the relay is idle.
        self.mailbox.lock().unwrap().refresh = true;
        if self.showcase.is_some() {
            showcase::populate(self);
        } else if self.demo {
            demo::populate(self);
        }
    }
}
pub fn on_key(s: &mut State, event: &rune::KeyEvent, tx: &mut Tx) -> bool {
    if s.onboarding.open {
        return onboarding::on_key(s, event, tx);
    }
    // Dialogs and the question menu own keyboard input until they close.
    // Returning false preserves their local Escape and navigation handlers.
    if s.workspace.help || s.workspace.export || s.cost.settings || s.diagram_questions_open {
        return false;
    }
    if s.tab == 3
        && s.diagram.inspection.is_some()
        && tx.focused().is_some_and(|key| key.contains(":inspector"))
    {
        return false;
    }
    // Element handlers cannot receive Tab after an action clears focus.
    if event.key == rune::Key::Tab && s.tab == 3 && !s.workspace.help && !s.workspace.export {
        diagram::reveal_keyboard_card(s);
        return false;
    }
    if event.is_command('k') || (event.is_command('f') && !matches!(s.tab, 1 | 2)) {
        s.request_text.search.focused = false;
        s.response_text.search.focused = false;
        s.filter.focused = true;
        return true;
    }
    if event.is_command('f') {
        s.filter.focused = false;
        if s.tab == 1 {
            s.response_text.search.focused = false;
        } else {
            s.request_text.search.focused = false;
        }
        let doc = if s.tab == 1 {
            &mut s.request_text
        } else {
            &mut s.response_text
        };
        doc.search_open = true;
        doc.tree = false;
        doc.search.focused = true;
        doc.schedule_reveal();
        return true;
    }
    if event.is_command('p') {
        s.toggle_pause();
        return true;
    }
    if event.is_command('b') {
        s.star();
        return true;
    }
    if event.is_command('l') {
        s.clear_view();
        return true;
    }
    if event.is_command('e') {
        s.workspace.export = !s.workspace.export;
        return true;
    }
    if event.is_command('o') {
        session::open_dialog(s, tx);
        return true;
    }
    for n in 0..5 {
        if event.is_command(char::from(b'1' + n)) {
            showcase::stop(s);
            s.select_tab(n);
            tx.blur();
            return true;
        }
    }
    match event.key {
        rune::Key::Function(1) => {
            s.workspace.help = !s.workspace.help;
            true
        }
        rune::Key::Up if event.mods.alt => {
            s.navigate(-1, tx);
            true
        }
        rune::Key::Down if event.mods.alt => {
            s.navigate(1, tx);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clear_is_reversible_and_restart_resets_instance_local_state() {
        let mut s = State::default();
        demo::populate(&mut s);
        s.star();
        s.clear_view();
        assert!(s.visible.is_empty());
        assert_eq!(s.snapshot.requests.len(), 12);
        s.workspace.clear_before = 0;
        s.filter_rows();
        assert_eq!(s.visible.len(), 12);
        let mut next = s.snapshot.clone();
        next.instance_id = "restarted".into();
        s.apply_snapshot(next);
        assert!(s.workspace.starred.is_empty());
        assert_eq!(s.workspace.clear_before, 0);
    }
    #[test]
    fn sorting_and_starring_preserve_stable_selection() {
        let mut s = State::default();
        demo::populate(&mut s);
        s.select(4);
        s.star();
        s.table.set_sort("time", rune_kit::SortDir::Ascending);
        s.filter_rows();
        assert_eq!(s.visible.first().unwrap().id, 1);
        assert_eq!(s.selected, Some(4));
        s.workspace.filter = Filter::Starred;
        s.filter_rows();
        assert_eq!(s.visible.len(), 1);
        s.star();
        assert!(s.visible.is_empty());
        assert_eq!(
            s.selected,
            Some(4),
            "Unstarring retains the investigated exchange outside the filter"
        );
    }
    #[test]
    fn pause_accumulates_latest_snapshot_and_resumes_without_losing_rows() {
        let mut s = State::default();
        demo::populate(&mut s);
        s.demo = false;
        s.toggle_pause();
        let mut next = s.snapshot.clone();
        next.requests.insert(0, demo::exchange(13).summary);
        s.workspace.pending = Some(next);
        assert_eq!(s.visible.len(), 12);
        s.toggle_pause();
        assert_eq!(s.visible.len(), 13);
        assert!(!s.workspace.paused);
    }
}
