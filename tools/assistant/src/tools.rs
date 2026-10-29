//! The tools the assistant may run, and the rules around them.
//!
//! - Only tools on the list in `TOOLS` run; a call names one of them and
//!   gives its arguments as JSON strings. Unknown tools or arguments are
//!   refused, nothing is passed to a shell.
//! - A tool of kind `Change` runs only after the user answered `y` to a
//!   question that says in plain words what will happen.
//! - Paths are taken relative to the home folder and must stay inside it,
//!   without following links.
//! - Every call, done or refused, is appended to `~/.assistant.log`.

use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::json::Value;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// Only looks; runs at once.
    Look,
    /// Changes something; asks first.
    Change,
}

pub struct Param {
    pub name: &'static str,
    pub required: bool,
    pub description: &'static str,
}

pub struct Tool {
    pub name: &'static str,
    pub kind: Kind,
    pub description: &'static str,
    pub params: &'static [Param],
}

pub const TOOLS: &[Tool] = &[
    Tool {
        name: "show_time",
        kind: Kind::Look,
        description: "Show the current date and time.",
        params: &[],
    },
    Tool {
        name: "list_files",
        kind: Kind::Look,
        description: "List the files and folders in a folder of the user's home folder.",
        params: &[Param {
            name: "folder",
            required: false,
            description: "Folder inside the home folder, for example \"Documents\". Empty for the home folder itself.",
        }],
    },
    Tool {
        name: "make_folder",
        kind: Kind::Change,
        description: "Create a new folder in the user's home folder.",
        params: &[Param {
            name: "folder",
            required: true,
            description: "Name of the new folder, for example \"Photos\" or \"Documents/Letters\".",
        }],
    },
];

/// Where the tools work and how they ask the user.
pub struct Context<'a> {
    pub home: PathBuf,
    pub log: Option<PathBuf>,
    pub confirm: &'a mut dyn FnMut(&str) -> bool,
}

/// Run one tool call `{"name": ..., "arguments": {...}}`. The text is the
/// answer for the user (and later for the model); an error says why the
/// call was refused.
pub fn run(call: &Value, ctx: &mut Context) -> Result<String, String> {
    let result = run_inner(call, ctx);
    let name = call.get("name").and_then(Value::as_str).unwrap_or("?");
    let args = describe_args(call.get("arguments"));
    let outcome = match &result {
        Ok(_) => "ok".to_string(),
        Err(e) => format!("refused: {e}"),
    };
    log(ctx, &format!("{name} {args} -> {outcome}"));
    result
}

fn run_inner(call: &Value, ctx: &mut Context) -> Result<String, String> {
    let name = call.get("name").and_then(Value::as_str).ok_or("the call has no tool name")?;
    let tool = TOOLS
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| format!("there is no tool called \"{name}\""))?;
    let args = check_args(tool, call.get("arguments"))?;
    let arg = |key: &str| args.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str()).unwrap_or("");

    match tool.name {
        "show_time" => Ok(show_time()),
        "list_files" => list_files(&ctx.home, arg("folder")),
        "make_folder" => {
            let path = home_path(&ctx.home, arg("folder"))?;
            if path == ctx.home {
                return Err("no folder name given".into());
            }
            if path.exists() {
                return Err(format!("{} already exists", shown(&ctx.home, &path)));
            }
            let question = format!("Create the folder {} in your home folder?", shown(&ctx.home, &path));
            if !(ctx.confirm)(&question) {
                return Err("you said no, nothing was changed".into());
            }
            fs::create_dir_all(&path).map_err(|e| format!("could not create it: {e}"))?;
            Ok(format!("Created the folder {}.", shown(&ctx.home, &path)))
        }
        _ => unreachable!("tool on the list without code"),
    }
}

/// The arguments as (name, value) pairs, after checking them against the
/// tool's parameters: strings only, no unknown names, required ones there.
fn check_args(tool: &Tool, args: Option<&Value>) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    match args {
        None | Some(Value::Null) => {}
        Some(Value::Object(map)) => {
            for (key, value) in map {
                if !tool.params.iter().any(|p| p.name == key) {
                    return Err(format!("{} has no argument \"{key}\"", tool.name));
                }
                let s = value.as_str().ok_or_else(|| format!("argument \"{key}\" must be text"))?;
                out.push((key.clone(), s.to_string()));
            }
        }
        Some(_) => return Err("the arguments must be an object".into()),
    }
    for p in tool.params.iter().filter(|p| p.required) {
        if !out.iter().any(|(k, _)| k == p.name) {
            return Err(format!("{} needs the argument \"{}\"", tool.name, p.name));
        }
    }
    Ok(out)
}

fn describe_args(args: Option<&Value>) -> String {
    match args {
        Some(Value::Object(map)) => map
            .iter()
            .map(|(k, v)| format!("{k}={:?}", v.as_str().unwrap_or("?")))
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

fn log(ctx: &Context, line: &str) {
    let Some(path) = &ctx.log else { return };
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{} {line}", format_time(secs as i64));
    }
}

/// `path` resolved inside `home`: `~`, `~/x` and plain relative paths are
/// taken from the home folder, an absolute path must lie inside it. `..`
/// may not leave it, and no part of the path may be a link.
pub fn home_path(home: &Path, arg: &str) -> Result<PathBuf, String> {
    let arg = arg.trim();
    let rel = arg.strip_prefix('~').map(|r| r.trim_start_matches('/')).unwrap_or(arg);
    let rel = Path::new(rel);
    let rel = if rel.is_absolute() {
        rel.strip_prefix(home).map_err(|_| "only folders inside your home folder are allowed".to_string())?
    } else {
        rel
    };
    let mut path = home.to_path_buf();
    for c in rel.components() {
        match c {
            Component::Normal(part) => {
                path.push(part);
                if fs::symlink_metadata(&path).map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                    return Err(format!("{} is a link; links are not followed", shown(home, &path)));
                }
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if path == home {
                    return Err("only folders inside your home folder are allowed".into());
                }
                path.pop();
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err("only folders inside your home folder are allowed".into())
            }
        }
    }
    Ok(path)
}

/// A path as the user sees it: `~` or `~/Documents`.
fn shown(home: &Path, path: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rel) if rel.as_os_str().is_empty() => "~".into(),
        Ok(rel) => format!("~/{}", rel.display()),
        Err(_) => path.display().to_string(),
    }
}

const MAX_ENTRIES: usize = 100;

fn list_files(home: &Path, folder: &str) -> Result<String, String> {
    let path = home_path(home, folder)?;
    let entries = fs::read_dir(&path).map_err(|e| format!("cannot open {}: {e}", shown(home, &path)))?;
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| {
            let mut name = e.file_name().to_string_lossy().into_owned();
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                name.push('/');
            }
            name
        })
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    let total = names.len();
    if total == 0 {
        return Ok(format!("{} is empty.", shown(home, &path)));
    }
    let mut out = format!("{} has {total} entr{}:\n", shown(home, &path), if total == 1 { "y" } else { "ies" });
    for name in names.iter().take(MAX_ENTRIES) {
        out.push_str("  ");
        out.push_str(name);
        out.push('\n');
    }
    if total > MAX_ENTRIES {
        out.push_str(&format!("  … and {} more\n", total - MAX_ENTRIES));
    }
    Ok(out.trim_end().to_string())
}

fn show_time() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let mut out = format!("It is {}.", format_time_long(secs));
    if secs < 1_000_000_000 {
        out.push_str(" The clock has not been set (no network time yet).");
    }
    out
}

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September", "October",
    "November", "December",
];
const WEEKDAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// Year, month (1-12), day for days since 1970-01-01 (H. Hinnant's
/// `civil_from_days`).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// "Thursday, 29 October 2026, 12:34 UTC"
fn format_time_long(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let weekday = WEEKDAYS[(days + 3).rem_euclid(7) as usize];
    format!("{weekday}, {d} {} {y}, {:02}:{:02} UTC", MONTHS[m as usize - 1], rem / 3600, rem % 3600 / 60)
}

/// "2026-10-29T12:34:56Z", for the log.
fn format_time(secs: i64) -> String {
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    format!("{y}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    fn temp_home(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("assistant-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Documents")).unwrap();
        fs::write(dir.join("notes.txt"), "hi").unwrap();
        dir
    }

    fn call(home: &Path, json: &str, answer: bool, asked: &mut Vec<String>) -> Result<String, String> {
        let mut confirm = |q: &str| {
            asked.push(q.to_string());
            answer
        };
        let mut ctx = Context { home: home.to_path_buf(), log: Some(home.join(".assistant.log")), confirm: &mut confirm };
        run(&parse(json).unwrap(), &mut ctx)
    }

    #[test]
    fn dates() {
        assert_eq!(format_time(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_time_long(0), "Thursday, 1 January 1970, 00:00 UTC");
        assert_eq!(format_time(1_793_277_296), "2026-10-29T12:34:56Z");
        assert_eq!(format_time_long(1_793_277_296), "Thursday, 29 October 2026, 12:34 UTC");
        assert_eq!(format_time(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn paths_stay_home() {
        let home = temp_home("paths");
        assert_eq!(home_path(&home, "").unwrap(), home);
        assert_eq!(home_path(&home, "~").unwrap(), home);
        assert_eq!(home_path(&home, "~/Documents").unwrap(), home.join("Documents"));
        assert_eq!(home_path(&home, "Documents/../notes.txt").unwrap(), home.join("notes.txt"));
        assert_eq!(home_path(&home, &home.join("Documents").to_string_lossy()).unwrap(), home.join("Documents"));
        assert!(home_path(&home, "..").is_err());
        assert!(home_path(&home, "Documents/../../x").is_err());
        assert!(home_path(&home, "/etc").is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/etc", home.join("etc")).unwrap();
            assert!(home_path(&home, "etc").is_err());
        }
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn allow_list() {
        let home = temp_home("allow");
        let mut asked = Vec::new();
        let out = call(&home, r#"{"name": "list_files", "arguments": {}}"#, false, &mut asked).unwrap();
        assert!(out.contains("Documents/") && out.contains("notes.txt"), "{out}");
        assert!(call(&home, r#"{"name": "run_shell", "arguments": {"cmd": "rm -rf /"}}"#, true, &mut asked).is_err());
        assert!(call(&home, r#"{"name": "list_files", "arguments": {"path": "/"}}"#, true, &mut asked).is_err());
        assert!(call(&home, r#"{"name": "list_files", "arguments": {"folder": "/etc"}}"#, true, &mut asked).is_err());
        assert!(call(&home, r#"{"name": "list_files", "arguments": {"folder": 5}}"#, true, &mut asked).is_err());
        assert!(call(&home, r#"{"name": "show_time"}"#, false, &mut asked).unwrap().starts_with("It is "));
        assert!(asked.is_empty(), "look-only tools must not ask");
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn changes_ask_first() {
        let home = temp_home("change");
        let mut asked = Vec::new();
        let json = r#"{"name": "make_folder", "arguments": {"folder": "Photos"}}"#;
        assert!(call(&home, json, false, &mut asked).is_err());
        assert!(!home.join("Photos").exists());
        assert_eq!(asked, ["Create the folder ~/Photos in your home folder?"]);
        assert_eq!(call(&home, json, true, &mut asked).unwrap(), "Created the folder ~/Photos.");
        assert!(home.join("Photos").is_dir());
        assert!(call(&home, json, true, &mut asked).is_err(), "already exists");
        assert!(call(&home, r#"{"name": "make_folder", "arguments": {"folder": "../x"}}"#, true, &mut asked).is_err());
        assert_eq!(asked.len(), 2, "refused calls must not ask");
        let log = fs::read_to_string(home.join(".assistant.log")).unwrap();
        assert_eq!(log.lines().count(), 4);
        assert!(log.contains("make_folder folder=\"Photos\" -> refused: you said no"));
        fs::remove_dir_all(&home).unwrap();
    }
}
