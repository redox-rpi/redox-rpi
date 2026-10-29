//! First-boot setup on the console (#24). getty starts it before `login`
//! while /etc/welcome.done is missing. It asks for the keyboard layout, the
//! time zone, the hostname, a root password and a Wi-Fi network, writes
//! their files and then /etc/welcome.done, so it does not run again.
//!
//! Every console shows the welcome; the setup runs on the first one where
//! someone presses Enter. The others go on to `login` when Enter is pressed
//! there.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::os::unix::fs::{symlink, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const DONE: &str = "/etc/welcome.done";
const LOCK: &str = "/tmp/welcome.lock";
const KEYMAP: &str = "/etc/keymap";
const LOCALTIME: &str = "/etc/localtime";
const ZONEINFO: &str = "/usr/share/zoneinfo";
const HOSTNAME: &str = "/etc/hostname";
const WIFI: &str = "/etc/net/wifi";

const USAGE: &str = "welcome [--force]
First-boot setup: keyboard layout, time zone, hostname, root password and
Wi-Fi. getty runs it before login until /etc/welcome.done exists.
  --force   run even when the setup is marked done (from a shell)";

/// End of input (Ctrl-D): stop without marking the setup done
struct Eof;

fn main() -> ExitCode {
    let mut force = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--force" => force = true,
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::from(if arg == "--help" { 0 } else { 2 });
            }
        }
    }
    if !force && Path::new(DONE).exists() {
        return ExitCode::SUCCESS;
    }

    println!();
    println!("Welcome to Redox OS on the Raspberry Pi.");
    println!();
    println!("This is the first start. Press Enter to set the keyboard layout,");
    println!("time zone, name, root password and Wi-Fi (takes a minute).");
    match read_line() {
        Ok(_) => {}
        Err(Eof) => return ExitCode::FAILURE,
    }
    // Someone may have done it on another console meanwhile
    if !force && Path::new(DONE).exists() {
        println!("The setup is already done.");
        return ExitCode::SUCCESS;
    }
    if OpenOptions::new().write(true).create_new(true).open(LOCK).is_err() {
        println!("The setup is running on another console. Log in here when it is done.");
        return ExitCode::SUCCESS;
    }

    let result = setup();
    let _ = fs::remove_file(LOCK);
    match result {
        Ok(()) => {
            if let Err(err) = fs::write(
                DONE,
                "First-boot setup done. Delete this file to run it again before the next login.\n",
            ) {
                println!("welcome: cannot write {DONE}: {err}");
            }
            println!();
            println!("Done. Log in as root. To change these settings later, run welcome --force.");
            println!("Press Enter for the login prompt.");
            let _ = read_line();
            ExitCode::SUCCESS
        }
        Err(Eof) => {
            println!();
            println!("Setup stopped; it starts again at the next boot.");
            ExitCode::FAILURE
        }
    }
}

fn setup() -> Result<(), Eof> {
    println!();
    println!("Press Enter to keep the value in brackets.");
    keyboard()?;
    time_zone()?;
    hostname()?;
    password()?;
    wifi()?;
    Ok(())
}

fn keyboard() -> Result<(), Eof> {
    let layouts = match Command::new("inputd").arg("--keymaps").output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    if layouts.is_empty() {
        println!("Keyboard: no layouts found (inputd --keymaps), skipped.");
        return Ok(());
    }
    let current = fs::read_to_string(KEYMAP)
        .ok()
        .and_then(|s| first_value(&s))
        .unwrap_or_else(|| "us".to_owned());
    println!();
    println!("Keyboard layouts: {}", layouts.join(", "));
    let layout = loop {
        let answer = ask("Keyboard layout", &current)?;
        if layouts.contains(&answer) {
            break answer;
        }
        println!("Not a layout here; choose one of: {}", layouts.join(", "));
    };
    match Command::new("inputd").args(["-K", &layout]).status() {
        Ok(status) if status.success() => {}
        _ => println!("welcome: inputd -K {layout} failed; the layout applies from the next boot"),
    }
    write_file(
        KEYMAP,
        &format!("# Keyboard layout, read at boot (inputd --keymaps lists them)\n{layout}\n"),
        0o644,
    );
    Ok(())
}

fn time_zone() -> Result<(), Eof> {
    let mut zones = Vec::new();
    list_zones(Path::new(ZONEINFO), &mut zones);
    zones.sort();
    if zones.is_empty() {
        println!();
        println!("Time zone: no zone files in {ZONEINFO}; the clock shows UTC.");
        return Ok(());
    }
    let current = fs::read_link(LOCALTIME)
        .ok()
        .and_then(|p| p.strip_prefix(ZONEINFO).ok().map(|p| p.display().to_string()))
        .unwrap_or_else(|| "Etc/UTC".to_owned());
    println!();
    if zones.len() <= 40 {
        println!("Time zones: {}", zones.join(", "));
    } else {
        println!("Time zones: {} (the files under {ZONEINFO})", zones.len());
    }
    let zone = loop {
        let answer = ask("Time zone", &current)?;
        if zones.contains(&answer) {
            break answer;
        }
        println!("No zone file {ZONEINFO}/{answer}.");
    };
    let target = PathBuf::from(ZONEINFO).join(&zone);
    let _ = fs::remove_file(LOCALTIME);
    if let Err(err) = symlink(&target, LOCALTIME) {
        println!("welcome: cannot link {LOCALTIME} to {}: {err}", target.display());
    }
    Ok(())
}

/// Zone names (paths relative to `ZONEINFO`) of the files under `dir`
fn list_zones(dir: &Path, zones: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            list_zones(&path, zones);
        } else if let Ok(name) = path.strip_prefix(ZONEINFO) {
            zones.push(name.display().to_string());
        }
    }
}

fn hostname() -> Result<(), Eof> {
    let current = fs::read_to_string(HOSTNAME)
        .ok()
        .and_then(|s| first_value(&s))
        .unwrap_or_else(|| "redox".to_owned());
    println!();
    let name = loop {
        let answer = ask("Name of this computer (hostname)", &current)?;
        if valid_hostname(&answer) {
            break answer;
        }
        println!("Use 1 to 63 letters, digits and '-', not at the start or end.");
    };
    // relibc's gethostname returns the whole file, so no newline
    write_file(HOSTNAME, &name, 0o644);
    Ok(())
}

fn valid_hostname(name: &str) -> bool {
    (1..=63).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-')
}

fn password() -> Result<(), Eof> {
    println!();
    let answer = ask("Set a new root password? The image's is \"password\" (y/n)", "n")?;
    if !answer.eq_ignore_ascii_case("y") && !answer.eq_ignore_ascii_case("yes") {
        return Ok(());
    }
    match Command::new("passwd").arg("root").status() {
        Ok(status) if status.success() => {}
        Ok(_) => println!("The password stays as it was."),
        Err(err) => println!("welcome: cannot run passwd: {err}"),
    }
    Ok(())
}

fn wifi() -> Result<(), Eof> {
    let current = fs::read_to_string(WIFI)
        .ok()
        .and_then(|s| s.lines().find_map(|l| l.strip_prefix("ssid=").map(str::to_owned)));
    println!();
    println!("Wi-Fi: the network to join at boot. Leave it empty for none (Ethernet only).");
    let ssid = ask("Wi-Fi network name", current.as_deref().unwrap_or(""))?;
    if ssid.is_empty() {
        if current.is_some() {
            let _ = fs::remove_file(WIFI);
        }
        return Ok(());
    }
    if Some(&ssid) == current.as_ref() {
        let answer = ask("Keep the saved passphrase? (y/n)", "y")?;
        if answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes") {
            return Ok(());
        }
    }
    if ssid.len() > 32 {
        println!("A network name has at most 32 bytes; Wi-Fi not set.");
        return Ok(());
    }
    let psk = loop {
        let psk = ask_hidden("Passphrase (8 to 63 characters, empty for an open network)")?;
        if psk.is_empty() || (8..=63).contains(&psk.len()) {
            break psk;
        }
        println!("The passphrase has {} characters; it needs 8 to 63.", psk.len());
    };
    let mut text = format!("# Wi-Fi network joined at boot, written by welcome\nssid={ssid}\n");
    if !psk.is_empty() {
        text.push_str(&format!("psk={psk}\n"));
    }
    let _ = fs::create_dir_all("/etc/net");
    write_file(WIFI, &text, 0o600);
    println!("Saved; the Wi-Fi driver joins it at the next boot.");
    Ok(())
}

/// The first line that is not empty or a comment
fn first_value(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
}

fn write_file(path: &str, text: &str, mode: u32) {
    let result = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(mode)
        .open(path)
        .and_then(|mut f: File| f.write_all(text.as_bytes()));
    if let Err(err) = result {
        println!("welcome: cannot write {path}: {err}");
    }
}

fn read_line() -> Result<String, Eof> {
    let _ = io::stdout().flush();
    let mut line = String::new();
    match io::stdin().lock().read_line(&mut line) {
        Ok(0) | Err(_) => Err(Eof),
        Ok(_) => Ok(line.trim_end_matches(['\r', '\n']).trim().to_owned()),
    }
}

/// Asks with `current` in brackets; Enter keeps it
fn ask(question: &str, current: &str) -> Result<String, Eof> {
    print!("{question} [{current}]: ");
    let answer = read_line()?;
    Ok(if answer.is_empty() { current.to_owned() } else { answer })
}

/// Asks without echoing the typed characters
fn ask_hidden(question: &str) -> Result<String, Eof> {
    print!("{question}: ");
    let _ = io::stdout().flush();
    let echo = Echo::off();
    let mut line = String::new();
    let result = io::stdin().lock().read_line(&mut line);
    drop(echo);
    println!();
    match result {
        Ok(0) | Err(_) => Err(Eof),
        // Spaces may be part of a passphrase; only the line end goes
        Ok(_) => Ok(line.trim_end_matches(['\r', '\n']).to_owned()),
    }
}

/// Terminal echo switched off until dropped
struct Echo(Option<libc::termios>);

impl Echo {
    fn off() -> Echo {
        unsafe {
            let mut term: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(0, &mut term) != 0 {
                return Echo(None);
            }
            let saved = term;
            term.c_lflag &= !libc::ECHO;
            libc::tcsetattr(0, libc::TCSANOW, &term);
            Echo(Some(saved))
        }
    }
}

impl Drop for Echo {
    fn drop(&mut self) {
        if let Some(saved) = self.0 {
            unsafe {
                libc::tcsetattr(0, libc::TCSANOW, &saved);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostnames() {
        assert!(valid_hostname("redox-pi"));
        assert!(valid_hostname("pi3"));
        assert!(!valid_hostname(""));
        assert!(!valid_hostname("-pi"));
        assert!(!valid_hostname("pi-"));
        assert!(!valid_hostname("my pi"));
        assert!(!valid_hostname(&"a".repeat(64)));
    }

    #[test]
    fn first_values() {
        assert_eq!(first_value("# comment\n\nde\n").as_deref(), Some("de"));
        assert_eq!(first_value("redox").as_deref(), Some("redox"));
        assert_eq!(first_value("# only\n"), None);
    }
}
