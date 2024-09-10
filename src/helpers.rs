use std::{io::Error, process::{Command, Output}};

pub fn get_tailscale_ip() -> String {
    let ip_cmd = Command::new("tailscale")
        .args(["ip", "-4"])
        .output()
        .unwrap();

    String::from_utf8(ip_cmd.stdout).expect("Could get tailscale IPv4 address!")
}

pub fn tailscale_int_up(up_down: bool, allow_ssh: bool, accept_routes: bool) -> bool {
    let ssh_routes = (allow_ssh, accept_routes);
    let cmd: Result<Output, Error>;
    if up_down {
        cmd = match ssh_routes {
            (true, false) => Command::new("tailscale")
                .args(["up", "--ssh"])
                .output(),
            (false, true) => Command::new("tailscale")
                .args(["up", "--accept-routes"])
                .output(),
            (true, true) => Command::new("tailscale")
                .args(["up", "--ssh", "--accept-routes"])
                .output(),
            (false, false) => Command::new("nmcli")
                .arg("up")
                .output(),
        };
        

        return match cmd {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        cmd = Command::new("tailscale")
            .arg("down")
            .output();

        return match cmd {
            Ok(_) => false,
            Err(_) => true,

        }
    }
}