use super::*;

const ARRIVAL_SECONDS: f64 = 1.2;

/// Track receipt time, not row mount time: scrolling and response updates must
/// not replay the arrival cue. The first snapshot is existing history.
#[derive(Default)]
pub struct Arrivals {
    now: f64,
    received: std::collections::BTreeMap<u64, f64>,
}

impl Arrivals {
    pub fn clear(&mut self) {
        self.received.clear();
    }

    pub fn record(&mut self, previous: &Snapshot, next: &Snapshot) {
        if previous.instance_id.is_empty() || previous.instance_id != next.instance_id {
            self.clear();
            return;
        }
        let existing: std::collections::BTreeSet<_> =
            previous.requests.iter().map(|r| r.id).collect();
        self.received
            .retain(|id, _| next.requests.iter().any(|r| r.id == *id));
        for r in &next.requests {
            if !existing.contains(&r.id) {
                self.received.insert(r.id, self.now);
            }
        }
    }

    pub fn tick(&mut self, now: f64) -> bool {
        self.now = now;
        let changed = self.active();
        self.received.retain(|_, at| now - *at < ARRIVAL_SECONDS);
        changed
    }

    pub fn active(&self) -> bool {
        !self.received.is_empty()
    }

    pub fn progress(&self, id: u64, reduced: bool) -> f32 {
        if reduced {
            return 1.0;
        }
        self.received.get(&id).map_or(1.0, |at| {
            let t = ((self.now - at) / ARRIVAL_SECONDS).clamp(0.0, 1.0) as f32;
            t * t * (3.0 - 2.0 * t)
        })
    }
}

pub fn traffic_selected(k: &Kit) -> Color {
    rune::color::hex(if k.dark { "#303032" } else { "#e8e8ed" }).unwrap()
}

pub fn traffic_muted(k: &Kit) -> Color {
    k.c.text_muted
}

/// Keep the pane's geometry stable. Replace its contents as one surface so long JSON,
/// cards and diagrams never interpolate their individual text/layout into each other.
pub fn detail(s: &State, body: El) -> El {
    let phase = match &s.detail {
        None if s.detail_error.is_some() || s.connection_error.is_some() => "unavailable",
        None => "loading",
        Some(d) if d.summary.outcome == Outcome::Pending => "pending",
        Some(_) => "ready",
    };
    col()
        .key("detail-stage")
        .grow(1.0)
        .min_h(0.0)
        .w_full()
        .clip_soft(0.0)
        .child(
            col()
                .grow(1.0)
                .min_h(0.0)
                .w_full()
                .key(format!(
                    "detail-content-{}-{}-{phase}",
                    s.selected.unwrap_or(0),
                    s.tab
                ))
                .child(body)
                .instant_changes()
                .transition(
                    rune::Transition::asymmetric(
                        rune::Transition::offset(0.0, 6.0).combined(rune::Transition::opacity()),
                        rune::Transition::none(),
                    )
                    .springs(rune::SpringPair::new(
                        rune::Spring::new(0.22, 0.0),
                        rune::Spring::NONE,
                    )),
                ),
        )
}

pub fn metric(key: &str, value: String, size: f32, color: Color, k: &Kit) -> El {
    rune_kit::RollingText::new(format!("inspector-metric-{key}"), value.clone())
        .font(rune::TextStyle::new(size, 590.0).features("'tnum' 1"))
        .color(color)
        .spring(rune::Spring::new(0.28, 0.0))
        .build(k)
        .role("text")
        .label(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrivals_ignore_history_updates_and_restart_and_expire_without_new_traffic() {
        let mut s = State::default();
        demo::populate(&mut s);
        let history = s.snapshot.clone();
        let mut arrivals = Arrivals::default();
        arrivals.record(&Snapshot::default(), &history);
        assert!(!arrivals.active());
        let mut next = history.clone();
        next.requests.insert(0, demo::exchange(13).summary);
        arrivals.tick(5.0);
        arrivals.record(&history, &next);
        assert_eq!(arrivals.progress(13, false), 0.0);
        assert_eq!(arrivals.progress(12, false), 1.0);
        assert_eq!(arrivals.progress(13, true), 1.0);
        arrivals.tick(5.6);
        let halfway = arrivals.progress(13, false);
        assert!((halfway - 0.5).abs() < 0.001);
        let mut completed = next.clone();
        completed.requests[0].status = Some(200);
        arrivals.record(&next, &completed);
        assert_eq!(arrivals.progress(13, false), halfway);
        assert!(arrivals.tick(6.3), "expiry must repaint the settled theme");
        assert!(!arrivals.active());
        assert!(!arrivals.tick(6.4), "idle feed must stop animating");
        let mut restarted = next.clone();
        restarted.instance_id = "another-relay".into();
        arrivals.record(&next, &restarted);
        assert!(!arrivals.active());
    }
}
