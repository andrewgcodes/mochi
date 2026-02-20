//! Main application logic
//!
//! Ties together the terminal, PTY, and renderer.
//! Supports split panes (horizontal/vertical terminal multiplexing) within each tab.

use std::io;
use std::rc::Rc;
use std::time::Instant;

use arboard::Clipboard;
use terminal_pty::{Child, WindowSize};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::window::{Window, WindowBuilder};

use terminal_core::{Point, SelectionType};

use crate::config::Config;
use crate::input::{encode_bracketed_paste, encode_focus, encode_key, encode_mouse, MouseEvent};
use crate::renderer::{PaneRenderInfo, Renderer, TabInfo};
use crate::terminal::Terminal;

const TAB_BAR_PADDING: u32 = 8;
const TAB_MAX_WIDTH: u32 = 200;
const CLOSE_BTN_WIDTH: u32 = 20;
const NEW_TAB_BTN_WIDTH: u32 = 32;
const SPLIT_DIVIDER_WIDTH: u32 = 4;

fn compute_tab_bar_height(cell_size: &crate::renderer::CellSize) -> u32 {
    cell_size.height as u32 + TAB_BAR_PADDING
}

type PaneId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Rect {
    fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x as f64
            && px < (self.x + self.width) as f64
            && py >= self.y as f64
            && py < (self.y + self.height) as f64
    }
}

#[derive(Debug)]
enum SplitNode {
    Leaf {
        pane_id: PaneId,
    },
    Split {
        direction: SplitDirection,
        ratio: f64,
        first: Box<SplitNode>,
        second: Box<SplitNode>,
    },
}

impl SplitNode {
    fn new_leaf(pane_id: PaneId) -> Self {
        SplitNode::Leaf { pane_id }
    }

    fn collect_pane_ids(&self) -> Vec<PaneId> {
        match self {
            SplitNode::Leaf { pane_id } => vec![*pane_id],
            SplitNode::Split { first, second, .. } => {
                let mut ids = first.collect_pane_ids();
                ids.extend(second.collect_pane_ids());
                ids
            }
        }
    }

    fn compute_rects(&self, area: Rect) -> Vec<(PaneId, Rect)> {
        match self {
            SplitNode::Leaf { pane_id } => vec![(*pane_id, area)],
            SplitNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let divider = SPLIT_DIVIDER_WIDTH;
                let (first_area, second_area) = match direction {
                    SplitDirection::Horizontal => {
                        let first_w = ((area.width.saturating_sub(divider)) as f64 * ratio) as u32;
                        let second_x = area.x + first_w + divider;
                        let second_w = area.width.saturating_sub(first_w + divider);
                        (
                            Rect::new(area.x, area.y, first_w, area.height),
                            Rect::new(second_x, area.y, second_w, area.height),
                        )
                    }
                    SplitDirection::Vertical => {
                        let first_h = ((area.height.saturating_sub(divider)) as f64 * ratio) as u32;
                        let second_y = area.y + first_h + divider;
                        let second_h = area.height.saturating_sub(first_h + divider);
                        (
                            Rect::new(area.x, area.y, area.width, first_h),
                            Rect::new(area.x, second_y, area.width, second_h),
                        )
                    }
                };
                let mut rects = first.compute_rects(first_area);
                rects.extend(second.compute_rects(second_area));
                rects
            }
        }
    }

    fn compute_dividers(&self, area: Rect) -> Vec<(Rect, SplitDirection)> {
        match self {
            SplitNode::Leaf { .. } => vec![],
            SplitNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let divider = SPLIT_DIVIDER_WIDTH;
                let (first_area, divider_rect, second_area) = match direction {
                    SplitDirection::Horizontal => {
                        let first_w = ((area.width.saturating_sub(divider)) as f64 * ratio) as u32;
                        let div_rect = Rect::new(area.x + first_w, area.y, divider, area.height);
                        let second_x = area.x + first_w + divider;
                        let second_w = area.width.saturating_sub(first_w + divider);
                        (
                            Rect::new(area.x, area.y, first_w, area.height),
                            div_rect,
                            Rect::new(second_x, area.y, second_w, area.height),
                        )
                    }
                    SplitDirection::Vertical => {
                        let first_h = ((area.height.saturating_sub(divider)) as f64 * ratio) as u32;
                        let div_rect = Rect::new(area.x, area.y + first_h, area.width, divider);
                        let second_y = area.y + first_h + divider;
                        let second_h = area.height.saturating_sub(first_h + divider);
                        (
                            Rect::new(area.x, area.y, area.width, first_h),
                            div_rect,
                            Rect::new(area.x, second_y, area.width, second_h),
                        )
                    }
                };
                let mut dividers = vec![(divider_rect, *direction)];
                dividers.extend(first.compute_dividers(first_area));
                dividers.extend(second.compute_dividers(second_area));
                dividers
            }
        }
    }

    fn split_pane(
        &mut self,
        target_pane_id: PaneId,
        new_pane_id: PaneId,
        direction: SplitDirection,
    ) -> bool {
        match self {
            SplitNode::Leaf { pane_id } if *pane_id == target_pane_id => {
                let old_leaf = Box::new(SplitNode::new_leaf(target_pane_id));
                let new_leaf = Box::new(SplitNode::new_leaf(new_pane_id));
                *self = SplitNode::Split {
                    direction,
                    ratio: 0.5,
                    first: old_leaf,
                    second: new_leaf,
                };
                true
            }
            SplitNode::Leaf { .. } => false,
            SplitNode::Split { first, second, .. } => {
                first.split_pane(target_pane_id, new_pane_id, direction)
                    || second.split_pane(target_pane_id, new_pane_id, direction)
            }
        }
    }

    fn remove_pane(&mut self, target_pane_id: PaneId) -> Option<SplitNode> {
        match self {
            SplitNode::Leaf { pane_id } if *pane_id == target_pane_id => None,
            SplitNode::Leaf { .. } => None,
            SplitNode::Split { first, second, .. } => {
                if let SplitNode::Leaf { pane_id } = first.as_ref() {
                    if *pane_id == target_pane_id {
                        return Some(std::mem::replace(second.as_mut(), SplitNode::new_leaf(0)));
                    }
                }
                if let SplitNode::Leaf { pane_id } = second.as_ref() {
                    if *pane_id == target_pane_id {
                        return Some(std::mem::replace(first.as_mut(), SplitNode::new_leaf(0)));
                    }
                }
                if let Some(replacement) = first.remove_pane(target_pane_id) {
                    *first.as_mut() = replacement;
                    return None;
                }
                if let Some(replacement) = second.remove_pane(target_pane_id) {
                    *second.as_mut() = replacement;
                    return None;
                }
                None
            }
        }
    }

    fn find_neighbor(
        &self,
        target_pane_id: PaneId,
        direction: NeighborDirection,
        area: Rect,
    ) -> Option<PaneId> {
        let rects = self.compute_rects(area);
        let target_rect = rects.iter().find(|(id, _)| *id == target_pane_id)?.1;

        let (target_cx, target_cy) = (
            target_rect.x as f64 + target_rect.width as f64 / 2.0,
            target_rect.y as f64 + target_rect.height as f64 / 2.0,
        );

        let mut best: Option<(PaneId, f64)> = None;

        for (id, rect) in &rects {
            if *id == target_pane_id {
                continue;
            }
            let cx = rect.x as f64 + rect.width as f64 / 2.0;
            let cy = rect.y as f64 + rect.height as f64 / 2.0;

            let is_valid = match direction {
                NeighborDirection::Left => cx < target_cx,
                NeighborDirection::Right => cx > target_cx,
                NeighborDirection::Up => cy < target_cy,
                NeighborDirection::Down => cy > target_cy,
            };

            if is_valid {
                let dist = (cx - target_cx).powi(2) + (cy - target_cy).powi(2);
                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((*id, dist));
                }
            }
        }

        best.map(|(id, _)| id)
    }

    fn adjust_divider_at(
        &mut self,
        px: f64,
        py: f64,
        area: Rect,
        delta_x: f64,
        delta_y: f64,
    ) -> bool {
        match self {
            SplitNode::Leaf { .. } => false,
            SplitNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let divider = SPLIT_DIVIDER_WIDTH;
                let (first_area, div_rect, second_area) = match direction {
                    SplitDirection::Horizontal => {
                        let first_w = ((area.width.saturating_sub(divider)) as f64 * *ratio) as u32;
                        let div_rect = Rect::new(area.x + first_w, area.y, divider, area.height);
                        let second_x = area.x + first_w + divider;
                        let second_w = area.width.saturating_sub(first_w + divider);
                        (
                            Rect::new(area.x, area.y, first_w, area.height),
                            div_rect,
                            Rect::new(second_x, area.y, second_w, area.height),
                        )
                    }
                    SplitDirection::Vertical => {
                        let first_h =
                            ((area.height.saturating_sub(divider)) as f64 * *ratio) as u32;
                        let div_rect = Rect::new(area.x, area.y + first_h, area.width, divider);
                        let second_y = area.y + first_h + divider;
                        let second_h = area.height.saturating_sub(first_h + divider);
                        (
                            Rect::new(area.x, area.y, area.width, first_h),
                            div_rect,
                            Rect::new(area.x, second_y, area.width, second_h),
                        )
                    }
                };

                if div_rect.contains(px, py) {
                    let available = match direction {
                        SplitDirection::Horizontal => area.width.saturating_sub(divider) as f64,
                        SplitDirection::Vertical => area.height.saturating_sub(divider) as f64,
                    };
                    if available > 0.0 {
                        let delta = match direction {
                            SplitDirection::Horizontal => delta_x,
                            SplitDirection::Vertical => delta_y,
                        };
                        *ratio = (*ratio + delta / available).clamp(0.1, 0.9);
                    }
                    return true;
                }

                first.adjust_divider_at(px, py, first_area, delta_x, delta_y)
                    || second.adjust_divider_at(px, py, second_area, delta_x, delta_y)
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum NeighborDirection {
    Left,
    Right,
    Up,
    Down,
}

struct Pane {
    id: PaneId,
    terminal: Terminal,
    child: Child,
    title: String,
    scroll_offset: usize,
}

impl Pane {
    fn new(id: PaneId, terminal: Terminal, child: Child) -> Self {
        Self {
            id,
            terminal,
            child,
            title: String::from("Terminal"),
            scroll_offset: 0,
        }
    }
}

struct Tab {
    panes: Vec<Pane>,
    split_root: SplitNode,
    active_pane_id: PaneId,
    next_pane_id: PaneId,
}

impl Tab {
    fn new(terminal: Terminal, child: Child) -> Self {
        let pane_id = 0;
        let pane = Pane::new(pane_id, terminal, child);
        Self {
            panes: vec![pane],
            split_root: SplitNode::new_leaf(pane_id),
            active_pane_id: pane_id,
            next_pane_id: 1,
        }
    }

    fn active_pane(&self) -> Option<&Pane> {
        self.panes.iter().find(|p| p.id == self.active_pane_id)
    }

    fn active_pane_mut(&mut self) -> Option<&mut Pane> {
        let id = self.active_pane_id;
        self.panes.iter_mut().find(|p| p.id == id)
    }

    fn pane_by_id(&self, id: PaneId) -> Option<&Pane> {
        self.panes.iter().find(|p| p.id == id)
    }

    fn pane_by_id_mut(&mut self, id: PaneId) -> Option<&mut Pane> {
        self.panes.iter_mut().find(|p| p.id == id)
    }

    fn title(&self) -> &str {
        self.active_pane()
            .map(|p| p.title.as_str())
            .unwrap_or("Terminal")
    }

    fn has_running_children(&self) -> bool {
        self.panes.iter().any(|p| p.child.is_running())
    }
}

pub struct App {
    config: Config,
    window: Option<Rc<Window>>,
    renderer: Option<Renderer>,
    tabs: Vec<Tab>,
    active_tab: usize,
    #[allow(dead_code)]
    clipboard: Option<Clipboard>,
    modifiers: ModifiersState,
    mouse_cell: (u16, u16),
    mouse_pixel: (f64, f64),
    mouse_buttons: [bool; 3],
    last_render: Instant,
    needs_redraw: bool,
    focused: bool,
    tab_bar_height: u32,
    scrollbar_dragging: bool,
    scrollbar_drag_start_y: f64,
    scrollbar_drag_start_offset: usize,
    divider_dragging: bool,
    divider_drag_last: (f64, f64),
}

impl App {
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            config,
            window: None,
            renderer: None,
            tabs: Vec::new(),
            active_tab: 0,
            clipboard: Clipboard::new().ok(),
            modifiers: ModifiersState::empty(),
            mouse_cell: (0, 0),
            mouse_pixel: (0.0, 0.0),
            mouse_buttons: [false; 3],
            last_render: Instant::now(),
            needs_redraw: true,
            focused: true,
            tab_bar_height: 0,
            scrollbar_dragging: false,
            scrollbar_drag_start_y: 0.0,
            scrollbar_drag_start_offset: 0,
            divider_dragging: false,
            divider_drag_last: (0.0, 0.0),
        })
    }

    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;

        let window = WindowBuilder::new()
            .with_title("Mochi Terminal")
            .with_inner_size(LogicalSize::new(800, 600))
            .build(&event_loop)?;

        let window = Rc::new(window);

        self.init_graphics(window.clone())?;

        event_loop.run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, .. } => {
                    self.handle_window_event(event, elwt);
                }
                Event::AboutToWait => {
                    self.poll_pty();

                    if !self.check_child() {
                        log::info!("Child process exited");
                        elwt.exit();
                        return;
                    }

                    if self.needs_redraw {
                        self.render();
                    }
                }
                _ => {}
            }
        })?;

        Ok(())
    }

    fn handle_window_event(
        &mut self,
        event: WindowEvent,
        elwt: &winit::event_loop::EventLoopWindowTarget<()>,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                elwt.exit();
            }
            WindowEvent::Resized(size) => {
                self.handle_resize(size);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_key_input(&event);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.handle_mouse_input(button, state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_mouse_motion(position);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_scroll(delta);
            }
            WindowEvent::Focused(focused) => {
                self.handle_focus(focused);
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => {}
        }
    }

    fn init_graphics(&mut self, window: Rc<Window>) -> Result<(), Box<dyn std::error::Error>> {
        let size = window.inner_size();

        let renderer = Renderer::new(
            window.clone(),
            self.config.font_size(),
            self.config.effective_colors(),
        )?;

        let cell_size = renderer.cell_size();
        self.tab_bar_height = compute_tab_bar_height(&cell_size);
        let cols = (size.width as f32 / cell_size.width) as usize;
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);
        let rows = (terminal_height as f32 / cell_size.height) as usize;

        let terminal = Terminal::new(cols.max(1), rows.max(1));
        let child = Child::spawn_shell(WindowSize::new(cols as u16, rows as u16))?;
        child.set_nonblocking(true)?;

        let tab = Tab::new(terminal, child);
        self.tabs.push(tab);
        self.active_tab = 0;

        self.window = Some(window);
        self.renderer = Some(renderer);

        Ok(())
    }

    fn create_new_tab(&mut self) {
        let Some(renderer) = &self.renderer else {
            return;
        };
        let Some(window) = &self.window else { return };

        let size = window.inner_size();
        let cell_size = renderer.cell_size();
        let cols = (size.width as f32 / cell_size.width) as usize;
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);
        let rows = (terminal_height as f32 / cell_size.height) as usize;

        let terminal = Terminal::new(cols.max(1), rows.max(1));
        match Child::spawn_shell(WindowSize::new(cols as u16, rows as u16)) {
            Ok(child) => {
                let _ = child.set_nonblocking(true);
                let tab = Tab::new(terminal, child);
                self.tabs.push(tab);
                self.active_tab = self.tabs.len() - 1;
                self.needs_redraw = true;
                log::info!("Created new tab {}", self.active_tab + 1);
            }
            Err(e) => {
                log::error!("Failed to create new tab: {}", e);
            }
        }
    }

    fn close_current_tab(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }

        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.needs_redraw = true;
        log::info!("Closed tab, now on tab {}", self.active_tab + 1);
        true
    }

    #[allow(dead_code)]
    fn switch_to_tab(&mut self, index: usize) {
        if index < self.tabs.len() && index != self.active_tab {
            self.active_tab = index;
            self.needs_redraw = true;
            log::info!("Switched to tab {}", index + 1);
        }
    }

    fn handle_tab_bar_click(&mut self, x: f64) {
        if self.tabs.is_empty() {
            return;
        }
        let Some(window) = &self.window else { return };

        let window_width = window.inner_size().width;
        let num_tabs = self.tabs.len() as u32;
        let available_width = window_width.saturating_sub(NEW_TAB_BTN_WIDTH);
        let tab_width = if num_tabs > 0 {
            (available_width / num_tabs).min(TAB_MAX_WIDTH)
        } else {
            TAB_MAX_WIDTH
        };

        let click_x = x as u32;
        let tabs_end = num_tabs * tab_width;

        if click_x >= tabs_end && click_x < tabs_end + NEW_TAB_BTN_WIDTH {
            self.create_new_tab();
            return;
        }

        if click_x < tabs_end {
            let tab_index = (click_x / tab_width) as usize;
            if tab_index < self.tabs.len() {
                let tab_start = tab_index as u32 * tab_width;
                let close_x_start = tab_start + tab_width.saturating_sub(CLOSE_BTN_WIDTH);

                if click_x >= close_x_start && self.tabs.len() > 1 {
                    self.tabs.remove(tab_index);
                    if self.active_tab >= self.tabs.len() {
                        self.active_tab = self.tabs.len() - 1;
                    } else if self.active_tab > tab_index {
                        self.active_tab -= 1;
                    }
                    self.needs_redraw = true;
                    log::info!("Closed tab via click {}", tab_index + 1);
                } else {
                    self.switch_to_tab(tab_index);
                }
            }
        }
    }

    fn content_area(&self) -> Rect {
        let Some(window) = &self.window else {
            return Rect::new(0, 0, 0, 0);
        };
        let size = window.inner_size();
        Rect::new(
            0,
            self.tab_bar_height,
            size.width,
            size.height.saturating_sub(self.tab_bar_height),
        )
    }

    fn split_active_pane(&mut self, direction: SplitDirection) {
        let Some(renderer) = &self.renderer else {
            return;
        };

        if self.tabs.is_empty() {
            return;
        }

        let cell_size = renderer.cell_size();
        let content = self.content_area();
        let tab = &mut self.tabs[self.active_tab];
        let active_id = tab.active_pane_id;
        let new_id = tab.next_pane_id;
        tab.next_pane_id += 1;

        if !tab.split_root.split_pane(active_id, new_id, direction) {
            return;
        }

        let rects = tab.split_root.compute_rects(content);
        let new_rect = rects
            .iter()
            .find(|(id, _)| *id == new_id)
            .map(|(_, r)| *r)
            .unwrap_or(content);

        let cols = (new_rect.width as f32 / cell_size.width) as usize;
        let rows = (new_rect.height as f32 / cell_size.height) as usize;

        let terminal = Terminal::new(cols.max(1), rows.max(1));
        match Child::spawn_shell(WindowSize::new(cols.max(1) as u16, rows.max(1) as u16)) {
            Ok(child) => {
                let _ = child.set_nonblocking(true);
                let pane = Pane::new(new_id, terminal, child);
                tab.panes.push(pane);
                tab.active_pane_id = new_id;

                self.resize_all_panes_in_tab();
                self.needs_redraw = true;
                log::info!("Split pane {:?}, new pane {}", direction, new_id);
            }
            Err(e) => {
                log::error!("Failed to split pane: {}", e);
            }
        }
    }

    fn close_active_pane(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }

        let tab = &mut self.tabs[self.active_tab];

        if tab.panes.len() <= 1 {
            return self.close_current_tab();
        }

        let active_id = tab.active_pane_id;

        if let Some(replacement) = tab.split_root.remove_pane(active_id) {
            tab.split_root = replacement;
        }

        tab.panes.retain(|p| p.id != active_id);

        let remaining_ids = tab.split_root.collect_pane_ids();
        if !remaining_ids.contains(&tab.active_pane_id) {
            tab.active_pane_id = remaining_ids[0];
        }

        self.resize_all_panes_in_tab();
        self.needs_redraw = true;
        log::info!("Closed pane {}", active_id);
        true
    }

    fn navigate_pane(&mut self, direction: NeighborDirection) {
        if self.tabs.is_empty() {
            return;
        }

        let content = self.content_area();
        let tab = &self.tabs[self.active_tab];
        let active_id = tab.active_pane_id;

        if let Some(neighbor_id) = tab.split_root.find_neighbor(active_id, direction, content) {
            let tab = &mut self.tabs[self.active_tab];
            tab.active_pane_id = neighbor_id;
            self.needs_redraw = true;
            log::info!("Navigated to pane {}", neighbor_id);
        }
    }

    fn resize_all_panes_in_tab(&mut self) {
        let Some(renderer) = &self.renderer else {
            return;
        };

        if self.tabs.is_empty() {
            return;
        }

        let cell_size = renderer.cell_size();
        let content = self.content_area();
        let tab = &mut self.tabs[self.active_tab];
        let rects = tab.split_root.compute_rects(content);

        for (pane_id, rect) in &rects {
            let cols = (rect.width as f32 / cell_size.width) as usize;
            let rows = (rect.height as f32 / cell_size.height) as usize;
            if cols > 0 && rows > 0 {
                if let Some(pane) = tab.pane_by_id_mut(*pane_id) {
                    pane.terminal.resize(cols, rows);
                    let _ = pane.child.resize(WindowSize::new(cols as u16, rows as u16));
                }
            }
        }
    }

    fn handle_resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        let Some(renderer) = &mut self.renderer else {
            return;
        };

        renderer.resize(size.width, size.height);

        let cell_size = renderer.cell_size();
        let content_area = Rect::new(
            0,
            self.tab_bar_height,
            size.width,
            size.height.saturating_sub(self.tab_bar_height),
        );

        for tab in &mut self.tabs {
            let rects = tab.split_root.compute_rects(content_area);
            for (pane_id, rect) in &rects {
                let cols = (rect.width as f32 / cell_size.width) as usize;
                let rows = (rect.height as f32 / cell_size.height) as usize;
                if cols > 0 && rows > 0 {
                    if let Some(pane) = tab.pane_by_id_mut(*pane_id) {
                        pane.terminal.resize(cols, rows);
                        let _ = pane.child.resize(WindowSize::new(cols as u16, rows as u16));
                    }
                }
            }
        }

        self.needs_redraw = true;
    }

    fn handle_key_input(&mut self, event: &winit::event::KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }

        let ctrl_shift = self.modifiers.control_key() && self.modifiers.shift_key();

        if ctrl_shift {
            match &event.logical_key {
                Key::Character(c) if c.to_lowercase() == "c" => {
                    self.handle_copy();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "v" => {
                    self.handle_paste();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "f" => {
                    self.handle_find();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "r" => {
                    self.handle_reload_config();
                    return;
                }
                #[cfg(target_os = "macos")]
                Key::Character(c) if c.to_lowercase() == "t" => {
                    self.handle_toggle_theme();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "d" => {
                    self.split_active_pane(SplitDirection::Horizontal);
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "e" => {
                    self.split_active_pane(SplitDirection::Vertical);
                    return;
                }
                Key::Named(NamedKey::ArrowLeft) => {
                    self.navigate_pane(NeighborDirection::Left);
                    return;
                }
                Key::Named(NamedKey::ArrowRight) => {
                    self.navigate_pane(NeighborDirection::Right);
                    return;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.navigate_pane(NeighborDirection::Up);
                    return;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    self.navigate_pane(NeighborDirection::Down);
                    return;
                }
                _ => {}
            }
        }

        #[cfg(target_os = "macos")]
        if self.modifiers.super_key() && !self.modifiers.control_key() && !self.modifiers.alt_key()
        {
            match &event.logical_key {
                Key::Character(c) if c.to_lowercase() == "v" => {
                    self.handle_paste();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "c" => {
                    self.handle_copy();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "n" => {
                    self.handle_new_window();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "t" => {
                    self.create_new_tab();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "w" => {
                    if !self.close_active_pane() {
                        self.tabs.clear();
                    }
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "d" => {
                    self.split_active_pane(SplitDirection::Horizontal);
                    return;
                }
                Key::Character(c) if c == "1" => {
                    self.switch_to_tab(0);
                    return;
                }
                Key::Character(c) if c == "2" => {
                    self.switch_to_tab(1);
                    return;
                }
                Key::Character(c) if c == "3" => {
                    self.switch_to_tab(2);
                    return;
                }
                Key::Character(c) if c == "4" => {
                    self.switch_to_tab(3);
                    return;
                }
                Key::Character(c) if c == "5" => {
                    self.switch_to_tab(4);
                    return;
                }
                Key::Character(c) if c == "6" => {
                    self.switch_to_tab(5);
                    return;
                }
                Key::Character(c) if c == "7" => {
                    self.switch_to_tab(6);
                    return;
                }
                Key::Character(c) if c == "8" => {
                    self.switch_to_tab(7);
                    return;
                }
                Key::Character(c) if c == "9" => {
                    self.switch_to_tab(8);
                    return;
                }
                _ => {}
            }
        }

        #[cfg(not(target_os = "macos"))]
        if ctrl_shift {
            match &event.logical_key {
                Key::Character(c) if c.to_lowercase() == "t" && !self.modifiers.super_key() => {
                    self.create_new_tab();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "w" => {
                    if !self.close_active_pane() {
                        self.tabs.clear();
                    }
                    return;
                }
                _ => {}
            }
        }

        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        let Some(pane) = tab.active_pane_mut() else {
            return;
        };

        if let Some(text) = event.text_with_all_modifiers() {
            if !text.is_empty() {
                let first_char = text.chars().next().unwrap();
                let char_code = first_char as u32;
                if (1..=26).contains(&char_code) || char_code == 0x7F {
                    log::debug!(
                        "Sending control character from text_with_all_modifiers: {:?} (0x{:02x})",
                        first_char,
                        first_char as u8
                    );
                    let _ = pane.child.write_all(&[first_char as u8]);
                    return;
                }
            }
        }

        if let Key::Character(c) = &event.logical_key {
            if let Some(ch) = c.chars().next() {
                let char_code = ch as u32;
                if (1..=26).contains(&char_code) || char_code == 0x7F {
                    log::debug!(
                        "Sending control character from logical_key: {:?} (0x{:02x})",
                        ch,
                        ch as u8
                    );
                    let _ = pane.child.write_all(&[ch as u8]);
                    return;
                }
            }
        }

        #[cfg(target_os = "macos")]
        let zoom_modifier = self.modifiers.super_key();
        #[cfg(not(target_os = "macos"))]
        let zoom_modifier = self.modifiers.control_key() && self.modifiers.shift_key();

        if zoom_modifier {
            match &event.logical_key {
                Key::Character(c) if c == "=" || c == "+" => {
                    self.change_font_size(2.0);
                    return;
                }
                Key::Character(c) if c == "-" => {
                    self.change_font_size(-2.0);
                    return;
                }
                Key::Character(c) if c == "0" => {
                    self.reset_font_size();
                    return;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.change_font_size(2.0);
                    return;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    self.change_font_size(-2.0);
                    return;
                }
                _ => {}
            }
        }

        let application_cursor_keys = pane.terminal.screen().modes().cursor_keys_application;

        if let Some(data) = encode_key(&event.logical_key, self.modifiers, application_cursor_keys)
        {
            log::debug!("Sending key data: {:?}", data);
            let _ = pane.child.write_all(&data);
        }
    }

    fn change_font_size(&mut self, delta: f32) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let Some(window) = &self.window else { return };

        let current_size = renderer.font_size();
        let new_size = (current_size + delta).clamp(8.0, 72.0);

        if (new_size - current_size).abs() < 0.1 {
            return;
        }

        renderer.set_font_size(new_size);

        let size = window.inner_size();
        let cell_size = renderer.cell_size();
        self.tab_bar_height = compute_tab_bar_height(&cell_size);
        let content_area = Rect::new(
            0,
            self.tab_bar_height,
            size.width,
            size.height.saturating_sub(self.tab_bar_height),
        );

        for tab in &mut self.tabs {
            let rects = tab.split_root.compute_rects(content_area);
            for (pane_id, rect) in &rects {
                let cols = (rect.width as f32 / cell_size.width) as usize;
                let rows = (rect.height as f32 / cell_size.height) as usize;
                if cols > 0 && rows > 0 {
                    if let Some(pane) = tab.pane_by_id_mut(*pane_id) {
                        pane.terminal.resize(cols, rows);
                        let _ = pane.child.resize(WindowSize::new(cols as u16, rows as u16));
                    }
                }
            }
        }

        self.needs_redraw = true;
    }

    fn reset_font_size(&mut self) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let Some(window) = &self.window else { return };

        let scale_factor = window.scale_factor() as f32;
        let default_size = self.config.font_size() * scale_factor;

        renderer.set_font_size(default_size);

        let size = window.inner_size();
        let cell_size = renderer.cell_size();
        self.tab_bar_height = compute_tab_bar_height(&cell_size);
        let content_area = Rect::new(
            0,
            self.tab_bar_height,
            size.width,
            size.height.saturating_sub(self.tab_bar_height),
        );

        for tab in &mut self.tabs {
            let rects = tab.split_root.compute_rects(content_area);
            for (pane_id, rect) in &rects {
                let cols = (rect.width as f32 / cell_size.width) as usize;
                let rows = (rect.height as f32 / cell_size.height) as usize;
                if cols > 0 && rows > 0 {
                    if let Some(pane) = tab.pane_by_id_mut(*pane_id) {
                        pane.terminal.resize(cols, rows);
                        let _ = pane.child.resize(WindowSize::new(cols as u16, rows as u16));
                    }
                }
            }
        }

        self.needs_redraw = true;
    }

    fn pane_at_pixel(&self, px: f64, py: f64) -> Option<PaneId> {
        if self.tabs.is_empty() {
            return None;
        }
        let content = self.content_area();
        let tab = &self.tabs[self.active_tab];
        let rects = tab.split_root.compute_rects(content);
        for (pane_id, rect) in &rects {
            if rect.contains(px, py) {
                return Some(*pane_id);
            }
        }
        None
    }

    fn pane_rect(&self, pane_id: PaneId) -> Option<Rect> {
        if self.tabs.is_empty() {
            return None;
        }
        let content = self.content_area();
        let tab = &self.tabs[self.active_tab];
        let rects = tab.split_root.compute_rects(content);
        rects.iter().find(|(id, _)| *id == pane_id).map(|(_, r)| *r)
    }

    fn is_on_divider(&self, px: f64, py: f64) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        let content = self.content_area();
        let tab = &self.tabs[self.active_tab];
        let dividers = tab.split_root.compute_dividers(content);
        dividers.iter().any(|(rect, _)| rect.contains(px, py))
    }

    fn handle_mouse_input(&mut self, button: MouseButton, state: ElementState) {
        if self.tabs.is_empty() {
            return;
        }

        if button == MouseButton::Left
            && state == ElementState::Pressed
            && self.mouse_pixel.1 < self.tab_bar_height as f64
        {
            self.handle_tab_bar_click(self.mouse_pixel.0);
            return;
        }

        if button == MouseButton::Left {
            if state == ElementState::Pressed {
                if self.is_on_divider(self.mouse_pixel.0, self.mouse_pixel.1) {
                    self.divider_dragging = true;
                    self.divider_drag_last = self.mouse_pixel;
                    return;
                }

                if let Some(window) = &self.window {
                    let window_width = window.inner_size().width as f64;
                    let scrollbar_width = 12.0;

                    if self.mouse_pixel.0 >= window_width - scrollbar_width
                        && self.mouse_pixel.1 >= self.tab_bar_height as f64
                    {
                        let tab = &self.tabs[self.active_tab];
                        if let Some(pane) = tab.active_pane() {
                            let scrollback_len = pane.terminal.screen().scrollback().len();
                            if scrollback_len > 0 {
                                self.scrollbar_dragging = true;
                                self.scrollbar_drag_start_y = self.mouse_pixel.1;
                                self.scrollbar_drag_start_offset = pane.scroll_offset;
                                return;
                            }
                        }
                    }
                }

                if let Some(clicked_pane) =
                    self.pane_at_pixel(self.mouse_pixel.0, self.mouse_pixel.1)
                {
                    let tab = &mut self.tabs[self.active_tab];
                    if tab.active_pane_id != clicked_pane {
                        tab.active_pane_id = clicked_pane;
                        self.needs_redraw = true;
                    }
                }
            } else {
                if self.divider_dragging {
                    self.divider_dragging = false;
                    return;
                }
                if self.scrollbar_dragging {
                    self.scrollbar_dragging = false;
                    return;
                }
            }
        }

        let tab = &mut self.tabs[self.active_tab];
        let active_id = tab.active_pane_id;
        let Some(pane) = tab.pane_by_id_mut(active_id) else {
            return;
        };
        let modes = pane.terminal.screen().modes().clone();

        if !modes.mouse_tracking_enabled() {
            if button == MouseButton::Left {
                let col = self.mouse_cell.0 as usize;
                let row = self.mouse_cell.1 as isize - pane.scroll_offset as isize;

                if state == ElementState::Pressed {
                    pane.terminal
                        .screen_mut()
                        .selection_mut()
                        .start(Point::new(col, row), SelectionType::Normal);
                    self.needs_redraw = true;
                } else {
                    pane.terminal.screen_mut().selection_mut().finish();
                }
            }
            let idx = match button {
                MouseButton::Left => 0,
                MouseButton::Middle => 1,
                MouseButton::Right => 2,
                _ => return,
            };
            self.mouse_buttons[idx] = state == ElementState::Pressed;
            return;
        }

        let event = if state == ElementState::Pressed {
            MouseEvent::Press(button, self.mouse_cell.0, self.mouse_cell.1)
        } else {
            MouseEvent::Release(button, self.mouse_cell.0, self.mouse_cell.1)
        };

        if let Some(data) = encode_mouse(
            event,
            modes.mouse_sgr,
            modes.mouse_button_event,
            modes.mouse_any_event,
        ) {
            let _ = pane.child.write_all(&data);
        }

        let idx = match button {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
            _ => return,
        };
        self.mouse_buttons[idx] = state == ElementState::Pressed;
    }

    fn handle_mouse_motion(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let old_pixel = self.mouse_pixel;
        self.mouse_pixel = (position.x, position.y);

        if self.tabs.is_empty() {
            return;
        }

        if self.divider_dragging {
            let delta_x = position.x - self.divider_drag_last.0;
            let delta_y = position.y - self.divider_drag_last.1;
            let content = self.content_area();
            let tab = &mut self.tabs[self.active_tab];
            if tab
                .split_root
                .adjust_divider_at(old_pixel.0, old_pixel.1, content, delta_x, delta_y)
            {
                self.divider_drag_last = (position.x, position.y);
                self.resize_all_panes_in_tab();
                self.needs_redraw = true;
            }
            return;
        }

        if self.scrollbar_dragging {
            if let Some(window) = &self.window {
                let tab = &mut self.tabs[self.active_tab];
                let active_id = tab.active_pane_id;
                if let Some(pane) = tab.pane_by_id_mut(active_id) {
                    let window_height =
                        (window.inner_size().height as f64 - self.tab_bar_height as f64).max(1.0);
                    let scrollback_len = pane.terminal.screen().scrollback().len();
                    let visible_rows = pane.terminal.screen().rows();

                    if scrollback_len > 0 && window_height > 0.0 {
                        let delta_y = position.y - self.scrollbar_drag_start_y;
                        let total_lines = scrollback_len + visible_rows;
                        let thumb_height =
                            ((visible_rows as f64 / total_lines as f64) * window_height).max(20.0);
                        let scroll_range = window_height - thumb_height;

                        if scroll_range > 0.0 {
                            let scroll_delta =
                                (-delta_y / scroll_range * scrollback_len as f64) as isize;
                            let new_offset = (self.scrollbar_drag_start_offset as isize
                                + scroll_delta)
                                .max(0)
                                .min(scrollback_len as isize)
                                as usize;

                            if new_offset != pane.scroll_offset {
                                pane.scroll_offset = new_offset;
                                self.needs_redraw = true;
                            }
                        }
                    }
                }
            }
            return;
        }

        let Some(renderer) = &self.renderer else {
            return;
        };

        let cell_size = renderer.cell_size();
        let tab = &self.tabs[self.active_tab];
        let active_id = tab.active_pane_id;

        let pane_rect = self.pane_rect(active_id).unwrap_or(self.content_area());
        let adjusted_x = (position.x - pane_rect.x as f64).max(0.0);
        let adjusted_y = (position.y - pane_rect.y as f64).max(0.0);
        let col = (adjusted_x / cell_size.width as f64) as u16;
        let row = (adjusted_y / cell_size.height as f64) as u16;

        if col == self.mouse_cell.0 && row == self.mouse_cell.1 {
            return;
        }

        self.mouse_cell = (col, row);

        let tab = &mut self.tabs[self.active_tab];
        let Some(pane) = tab.pane_by_id_mut(active_id) else {
            return;
        };
        let modes = pane.terminal.screen().modes().clone();

        if !modes.mouse_tracking_enabled() && self.mouse_buttons[0] {
            let sel_col = col as usize;
            let sel_row = row as isize - pane.scroll_offset as isize;
            pane.terminal
                .screen_mut()
                .selection_mut()
                .update(Point::new(sel_col, sel_row));
            self.needs_redraw = true;
            return;
        }

        if modes.mouse_any_event
            || (modes.mouse_button_event && self.mouse_buttons.iter().any(|&b| b))
        {
            let event = MouseEvent::Move(col, row);
            if let Some(data) = encode_mouse(
                event,
                modes.mouse_sgr,
                modes.mouse_button_event,
                modes.mouse_any_event,
            ) {
                let _ = pane.child.write_all(&data);
            }
        }
    }

    fn handle_mouse_scroll(&mut self, delta: MouseScrollDelta) {
        if self.tabs.is_empty() {
            return;
        }

        let tab = &mut self.tabs[self.active_tab];
        let active_id = tab.active_pane_id;
        let Some(pane) = tab.pane_by_id_mut(active_id) else {
            return;
        };
        let modes = pane.terminal.screen().modes().clone();
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => y as i32,
            MouseScrollDelta::PixelDelta(pos) => (pos.y / 20.0) as i32,
        };

        if lines == 0 {
            return;
        }

        if modes.mouse_tracking_enabled() || modes.alternate_screen {
            let event = MouseEvent::Scroll {
                x: self.mouse_cell.0,
                y: self.mouse_cell.1,
                delta: lines as i8,
            };
            if let Some(data) = encode_mouse(
                event,
                modes.mouse_sgr,
                modes.mouse_button_event,
                modes.mouse_any_event,
            ) {
                let _ = pane.child.write_all(&data);
            }
        } else {
            let scrollback_len = pane.terminal.screen().scrollback().len();
            if lines > 0 {
                pane.scroll_offset = (pane.scroll_offset + lines as usize).min(scrollback_len);
            } else {
                pane.scroll_offset = pane.scroll_offset.saturating_sub((-lines) as usize);
            }
            self.needs_redraw = true;
        }
    }

    fn handle_copy(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let tab = &self.tabs[self.active_tab];
        let Some(pane) = tab.active_pane() else {
            return;
        };

        let screen = pane.terminal.screen();
        let selection = screen.selection();

        if selection.is_empty() {
            return;
        }

        let (start, end) = selection.bounds();
        let mut text = String::new();
        let cols = screen.cols();

        for row in start.row..=end.row {
            let start_col = if row == start.row { start.col } else { 0 };
            let end_col = if row == end.row { end.col } else { cols };

            if row < 0 {
                let scrollback_idx = (-row - 1) as usize;
                if let Some(line) = screen.scrollback().get_from_end(scrollback_idx) {
                    let line_text = line.text();
                    let chars: Vec<char> = line_text.chars().collect();
                    for ch in chars.iter().take(end_col.min(chars.len())).skip(start_col) {
                        text.push(*ch);
                    }
                }
            } else if (row as usize) < screen.grid().rows() {
                let line = screen.line(row as usize);
                let line_text = line.text();
                let chars: Vec<char> = line_text.chars().collect();
                for ch in chars.iter().take(end_col.min(chars.len())).skip(start_col) {
                    text.push(*ch);
                }
            }

            if row < end.row {
                while text.ends_with(' ') {
                    text.pop();
                }
                text.push('\n');
            }
        }

        let text = text.trim_end().to_string();

        if text.is_empty() {
            return;
        }

        let Some(clipboard) = &mut self.clipboard else {
            return;
        };

        if let Err(e) = clipboard.set_text(&text) {
            log::warn!("Failed to copy to clipboard: {}", e);
        } else {
            log::debug!("Copied {} bytes to clipboard", text.len());
        }
    }

    fn handle_paste(&mut self) {
        let Some(clipboard) = &mut self.clipboard else {
            log::warn!("Clipboard not available");
            return;
        };
        if self.tabs.is_empty() {
            return;
        }

        let tab = &mut self.tabs[self.active_tab];
        let Some(pane) = tab.active_pane_mut() else {
            return;
        };

        match clipboard.get_text() {
            Ok(text) => {
                if text.is_empty() {
                    return;
                }
                let data = if pane.terminal.screen().modes().bracketed_paste {
                    encode_bracketed_paste(&text)
                } else {
                    text.into_bytes()
                };
                if let Err(e) = pane.child.write_all(&data) {
                    log::warn!("Failed to write paste data to PTY: {}", e);
                } else {
                    log::debug!("Pasted {} bytes", data.len());
                }
            }
            Err(e) => {
                log::warn!("Failed to get clipboard text: {}", e);
            }
        }
    }

    fn handle_find(&mut self) {
        log::info!("Find requested (Ctrl+Shift+F) - search UI not yet implemented");
    }

    #[cfg(target_os = "macos")]
    fn handle_new_window(&mut self) {
        log::info!("Opening new terminal window...");

        if let Ok(exe_path) = std::env::current_exe() {
            match std::process::Command::new(&exe_path).spawn() {
                Ok(child) => {
                    log::info!("New terminal window spawned successfully");
                    std::thread::spawn(move || {
                        let mut child = child;
                        let _ = child.wait();
                    });
                }
                Err(e) => {
                    log::error!("Failed to spawn new terminal window: {}", e);
                }
            }
        } else {
            log::error!("Failed to get current executable path");
        }
    }

    fn handle_reload_config(&mut self) {
        log::info!("Reloading configuration...");

        match Config::load() {
            Some(new_config) => {
                self.config.theme = new_config.theme;
                self.config.font = new_config.font.clone();
                self.config.keybindings = new_config.keybindings.clone();
                self.config.security = new_config.security.clone();

                if let Some(renderer) = &mut self.renderer {
                    renderer.set_colors(self.config.effective_colors());
                }

                log::info!("Configuration reloaded successfully");
                self.needs_redraw = true;
            }
            None => {
                log::warn!("No config file found or failed to parse");
            }
        }
    }

    #[allow(dead_code)]
    fn handle_toggle_theme(&mut self) {
        let new_theme = self.config.theme.next();
        log::info!(
            "Switching theme from {:?} to {:?}",
            self.config.theme,
            new_theme
        );

        self.config.theme = new_theme;

        if let Some(renderer) = &mut self.renderer {
            renderer.set_colors(self.config.effective_colors());
        }

        self.needs_redraw = true;
    }

    fn handle_focus(&mut self, focused: bool) {
        self.focused = focused;

        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        let Some(pane) = tab.active_pane_mut() else {
            return;
        };

        if pane.terminal.screen().modes().focus_events {
            let data = encode_focus(focused);
            let _ = pane.child.write_all(&data);
        }
    }

    fn poll_pty(&mut self) {
        let mut buf = [0u8; 65536];

        for (tab_idx, tab) in self.tabs.iter_mut().enumerate() {
            for pane in &mut tab.panes {
                let mut received_output = false;

                loop {
                    match pane.child.pty_mut().try_read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            pane.terminal.process(&buf[..n]);
                            received_output = true;
                            if tab_idx == self.active_tab && !pane.terminal.is_synchronized_output()
                            {
                                self.needs_redraw = true;
                            }
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                        Err(_) => break,
                    }
                }

                if received_output && pane.scroll_offset > 0 {
                    pane.scroll_offset = 0;
                }

                if pane.terminal.take_title_changed() {
                    pane.title = pane.terminal.title().to_string();
                    if tab_idx == self.active_tab && pane.id == tab.active_pane_id {
                        if let Some(window) = &self.window {
                            window.set_title(&pane.title);
                        }
                    }
                }

                if pane.terminal.take_bell() {
                    log::debug!("Bell!");
                }

                let responses = pane.terminal.take_pending_responses();
                for response in responses {
                    if let Err(e) = pane.child.write_all(&response) {
                        log::warn!("Failed to send response to PTY: {}", e);
                    }
                }
            }
        }
    }

    fn render(&mut self) {
        if self.renderer.is_none() || self.tabs.is_empty() {
            return;
        }

        let content = self.content_area();
        let tab_bar_height = self.tab_bar_height;
        let active_tab = self.active_tab;

        let tab_infos: Vec<TabInfo<'_>> = self
            .tabs
            .iter()
            .map(|t| TabInfo { title: t.title() })
            .collect();

        let tab = &self.tabs[active_tab];
        let rects = tab.split_root.compute_rects(content);
        let dividers = tab.split_root.compute_dividers(content);

        let mut pane_infos: Vec<PaneRenderInfo<'_>> = Vec::new();
        for (pane_id, rect) in &rects {
            if let Some(pane) = tab.pane_by_id(*pane_id) {
                let screen = pane.terminal.screen();
                let selection = screen.selection();
                pane_infos.push(PaneRenderInfo {
                    screen,
                    selection,
                    scroll_offset: pane.scroll_offset,
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height,
                    is_active: *pane_id == tab.active_pane_id,
                });
            }
        }

        let divider_rects: Vec<(u32, u32, u32, u32)> = dividers
            .iter()
            .map(|(r, _)| (r.x, r.y, r.width, r.height))
            .collect();

        let renderer = self.renderer.as_mut().unwrap();
        if let Err(e) = renderer.render_split(
            &pane_infos,
            &divider_rects,
            tab_bar_height,
            &tab_infos,
            active_tab,
        ) {
            log::warn!("Render error: {:?}", e);
        }

        self.needs_redraw = false;
        self.last_render = Instant::now();
    }

    fn check_child(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }

        for tab in &mut self.tabs {
            tab.panes.retain(|pane| pane.child.is_running());

            if tab.panes.is_empty() {
                continue;
            }

            let remaining_ids = tab.split_root.collect_pane_ids();
            let live_ids: Vec<PaneId> = tab.panes.iter().map(|p| p.id).collect();

            for id in &remaining_ids {
                if !live_ids.contains(id) {
                    if let Some(replacement) = tab.split_root.remove_pane(*id) {
                        tab.split_root = replacement;
                    }
                }
            }

            if !live_ids.contains(&tab.active_pane_id) {
                tab.active_pane_id = tab.panes[0].id;
            }
        }

        self.tabs.retain(|tab| tab.has_running_children());

        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len().saturating_sub(1);
        }

        !self.tabs.is_empty()
    }
}
