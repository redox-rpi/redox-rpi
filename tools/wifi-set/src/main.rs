//! `wifi-set`: writes the Wi-Fi settings file `/etc/net/wifi` for the
//! Wi-Fi driver (`bcm2835-sdiod`, experiment `wifi-sdio`, redox-rpi #26).
//!
//! It asks for the network name and the passphrase with the terminal echo
//! switched off, so neither shows on the console or in a serial log, and
//! writes them as `ssid_hex=` and `psk_hex=` lines, readable by root only.
//! It prints only their lengths. An empty passphrase means an open network.
//! A test script on the build host can feed the two lines over the serial
//! console once each prompt has appeared (`wifi-config.py --serial`).

use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::process;

const WIFI: &str = "/etc/net/wifi";

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| WIFI.to_string());
    let Some(ssid) = ask_hidden("wifi-set: network name (not shown)") else {
        eprintln!("wifi-set: no input");
        process::exit(1);
    };
    let Some(psk) = ask_hidden("wifi-set: passphrase (not shown, empty for an open network)") else {
        eprintln!("wifi-set: no input");
        process::exit(1);
    };
    let text = match settings(ssid.as_bytes(), psk.as_bytes()) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("wifi-set: {err}");
            process::exit(2);
        }
    };
    if let Err(err) = write_private(&path, &text) {
        eprintln!("wifi-set: {path}: {err}");
        process::exit(1);
    }
    println!(
        "wifi-set: {path} written: network name {} bytes, passphrase {} bytes",
        ssid.len(),
        psk.len()
    );
}

/// The settings file's text; checks the lengths the driver accepts.
fn settings(ssid: &[u8], psk: &[u8]) -> Result<String, String> {
    if ssid.is_empty() || ssid.len() > 32 {
        return Err(format!("network name of {} bytes, needs 1 to 32", ssid.len()));
    }
    let mut text = format!("ssid_hex={}\n", hex(ssid));
    if !psk.is_empty() {
        if !(8..=63).contains(&psk.len()) {
            return Err(format!("passphrase of {} characters, needs 8 to 63", psk.len()));
        }
        text.push_str(&format!("psk_hex={}\n", hex(psk)));
    }
    Ok(text)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Writes the file with mode 600 from the start.
fn write_private(path: &str, text: &str) -> io::Result<()> {
    let _ = fs::remove_file(path);
    let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)?;
    file.write_all(text.as_bytes())
}

/// Asks without echoing the typed characters; None at the end of input.
/// The echo goes off before the question appears, so whoever answers on
/// seeing it is never echoed.
fn ask_hidden(question: &str) -> Option<String> {
    let echo = Echo::off();
    print!("{question}: ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let result = io::stdin().lock().read_line(&mut line);
    drop(echo);
    println!();
    match result {
        Ok(0) | Err(_) => None,
        // Spaces may be part of a name or passphrase; only the line end goes.
        Ok(_) => Some(line.trim_end_matches(['\r', '\n']).to_owned()),
    }
}

/// Terminal echo switched off until dropped (as in `welcome`).
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
    fn wpa2_network() {
        assert_eq!(
            settings(b"Lab net", b"12345678").unwrap(),
            "ssid_hex=4c6162206e6574\npsk_hex=3132333435363738\n"
        );
    }

    #[test]
    fn open_network() {
        assert_eq!(settings(b"x", b"").unwrap(), "ssid_hex=78\n");
    }

    #[test]
    fn bad_lengths() {
        assert!(settings(b"", b"12345678").is_err());
        assert!(settings(&[b'a'; 33], b"12345678").is_err());
        assert!(settings(b"x", b"1234567").is_err());
        assert!(settings(b"x", &[b'p'; 64]).is_err());
    }
}
