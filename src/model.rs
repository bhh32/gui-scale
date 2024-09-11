use crate::messages::TailscaleActions;
use crate::logic::{get_tailscale_ip, tailscale_int_up};
use iced::widget::Text;
use iced::{
    widget::{
        button, checkbox, column, row,
        Theme, Renderer
    },
    Element,
    Alignment::Center,
    Sandbox
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
            use_ssh: false,
            allow_routes: false,
        }
    }
}

impl Sandbox for GuiScale {
    type Message = TailscaleActions;

    fn new() -> Self {
        Self::default()
    }

    fn title(&self) -> String {
        String::from("GUI-Scale")
    }

    fn update(&mut self, message: TailscaleActions) {
        match message {
            TailscaleActions::Up => tailscale_int_up(true, false, false),
            TailscaleActions::Down => tailscale_int_up(false, false, false),
            TailscaleActions::UseSSH(ssh) => tailscale_int_up(true, ssh, false),
            TailscaleActions::AcceptRoutes(accept_routes) => tailscale_int_up(true, false, accept_routes),
            TailscaleActions::UseSshAndAcceptRoutes(ssh, accept_routes) => tailscale_int_up(true, ssh, accept_routes),
        };
    }

    fn view(&self) -> Element<TailscaleActions> {
        let tailscale_ip = Text::new(format!("Tailscale IPv4 Address: {}", get_tailscale_ip()));
        
        column![
            tailscale_ip,
            column![

            ]
            .align_items(Center),
        ]
        .align_items(Center)
        .into()
    }
}