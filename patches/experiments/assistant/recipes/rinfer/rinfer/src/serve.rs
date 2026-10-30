//! `rinfer <model.gguf> --serve`: keep the model loaded and take requests on
//! stdin, for the `assistant` program. The model process runs no tools; the
//! assistant decides what the model's text leads to.
//!
//! Requests, one per line:
//!   feed <n>    followed by n bytes of text (chat template included); the
//!               text is added to the conversation
//!   gen <max>   write the reply, at most max tokens, until `<|im_end|>`
//!   reset       start a new conversation
//!
//! Every answer ends with the byte 0x04 (EOT): `feed` and `reset` answer with
//! nothing else, `gen` with the reply text as it is written. The first EOT,
//! before any request, says the model is loaded. Timings go to stderr.
use std::io::{BufRead, Read, Write};
use std::time::Instant;

use candle::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_qwen3::ModelWeights;
use tokenizers::Tokenizer;

const EOT: u8 = 0x04;

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
            "rinfer: wrote {} tokens in {secs:.1} s ({:.2} t/s), {} tokens in the cache, heap {} MiB (peak {} MiB)",
            reply.len(),
            reply.len() as f32 / secs,
            self.pos,
            super::heap_mib(),
            super::peak_mib()
        );
        Ok(())
    }
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
        let mut words = line.split_whitespace();
        let result = match (words.next(), words.next().and_then(|n| n.parse::<usize>().ok())) {
            (Some("feed"), Some(n)) => {
                let mut buf = vec![0; n];
                input.read_exact(&mut buf)?;
                let text = String::from_utf8(buf)?;
                s.feed(&text)
            }
            (Some("gen"), Some(max)) => s.gen(max, &mut out),
            (Some("reset"), None) => {
                s.model.clear_kv_cache();
                s.pos = 0;
                s.pending = None;
                s.logits = None;
                Ok(())
            }
            (None, _) => continue,
            _ => Err(anyhow::anyhow!("unknown request: {}", line.trim())),
        };
        if let Err(e) = result {
            eprintln!("rinfer: {e}");
        }
        out.write_all(&[EOT])?;
        out.flush()?;
    }
}
