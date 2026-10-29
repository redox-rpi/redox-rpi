//! Minimal Qwen3 GGUF inference with candle, for testing on Redox.
//! Usage: rinfer <model.gguf> <prompt> [tokens]
use std::alloc::{GlobalAlloc, Layout, System};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use candle::quantized::gguf_file;
use candle::quantized::tokenizer::TokenizerFromGguf;
use candle::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_qwen3::ModelWeights;
use tokenizers::Tokenizer;

/// Counts heap bytes in use and the peak, and with RINFER_TRACE set prints
/// every allocation of 16 MiB or more.
struct Counting;

static IN_USE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() {
            note_alloc(layout.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(layout);
        if !p.is_null() {
            note_alloc(layout.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        System.dealloc(p, layout);
        IN_USE.fetch_sub(layout.size(), Ordering::Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let q = System.realloc(p, layout, new_size);
        if !q.is_null() {
            IN_USE.fetch_sub(layout.size(), Ordering::Relaxed);
            note_alloc(new_size);
        }
        q
    }
}

fn note_alloc(size: usize) {
    let now = IN_USE.fetch_add(size, Ordering::Relaxed) + size;
    PEAK.fetch_max(now, Ordering::Relaxed);
    if size >= 16 << 20 && TRACE.load(Ordering::Relaxed) == 1 {
        // eprintln would allocate; write a fixed buffer instead.
        let mut buf = [0u8; 64];
        let mut w = &mut buf[..];
        let _ = write!(w, "alloc {} MiB, in use {} MiB\n", size >> 20, now >> 20);
        let len = 64 - w.len();
        let _ = std::io::stderr().write_all(&buf[..len]);
    }
}

static TRACE: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOC: Counting = Counting;

fn mib(n: usize) -> usize {
    n >> 20
}

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
    // candle preallocates a 4096-position key/value cache per layer (900 MiB
    // for Qwen3 0.6B). Redox backs it with real memory and has no swap, so
    // start small; the cache doubles when a reply needs more.
    if std::env::var_os("CANDLE_KV_CACHE_LEN").is_none() {
        std::env::set_var("CANDLE_KV_CACHE_LEN", "256");
    }
    if std::env::var_os("RINFER_TRACE").is_some() {
        TRACE.store(1, Ordering::Relaxed);
    }
    println!("neon: {}, threads: {}", candle::utils::with_neon(), candle::utils::get_num_threads());

    let t0 = Instant::now();
    let mut file = ChunkedReader(std::fs::File::open(path)?);
    let content = gguf_file::Content::read(&mut file)?;
    let tokenizer = Tokenizer::from_gguf(&content)?;
    let device = Device::Cpu;
    let mut model = ModelWeights::from_gguf(content, &mut file, &device)?;
    println!(
        "loaded in {:.1} s, heap {} MiB (peak {} MiB)",
        t0.elapsed().as_secs_f32(),
        mib(IN_USE.load(Ordering::Relaxed)),
        mib(PEAK.load(Ordering::Relaxed))
    );

    let text = format!("<|im_start|>user\n{prompt} /no_think<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n");
    let tokens = tokenizer.encode(text, true).map_err(anyhow::Error::msg)?;
    let tokens = tokens.get_ids().to_vec();
    let mut lp = LogitsProcessor::from_sampling(1, Sampling::ArgMax);
    let eos = tokenizer.token_to_id("<|im_end|>");

    let t1 = Instant::now();
    // RINFER_STEP: feed the prompt one token at a time and print each
    // step's time, to see progress where a step is slow (QEMU).
    let logits = if std::env::var_os("RINFER_STEP").is_some() {
        let mut last = None;
        for (pos, &t) in tokens.iter().enumerate() {
            let ts = Instant::now();
            let input = Tensor::new(&[t], &device)?.unsqueeze(0)?;
            last = Some(model.forward(&input, pos)?.squeeze(0)?);
            println!("step {pos}: {:.1} s", ts.elapsed().as_secs_f32());
        }
        last.expect("empty prompt")
    } else {
        let input = Tensor::new(tokens.as_slice(), &device)?.unsqueeze(0)?;
        model.forward(&input, 0)?.squeeze(0)?
    };
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
    println!("heap {} MiB (peak {} MiB)", mib(IN_USE.load(Ordering::Relaxed)), mib(PEAK.load(Ordering::Relaxed)));
    Ok(())
}
