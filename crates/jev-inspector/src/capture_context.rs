use crate::{Outcome, State, Summary};

#[derive(Clone, Copy)]
pub enum PendingContext {
    Live,
    Saved,
    Disconnected,
}

pub fn pending(s: &State) -> PendingContext {
    if s.workspace.session_name.is_some() {
        PendingContext::Saved
    } else if s.connection_error.is_some() || (!s.connected && !s.demo) {
        PendingContext::Disconnected
    } else {
        PendingContext::Live
    }
}

pub fn status(s: &State, r: &Summary) -> String {
    if r.outcome != Outcome::Pending {
        return crate::status(r);
    }
    match pending(s) {
        PendingContext::Live => "In flight",
        PendingContext::Saved => "Pending when saved",
        PendingContext::Disconnected => "Status unknown",
    }
    .into()
}

impl PendingContext {
    pub fn partial_notice(self) -> &'static str {
        match self {
            Self::Live => "Partial response · The exchange is still in flight.",
            Self::Saved => {
                "Incomplete when saved · These results are a snapshot and will not update."
            }
            Self::Disconnected => {
                "Partial response · These are the last captured results; current status is unknown."
            }
        }
    }
    pub fn count_label(self) -> &'static str {
        match self {
            Self::Live => "In flight",
            Self::Saved => "Incomplete",
            Self::Disconnected => "Last known pending",
        }
    }
    pub fn filter_label(self) -> &'static str {
        match self {
            Self::Live => "In flight",
            Self::Saved => "Incomplete",
            Self::Disconnected => "Pending (offline)",
        }
    }
    pub fn missing_answers(self) -> (&'static str, &'static str) {
        match self {
            Self::Live => (
                "Response pending",
                "Typed answers will appear when the response is captured.",
            ),
            Self::Saved => (
                "Response incomplete when saved",
                "This snapshot has no captured typed answers. It will not update.",
            ),
            Self::Disconnected => (
                "Response status unknown",
                "No typed answers were captured before the connection was lost. Reconnect to check the response.",
            ),
        }
    }
    pub fn timing(self) -> (&'static str, &'static str) {
        match self {
            Self::Live => (
                "In flight",
                "Timing becomes available as the exchange progresses.",
            ),
            Self::Saved => (
                "Incomplete when saved",
                "This snapshot contains only the timing recorded before it was saved. It will not update.",
            ),
            Self::Disconnected => (
                "Status unknown",
                "These are the last recorded timings. The current response status is unavailable while the relay is disconnected.",
            ),
        }
    }
}
