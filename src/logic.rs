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
pub fn get_tailscale_devices() -> Vec<String> {
    let output = Command::new("tailscale")
        .arg("status")
        .output()
        .expect("Failed to get Tailscale status for retrieving devices");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().map(|line| String::from(line)).collect()
}

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

pub fn tailscale_recieve() -> String {
    let output = Command::new("tailscale")
        .args(["file", "receive"])
        .output()
        .expect("Failed to recieve files");

    String::from_utf8(output.stdout).unwrap_or_default()
}

// SSH Management
lazy_static::lazy_static! {
    static ref ACTIVE_SSH_PROCESSES: Arc<Mutex<HashMap<String, Child>>> = Arc::new(Mutex::new(HashMap::new()));
}

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

        if let Some(stdout) = ACTIVE_SSH_PROCESSES
            .lock()
            .unwrap()
            .get_mut(&device)
            .and_then(|child| child.stdout.take())
        {
            let reader = BufReader::new(stdout);

            for line in reader.lines().flatten() {
                println!("{line}");
            }
        }
    }
}

pub fn send_ssh_command(device: String, command: String) {
    if let Some(child) = ACTIVE_SSH_PROCESSES.lock().unwrap().get_mut(&device) {
        if let Some(stdin) = child.stdin.as_mut() {
            writeln!(stdin, "{command}\n").unwrap();
        }
    }
}

// System Notifications
pub fn notify(title: &str, message: &str) {
    Notification::new()
        .summary(title)
        .body(message)
        .show()
        .unwrap();
}

// Sound Alerts
pub fn play_sound(file_path: &str) {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let file = File::open(file_path).unwrap();
    let source = rodio::Decoder::new(AudioBufReader::new(file)).unwrap();

    sink.append(source);
    sink.sleep_until_end();
}

// Updating mechanisms
pub fn check_flatpak_updates() {
    let output = Command::new("flatpak")
        .args(["update", "--assumeyes"])
        .output();

    if let Ok(result) = output {
        println!(
            "Flatpak update status: {}",
            String::from_utf8_lossy(&result.stdout)
        );
    } else {
        println!("Flatpak update failed");
    }
}

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
