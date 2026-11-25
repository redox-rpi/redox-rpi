//! homesync: back up the home directory to a home server with rsync over
//! SSH (redox-rpi #712, stage 1 of docs/home-server-storage.md).
//!
//! ```text
//! homesync setup <user>@<host>[:<port>]:<dir>   key, test login, remember the server
//! homesync push                                copy changes from here to the server
//! homesync pull [<path>]                       restore into ~/restore-<date>/
//! homesync status                              server, last push, what would change
//! ```
//!
//! Push is a backup: the home directory goes to `<dir>/current/`, and files
//! deleted or changed here are kept on the server in `<dir>/deleted/<date>/`.
//! Pull never writes over local files. No two-way sync, so no conflicts.

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{self, Command, ExitStatus, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// Where the server is, from `setup`.
#[derive(Debug, PartialEq)]
struct Server {
    user: String,
    host: String,
    port: u16,
    dir: String,
}

impl Server {
    /// `<user>@<host>[:<port>]:<dir>`.
    fn parse(spec: &str) -> Result<Server, String> {
        let bad = || format!("{spec:?}: expected <user>@<host>[:<port>]:<dir>");
        let (user, rest) = spec.split_once('@').ok_or_else(bad)?;
        let (host, rest) = rest.split_once(':').ok_or_else(bad)?;
        let (port, dir) = match rest.split_once(':') {
            Some((p, d)) if !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) => {
                (p.parse().map_err(|_| bad())?, d)
            }
            _ => (22, rest),
        };
        if user.is_empty() || host.is_empty() || dir.is_empty() {
            return Err(bad());
        }
        let dir = dir.trim_end_matches('/');
        let dir = if dir.is_empty() { "/" } else { dir };
        Ok(Server { user: user.into(), host: host.into(), port, dir: dir.into() })
    }

    fn spec(&self) -> String {
        format!("{}@{}:{}:{}", self.user, self.host, self.port, self.dir)
    }

    fn login(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }

    /// `<user>@<host>:<dir>/<sub>` for rsync.
    fn remote(&self, sub: &str) -> String {
        format!("{}:{}/{}", self.login(), self.dir, sub)
    }

    /// ssh options: no questions in a batch run, the host key pinned on
    /// first use, a short timeout when the server is away.
    fn ssh_opts(&self) -> Vec<String> {
        vec![
            "-p".into(),
            self.port.to_string(),
            "-o".into(),
            "BatchMode=yes".into(),
            "-o".into(),
            "StrictHostKeyChecking=accept-new".into(),
            "-o".into(),
            "ConnectTimeout=10".into(),
        ]
    }

    /// The `-e` value for rsync.
    fn rsh(&self) -> String {
        format!("ssh {}", self.ssh_opts().join(" "))
    }

    fn ssh(&self, command: &str) -> Command {
        let mut cmd = Command::new("ssh");
        cmd.args(self.ssh_opts()).arg(self.login()).arg(command);
        cmd
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| "/".into())
}

fn config_dir() -> PathBuf {
    home().join(".config/homesync")
}

/// The last push's result; under `.cache`, so a push does not back it up.
fn last_file() -> PathBuf {
    home().join(".cache/homesync/last")
}

fn load_server() -> Result<Server, String> {
    let path = config_dir().join("server");
    let text = fs::read_to_string(&path)
        .map_err(|_| "no server yet: run homesync setup <user>@<host>[:<port>]:<dir>".to_string())?;
    Server::parse(text.trim())
}

/// The default exclude list, written by `setup` if there is none.
const EXCLUDE: &str = "# Not backed up by homesync push (rsync filter patterns).\n.cache/\n/restore-*/\n";

/// The exclude list; the default one is written if there is none, so a
/// missing file never stops a push.
fn exclude_file() -> PathBuf {
    let path = config_dir().join("exclude");
    if !path.exists() {
        let _ = fs::create_dir_all(config_dir());
        let _ = fs::write(&path, EXCLUDE);
    }
    path
}

/// Local time is not known on Redox without a time zone; dates are UTC.
fn stamp() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil(days as i64);
    format!("{y:04}-{m:02}-{d:02}-{:02}{:02}{:02}", rem / 3600, rem / 60 % 60, rem % 60)
}

/// Days since 1970-01-01 to year, month, day (H. Hinnant's algorithm).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

fn run(cmd: &mut Command) -> Result<(ExitStatus, String), String> {
    let out = cmd
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("cannot run {:?}: {e}", cmd.get_program()))?;
    Ok((out.status, String::from_utf8_lossy(&out.stdout).into_owned()))
}

/// ssh exits with 255 when the connection fails.
fn unreachable(status: ExitStatus) -> bool {
    status.code() == Some(255)
}

/// rsync gives 255 when its ssh cannot connect, but sometimes 12 (protocol
/// stream error) if it notices the closed pipe before ssh's exit status: then
/// a plain ssh login decides.
fn server_away(server: &Server, status: ExitStatus) -> bool {
    match status.code() {
        Some(255) => true,
        Some(12) => server
            .ssh("true")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_or(false, unreachable),
        _ => false,
    }
}

/// A number from rsync's `--stats` output, e.g. "Number of regular files
/// transferred: 3".
fn stat(out: &str, label: &str) -> u64 {
    out.lines()
        .find_map(|l| l.trim().strip_prefix(label))
        .and_then(|v| v.trim_start_matches(':').trim().split_whitespace().next().map(|n| n.replace(',', "")))
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

fn setup(spec: &str) -> Result<(), String> {
    let server = Server::parse(spec)?;
    let ssh_dir = home().join(".ssh");
    let key = ssh_dir.join("id_ed25519");
    if !key.exists() {
        fs::create_dir_all(&ssh_dir).map_err(|e| format!("{}: {e}", ssh_dir.display()))?;
        let status = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&key)
            .status()
            .map_err(|e| format!("cannot run ssh-keygen: {e}"))?;
        if !status.success() {
            return Err("ssh-keygen failed".into());
        }
    }
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    fs::write(dir.join("server"), server.spec() + "\n").map_err(|e| e.to_string())?;
    exclude_file();
    let public = fs::read_to_string(key.with_extension("pub")).unwrap_or_default();
    println!("homesync: this Pi's key (add it to ~/.ssh/authorized_keys of {} on the server):", server.user);
    println!("{}", public.trim());
    let dirs = format!("mkdir -p '{0}/current' '{0}/deleted'", server.dir.replace('\'', ""));
    let (status, _) = run(server.ssh(&dirs).stdin(Stdio::null()))?;
    if status.success() {
        println!("homesync: login to {} works, backups go to {}/current", server.login(), server.dir);
        Ok(())
    } else if unreachable(status) {
        Err(format!("cannot log in to {} port {} yet: add the key above, then run setup again", server.login(), server.port))
    } else {
        Err(format!("could not create {} on the server", server.dir))
    }
}

fn push() -> Result<(), String> {
    let server = load_server()?;
    let when = stamp();
    let mut cmd = Command::new("rsync");
    cmd.args(["-a", "--delete", "--stats", "--backup"])
        .arg(format!("--backup-dir=../deleted/{when}"))
        .arg(format!("--exclude-from={}", exclude_file().display()))
        .arg("-e")
        .arg(server.rsh())
        .arg(format!("{}/", home().display()))
        .arg(server.remote("current/"));
    let (status, out) = run(&mut cmd)?;
    let result = if status.success() {
        let files = stat(&out, "Number of regular files transferred");
        let bytes = stat(&out, "Total transferred file size");
        format!("pushed {files} files ({bytes} bytes) to {}", server.remote("current"))
    } else if server_away(&server, status) {
        format!("server {} port {} not reachable, nothing changed", server.host, server.port)
    } else {
        format!("rsync failed ({status})")
    };
    let line = format!("{when} {} {result}\n", if status.success() { "ok" } else { "failed" });
    let _ = fs::create_dir_all(home().join(".cache/homesync"));
    let _ = fs::write(last_file(), &line);
    if status.success() {
        println!("homesync: {result}");
        Ok(())
    } else {
        Err(result)
    }
}

fn pull(path: Option<&str>) -> Result<(), String> {
    let server = load_server()?;
    let sub = path.unwrap_or("").trim_start_matches('/');
    let target = home().join(format!("restore-{}", stamp()));
    fs::create_dir_all(&target).map_err(|e| format!("{}: {e}", target.display()))?;
    let mut cmd = Command::new("rsync");
    cmd.args(["-a", "--stats", "-e"])
        .arg(server.rsh())
        .arg(server.remote(&format!("current/{sub}")))
        .arg(format!("{}/", target.display()));
    let (status, out) = run(&mut cmd)?;
    if status.success() {
        println!(
            "homesync: restored {} files into {}",
            stat(&out, "Number of regular files transferred"),
            target.display()
        );
        Ok(())
    } else if server_away(&server, status) {
        let _ = fs::remove_dir(&target);
        Err(format!("server {} port {} not reachable", server.host, server.port))
    } else {
        Err(format!("rsync failed ({status})"))
    }
}

fn status() -> Result<(), String> {
    let server = load_server()?;
    println!("server: {}", server.remote("current"));
    match fs::read_to_string(last_file()) {
        Ok(last) => print!("last push: {last}"),
        Err(_) => println!("last push: never"),
    }
    let mut cmd = Command::new("rsync");
    cmd.args(["-a", "-n", "--delete", "--itemize-changes"])
        .arg(format!("--exclude-from={}", exclude_file().display()))
        .arg("-e")
        .arg(server.rsh())
        .arg(format!("{}/", home().display()))
        .arg(server.remote("current/"));
    let (st, out) = run(&mut cmd)?;
    if st.success() {
        let changes = out.lines().filter(|l| !l.starts_with(".d")).count();
        println!("reachable: yes\nwould change: {changes}");
        Ok(())
    } else if server_away(&server, st) {
        println!("reachable: no");
        Err("server not reachable".into())
    } else {
        Err(format!("rsync failed ({st})"))
    }
}

fn usage() -> ! {
    eprintln!("usage: homesync setup <user>@<host>[:<port>]:<dir> | push | pull [<path>] | status");
    process::exit(2)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["setup", spec] => setup(spec),
        ["push"] => push(),
        ["pull"] => pull(None),
        ["pull", path] => pull(Some(path)),
        ["status"] => status(),
        _ => usage(),
    };
    let _ = io::stdout().flush();
    if let Err(err) = result {
        eprintln!("homesync: {err}");
        process::exit(1);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_server() {
        let s = Server::parse("pi@nas:2222:/backup/pi/").unwrap();
        assert_eq!(s, Server { user: "pi".into(), host: "nas".into(), port: 2222, dir: "/backup/pi".into() });
        assert_eq!(s.spec(), "pi@nas:2222:/backup/pi");
        assert_eq!(Server::parse(&s.spec()).unwrap(), s);
        let d = Server::parse("pi@nas:backup").unwrap();
        assert_eq!((d.port, d.dir.as_str()), (22, "backup"));
        assert_eq!(d.remote("current/"), "pi@nas:backup/current/");
        for bad in ["nas:/x", "pi@nas", "@nas:/x", "pi@:/x", "pi@nas:"] {
            assert!(Server::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_597), (2026, 5, 24));
        assert_eq!(civil(11_016), (2000, 2, 29));
    }

    #[test]
    fn rsync_stats() {
        let out = "Number of files: 104 (reg: 100, dir: 4)\nNumber of regular files transferred: 1\nTotal transferred file size: 524,288 bytes\n";
        assert_eq!(stat(out, "Number of regular files transferred"), 1);
        assert_eq!(stat(out, "Total transferred file size"), 524_288);
        assert_eq!(stat(out, "Missing"), 0);
    }
}
