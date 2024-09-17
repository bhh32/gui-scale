use std::{
    io::Error, 
    process::{Command, Output, Stdio}
};

pub fn get_tailscale_ip() -> String {
    let ip_cmd = Command::new("tailscale")
        .args(["ip", "-4"])
        .output()
        .unwrap();

    match String::from_utf8(ip_cmd.stdout) {
        Ok(ip) => ip,
        Err(e) => format!("Could get tailscale IPv4 address!\n{e}"),
    }
}

pub fn get_tailscale_con_status() -> bool {
    let con_cmd = Command::new("tailscale")
        .args(["debug", "prefs"])
        .stdout(Stdio::piped())
        .spawn();

    let grep_cmd = Command::new("grep")
        .arg("WantRunning")
        .stdin(con_cmd.unwrap().stdout.unwrap())
        .output();

    let con_status = String::from_utf8(grep_cmd.unwrap().stdout).unwrap();

    if con_status.contains("true") {
        return true;
    }

    false
}

pub fn get_tailscale_ssh_status() -> bool {
    let ssh_cmd = Command::new("tailscale")
    .args(["debug", "prefs"])
    .stdout(Stdio::piped())
    .spawn();

    let grep_cmd = Command::new("grep")
        .arg("RunSSH")
        .stdin(ssh_cmd.unwrap().stdout.unwrap())
        .output();

    let ssh_status = String::from_utf8(grep_cmd.unwrap().stdout).unwrap();

    if ssh_status.contains("true") {
        return true;
    }

    false
}

pub fn get_tailscale_routes_status() -> bool {
    let ssh_cmd = Command::new("tailscale")
    .args(["debug", "prefs"])
    .stdout(Stdio::piped())
    .spawn();

    let grep_cmd = Command::new("grep")
        .arg("RouteAll")
        .stdin(ssh_cmd.unwrap().stdout.unwrap())
        .output();

    let ssh_status = String::from_utf8(grep_cmd.unwrap().stdout).unwrap();

    if ssh_status.contains("true") {
        return true;
    }

    false
}

pub fn get_tailscale_up_status() -> String {
    todo!()
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
            (false, false) => Command::new("tailscale")
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

pub fn set_ssh(ssh: bool) -> bool {
    let cmd: Result<Output, Error>;
    
    if ssh {
        cmd = Command::new("tailscale")
        .args(["set", "--ssh"])
        .output();
    } else {
        cmd = Command::new("tailscale")
            .args(["set", "--ssh=false"])
            .output();
    }

    match cmd {
        Ok(_) => true,
        Err(e) => {
            println!("Error occurred: {e}");
            false
        }
    }
}

pub fn set_routes(accept_routes: bool) -> bool {
    let cmd: Result<Output, Error>;
    
    if accept_routes {
        cmd = Command::new("tailscale")
        .args(["set", "--accept-routes"])
        .output();
    } else {
        cmd = Command::new("tailscale")
            .args(["set", "--accept-routes=false"])
            .output();
    }

    match cmd {
        Ok(_) => true,
        Err(e) => {
            println!("Error occurred: {e}");
            false
        }
    }
}