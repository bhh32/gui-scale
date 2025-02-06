mod config;
mod logic;
pub mod widgets;
mod window;

use crate::window::Window;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmic::app::run::<Window>(cosmic::app::Settings::default(), ())?;

    Ok(())
}
