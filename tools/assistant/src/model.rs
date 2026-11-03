//! The language model, run as a separate process: `rinfer <model> --serve`
//! (see patches/experiments/assistant/recipes/rinfer). The model only writes
//! text; tool calls in that text go through `tools::run` like typed ones.
//!
//! The conversation uses Qwen3's chat template: tools are described in the
//! system message, the model answers a tool call with
//! `<tool_call>{"name": ..., "arguments": {...}}</tool_call>`, and the result
//! goes back in a `<tool_response>` block.

use std::io::{Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use crate::tools::{Kind, TOOLS};

pub const DEFAULT_MODEL: &str = "/usr/share/assistant/models/Qwen3-0.6B-Q4_0.gguf";

/// Most tokens in one reply.
const MAX_REPLY: usize = 200;
/// Most tool calls the model may make for one question.
const MAX_ROUNDS: usize = 3;

const EOT: u8 = 0x04;

pub struct Model {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

impl Model {
    /// Start rinfer and wait until the model is loaded. Its timings go to
    /// the terminal only with ASSISTANT_DEBUG set.
    pub fn start(path: &str) -> std::io::Result<Model> {
        let stderr = if std::env::var_os("ASSISTANT_DEBUG").is_some() {
            Stdio::inherit()
        } else {
            Stdio::null()
        };
        let mut child = Command::new("rinfer")
            .arg(path)
            .arg("--serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut m = Model { child, stdin, stdout };
        m.answer(|_| {})?;
        // Reading the system prompt takes minutes on a Pi 3; the model's
        // state after it is saved once and loaded on later starts.
        let system = format!("<|im_start|>system\n{}<|im_end|>\n", system_prompt());
        match saved_prompt_file() {
            Some(file) => {
                let key = prompt_key(path, &system);
                if m.request(&format!("load {file} {key}"))? != "ok" {
                    m.feed(&system)?;
                    if let Some(dir) = std::path::Path::new(&file).parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    // A failed save only costs the same wait next time.
                    m.request(&format!("save {file} {key}"))?;
                }
            }
            None => m.feed(&system)?,
        }
        Ok(m)
    }

    /// Send a one-line request and return the answer.
    fn request(&mut self, line: &str) -> std::io::Result<String> {
        writeln!(self.stdin, "{line}")?;
        self.stdin.flush()?;
        self.answer(|_| {})
    }

    /// Read until EOT, passing the text on as it arrives.
    fn answer(&mut self, mut on_text: impl FnMut(&str)) -> std::io::Result<String> {
        let mut all = Vec::new();
        let mut shown = 0;
        let mut buf = [0u8; 256];
        loop {
            let n = self.stdout.read(&mut buf)?;
            if n == 0 {
                return Err(std::io::Error::other("the model stopped"));
            }
            let end = buf[..n].iter().position(|&b| b == EOT);
            all.extend_from_slice(&buf[..end.unwrap_or(n)]);
            // Pass on whole UTF-8 characters only.
            let valid = match std::str::from_utf8(&all[shown..]) {
                Ok(s) => s.len(),
                Err(e) => e.valid_up_to(),
            };
            if valid > 0 {
                on_text(std::str::from_utf8(&all[shown..shown + valid]).unwrap());
                shown += valid;
            }
            if end.is_some() {
                return Ok(String::from_utf8_lossy(&all).into_owned());
            }
        }
    }

    fn feed(&mut self, text: &str) -> std::io::Result<()> {
        write!(self.stdin, "feed {}\n{text}", text.len())?;
        self.stdin.flush()?;
        self.answer(|_| {}).map(|_| ())
    }

    fn gen(&mut self, on_text: impl FnMut(&str)) -> std::io::Result<String> {
        writeln!(self.stdin, "gen {MAX_REPLY}")?;
        self.stdin.flush()?;
        self.answer(on_text)
    }

    /// Answer one question. `print` shows the model's words; `call` runs a
    /// tool call (with the user's confirmation if it changes something) and
    /// returns what goes back to the model.
    pub fn ask(
        &mut self,
        question: &str,
        print: &mut dyn FnMut(&str),
        call: &mut dyn FnMut(&str) -> String,
    ) -> std::io::Result<()> {
        // The previous reply ended with <|im_end|>; its newline follows here.
        let mut turn = format!(
            "\n<|im_start|>user\n{question} /no_think<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
        for _ in 0..MAX_ROUNDS {
            self.feed(&turn)?;
            let mut hide = Hider::default();
            let reply = self.gen(|t| print(&hide.visible(t)))?;
            print(&hide.rest());
            let mut calls: Vec<String> = tool_calls(&reply).into_iter().map(String::from).collect();
            if calls.is_empty() {
                calls.extend(plain_call(&reply));
            }
            if calls.is_empty() {
                return Ok(());
            }
            let mut responses = String::new();
            for c in calls {
                responses.push_str(&format!("<tool_response>\n{}\n</tool_response>\n", call(&c)));
            }
            turn = format!(
                "\n<|im_start|>user\n{responses}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
            );
        }
        print("(Stopped after three tool calls.)\n");
        Ok(())
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The text between `<tool_call>` and `</tool_call>`, for each call.
fn tool_calls(reply: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = reply;
    while let Some(start) = rest.find("<tool_call>") {
        let after = &rest[start + "<tool_call>".len()..];
        let Some(end) = after.find("</tool_call>") else {
            out.push(after.trim());
            break;
        };
        out.push(after[..end].trim());
        rest = &after[end + "</tool_call>".len()..];
    }
    out
}

/// The small model sometimes writes a call without the tags: only the tool's
/// name, or `name(key="value", ...)`. Returns it as a JSON tool call.
fn plain_call(reply: &str) -> Option<String> {
    let r = reply.trim();
    let (name, args) = match r.split_once('(') {
        Some((n, rest)) => (n, rest.strip_suffix(')')?),
        None => (r, ""),
    };
    if !TOOLS.iter().any(|t| t.name == name) {
        return None;
    }
    let mut json = Vec::new();
    for arg in args.split(',').map(str::trim).filter(|a| !a.is_empty()) {
        let (k, v) = arg.split_once('=')?;
        let v = v.trim().strip_prefix('"')?.strip_suffix('"')?;
        json.push(format!("\"{}\": \"{}\"", escape(k.trim()), escape(v)));
    }
    Some(format!("{{\"name\": \"{name}\", \"arguments\": {{{}}}}}", json.join(", ")))
}

/// Holds back streamed text from `<tool_call>` on, so the user sees the
/// model's words but not the raw call.
#[derive(Default)]
struct Hider {
    held: String,
    hiding: bool,
}

impl Hider {
    fn visible(&mut self, text: &str) -> String {
        if self.hiding {
            return String::new();
        }
        self.held.push_str(text);
        if let Some(i) = self.held.find("<tool_call>") {
            self.hiding = true;
            let out = self.held[..i].to_string();
            self.held.clear();
            return out;
        }
        // Keep back a tail that could be the start of "<tool_call>".
        let keep = (1.."<tool_call>".len())
            .rev()
            .find(|&n| self.held.ends_with(&"<tool_call>"[..n]))
            .unwrap_or(0);
        let out = self.held[..self.held.len() - keep].to_string();
        self.held.drain(..self.held.len() - keep);
        out
    }

    fn rest(&mut self) -> String {
        let mut out = std::mem::take(&mut self.held);
        if !self.hiding && !out.ends_with('\n') {
            out.push('\n');
        }
        if self.hiding {
            out.clear();
        }
        out
    }
}

/// The system message: who the assistant is for, and the tools in the form
/// Qwen3 was trained on.
/// Where the state after the system prompt is saved: ASSISTANT_CACHE (empty
/// turns saving off), else ~/.cache/assistant/prompt.kv. None if the path
/// has spaces (rinfer's requests are split at spaces).
fn saved_prompt_file() -> Option<String> {
    let file = match std::env::var("ASSISTANT_CACHE") {
        Ok(f) => f,
        Err(_) => format!("{}/.cache/assistant/prompt.kv", std::env::var("HOME").ok()?),
    };
    (!file.is_empty() && !file.contains(char::is_whitespace)).then_some(file)
}

/// Marks a saved state as made by this model file and system prompt (FNV-1a
/// over the model's path and size and the prompt).
fn prompt_key(model: &str, system: &str) -> String {
    let size = std::fs::metadata(model).map(|m| m.len()).unwrap_or(0);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in model.bytes().chain(size.to_le_bytes()).chain(system.bytes()) {
        h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

pub fn system_prompt() -> String {
    let mut s = String::from(
        "You are the assistant on a Raspberry Pi running Redox OS. The user knows macOS, Windows \
         or a Linux shell. Answer in one to three short sentences, in plain words. \
         Redox's shell is Ion; ls, cp, mv, rm, mkdir and cat work as on Linux. \
         Use a tool when the user asks for the time or for their files.\n\n\
         # Tools\n\nYou may call one or more functions to assist with the user query.\n\n\
         You are provided with function signatures within <tools></tools> XML tags:\n<tools>\n",
    );
    for t in TOOLS {
        let mut props = String::new();
        let mut required = Vec::new();
        for p in t.params {
            if !props.is_empty() {
                props.push_str(", ");
            }
            props.push_str(&format!(
                "\"{}\": {{\"type\": \"string\", \"description\": \"{}\"}}",
                p.name,
                escape(p.description)
            ));
            if p.required {
                required.push(format!("\"{}\"", p.name));
            }
        }
        let note = match t.kind {
            Kind::Look => "",
            Kind::Change => " The user is asked before it runs.",
        };
        s.push_str(&format!(
            "{{\"type\": \"function\", \"function\": {{\"name\": \"{}\", \"description\": \"{}{}\", \
             \"parameters\": {{\"type\": \"object\", \"properties\": {{{props}}}, \"required\": [{}]}}}}}}\n",
            t.name,
            escape(t.description),
            note,
            required.join(", ")
        ));
    }
    s.push_str(
        "</tools>\n\nFor each function call, return a json object with function name and arguments \
         within <tool_call></tool_call> XML tags:\n<tool_call>\n\
         {\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call>",
    );
    s
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_tool_calls() {
        let r = "Let me look.\n<tool_call>\n{\"name\": \"show_time\", \"arguments\": {}}\n</tool_call>";
        assert_eq!(tool_calls(r), vec!["{\"name\": \"show_time\", \"arguments\": {}}"]);
        assert!(tool_calls("Paris.").is_empty());
    }

    #[test]
    fn plain_calls() {
        assert_eq!(plain_call("list_files\n").unwrap(), "{\"name\": \"list_files\", \"arguments\": {}}");
        assert_eq!(
            plain_call("list_files(folder=\"Documents\")").unwrap(),
            "{\"name\": \"list_files\", \"arguments\": {\"folder\": \"Documents\"}}"
        );
        assert!(plain_call("run_shell(cmd=\"rm\")").is_none());
        assert!(plain_call("The time is 9.").is_none());
    }

    #[test]
    fn hides_calls_while_streaming() {
        let mut h = Hider::default();
        let mut shown = String::new();
        for piece in ["Sure", ". <to", "ol_c", "all>\n{\"name\"", "}</tool_call>"] {
            shown.push_str(&h.visible(piece));
        }
        shown.push_str(&h.rest());
        assert_eq!(shown, "Sure. ");
        let mut h = Hider::default();
        let s = h.visible("a < b") + &h.rest();
        assert_eq!(s, "a < b\n");
    }

    #[test]
    fn prompt_key_changes_with_the_prompt() {
        let a = prompt_key("/nonexistent/model.gguf", "one");
        assert_eq!(a.len(), 16);
        assert_eq!(a, prompt_key("/nonexistent/model.gguf", "one"));
        assert_ne!(a, prompt_key("/nonexistent/model.gguf", "two"));
        assert_ne!(a, prompt_key("/nonexistent/other.gguf", "one"));
    }

    #[test]
    fn system_prompt_is_valid_json_per_tool() {
        let p = system_prompt();
        let tools = &p[p.find("<tools>\n").unwrap() + 8..p.rfind("</tools>").unwrap()];
        for line in tools.lines() {
            crate::json::parse(line).unwrap();
        }
    }
}
