//! Composable, native diagram components adapted from jev-devtools.
//! The caller supplies graph geometry and semantics; the renderer never infers execution.
mod camera;
mod content;
mod inspector;
mod model;
mod result;
mod theme;
mod view;

pub use camera::Camera;
pub use content::ContentState;
pub use content::DiagramContent;
pub use inspector::InspectionState;
pub use model::CardStyle;
pub use model::DiagramState;
pub use model::Edge;
pub use model::Graph;
pub use model::Node;
pub use model::NodeBody;
pub use model::NodeStatus;
pub use result::ResultContent;
pub use theme::jev_theme;
pub use theme::percent;
pub use view::Diagram;
pub use view::DiagramCard;
pub use view::ProbabilityBars;

#[cfg(test)]
mod tests;
