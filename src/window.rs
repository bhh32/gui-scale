use crate::{
    config::{load_config, update_config, UIConfig},
    logic::{
        check_for_updates, check_tailscale, get_tailscale_devices, get_tailscale_status, notify,
        play_sound, run_ssh_session, send_ssh_command, tailscale_down, tailscale_recieve,
        tailscale_send, tailscale_up, terminate_ssh_session,
    },
    widgets::tab::{tab_bar::TabBar, IsTab, Tab},
};
use cosmic::{cosmic_config::Config, iced::{Alignment::{self, Center, End}, Length}, iced_widget::text_input, widget::{image::Handle, scrollable}, Element, Renderer, Theme};
use cosmic::iced::Length::{Fill, FillPortion, Fixed, Shrink};
use cosmic::app::{Core, Settings, Task};
use cosmic::widget::{
        button, container, dropdown, column, Column, row, slider, text, toggler, Image,
};
use native_dialog::FileDialog;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
    process::Command,
};

const ID: &str = "com.bhh32.gui-scale";
const CONFIG_VERS: u64 = 1;

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

impl AsRef<str> for AppTheme {
    fn as_ref(&self) -> &str {
        match self {
            AppTheme::System => "System",
            AppTheme::Light => "Light",
            AppTheme::Dark => "Dark",
        }
    }
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
    // Required by LIBCOSMIC
    pub core: cosmic::app::Core,

    // Configuration
    pub config: Config,
    pub ui_config: UIConfig,

    // Tailscale Status, these are initialized and updated by the Tailscale CLI
    pub tailscale_status: crate::logic::TailscaleStatus,

    // Main navigation tabs (Home, Exit Node, etc.)
    pub tabs: TabBar<Tab, Message>,
    pub active_tab: String,

    // We keep a separate set of references for dynamic SSH sessions
    // Each session has a unique tab, stored at the bottom of the UI
    // that lists active sessions. This is a separate from the main nav tabs.
    pub ssh_sessions: Vec<SSHSession>,

    // The devices discovered by "tailscale status"
    pub devices: Arc<Mutex<Vec<(String, String, String)>>>,

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

impl cosmic::Application for Window {
    const APP_ID: &'static str = "com.bhh32.gui-scale";
    type Message = Message;
    type Executor = cosmic::executor::Default;
    type Flags = ();

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        // Ensure Tailscale is installed, check for connectivity
        check_tailscale();

        // Get the current status from the Tailscale CLI
        let tailscale_status = get_tailscale_status();

        // Build main navigation TabBar
        // 1) Home Tab
        let mut home_tab = Tab::new("Home");
        home_tab.active(true);

        // Create the TabBar with home as the default
        let mut tabs = TabBar::new(home_tab, || {
            // We'll fill its content dynamaically in view()
            Column::new().into()
        });

        // 2) A placeholder "Exit Node" tab
        let exit_node_tab = Tab::new("Exit Nodes");
        tabs.push(exit_node_tab, || Column::new().into());

        // 3) A placeholder "Settings" tab
        let settings_tab = Tab::new("Settings");
        tabs.push(settings_tab, || Column::new().into());

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
        let mut window = Self {
            core,
            config: Config::new(ID, CONFIG_VERS).unwrap(),
            ui_config: UIConfig::default(),
            tailscale_status,
            tabs,
            active_tab: "Home".to_string(),
            ssh_sessions: vec![],
            devices,
            exit_nodes,
            send_progress: 0.0,
            ssh_input: HashMap::new(),
        };

        // Update the config with the current status
        // This is done just in case the CLI was ran without the GUI
        update_config(&window.config, "ipv4", match window.tailscale_status.tailscale_ip.clone() {
            Some(ip) => ip,
            None => "".to_string(),
        });
        update_config(&window.config, "connected", window.tailscale_status.connected.clone());
        update_config(&window.config, "ssh_enabled", window.tailscale_status.ssh_enabled.clone());
        update_config(&window.config, "routes", window.tailscale_status.routes.clone());
        update_config(&window.config, "allow_lan", window.tailscale_status.allow_lan.clone());
        update_config(&window.config, "is_exit_node", window.tailscale_status.is_exit_node.clone());

        // Update the UI config with previous closed settings
        window.ui_config = UIConfig::load().unwrap_or_default();

        (window, Task::none())
    }

    /// The main update function for Iced messages
    fn update(&mut self, message: Message) -> Task<Self::Message> {
        match message {
            // Home Tab
            Message::ToggleTailscaleConnectivity(should_connect) => {
                if should_connect {
                    // Attempt "tailscale up" with config settings
                    tailscale_up(
                        self.tailscale_status.ssh_enabled,
                        self.tailscale_status.routes,
                        self.tailscale_status.is_exit_node,
                        None,
                        self.tailscale_status.allow_lan,
                    );
                } else {
                    tailscale_down();
                }
                // After toggling, update the status
                self.tailscale_status = get_tailscale_status();
            }
            Message::ToggleSSHConnectivity(enable_ssh) => {
                self.tailscale_status.ssh_enabled = enable_ssh;

                // Re-run tailscale up if Tailscale is connected
                if self.tailscale_status.connected {
                    tailscale_up(
                        self.tailscale_status.ssh_enabled,
                        self.tailscale_status.routes,
                        self.tailscale_status.is_exit_node,
                        None,
                        self.tailscale_status.allow_lan,
                    );
                }
            }
            Message::ToggleAcceptRoutes(accept) => {
                self.tailscale_status.routes = accept;
                if self.tailscale_status.connected {
                    tailscale_up(
                        self.tailscale_status.ssh_enabled,
                        self.tailscale_status.routes,
                        self.tailscale_status.is_exit_node,
                        None,
                        self.tailscale_status.allow_lan,
                    );
                }
            }
            Message::TailscaleReceiveFiles => {
                let output = tailscale_recieve();
                if !output.is_empty() {
                    // Possibly notify user, etc.
                    if self.ui_config.enable_notifications
                        && !self.ui_config.disable_received_file_notifications
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
                self.tailscale_status.is_exit_node = advertise;
                // If we are toggling on, we can't connect to another exit node
                if advertise {
                    //self.tailscale_status.use_exit_node = false;
                }

                // Re-run tailscale up if Tailscale if needed
                if self.tailscale_status.connected {
                    tailscale_up(
                        self.tailscale_status.ssh_enabled,
                        self.tailscale_status.routes,
                        self.tailscale_status.is_exit_node,
                        None,
                        self.tailscale_status.allow_lan,
                    );
                }
            }
            Message::UseExitNode(node_name) => {
                // If we're not an exit node outselves -> connect to that node.
                if !self.tailscale_status.is_exit_node {
                    //self.tailscale_status.use_exit_node = true;
                    // Re-run tailscale up with exit node name
                    if self.tailscale_status.connected {
                        tailscale_up(
                            self.tailscale_status.ssh_enabled,
                            self.tailscale_status.routes,
                            false, // is_exit_node = false
                            Some(node_name),
                            self.tailscale_status.allow_lan,
                        );
                    }
                }
            }
            Message::DisconnectExitNode => {
                // Stop using exit node
                //self.config.use_exit_node = false;
                if self.tailscale_status.connected {
                    tailscale_up(
                        self.tailscale_status.ssh_enabled,
                        self.tailscale_status.routes,
                        self.tailscale_status.is_exit_node,
                        None,
                        self.tailscale_status.allow_lan,
                    );
                }
            }
            Message::ToggleConnectToLan(allow) => {
                self.tailscale_status.allow_lan = allow;
                if self.tailscale_status.connected {
                    tailscale_up(
                        self.tailscale_status.ssh_enabled,
                        self.tailscale_status.routes,
                        self.tailscale_status.is_exit_node,
                        None,
                        self.tailscale_status.allow_lan,
                    );
                }
            }

            // Settings Tab
            Message::FontSizeChanged(size) => {
                let clamped = size.max(10).min(24);
                self.ui_config.font_size = clamped;
            }
            Message::ThemeSelected(app_theme) => {
                self.ui_config.theme = match app_theme {
                    AppTheme::System => "system".to_string(),
                    AppTheme::Light => "light".to_string(),
                    AppTheme::Dark => "dark".to_string(),
                };

                match self.ui_config.theme.as_str() {
                    "system" => self.system_theme_update(&["system", "light", "dark"], cosmic::Theme::cosmic(&cosmic::Theme::default())),
                    "light" => self.system_theme_update(&["system", "light", "dark"], cosmic::Theme::cosmic(&cosmic::Theme::light())),
                    "dark" => self.system_theme_update(&["system", "light", "dark"], cosmic::Theme::cosmic(&cosmic::Theme::dark())),
                    _ => self.system_theme_update(&["system", "light", "dark"], cosmic::Theme::cosmic(&cosmic::Theme::default())),
                };
                
            }
            Message::ToggleNotifications(enabled) => {
                self.ui_config.enable_notifications = enabled;
            }
            Message::ToggleSounds(enabled) => {
                self.ui_config.enable_sounds = enabled;
            }
            Message::ToggleAutoReceive(auto) => {
                // If user disables auto-receive, the cannot disable notifications for received files
                // so if "disable_received_file_notifications" is true, force it to be false.
                self.ui_config.auto_receive_files = auto;
                if !auto {
                    // Show a note or enfor logic
                    if self.ui_config.disable_received_file_notifications {
                        // Re-enable them automatically
                        self.ui_config.disable_received_file_notifications = false;
                        notify("Auto-Recieve Disabled", "You have disabled auto-receive. Disabling notification for received files is no longer possible, so it has been re-enabled.");
                    }
                }
            }
            Message::ToggleDisableRecievedFileNotifications(disable) => {
                // If auto-receive is disabled, user can't set this to true.
                if self.ui_config.auto_receive_files {
                    self.ui_config.disable_received_file_notifications = disable;
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
            Message::CloseSSHSession(device_name) => {
                // Remove the first SSH session (or the specified one if found)
                if let Some(index) = self.ssh_sessions.iter().position(|s| s.device_name == device_name) {
                    self.ssh_sessions.remove(index);
                }
                // Remove the corresponding input if it exists
                self.ssh_input.remove(&device_name);
            }
            Message::SwitchTab(tab_name) => {
                self.active_tab = tab_name;
            }
            Message::Tick => {
                // If we want to do any periodic updates, do them here
            }
        }

        // Attempt to save configs each time we make a change
        update_config(&self.config, "connected", &self.tailscale_status.connected.clone());
        update_config(&self.config, "ssh_enabled", &self.tailscale_status.ssh_enabled.clone());
        update_config(&&self.config, "routes", &self.tailscale_status.routes.clone());
        update_config(&&self.config, "allow_lan", &self.tailscale_status.allow_lan.clone());
        update_config(&&self.config, "is_exit_node", &self.tailscale_status.is_exit_node.clone());
        let _ = self.ui_config.save();
        Task::none()
    }

    /// Build the main UI
    fn view(&self) -> Element<Message> {
        // Main layout with navigation and content area
        let content = match self.active_tab.as_str() {
            "Home" => self.view_home_tab(),
            "Exit Nodes" => self.view_exit_node_tab(),
            "Settings" => self.view_settings_tab(),
            "Active Sessions" => self.active_sessions_view(),
            _ => Column::new()
                .push(
                    text("Select a tab to view content")
                        .size(self.ui_config.font_size)
                )
                .into(),
        };

        // Main layout with navigation, content, and SSH sessions
        Column::new()
            .push(self.main_navigation())
            .push(
                container(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(10)
            )
            .push(
                // Always show SSH sessions at the bottom, but make it collapsible
                if !self.ssh_sessions.is_empty() {
                    Column::new()
                        .push(
                            row::row()
                                .push(text("SSH Sessions").size(self.ui_config.font_size))
                                .push(
                                    button::standard("Close")
                                        .on_press(Message::CloseSSHSession(
                                            self.ssh_sessions.first().unwrap().device_name.clone()
                                        ))
                                )
                                .spacing(10)
                        )
                        .push(self.active_sessions_view())
                        .spacing(10)
                        .into()
                } else {
                    // If no SSH sessions, return an empty element
                    Column::new()
                }
            )
            .spacing(10)
            .into()
    }
}

impl Window {
    /// Builds the main navigation tabs content
    fn main_navigation(&self) -> Element<Message> {
        let main_tabs = vec![
            ("Home", Message::SwitchTab("Home".to_string())),
            ("Exit Nodes", Message::SwitchTab("Exit Nodes".to_string())),
            ("Settings", Message::SwitchTab("Settings".to_string())),
        ];

        let mut nav_row = row::row().spacing(10);
        for (tab_name, message) in main_tabs {
            let btn = if self.active_tab == tab_name {
                button::standard(tab_name).on_press(message)
            } else {
                button::standard(tab_name).on_press(message)
            };
            nav_row = nav_row.push(btn);
        }

        nav_row.into()
    }

    /// The "Home" tab content
    fn view_home_tab(&self) -> Element<Message> {
        let connected = self.tailscale_status.connected;
        let ip_display = self
            .tailscale_status
            .tailscale_ip
            .clone()
            .unwrap_or_else(|| "Not connected".into());

        let tailscale_toggle = toggler(connected)
            .label("Tailscale")
            .on_toggle(|checked| Message::ToggleTailscaleConnectivity(checked));

        let ssh_toggle = toggler(self.tailscale_status.ssh_enabled)
            .label("SSH")
            .on_toggle(|enabled| Message::ToggleSSHConnectivity(enabled));

        let accept_routes_toggle = toggler(self.tailscale_status.routes)
            .label("Accept Routes")
            .on_toggle(|accept| Message::ToggleAcceptRoutes(accept));

        // Receive files button
        let receive_btn = if !self.ui_config.auto_receive_files {
            button::standard("Receive Files").on_press(Message::TailscaleReceiveFiles)
        } else {
            button::standard("Receive Files")
        };

        // Devices list with file send buttons
        let devices_list = {
            let mut col: Column<Message> = Column::new().spacing(5);
            
            // Table header
            let header = row::row()
                .push(
                    text("Device")
                        .size(self.ui_config.font_size)
                        .width(Length::Fill)
                )
                .push(
                    text("Username")
                        .size(self.ui_config.font_size)
                        .width(Length::Fill)
                )
                .push(
                    text("Actions")
                        .size(self.ui_config.font_size)
                        .width(Length::Shrink)
                )
                .spacing(10)
                .padding(5);
            col = col.push(header);

            let mut devices_col: Column<Message> = Column::new().spacing(5);
            let devices = self.devices.lock().unwrap();
            for (dev, username, ip) in devices.iter() {
                let row_dev = row::row()
                    .push(
                        text(dev.clone())
                            .size(self.ui_config.font_size)
                            .width(Length::Fill)
                    )
                    .push(
                        text(username.clone())
                            .size(self.ui_config.font_size)
                            .width(Length::Fill)
                    )
                    .push(
                        row::row()
                            .push(
                                button::standard("Send Files")
                                    .on_press(Message::OpenFileDialogToSend(dev.clone()))
                            )
                            .push(
                                button::standard("SSH")
                                    .on_press(Message::OpenSSHSession(dev.clone(), ip.clone()))
                            )
                            .spacing(5)
                            .width(Length::Shrink)
                    )
                    .spacing(10)
                    .padding(5)
                    .width(Length::Fill);
                devices_col = devices_col.push(row_dev);
            }

            // Make the devices list scrollable
            scrollable(devices_col)
                .height(Length::FillPortion(1))
        };

        Column::new()
            .push(
                row::row()
                    .push(tailscale_toggle)
                    .push(ssh_toggle)
                    .push(accept_routes_toggle)
                    .push(receive_btn)
                    .spacing(10)
            )
            .push(
                text(format!("Tailscale IP: {}", ip_display)).size(self.ui_config.font_size)
            )
            .push(
                text("Devices").size(self.ui_config.font_size + 2)
            )
            .push(devices_list)
            .spacing(20)
            .into()
    }

    /// The "Exit Node" tab content
    fn view_exit_node_tab(&self) -> Element<Message> {
        // Toggler to advertise self as exit node
        let is_exit_node_toggle = toggler(self.tailscale_status.is_exit_node)
            .label("Host is Exit Node")
            .on_toggle(|val| Message::ToggleIsExitNode(val));

        // If we are an exit node, we can't connect to another
        let mut connect_to_lan_toggle = toggler(self.tailscale_status.allow_lan).label("Connect to LAN");
        if self.tailscale_status.is_exit_node {
            connect_to_lan_toggle = toggler(self.tailscale_status.allow_lan)
                .label("Connect to LAN")
                .on_toggle(|val| Message::ToggleConnectToLan(val));
        }

        // Build a scrollable list of exit nodes
        let nodes = self.exit_nodes.lock().unwrap();
        let mut node_list_col = Column::new();
        for node in nodes.iter() {
            let label = node.clone();
            // If we are an exit node, we can't connect to another
            let (btn_label, btn_msg, disabled) = if self.tailscale_status.is_exit_node {
                (format!("💻❌"), Message::Tick, true) // do nothing
            } else if self.tailscale_status.use_exit_node && label == "dell-webserver" {
                (format!("Disconnect"), Message::DisconnectExitNode, false)
            } else if self.tailscale_status.use_exit_node {
                (format!("💻❌"), Message::Tick, true)
            } else {
                (format!("💻"), Message::UseExitNode(label.clone()), false)
            };

            let connect_btn = if !disabled {
                button::standard(btn_label).on_press(btn_msg).padding(5)
            } else {
                button::standard(btn_label).padding(5)
            };

            node_list_col = node_list_col
                .push(
                    row::row()
                        .push(text(label).size(self.ui_config.font_size))
                        .push(connect_btn)
                );
        }

        let exit_col: Element<Message> = Column::new()
        .push(
            row::row()
                .push(is_exit_node_toggle)
                .push(connect_to_lan_toggle)
                .spacing(10)
        )
        .push(text("Exit Nodes").size(self.ui_config.font_size + 2))
        .push(scrollable(node_list_col).height(Length::FillPortion(1)))
        .spacing(20)
        .into();

        container(exit_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// The "Settings" tab content
    fn view_settings_tab(&self) -> Element<Message> {
        // Font size slider
        let font_slider = slider(10..=24, self.ui_config.font_size, Message::FontSizeChanged);

        // Theme picker
        let current_theme = match self.ui_config.theme.as_str() {
            "system" => AppTheme::System,
            "light" => AppTheme::Light,
            "dark" => AppTheme::Dark,
            _ => AppTheme::System,
        };

        let theme_picker = dropdown(
            &AppTheme::ALL,
            AppTheme::ALL.iter().position(|&theme| theme == current_theme),
            |index| Message::ThemeSelected(AppTheme::ALL[index]),
        );

        let notifications_toggle = toggler(self.ui_config.enable_notifications)
            .label("Enable Notifications")
            .on_toggle(|val| Message::ToggleNotifications(val));

        let sounds_toggle = toggler(self.ui_config.enable_sounds)
            .label("Enable Sounds")
            .on_toggle(|val| Message::ToggleSounds(val));

        let auto_receive_toggle =
            toggler::<Message, Theme, Renderer>(self.ui_config.auto_receive_files)
                .label("Auto-Receive Files")
                .on_toggle(|val| Message::ToggleAutoReceive(val));

        // If auto-receive is disabled, user cannot disable notifications for received files
        let disable_file_notif =
            toggler::<Message, Theme, Renderer>(self.ui_config.disable_received_file_notifications)
                .label("Disable Received File Notifications")
                .on_toggle(|val| Message::ToggleDisableRecievedFileNotifications(val));

        let update_check_btn = button::standard("Check for Updates").on_press(Message::CheckForUpdates);

        let settings_col: Element<Message> = Column::new()
        .push(
            row::row()
                .push(
                    text("Font Size: ")
                        .size(self.ui_config.font_size)
                )
                .push(font_slider)
                .spacing(10)
        )
        .push(
            row::row()
                .push(text("Theme: ").size(self.ui_config.font_size))
                .push(theme_picker)
                .spacing(10)
        )
        .push(
            row::row()
                .push(notifications_toggle)
                .push(sounds_toggle)
                .spacing(10)
        )
        .push(
            row::row()
                .push(auto_receive_toggle)
                .push(disable_file_notif)
                .spacing(10)
        )
        .push(
            button::standard("Check for Updates")
                .on_press(Message::CheckForUpdates)
        )
        .spacing(20)
        .into();

        container(settings_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// The "Active SSH Sessions" tab content
    fn active_sessions_view(&self) -> Element<Message> {
        if self.ssh_sessions.is_empty() {
            return Column::new()
                .push(
                    text("No Active SSH Sessions")
                        .size(self.ui_config.font_size)
                )
                .into();
        }

        let mut sessions_content = Column::new().spacing(10);

        // Tabs for each SSH session
        let mut sessions_tabs = row::row().spacing(10);
        for session in &self.ssh_sessions {
            sessions_tabs = sessions_tabs
                .push(
                    container(
                        row::row()
                            .push(text(&session.device_name).size(self.ui_config.font_size))
                            .push(
                                button::standard("X")
                                    .on_press(Message::CloseSSHSession(session.device_name.clone()))
                            )
                            .spacing(5)
                    )
                    .padding(5)
                );
        }

        // Content for the active session
        let active_session_content = if let Some(session) = self.ssh_sessions.iter().find(|s| s.active) {
            let mut output_column = Column::new().spacing(5);
            
            // Add Tailscale CLI output
            let tailscale_output = run_command("tailscale status").unwrap_or_else(|_| "Failed to get Tailscale status".to_string());
            output_column = output_column
                .push(text("Tailscale Status:").size(self.ui_config.font_size + 2))
                .push(text(tailscale_output).size(self.ui_config.font_size));

            // Existing session output
            for line in &session.output_lines {
                output_column = output_column.push(text(line).size(self.ui_config.font_size));
            }

            // SSH command input
            let input = text_input(
                "Enter SSH command",
                self.ssh_input.get(&session.device_name).unwrap_or(&String::new())
            )
            .on_input(|text| Message::UpdateSSHInput(session.device_name.clone(), text))
            .on_submit(Message::SendSSHCommand(session.device_name.clone()));

            output_column
                .push(input)
                .push(
                    button::standard("Send Command")
                        .on_press(Message::SendSSHCommand(session.device_name.clone()))
                )
        } else {
            Column::new()
                .push(
                    text("Select an SSH session to view details")
                        .size(self.ui_config.font_size)
                )
        };

        sessions_content
            .push(sessions_tabs)
            .push(active_session_content)
            .into()
    }
}

fn run_command(cmd: &str) -> Result<String, String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| format!("Failed to run command: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8(output.stdout).unwrap())
    } else {
        Err(String::from_utf8(output.stderr).unwrap())
    }
}
