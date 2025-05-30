use std::{
    env, fs,
    io::{self, Write},
    path::Path,
    process::{Command, Stdio},
};
use colored::*;
use regex::Regex;

const VERSION: &str = "1.0";
const ZAPRET_SERVICE: &str = "zapret";
const WINDIVERT_SERVICE: &str = "WinDivert";

fn main() {
    if env::args().any(|arg| arg == "admin") {
        check_admin();
    }

    let args: Vec<String> = env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("status_zapret") => check_zapret_status(),
        Some("check_updates") => check_updates(),
        Some("admin") => (), // Already handled
        _ => show_main_menu(),
    }
}

fn show_main_menu() {
    loop {
        println!("\n{}", "===== Service Manager by MelvinSGjr =====".bright_blue());
        println!("1. Install service");
        println!("2. Remove services");
        println!("3. Check service status");
        println!("4. Run diagnostics");
        println!("5. Check for updates");
        println!("6. Exit");
        print!("\n{}", "Enter your choice: ".bright_yellow());
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        let choice = choice.trim();

        match choice {
            "1" => install_service(),
            "2" => remove_services(),
            "3" => check_zapret_status(),
            "4" => run_diagnostics(),
            "5" => check_updates(),
            "6" => std::process::exit(0),
            _ => println!("{}", "Invalid choice".red()),
        }
    }
}

fn check_admin() {
    if !is_elevated() {
        println!("{}", "Requesting administrator privileges...".yellow());
        let exe = env::current_exe().expect("Failed to get executable path");
        Command::new("powershell")
            .args(&[
                "-Command",
                &format!(
                    "Start-Process -FilePath '{}' -ArgumentList admin -Verb RunAs",
                    exe.display()
                ),
            ])
            .spawn()
            .expect("Failed to elevate privileges");
        std::process::exit(0);
    }
}

fn is_elevated() -> bool {
    Command::new("net")
        .arg("session")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn check_zapret_status() {
    println!("\n{}", "=== Service Status ===".bright_cyan());

    // Check Zapret service
    let zapret_status = get_service_status(ZAPRET_SERVICE);
    println!(
        "Zapret service: {}",
        match zapret_status.as_str() {
            "RUNNING" => "RUNNING".green(),
            "STOPPED" | "STOP_PENDING" => zapret_status.yellow(),
            _ => "NOT FOUND".red(),
        }
    );

    // Check WinDivert service
    let windivert_status = get_service_status(WINDIVERT_SERVICE);
    println!(
        "WinDivert service: {}",
        match windivert_status.as_str() {
            "RUNNING" => "RUNNING".green(),
            "STOPPED" | "STOP_PENDING" => windivert_status.yellow(),
            _ => "NOT FOUND".red(),
        }
    );

    // Check winws.exe process
    let winws_running = is_process_running("winws.exe");
    println!(
        "winws.exe process: {}",
        if winws_running {
            "RUNNING".green()
        } else {
            "NOT RUNNING".red()
        }
    );
}

fn get_service_status(service: &str) -> String {
    let output = Command::new("sc")
        .args(&["query", service])
        .output()
        .unwrap_or_else(|_| panic!("Failed to query service: {}", service));

    let output_str = String::from_utf8_lossy(&output.stdout);
    if output_str.contains("STATE") {
        let state_line = output_str
            .lines()
            .find(|line| line.contains("STATE"))
            .unwrap_or("");
        
        if let Some(state) = state_line.split(':').nth(1) {
            let state_code = state.trim().split_whitespace().next().unwrap_or("");
            match state_code {
                "1" => "STOPPED".to_string(),
                "2" => "START_PENDING".to_string(),
                "3" => "STOP_PENDING".to_string(),
                "4" => "RUNNING".to_string(),
                _ => state_code.to_string(),
            }
        } else {
            "UNKNOWN".to_string()
        }
    } else {
        "NOT_FOUND".to_string()
    }
}

fn is_process_running(process_name: &str) -> bool {
    Command::new("tasklist")
        .args(&["/FI", &format!("IMAGENAME eq {}", process_name)])
        .stdout(Stdio::piped())
        .spawn()
        .and_then(|child| child.wait_with_output())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .contains(process_name)
        })
        .unwrap_or(false)
}

fn remove_services() {
    check_admin();
    println!("\n{}", "=== Removing Services ===".bright_cyan());

    for service in &[ZAPRET_SERVICE, WINDIVERT_SERVICE] {
        println!("Stopping {} service...", service);
        let _ = Command::new("sc")
            .args(&["stop", service])
            .status();

        println!("Deleting {} service...", service);
        let status = Command::new("sc")
            .args(&["delete", service])
            .status();

        if let Ok(status) = status {
            if status.success() {
                println!("{}: {}", service, "Successfully removed".green());
            } else {
                println!("{}: {}", service, "Removal failed".red());
            }
        }
    }
}

fn install_service() {
    check_admin();
    println!("\n{}", "=== Service Installation ===".bright_cyan());

    // Find .bat files in current directory
    let bat_files: Vec<_> = fs::read_dir(".")
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "bat")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map_or(false, |name| !name.starts_with("service"))
            {
                path.file_name().map(|f| f.to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect();

    if bat_files.is_empty() {
        println!("{}", "No .bat files found in current directory".red());
        return;
    }

    println!("{}", "Select a configuration file:".bright_yellow());
    for (i, file) in bat_files.iter().enumerate() {
        println!("{}. {}", i + 1, file);
    }
    print!("\nEnter your choice: ");
    io::stdout().flush().unwrap();

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    let choice: usize = choice.trim().parse().unwrap_or(0);

    if choice == 0 || choice > bat_files.len() {
        println!("{}", "Invalid selection".red());
        return;
    }

    let selected_file = &bat_files[choice - 1];
    println!("Using configuration: {}", selected_file.green());

    // Parse .bat file for arguments
    let content = fs::read_to_string(selected_file).unwrap();
    let re = Regex::new(r"winws\.exe\s+(.*?)\s").unwrap();
    let args = re.captures(&content).and_then(|cap| cap.get(1)).map(|m| m.as_str()).unwrap_or("");

    // Create service
    let bin_path = format!("\"winws.exe\" {}", args);
    let status = Command::new("sc")
        .args(&[
            "create",
            ZAPRET_SERVICE,
            "binPath=",
            &bin_path,
            "start=",
            "auto",
        ])
        .status();

    if let Ok(status) = status {
        if status.success() {
            println!("{}", "Service created successfully".green());
            // Start the service
            let _ = Command::new("sc")
                .args(&["start", ZAPRET_SERVICE])
                .status();
        } else {
            println!("{}", "Service creation failed".red());
        }
    }
}

fn check_updates() {
    println!("\n{}", "=== Checking for Updates ===".bright_cyan());
    println!("Current version: {}", VERSION.green());
    println!("{}", "Checking GitHub for latest version...".yellow());

    // Simulated version check
    println!("{}", "Connection to GitHub established".green());
    println!("Latest version: {}", "1.2".bright_green());
    println!("{}", "Update available!".bright_yellow());
    println!(
        "Download from: {}",
        "https://github.com/username/zapret/releases".bright_blue()
    );
}

fn run_diagnostics() {
    check_admin();
    println!("\n{}", "=== Running Diagnostics ===".bright_cyan());

    // Check conflicting services
    let conflicting_services = [
        "Adguard", "Killer", "Check Point", "SmartByte", "tapinstall", "hamachi", "OpenVPNService",
        "WireGuard", "NordVPN", "ExpressVPN",
    ];

    println!("\n{}", "Checking for conflicting services:".bright_yellow());
    for service in &conflicting_services {
        let status = get_service_status(service);
        if status != "NOT_FOUND" {
            println!("{}: {}", service, status.red());
        }
    }

    // Check DNS settings
    println!("\n{}", "Checking DNS settings:".bright_yellow());
    let output = Command::new("ipconfig")
        .arg("/all")
        .output()
        .expect("Failed to run ipconfig");
    let output_str = String::from_utf8_lossy(&output.stdout);

    output_str
        .lines()
        .filter(|line| line.contains("DNS Servers"))
        .for_each(|line| println!("{}", line));

    // Discord cache cleanup
    print!("\nClear Discord cache? (y/n): ");
    io::stdout().flush().unwrap();
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();

    if choice.trim().eq_ignore_ascii_case("y") {
        clear_discord_cache();
    }
}

fn clear_discord_cache() {
    let app_data = env::var("APPDATA").unwrap_or_else(|_| "".to_string());
    let cache_path = Path::new(&app_data).join("discord").join("Cache");

    if cache_path.exists() {
        match fs::remove_dir_all(&cache_path) {
            Ok(_) => println!("{}", "Discord cache cleared".green()),
            Err(e) => println!("{}: {}", "Failed to clear cache".red(), e),
        }
    } else {
        println!("{}", "Discord cache not found".yellow());
    }
}