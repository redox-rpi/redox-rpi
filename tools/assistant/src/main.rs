//! assistant: the tool layer of a local assistant for Redox on the Pi (#43).
//!
//! A small language model (Qwen3 0.6B, run by `rinfer`, see `model.rs`)
//! answers questions and asks for tools; this program decides what runs.
//! Tool calls can also be typed by hand, in the form the model writes them:
//!
//! ```text
//! <tool_call>{"name": "list_files", "arguments": {"folder": "Documents"}}</tool_call>
//! ```
//!
//! Usage:
//!   assistant                 interactive: questions, `/tools`, `/call <tool call>`, `/quit`
//!   assistant --no-model      the same without the language model
//!   assistant --ask <text>    answer one question with the model
//!   assistant --tools         list the tools
//!   assistant --call <call>   run one tool call; a change asks on stdin first
//!
//! ASSISTANT_MODEL sets the model file, ASSISTANT_DEBUG shows rinfer's timings.
//! See `tools.rs` for the rules (allow-list, confirmation, home folder only).

mod json;
mod model;
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
        None => interactive(&mut ctx, start_model()),
        Some("--no-model") if args.len() == 1 => interactive(&mut ctx, None),
        Some("--ask") if args.len() == 2 => {
            let Some(mut m) = start_model() else {
                return ExitCode::FAILURE;
            };
            match ask_model(&mut m, &args[1], &mut ctx) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("assistant: {e}");
                    ExitCode::FAILURE
                }
            }
        }
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
            eprintln!("usage: assistant [--no-model | --ask '<question>' | --tools | --call '<tool call>']");
            ExitCode::from(2)
        }
    }
}

fn start_model() -> Option<model::Model> {
    let path = std::env::var("ASSISTANT_MODEL").unwrap_or_else(|_| model::DEFAULT_MODEL.into());
    println!("Loading the language model, this takes a moment...");
    match model::Model::start(&path) {
        Ok(m) => Some(m),
        Err(e) => {
            println!("The language model did not start ({e}); tools only.");
            None
        }
    }
}

fn ask_model(m: &mut model::Model, question: &str, ctx: &mut Context) -> std::io::Result<()> {
    let mut print = |t: &str| {
        print!("{t}");
        let _ = io::stdout().flush();
    };
    let mut call = |text: &str| match run_call(text, ctx) {
        Ok(out) => {
            println!("{out}");
            out
        }
        Err(e) => {
            println!("Not done: {e}.");
            format!("Not done: {e}.")
        }
    };
    m.ask(question, &mut print, &mut call)
}

fn interactive(ctx: &mut Context, mut model: Option<model::Model>) -> ExitCode {
    if model.is_some() {
        println!("Assistant. Ask a question; /tools lists the tools, /quit ends.");
    } else {
        println!("Assistant (tools only). /tools lists the tools, /quit ends.");
    }
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
        } else if let Some(m) = model.as_mut() {
            if let Err(e) = ask_model(m, line, ctx) {
                println!("The language model stopped ({e}); tools only.");
                model = None;
            }
        } else {
            println!("There is no language model running. Try /tools or /call.");
        }
    }
}

/// Run a tool call given as JSON, with or without the `<tool_call>` tags.
fn call(text: &str, ctx: &mut Context) -> bool {
    match run_call(text, ctx) {
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

fn run_call(text: &str, ctx: &mut Context) -> Result<String, String> {
    let text = text.trim();
    let text = text
        .strip_prefix("<tool_call>")
        .and_then(|t| t.strip_suffix("</tool_call>"))
        .unwrap_or(text);
    let v = json::parse(text.trim()).map_err(|e| format!("not a valid tool call ({e})"))?;
    tools::run(&v, ctx)
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
