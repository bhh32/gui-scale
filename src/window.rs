use crate::{
    logic::{
        check_for_updates, check_tailscale, get_tailscale_devices, notify, play_sound,
        run_ssh_session, send_ssh_command, tailscale_recieve, tailscale_send,
    },
    widgets::tab::tab_bar::TabBar,
    widgets::tab::Tab,
};
use iced::{
    widget::{button, column, container, row, text, text_input, Column, ProgressBar},
    Element, Length, Theme,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct Window {
    active_tab: Option<String>,
    tabs: TabBar<Tab, Message>,
    ssh_sessions: Arc<Mutex<HashMap<String, String>>>,
    ssh_input: HashMap<String, String>,
    active_ssh_sessions: Arc<Mutex<HashMap<String, bool>>>,
    devices: Arc<Mutex<Vec<String>>>,
    selected_device: Option<String>,
    send_progress: f32,
    receive_progress: f32,
    font_size: u16,
    theme: Theme,
    compact_mode: bool,
    enable_notifications: bool,
    enable_sounds: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    ConnectSSH(String, String),
    SendSSHCommand(String),
    FileTransfer(String),
    ToggleTheme,
    ToggleNotifications,
    ToggleSounds,
    RefreshDevices,
    UpdateSSHInput(String, String),
    ToggleCompactMode,
    CheckForUpdates,
    UpdateSendProgress(f32),
    UpdateReceiveProgress(f32),
    SwitchTab(String),
    TerminateSSH(String),
}

impl Window {
    pub fn new() -> Self {
        // Ensure that tailscale is installed
        check_tailscale();
        // Check for updates
        //check_for_updates();

        let tabs = TabBar::<Tab, Message>::new(Tab::new("No Sessions"), || {
            column![text("No active SSH sessions")].into()
        });

        let active_ssh_sessions = Arc::new(Mutex::new(HashMap::new()));
        // Get the device list
        let devices = Arc::new(Mutex::new(get_tailscale_devices()));

        let devices_clone = Arc::clone(&devices);

        // Start the device update thread
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(10));
            let mut devices_lock = devices_clone.lock().unwrap();
            *devices_lock = get_tailscale_devices();
        });

        Self {
            ssh_sessions: Arc::new(Mutex::new(HashMap::new())),
            ssh_input: HashMap::new(),
            active_ssh_sessions,
            tabs,
            devices,
            selected_device: None,
            font_size: 14,
            theme: Theme::Dark,
            enable_notifications: true,
            enable_sounds: true,
            compact_mode: false,
            send_progress: 0.0,
            receive_progress: 0.0,
            active_tab: None,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ConnectSSH(device, ip) => {
                run_ssh_session(device.clone(), ip);
                self.active_tab = Some(device.clone());

                let mut active_sessions = self.active_ssh_sessions.lock().unwrap();
                active_sessions.insert(device.clone(), true);

                // Add new tab for the SSH session
                let device_clone = device.clone();
                let ssh_input = self.ssh_input.clone();

                self.tabs.push(Tab::new(&device.clone()), move || {
                    let device_ref = device_clone.clone();
                    let device_send = device_clone.clone();
                    column![
                        text(format!("SSH Session: {}", device_ref)),
                        text_input(
                            "Enter command...",
                            ssh_input.get(&device_ref).unwrap_or(&String::new()),
                        )
                        .on_input(move |input| {
                            let device_input = device_ref.clone();
                            Message::UpdateSSHInput(device_input, input)
                        }),
                        button(text("Send")).on_press(Message::SendSSHCommand(device_send))
                    ]
                    .into()
                });
            }
            Message::SendSSHCommand(device) => {
                if let Some(command) = self.ssh_input.get(&device).cloned() {
                    send_ssh_command(device, command);
                }
            }
            Message::TerminateSSH(device) => {
                let mut active_sessions = self.active_ssh_sessions.lock().unwrap();
                active_sessions.remove(&device);
                if self.active_tab == Some(device.clone()) {
                    self.active_tab = None;
                }
            }
            Message::FileTransfer(_device) => {
                if let Some(selected) = self.selected_device.clone() {
                    tailscale_send(vec!["example.txt".to_string()], &selected);
                }
            }
            Message::ToggleTheme => {
                self.theme = match self.theme {
                    Theme::Dark => Theme::Light,
                    _ => Theme::Dark,
                };
            }
            Message::ToggleNotifications => self.enable_notifications = !self.enable_notifications,
            Message::ToggleSounds => self.enable_sounds = !self.enable_sounds,
            Message::RefreshDevices => {
                let mut devices_lock = self.devices.lock().unwrap();
                *devices_lock = get_tailscale_devices();
            }
            Message::UpdateSSHInput(device, input) => {
                self.ssh_input.insert(device, input);
            }
            Message::ToggleCompactMode => {
                self.compact_mode = !self.compact_mode;
            }
            Message::UpdateSendProgress(progress) => {
                self.send_progress = progress;
            }
            Message::UpdateReceiveProgress(progress) => {
                self.receive_progress = progress;
            }
            Message::SwitchTab(device) => {
                self.active_tab = Some(device);
            }
            Message::CheckForUpdates => {
                check_for_updates();
            }
        }
    }

    pub fn view(&self) -> Element<Message, Theme> {
        let devices_lock = self.devices.lock().unwrap();
        let device_list: Element<Message, Theme> = {
            // Create a row for each device
            let rows = devices_lock.iter().map(|device| {
                row![
                    text(device.clone()).size(16),
                    ProgressBar::new(0.0..=1.0, self.send_progress),
                    button(text("Send")).on_press(Message::FileTransfer(device.clone())),
                    button(text("Connect")).on_press(Message::ConnectSSH(
                        device.clone(),
                        "tailscale-ip".to_string(),
                    ))
                ]
                .spacing(10)
                .padding(5)
                .into()
            });
            column(rows).spacing(10).padding(20)
        }
        .into();

        let active_sessions = self.active_ssh_sessions.lock().unwrap();
        let active_sessions_vec: Vec<_> = active_sessions.keys().cloned().collect();

        // Create the initial tab
        let tabs = if let Some(first_device) = active_sessions_vec.first() {
            let first_device_owned = first_device.to_string();
            let first_device_ref = first_device_owned.clone();
            let ssh_input = self.ssh_input.clone();
            let mut tabs: TabBar<Tab, Message> =
                TabBar::<_, Message>::new(Tab::new(&first_device_owned), move || {
                    let device_clone = first_device_ref.clone();
                    let content: Column<Message> = column![
                        text(format!("SSH Session: {}", device_clone)),
                        text_input(
                            "Enter command...",
                            ssh_input.get(&device_clone).unwrap_or(&String::new()),
                        )
                        .on_input({
                            let device_clone = device_clone.clone();
                            move |input| Message::UpdateSSHInput(device_clone.clone(), input)
                        }),
                        button(button("Send"))
                            .on_press(Message::SendSSHCommand(device_clone.clone()))
                    ];

                    content.into()
                })
                .into();
        };
        // Main container
        container(
            column![
                // Title section
                container(text("GUI Scale").size(24))
                    .padding(20)
                    .center_x(Length::Fill),
                // Devices section
                container(column![text("Devices").size(20), device_list].spacing(10)).padding(10),
                // Tabs section
                container(self.tabs.view(Message::SwitchTab)).padding(10),
                // Settings section
                container(column![row![text("Font Size:").size(16)].padding(10)]).padding(10)
            ]
            .spacing(20),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .into()
    }
}

impl Default for Window {
    fn default() -> Self {
        Window::new()
    }
}
