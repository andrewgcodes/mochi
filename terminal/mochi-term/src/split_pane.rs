//! Split pane management for the terminal
//!
//! Provides a binary tree structure for managing split panes within a tab.
//! Each leaf node contains a terminal + child process, and internal nodes
//! represent horizontal or vertical splits with a draggable divider.

use terminal_pty::Child;

use crate::terminal::Terminal;

/// Width of the divider between split panes in pixels
pub const DIVIDER_SIZE: u32 = 4;

/// Minimum pane size in pixels (prevents panes from being too small)
pub const MIN_PANE_SIZE: u32 = 50;

/// Direction of a split
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// Side by side (left | right)
    Horizontal,
    /// Top and bottom (top / bottom)
    Vertical,
}

/// A unique identifier for a pane leaf node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneId(pub u64);

/// A rectangular region in pixels
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a point (px, py) is inside this rect
    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}

/// Data stored in a leaf pane
pub struct PaneLeaf {
    pub id: PaneId,
    pub terminal: Terminal,
    pub child: Child,
    pub title: String,
    pub scroll_offset: usize,
}

/// A node in the split pane tree
pub enum PaneNode {
    /// A terminal pane (leaf)
    Leaf(Box<PaneLeaf>),
    /// A split containing two sub-panes
    Split {
        direction: SplitDirection,
        /// Position of the divider as a ratio (0.0 to 1.0)
        ratio: f64,
        /// First child (left or top)
        first: Box<PaneNode>,
        /// Second child (right or bottom)
        second: Box<PaneNode>,
    },
    /// Temporary placeholder used during tree restructuring. Never visible externally.
    #[doc(hidden)]
    Placeholder,
}

/// Information about a divider for hit-testing
#[derive(Debug, Clone)]
pub struct DividerInfo {
    /// The rectangle of the divider in pixels
    pub rect: Rect,
    /// Direction of the split this divider belongs to
    pub direction: SplitDirection,
    /// Path to this split node in the tree (sequence of choices: false=first, true=second)
    pub path: Vec<bool>,
}

/// Information about a pane's layout
#[derive(Debug, Clone)]
pub struct PaneLayout {
    pub id: PaneId,
    pub rect: Rect,
}

impl PaneNode {
    /// Create a new leaf pane
    pub fn new_leaf(id: PaneId, terminal: Terminal, child: Child) -> Self {
        PaneNode::Leaf(Box::new(PaneLeaf {
            id,
            terminal,
            child,
            title: String::from("Terminal"),
            scroll_offset: 0,
        }))
    }

    /// Create a split node from two existing pane nodes
    pub fn new_split(
        direction: SplitDirection,
        ratio: f64,
        first: PaneNode,
        second: PaneNode,
    ) -> Self {
        PaneNode::Split {
            direction,
            ratio,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    /// Check if this node is a leaf
    #[allow(dead_code)]
    pub fn is_leaf(&self) -> bool {
        matches!(self, PaneNode::Leaf(_))
    }

    /// Get the leaf data if this is a leaf
    #[allow(dead_code)]
    pub fn as_leaf(&self) -> Option<&PaneLeaf> {
        match self {
            PaneNode::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    /// Get mutable leaf data if this is a leaf
    #[allow(dead_code)]
    pub fn as_leaf_mut(&mut self) -> Option<&mut PaneLeaf> {
        match self {
            PaneNode::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    /// Find a leaf by ID and return a reference
    pub fn find_leaf(&self, id: PaneId) -> Option<&PaneLeaf> {
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
            PaneNode::Placeholder => None,
        }
    }

    /// Find a leaf by ID and return a mutable reference
    pub fn find_leaf_mut(&mut self, id: PaneId) -> Option<&mut PaneLeaf> {
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
            PaneNode::Placeholder => None,
        }
    }

    /// Collect all leaf IDs in tree order (left-to-right / top-to-bottom)
    pub fn collect_leaf_ids(&self) -> Vec<PaneId> {
        let mut ids = Vec::new();
        self.collect_leaf_ids_inner(&mut ids);
        ids
    }

    fn collect_leaf_ids_inner(&self, ids: &mut Vec<PaneId>) {
        match self {
            PaneNode::Leaf(leaf) => ids.push(leaf.id),
            PaneNode::Split { first, second, .. } => {
                first.collect_leaf_ids_inner(ids);
                second.collect_leaf_ids_inner(ids);
            }
            PaneNode::Placeholder => {}
        }
    }

    /// Get the first leaf in tree order
    #[allow(dead_code)]
    pub fn first_leaf_id(&self) -> PaneId {
        match self {
            PaneNode::Leaf(leaf) => leaf.id,
            PaneNode::Split { first, .. } => first.first_leaf_id(),
            PaneNode::Placeholder => PaneId(0),
        }
    }

    /// Count total leaf panes
    pub fn leaf_count(&self) -> usize {
        match self {
            PaneNode::Leaf(_) => 1,
            PaneNode::Split { first, second, .. } => first.leaf_count() + second.leaf_count(),
            PaneNode::Placeholder => 0,
        }
    }

    /// Check if a leaf with the given ID exists in this tree
    pub fn contains_leaf(&self, id: PaneId) -> bool {
        match self {
            PaneNode::Leaf(leaf) => leaf.id == id,
            PaneNode::Split { first, second, .. } => {
                first.contains_leaf(id) || second.contains_leaf(id)
            }
            PaneNode::Placeholder => false,
        }
    }

    /// Split the leaf identified by `target_id`. The current leaf becomes the first child,
    /// and `new_leaf_node` becomes the second child in a new split.
    ///
    /// This uses `contains_leaf` to route recursion correctly, avoiding move issues.
    pub fn split_pane(
        &mut self,
        target_id: PaneId,
        direction: SplitDirection,
        new_leaf_node: PaneNode,
    ) -> bool {
        // If this is the target leaf, replace self in-place with a split
        if let PaneNode::Leaf(leaf) = &self {
            if leaf.id == target_id {
                // Use Placeholder as a lightweight temporary value during the swap
                let old_self = std::mem::replace(self, PaneNode::Placeholder);
                *self = PaneNode::new_split(direction, 0.5, old_self, new_leaf_node);
                return true;
            }
            return false;
        }

        // Recurse into split children using contains_leaf to avoid move issues
        if let PaneNode::Split { first, second, .. } = self {
            if first.contains_leaf(target_id) {
                return first.split_pane(target_id, direction, new_leaf_node);
            }
            if second.contains_leaf(target_id) {
                return second.split_pane(target_id, direction, new_leaf_node);
            }
        }

        false
    }

    /// Remove a pane by ID. The sibling takes the place of the parent split.
    /// Returns true if the pane was found and removed.
    pub fn remove_pane(&mut self, target_id: PaneId) -> bool {
        if let PaneNode::Split { first, second, .. } = self {
            // Check if first child is the target leaf
            if matches!(first.as_ref(), PaneNode::Leaf(leaf) if leaf.id == target_id) {
                // Use Placeholder as lightweight temporary during swap
                let replacement = std::mem::replace(second.as_mut(), PaneNode::Placeholder);
                *self = replacement;
                return true;
            }

            // Check if second child is the target leaf
            if matches!(second.as_ref(), PaneNode::Leaf(leaf) if leaf.id == target_id) {
                let replacement = std::mem::replace(first.as_mut(), PaneNode::Placeholder);
                *self = replacement;
                return true;
            }

            // Recurse into children
            if first.remove_pane(target_id) {
                return true;
            }
            return second.remove_pane(target_id);
        }

        false
    }

    /// Calculate the layout of all panes given the available rectangle.
    /// Returns a list of PaneLayout for each leaf pane, plus divider info.
    pub fn calculate_layout(&self, available: Rect) -> (Vec<PaneLayout>, Vec<DividerInfo>) {
        let mut panes = Vec::new();
        let mut dividers = Vec::new();
        let mut path = Vec::new();
        self.calculate_layout_inner(available, &mut panes, &mut dividers, &mut path);
        (panes, dividers)
    }

    fn calculate_layout_inner(
        &self,
        available: Rect,
        panes: &mut Vec<PaneLayout>,
        dividers: &mut Vec<DividerInfo>,
        path: &mut Vec<bool>,
    ) {
        match self {
            PaneNode::Leaf(leaf) => {
                panes.push(PaneLayout {
                    id: leaf.id,
                    rect: available,
                });
            }
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_rect, divider_rect, second_rect) =
                    compute_split_rects(available, *direction, *ratio);

                dividers.push(DividerInfo {
                    rect: divider_rect,
                    direction: *direction,
                    path: path.clone(),
                });

                path.push(false);
                first.calculate_layout_inner(first_rect, panes, dividers, path);
                path.pop();

                path.push(true);
                second.calculate_layout_inner(second_rect, panes, dividers, path);
                path.pop();
            }
            PaneNode::Placeholder => {}
        }
    }

    /// Resize all terminal panes to match their layout rectangles
    pub fn resize_to_layout(&mut self, available: Rect, cell_width: f32, cell_height: f32) {
        match self {
            PaneNode::Leaf(leaf) => {
                let cols = (available.width as f32 / cell_width) as usize;
                let rows = (available.height as f32 / cell_height) as usize;
                if cols > 0 && rows > 0 {
                    leaf.terminal.resize(cols, rows);
                    let _ = leaf
                        .child
                        .resize(terminal_pty::WindowSize::new(cols as u16, rows as u16));
                }
            }
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_rect, _divider_rect, second_rect) =
                    compute_split_rects(available, *direction, *ratio);
                first.resize_to_layout(first_rect, cell_width, cell_height);
                second.resize_to_layout(second_rect, cell_width, cell_height);
            }
            PaneNode::Placeholder => {}
        }
    }

    /// Update the split ratio at a given path in the tree.
    pub fn update_ratio_at_path(&mut self, path: &[bool], new_ratio: f64) -> bool {
        match (path.first(), self) {
            (None, PaneNode::Split { ratio, .. }) => {
                *ratio = new_ratio.clamp(0.1, 0.9);
                true
            }
            (Some(&choice), PaneNode::Split { first, second, .. }) => {
                if choice {
                    second.update_ratio_at_path(&path[1..], new_ratio)
                } else {
                    first.update_ratio_at_path(&path[1..], new_ratio)
                }
            }
            _ => false,
        }
    }

    /// Find which pane contains the given pixel position
    pub fn find_pane_at(&self, available: Rect, px: u32, py: u32) -> Option<PaneId> {
        match self {
            PaneNode::Leaf(leaf) => {
                if available.contains(px, py) {
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
                let (first_rect, _div_rect, second_rect) =
                    compute_split_rects(available, *direction, *ratio);
                first
                    .find_pane_at(first_rect, px, py)
                    .or_else(|| second.find_pane_at(second_rect, px, py))
            }
            PaneNode::Placeholder => None,
        }
    }

    /// Get the next pane ID after the given one (cycling through leaves in order)
    pub fn next_pane_id(&self, current: PaneId) -> PaneId {
        let ids = self.collect_leaf_ids();
        if ids.len() <= 1 {
            return current;
        }
        let pos = ids.iter().position(|&id| id == current).unwrap_or(0);
        ids[(pos + 1) % ids.len()]
    }

    /// Get the previous pane ID before the given one
    pub fn prev_pane_id(&self, current: PaneId) -> PaneId {
        let ids = self.collect_leaf_ids();
        if ids.len() <= 1 {
            return current;
        }
        let pos = ids.iter().position(|&id| id == current).unwrap_or(0);
        if pos == 0 {
            ids[ids.len() - 1]
        } else {
            ids[pos - 1]
        }
    }

    /// Iterate over all leaves mutably (for polling PTY output, etc.)
    pub fn for_each_leaf_mut<F: FnMut(&mut PaneLeaf)>(&mut self, f: &mut F) {
        match self {
            PaneNode::Leaf(leaf) => f(leaf),
            PaneNode::Split { first, second, .. } => {
                first.for_each_leaf_mut(f);
                second.for_each_leaf_mut(f);
            }
            PaneNode::Placeholder => {}
        }
    }

    /// Iterate over all leaves immutably
    pub fn for_each_leaf<F: FnMut(&PaneLeaf)>(&self, f: &mut F) {
        match self {
            PaneNode::Leaf(leaf) => f(leaf),
            PaneNode::Split { first, second, .. } => {
                first.for_each_leaf(f);
                second.for_each_leaf(f);
            }
            PaneNode::Placeholder => {}
        }
    }

    /// Check if any leaf's child process has exited. Returns list of dead pane IDs.
    pub fn dead_pane_ids(&self) -> Vec<PaneId> {
        let mut dead = Vec::new();
        self.for_each_leaf(&mut |leaf| {
            if !leaf.child.is_running() {
                dead.push(leaf.id);
            }
        });
        dead
    }

    /// Check if any child is still running
    pub fn any_child_running(&self) -> bool {
        match self {
            PaneNode::Leaf(leaf) => leaf.child.is_running(),
            PaneNode::Split { first, second, .. } => {
                first.any_child_running() || second.any_child_running()
            }
            PaneNode::Placeholder => false,
        }
    }
}

/// Compute the rectangles for first child, divider, and second child of a split
pub fn compute_split_rects(
    available: Rect,
    direction: SplitDirection,
    ratio: f64,
) -> (Rect, Rect, Rect) {
    match direction {
        SplitDirection::Horizontal => {
            let total_width = available.width.saturating_sub(DIVIDER_SIZE);
            let first_width = (total_width as f64 * ratio) as u32;
            let second_width = total_width.saturating_sub(first_width);

            let first = Rect::new(available.x, available.y, first_width, available.height);
            let divider = Rect::new(
                available.x + first_width,
                available.y,
                DIVIDER_SIZE,
                available.height,
            );
            let second = Rect::new(
                available.x + first_width + DIVIDER_SIZE,
                available.y,
                second_width,
                available.height,
            );
            (first, divider, second)
        }
        SplitDirection::Vertical => {
            let total_height = available.height.saturating_sub(DIVIDER_SIZE);
            let first_height = (total_height as f64 * ratio) as u32;
            let second_height = total_height.saturating_sub(first_height);

            let first = Rect::new(available.x, available.y, available.width, first_height);
            let divider = Rect::new(
                available.x,
                available.y + first_height,
                available.width,
                DIVIDER_SIZE,
            );
            let second = Rect::new(
                available.x,
                available.y + first_height + DIVIDER_SIZE,
                available.width,
                second_height,
            );
            (first, divider, second)
        }
    }
}

/// Counter for generating unique pane IDs
pub struct PaneIdCounter {
    next: u64,
}

impl PaneIdCounter {
    pub fn new() -> Self {
        Self { next: 1 }
    }

    pub fn next_id(&mut self) -> PaneId {
        let id = PaneId(self.next);
        self.next += 1;
        id
    }
}

impl Default for PaneIdCounter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10, 20, 100, 50);
        assert!(rect.contains(10, 20));
        assert!(rect.contains(50, 40));
        assert!(rect.contains(109, 69));
        assert!(!rect.contains(9, 20));
        assert!(!rect.contains(10, 19));
        assert!(!rect.contains(110, 20));
        assert!(!rect.contains(10, 70));
    }

    #[test]
    fn test_rect_contains_zero_origin() {
        let rect = Rect::new(0, 0, 800, 600);
        assert!(rect.contains(0, 0));
        assert!(rect.contains(400, 300));
        assert!(rect.contains(799, 599));
        assert!(!rect.contains(800, 0));
        assert!(!rect.contains(0, 600));
    }

    #[test]
    fn test_compute_split_rects_horizontal() {
        let available = Rect::new(0, 0, 800, 600);
        let (first, divider, second) =
            compute_split_rects(available, SplitDirection::Horizontal, 0.5);

        let total_usable = 800 - DIVIDER_SIZE;
        let first_w = (total_usable as f64 * 0.5) as u32;

        assert_eq!(first.x, 0);
        assert_eq!(first.width, first_w);
        assert_eq!(first.height, 600);

        assert_eq!(divider.x, first_w);
        assert_eq!(divider.width, DIVIDER_SIZE);
        assert_eq!(divider.height, 600);

        assert_eq!(second.x, first_w + DIVIDER_SIZE);
        assert_eq!(second.width, total_usable - first_w);
        assert_eq!(second.height, 600);
    }

    #[test]
    fn test_compute_split_rects_vertical() {
        let available = Rect::new(0, 0, 800, 600);
        let (first, divider, second) =
            compute_split_rects(available, SplitDirection::Vertical, 0.5);

        let total_usable = 600 - DIVIDER_SIZE;
        let first_h = (total_usable as f64 * 0.5) as u32;

        assert_eq!(first.y, 0);
        assert_eq!(first.height, first_h);
        assert_eq!(first.width, 800);

        assert_eq!(divider.y, first_h);
        assert_eq!(divider.height, DIVIDER_SIZE);
        assert_eq!(divider.width, 800);

        assert_eq!(second.y, first_h + DIVIDER_SIZE);
        assert_eq!(second.height, total_usable - first_h);
        assert_eq!(second.width, 800);
    }

    #[test]
    fn test_compute_split_rects_with_offset() {
        let available = Rect::new(100, 50, 400, 300);
        let (first, divider, _second) =
            compute_split_rects(available, SplitDirection::Horizontal, 0.5);

        assert_eq!(first.x, 100);
        assert_eq!(first.y, 50);

        let total_usable = 400 - DIVIDER_SIZE;
        let first_w = (total_usable as f64 * 0.5) as u32;
        assert_eq!(first.width, first_w);

        assert_eq!(divider.x, 100 + first_w);
        assert_eq!(divider.y, 50);
    }

    #[test]
    fn test_compute_split_rects_uneven_ratio() {
        let available = Rect::new(0, 0, 1000, 500);
        let (first, _divider, second) =
            compute_split_rects(available, SplitDirection::Horizontal, 0.3);

        let total_usable = 1000 - DIVIDER_SIZE;
        let first_w = (total_usable as f64 * 0.3) as u32;
        assert_eq!(first.width, first_w);
        assert_eq!(second.width, total_usable - first_w);
    }

    #[test]
    fn test_pane_id_counter() {
        let mut counter = PaneIdCounter::new();
        assert_eq!(counter.next_id(), PaneId(1));
        assert_eq!(counter.next_id(), PaneId(2));
        assert_eq!(counter.next_id(), PaneId(3));
    }

    #[test]
    fn test_pane_id_counter_default() {
        let mut counter = PaneIdCounter::default();
        assert_eq!(counter.next_id(), PaneId(1));
    }

    #[test]
    fn test_divider_size_constant() {
        assert_eq!(DIVIDER_SIZE, 4);
    }

    #[test]
    fn test_min_pane_size_constant() {
        assert_eq!(MIN_PANE_SIZE, 50);
    }

    #[test]
    fn test_split_direction_equality() {
        assert_eq!(SplitDirection::Horizontal, SplitDirection::Horizontal);
        assert_eq!(SplitDirection::Vertical, SplitDirection::Vertical);
        assert_ne!(SplitDirection::Horizontal, SplitDirection::Vertical);
    }

    #[test]
    fn test_pane_id_equality() {
        assert_eq!(PaneId(1), PaneId(1));
        assert_ne!(PaneId(1), PaneId(2));
    }

    #[test]
    fn test_rect_new() {
        let rect = Rect::new(10, 20, 30, 40);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.width, 30);
        assert_eq!(rect.height, 40);
    }

    #[test]
    fn test_split_rects_add_up_horizontal() {
        for ratio_pct in &[10u32, 25, 33, 50, 67, 75, 90] {
            let ratio = *ratio_pct as f64 / 100.0;
            let available = Rect::new(0, 0, 1000, 500);
            let (first, divider, second) =
                compute_split_rects(available, SplitDirection::Horizontal, ratio);

            let total = first.width + divider.width + second.width;
            assert_eq!(
                total, available.width,
                "Horizontal split at ratio {} doesn't add up: {} + {} + {} = {} (expected {})",
                ratio, first.width, divider.width, second.width, total, available.width
            );
        }
    }

    #[test]
    fn test_split_rects_add_up_vertical() {
        for ratio_pct in &[10u32, 25, 33, 50, 67, 75, 90] {
            let ratio = *ratio_pct as f64 / 100.0;
            let available = Rect::new(0, 0, 800, 600);
            let (first, divider, second) =
                compute_split_rects(available, SplitDirection::Vertical, ratio);

            let total = first.height + divider.height + second.height;
            assert_eq!(
                total, available.height,
                "Vertical split at ratio {} doesn't add up: {} + {} + {} = {} (expected {})",
                ratio, first.height, divider.height, second.height, total, available.height
            );
        }
    }

    #[test]
    fn test_nested_split_rects() {
        let available = Rect::new(0, 0, 800, 600);
        let (first, _div1, second) =
            compute_split_rects(available, SplitDirection::Horizontal, 0.5);

        let (top, _div2, bottom) = compute_split_rects(first, SplitDirection::Vertical, 0.5);

        assert!(top.x + top.width <= available.width);
        assert!(bottom.x + bottom.width <= available.width);
        assert!(second.x + second.width <= available.x + available.width);
        assert!(top.y + top.height <= first.y + first.height);
        assert!(bottom.y >= top.y + top.height);
    }

    #[test]
    fn test_split_rects_contiguous_horizontal() {
        let available = Rect::new(50, 100, 600, 400);
        let (first, divider, second) =
            compute_split_rects(available, SplitDirection::Horizontal, 0.4);

        assert_eq!(first.x + first.width, divider.x);
        assert_eq!(divider.x + divider.width, second.x);
        assert_eq!(second.x + second.width, available.x + available.width);
    }

    #[test]
    fn test_split_rects_contiguous_vertical() {
        let available = Rect::new(50, 100, 600, 400);
        let (first, divider, second) =
            compute_split_rects(available, SplitDirection::Vertical, 0.6);

        assert_eq!(first.y + first.height, divider.y);
        assert_eq!(divider.y + divider.height, second.y);
        assert_eq!(second.y + second.height, available.y + available.height);
    }
}
