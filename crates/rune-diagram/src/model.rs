use crate::{Camera, ContentState};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NodeStatus {
    #[default]
    Neutral,
    Active,
    Success,
    Error,
    Unavailable,
    Alternative,
}

#[derive(Clone, Debug)]
pub enum NodeBody {
    Result(crate::ResultContent),
    Text(String),
    /// None is unavailable; Some(Null) is a captured JSON null.
    Json(Option<Value>),
    Probabilities {
        values: Vec<(String, f64)>,
        selected: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub struct Node {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub status: NodeStatus,
    pub rect: [f32; 4],
    pub body: NodeBody,
    pub style: CardStyle,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardStyle {
    #[default]
    Trace,
    Workflow,
}

impl Node {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        rect: [f32; 4],
        body: NodeBody,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            kind: String::new(),
            status: NodeStatus::Neutral,
            rect,
            body,
            style: CardStyle::Trace,
        }
    }
    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = kind.into();
        self
    }
    pub fn status(mut self, status: NodeStatus) -> Self {
        self.status = status;
        self
    }
    pub fn style(mut self, style: CardStyle) -> Self {
        self.style = style;
        self
    }
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub from: String,
    pub to: String,
    /// Supplied by the caller: no route is inferred from probabilities.
    pub active: bool,
    pub label: Option<String>,
}
impl Edge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            active: false,
            label: None,
        }
    }
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}
impl Graph {
    /// Fail explicitly instead of silently dropping nodes or invalid connections.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.nodes.len() > 256 || self.edges.len() > 1024 {
            return Err("Diagram exceeds 256 nodes or 1024 edges.");
        }
        let mut ids = HashSet::new();
        for n in &self.nodes {
            if n.id.is_empty() || !ids.insert(n.id.as_str()) {
                return Err("Node IDs must be nonempty and unique.");
            }
            if !n
                .rect
                .into_iter()
                .all(|v| v.is_finite() && (0.0..=100_000.0).contains(&v))
                || n.rect[2] < 120.0
                || n.rect[3] < 100.0
            {
                return Err(
                    "Node rectangles must be finite, nonnegative, and at least 120 by 100.",
                );
            }
        }
        if self
            .edges
            .iter()
            .any(|e| !ids.contains(e.from.as_str()) || !ids.contains(e.to.as_str()))
        {
            return Err("Each connection must reference existing nodes.");
        }
        Ok(())
    }
    pub fn size(&self) -> [f32; 2] {
        self.nodes.iter().fold([1.0_f32, 1.0_f32], |s, n| {
            [
                s[0].max(n.rect[0] + n.rect[2]),
                s[1].max(n.rect[1] + n.rect[3]),
            ]
        })
    }
}

#[derive(Default)]
pub struct DiagramState {
    pub camera: Camera,
    pub selected: Option<String>,
    pub content: ContentState,
    pub(crate) fitted: bool,
    pub(crate) viewport: [f32; 2],
    pub(crate) drag_start: Camera,
    pub(crate) initial_view: bool,
    pub(crate) manual_view: bool,
    fit_requested: bool,
}
impl DiagramState {
    /// Fit on the next viewport layout while preserving selection and disclosures.
    pub fn request_fit(&mut self) {
        self.fitted = false;
        self.fit_requested = true;
    }
    pub fn reset(&mut self) {
        let viewport = self.viewport;
        *self = Self::default();
        self.viewport = viewport;
    }
    pub fn fit(&mut self, content: [f32; 2]) {
        self.camera = Camera::fit(self.viewport, content);
        self.fitted = true;
        self.initial_view = false;
        self.manual_view = false;
        self.fit_requested = false;
    }
    pub(crate) fn initial_camera(&self, content: [f32; 2], focus: Option<[f32; 4]>) -> Camera {
        let fit = Camera::fit(self.viewport, content);
        if self.fit_requested || fit.scale >= 0.75 {
            return fit;
        }
        let Some([x, y, w, h]) = focus else {
            return fit;
        };
        let scale = ((self.viewport[0] - 16.0) / w)
            .min((self.viewport[1] - 16.0) / h)
            .clamp(0.8, 1.0);
        Camera {
            x: (self.viewport[0] - w * scale) / 2.0 - x * scale,
            y: ((self.viewport[1] - h * scale) / 2.0).max(8.0) - y * scale,
            scale,
        }
    }
    pub(crate) fn initialize(&mut self, content: [f32; 2], focus: Option<[f32; 4]>) {
        if self.fit_requested {
            self.fit(content);
            return;
        }
        self.camera = self.initial_camera(content, focus);
        self.fitted = true;
        self.initial_view = true;
        self.manual_view = false;
    }
    pub fn zoom(&mut self, factor: f32) {
        self.manual_view = true;
        let center = [self.viewport[0] / 2.0, self.viewport[1] / 2.0];
        self.camera.zoom(factor, center, center);
    }
    /// Reveal a card without changing zoom; tall cards start at their heading.
    /// Call before Tab traversal if the surrounding app can enter the diagram
    /// from another pane after a manual pan.
    pub fn reveal(&mut self, rect: [f32; 4]) {
        let [x, y, w, h] = rect.map(|v| v * self.camera.scale);
        let [vw, vh] = self.viewport;
        let target_h = h.min((vh - 16.0).max(1.0));
        if x + self.camera.x < 8.0 || x + w + self.camera.x > vw - 8.0 {
            self.camera.x = (vw - w) / 2.0 - x;
        }
        if y + self.camera.y < 8.0 || y + target_h + self.camera.y > vh - 8.0 {
            self.camera.y = 8.0 - y;
        }
        self.manual_view = true;
    }
}
