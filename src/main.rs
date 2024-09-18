pub mod logic;
pub mod messages;
mod  model;

use std::any::Any;

use model::GuiScale;
use iced::{window::{self, icon}, Application, Settings, Size};

fn main() -> iced::Result {
    let mut custom_settings = Settings::default();
    custom_settings.window.size = Size::new(740.0, 125.0);
    custom_settings.window.resizable = false;
    GuiScale::run(custom_settings)
}