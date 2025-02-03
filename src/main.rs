mod config;
mod logic;
pub mod widgets;
mod window;

use crate::window::Window;

fn main() -> iced::Result {
    iced::run("GUI Scale", Window::update, Window::view)
}
