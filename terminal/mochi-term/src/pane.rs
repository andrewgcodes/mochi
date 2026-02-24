//! Split pane management
//!
//! Provides a binary tree structure for terminal multiplexing within a tab.
//! Each tab can contain a tree of panes arranged in horizontal (top/bottom)
//! or vertical (left/right) splits.

use crate::terminal::Terminal;
use terminal_pty::Child;

/// Unique identifier for a pane
pub type PaneId = u32;

/// Direction of a split
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// Panes arranged top and bottom
    Horizontal,
    /// Panes arranged left and right
    Vertical,
}

/// A rectangle in pixel coordinates
#[derive(Debug, Clone, Copy)]
pub struct PaneRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl PaneRect {
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    pub fn center(&self) -> (i32, i32) {
        (
            self.x as i32 + self.w as i32 / 2,
            self.y as i32 + self.h as i32 / 2,
        )
    }

    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

/// Information about a divider between panes
#[derive(Debug, Clone, Copy)]
pub struct DividerInfo {
    pub direction: SplitDirection,
    /// For Vertical split: x coordinate of the divider
    /// For Horizontal split: y coordinate of the divider
    pub position: u32,
    /// Start of the divider line (x for horizontal, y for vertical)
    pub start: u32,
    /// Length of the divider line
    pub length: u32,
    /// The parent rect that contains this split (used for drag calculations)
    pub parent_rect: PaneRect,
}

/// Width of the divider between panes in pixels
pub const DIVIDER_WIDTH: u32 = 4;

/// Minimum pane size in pixels (prevents panes from becoming too small)
const MIN_PANE_SIZE: u32 = 40;

/// Minimum ratio for a split (prevents near-zero splits)
const MIN_RATIO: f32 = 0.1;

/// Maximum ratio for a split
const MAX_RATIO: f32 = 0.9;

/// Direction for pane navigation
#[derive(Debug, Clone, Copy)]
pub enum NavigationDirection {
    Left,
    Right,
    Up,
    Down,
}

/// A single terminal pane (leaf node in the pane tree)
pub struct Pane {
    pub terminal: Terminal,
    pub child: Child,
    pub title: String,
    pub scroll_offset: usize,
    pub id: PaneId,
}

impl Pane {
    pub fn new(id: PaneId, terminal: Terminal, child: Child) -> Self {
        Self {
            terminal,
            child,
            title: String::from("Terminal"),
            scroll_offset: 0,
            id,
        }
    }
}

/// A node in the pane tree
pub enum PaneNode {
    /// Temporary placeholder used during tree restructuring
    Empty,
    /// A leaf node containing a terminal pane
    Leaf(Box<Pane>),
    /// A split containing two child nodes
    Split {
        direction: SplitDirection,
        /// Ratio of first child's size (0.0 to 1.0)
        ratio: f32,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

impl PaneNode {
    /// Create a new leaf node
    pub fn leaf(id: PaneId, terminal: Terminal, child: Child) -> Self {
        PaneNode::Leaf(Box::new(Pane::new(id, terminal, child)))
    }

    /// Check if a pane with the given ID exists in this subtree
    pub fn contains_pane(&self, id: PaneId) -> bool {
        match self {
            PaneNode::Leaf(pane) => pane.id == id,
            PaneNode::Split { first, second, .. } => {
                first.contains_pane(id) || second.contains_pane(id)
            }
            PaneNode::Empty => false,
        }
    }

    /// Find a pane by ID (immutable)
    pub fn find_pane(&self, id: PaneId) -> Option<&Pane> {
        match self {
            PaneNode::Leaf(pane) => {
                if pane.id == id {
                    Some(pane)
                } else {
                    None
                }
            }
            PaneNode::Split { first, second, .. } => {
                first.find_pane(id).or_else(|| second.find_pane(id))
            }
            PaneNode::Empty => None,
        }
    }

    /// Find a pane by ID (mutable)
    pub fn find_pane_mut(&mut self, id: PaneId) -> Option<&mut Pane> {
        match self {
            PaneNode::Leaf(pane) => {
                if pane.id == id {
                    Some(pane)
                } else {
                    None
                }
            }
            PaneNode::Split { first, second, .. } => {
                if first.contains_pane(id) {
                    first.find_pane_mut(id)
                } else {
                    second.find_pane_mut(id)
                }
            }
            PaneNode::Empty => None,
        }
    }

    /// Get all pane IDs in tree traversal order
    pub fn all_pane_ids(&self) -> Vec<PaneId> {
        match self {
            PaneNode::Leaf(pane) => vec![pane.id],
            PaneNode::Split { first, second, .. } => {
                let mut ids = first.all_pane_ids();
                ids.extend(second.all_pane_ids());
                ids
            }
            PaneNode::Empty => vec![],
        }
    }

    /// Count the number of leaf panes
    pub fn pane_count(&self) -> usize {
        match self {
            PaneNode::Leaf(_) => 1,
            PaneNode::Split { first, second, .. } => first.pane_count() + second.pane_count(),
            PaneNode::Empty => 0,
        }
    }

    /// Calculate the pixel rect for each leaf pane
    pub fn calculate_rects(&self, available: PaneRect) -> Vec<(PaneId, PaneRect)> {
        match self {
            PaneNode::Leaf(pane) => vec![(pane.id, available)],
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_rect, second_rect) = split_rect(available, *direction, *ratio);
                let mut rects = first.calculate_rects(first_rect);
                rects.extend(second.calculate_rects(second_rect));
                rects
            }
            PaneNode::Empty => vec![],
        }
    }

    /// Calculate divider positions with parent rect info for dragging
    pub fn calculate_dividers(&self, available: PaneRect) -> Vec<DividerInfo> {
        match self {
            PaneNode::Leaf(_) | PaneNode::Empty => vec![],
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_rect, second_rect) = split_rect(available, *direction, *ratio);

                let divider = match direction {
                    SplitDirection::Vertical => {
                        let div_x = available.x
                            + ((available.w as f32 - DIVIDER_WIDTH as f32) * ratio) as u32;
                        DividerInfo {
                            direction: SplitDirection::Vertical,
                            position: div_x,
                            start: available.y,
                            length: available.h,
                            parent_rect: available,
                        }
                    }
                    SplitDirection::Horizontal => {
                        let div_y = available.y
                            + ((available.h as f32 - DIVIDER_WIDTH as f32) * ratio) as u32;
                        DividerInfo {
                            direction: SplitDirection::Horizontal,
                            position: div_y,
                            start: available.x,
                            length: available.w,
                            parent_rect: available,
                        }
                    }
                };

                let mut dividers = vec![divider];
                dividers.extend(first.calculate_dividers(first_rect));
                dividers.extend(second.calculate_dividers(second_rect));
                dividers
            }
        }
    }

    /// Split the pane with the given ID, creating a new sibling pane.
    /// The existing pane becomes the first child, the new pane becomes the second.
    pub fn split_pane(
        &mut self,
        target_id: PaneId,
        direction: SplitDirection,
        new_pane: Pane,
    ) -> bool {
        let is_target = matches!(self, PaneNode::Leaf(p) if p.id == target_id);

        if is_target {
            let old_node = std::mem::replace(self, PaneNode::Empty);
            *self = PaneNode::Split {
                direction,
                ratio: 0.5,
                first: Box::new(old_node),
                second: Box::new(PaneNode::Leaf(Box::new(new_pane))),
            };
            return true;
        }

        match self {
            PaneNode::Split { first, second, .. } => {
                if first.contains_pane(target_id) {
                    first.split_pane(target_id, direction, new_pane)
                } else {
                    second.split_pane(target_id, direction, new_pane)
                }
            }
            _ => false,
        }
    }

    /// Remove a pane by ID. The sibling takes the parent's place.
    /// Returns true if the pane was removed.
    pub fn remove_pane(&mut self, target_id: PaneId) -> bool {
        match self {
            PaneNode::Split { first, second, .. } => {
                // Check if first child is the target leaf
                if matches!(first.as_ref(), PaneNode::Leaf(p) if p.id == target_id) {
                    let sibling = std::mem::replace(second.as_mut(), PaneNode::Empty);
                    *self = sibling;
                    return true;
                }
                // Check if second child is the target leaf
                if matches!(second.as_ref(), PaneNode::Leaf(p) if p.id == target_id) {
                    let sibling = std::mem::replace(first.as_mut(), PaneNode::Empty);
                    *self = sibling;
                    return true;
                }
                // Recurse into the subtree that contains the target
                if first.contains_pane(target_id) {
                    first.remove_pane(target_id)
                } else {
                    second.remove_pane(target_id)
                }
            }
            _ => false,
        }
    }

    /// Update the ratio of the nth split node (in DFS traversal order).
    /// Used for divider dragging where the divider index corresponds to
    /// the traversal order from calculate_dividers().
    pub fn update_nth_split_ratio(&mut self, n: &mut usize, new_ratio: f32) -> bool {
        match self {
            PaneNode::Split {
                ratio,
                first,
                second,
                ..
            } => {
                if *n == 0 {
                    *ratio = new_ratio.clamp(MIN_RATIO, MAX_RATIO);
                    return true;
                }
                *n -= 1;
                if first.update_nth_split_ratio(n, new_ratio) {
                    return true;
                }
                second.update_nth_split_ratio(n, new_ratio)
            }
            _ => false,
        }
    }

    /// Apply a function to every leaf pane (mutable)
    pub fn for_each_pane_mut<F: FnMut(&mut Pane)>(&mut self, f: &mut F) {
        match self {
            PaneNode::Leaf(pane) => f(pane),
            PaneNode::Split { first, second, .. } => {
                first.for_each_pane_mut(f);
                second.for_each_pane_mut(f);
            }
            PaneNode::Empty => {}
        }
    }

    /// Check if any child has exited. Returns IDs of exited panes.
    pub fn exited_pane_ids(&self) -> Vec<PaneId> {
        match self {
            PaneNode::Leaf(pane) => {
                if !pane.child.is_running() {
                    vec![pane.id]
                } else {
                    vec![]
                }
            }
            PaneNode::Split { first, second, .. } => {
                let mut ids = first.exited_pane_ids();
                ids.extend(second.exited_pane_ids());
                ids
            }
            PaneNode::Empty => vec![],
        }
    }
}

/// Split a rect into two sub-rects based on direction and ratio,
/// accounting for the divider width.
fn split_rect(available: PaneRect, direction: SplitDirection, ratio: f32) -> (PaneRect, PaneRect) {
    match direction {
        SplitDirection::Vertical => {
            let usable = available.w.saturating_sub(DIVIDER_WIDTH);
            let first_w = ((usable as f32) * ratio) as u32;
            let first_w = first_w
                .max(MIN_PANE_SIZE)
                .min(usable.saturating_sub(MIN_PANE_SIZE));
            let second_x = available.x + first_w + DIVIDER_WIDTH;
            let second_w = available.w.saturating_sub(first_w + DIVIDER_WIDTH);
            (
                PaneRect::new(available.x, available.y, first_w, available.h),
                PaneRect::new(second_x, available.y, second_w, available.h),
            )
        }
        SplitDirection::Horizontal => {
            let usable = available.h.saturating_sub(DIVIDER_WIDTH);
            let first_h = ((usable as f32) * ratio) as u32;
            let first_h = first_h
                .max(MIN_PANE_SIZE)
                .min(usable.saturating_sub(MIN_PANE_SIZE));
            let second_y = available.y + first_h + DIVIDER_WIDTH;
            let second_h = available.h.saturating_sub(first_h + DIVIDER_WIDTH);
            (
                PaneRect::new(available.x, available.y, available.w, first_h),
                PaneRect::new(available.x, second_y, available.w, second_h),
            )
        }
    }
}

/// Find the pane in a given direction from the active pane.
/// Uses center-point comparison and Manhattan distance for tie-breaking.
pub fn find_pane_in_direction(
    rects: &[(PaneId, PaneRect)],
    active_id: PaneId,
    direction: NavigationDirection,
) -> Option<PaneId> {
    let active_rect = rects.iter().find(|(id, _)| *id == active_id)?.1;
    let (cx, cy) = active_rect.center();

    rects
        .iter()
        .filter(|(id, _)| *id != active_id)
        .filter(|(_, rect)| {
            let (rx, ry) = rect.center();
            match direction {
                NavigationDirection::Left => rx < cx,
                NavigationDirection::Right => rx > cx,
                NavigationDirection::Up => ry < cy,
                NavigationDirection::Down => ry > cy,
            }
        })
        .min_by_key(|(_, rect)| {
            let (rx, ry) = rect.center();
            let dx = (rx - cx).abs();
            let dy = (ry - cy).abs();
            dx + dy // Manhattan distance
        })
        .map(|(id, _)| *id)
}

/// Check if a pixel position is near a divider (within DIVIDER_WIDTH)
pub fn divider_at_position(dividers: &[DividerInfo], px: u32, py: u32) -> Option<usize> {
    for (i, div) in dividers.iter().enumerate() {
        let hit = match div.direction {
            SplitDirection::Vertical => {
                px >= div.position
                    && px < div.position + DIVIDER_WIDTH
                    && py >= div.start
                    && py < div.start + div.length
            }
            SplitDirection::Horizontal => {
                py >= div.position
                    && py < div.position + DIVIDER_WIDTH
                    && px >= div.start
                    && px < div.start + div.length
            }
        };
        if hit {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pane_rect_contains() {
        let rect = PaneRect::new(10, 20, 100, 50);
        assert!(rect.contains(10, 20));
        assert!(rect.contains(50, 40));
        assert!(!rect.contains(9, 20));
        assert!(!rect.contains(110, 20));
    }

    #[test]
    fn test_pane_rect_center() {
        let rect = PaneRect::new(0, 0, 100, 50);
        assert_eq!(rect.center(), (50, 25));
    }

    #[test]
    fn test_split_rect_vertical() {
        let available = PaneRect::new(0, 0, 200, 100);
        let (first, second) = split_rect(available, SplitDirection::Vertical, 0.5);
        assert_eq!(first.x, 0);
        assert_eq!(first.w + DIVIDER_WIDTH + second.w, 200);
        assert_eq!(second.x, first.w + DIVIDER_WIDTH);
    }

    #[test]
    fn test_split_rect_horizontal() {
        let available = PaneRect::new(0, 0, 200, 100);
        let (first, second) = split_rect(available, SplitDirection::Horizontal, 0.5);
        assert_eq!(first.y, 0);
        assert_eq!(first.h + DIVIDER_WIDTH + second.h, 100);
        assert_eq!(second.y, first.h + DIVIDER_WIDTH);
    }
}
