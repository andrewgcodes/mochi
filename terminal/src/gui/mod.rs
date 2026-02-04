//! GUI Module
//!
//! Provides the graphical user interface for the terminal emulator.
//! Uses winit for window management and softbuffer for rendering.

mod font;
mod input;
mod renderer;
mod selection;

use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{debug, error, info, warn};
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event::{
    ElementState, Event, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use crate::core::{EraseMode, MouseEncoding, MouseMode, Screen, TabClearMode};
use crate::parser::{Action, ControlCode, CsiAction, EscAction, OscAction, Parser};
use crate::pty::Pty;

pub use font::FontRenderer;
pub use input::InputEncoder;
pub use renderer::Renderer;
pub use selection::Selection;

#[derive(Debug, Clone)]
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub font_size: f32,
    pub font_family: String,
    pub scrollback_size: usize,
    pub osc52_enabled: bool,
    pub osc52_max_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            font_size: 14.0,
            font_family: "monospace".to_string(),
            scrollback_size: 10000,
            osc52_enabled: false,
            osc52_max_size: 65536,
        }
    }
}

pub fn run(_config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // GUI implementation requires winit 0.28 compatible code
    // This is a placeholder that will be filled in when we have the correct API
    unimplemented!("GUI not yet implemented for winit 0.28")
}

impl Config {
    pub fn osc52_enabled(&self) -> bool {
        self.osc52_enabled
    }
}
