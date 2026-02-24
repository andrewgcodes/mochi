//! Split pane support for terminal multiplexing
//!
//! Provides a tree-based pane layout where each node is either a leaf
//! (containing a terminal + PTY) or a split (dividing space between two children).

use terminal_pty::Child;

use crate::terminal::Terminal;

/// Unique identifier for a pane
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneId(u64);

impl PaneId {
    /// Create a new unique pane ID
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        PaneId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Direction of a split
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// Side by side (left | right)
    Vertical,
    /// Stacked (top / bottom)
    Horizontal,
}

/// A rectangle in pixel coordinates
#[derive(Debug, Clone, Copy)]
pub struct PaneRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Width of the divider between panes in pixels
pub const DIVIDER_WIDTH: u32 = 2;

/// A leaf pane containing a terminal and child process
pub struct LeafPane {
    pub id: PaneId,
    pub terminal: Terminal,
    pub child: Child,
    pub title: String,
    pub scroll_offset: usize,
}

/// A node in the pane tree
pub enum PaneNode {
    /// A leaf node containing a terminal
    Leaf(Box<LeafPane>),
    /// A split node containing two children
    Split {
        direction: SplitDirection,
        /// Split ratio (0.0 to 1.0) - fraction of space given to first
        ratio: f32,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

/// Information about a leaf pane and its layout
pub struct PaneLayout {
    pub id: PaneId,
    pub rect: PaneRect,
}

/// Information about a divider between panes
pub struct DividerLayout {
    #[allow(dead_code)]
    pub direction: SplitDirection,
    pub rect: PaneRect,
}

/// Navigation direction for pane switching
#[derive(Debug, Clone, Copy)]
pub enum NavDirection {
    Left,
    Right,
    Up,
    Down,
}

impl PaneNode {
    /// Create a new leaf node
    pub fn new_leaf(terminal: Terminal, child: Child) -> Self {
        PaneNode::Leaf(Box::new(LeafPane {
            id: PaneId::new(),
            terminal,
            child,
            title: String::from("Terminal"),
            scroll_offset: 0,
        }))
    }

    /// Get the first leaf ID in the tree
    pub fn first_leaf_id(&self) -> PaneId {
        match self {
            PaneNode::Leaf(leaf) => leaf.id,
            PaneNode::Split { first, .. } => first.first_leaf_id(),
        }
    }

    /// Get a reference to a leaf pane by ID
    pub fn find_leaf(&self, id: PaneId) -> Option<&LeafPane> {
        match self {
            PaneNode::Leaf(leaf) => {
                if leaf.id == id {
                    Some(leaf)
                } else {
                    None
                }
            }
            PaneNode::Split { first, second, .. } => {
                first.find_leaf(id).or_else(|| second.find_leaf(id))
            }
        }
    }

    /// Get a mutable reference to a leaf pane by ID
    pub fn find_leaf_mut(&mut self, id: PaneId) -> Option<&mut LeafPane> {
        match self {
            PaneNode::Leaf(leaf) => {
                if leaf.id == id {
                    Some(leaf)
                } else {
                    None
                }
            }
            PaneNode::Split { first, second, .. } => {
                if let Some(leaf) = first.find_leaf_mut(id) {
                    Some(leaf)
                } else {
                    second.find_leaf_mut(id)
                }
            }
        }
    }

    /// Compute layoutrectangles for all leaf panes
    pub fn compute_layout(&self, rect: PaneRect) -> (Vec<PaneLayout>, Vec<DividerLayout>) {
        let mut panes = Vec::new();
        let mut dividers = Vec::new();
        self.compute_layout_inner(rect, &mut panes, &mut dividers);
        (panes, dividers)
    }

    fn compute_layout_inner(
        &self,
        rect: PaneRect,
        panes: &mut Vec<PaneLayout>,
        dividers: &mut Vec<DividerLayout>,
    ) {
        match self {
            PaneNode::Leaf(leaf) => {
                panes.push(PaneLayout { id: leaf.id, rect });
            }
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_rect, divider_rect, second_rect) = split_rect(rect, *direction, *ratio);
                dividers.push(DividerLayout {
                    direction: *direction,
                    rect: divider_rect,
                });
                first.compute_layout_inner(first_rect, panes, dividers);
                second.compute_layout_inner(second_rect, panes, dividers);
            }
        }
    }

    /// Check if this tree contains a leaf with the given ID
    pub fn contains_leaf(&self, id: PaneId) -> bool {
        match self {
            PaneNode::Leaf(leaf) => leaf.id == id,
            PaneNode::Split { first, second, .. } => {
                first.contains_leaf(id) || second.contains_leaf(id)
            }
        }
    }

    /// Split the leaf pane with the given target_id.
    /// The existing content becomes the first child, the new pane the second.
    /// Returns the new pane ID on success.
    pub fn split_pane(
        &mut self,
        target_id: PaneId,
        direction: SplitDirection,
        new_terminal: Terminal,
        new_child: Child,
    ) -> Option<PaneId> {
        if let PaneNode::Leaf(leaf) = self {
            if leaf.id == target_id {
                let new_leaf = PaneNode::new_leaf(new_terminal, new_child);
                let new_id = match &new_leaf {
                    PaneNode::Leaf(l) => l.id,
                    _ => unreachable!(),
                };

                // Use unsafe ptr swap to avoid needing a dummy PaneNode.
                // Safety: we read self out, build the split, write it back.
                // No aliasing occurs because we have exclusive &mut self.
                unsafe {
                    let self_ptr = self as *mut PaneNode;
                    let old_self = std::ptr::read(self_ptr);
                    let split = PaneNode::Split {
                        direction,
                        ratio: 0.5,
                        first: Box::new(old_self),
                        second: Box::new(new_leaf),
                    };
                    std::ptr::write(self_ptr, split);
                }
                return Some(new_id);
            }
        }

        // Recurse into split children
        if let PaneNode::Split { first, second, .. } = self {
            if first.contains_leaf(target_id) {
                return first.split_pane(target_id, direction, new_terminal, new_child);
            }
            if second.contains_leaf(target_id) {
                return second.split_pane(target_id, direction, new_terminal, new_child);
            }
        }
        None
    }

    /// Remove a leaf pane by ID. The sibling takes over the parent position.
    /// Returns true if removal was successful.
    pub fn remove_pane(&mut self, target_id: PaneId) -> bool {
        if let PaneNode::Split { first, second, .. } = self {
            let first_is_target = matches!(first.as_ref(), PaneNode::Leaf(l) if l.id == target_id);
            let second_is_target =
                matches!(second.as_ref(), PaneNode::Leaf(l) if l.id == target_id);

            if first_is_target || second_is_target {
                // Replace self with the sibling using unsafe ptr swap
                unsafe {
                    let self_ptr = self as *mut PaneNode;
                    let old_split = std::ptr::read(self_ptr);
                    if let PaneNode::Split {
                        first: f,
                        second: s,
                        ..
                    } = old_split
                    {
                        let keeper = if first_is_target { *s } else { *f };
                        std::ptr::write(self_ptr, keeper);
                    }
                }
                return true;
            }

            // Recurse
            return first.remove_pane(target_id) || second.remove_pane(target_id);
        }
        false
    }

    /// Call a function on every leaf pane mutably
    pub fn for_each_leaf_mut<F: FnMut(&mut LeafPane)>(&mut self, f: &mut F) {
        match self {
            PaneNode::Leaf(leaf) => f(leaf.as_mut()),
            PaneNode::Split { first, second, .. } => {
                first.for_each_leaf_mut(f);
                second.for_each_leaf_mut(f);
            }
        }
    }

    /// Call a function on every leaf pane
    pub fn for_each_leaf<F: FnMut(&LeafPane)>(&self, f: &mut F) {
        match self {
            PaneNode::Leaf(leaf) => f(leaf.as_ref()),
            PaneNode::Split { first, second, .. } => {
                first.for_each_leaf(f);
                second.for_each_leaf(f);
            }
        }
    }

    /// Check if any leaf pane child has exited, and remove dead panes.
    /// Returns list of IDs that were removed.
    pub fn remove_dead_panes(&mut self) -> Vec<PaneId> {
        let mut dead_ids = Vec::new();
        self.for_each_leaf(&mut |leaf| {
            if !leaf.child.is_running() {
                dead_ids.push(leaf.id);
            }
        });
        for id in &dead_ids {
            self.remove_pane(*id);
        }
        dead_ids
    }

    /// Find which leaf pane contains the given pixel coordinate
    pub fn find_pane_at(&self, rect: PaneRect, px: u32, py: u32) -> Option<PaneId> {
        match self {
            PaneNode::Leaf(leaf) => {
                if px >= rect.x
                    && px < rect.x + rect.width
                    && py >= rect.y
                    && py < rect.y + rect.height
                {
                    Some(leaf.id)
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
                let (first_rect, _, second_rect) = split_rect(rect, *direction, *ratio);
                first
                    .find_pane_at(first_rect, px, py)
                    .or_else(|| second.find_pane_at(second_rect, px, py))
            }
        }
    }

    /// Navigate to the next pane in the given direction
    pub fn navigate(
        &self,
        active_id: PaneId,
        rect: PaneRect,
        nav_direction: NavDirection,
    ) -> Option<PaneId> {
        let (pane_layouts, _) = self.compute_layout(rect);

        let active_layout = pane_layouts.iter().find(|p| p.id == active_id)?;
        let acx = active_layout.rect.x + active_layout.rect.width / 2;
        let acy = active_layout.rect.y + active_layout.rect.height / 2;

        let mut best: Option<(PaneId, u32)> = None;

        for layout in &pane_layouts {
            if layout.id == active_id {
                continue;
            }

            let cx = layout.rect.x + layout.rect.width / 2;
            let cy = layout.rect.y + layout.rect.height / 2;

            let is_valid = match nav_direction {
                NavDirection::Left => cx < acx,
                NavDirection::Right => cx > acx,
                NavDirection::Up => cy < acy,
                NavDirection::Down => cy > acy,
            };

            if !is_valid {
                continue;
            }

            let dist = match nav_direction {
                NavDirection::Left | NavDirection::Right => {
                    let dx = (cx as i64 - acx as i64).unsigned_abs() as u32;
                    let dy = (cy as i64 - acy as i64).unsigned_abs() as u32;
                    dx + dy / 2
                }
                NavDirection::Up | NavDirection::Down => {
                    let dx = (cx as i64 - acx as i64).unsigned_abs() as u32;
                    let dy = (cy as i64 - acy as i64).unsigned_abs() as u32;
                    dy + dx / 2
                }
            };

            if best.is_none() || dist < best.unwrap().1 {
                best = Some((layout.id, dist));
            }
        }

        best.map(|(id, _)| id)
    }

    /// Count the number of leaf panes
    pub fn leaf_count(&self) -> usize {
        match self {
            PaneNode::Leaf(_) => 1,
            PaneNode::Split { first, second, .. } => first.leaf_count() + second.leaf_count(),
        }
    }
}

/// Split a rectangle into two parts with a divider between them
fn split_rect(
    rect: PaneRect,
    direction: SplitDirection,
    ratio: f32,
) -> (PaneRect, PaneRect, PaneRect) {
    match direction {
        SplitDirection::Vertical => {
            let available = rect.width.saturating_sub(DIVIDER_WIDTH);
            let first_width = (available as f32 * ratio) as u32;
            let second_width = available.saturating_sub(first_width);

            let first_r = PaneRect {
                x: rect.x,
                y: rect.y,
                width: first_width,
                height: rect.height,
            };
            let divider = PaneRect {
                x: rect.x + first_width,
                y: rect.y,
                width: DIVIDER_WIDTH,
                height: rect.height,
            };
            let second_r = PaneRect {
                x: rect.x + first_width + DIVIDER_WIDTH,
                y: rect.y,
                width: second_width,
                height: rect.height,
            };
            (first_r, divider, second_r)
        }
        SplitDirection::Horizontal => {
            let available = rect.height.saturating_sub(DIVIDER_WIDTH);
            let first_height = (available as f32 * ratio) as u32;
            let second_height = available.saturating_sub(first_height);

            let first_r = PaneRect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: first_height,
            };
            let divider = PaneRect {
                x: rect.x,
                y: rect.y + first_height,
                width: rect.width,
                height: DIVIDER_WIDTH,
            };
            let second_r = PaneRect {
                x: rect.x,
                y: rect.y + first_height + DIVIDER_WIDTH,
                width: rect.width,
                height: second_height,
            };
            (first_r, divider, second_r)
        }
    }
}
