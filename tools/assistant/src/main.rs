//! assistant: the tool layer of a local assistant for Redox on the Pi (#43).
//!
//! A small language model will answer questions and ask for tools; this
//! program decides what runs. It has no model yet: tool calls are typed by
//! hand, in the form the model (Qwen3) writes them:
//!
//! ```text
//! <tool_call>{"name": "list_files", "arguments": {"folder": "Documents"}}</tool_call>
//! ```
//!
//! Usage:
//!   assistant                 interactive: `/tools`, `/call <tool call>`, `/quit`
//!   assistant --tools         list the tools
//!   assistant --call <call>   run one tool call; a change asks on stdin first
//!
//! See `tools.rs` for the rules (allow-list, confirmation, home folder only).

mod json;
mod tools;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use tools::{Context, Kind, TOOLS};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        eprintln!("assistant: HOME is not set");
        return ExitCode::FAILURE;
    };
    let log = Some(home.join(".assistant.log"));
    let mut confirm = ask;
    let mut ctx = Context { home, log, confirm: &mut confirm };

    match args.first().map(String::as_str) {
        None => interactive(&mut ctx),
        Some("--tools") => {
            print_tools();
            ExitCode::SUCCESS
        }
        Some("--call") if args.len() == 2 => {
            if call(&args[1], &mut ctx) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        _ => {
            eprintln!("usage: assistant [--tools | --call '<tool call>']");
            ExitCode::from(2)
        }
    }
}

fn interactive(ctx: &mut Context) -> ExitCode {
    println!("Assistant (tools only, no language model yet). /tools lists the tools, /quit ends.");
    let stdin = io::stdin();
    loop {
        print!("> ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            return ExitCode::SUCCESS;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "/quit" || line == "/exit" {
            return ExitCode::SUCCESS;
        } else if line == "/tools" {
            print_tools();
        } else if let Some(rest) = line.strip_prefix("/call ") {
            call(rest, ctx);
        } else {
            println!("There is no language model in this build yet. Try /tools or /call.");
        }
    }
}

/// Run a tool call given as JSON, with or without the `<tool_call>` tags.
fn call(text: &str, ctx: &mut Context) -> bool {
    let text = text.trim();
    let text = text
        .strip_prefix("<tool_call>")
        .and_then(|t| t.strip_suffix("</tool_call>"))
        .unwrap_or(text);
    let result = json::parse(text.trim()).map_err(|e| format!("not a valid tool call ({e})"));
    match result.and_then(|v| tools::run(&v, ctx)) {
        Ok(out) => {
            println!("{out}");
            true
        }
        Err(e) => {
            println!("Not done: {e}.");
            false
        }
    }
}

/// Ask a yes/no question on the terminal; only `y` or `yes` means yes.
fn ask(question: &str) -> bool {
    print!("{question} [y/N] ");
    let _ = io::stdout().flush();
    let mut answer = String::new();
    if io::stdin().lock().read_line(&mut answer).unwrap_or(0) == 0 {
        println!();
        return false;
    }
    matches!(answer.trim().to_lowercase().as_str(), "y" | "yes")
}

fn print_tools() {
    for t in TOOLS {
        let kind = match t.kind {
            Kind::Look => "looks only",
            Kind::Change => "changes something, asks first",
        };
        println!("{} ({kind}): {}", t.name, t.description);
        for p in t.params {
            let req = if p.required { "needed" } else { "optional" };
            println!("    {} ({req}): {}", p.name, p.description);
        }
    }
}
