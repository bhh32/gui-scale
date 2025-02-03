mod config;
mod logic;
pub mod widgets;
mod window;

use crate::window::Window;
use iced::{Application, Settings};

fn main() -> iced::Result {
    Window::run(Settings::default())
}
