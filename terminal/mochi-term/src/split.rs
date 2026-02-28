//! Split pane tree structure for terminal panes
//!
//! Supports recursive horizontal and vertical splits within each tab.
//! Each tab has a `SplitPaneContainer` that manages a tree of panes.
//!
//! Tree mutation (split/remove) is done by consuming and rebuilding the tree
//! via free functions, which avoids any need for unsafe code or dummy values.

use terminal_pty::{Child, WindowSize};

use crate::terminal::Terminal;

/// Unique identifier for a pane
pub type PaneId = u64;

/// Direction of a split
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// Side by side (left | right)
    Vertical,
    /// Stacked (top / bottom)
    Horizontal,
}

/// Width of the divider between panes in pixels
pub const DIVIDER_WIDTH: u32 = 2;

/// A rectangular region in pixels (physical coordinates)
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

    /// Check if a pixel coordinate is inside this rect
    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}

/// A leaf pane holding a terminal and PTY child
pub struct Pane {
    pub id: PaneId,
    pub terminal: Terminal,
    pub child: Child,
    pub title: String,
    pub scroll_offset: usize,
}

impl Pane {
    pub fn new(id: PaneId, terminal: Terminal, child: Child) -> Self {
        Self {
            id,
            terminal,
            child,
            title: String::from("Terminal"),
            scroll_offset: 0,
        }
    }
}

/// A node in the split pane tree.
pub enum SplitNode {
    /// A leaf node containing a single pane
    Leaf(Box<Pane>),
    /// An internal node with two children and a split
    Split {
        direction: SplitDirection,
        /// Fraction of space given to the first child (0.0 - 1.0)
        ratio: f32,
        first: Box<SplitNode>,
        second: Box<SplitNode>,
    },
}

impl SplitNode {
    /// Create a leaf node
    pub fn leaf(pane: Pane) -> Self {
        SplitNode::Leaf(Box::new(pane))
    }

    /// Get a reference to the pane with the given id
    pub fn find_pane(&self, id: PaneId) -> Option<&Pane> {
        match self {
            SplitNode::Leaf(pane) => {
                if pane.id == id {
                    Some(pane.as_ref())
                } else {
                    None
                }
            }
            SplitNode::Split { first, second, .. } => {
                first.find_pane(id).or_else(|| second.find_pane(id))
            }
        }
    }

    /// Get a mutable reference to the pane with the given id
    pub fn find_pane_mut(&mut self, id: PaneId) -> Option<&mut Pane> {
        match self {
            SplitNode::Leaf(pane) => {
                if pane.id == id {
                    Some(pane.as_mut())
                } else {
                    None
                }
            }
            SplitNode::Split { first, second, .. } => {
                if let Some(p) = first.find_pane_mut(id) {
                    Some(p)
                } else {
                    second.find_pane_mut(id)
                }
            }
        }
    }

    /// Check whether the given pane id exists in this tree
    pub fn contains_pane(&self, id: PaneId) -> bool {
        self.find_pane(id).is_some()
    }

    /// Get the first leaf pane id (leftmost / topmost)
    pub fn first_pane_id(&self) -> PaneId {
        match self {
            SplitNode::Leaf(ref pane) => pane.id,
            SplitNode::Split { first, .. } => first.first_pane_id(),
        }
    }

    /// Count the number of leaf panes
    pub fn pane_count(&self) -> usize {
        match self {
            SplitNode::Leaf(..) => 1,
            SplitNode::Split { first, second, .. } => first.pane_count() + second.pane_count(),
        }
    }

    /// Visit all panes mutably
    pub fn for_each_pane_mut<F: FnMut(&mut Pane)>(&mut self, f: &mut F) {
        match self {
            SplitNode::Leaf(ref mut pane) => f(pane.as_mut()),
            SplitNode::Split { first, second, .. } => {
                first.for_each_pane_mut(f);
                second.for_each_pane_mut(f);
            }
        }
    }

    /// Visit all panes immutably
    pub fn for_each_pane<F: FnMut(&Pane)>(&self, f: &mut F) {
        match self {
            SplitNode::Leaf(ref pane) => f(pane.as_ref()),
            SplitNode::Split { first, second, .. } => {
                first.for_each_pane(f);
                second.for_each_pane(f);
            }
        }
    }

    /// Check if this tree has only one pane (is a single leaf)
    pub fn is_single_pane(&self) -> bool {
        matches!(self, SplitNode::Leaf(..))
    }

    /// Compute the layout rectangles for each pane.
    pub fn layout(&self, rect: PaneRect) -> Vec<(PaneId, PaneRect)> {
        let mut result = Vec::new();
        self.layout_inner(rect, &mut result);
        result
    }

    fn layout_inner(&self, rect: PaneRect, result: &mut Vec<(PaneId, PaneRect)>) {
        match self {
            SplitNode::Leaf(pane) => {
                result.push((pane.id, rect));
            }
            SplitNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (r1, r2) = split_rect(rect, *direction, *ratio);
                first.layout_inner(r1, result);
                second.layout_inner(r2, result);
            }
        }
    }

    /// Find the pane id at a pixel position, given a bounding rect.
    pub fn pane_at(&self, rect: PaneRect, px: u32, py: u32) -> Option<PaneId> {
        match self {
            SplitNode::Leaf(pane) => {
                if rect.contains(px, py) {
                    Some(pane.id)
                } else {
                    None
                }
            }
            SplitNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (r1, r2) = split_rect(rect, *direction, *ratio);
                first
                    .pane_at(r1, px, py)
                    .or_else(|| second.pane_at(r2, px, py))
            }
        }
    }

    /// Find the neighbor pane in a given direction from the focused pane.
    pub fn find_neighbor(
        &self,
        rect: PaneRect,
        focused_id: PaneId,
        nav_direction: NavDirection,
    ) -> Option<PaneId> {
        let layouts = self.layout(rect);
        let focused_rect = layouts
            .iter()
            .find(|(id, _)| *id == focused_id)
            .map(|(_, r)| *r)?;

        let cx = focused_rect.x + focused_rect.width / 2;
        let cy = focused_rect.y + focused_rect.height / 2;

        let mut best: Option<(PaneId, u32)> = None;

        for (id, r) in &layouts {
            if *id == focused_id {
                continue;
            }
            let ncx = r.x + r.width / 2;
            let ncy = r.y + r.height / 2;

            let is_valid = match nav_direction {
                NavDirection::Left => ncx < cx,
                NavDirection::Right => ncx > cx,
                NavDirection::Up => ncy < cy,
                NavDirection::Down => ncy > cy,
            };

            if is_valid {
                let dist = cx.abs_diff(ncx) + cy.abs_diff(ncy);
                if best.is_none_or(|(_, d)| dist < d) {
                    best = Some((*id, dist));
                }
            }
        }

        best.map(|(id, _)| id)
    }
}

/// Navigation direction for moving focus between panes
#[derive(Debug, Clone, Copy)]
pub enum NavDirection {
    Left,
    Right,
    Up,
    Down,
}

/// Split a rectangle into two parts based on direction and ratio,
/// accounting for the divider width.
fn split_rect(rect: PaneRect, direction: SplitDirection, ratio: f32) -> (PaneRect, PaneRect) {
    match direction {
        SplitDirection::Vertical => {
            let total = rect.width.saturating_sub(DIVIDER_WIDTH);
            let first_w = (total as f32 * ratio) as u32;
            let second_w = total.saturating_sub(first_w);
            let r1 = PaneRect::new(rect.x, rect.y, first_w, rect.height);
            let r2 = PaneRect::new(
                rect.x + first_w + DIVIDER_WIDTH,
                rect.y,
                second_w,
                rect.height,
            );
            (r1, r2)
        }
        SplitDirection::Horizontal => {
            let total = rect.height.saturating_sub(DIVIDER_WIDTH);
            let first_h = (total as f32 * ratio) as u32;
            let second_h = total.saturating_sub(first_h);
            let r1 = PaneRect::new(rect.x, rect.y, rect.width, first_h);
            let r2 = PaneRect::new(
                rect.x,
                rect.y + first_h + DIVIDER_WIDTH,
                rect.width,
                second_h,
            );
            (r1, r2)
        }
    }
}

/// Compute the divider rectangles for drawing between panes.
pub fn divider_rects(node: &SplitNode, rect: PaneRect) -> Vec<(PaneRect, SplitDirection)> {
    let mut result = Vec::new();
    divider_rects_inner(node, rect, &mut result);
    result
}

fn divider_rects_inner(
    node: &SplitNode,
    rect: PaneRect,
    result: &mut Vec<(PaneRect, SplitDirection)>,
) {
    if let SplitNode::Split {
        direction,
        ratio,
        first,
        second,
    } = node
    {
        let (r1, r2) = split_rect(rect, *direction, *ratio);

        let divider = match direction {
            SplitDirection::Vertical => {
                PaneRect::new(r1.x + r1.width, rect.y, DIVIDER_WIDTH, rect.height)
            }
            SplitDirection::Horizontal => {
                PaneRect::new(rect.x, r1.y + r1.height, rect.width, DIVIDER_WIDTH)
            }
        };
        result.push((divider, *direction));

        divider_rects_inner(first, r1, result);
        divider_rects_inner(second, r2, result);
    }
}

// ---------------------------------------------------------------------------
// Tree mutation helpers (consume-and-rebuild pattern)
// ---------------------------------------------------------------------------

/// Insert a split at `target_id`. Consumes the tree and returns the new tree.
/// Returns `(new_tree, None)` on success (pane consumed), or
/// `(original_tree, Some(pane))` if target was not found.
fn insert_split(
    node: SplitNode,
    target_id: PaneId,
    direction: SplitDirection,
    new_pane: Pane,
) -> (SplitNode, Option<Pane>) {
    match node {
        SplitNode::Leaf(pane) if pane.id == target_id => {
            let new_node = SplitNode::Split {
                direction,
                ratio: 0.5,
                first: Box::new(SplitNode::Leaf(pane)),
                second: Box::new(SplitNode::leaf(new_pane)),
            };
            (new_node, None)
        }
        SplitNode::Split {
            direction: d,
            ratio,
            first,
            second,
        } => {
            // Check which subtree contains the target BEFORE consuming
            if first.contains_pane(target_id) {
                let (new_first, leftover) = insert_split(*first, target_id, direction, new_pane);
                let rebuilt = SplitNode::Split {
                    direction: d,
                    ratio,
                    first: Box::new(new_first),
                    second,
                };
                (rebuilt, leftover)
            } else if second.contains_pane(target_id) {
                let (new_second, leftover) = insert_split(*second, target_id, direction, new_pane);
                let rebuilt = SplitNode::Split {
                    direction: d,
                    ratio,
                    first,
                    second: Box::new(new_second),
                };
                (rebuilt, leftover)
            } else {
                let rebuilt = SplitNode::Split {
                    direction: d,
                    ratio,
                    first,
                    second,
                };
                (rebuilt, Some(new_pane))
            }
        }
        other => (other, Some(new_pane)),
    }
}

/// Remove a pane from the tree. Returns the rebuilt tree.
fn remove_pane_from_tree(node: SplitNode, target_id: PaneId) -> SplitNode {
    match node {
        SplitNode::Leaf(_) => node,
        SplitNode::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            // Check if first child is the target leaf
            if let SplitNode::Leaf(ref pane) = *first {
                if pane.id == target_id {
                    return *second;
                }
            }
            // Check if second child is the target leaf
            if let SplitNode::Leaf(ref pane) = *second {
                if pane.id == target_id {
                    return *first;
                }
            }
            // Recurse into the subtree that contains the target
            if first.contains_pane(target_id) {
                let new_first = remove_pane_from_tree(*first, target_id);
                SplitNode::Split {
                    direction,
                    ratio,
                    first: Box::new(new_first),
                    second,
                }
            } else if second.contains_pane(target_id) {
                let new_second = remove_pane_from_tree(*second, target_id);
                SplitNode::Split {
                    direction,
                    ratio,
                    first,
                    second: Box::new(new_second),
                }
            } else {
                SplitNode::Split {
                    direction,
                    ratio,
                    first,
                    second,
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SplitPaneContainer
// ---------------------------------------------------------------------------

/// The root split pane container for a single tab.
/// Manages the tree and focused pane tracking.
pub struct SplitPaneContainer {
    /// Root is wrapped in Option so we can temporarily take it for rebuilding.
    root: Option<SplitNode>,
    pub focused_pane_id: PaneId,
    next_pane_id: PaneId,
}

impl SplitPaneContainer {
    /// Create a container with a single pane
    pub fn new(terminal: Terminal, child: Child) -> Self {
        let id = 1;
        let pane = Pane::new(id, terminal, child);
        Self {
            root: Some(SplitNode::leaf(pane)),
            focused_pane_id: id,
            next_pane_id: 2,
        }
    }

    /// Allocate a new pane id
    fn next_id(&mut self) -> PaneId {
        let id = self.next_pane_id;
        self.next_pane_id += 1;
        id
    }

    /// Get a reference to the root node
    pub fn root(&self) -> &SplitNode {
        self.root
            .as_ref()
            .expect("split tree should never be empty")
    }

    /// Get a mutable reference to the root node
    pub fn root_mut(&mut self) -> &mut SplitNode {
        self.root
            .as_mut()
            .expect("split tree should never be empty")
    }

    /// Get a reference to the focused pane
    pub fn focused_pane(&self) -> Option<&Pane> {
        self.root().find_pane(self.focused_pane_id)
    }

    /// Get a mutable reference to the focused pane
    pub fn focused_pane_mut(&mut self) -> Option<&mut Pane> {
        let id = self.focused_pane_id;
        self.root_mut().find_pane_mut(id)
    }

    /// Split the focused pane in the given direction.
    /// Returns the new pane id, or None if the split failed.
    pub fn split_focused(
        &mut self,
        direction: SplitDirection,
        cols: usize,
        rows: usize,
    ) -> Option<PaneId> {
        let new_id = self.next_id();
        let target_id = self.focused_pane_id;

        let terminal = Terminal::new(cols.max(1), rows.max(1));
        let child = match Child::spawn_shell(WindowSize::new(cols as u16, rows as u16)) {
            Ok(child) => {
                let _ = child.set_nonblocking(true);
                child
            }
            Err(e) => {
                log::error!("Failed to spawn shell for split pane: {}", e);
                return None;
            }
        };

        let new_pane = Pane::new(new_id, terminal, child);

        // Take the tree, rebuild with split, put back
        let old_root = self.root.take().expect("split tree should never be empty");
        let (new_root, leftover) = insert_split(old_root, target_id, direction, new_pane);
        self.root = Some(new_root);

        if leftover.is_none() {
            self.focused_pane_id = new_id;
            log::info!(
                "Split pane {} {:?}, new pane {}",
                target_id,
                direction,
                new_id
            );
            Some(new_id)
        } else {
            log::error!("Failed to find target pane {} for split", target_id);
            None
        }
    }

    /// Get the number of panes
    pub fn pane_count(&self) -> usize {
        self.root().pane_count()
    }

    /// Check if there's only one pane
    pub fn is_single_pane(&self) -> bool {
        self.root().is_single_pane()
    }

    /// Compute layout for the given available rect
    pub fn layout(&self, rect: PaneRect) -> Vec<(PaneId, PaneRect)> {
        self.root().layout(rect)
    }

    /// Find pane at pixel coordinates
    pub fn pane_at(&self, rect: PaneRect, px: u32, py: u32) -> Option<PaneId> {
        self.root().pane_at(rect, px, py)
    }

    /// Navigate focus in the given direction. Returns true if focus changed.
    pub fn navigate(&mut self, rect: PaneRect, direction: NavDirection) -> bool {
        if let Some(new_id) = self
            .root()
            .find_neighbor(rect, self.focused_pane_id, direction)
        {
            self.focused_pane_id = new_id;
            true
        } else {
            false
        }
    }

    /// Set focus to a specific pane id. Returns true if the pane exists.
    pub fn set_focus(&mut self, id: PaneId) -> bool {
        if self.root().contains_pane(id) {
            self.focused_pane_id = id;
            true
        } else {
            false
        }
    }

    /// Visit all panes mutably
    pub fn for_each_pane_mut<F: FnMut(&mut Pane)>(&mut self, f: &mut F) {
        self.root_mut().for_each_pane_mut(f);
    }

    /// Remove a specific pane by id. Returns false if it's the last pane.
    pub fn remove_pane(&mut self, pane_id: PaneId) -> bool {
        if self.is_single_pane() {
            return false;
        }

        let old_root = self.root.take().expect("split tree should never be empty");
        let new_root = remove_pane_from_tree(old_root, pane_id);
        self.root = Some(new_root);

        if !self.root().contains_pane(self.focused_pane_id) {
            self.focused_pane_id = self.root().first_pane_id();
        }

        log::info!("Removed pane {}", pane_id);
        true
    }

    /// Remove all panes whose child process has exited.
    /// Returns true if any were removed.
    pub fn remove_dead_panes(&mut self) -> bool {
        let mut dead_ids = Vec::new();
        self.root().for_each_pane(&mut |pane| {
            if !pane.child.is_running() {
                dead_ids.push(pane.id);
            }
        });

        let mut removed_any = false;
        for id in dead_ids {
            if self.pane_count() <= 1 {
                break;
            }
            if self.remove_pane(id) {
                removed_any = true;
            }
        }
        removed_any
    }

    /// Check if any child process is still running
    pub fn any_running(&self) -> bool {
        let mut running = false;
        self.root().for_each_pane(&mut |pane| {
            if pane.child.is_running() {
                running = true;
            }
        });
        running
    }

    /// Get the title of the focused pane
    pub fn focused_title(&self) -> &str {
        self.focused_pane()
            .map(|p| p.title.as_str())
            .unwrap_or("Terminal")
    }
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
        assert!(!rect.contains(10, 70));
        assert!(!rect.contains(110, 20));
    }

    #[test]
    fn test_split_rect_vertical() {
        let rect = PaneRect::new(0, 0, 100, 50);
        let (r1, r2) = split_rect(rect, SplitDirection::Vertical, 0.5);
        assert_eq!(r1.x, 0);
        assert_eq!(r1.width, 49);
        assert_eq!(r2.x, 51);
        assert_eq!(r2.width, 49);
    }

    #[test]
    fn test_split_rect_horizontal() {
        let rect = PaneRect::new(0, 0, 100, 50);
        let (r1, r2) = split_rect(rect, SplitDirection::Horizontal, 0.5);
        assert_eq!(r1.y, 0);
        assert_eq!(r1.height, 24);
        assert_eq!(r2.y, 26);
        assert_eq!(r2.height, 24);
    }
}
