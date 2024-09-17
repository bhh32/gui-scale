use crate::messages::TailscaleActions;
use crate::logic::{get_tailscale_con_status, get_tailscale_ip, get_tailscale_routes_status, get_tailscale_ssh_status, set_routes, set_ssh, tailscale_int_up};
use iced::widget::{Button, Checkbox, Text};
use iced::{executor, Command, Executor};
use iced::{
    widget::{
        button, checkbox, column, row,
        Theme, Renderer
    },
    Element,
    Alignment::Center,
    Application
};

pub struct GuiScale {
    pub up_down: bool,
    pub use_ssh: bool,
    pub allow_routes: bool,
}

impl Default for GuiScale {
    fn default() -> Self {
        Self {
            up_down: true,
            use_ssh: get_tailscale_ssh_status(),
            allow_routes: get_tailscale_routes_status(),
        }
    }
}

impl Application for GuiScale {
    type Executor = executor::Default;
    type Message = TailscaleActions;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        (Self::default(), Command::none())
    }

    fn title(&self) -> String {
        String::from("GUI-Scale")
    }

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

    fn view(&self) -> Element<TailscaleActions> {
        let tailscale_ip = Text::new(format!("Tailscale IPv4 Address: {}", get_tailscale_ip()));
        let connection_status = get_tailscale_con_status();

        let con_status: Text<Theme, Renderer> = Text::new(match connection_status {
            true => format!("Tailscale Connected"),
            false => format!("Tailscale Not Connected")
        });
        
        let ssh_status: Text<Theme, Renderer> = Text::new(match get_tailscale_ssh_status() {
            true => format!("SSH is enabled"),
            false => format!("SSH is disabled")
        });
        let ssh_check: Checkbox<TailscaleActions, Theme, Renderer> = Checkbox::new("Enable SSH", self.use_ssh)
            .on_toggle(|ssh| {
                TailscaleActions::UseSSH(ssh)
            });
        
        let routes_status: Text<Theme, Renderer> = Text::new(match get_tailscale_routes_status() {
            true => format!("Routes are allowed"),
            false => format!("Routes are disabled")
        });
        let allow_routes_check: Checkbox<TailscaleActions, Theme, Renderer> = Checkbox::new("Allow Routes", self.allow_routes)
            .on_toggle(|routes| {
                TailscaleActions::AcceptRoutes(routes)
            });

        let mut up_btn: Button<TailscaleActions, Theme, Renderer> = if connection_status {
            button("Connect to Tailscale").padding(5)
        } else {
            button("Connect to Tailscale")
            .on_press(TailscaleActions::Up)
            .padding(5)
        };

        let mut down_btn: Button<TailscaleActions, Theme, Renderer> = if connection_status {
            button("Disconnect from Tailscale")
            .on_press(TailscaleActions::Down)
        } else {
            button("Disconnect from Tailscale")
        };

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