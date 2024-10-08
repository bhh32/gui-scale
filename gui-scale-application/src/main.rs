pub mod logic;
pub mod messages;
mod  model;

use model::GuiScale;
use iced::{Application, Settings, Size};

fn main() -> iced::Result {
    let mut custom_settings = Settings::default();
    custom_settings.window.size = Size::new(740.0, 175.0);
    //custom_settings.window.resizable = false;
    GuiScale::run(custom_settings)
}
