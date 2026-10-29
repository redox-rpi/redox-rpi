//! Minimal Qwen3 GGUF inference with candle, for testing on Redox.
//! Usage: rinfer <model.gguf> <prompt> [tokens]
use std::io::{Read, Seek, SeekFrom, Write};
use std::time::Instant;

use candle::quantized::gguf_file;
use candle::quantized::tokenizer::TokenizerFromGguf;
use candle::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_qwen3::ModelWeights;
use tokenizers::Tokenizer;

/// Reads at most 1 MiB per call. candle reads each tensor with one
/// `read_exact`, up to 127 MB for the token table; RedoxFS then allocates a
/// buffer of the same size, and the extra copies ran the 1 GiB Pi (QEMU) out
/// of memory.
struct ChunkedReader<R>(R);

impl<R: Read> Read for ChunkedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = buf.len().min(1 << 20);
        self.0.read(&mut buf[..n])
    }
}

impl<R: Seek> Seek for ChunkedReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.0.seek(pos)
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("model path");
    let prompt = args.get(2).map(String::as_str).unwrap_or("The capital of France is");
    let n: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(16);
    // candle sizes its thread pool from RAYON_NUM_THREADS or the CPU count,
    // which Redox does not report (candle then uses one thread). The Pi 3
    // has four cores.
    if std::env::var_os("RAYON_NUM_THREADS").is_none() {
        let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).max(4);
        std::env::set_var("RAYON_NUM_THREADS", n.to_string());
    }
    println!("neon: {}, threads: {}", candle::utils::with_neon(), candle::utils::get_num_threads());

    let t0 = Instant::now();
    let mut file = ChunkedReader(std::fs::File::open(path)?);
    let content = gguf_file::Content::read(&mut file)?;
    let tokenizer = Tokenizer::from_gguf(&content)?;
    let device = Device::Cpu;
    let mut model = ModelWeights::from_gguf(content, &mut file, &device)?;
    println!("loaded in {:.1} s", t0.elapsed().as_secs_f32());

    let text = format!("<|im_start|>user\n{prompt} /no_think<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n");
    let tokens = tokenizer.encode(text, true).map_err(anyhow::Error::msg)?;
    let tokens = tokens.get_ids().to_vec();
    let mut lp = LogitsProcessor::from_sampling(1, Sampling::ArgMax);
    let eos = tokenizer.token_to_id("<|im_end|>");

    let t1 = Instant::now();
    let input = Tensor::new(tokens.as_slice(), &device)?.unsqueeze(0)?;
    let logits = model.forward(&input, 0)?.squeeze(0)?;
    let mut next = lp.sample(&logits)?;
    let prompt_s = t1.elapsed().as_secs_f32();

    let mut out = vec![next];
    let mut shown = 0;
    let t2 = Instant::now();
    for i in 0..n.saturating_sub(1) {
        if Some(next) == eos {
            break;
        }
        let s = tokenizer.decode(&out, true).map_err(anyhow::Error::msg)?;
        print!("{}", &s[shown..]);
        shown = s.len();
        std::io::stdout().flush()?;
        let input = Tensor::new(&[next], &device)?.unsqueeze(0)?;
        let logits = model.forward(&input, tokens.len() + i)?.squeeze(0)?;
        next = lp.sample(&logits)?;
        out.push(next);
    }
    let s = tokenizer.decode(&out, true).map_err(anyhow::Error::msg)?;
    println!("{}", &s[shown.min(s.len())..]);
    let gen_s = t2.elapsed().as_secs_f32();
    println!("prompt: {} tokens in {:.1} s ({:.2} t/s)", tokens.len(), prompt_s, tokens.len() as f32 / prompt_s);
    println!("generated: {} tokens in {:.1} s ({:.2} t/s)", out.len() - 1, gen_s, (out.len() - 1) as f32 / gen_s);
    Ok(())
}
