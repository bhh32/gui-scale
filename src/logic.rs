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
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

const REPO_OWNER: &str = "bhh32";
const REPO_NAME: &str = "gui-scale";

/// Store Tailscale status information
#[derive(Debug, Clone)]
pub struct TailscaleStatus {
    pub connected: bool,
    pub tailscale_ip: Option<String>,
    pub ssh_enabled: bool,
    pub routes: bool,
    pub allow_lan: bool,
    pub is_exit_node: bool,
    pub use_exit_node: bool,
}

impl Default for TailscaleStatus {
    fn default() -> Self {
        Self {
            connected: false,
            tailscale_ip: None,
            ssh_enabled: false,
            routes: false,
            allow_lan: false,
            is_exit_node: false,
            use_exit_node: false,
        }
    }
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
/// - each line of output is returned as a tuple of (device_name, username, tailscale_ip)
pub fn get_tailscale_devices() -> Vec<(String, String, String)> {
    let output = match Command::new("tailscale").arg("status").output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Failed to run `tailscale status`.\nTailscale is possibly not installed or isn't connected.\nError: {}", e);
            return Vec::new();
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    stdout
        .lines()
        .filter_map(|line| {
            // Skip header lines
            if line.is_empty() || line.contains("Tailscale") {
                return None;
            }

            // Split the line into parts
            let parts: Vec<&str> = line.split_whitespace().collect();

            // Ensure we have at least 3 parts (IP, device_name, username)
            if parts.len() >= 3 {
                Some((
                    parts[1].to_string(),
                    parts[2].to_string(),
                    parts[0].to_string(),
                ))
            } else {
                None
            }
        })
        .collect()
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

/// Attempt to parse Tailscale status information
pub fn get_tailscale_status() -> TailscaleStatus {
    let mut ts_status = TailscaleStatus::default();
    // Use `tailscale ip -4` command to get the IPv4 address
    let ip_output = Command::new("tailscale")
        .args(["ip", "-4"])
        .output()
        .unwrap();

    // Set the IP if we can parse it
    match String::from_utf8(ip_output.stdout) {
        Ok(ip) => ts_status.tailscale_ip = Some(ip.trim().to_string()),
        Err(_) => ts_status.tailscale_ip = None,
    };

    // Use `tailscale debug prefs` to check if we're connected
    let status_cmd = Command::new("tailscale")
        .args(["debug", "prefs"])
        .output()
        .unwrap();

    // Convert the output to a String so it can be filtered
    let status_output = String::from_utf8(status_cmd.stdout).unwrap();
    // Filter the output to find the "WantRunning" line and check if it's true
    let status_test_vec: Vec<String> = status_output
        .lines()
        .filter(|line| {
            ((line.contains("WantRunning") || line.contains("RunSSH") || line.contains("RouteAll"))
                && line.contains("true"))
                || line.contains("AdvertiseRoutes") && !line.contains("null")
        })
        .map(|line| line.to_string())
        .collect();

    match status_test_vec.len() {
        4 => {
            ts_status.connected = true;
            ts_status.ssh_enabled = true;
            ts_status.routes = true;
            ts_status.is_exit_node = true;
        }
        _ => {
            for line in status_test_vec.iter() {
                if line.contains("WantRunning") && !ts_status.connected {
                    ts_status.connected = true;
                } else if line.contains("RunSSH") && !ts_status.ssh_enabled {
                    ts_status.ssh_enabled = true;
                } else if line.contains("RouteAll") && !ts_status.routes {
                    ts_status.routes = true;
                } else if line.contains("AdvertiseRoutes") && !ts_status.is_exit_node {
                    ts_status.is_exit_node = true;
                }
            }
        }
    }

    ts_status
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
pub fn run_ssh_session(device: &String, ip: String) -> (u32, Sender<String>, Receiver<String>) {
    let (stdin_tx, stdin_rx) = channel();
    let (stdout_tx, stdout_rx) = channel();

    let mut process = Command::new("ssh")
        .args([
            "-tt", // Force pseudo-terminal allocation
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "UserKnownHostsFile=/dev/null",
            "-o",
            "ServerAliveInterval=60",
            "-o",
            "ServerAliveCountMax=5",
            &ip,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start SSH session");

    let pid = process.id();

    // Handle stdout
    let stdout = process.stdout.take().unwrap();
    let stdout_tx_clone = stdout_tx.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                let _ = stdout_tx_clone.send(line);
            }
        }
    });

    // Handle stderr
    let stderr = process.stderr.take().unwrap();
    let stderr_tx = stdout_tx.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                let _ = stderr_tx.send(format!("Error: {}", line));
            }
        }
    });

    // Handle stdin
    let mut stdin = process.stdin.take().unwrap();
    std::thread::spawn(move || {
        while let Ok(input) = stdin_rx.recv() {
            let _ = stdin.write_all(format!("{}\n", input).as_bytes());
            let _ = stdin.flush();
        }
    });

    // Store process in active processes
    ACTIVE_SSH_PROCESSES
        .lock()
        .unwrap()
        .insert(device.clone(), process);

    (pid, stdin_tx, stdout_rx)
}

pub fn send_ssh_command(device: String, command: String) -> Option<String> {
    if let Some(process) = ACTIVE_SSH_PROCESSES.lock().unwrap().get_mut(&device) {
        if let Some(stdin) = process.stdin.as_mut() {
            match writeln!(stdin, "{}", command) {
                Ok(_) => {
                    let _ = stdin.flush();
                    Some(command)
                }
                Err(e) => {
                    eprintln!("Failed to send command: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub fn terminate_ssh_session(device: &str) {
    if let Some(mut process) = ACTIVE_SSH_PROCESSES.lock().unwrap().remove(device) {
        let _ = process.kill();
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

/// Open a URL in the default browser
pub fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(&["/c", "start", url])
        .status()
        .map_err(|e| format!("Failed to open URL on Windows: {}", e));

    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open")
        .arg(url)
        .status()
        .map_err(|e| format!("Failed to open URL on macOS: {}", e));

    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open")
        .arg(url)
        .status()
        .map_err(|e| format!("Failed to open URL on Linux: {}", e));

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let result = Err("Unsupported platform for URL opening".to_string());

    // Convert the result to match the function's return type
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("{}", e);
            Err(e)
        }
    }
}
