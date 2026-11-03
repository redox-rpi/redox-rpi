//! `rinfer <model.gguf> --serve`: keep the model loaded and take requests on
//! stdin, for the `assistant` program. The model process runs no tools; the
//! assistant decides what the model's text leads to.
//!
//! Requests, one per line:
//!   feed <n>    followed by n bytes of text (chat template included); the
//!               text is added to the conversation
//!   gen <max>   write the reply, at most max tokens, until `<|im_end|>`
//!   reset       start a new conversation
//!   save <path> <key>
//!               write the conversation so far (the key/value cache) to a
//!               file, marked with key (no spaces)
//!   load <path> <key>
//!               continue from such a file, if it has the same key
//!
//! Every answer ends with the byte 0x04 (EOT): `feed` and `reset` answer with
//! nothing else, `gen` with the reply text as it is written, `save` and
//! `load` with `ok` or `no` (`load` then starts a new conversation). The
//! first EOT, before any request, says the model is loaded. Timings go to
//! stderr.
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::time::Instant;

use candle::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_qwen3::ModelWeights;
use tokenizers::Tokenizer;

const EOT: u8 = 0x04;
/// First line of a saved conversation; the key follows on the second.
const SAVE_MAGIC: &str = "rinfer-kv 1";
/// The first line while a save is being written; as long as SAVE_MAGIC.
const UNFINISHED: &str = "rinfer-kv -";

struct Session<'a> {
    model: &'a mut ModelWeights,
    tokenizer: &'a Tokenizer,
    device: Device,
    /// Tokens in the key/value cache.
    pos: usize,
    /// Written by `gen` but not yet run through the model (the last token).
    pending: Option<u32>,
    /// Logits after the last token run, for the first token of a reply.
    logits: Option<Tensor>,
}

impl Session<'_> {
    fn run(&mut self, tokens: &[u32]) -> anyhow::Result<()> {
        if tokens.is_empty() {
            return Ok(());
        }
        let logits = if super::step_mode() {
            let mut last = None;
            for &t in tokens {
                let input = Tensor::new(&[t], &self.device)?.unsqueeze(0)?;
                last = Some(self.model.forward(&input, self.pos)?.squeeze(0)?);
                self.pos += 1;
            }
            last.unwrap()
        } else {
            let input = Tensor::new(tokens, &self.device)?.unsqueeze(0)?;
            let l = self.model.forward(&input, self.pos)?.squeeze(0)?;
            self.pos += tokens.len();
            l
        };
        self.logits = Some(logits);
        Ok(())
    }

    fn feed(&mut self, text: &str) -> anyhow::Result<()> {
        let t = Instant::now();
        let mut tokens: Vec<u32> = self.pending.take().into_iter().collect();
        let enc = self.tokenizer.encode(text, false).map_err(anyhow::Error::msg)?;
        tokens.extend_from_slice(enc.get_ids());
        self.run(&tokens)?;
        eprintln!(
            "rinfer: read {} tokens in {:.1} s, heap {} MiB (peak {} MiB)",
            tokens.len(),
            t.elapsed().as_secs_f32(),
            super::heap_mib(),
            super::peak_mib()
        );
        Ok(())
    }

    fn gen(&mut self, max: usize, out: &mut impl Write) -> anyhow::Result<()> {
        if let Some(p) = self.pending.take() {
            self.run(&[p])?;
        }
        let Some(mut logits) = self.logits.take() else {
            anyhow::bail!("nothing to answer: feed text first");
        };
        let eos = self.tokenizer.token_to_id("<|im_end|>");
        let mut lp = LogitsProcessor::from_sampling(1, Sampling::ArgMax);
        let t = Instant::now();
        let sleeps0 = candle::utils::POOL_SLEEPS.load(std::sync::atomic::Ordering::Relaxed);
        let mut reply = Vec::new();
        let mut shown = 0;
        loop {
            let next = lp.sample(&logits)?;
            if Some(next) == eos || reply.len() == max {
                // Run through the model with the next feed, so the cache
                // holds the whole conversation.
                self.pending = Some(next);
                break;
            }
            reply.push(next);
            let s = self.tokenizer.decode(&reply, false).map_err(anyhow::Error::msg)?;
            // A token can end inside a UTF-8 character; wait for the rest.
            if !s.ends_with('\u{fffd}') {
                out.write_all(&s.as_bytes()[shown..])?;
                out.flush()?;
                shown = s.len();
            }
            let input = Tensor::new(&[next], &self.device)?.unsqueeze(0)?;
            logits = self.model.forward(&input, self.pos)?.squeeze(0)?;
            self.pos += 1;
        }
        let secs = t.elapsed().as_secs_f32();
        eprintln!(
            "rinfer: wrote {} tokens in {secs:.1} s ({:.2} t/s, {:.0} pool sleeps per token), {} tokens in the cache, heap {} MiB (peak {} MiB)",
            reply.len(),
            reply.len() as f32 / secs,
            (candle::utils::POOL_SLEEPS.load(std::sync::atomic::Ordering::Relaxed) - sleeps0) as f32
                / reply.len().max(1) as f32,
            self.pos,
            super::heap_mib(),
            super::peak_mib()
        );
        Ok(())
    }

    fn save(&mut self, path: &str, key: &str) -> anyhow::Result<()> {
        if let Some(p) = self.pending.take() {
            self.run(&[p])?;
        }
        let t = Instant::now();
        // Written in place with a placeholder first line, which becomes
        // SAVE_MAGIC only once the rest is on disk: a file cut short is then
        // refused by `load`. (A temporary file and a rename would be the
        // usual way, but rename fails on /scheme/data, #57.)
        let mut w = BufWriter::with_capacity(1 << 20, File::create(path)?);
        write!(w, "{UNFINISHED}\n{key}\n")?;
        let n = self.model.save_kv_cache(&mut w)?;
        let mut f = w.into_inner().map_err(|e| e.into_error())?;
        f.sync_all()?;
        f.seek(SeekFrom::Start(0))?;
        f.write_all(SAVE_MAGIC.as_bytes())?;
        f.sync_all()?;
        eprintln!("rinfer: saved {n} tokens in {:.1} s", t.elapsed().as_secs_f32());
        Ok(())
    }

    fn load(&mut self, path: &str, key: &str) -> anyhow::Result<()> {
        let t = Instant::now();
        self.reset();
        let mut r = BufReader::with_capacity(1 << 20, File::open(path)?);
        let mut head = String::new();
        for _ in 0..2 {
            r.read_line(&mut head)?;
        }
        if head != format!("{SAVE_MAGIC}\n{key}\n") {
            anyhow::bail!("{path} is for another model or prompt");
        }
        match self.model.load_kv_cache(&mut r) {
            Ok(n) => self.pos = n,
            Err(e) => {
                self.reset();
                return Err(e.into());
            }
        }
        eprintln!(
            "rinfer: loaded {} tokens in {:.1} s, heap {} MiB (peak {} MiB)",
            self.pos,
            t.elapsed().as_secs_f32(),
            super::heap_mib(),
            super::peak_mib()
        );
        Ok(())
    }

    fn reset(&mut self) {
        self.model.clear_kv_cache();
        self.pos = 0;
        self.pending = None;
        self.logits = None;
    }
}

/// `ok` or `no` for `save` and `load`; the reason goes to stderr.
fn status(result: anyhow::Result<()>, out: &mut impl Write) -> anyhow::Result<()> {
    if let Err(e) = result {
        eprintln!("rinfer: {e}");
        out.write_all(b"no")?;
    } else {
        out.write_all(b"ok")?;
    }
    Ok(())
}

pub fn serve(model: &mut ModelWeights, tokenizer: &Tokenizer) -> anyhow::Result<()> {
    let mut s = Session { model, tokenizer, device: Device::Cpu, pos: 0, pending: None, logits: None };
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    out.write_all(&[EOT])?;
    out.flush()?;
    let mut line = String::new();
    loop {
        line.clear();
        if input.read_line(&mut line)? == 0 {
            return Ok(());
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let result = match words[..] {
            [] => continue,
            ["feed", n] if n.parse::<usize>().is_ok() => {
                let mut buf = vec![0; n.parse().unwrap()];
                input.read_exact(&mut buf)?;
                let text = String::from_utf8(buf)?;
                s.feed(&text)
            }
            ["gen", max] if max.parse::<usize>().is_ok() => s.gen(max.parse().unwrap(), &mut out),
            ["reset"] => {
                s.reset();
                Ok(())
            }
            ["save", path, key] => {
                let r = s.save(path, key);
                status(r, &mut out)
            }
            ["load", path, key] => {
                let r = s.load(path, key);
                status(r, &mut out)
            }
            _ => Err(anyhow::anyhow!("unknown request: {}", line.trim())),
        };
        if let Err(e) = result {
            eprintln!("rinfer: {e}");
        }
        out.write_all(&[EOT])?;
        out.flush()?;
    }
}
