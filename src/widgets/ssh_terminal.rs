use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Clone)]
pub struct SshTerminal {
    pub device_name: String,
    pub content: Vec<String>,
}

impl SshTerminal {
    pub fn new(device_name: String) -> Self {
        Self {
            device_name,
            content: Vec::new(),
        }
    }

    pub fn start(&mut self, stdout_rx: Receiver<String>) {
        let device = self.device_name.clone();
        std::thread::spawn(move || {
            while let Ok(output) = stdout_rx.recv() {
                if let Ok(_) = crate::window::GLOBAL_MESSAGE_SENDER.send(
                    crate::window::Message::SSHTerminalOutput {
                        device: device.clone(),
                        output,
                    },
                ) {}
            }
        });
    }
}
