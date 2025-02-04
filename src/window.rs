use crate::{
    config::UIConfig,
    logic::{
        check_for_updates, check_tailscale, get_tailscale_devices, get_tailscale_status, notify,
        play_sound, run_ssh_session, send_ssh_command, tailscale_down, tailscale_recieve,
        tailscale_send, tailscale_up, terminate_ssh_session,
    },
    widgets::tab::tab_bar::TabBar,
    widgets::tab::{IsTab, Tab},
};
use iced::{
    alignment::{self, Horizontal, Vertical},
    futures::executor::block_on,
    widget::{
        button, checkbox, column, container, pick_list, row, scrollable, slider, text, text_input,
        toggler, Column, ProgressBar, Toggler,
    },
    Alignment, Background, Color, Element, Length, Renderer, Theme,
};
use native_dialog::FileDialog;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

/// Possible Application Themes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTheme {
    System, // default theme
    Light,
    Dark,
}

impl AppTheme {
    pub const ALL: [AppTheme; 3] = [AppTheme::System, AppTheme::Light, AppTheme::Dark];
}

impl std::fmt::Display for AppTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppTheme::System => write!(f, "System"),
            AppTheme::Light => write!(f, "Light"),
            AppTheme::Dark => write!(f, "Dark"),
        }
    }
}

/// Primary Application Window
pub struct Window {
    // Configuration
    pub config: UIConfig,

    // Tailscale Status
    pub tailscale_status: crate::logic::TailscaleStatus,

    // Main navigation tabs (Home, Exit Node, etc.)
    pub tabs: TabBar<Tab, Message>,
    pub active_tab: String,

    // We keep a separate set of references for dynamic SSH sessions
    // Each session has a unique tab, stored at the bottom of the UI
    // that lists active sessions. This is a separate from the main nav tabs.
    pub ssh_sessions: Vec<SSHSession>,

    // The devices discovered by "tailscale status"
    pub devices: Arc<Mutex<Vec<String>>>,

    // We store the "exit node" device list separately.
    pub exit_nodes: Arc<Mutex<Vec<String>>>,

    // For file sending
    pub send_progress: f32,

    // For storing the typed command in each SSH session's text input
    // Key = device name; Value = current command buffer
    pub ssh_input: HashMap<String, String>,
}

/// Each active SSH session is represented by a tab at the bottom of the UI.
#[derive(Debug, Clone)]
pub struct SSHSession {
    pub device_name: String,
    /// For display output, we could keep a buffer that we fill from the child process's
    /// stout in real-time. For brevity, we'll just store the lines:
    pub output_lines: Vec<String>,
    /// Whether this session is currently active
    pub active: bool,
}

/// The messages that can be sent around in the UI
#[derive(Debug, Clone)]
pub enum Message {
    // Home Tab
    ToggleTailscaleConnectivity(bool),
    ToggleSSHConnectivity(bool),
    ToggleAcceptRoutes(bool),
    TailscaleReceiveFiles,
    OpenFileDialogToSend(String),             // device name
    FilesSelectedToSend(Vec<String>, String), // files, device name
    // Manual "Refresh" device list
    RefreshDevices,

    // Exit Node Tab
    ToggleIsExitNode(bool),
    UseExitNode(String), // connect to a selected exit node
    DisconnectExitNode,
    ToggleConnectToLan(bool),

    // Settings Tab
    FontSizeChanged(u16),
    ThemeSelected(AppTheme),
    ToggleNotifications(bool),
    ToggleSounds(bool),
    ToggleAutoReceive(bool),
    ToggleDisableRecievedFileNotifications(bool),
    CheckForUpdates,

    // SSH Sessions
    OpenSSHSession(String, String), // device, IP
    UpdateSSHInput(String, String), // device, typed command
    SendSSHCommand(String),         // device
    CloseSSHSession(String),        // device

    // Switching Tabs
    SwitchTab(String),

    // For continuing local logic
    Tick,
}

impl Window {
    pub fn new() -> Self {
        // Load config from disk or default
        let config = match UIConfig::load() {
            Ok(cfg) => cfg,
            Err(_) => UIConfig::default(),
        };

        // Ensure Tailscale is installed, check for connectivity
        check_tailscale();
        let tailscale_status = get_tailscale_status();

        // Build main navigation TabBar
        // 1) Home Tab
        let mut home_tab = Tab::new("Home");
        home_tab.active(true);

        // Create the TabBar with home as the default
        let mut tabs = TabBar::new(home_tab, || {
            // We'll fill its content dynamaically in view()
            column![].into()
        });

        // 2) A placeholder "Exit Node" tab
        let exit_node_tab = Tab::new("Exit Nodes");
        tabs.push(exit_node_tab, || column![].into());

        // 3) A placeholder "Settings" tab
        let settings_tab = Tab::new("Settings");
        tabs.push(settings_tab, || column![].into());

        // Start a background thread to periodically refresh the device list
        let devices = Arc::new(Mutex::new(get_tailscale_devices()));
        let devices_clone = Arc::clone(&devices);
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(10));
            let mut dev_lock = devices_clone.lock().unwrap();
            *dev_lock = get_tailscale_devices();
        });

        // Create a separate list for "exit node devices".
        // TODO: Implement getting exit nodes from Tailscale
        let exit_nodes = Arc::new(Mutex::new(vec!["dell-webserver".to_string()]));

        Self {
            config,
            tailscale_status,
            tabs,
            active_tab: "Home".to_string(),
            ssh_sessions: vec![],
            devices,
            exit_nodes,
            send_progress: 0.0,
            ssh_input: HashMap::new(),
        }
    }

    /// The main update function for Iced messages
    pub fn update(&mut self, message: Message) {
        match message {
            // Home Tab
            Message::ToggleTailscaleConnectivity(should_connect) => {
                if should_connect {
                    // Attempt "tailscale up" with config settings
                    tailscale_up(
                        self.config.ssh_enabled,
                        self.config.accept_routes,
                        self.config.is_exit_node,
                        if self.config.use_exit_node {
                            // We might store the IP or hostname of the exit node somewhere
                            Some("some-exit-node.example".to_owned())
                        } else {
                            None
                        },
                        self.config.connect_to_lan,
                    );
                } else {
                    tailscale_down();
                }
                // After toggling, update the status
                self.tailscale_status = get_tailscale_status();
            }
            Message::ToggleSSHConnectivity(enable_ssh) => {
                self.config.ssh_enabled = enable_ssh;

                // Re-run tailscale up if Tailscale is connected
                if self.tailscale_status.connected {
                    tailscale_up(
                        self.config.ssh_enabled,
                        self.config.accept_routes,
                        self.config.is_exit_node,
                        if self.config.use_exit_node {
                            Some("some-exit-node.example".to_owned())
                        } else {
                            None
                        },
                        self.config.connect_to_lan,
                    );
                }
            }
            Message::ToggleAcceptRoutes(accept) => {
                self.config.accept_routes = accept;
                if self.tailscale_status.connected {
                    tailscale_up(
                        self.config.ssh_enabled,
                        self.config.accept_routes,
                        self.config.is_exit_node,
                        if self.config.use_exit_node {
                            Some("some-exit-node.example".to_owned())
                        } else {
                            None
                        },
                        self.config.connect_to_lan,
                    );
                }
            }
            Message::TailscaleReceiveFiles => {
                let output = tailscale_recieve();
                if !output.is_empty() {
                    // Possibly notify user, etc.
                    if self.config.enable_notifications
                        && !self.config.disable_received_file_notifications
                    {
                        notify("Received files", &format!("{output}\n"));
                    }
                }
            }
            Message::OpenFileDialogToSend(device) => {
                // We must open a file dialog and wait for user selection
                // TODO: Implement non-blocking file dialog
                if let Ok(dialog_result) = FileDialog::new().show_open_multiple_file() {
                    let filepaths = dialog_result
                        .iter()
                        .map(|path| path.to_string_lossy().to_string())
                        .collect::<Vec<_>>();
                    // Now send a follow-up message
                    self.update(Message::FilesSelectedToSend(filepaths, device));
                }
            }
            Message::FilesSelectedToSend(paths, device) => {
                let results = tailscale_send(paths.clone(), &device);

                // if success, do something

                // If error, do something else
                for (i, res) in results.iter().enumerate() {
                    if let Some(e) = res {
                        eprintln!("Error sending file {} -> {}: {}", paths[i], &device, e);
                    }
                }
            }
            Message::RefreshDevices => {
                let mut dev_lock = self.devices.lock().unwrap();
                *dev_lock = get_tailscale_devices();
            }

            // Exit Node Tab
            Message::ToggleIsExitNode(advertise) => {
                self.config.is_exit_node = advertise;
                // If we are toggling on, we can't connect to another exit node
                if advertise {
                    self.config.use_exit_node = false;
                }

                // Re-run tailscale up if Tailscale if needed
                if self.tailscale_status.connected {
                    tailscale_up(
                        self.config.ssh_enabled,
                        self.config.accept_routes,
                        self.config.is_exit_node,
                        if self.config.use_exit_node {
                            Some("dell-webserver".to_owned())
                        } else {
                            None
                        },
                        self.config.connect_to_lan,
                    );
                }
            }
            Message::UseExitNode(node_name) => {
                // If we're not an exit node outselves -> connect to that node.
                if !self.config.is_exit_node {
                    self.config.use_exit_node = true;
                    // Re-run tailscale up with exit node name
                    if self.tailscale_status.connected {
                        tailscale_up(
                            self.config.ssh_enabled,
                            self.config.accept_routes,
                            false, // is_exit_node = false
                            Some(node_name),
                            self.config.connect_to_lan,
                        );
                    }
                }
            }
            Message::DisconnectExitNode => {
                // Stop using exit node
                self.config.use_exit_node = false;
                if self.tailscale_status.connected {
                    tailscale_up(
                        self.config.ssh_enabled,
                        self.config.accept_routes,
                        self.config.is_exit_node,
                        None,
                        self.config.connect_to_lan,
                    );
                }
            }
            Message::ToggleConnectToLan(allow) => {
                self.config.connect_to_lan = allow;
                if self.tailscale_status.connected {
                    tailscale_up(
                        self.config.ssh_enabled,
                        self.config.accept_routes,
                        self.config.is_exit_node,
                        if self.config.use_exit_node {
                            Some("dell-webserver".to_owned())
                        } else {
                            None
                        },
                        self.config.connect_to_lan,
                    );
                }
            }

            // Settings Tab
            Message::FontSizeChanged(size) => {
                let clamped = size.max(10).min(24);
                self.config.font_size = clamped;
            }
            Message::ThemeSelected(app_theme) => {
                self.config.theme = match app_theme {
                    AppTheme::System => "system".to_string(),
                    AppTheme::Light => "light".to_string(),
                    AppTheme::Dark => "dark".to_string(),
                };
            }
            Message::ToggleNotifications(enabled) => {
                self.config.enable_notifications = enabled;
            }
            Message::ToggleSounds(enabled) => {
                self.config.enable_sounds = enabled;
            }
            Message::ToggleAutoReceive(auto) => {
                // If user disables auto-receive, the cannot disable notifications for received files
                // so if "disable_received_file_notifications" is true, force it to be false.
                self.config.auto_receive_files = auto;
                if !auto {
                    // Show a note or enfor logic
                    if self.config.disable_received_file_notifications {
                        // Re-enable them automatically
                        self.config.disable_received_file_notifications = false;
                        notify("Auto-Recieve Disabled", "You have disabled auto-receive. Disabling notification for received files is no longer possible, so it has been re-enabled.");
                    }
                }
            }
            Message::ToggleDisableRecievedFileNotifications(disable) => {
                // If auto-receive is disabled, user can't set this to true.
                if self.config.auto_receive_files {
                    self.config.disable_received_file_notifications = disable;
                } else {
                    // We do not allow disabling file notifications if auto-receive is also disabled
                    notify(
                        "Action Restricted",
                        "You cannot disable file notifications while auto-receive is disabled.",
                    );
                }
            }
            Message::CheckForUpdates => {
                check_for_updates();
            }

            // SSH Sessions
            Message::OpenSSHSession(device, ip) => {
                // Create a new tab at the bottom
                // Start the session
                run_ssh_session(device.clone(), ip.clone());
                // Add a new session to our vec
                let sesh = SSHSession {
                    device_name: device.clone(),
                    output_lines: vec![format!("Connected to {device} at {ip}")],
                    active: true,
                };
                self.ssh_sessions.push(sesh);

                // Initialize input buffer
                self.ssh_input.insert(device.clone(), "".to_string());
            }
            Message::UpdateSSHInput(device, typed) => {
                self.ssh_input.insert(device, typed);
            }
            Message::SendSSHCommand(device) => {
                if let Some(cmd) = self.ssh_input.get(&device).cloned() {
                    send_ssh_command(device.clone(), cmd.clone());

                    // Potentially append to session's output buffer
                    if let Some(sesh) = self
                        .ssh_sessions
                        .iter_mut()
                        .find(|s| s.device_name == device)
                    {
                        sesh.output_lines.push(format!("> {cmd}"));
                    }

                    // Clear input
                    self.ssh_input.insert(device, "".to_string());
                }
            }
            Message::CloseSSHSession(device) => {
                // Terminate
                terminate_ssh_session(&device);
                // Remove from active sessions
                self.ssh_sessions.retain(|s| s.device_name != device);
                // Remove from input buffer
                self.ssh_input.remove(&device);
            }
            Message::SwitchTab(tab_name) => {
                self.active_tab = tab_name.clone();

                // Update the active tab in the TabBar
                for (tab, _) in &mut self.tabs.tabs {
                    tab.active(tab.title() == tab_name);
                }
            }
            Message::Tick => {
                // If we want to do any periodic updates, do them here
            }
        }

        // Attempt to save config each time we make a change
        let _ = self.config.save();
    }

    /// Build the main UI
    pub fn view(&self) -> Element<Message, Theme> {
        // Let's build a top-level Column to hold the main nav and content
        let main_column = column![
            // Title
            container(row![text("GUI Scale").size(18).align_x(Alignment::Center),].padding(5))
                .width(Length::Fill),
            // Render the main navigation tabs
            self.main_navigation(),
            // Render the "Active SSH Sessions" TabBar at the bottom if there are active sessions
            self.active_sessions_view(),
        ]
        .spacing(20)
        .padding(10);

        container(main_column)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    /// Builds the main navigation tabs content
    fn main_navigation(&self) -> Element<Message, Theme> {
        // For each tab in self.tabs, we see if it is active, then draw the content
        let tab_row = row![
            button("Home").on_press(Message::SwitchTab("Home".to_string())),
            button("Exit Nodes").on_press(Message::SwitchTab("Exit Nodes".to_string())),
            button("Settings").on_press(Message::SwitchTab("Settings".to_string())),
        ]
        .spacing(10)
        .padding(10)
        .width(Length::Fill);

        // Add the tabs to the view
        let content = match self.active_tab.as_str() {
            "Home" => self.view_home_tab(),
            "Exit Node" => self.view_exit_node_tab(),
            "Settings" => self.view_settings_tab(),
            _ => column![text("Unknown Tab")].into(),
        };

        column![tab_row, content].spacing(20).into()
    }

    /// The "Home" tab content
    fn view_home_tab(&self) -> Element<Message, Theme> {
        // Show Tailscale status, toggles, device list, etc.
        let connected = self.tailscale_status.connected;
        let ip_display = self
            .tailscale_status
            .tailscale_ip
            .clone()
            .unwrap_or_else(|| "Not connected".into());

        let tailscale_toggle = toggler(connected)
            .label("Tailscale")
            .on_toggle(|checked| Message::ToggleTailscaleConnectivity(checked));

        let ssh_toggle = toggler(self.config.ssh_enabled)
            .label("SSH")
            .on_toggle(|enabled| Message::ToggleSSHConnectivity(enabled));

        let accept_routes_toggle = toggler(self.config.accept_routes)
            .label("Accept Routes")
            .on_toggle(|accept| Message::ToggleAcceptRoutes(accept));

        // Receive files button
        let receive_btn = if !self.config.auto_receive_files {
            button("Receive Files").on_press(Message::TailscaleReceiveFiles)
        } else {
            button("Recieve Files") // disabled
        };

        // Build the device list
        let devs = self.devices.lock().unwrap();
        let mut dev_list_col = column![];
        for dev in devs.iter() {
            let send_file_btn =
                button("🔼") // replace this with a real send icon
                    .on_press(Message::OpenFileDialogToSend(dev.clone()))
                    .padding(5);

            let connect_button = button("💻")
                .on_press(Message::OpenSSHSession(
                    dev.clone(),
                    format!("{}.ts.net", dev),
                ))
                .padding(5);

            let row_dev = row![
                text(dev.clone()).size(self.config.font_size as u16),
                send_file_btn,
                connect_button
            ]
            .spacing(10);

            dev_list_col = dev_list_col.push(row_dev);
        }

        let home_col = column![
            row![text(format!(
                "Status: {} IPv4: {}",
                if connected {
                    "Connected"
                } else {
                    "Disconnected"
                },
                ip_display
            ))
            .size(self.config.font_size),]
            .spacing(10),
            row![
                tailscale_toggle,
                ssh_toggle,
                accept_routes_toggle,
                receive_btn,
            ]
            .spacing(40),
            scrollable(dev_list_col).height(Length::FillPortion(1)),
        ]
        .spacing(20);

        container(home_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// The "Exit Node" tab content
    fn view_exit_node_tab(&self) -> Element<Message, Theme> {
        // Toggler to advertise self as exit node
        let is_exit_node_toggle = toggler(self.config.is_exit_node)
            .label("Host is Exit Node")
            .on_toggle(|val| Message::ToggleIsExitNode(val));

        // If we are an exit node, we can't connect to another
        let mut connect_to_lan_toggle = toggler(self.config.connect_to_lan).label("Connect to LAN");
        if self.config.is_exit_node {
            connect_to_lan_toggle = toggler(self.config.connect_to_lan)
                .label("Connect to LAN")
                .on_toggle(|val| Message::ToggleConnectToLan(val));
        }

        // Build a scrollable list of exit nodes
        let nodes = self.exit_nodes.lock().unwrap();
        let mut node_list_col = column![];
        for node in nodes.iter() {
            let label = node.clone();
            // If we are an exit node, we can't connect to another
            let (btn_label, btn_msg, disabled) = if self.config.is_exit_node {
                (format!("💻❌"), Message::Tick, true) // do nothing
            } else if self.config.use_exit_node && label == "dell-webserver" {
                (format!("Disconnect"), Message::DisconnectExitNode, false)
            } else if self.config.use_exit_node {
                (format!("💻❌"), Message::Tick, true)
            } else {
                (format!("💻"), Message::UseExitNode(label.clone()), false)
            };

            let connect_btn = if !disabled {
                button(text(btn_label)).on_press(btn_msg).padding(5)
            } else {
                button(text(btn_label)).padding(5)
            };

            node_list_col =
                node_list_col.push(row![text(label).size(self.config.font_size), connect_btn]);
        }

        let exit_col = column![
            is_exit_node_toggle,
            connect_to_lan_toggle,
            scrollable(node_list_col).height(Length::FillPortion(1)),
        ]
        .spacing(20)
        .padding(10);

        container(exit_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// The "Settings" tab content
    fn view_settings_tab(&self) -> Element<Message, Theme> {
        // Font size slider
        let font_slider = slider(10..=24, self.config.font_size, Message::FontSizeChanged);

        // Theme picker
        let current_theme = match self.config.theme.as_str() {
            "system" => AppTheme::System,
            "light" => AppTheme::Light,
            "dark" => AppTheme::Dark,
            _ => AppTheme::System,
        };

        let theme_picker = pick_list(
            &AppTheme::ALL[..],
            Some(current_theme),
            Message::ThemeSelected,
        );

        let notifications_toggle = toggler(self.config.enable_notifications)
            .label("Enable Notifications")
            .on_toggle(|val| Message::ToggleNotifications(val));

        let sounds_toggle = toggler(self.config.enable_sounds)
            .label("Enable Sounds")
            .on_toggle(|val| Message::ToggleSounds(val));

        let auto_receive_toggle =
            toggler::<Message, Theme, Renderer>(self.config.auto_receive_files)
                .label("Auto-Receive Files")
                .on_toggle(|val| Message::ToggleAutoReceive(val));

        // If auto-receive is disabled, user cannot disable notifications for received files
        let disable_file_notif =
            toggler::<Message, Theme, Renderer>(self.config.disable_received_file_notifications)
                .label("Disable Received File Notifications")
                .on_toggle(|val| Message::ToggleDisableRecievedFileNotifications(val));

        let update_check_btn = button("Check for Updates").on_press(Message::CheckForUpdates);

        let settings_col = column![
            row![
                text("Font Size: ")
                    .size(self.config.font_size)
                    .width(Length::Shrink),
                font_slider,
            ]
            .spacing(10),
            row![
                text("Theme: ")
                    .size(self.config.font_size)
                    .width(Length::Shrink),
                theme_picker,
            ]
            .spacing(10),
            row![notifications_toggle, sounds_toggle].spacing(10),
            row![auto_receive_toggle, disable_file_notif].spacing(10),
            row![update_check_btn,].spacing(20)
        ]
        .spacing(15)
        .padding(10);

        container(settings_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// The "Active SSH Sessions" tab content
    fn active_sessions_view(&self) -> Element<Message, Theme> {
        if self.ssh_sessions.is_empty() {
            // return empty element
            return row![].into();
        }

        // Now we've ensured we have active sessions to display the TabBar for
        let mut sessions_tabs = row![].spacing(10);
        for session in &self.ssh_sessions {
            // Each tab has the device name + close button
            sessions_tabs = sessions_tabs.push(
                container(
                    row![
                        text(&session.device_name).size(self.config.font_size),
                        button("X").on_press(Message::CloseSSHSession(session.device_name.clone()))
                    ]
                    .spacing(5),
                )
                .padding(5),
            );
        }

        // The content for the active session we might be "focusing"
        // TODO: Implement a way to focus on a specific session
        let current_sesh = self.ssh_sessions.last().unwrap();
        let buffer = self
            .ssh_input
            .get(&current_sesh.device_name)
            .cloned()
            .unwrap_or_default();
        let output_list = current_sesh
            .output_lines
            .iter()
            .fold(column![], |col, line| col.push(text(line)));

        let ssh_content = column![
            scrollable(output_list).height(Length::Fixed(200.0)),
            row![
                text_input("Enter command...", &buffer)
                    .on_input(|msg| Message::UpdateSSHInput(current_sesh.device_name.clone(), msg))
                    .on_submit(Message::SendSSHCommand(current_sesh.device_name.clone()))
                    .width(Length::Fill),
                button("Send")
                    .on_press(Message::SendSSHCommand(current_sesh.device_name.clone()))
                    .padding(5),
            ]
            .spacing(10)
        ]
        .spacing(10)
        .padding(5);

        column![sessions_tabs, container(ssh_content).padding(5)]
            .spacing(10)
            .into()
    }
}

impl Default for Window {
    fn default() -> Self {
        Window::new()
    }
}
