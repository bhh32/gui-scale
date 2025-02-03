use crate::{
    logic::{
        check_for_updates, check_tailscale, get_tailscale_devices, notify, play_sound,
        run_ssh_session, send_ssh_command, tailscale_recieve, tailscale_send,
    },
    widgets::tab::tab_bar::TabBar,
    widgets::tab::Tab,
};
use iced::{
    executor,
    widget::{Button, Column, Container, ProgressBar, Row, Slider, Text, TextInput, Toggler},
    Application, Command, Element, Length, Renderer, Settings, Theme,
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

impl Application for Window {
    type Executor = executor::Default;
    type Flags = ();
    type Theme = Theme;
    type Message = Message;

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        // Ensure that tailscale is installed
        check_tailscale();
        // Check for updates
        check_for_updates();

        let tabs = TabBar::<Tab, Message>::new(Tab::new("No Sessions"), || {
            Column::new()
                .push(Text::new("No active SSH sessions"))
                .into()
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

        let app_window = Self {
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
        };

        (app_window, Command::none())
    }

    fn title(&self) -> String {
        String::from("GUI Scale")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
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
                    Column::new()
                        .push(Text::new(format!("SSH Session: {}", device_ref)))
                        .push(
                            TextInput::new(
                                "Enter command...",
                                ssh_input.get(&device_ref).unwrap_or(&String::new()),
                            )
                            .on_input(move |input| {
                                let device_input = device_ref.clone();
                                Message::UpdateSSHInput(device_input, input)
                            }),
                        )
                        .push(
                            Button::new(Text::new("Send"))
                                .on_press(Message::SendSSHCommand(device_send)),
                        )
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

        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        let devices_lock = self.devices.lock().unwrap();
        let device_list = devices_lock.iter().enumerate().fold(
            Column::new().spacing(10),
            |column, (_index, device)| {
                let progress = ProgressBar::new(0.0..=1.0, self.send_progress);

                let send_button =
                    Button::new(Text::new("Send")).on_press(Message::FileTransfer(device.clone()));

                let ssh_button = Button::new(Text::new("Connect")).on_press(Message::ConnectSSH(
                    device.clone(),
                    "tailscale-ip".to_string(),
                ));

                column.push(
                    Row::new()
                        .push(Text::new(device.clone()).size(16))
                        .push(progress)
                        .push(send_button)
                        .push(ssh_button)
                        .spacing(10),
                )
            },
        );

        let active_sessions = self.active_ssh_sessions.lock().unwrap();
        let active_sessions_vec: Vec<_> = active_sessions.keys().cloned().collect();

        // Create the initial tab
        let tabs = if let Some(first_device) = active_sessions_vec.first() {
            let first_device_owned = first_device.to_string();
            let first_device_ref = first_device_owned.clone();
            let ssh_input = self.ssh_input.clone();
            let mut tabs = TabBar::<_, Message>::new(Tab::new(&first_device_owned), move || {
                let device_clone = first_device_ref.clone();
                let content = Column::new()
                    .push(Text::new(format!("SSH Session: {}", device_clone)))
                    .push(
                        TextInput::new(
                            "Enter command...",
                            ssh_input.get(&device_clone).unwrap_or(&String::new()),
                        )
                        .on_input({
                            let device_clone = device_clone.clone();
                            move |input| Message::UpdateSSHInput(device_clone.clone(), input)
                        }),
                    )
                    .push(
                        Button::new(Text::new("Send"))
                            .on_press(Message::SendSSHCommand(device_clone.clone())),
                    );

                content.into()
            });
        };

        // Additional tabs

        let settings_section = Column::new().push(Row::new().push(Text::new("Font Size:")));
        let view = self.tabs.view(Message::SwitchTab);

        Column::new()
            .push(device_list)
            .push(view)
            .push(settings_section)
            .into()
    }
}
