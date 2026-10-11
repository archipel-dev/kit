//! UX audits: checks that hold for every screen built with the kit, whoever built it.

use super::{Harness, Node, NodeT, Role};
use egui::accesskit::Action;
use std::fmt;

/// Smallest width or height of a pointer target, in points.
pub const MIN_TARGET: f32 = 16.0;

/// What an audit found wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Problem {
    /// An interactive element has no accessible name: screen readers and tests can't
    /// find it.
    MissingName,
    /// A pointer target is smaller than [`MIN_TARGET`].
    SmallTarget,
    /// Two interactive elements partly overlap: one click could hit either. (A target
    /// fully inside another, like a tab's close button, is fine.)
    Overlap,
    /// <kbd>Tab</kbd> never reaches a focusable element.
    Unreachable,
}

/// One problem on one element.
#[derive(Debug, Clone)]
pub struct Finding {
    /// What is wrong.
    pub problem: Problem,
    /// The element, as `Role "name"`.
    pub element: String,
    /// Details (sizes, the other element).
    pub detail: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {} {}", self.problem, self.element, self.detail)
    }
}

/// The result of [`audit`].
#[derive(Debug, Clone, Default)]
pub struct Audit {
    /// Everything found, in tree order.
    pub findings: Vec<Finding>,
}

impl Audit {
    /// Whether nothing was found.
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// The findings of one kind.
    pub fn of(&self, problem: Problem) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.problem == problem)
            .collect()
    }

    /// Panics with every finding listed.
    ///
    /// # Panics
    /// If the audit found anything.
    #[track_caller]
    pub fn assert_clean(&self) {
        assert!(
            self.is_clean(),
            "UX audit found {} problem(s):\n  {}",
            self.findings.len(),
            self.findings
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n  ")
        );
    }
}

/// Roles that group other elements; they may overlap their content.
fn is_container(role: Role) -> bool {
    matches!(
        role,
        Role::Window
            | Role::ScrollView
            | Role::GenericContainer
            | Role::List
            | Role::ListBox
            | Role::Tree
            | Role::Table
            | Role::Grid
            | Role::Menu
            | Role::MenuBar
            | Role::Dialog
            | Role::TabList
            | Role::Group
    )
}

/// Roles of a collection that holds the keyboard focus for its rows.
fn is_collection(role: Role) -> bool {
    matches!(
        role,
        Role::List | Role::ListBox | Role::Tree | Role::Table | Role::Grid | Role::TreeGrid
    )
}

/// Roles of the rows (and cells) of a collection.
fn is_row(role: Role) -> bool {
    matches!(
        role,
        Role::ListItem | Role::ListBoxOption | Role::TreeItem | Role::Row | Role::Cell
    )
}

/// Whether `node` supports `action` (focus, click).
pub(crate) fn supports(node: &Node<'_>, action: Action) -> bool {
    node.accesskit_node().data().supports_action(action)
}

/// The part of `node` that can be seen and clicked: its bounds clipped by every ancestor
/// that has bounds (collections give their rows' parent the viewport bounds).
fn visible_rect(node: &Node<'_>) -> egui::Rect {
    let mut rect = node.rect();
    let mut parent = node.parent();
    while let Some(p) = parent {
        if p.accesskit_node().has_bounds() {
            rect = rect.intersect(p.rect());
        }
        parent = p.parent();
    }
    rect
}

/// egui's scroll bars have no role: thin, long, unnamed targets.
fn is_scroll_bar(node: &Node<'_>) -> bool {
    let n = node.accesskit_node();
    let rect = node.rect();
    let (thin, long) = (
        rect.width().min(rect.height()),
        rect.width().max(rect.height()),
    );
    n.role() == Role::Unknown && n.label().is_none() && thin <= 12.0 && long >= 3.0 * thin
}

/// Whether `node` is something the user operates.
fn is_interactive(node: &Node<'_>) -> bool {
    let n = node.accesskit_node();
    !n.is_hidden()
        && !is_container(n.role())
        && !is_scroll_bar(node)
        && (supports(node, Action::Click) || supports(node, Action::Focus))
}

fn describe(node: &Node<'_>) -> String {
    let n = node.accesskit_node();
    match n.label() {
        Some(label) if !label.is_empty() => format!("{:?} {label:?}", n.role()),
        _ => format!("{:?} #{:?}", n.role(), n.id()),
    }
}

fn interactive_nodes<'t>(root: Node<'t>) -> Vec<Node<'t>> {
    root.children_recursive()
        .filter(|n| n.accesskit_node().has_bounds())
        .filter(is_interactive)
        .collect()
}

/// Checks the current screen of `harness` for problems every kit UI must avoid:
///
/// - [`Problem::MissingName`]: interactive elements without an accessible name.
/// - [`Problem::SmallTarget`]: enabled pointer targets under 16×16 points.
/// - [`Problem::Overlap`]: interactive elements that partly overlap (nested targets,
///   splitters, which sit on edges, and containers excepted).
///
/// egui's scroll bars, which have no accessible role, are skipped.
/// - [`Problem::Unreachable`]: enabled focusable elements that <kbd>Tab</kbd> never
///   reaches. The audit presses Tab, so it changes focus. Rows of a collection that Tab
///   reaches count as reachable: the arrow keys move between them (roving focus).
///
/// Run it at 1× (`testing::harness`); bounds are compared in physical pixels.
pub fn audit<State>(harness: &mut Harness<'_, State>) -> Audit {
    let mut findings = Vec::new();
    let mut focusable = Vec::new();
    {
        let nodes = interactive_nodes(harness.root());
        for node in &nodes {
            let n = node.accesskit_node();
            let element = describe(node);
            if n.label().is_none_or(|l| l.trim().is_empty()) && n.role() != Role::TextInput {
                findings.push(Finding {
                    problem: Problem::MissingName,
                    element: element.clone(),
                    detail: String::new(),
                });
            }
            if n.role() == Role::TextInput && n.label().is_none_or(|l| l.trim().is_empty()) {
                // Text inputs may be named by a placeholder.
                if n.placeholder().is_none_or(|p| p.trim().is_empty()) {
                    findings.push(Finding {
                        problem: Problem::MissingName,
                        element: element.clone(),
                        detail: "(no label or placeholder)".into(),
                    });
                }
            }
            if !n.is_disabled() && n.role() != Role::Splitter {
                // Rows cut off at a scroll edge are fine; only targets drawn small are not.
                let rect = node.rect();
                if rect.width() < MIN_TARGET - 0.5 || rect.height() < MIN_TARGET - 0.5 {
                    findings.push(Finding {
                        problem: Problem::SmallTarget,
                        element: element.clone(),
                        detail: format!("is {:.0}×{:.0}", rect.width(), rect.height()),
                    });
                }
            }
            if supports(node, Action::Focus) && !n.is_disabled() {
                focusable.push((n.id(), element));
            }
        }
        for (i, a) in nodes.iter().enumerate() {
            for b in &nodes[i + 1..] {
                let (na, nb) = (a.accesskit_node(), b.accesskit_node());
                if na.role() == Role::Splitter || nb.role() == Role::Splitter {
                    continue;
                }
                if na.is_descendant_of(&nb) || nb.is_descendant_of(&na) {
                    continue;
                }
                let (ra, rb) = (visible_rect(a), visible_rect(b));
                if !ra.is_positive() || !rb.is_positive() {
                    continue;
                }
                let nested = ra.contains_rect(rb) || rb.contains_rect(ra);
                let overlap = ra.intersect(rb);
                if !nested && overlap.is_positive() && overlap.area() > 4.0 {
                    findings.push(Finding {
                        problem: Problem::Overlap,
                        element: describe(a),
                        detail: format!("overlaps {}", describe(b)),
                    });
                }
            }
        }
    }

    // Walk the Tab order and see which focusable elements it reaches.
    let mut reached = std::collections::HashSet::new();
    for _ in 0..focusable.len() * 2 + 2 {
        harness.key_press(egui::Key::Tab);
        harness.run_ok();
        if let Some(focused) = harness
            .root()
            .children_recursive()
            .find(|n| n.accesskit_node().is_focused())
        {
            reached.insert(focused.accesskit_node().id());
        }
    }
    // Rows of a collection use roving focus: Tab focuses the collection, the arrow keys
    // move between its rows. A row inside the bounds of a reached collection is reachable.
    let collections: Vec<egui::Rect> = harness
        .root()
        .children_recursive()
        .filter(|n| {
            let a = n.accesskit_node();
            is_collection(a.role()) && a.has_bounds() && reached.contains(&a.id())
        })
        .map(|n| n.rect())
        .collect();
    let rows: std::collections::HashMap<_, _> = harness
        .root()
        .children_recursive()
        .filter(|n| is_row(n.accesskit_node().role()))
        .map(|n| (n.accesskit_node().id(), n.rect().center()))
        .collect();
    for (id, element) in focusable {
        let in_collection = rows
            .get(&id)
            .is_some_and(|center| collections.iter().any(|c| c.contains(*center)));
        if !reached.contains(&id) && !in_collection {
            findings.push(Finding {
                problem: Problem::Unreachable,
                element,
                detail: "is never focused by Tab".into(),
            });
        }
    }
    Audit { findings }
}
