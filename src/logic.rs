use notify_rust::Notification;
use rodio::{OutputStream, Sink};
use self_update::backends::github::ReleaseList;
use self_update::cargo_crate_version;
use self_update::Status;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufReader as AudioBufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

const REPO_OWNER: &str = "bhh32";
const REPO_NAME: &str = "gui-scale";

/// Store Tailscale status information
#[derive(Debug, Default, Clone)]
pub struct TailscaleStatus {
    pub connected: bool,
    pub tailscale_ip: Option<String>,
}

// Active SSH process store
lazy_static::lazy_static! {
    static ref ACTIVE_SSH_PROCESSES: Arc<Mutex<HashMap<String, Child>>> = Arc::new(Mutex::new(HashMap::new()));
}

/// Ensure that Tailscale has been installed
pub fn check_tailscale() {
    let output = Command::new("tailscale").arg("status").output();

    if output.is_err() {
        println!("Tailscale is not installed! Prompting user to install...");

        #[cfg(target_os = "linux")]
        Command::new("bash")
            .args(["-c", "xdg-open", "https://tailscale.com/download"])
            .spawn()
            .expect("Failed to open Tailscale download page");

        #[cfg(target_os = "windows")]
        Command::new("cmd")
            .args(["/C", "start", "https://tailscale.com/download"])
            .spawn()
            .expect("Failed to open Tailscale download page");

        #[cfg(target_os = "macos")]
        Command::new("open")
            .arg("https://tailscale.com/download")
            .spawn()
            .expect("Failed to open Tailscale download page");
    }
}

// Tailscale Commands

/// Return Tailscale devices from "tailscale status"
/// - each line of output is returned as a device entry for the GUI
pub fn get_tailscale_devices() -> Vec<String> {
    let output = match Command::new("tailscale").arg("status").output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Failed to run `tailscale status`.\nTailscale is possibly not installed or isn't connected.\nError: {}", e);
            return Vec::new();
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().map(|line| line.to_owned()).collect()
}

/// Connect, disconnect, or reconfigure Tailscale
/// e.g. tailscale up --ssh={true/false}
/// e.g. tailscale up --accept-routes={true/false}
/// e.g. tailscale up --advertise-exit-node={true/false}
/// etc.
pub fn tailscale_up(
    ssh_enabled: bool,
    accept_routes: bool,
    advertise_exit_node: bool,
    exit_node: Option<String>,
    allow_lan: bool,
) {
    let mut args: Vec<String> = vec!["up".to_string()];

    // Handle toggles
    args.push(format!(
        "--ssh={}",
        if ssh_enabled { "true" } else { "false" }
    ));
    args.push(format!(
        "--accept-routes={}",
        if accept_routes { "true" } else { "false" }
    ));
    args.push(format!(
        "--advertise-exit-node={}",
        if advertise_exit_node { "true" } else { "false" }
    ));

    if let Some(node) = exit_node {
        args.push("--exit-node".to_string());
        args.push(node);
        if allow_lan {
            args.push("--exit-node-allow-lan-access".to_string());
        }
    }

    println!("Running tailscale {:?}", args);
    let _ = Command::new("tailscale").args(&args).output();
}

/// Disconnect Tailscale connection
pub fn tailscale_down() {
    let _ = Command::new("tailscale").args(["down"]).output();
}

/// Attempt to parse Tailscale status to see if connected and get an IP
pub fn get_tailscale_status() -> TailscaleStatus {
    let output = Command::new("tailscale").arg("ip").output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        if !lines.is_empty() {
            let ip = lines[0].to_string();
            return TailscaleStatus {
                connected: true,
                tailscale_ip: Some(ip),
            };
        }
    }
    // If we get here, likely not connected
    TailscaleStatus {
        connected: false,
        tailscale_ip: None,
    }
}

/// Use Tailscale CLI to send one or more files to a device
pub fn tailscale_send(files: Vec<String>, device: &str) -> Vec<Option<String>> {
    let mut statuses: Vec<Option<String>> = Vec::new();

    files.iter().for_each(|file| {
        let output = Command::new("tailscale")
            .args(["file", "send", file, format!("{device}:").as_str()])
            .output()
            .expect("Failed to send {file} to {device}");

        if output.status.success() {
            statuses.push(None);
        } else {
            statuses.push(Some(String::from_utf8_lossy(&output.stderr).to_string()));
        }
    });

    statuses
}

/// Manually run "tailscale file receive"
pub fn tailscale_recieve() -> String {
    let output = Command::new("tailscale")
        .args(["file", "receive"])
        .output()
        .expect("Failed to recieve files");

    String::from_utf8(output.stdout).unwrap_or_default()
}

// SSH Management
pub fn run_ssh_session(device: String, ip: String) {
    let process = Command::new("ssh")
        .args([
            "-o",
            "ServerAliveInterval=60",
            "-o",
            "ServerAliveCountMax=5",
            &ip,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    if let Ok(mut child) = process {
        ACTIVE_SSH_PROCESSES
            .lock()
            .unwrap()
            .insert(device.clone(), child);

        // Read the child's stdout in a separate thread so we can print it / store it
        let mut child_stdout = ACTIVE_SSH_PROCESSES
            .lock()
            .unwrap()
            .get_mut(&device)
            .and_then(|c| c.stdout.take());

        if let Some(stdout) = child_stdout {
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().flatten() {
                    println!("[SSH] {line}");
                }
            });
        }
    }
}

pub fn send_ssh_command(device: String, command: String) {
    if let Some(child) = ACTIVE_SSH_PROCESSES.lock().unwrap().get_mut(&device) {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = writeln!(stdin, "{command}");
        }
    }
}

/// Terminate an active SSH session
pub fn terminate_ssh_session(device: &str) {
    if let Some(mut child) = ACTIVE_SSH_PROCESSES.lock().unwrap().remove(device) {
        let _ = child.kill();
    }
}

// System Notifications
pub fn notify(title: &str, message: &str) {
    Notification::new().summary(title).body(message).show();
}

// Sound Alerts
pub fn play_sound(file_path: &str) {
    if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
        if let Ok(sink) = Sink::try_new(&stream_handle) {
            if let Ok(file) = File::open(file_path) {
                if let Ok(source) = rodio::Decoder::new(AudioBufReader::new(file)) {
                    sink.append(source);
                    sink.sleep_until_end();
                }
            }
        }
    }
}

// Updating mechanisms
pub fn check_for_updates() {
    match ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
        .unwrap()
        .fetch()
    {
        Ok(releases) => {
            if let Some(latest) = releases.get(0) {
                if latest.version != cargo_crate_version!() {
                    println!("New version {} available!", latest.version);
                    update_to_latest(latest.version.clone());
                } else {
                    println!("Already up to date");
                }
            }
        }
        Err(e) => {
            println!("Update check failed: {e}");
        }
    }
}

// Helper functions
fn update_to_latest(version: String) {
    match self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("gui-scale")
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .build()
        .unwrap()
        .update()
    {
        Ok(Status::Updated(_)) => println!("Updated to version {version}"),
        Ok(Status::UpToDate(_)) => println!("Already up to date"),
        Err(e) => eprintln!("Update failed: {e}"),
    }
}
