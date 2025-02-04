use serde::ser::Error;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

#[derive(Serialize, Deserialize, Debug)]
pub struct UIConfig {
    // Basic Settings
    pub font_size: u16, // range: 10 - 24
    pub theme: String,  // "system", "light", or "dark"
    pub window_width: u32,
    pub window_height: u32,
    pub enable_notifications: bool, // Master notifications toggle
    pub enable_sounds: bool,        // Master sounds toggle

    // Tailscale related toggles
    pub ssh_enabled: bool,    // "tailscale up --ssh=true/false"
    pub accept_routes: bool,  // "tailscale up --accept-routes=true/false"
    pub is_exit_node: bool,   // "tailscale up --advertise-exit-node=true/false"
    pub use_exit_node: bool,  // "tailscale up --exit-node=exit-node-name"
    pub connect_to_lan: bool, // "tailscale up --exit-node-allow-lan-access=true/false"

    // Auto-receive file toggles
    pub auto_receive_files: bool, // Automatically receive files from other devices
    // Whether to disable notifications for received files
    // NOTE: If auto_receive is disabled, we cannot disable these notifications
    pub disable_received_file_notifications: bool,
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            font_size: 14,
            theme: "system".to_string(),
            window_width: 1024,
            window_height: 768,
            enable_notifications: true,
            enable_sounds: true,
            ssh_enabled: false, // Should be automatically set based on what is currently set
            accept_routes: false, // Should be automatically set based on what's currently set
            is_exit_node: false, // Should be automatically set based on what's currently set
            use_exit_node: false, // Should be automatically set based on what's currently set
            connect_to_lan: false, // Should be automatically set based on what's currently set
            auto_receive_files: true,
            disable_received_file_notifications: false,
        }
    }
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
                Ok(cfg) => Ok(cfg),
                Err(e) => return Err(e),
            }
        } else {
            Ok(UIConfig::default())
        }
    }
}
