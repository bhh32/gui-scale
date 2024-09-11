pub mod logic;
pub mod messages;
mod  model;

use model::GuiScale;
use iced::{Sandbox, Settings};

fn main() -> iced::Result {
    GuiScale::run(Settings::default())
}