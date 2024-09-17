use crate::messages::TailscaleActions;
use crate::logic::{get_tailscale_con_status, get_tailscale_ip, get_tailscale_routes_status, get_tailscale_ssh_status, set_routes, set_ssh, tailscale_int_up};
use iced::widget::{Button, Checkbox, Text};
use iced::{executor, Command};
use iced::{
    widget::{
        button, column, row,
        Theme, Renderer
    },
    Element,
    Alignment::Center,
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
        // Text widget to hold the Tailscale IPv4 address of the running computer.
        let tailscale_ip = Text::new(format!("Tailscale IPv4 Address: {}", get_tailscale_ip()));
        
        // Bool to hold if Tailscale is currently connected or not.
        let connection_status = get_tailscale_con_status();

        // Text widget to hold the status of the Tailscale connection based on the connection_status bool.
        let con_status: Text<Theme, Renderer> = Text::new(match connection_status {
            true => format!("Tailscale Connected"),
            false => format!("Tailscale Not Connected")
        });
        
        // Text widget to hold status of if SSH is enabled or not.
        let ssh_status: Text<Theme, Renderer> = Text::new(match get_tailscale_ssh_status() {
            true => format!("SSH is enabled"),
            false => format!("SSH is disabled")
        });

        // Checkbox widget to enable/disable SSH
        let ssh_check: Checkbox<TailscaleActions, Theme, Renderer> = Checkbox::new("Enable SSH", self.use_ssh)
            .on_toggle(|ssh| {
                TailscaleActions::UseSSH(ssh)
            });
        
        // Text widget to hold the status of if routes are allowed or not.
        let routes_status: Text<Theme, Renderer> = Text::new(match get_tailscale_routes_status() {
            true => format!("Routes are allowed"),
            false => format!("Routes are disabled")
        });

        // Checkbox widget to enable/disable allowing routes.
        let allow_routes_check: Checkbox<TailscaleActions, Theme, Renderer> = Checkbox::new("Allow Routes", self.allow_routes)
            .on_toggle(|routes| {
                TailscaleActions::AcceptRoutes(routes)
            });
        
        // Connect button, enabled if Tailscale is not connected; disabled if it is.
        let up_btn: Button<TailscaleActions, Theme, Renderer> = if connection_status {
            button("Connect to Tailscale").padding(5)
        } else {
            button("Connect to Tailscale")
            .on_press(TailscaleActions::Up)
            .padding(5)
        };

        // Disconnect button, enabled if Tailscale is connected, disabled if it is not.
        let down_btn: Button<TailscaleActions, Theme, Renderer> = if connection_status {
            button("Disconnect from Tailscale")
            .on_press(TailscaleActions::Down)
        } else {
            button("Disconnect from Tailscale")
        };

        // Layout of the GUI in the window.
        column![
            tailscale_ip,
            con_status,
            row![
                ssh_check,
                allow_routes_check,
            ]
            .spacing(10)
            .padding(5)
            .align_items(Center),
            row![
                up_btn,
                down_btn,
            ]
            .spacing(2)
            .padding(8)
            .align_items(Center),
            Text::new("Status Messages:"),
            row![
                ssh_status,
                routes_status,
            ]
            .spacing(12)
            .padding(8)
            .align_items(Center),
        ]
        .align_items(Center)
        .into()
    }
}