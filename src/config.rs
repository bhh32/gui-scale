use serde::ser::Error;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct UIConfig {
    pub font_size: u16,
    pub theme: String,
    pub window_width: u32,
    pub window_height: u32,
    pub enable_notifications: bool,
    pub enable_sounds: bool,
}

impl UIConfig {
    pub fn save(&self) -> serde_json::Result<()> {
        let file = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open("ui_config.json")
        {
            Ok(file) => file,
            Err(e) => return Err(serde_json::error::Error::custom(e.to_string())),
        };

        match serde_json::to_writer(file, self) {
            Ok(_) => {
                println!("Configuration file saved successfully!");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub fn load() -> serde_json::Result<Self> {
        if let Ok(mut file) = File::open("ui_config.json") {
            let mut contents = String::new();
            match file.read_to_string(&mut contents) {
                Ok(_) => println!("Contents read successfully!"),
                Err(e) => return Err(serde_json::error::Error::custom(e.to_string())),
            }

            match serde_json::from_str(&contents) {
                Ok(contents) => Ok(contents),
                Err(e) => return Err(e),
            }
        } else {
            Ok(UIConfig::default())
        }
    }
}
