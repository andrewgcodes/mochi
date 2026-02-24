//! Split pane management for terminal multiplexing
//!
//! Provides a tree-based pane layout supporting horizontal and vertical splits.

use std::sync::atomic::{AtomicU64, Ordering};

use terminal_pty::{Child, WindowSize};

use crate::terminal::Terminal;

/// Global pane ID counter
static PANE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for a pane
pub type PaneId = u64;

/// Generate a new unique pane ID
fn next_pane_id() -> PaneId {
    PANE_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Direction of a split
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// Top/Bottom split
    Horizontal,
    /// Left/Right split
    Vertical,
}

/// A rectangle in pixel coordinates
#[derive(Debug, Clone, Copy)]
pub struct PaneRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PaneRect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Width of the divider between panes in pixels
pub const DIVIDER_WIDTH: u32 = 2;

/// A leaf pane containing a terminal and child process
pub struct Pane {
    pub id: PaneId,
    pub terminal: Terminal,
    pub child: Child,
    pub title: String,
    pub scroll_offset: usize,
}

impl Pane {
    pub fn new(terminal: Terminal, child: Child) -> Self {
        Self {
            id: next_pane_id(),
            terminal,
            child,
            title: String::from("Terminal"),
            scroll_offset: 0,
        }
    }
}

/// A node in the pane tree
pub enum PaneNode {
    /// A leaf node containing a single terminal pane
    Leaf(Box<Pane>),
    /// A split node containing two children
    Split {
        direction: SplitDirection,
        /// Split ratio (0.0 to 1.0) - fraction allocated to the first child
        ratio: f64,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

impl PaneNode {
    /// Create a new leaf node
    pub fn new_leaf(terminal: Terminal, child: Child) -> Self {
        PaneNode::Leaf(Box::new(Pane::new(terminal, child)))
    }

    /// Get the active pane (leaf) by ID
    pub fn find_pane(&self, id: PaneId) -> Option<&Pane> {
        match self {
            PaneNode::Leaf(pane) => {
                if pane.id == id {
                    Some(pane.as_ref())
                } else {
                    None
                }
            }
            PaneNode::Split { first, second, .. } => {
                first.find_pane(id).or_else(|| second.find_pane(id))
            }
        }
    }

    /// Get the active pane (leaf) by ID mutably
    pub fn find_pane_mut(&mut self, id: PaneId) -> Option<&mut Pane> {
        match self {
            PaneNode::Leaf(pane) => {
                if pane.id == id {
                    Some(pane.as_mut())
                } else {
                    None
                }
            }
            PaneNode::Split { first, second, .. } => {
                if let Some(pane) = first.find_pane_mut(id) {
                    Some(pane)
                } else {
                    second.find_pane_mut(id)
                }
            }
        }
    }

    /// Get the first leaf pane's ID
    pub fn first_pane_id(&self) -> PaneId {
        match self {
            PaneNode::Leaf(ref pane) => pane.id,
            PaneNode::Split { first, .. } => first.first_pane_id(),
        }
    }

    /// Collect all leaf panes
    pub fn collect_panes(&self) -> Vec<&Pane> {
        match self {
            PaneNode::Leaf(pane) => vec![pane.as_ref()],
            PaneNode::Split { first, second, .. } => {
                let mut panes = first.collect_panes();
                panes.extend(second.collect_panes());
                panes
            }
        }
    }

    /// Collect all leaf panes mutably
    pub fn collect_panes_mut(&mut self) -> Vec<&mut Pane> {
        match self {
            PaneNode::Leaf(pane) => vec![pane.as_mut()],
            PaneNode::Split { first, second, .. } => {
                let mut panes = first.collect_panes_mut();
                panes.extend(second.collect_panes_mut());
                panes
            }
        }
    }

    /// Count total leaf panes
    pub fn pane_count(&self) -> usize {
        match self {
            PaneNode::Leaf(_) => 1,
            PaneNode::Split { first, second, .. } => first.pane_count() + second.pane_count(),
        }
    }

    /// Check if a pane with the given ID exists in the tree
    pub fn contains_pane(&self, id: PaneId) -> bool {
        self.find_pane(id).is_some()
    }

    /// Compute the pixel rect for each pane given the available area
    pub fn compute_rects(&self, available: PaneRect) -> Vec<(PaneId, PaneRect)> {
        match self {
            PaneNode::Leaf(ref pane) => vec![(pane.id, available)],
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_rect, second_rect) = compute_split_rects(available, *direction, *ratio);
                let mut rects = first.compute_rects(first_rect);
                rects.extend(second.compute_rects(second_rect));
                rects
            }
        }
    }

    /// Compute the divider rects for rendering
    pub fn compute_dividers(&self, available: PaneRect) -> Vec<(SplitDirection, PaneRect)> {
        match self {
            PaneNode::Leaf(_) => vec![],
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let divider_rect = compute_divider_rect(available, *direction, *ratio);
                let (first_rect, second_rect) = compute_split_rects(available, *direction, *ratio);
                let mut dividers = vec![(*direction, divider_rect)];
                dividers.extend(first.compute_dividers(first_rect));
                dividers.extend(second.compute_dividers(second_rect));
                dividers
            }
        }
    }

    /// Split the pane with the given ID. Takes ownership of `self` and returns the new tree.
    /// Checks containment first, then only recurses into the branch that contains the target.
    /// Returns (new_tree, Some(new_pane_id)) on success, or (self, None) if target not found.
    pub fn split_pane(
        self,
        target_id: PaneId,
        direction: SplitDirection,
        new_terminal: Terminal,
        new_child: Child,
    ) -> (Self, Option<PaneId>) {
        match self {
            PaneNode::Leaf(pane) => {
                if pane.id == target_id {
                    let new_pane = Pane::new(new_terminal, new_child);
                    let new_id = new_pane.id;
                    let new_node = PaneNode::Split {
                        direction,
                        ratio: 0.5,
                        first: Box::new(PaneNode::Leaf(pane)),
                        second: Box::new(PaneNode::Leaf(Box::new(new_pane))),
                    };
                    (new_node, Some(new_id))
                } else {
                    (PaneNode::Leaf(pane), None)
                }
            }
            PaneNode::Split {
                direction: split_dir,
                ratio,
                first,
                second,
            } => {
                if first.contains_pane(target_id) {
                    let (new_first, result) =
                        first.split_pane(target_id, direction, new_terminal, new_child);
                    (
                        PaneNode::Split {
                            direction: split_dir,
                            ratio,
                            first: Box::new(new_first),
                            second,
                        },
                        result,
                    )
                } else if second.contains_pane(target_id) {
                    let (new_second, result) =
                        second.split_pane(target_id, direction, new_terminal, new_child);
                    (
                        PaneNode::Split {
                            direction: split_dir,
                            ratio,
                            first,
                            second: Box::new(new_second),
                        },
                        result,
                    )
                } else {
                    (
                        PaneNode::Split {
                            direction: split_dir,
                            ratio,
                            first,
                            second,
                        },
                        None,
                    )
                }
            }
        }
    }

    /// Remove a pane by ID and collapse the tree.
    /// Takes ownership and returns the new tree (or None if this was the last pane).
    pub fn remove_pane(self, target_id: PaneId) -> Option<Self> {
        match self {
            PaneNode::Leaf(pane) => {
                if pane.id == target_id {
                    None
                } else {
                    Some(PaneNode::Leaf(pane))
                }
            }
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                // Check if first child is the target leaf
                if let PaneNode::Leaf(ref pane) = *first {
                    if pane.id == target_id {
                        return Some(*second);
                    }
                }
                // Check if second child is the target leaf
                if let PaneNode::Leaf(ref pane) = *second {
                    if pane.id == target_id {
                        return Some(*first);
                    }
                }
                // Recurse into children
                if first.contains_pane(target_id) {
                    let new_first = first.remove_pane(target_id);
                    match new_first {
                        Some(node) => Some(PaneNode::Split {
                            direction,
                            ratio,
                            first: Box::new(node),
                            second,
                        }),
                        None => Some(*second),
                    }
                } else if second.contains_pane(target_id) {
                    let new_second = second.remove_pane(target_id);
                    match new_second {
                        Some(node) => Some(PaneNode::Split {
                            direction,
                            ratio,
                            first,
                            second: Box::new(node),
                        }),
                        None => Some(*first),
                    }
                } else {
                    Some(PaneNode::Split {
                        direction,
                        ratio,
                        first,
                        second,
                    })
                }
            }
        }
    }

    /// Find the pane ID at the given pixel position
    pub fn pane_at_position(&self, available: PaneRect, px: u32, py: u32) -> Option<PaneId> {
        match self {
            PaneNode::Leaf(ref pane) => {
                if px >= available.x
                    && px < available.x + available.width
                    && py >= available.y
                    && py < available.y + available.height
                {
                    Some(pane.id)
                } else {
                    None
                }
            }
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_rect, second_rect) = compute_split_rects(available, *direction, *ratio);
                first
                    .pane_at_position(first_rect, px, py)
                    .or_else(|| second.pane_at_position(second_rect, px, py))
            }
        }
    }

    /// Get the neighboring pane ID in the given direction from the target pane.
    /// Uses the computed rects to determine spatial relationships.
    pub fn navigate(
        &self,
        available: PaneRect,
        from_id: PaneId,
        nav_direction: NavDirection,
    ) -> Option<PaneId> {
        let rects = self.compute_rects(available);

        // Find the current pane's rect
        let from_rect = rects.iter().find(|(id, _)| *id == from_id)?.1;
        let from_center_x = from_rect.x + from_rect.width / 2;
        let from_center_y = from_rect.y + from_rect.height / 2;

        let mut best: Option<(PaneId, u32)> = None;

        for (id, rect) in &rects {
            if *id == from_id {
                continue;
            }
            let center_x = rect.x + rect.width / 2;
            let center_y = rect.y + rect.height / 2;

            let is_valid = match nav_direction {
                NavDirection::Left => center_x < from_center_x,
                NavDirection::Right => center_x > from_center_x,
                NavDirection::Up => center_y < from_center_y,
                NavDirection::Down => center_y > from_center_y,
            };

            if is_valid {
                let dist = match nav_direction {
                    NavDirection::Left | NavDirection::Right => {
                        let dx = (center_x as i64 - from_center_x as i64).unsigned_abs() as u32;
                        let dy = (center_y as i64 - from_center_y as i64).unsigned_abs() as u32;
                        dx + dy / 2
                    }
                    NavDirection::Up | NavDirection::Down => {
                        let dx = (center_x as i64 - from_center_x as i64).unsigned_abs() as u32;
                        let dy = (center_y as i64 - from_center_y as i64).unsigned_abs() as u32;
                        dy + dx / 2
                    }
                };

                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((*id, dist));
                }
            }
        }

        best.map(|(id, _)| id)
    }

    /// Resize all panes in the tree given the available area and cell size
    pub fn resize_all(&mut self, available: PaneRect, cell_width: f32, cell_height: f32) {
        match self {
            PaneNode::Leaf(pane) => {
                let cols = (available.width as f32 / cell_width) as usize;
                let rows = (available.height as f32 / cell_height) as usize;
                let cols = cols.max(1);
                let rows = rows.max(1);
                pane.terminal.resize(cols, rows);
                let _ = pane.child.resize(WindowSize::new(cols as u16, rows as u16));
            }
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_rect, second_rect) = compute_split_rects(available, *direction, *ratio);
                first.resize_all(first_rect, cell_width, cell_height);
                second.resize_all(second_rect, cell_width, cell_height);
            }
        }
    }
}

/// Navigation direction for pane switching
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDirection {
    Left,
    Right,
    Up,
    Down,
}

/// Compute the two child rects for a split
fn compute_split_rects(
    available: PaneRect,
    direction: SplitDirection,
    ratio: f64,
) -> (PaneRect, PaneRect) {
    match direction {
        SplitDirection::Vertical => {
            let first_width = ((available.width as f64 - DIVIDER_WIDTH as f64) * ratio) as u32;
            let second_width = available.width.saturating_sub(first_width + DIVIDER_WIDTH);
            (
                PaneRect::new(available.x, available.y, first_width, available.height),
                PaneRect::new(
                    available.x + first_width + DIVIDER_WIDTH,
                    available.y,
                    second_width,
                    available.height,
                ),
            )
        }
        SplitDirection::Horizontal => {
            let first_height = ((available.height as f64 - DIVIDER_WIDTH as f64) * ratio) as u32;
            let second_height = available
                .height
                .saturating_sub(first_height + DIVIDER_WIDTH);
            (
                PaneRect::new(available.x, available.y, available.width, first_height),
                PaneRect::new(
                    available.x,
                    available.y + first_height + DIVIDER_WIDTH,
                    available.width,
                    second_height,
                ),
            )
        }
    }
}

/// Compute the divider rect for a split
fn compute_divider_rect(available: PaneRect, direction: SplitDirection, ratio: f64) -> PaneRect {
    match direction {
        SplitDirection::Vertical => {
            let first_width = ((available.width as f64 - DIVIDER_WIDTH as f64) * ratio) as u32;
            PaneRect::new(
                available.x + first_width,
                available.y,
                DIVIDER_WIDTH,
                available.height,
            )
        }
        SplitDirection::Horizontal => {
            let first_height = ((available.height as f64 - DIVIDER_WIDTH as f64) * ratio) as u32;
            PaneRect::new(
                available.x,
                available.y + first_height,
                available.width,
                DIVIDER_WIDTH,
            )
        }
    }
}
