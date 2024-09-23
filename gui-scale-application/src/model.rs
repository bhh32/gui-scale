use crate::messages::TailscaleActions;
use crate::logic::{
    get_tailscale_con_status, get_tailscale_ip, get_tailscale_routes_status, 
    get_tailscale_ssh_status, set_routes, set_ssh, tailscale_int_up,
    get_available_devices,
};
use iced::widget::{container, horizontal_space, vertical_space, text, Checkbox, Text};
use iced::{executor, Command};
use iced::{
    widget::{
        button, column, row,
        Theme
    },
    theme::Container,
    Element,
    alignment::{
        Horizontal, 
        Vertical, 
        Alignment,
    },
    Application
};

/// The state of the application.
pub struct GuiScale {
    up_down: bool,
    use_ssh: bool,
    allow_routes: bool,
}

/** 
 * The default state of the application.
 * 
 * This gets set with the current state of Tailscale when
 * the application starts.
 */
impl Default for GuiScale {
    fn default() -> Self {
        Self {
            up_down: get_tailscale_con_status(),
            use_ssh: get_tailscale_ssh_status(),
            allow_routes: get_tailscale_routes_status(),
        }
    }
}

/// GUI implementation that implements the Application trait on the app's state.
impl Application for GuiScale {
    type Executor = executor::Default;
    type Message = TailscaleActions;
    type Theme = Theme;
    type Flags = ();

    /// Creates the application.
    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        (Self::default(), Command::none())
    }

    /// Sets the application's title.
    fn title(&self) -> String {
        String::from("GUI-Scale")
    }

    /// Runs the logic based on the messages <TailscaleActions> passed to it.
    fn update(&mut self, message: TailscaleActions) -> Command<Self::Message> {
        match message {
            TailscaleActions::Up => {
                self.up_down = true;
                tailscale_int_up(self.up_down, self.use_ssh, self.allow_routes)
            },
            TailscaleActions::Down => tailscale_int_up(false, false, false),
            TailscaleActions::UseSSH(ssh) => {
                self.use_ssh = ssh;
                set_ssh(ssh)
            },
            TailscaleActions::AcceptRoutes(accept_routes) => {
                self.allow_routes = accept_routes;
                set_routes(accept_routes)
            },
        };

        Command::none()
    }

    /// Sets up and runs the frontend part of the application (GUI).
    fn view(&self) -> Element<TailscaleActions> {
        // Bool to hold if Tailscale is currently connected or not.
        let connection_status = get_tailscale_con_status();

        let statuses = container(
            column![
                text("Status Messages:"),
                row![
                    // Text widget to hold the Tailscale IPv4 address of the running computer.
                    Text::new(format!("Tailscale IPv4 Address:\n{}", get_tailscale_ip())),
                    horizontal_space(),
                    // Text widget to hold the status of the Tailscale connection based on the connection_status bool.
                    Text::new(match connection_status {
                        true => format!("Tailscale Connected"),
                        false => format!("Tailscale Not Connected")
                    })
                ]
                .padding(5)
                .align_items(Alignment::Start),
                row![
                    // Text widget to hold status of if SSH is enabled or not.
                    text({
                        match get_tailscale_ssh_status() {
                            true => format!("SSH is enabled"),
                            false => format!("SSH is disabled")
                        }
                    }),
                    horizontal_space(),
                    // Text widget to hold the status of if routes are allowed or not.
                    text({
                        match get_tailscale_routes_status() {
                            true => format!("Routes are allowed"),
                            false => format!("Routes are disabled")
                        }
                    })
                ]
                .padding(5)
                .align_items(Alignment::Start)
            ])
            .align_x(Horizontal::Left)
            .align_y(Vertical::Top)
            .width(740.0 * 0.4)
            .style(Container::Box);

        let checkbox_row = row![
            // Checkbox widget to enable/disable SSH
            Checkbox::new("Enable SSH", self.use_ssh)
                .on_toggle(|ssh| {
                    TailscaleActions::UseSSH(ssh)
                }),
            // Checkbox widget to enable/disable allowing routes.
            Checkbox::new("Allow Routes", self.allow_routes)
                .on_toggle(|routes| {
                    TailscaleActions::AcceptRoutes(routes)
                })
        ]
        .padding(5)
        .spacing(60)
        .align_items(Alignment::Center);

        let btn_row = row![
            // Connect button, enabled if Tailscale is not connected; disabled if it is.
            column![
                if connection_status {
                    button("Connect to Tailscale")
                    .padding(7)
                } else {
                    button("Connect to Tailscale")
                    .on_press(TailscaleActions::Up)
                    .padding(7)
                },
            ].align_items(Alignment::Start),
            column![
                // Disconnect button, enabled if Tailscale is connected, disabled if it is not.
                if connection_status {
                    button("Disconnect from Tailscale")
                    .on_press(TailscaleActions::Down)
                    .padding(7)
                } else {
                    button("Disconnect from Tailscale")
                    .padding(7)
                }
            ].align_items(Alignment::End),
        ]
        .padding(5)
        .spacing(10);

        let controls = container(column![text("Controls:"), vertical_space(), checkbox_row, btn_row])
            .align_x(Horizontal::Right)
            .align_y(Vertical::Top)
            .width(740.0 * 0.6);

        let avail = row![text(format!("{}", get_available_devices()))];

        // Layout of the GUI in the window.
        container(column![
            row![statuses, horizontal_space(), controls]
                .spacing(10)
                .align_items(Alignment::Center),
                row![],
                avail,
            ]
        )
        .padding(10)
        .align_x(Horizontal::Center)
        .into()
    }
}
