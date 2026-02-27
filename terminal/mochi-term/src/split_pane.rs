//! Split pane layout management
//!
//! Provides a binary tree structure for managing split panes within a tab.
//! Each leaf node represents a terminal pane, and internal nodes represent
//! horizontal or vertical splits with a configurable ratio.

/// Unique identifier for a pane
pub type PaneId = u64;

/// Direction of a split
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// Split horizontally (panes side by side, divider is vertical line)
    Horizontal,
    /// Split vertically (panes stacked, divider is horizontal line)
    Vertical,
}

/// A rectangle in pixel coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Width of the divider between panes in pixels
pub const DIVIDER_WIDTH: f64 = 4.0;

/// Minimum pane size in pixels (width or height)
pub const MIN_PANE_SIZE: f64 = 50.0;

/// A node in the split pane tree
#[derive(Debug, Clone)]
pub enum PaneNode {
    /// A leaf node containing a single terminal pane
    Leaf { id: PaneId },
    /// An internal node splitting into two children
    Split {
        direction: SplitDirection,
        /// Ratio of space allocated to the first child (0.0 to 1.0)
        ratio: f64,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

impl PaneNode {
    /// Create a new leaf node
    pub fn leaf(id: PaneId) -> Self {
        PaneNode::Leaf { id }
    }

    /// Check if this node is a leaf
    pub fn is_leaf(&self) -> bool {
        matches!(self, PaneNode::Leaf { .. })
    }

    /// Get the pane ID if this is a leaf
    pub fn pane_id(&self) -> Option<PaneId> {
        match self {
            PaneNode::Leaf { id } => Some(*id),
            PaneNode::Split { .. } => None,
        }
    }

    /// Collect all pane IDs in this tree (in order)
    pub fn pane_ids(&self) -> Vec<PaneId> {
        match self {
            PaneNode::Leaf { id } => vec![*id],
            PaneNode::Split { first, second, .. } => {
                let mut ids = first.pane_ids();
                ids.extend(second.pane_ids());
                ids
            }
        }
    }

    /// Count the total number of panes (leaf nodes)
    pub fn pane_count(&self) -> usize {
        match self {
            PaneNode::Leaf { .. } => 1,
            PaneNode::Split { first, second, .. } => first.pane_count() + second.pane_count(),
        }
    }

    /// Split the pane with the given ID, returning the new pane's ID.
    /// The new pane is placed as the second child.
    pub fn split_pane(
        &mut self,
        target_id: PaneId,
        new_id: PaneId,
        direction: SplitDirection,
    ) -> bool {
        match self {
            PaneNode::Leaf { id } if *id == target_id => {
                let old_leaf = PaneNode::Leaf { id: *id };
                let new_leaf = PaneNode::Leaf { id: new_id };
                *self = PaneNode::Split {
                    direction,
                    ratio: 0.5,
                    first: Box::new(old_leaf),
                    second: Box::new(new_leaf),
                };
                true
            }
            PaneNode::Leaf { .. } => false,
            PaneNode::Split { first, second, .. } => {
                first.split_pane(target_id, new_id, direction)
                    || second.split_pane(target_id, new_id, direction)
            }
        }
    }

    /// Remove a pane by ID. Returns true if removed.
    /// When a pane is removed, its sibling replaces the parent split node.
    pub fn remove_pane(&mut self, target_id: PaneId) -> bool {
        match self {
            PaneNode::Leaf { .. } => false,
            PaneNode::Split { first, second, .. } => {
                // Check if first child is the target leaf
                if let PaneNode::Leaf { id } = first.as_ref() {
                    if *id == target_id {
                        *self = *second.clone();
                        return true;
                    }
                }
                // Check if second child is the target leaf
                if let PaneNode::Leaf { id } = second.as_ref() {
                    if *id == target_id {
                        *self = *first.clone();
                        return true;
                    }
                }
                // Recurse into children
                first.remove_pane(target_id) || second.remove_pane(target_id)
            }
        }
    }

    /// Compute the rectangle for each pane given the available area.
    /// Returns a list of (PaneId, Rect) pairs.
    pub fn layout(&self, area: Rect) -> Vec<(PaneId, Rect)> {
        match self {
            PaneNode::Leaf { id } => vec![(*id, area)],
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_area, second_area) = split_rect(area, *direction, *ratio);
                let mut result = first.layout(first_area);
                result.extend(second.layout(second_area));
                result
            }
        }
    }

    /// Find divider rectangles for hit-testing and rendering.
    /// Returns a list of (Rect, SplitDirection, path) where path identifies the split node.
    pub fn dividers(&self, area: Rect) -> Vec<DividerInfo> {
        match self {
            PaneNode::Leaf { .. } => vec![],
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let divider_rect = divider_rect(area, *direction, *ratio);
                let (first_area, second_area) = split_rect(area, *direction, *ratio);

                let mut result = vec![DividerInfo {
                    rect: divider_rect,
                    direction: *direction,
                    area,
                    ratio: *ratio,
                }];
                result.extend(first.dividers(first_area));
                result.extend(second.dividers(second_area));
                result
            }
        }
    }

    /// Find the next pane ID in the given direction from the current pane.
    /// Uses the layout rectangles to determine spatial relationships.
    pub fn find_neighbor(
        &self,
        current_id: PaneId,
        nav_direction: NavDirection,
        area: Rect,
    ) -> Option<PaneId> {
        let panes = self.layout(area);
        let current_rect = panes
            .iter()
            .find(|(id, _)| *id == current_id)
            .map(|(_, r)| *r)?;

        let (cx, cy) = rect_center(current_rect);

        // Find the closest pane in the given direction
        let mut best: Option<(PaneId, f64)> = None;

        for (id, rect) in &panes {
            if *id == current_id {
                continue;
            }
            let (px, py) = rect_center(*rect);

            let is_in_direction = match nav_direction {
                NavDirection::Left => px < cx,
                NavDirection::Right => px > cx,
                NavDirection::Up => py < cy,
                NavDirection::Down => py > cy,
            };

            if !is_in_direction {
                continue;
            }

            let dist = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
            if best.is_none() || dist < best.unwrap().1 {
                best = Some((*id, dist));
            }
        }

        best.map(|(id, _)| id)
    }
}

/// Direction for pane navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDirection {
    Left,
    Right,
    Up,
    Down,
}

/// Information about a divider for hit-testing and rendering
#[derive(Debug, Clone)]
pub struct DividerInfo {
    /// The rectangle of the divider itself
    pub rect: Rect,
    /// The direction of the split this divider belongs to
    pub direction: SplitDirection,
    /// The area of the parent split node (used for ratio updates)
    pub area: Rect,
    /// Current ratio
    pub ratio: f64,
}

/// Split a rectangle into two sub-rectangles based on direction and ratio.
/// Accounts for the divider width between the two areas.
fn split_rect(area: Rect, direction: SplitDirection, ratio: f64) -> (Rect, Rect) {
    match direction {
        SplitDirection::Horizontal => {
            let available = area.width - DIVIDER_WIDTH;
            let first_width = (available * ratio).max(0.0);
            let second_width = (available - first_width).max(0.0);
            (
                Rect::new(area.x, area.y, first_width, area.height),
                Rect::new(
                    area.x + first_width + DIVIDER_WIDTH,
                    area.y,
                    second_width,
                    area.height,
                ),
            )
        }
        SplitDirection::Vertical => {
            let available = area.height - DIVIDER_WIDTH;
            let first_height = (available * ratio).max(0.0);
            let second_height = (available - first_height).max(0.0);
            (
                Rect::new(area.x, area.y, area.width, first_height),
                Rect::new(
                    area.x,
                    area.y + first_height + DIVIDER_WIDTH,
                    area.width,
                    second_height,
                ),
            )
        }
    }
}

/// Compute the divider rectangle for a split
fn divider_rect(area: Rect, direction: SplitDirection, ratio: f64) -> Rect {
    match direction {
        SplitDirection::Horizontal => {
            let available = area.width - DIVIDER_WIDTH;
            let first_width = (available * ratio).max(0.0);
            Rect::new(area.x + first_width, area.y, DIVIDER_WIDTH, area.height)
        }
        SplitDirection::Vertical => {
            let available = area.height - DIVIDER_WIDTH;
            let first_height = (available * ratio).max(0.0);
            Rect::new(area.x, area.y + first_height, area.width, DIVIDER_WIDTH)
        }
    }
}

/// Check if a point is inside a rectangle
pub fn point_in_rect(x: f64, y: f64, rect: &Rect) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

/// Get center of a rectangle
fn rect_center(rect: Rect) -> (f64, f64) {
    (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

/// Approximate equality for rectangles (for matching during ratio update)
fn rect_approx_eq(a: Rect, b: Rect) -> bool {
    (a.x - b.x).abs() < 1.0
        && (a.y - b.y).abs() < 1.0
        && (a.width - b.width).abs() < 1.0
        && (a.height - b.height).abs() < 1.0
}

/// Manages the split pane state for a single tab.
/// Tracks the pane tree, active pane, and ID generation.
pub struct PaneManager {
    /// The root of the pane tree
    root: PaneNode,
    /// The currently focused pane ID
    active_pane: PaneId,
    /// Counter for generating unique pane IDs
    next_id: PaneId,
}

impl PaneManager {
    /// Create a new pane manager with a single pane
    pub fn new(initial_id: PaneId) -> Self {
        Self {
            root: PaneNode::leaf(initial_id),
            active_pane: initial_id,
            next_id: initial_id + 1,
        }
    }

    /// Get the root node
    pub fn root(&self) -> &PaneNode {
        &self.root
    }

    /// Get the root node mutably
    pub fn root_mut(&mut self) -> &mut PaneNode {
        &mut self.root
    }

    /// Get the active pane ID
    pub fn active_pane(&self) -> PaneId {
        self.active_pane
    }

    /// Set the active pane ID
    pub fn set_active_pane(&mut self, id: PaneId) {
        self.active_pane = id;
    }

    /// Generate a new unique pane ID
    pub fn next_id(&mut self) -> PaneId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Split the active pane in the given direction.
    /// Returns the new pane ID if successful.
    pub fn split_active(&mut self, direction: SplitDirection) -> Option<PaneId> {
        let new_id = self.next_id();
        if self.root.split_pane(self.active_pane, new_id, direction) {
            Some(new_id)
        } else {
            // Roll back the ID counter
            self.next_id -= 1;
            None
        }
    }

    /// Remove the active pane. Returns true if removed.
    /// Activates a sibling or the first remaining pane.
    pub fn remove_active(&mut self) -> bool {
        if self.root.pane_count() <= 1 {
            return false;
        }
        let old_active = self.active_pane;
        if self.root.remove_pane(old_active) {
            // Activate the first remaining pane
            let ids = self.root.pane_ids();
            self.active_pane = ids.first().copied().unwrap_or(0);
            true
        } else {
            false
        }
    }

    /// Get all pane IDs
    pub fn pane_ids(&self) -> Vec<PaneId> {
        self.root.pane_ids()
    }

    /// Get the number of panes
    pub fn pane_count(&self) -> usize {
        self.root.pane_count()
    }

    /// Compute the layout for all panes in the given area
    pub fn layout(&self, area: Rect) -> Vec<(PaneId, Rect)> {
        self.root.layout(area)
    }

    /// Get all divider info for rendering and hit-testing
    pub fn dividers(&self, area: Rect) -> Vec<DividerInfo> {
        self.root.dividers(area)
    }

    /// Navigate to a neighbor pane in the given direction
    pub fn navigate(&mut self, direction: NavDirection, area: Rect) -> bool {
        if let Some(neighbor) = self.root.find_neighbor(self.active_pane, direction, area) {
            self.active_pane = neighbor;
            true
        } else {
            false
        }
    }

    /// Update the ratio of a divider given its info and a new pixel position.
    /// `pos` is the mouse position (x for horizontal splits, y for vertical).
    /// `total_area` is the full area used for layout (needed for area matching during recursion).
    pub fn drag_divider(&mut self, info: &DividerInfo, pos: f64, total_area: Rect) -> bool {
        let new_ratio = match info.direction {
            SplitDirection::Horizontal => {
                let available = info.area.width - DIVIDER_WIDTH;
                if available <= 0.0 {
                    return false;
                }
                ((pos - info.area.x) / available).clamp(0.05, 0.95)
            }
            SplitDirection::Vertical => {
                let available = info.area.height - DIVIDER_WIDTH;
                if available <= 0.0 {
                    return false;
                }
                ((pos - info.area.y) / available).clamp(0.05, 0.95)
            }
        };

        update_ratio_recursive(
            &mut self.root,
            total_area,
            &info.area,
            info.direction,
            new_ratio,
        )
    }
}

/// Recursive helper that carries the current area for matching
fn update_ratio_recursive(
    node: &mut PaneNode,
    current_area: Rect,
    target_area: &Rect,
    target_direction: SplitDirection,
    new_ratio: f64,
) -> bool {
    match node {
        PaneNode::Leaf { .. } => false,
        PaneNode::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            if *direction == target_direction && rect_approx_eq(current_area, *target_area) {
                *ratio = new_ratio;
                return true;
            }
            let (first_area, second_area) = split_rect(current_area, *direction, *ratio);
            update_ratio_recursive(first, first_area, target_area, target_direction, new_ratio)
                || update_ratio_recursive(
                    second,
                    second_area,
                    target_area,
                    target_direction,
                    new_ratio,
                )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_pane() {
        let mgr = PaneManager::new(0);
        assert_eq!(mgr.pane_count(), 1);
        assert_eq!(mgr.active_pane(), 0);
        assert_eq!(mgr.pane_ids(), vec![0]);
    }

    #[test]
    fn test_split_horizontal() {
        let mut mgr = PaneManager::new(0);
        let new_id = mgr.split_active(SplitDirection::Horizontal);
        assert_eq!(new_id, Some(1));
        assert_eq!(mgr.pane_count(), 2);
        assert_eq!(mgr.pane_ids(), vec![0, 1]);
    }

    #[test]
    fn test_split_vertical() {
        let mut mgr = PaneManager::new(0);
        let new_id = mgr.split_active(SplitDirection::Vertical);
        assert_eq!(new_id, Some(1));
        assert_eq!(mgr.pane_count(), 2);
        assert_eq!(mgr.pane_ids(), vec![0, 1]);
    }

    #[test]
    fn test_nested_splits() {
        let mut mgr = PaneManager::new(0);
        // Split pane 0 horizontally -> panes 0, 1
        mgr.split_active(SplitDirection::Horizontal);
        assert_eq!(mgr.pane_count(), 2);

        // Split pane 0 vertically -> panes 0, 2, 1
        mgr.split_active(SplitDirection::Vertical);
        assert_eq!(mgr.pane_count(), 3);
        assert_eq!(mgr.pane_ids(), vec![0, 2, 1]);
    }

    #[test]
    fn test_remove_pane() {
        let mut mgr = PaneManager::new(0);
        mgr.split_active(SplitDirection::Horizontal);
        assert_eq!(mgr.pane_count(), 2);

        // Remove active pane (0)
        assert!(mgr.remove_active());
        assert_eq!(mgr.pane_count(), 1);
        assert_eq!(mgr.pane_ids(), vec![1]);
        assert_eq!(mgr.active_pane(), 1);
    }

    #[test]
    fn test_cannot_remove_last_pane() {
        let mut mgr = PaneManager::new(0);
        assert!(!mgr.remove_active());
        assert_eq!(mgr.pane_count(), 1);
    }

    #[test]
    fn test_remove_from_nested() {
        let mut mgr = PaneManager::new(0);
        mgr.split_active(SplitDirection::Horizontal); // 0, 1
        mgr.split_active(SplitDirection::Vertical); // 0, 2, 1
        assert_eq!(mgr.pane_count(), 3);

        // Remove pane 0 (active)
        mgr.remove_active();
        assert_eq!(mgr.pane_count(), 2);
        // Pane 2 and 1 should remain
        let ids = mgr.pane_ids();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
    }

    #[test]
    fn test_layout_single_pane() {
        let mgr = PaneManager::new(0);
        let area = Rect::new(0.0, 0.0, 800.0, 600.0);
        let layout = mgr.layout(area);
        assert_eq!(layout.len(), 1);
        assert_eq!(layout[0].0, 0);
        assert_eq!(layout[0].1, area);
    }

    #[test]
    fn test_layout_horizontal_split() {
        let mut mgr = PaneManager::new(0);
        mgr.split_active(SplitDirection::Horizontal);
        let area = Rect::new(0.0, 0.0, 800.0, 600.0);
        let layout = mgr.layout(area);
        assert_eq!(layout.len(), 2);

        // First pane should be on the left
        let (id0, rect0) = &layout[0];
        assert_eq!(*id0, 0);
        assert!(rect0.x < 400.0);
        assert!((rect0.width - (800.0 - DIVIDER_WIDTH) / 2.0).abs() < 1.0);

        // Second pane should be on the right
        let (id1, rect1) = &layout[1];
        assert_eq!(*id1, 1);
        assert!(rect1.x > 400.0 - DIVIDER_WIDTH);
    }

    #[test]
    fn test_layout_vertical_split() {
        let mut mgr = PaneManager::new(0);
        mgr.split_active(SplitDirection::Vertical);
        let area = Rect::new(0.0, 0.0, 800.0, 600.0);
        let layout = mgr.layout(area);
        assert_eq!(layout.len(), 2);

        // First pane should be on top
        let (id0, rect0) = &layout[0];
        assert_eq!(*id0, 0);
        assert!(rect0.y < 300.0);

        // Second pane should be on bottom
        let (id1, rect1) = &layout[1];
        assert_eq!(*id1, 1);
        assert!(rect1.y > 300.0 - DIVIDER_WIDTH);
    }

    #[test]
    fn test_dividers() {
        let mut mgr = PaneManager::new(0);
        mgr.split_active(SplitDirection::Horizontal);
        let area = Rect::new(0.0, 0.0, 800.0, 600.0);
        let dividers = mgr.dividers(area);
        assert_eq!(dividers.len(), 1);
        assert_eq!(dividers[0].direction, SplitDirection::Horizontal);
        assert!((dividers[0].rect.width - DIVIDER_WIDTH).abs() < 0.1);
        assert!((dividers[0].rect.height - 600.0).abs() < 0.1);
    }

    #[test]
    fn test_dividers_nested() {
        let mut mgr = PaneManager::new(0);
        mgr.split_active(SplitDirection::Horizontal); // 0, 1
        mgr.set_active_pane(1);
        mgr.split_active(SplitDirection::Vertical); // 0, 1, 2 (1 split vertically)
        let area = Rect::new(0.0, 0.0, 800.0, 600.0);
        let dividers = mgr.dividers(area);
        assert_eq!(dividers.len(), 2);
    }

    #[test]
    fn test_navigate_horizontal() {
        let mut mgr = PaneManager::new(0);
        mgr.split_active(SplitDirection::Horizontal);
        let area = Rect::new(0.0, 0.0, 800.0, 600.0);

        // Active is 0 (left), navigate right should go to 1
        assert!(mgr.navigate(NavDirection::Right, area));
        assert_eq!(mgr.active_pane(), 1);

        // Navigate left should go back to 0
        assert!(mgr.navigate(NavDirection::Left, area));
        assert_eq!(mgr.active_pane(), 0);
    }

    #[test]
    fn test_navigate_vertical() {
        let mut mgr = PaneManager::new(0);
        mgr.split_active(SplitDirection::Vertical);
        let area = Rect::new(0.0, 0.0, 800.0, 600.0);

        // Active is 0 (top), navigate down should go to 1
        assert!(mgr.navigate(NavDirection::Down, area));
        assert_eq!(mgr.active_pane(), 1);

        // Navigate up should go back to 0
        assert!(mgr.navigate(NavDirection::Up, area));
        assert_eq!(mgr.active_pane(), 0);
    }

    #[test]
    fn test_navigate_no_neighbor() {
        let mut mgr = PaneManager::new(0);
        mgr.split_active(SplitDirection::Horizontal);
        let area = Rect::new(0.0, 0.0, 800.0, 600.0);

        // Active is 0 (left), navigate left should fail
        assert!(!mgr.navigate(NavDirection::Left, area));
        assert_eq!(mgr.active_pane(), 0);
    }

    #[test]
    fn test_point_in_rect() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert!(point_in_rect(50.0, 30.0, &rect));
        assert!(point_in_rect(10.0, 20.0, &rect));
        assert!(!point_in_rect(9.0, 20.0, &rect));
        assert!(!point_in_rect(110.0, 20.0, &rect));
        assert!(!point_in_rect(50.0, 70.0, &rect));
    }

    #[test]
    fn test_split_rect_horizontal() {
        let area = Rect::new(0.0, 0.0, 100.0, 50.0);
        let (first, second) = split_rect(area, SplitDirection::Horizontal, 0.5);
        let available = 100.0 - DIVIDER_WIDTH;
        assert!((first.width - available / 2.0).abs() < 0.01);
        assert!((second.width - available / 2.0).abs() < 0.01);
        assert!((second.x - (first.width + DIVIDER_WIDTH)).abs() < 0.01);
        assert!((first.height - 50.0).abs() < 0.01);
        assert!((second.height - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_split_rect_vertical() {
        let area = Rect::new(0.0, 0.0, 100.0, 50.0);
        let (first, second) = split_rect(area, SplitDirection::Vertical, 0.5);
        let available = 50.0 - DIVIDER_WIDTH;
        assert!((first.height - available / 2.0).abs() < 0.01);
        assert!((second.height - available / 2.0).abs() < 0.01);
        assert!((second.y - (first.height + DIVIDER_WIDTH)).abs() < 0.01);
        assert!((first.width - 100.0).abs() < 0.01);
        assert!((second.width - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_drag_divider() {
        let mut mgr = PaneManager::new(0);
        mgr.split_active(SplitDirection::Horizontal);
        let area = Rect::new(0.0, 0.0, 800.0, 600.0);
        let dividers = mgr.dividers(area);
        assert_eq!(dividers.len(), 1);

        // Drag divider to x=300
        let result = mgr.drag_divider(&dividers[0], 300.0, area);
        assert!(result);

        // Verify the layout changed
        let layout = mgr.layout(area);
        let first_width = layout[0].1.width;
        // New ratio should be approximately 300 / (800 - DIVIDER_WIDTH)
        let expected_width = (800.0 - DIVIDER_WIDTH) * (300.0 / (800.0 - DIVIDER_WIDTH));
        assert!((first_width - expected_width).abs() < 1.0);
    }

    #[test]
    fn test_pane_manager_next_id() {
        let mut mgr = PaneManager::new(0);
        assert_eq!(mgr.next_id(), 1);
        assert_eq!(mgr.next_id(), 2);
        assert_eq!(mgr.next_id(), 3);
    }

    #[test]
    fn test_split_nonexistent_pane() {
        let mut mgr = PaneManager::new(0);
        // Try to split a non-existent pane by setting active to invalid ID
        mgr.set_active_pane(999);
        let result = mgr.split_active(SplitDirection::Horizontal);
        assert!(result.is_none());
    }

    #[test]
    fn test_complex_layout() {
        let mut mgr = PaneManager::new(0);
        // Create a 4-pane layout:
        // Split 0 horizontally -> [0, 1]
        mgr.split_active(SplitDirection::Horizontal);
        // Split 0 vertically -> [[0, 2], 1]
        mgr.split_active(SplitDirection::Vertical);
        // Switch to pane 1 and split vertically -> [[0, 2], [1, 3]]
        mgr.set_active_pane(1);
        mgr.split_active(SplitDirection::Vertical);

        assert_eq!(mgr.pane_count(), 4);
        let area = Rect::new(0.0, 0.0, 800.0, 600.0);
        let layout = mgr.layout(area);
        assert_eq!(layout.len(), 4);

        // All panes should have non-zero dimensions
        for (_, rect) in &layout {
            assert!(rect.width > 0.0);
            assert!(rect.height > 0.0);
        }

        // Dividers: 1 horizontal (top-level) + 2 vertical (one per side)
        let dividers = mgr.dividers(area);
        assert_eq!(dividers.len(), 3);
    }
}
