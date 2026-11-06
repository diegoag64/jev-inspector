//! Synthetic workflow supplied by the application, not inferred from HTTP traffic.
use rune::{AppSpec, Cx, El, col, text};
use rune_diagram::{Diagram, DiagramState, Edge, Graph, Node, NodeBody, NodeStatus};
use rune_kit::Kit;
use serde_json::json;

#[derive(Default)]
struct State {
    diagram: DiagramState,
}
fn graph() -> Graph {
    let mut graph=Graph { nodes: vec![
        Node::new("input", "Ticket received", [0.0,160.0,240.0,200.0], NodeBody::Json(Some(json!({"message":"Charged twice", "channel":"support"})))).kind("INPUT").status(NodeStatus::Success),
        Node::new("classify", "Classify ticket", [330.0,160.0,260.0,260.0], NodeBody::Probabilities { values:vec![("billing".into(),0.94),("technical".into(),0.04),("sales".into(),0.02)],selected:Some("billing".into()) }).kind("JEV REQUEST").status(NodeStatus::Success),
        Node::new("condition", "Application condition", [680.0,160.0,240.0,200.0], NodeBody::Text("The application checks whether the returned category is billing. This condition is explicit fixture data.".into())).kind("CONDITION").status(NodeStatus::Success),
        Node::new("billing", "Billing queue", [1010.0,0.0,230.0,180.0], NodeBody::Text("Fixture route: selected".into())).kind("TRANSFORM").status(NodeStatus::Success),
        Node::new("other", "General queue", [1010.0,320.0,230.0,180.0], NodeBody::Text("Fixture route: not taken".into())).kind("TRANSFORM").status(NodeStatus::Unavailable),
        Node::new("output", "Ticket routed", [1330.0,160.0,230.0,200.0], NodeBody::Json(Some(json!({"queue":"billing","assigned":true})))).kind("OUTPUT").status(NodeStatus::Success),
    ], edges: vec![Edge::new("input","classify").active(true),Edge::new("classify","condition").active(true),Edge::new("condition","billing").active(true),Edge::new("condition","other"),Edge::new("billing","output").active(true),Edge::new("other","output")] };
    for node in &mut graph.nodes {
        node.style = rune_diagram::CardStyle::Workflow;
    }
    graph
}
fn view(s: &State, _: &mut Cx<State>) -> El {
    let kit = rune_diagram::jev_theme(&Kit::neutral(true));
    col()
        .w_full()
        .h_full()
        .padding(24.0)
        .gap(16.0)
        .fill(kit.c.bg_base)
        .child(text("Rune diagram · synthetic support workflow").color(kit.c.text_base))
        .child(
            Diagram::new("flow", &graph(), &s.diagram, |s: &mut State| &mut s.diagram).build(&kit),
        )
}
fn main() {
    rune::run(AppSpec {
        families: ("Inter", "ui-monospace"),
        ..AppSpec::new(
            "Rune diagram components",
            (1400.0, 850.0),
            State::default,
            view,
        )
    });
}
