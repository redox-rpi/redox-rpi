//! Minimal Qwen3 GGUF inference with candle, for testing on Redox.
//! Usage: rinfer <model.gguf> <prompt> [tokens]
//!        rinfer <model.gguf> --serve     (for the assistant, see serve.rs)
//!        rinfer --download <http://host[:port]/path> <file>
//!            fetch the model over plain HTTP, for test images without it
mod serve;

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

fn heap_mib() -> usize {
    mib(IN_USE.load(Ordering::Relaxed))
}

fn peak_mib() -> usize {
    mib(PEAK.load(Ordering::Relaxed))
}

/// Reads the model file for candle's GGUF loader.
///
/// - The GGUF header holds the tokenizer, about 300,000 strings, read with
///   760,000 small reads and a `stream_position` per string. Unbuffered,
///   each was a request to the file system: on the Pi loading ran at
///   0.2 MB/s. This reader buffers 1 MiB and knows its position, so neither
///   reaches the file.
/// - Reads are at most 1 MiB: candle reads each tensor with one
///   `read_exact`, up to 127 MB for the token table, and RedoxFS then
///   allocated a buffer of the same size, which ran the 1 GiB Pi (QEMU) out
///   of memory.
struct ModelReader {
    inner: std::io::BufReader<CountingFile>,
    pos: u64,
}

impl ModelReader {
    fn open(path: &str) -> std::io::Result<Self> {
        let f = CountingFile(std::fs::File::open(path)?);
        Ok(ModelReader { inner: std::io::BufReader::with_capacity(1 << 20, f), pos: 0 })
    }
}

impl Read for ModelReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = buf.len().min(1 << 20);
        let n = self.inner.read(&mut buf[..n])?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for ModelReader {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let target = match to {
            SeekFrom::Start(p) => p as i64,
            SeekFrom::Current(d) => self.pos as i64 + d,
            SeekFrom::End(_) => {
                self.pos = self.inner.seek(to)?;
                return Ok(self.pos);
            }
        };
        // Keeps the buffer when the target is inside it.
        self.inner.seek_relative(target - self.pos as i64)?;
        self.pos = target as u64;
        Ok(self.pos)
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(self.pos)
    }
}

/// Counts the reads that reach the file.
struct CountingFile(std::fs::File);

impl Read for CountingFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        FILE_READS.fetch_add(1, Ordering::Relaxed);
        self.0.read(buf)
    }
}

impl Seek for CountingFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.0.seek(pos)
    }
}

static FILE_READS: AtomicUsize = AtomicUsize::new(0);

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 4 && args[1] == "--download" {
        return download(&args[2], &args[3]);
    }
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
    // start small; the cache grows when a reply needs more.
    if std::env::var_os("CANDLE_KV_CACHE_LEN").is_none() {
        std::env::set_var("CANDLE_KV_CACHE_LEN", "256");
    }
    if std::env::var_os("RINFER_TRACE").is_some() {
        TRACE.store(1, Ordering::Relaxed);
    }
    eprintln!("neon: {}, threads: {}", candle::utils::with_neon(), candle::utils::get_num_threads());

    let t0 = Instant::now();
    let mut file = ModelReader::open(path)?;
    let mut content = gguf_file::Content::read(&mut file)?;
    let tokenizer = Tokenizer::from_gguf(&content)?;
    // The tokenizer's 300,000 strings (27 MiB) are no longer needed; free
    // them before the weights take the memory.
    content.metadata.retain(|k, _| !k.starts_with("tokenizer."));
    let device = Device::Cpu;
    let mut model = ModelWeights::from_gguf(content, &mut file, &device)?;
    eprintln!(
        "loaded in {:.1} s ({} reads of the file), heap {} MiB (peak {} MiB)",
        t0.elapsed().as_secs_f32(),
        FILE_READS.load(Ordering::Relaxed),
        mib(IN_USE.load(Ordering::Relaxed)),
        mib(PEAK.load(Ordering::Relaxed))
    );
    if prompt == "--serve" {
        return serve::serve(&mut model, &tokenizer);
    }

    let text = format!("<|im_start|>user\n{prompt} /no_think<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n");
    let tokens = tokenizer.encode(text, true).map_err(anyhow::Error::msg)?;
    let tokens = tokens.get_ids().to_vec();
    let mut lp = LogitsProcessor::from_sampling(1, Sampling::ArgMax);
    let eos = tokenizer.token_to_id("<|im_end|>");

    let t1 = Instant::now();
    // RINFER_STEP: feed the prompt one token at a time and print each
    // step's time, to see progress where a step is slow (QEMU).
    let logits = if step_mode() {
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

/// RINFER_STEP: run the prompt one token at a time.
fn step_mode() -> bool {
    std::env::var_os("RINFER_STEP").is_some()
}

/// HTTP/1.0 GET of `url` into `file`, in 1 MiB pieces, with the length the
/// server gives checked. No TLS: meant for the build host on the home
/// network. `NNN` in the URL stands for a part number (000, 001, ...): the
/// parts are fetched one by one, each on its own connection, until the server
/// answers 404, and joined, so no single connection runs long. (In QEMU one
/// long download stopped after 64 MiB without an error; the disk was close to
/// full, and a full disk can hang writes on Redox.) With parts, a file left by
/// an earlier try is kept up to its last whole part and the rest is fetched:
/// the Pi downloads at 0.06 to 0.25 MB/s, so a stopped try is not lost. A
/// part whose connection breaks is fetched again, up to 10 times in a row.
fn download(url: &str, file: &str) -> anyhow::Result<()> {
    let t = Instant::now();
    let mut got = 0u64;
    let mut out;
    if url.contains("NNN") {
        // Whole parts already in the file, by the lengths the server gives.
        let have = std::fs::metadata(file).map(|m| m.len()).unwrap_or(0);
        let mut first = 0;
        while have > 0 {
            let u = url.replace("NNN", &format!("{first:03}"));
            match head(&u)? {
                Some(l) if got + l <= have => {
                    got += l;
                    first += 1;
                }
                _ => break,
            }
        }
        out = std::fs::OpenOptions::new().create(true).write(true).open(file)?;
        out.set_len(got)?;
        out.seek(SeekFrom::End(0))?;
        if first > 0 {
            eprintln!("kept parts 000 to {:03} from an earlier try, {} MiB", first - 1, got >> 20);
        }
        let mut part = first;
        let mut tries = 0;
        loop {
            let u = url.replace("NNN", &format!("{part:03}"));
            let (start, t_part) = (got, Instant::now());
            match fetch(&u, &mut out, &mut got) {
                Ok(true) => {
                    let mb = (got - start) as f32 / 1e6;
                    eprintln!("part {part:03}, {} MiB, {:.2} MB/s", got >> 20, mb / t_part.elapsed().as_secs_f32());
                    part += 1;
                    tries = 0;
                }
                Ok(false) if part > 0 => break,
                Ok(false) => anyhow::bail!("{u}: not found"),
                // A dropped connection (the Pi saw ENOTCONN after 204 MB):
                // cut the broken part off and fetch it again.
                Err(e) if tries < 10 => {
                    tries += 1;
                    eprintln!("part {part:03}: {e}; again in 5 s (try {tries} of 10)");
                    got = start;
                    out.set_len(got)?;
                    out.seek(SeekFrom::End(0))?;
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
                Err(e) => return Err(e),
            }
        }
    } else {
        out = std::fs::File::create(file)?;
        if !fetch(url, &mut out, &mut got)? {
            anyhow::bail!("{url}: not found");
        }
    }
    out.sync_all()?;
    println!("{file}: {got} bytes in {:.0} s", t.elapsed().as_secs_f32());
    Ok(())
}

/// Opens a connection for `method` on `url` and reads the status line and
/// headers: the reader at the body, the status code and the Content-Length.
fn request(method: &str, url: &str) -> anyhow::Result<(std::io::BufReader<std::net::TcpStream>, String, Option<u64>)> {
    use std::io::BufRead;
    let rest = url.strip_prefix("http://").ok_or_else(|| anyhow::anyhow!("only http:// URLs"))?;
    let (host, path) = rest.split_once('/').map(|(h, p)| (h, format!("/{p}"))).unwrap_or((rest, "/".into()));
    let addr = if host.contains(':') { host.to_string() } else { format!("{host}:80") };
    let stream = std::net::TcpStream::connect(&addr)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(60)))?;
    let mut w = stream.try_clone()?;
    write!(w, "{method} {path} HTTP/1.0\r\nHost: {host}\r\n\r\n")?;
    let mut r = std::io::BufReader::with_capacity(1 << 20, stream);
    let mut line = String::new();
    r.read_line(&mut line)?;
    let status = line.split_whitespace().nth(1).unwrap_or("").to_string();
    if status != "200" && status != "404" {
        anyhow::bail!("server answered: {}", line.trim());
    }
    let mut len = None;
    loop {
        line.clear();
        r.read_line(&mut line)?;
        let l = line.trim();
        if l.is_empty() {
            break;
        }
        if let Some((k, v)) = l.split_once(':') {
            if k.eq_ignore_ascii_case("content-length") {
                len = v.trim().parse::<u64>().ok();
            }
        }
    }
    Ok((r, status, len))
}

/// The length of `url` from a HEAD request; None if the server answers 404.
fn head(url: &str) -> anyhow::Result<Option<u64>> {
    let (_, status, len) = request("HEAD", url)?;
    if status == "404" {
        return Ok(None);
    }
    len.map(Some).ok_or_else(|| anyhow::anyhow!("{url}: no length"))
}

/// One GET, appended to `out`; false if the server answers 404.
fn fetch(url: &str, out: &mut std::fs::File, got: &mut u64) -> anyhow::Result<bool> {
    let (mut r, status, len) = request("GET", url)?;
    if status == "404" {
        return Ok(false);
    }
    let mut buf = vec![0u8; 1 << 20];
    let start = *got;
    loop {
        let n = r.read(&mut buf).map_err(|e| anyhow::anyhow!("after {got} bytes: {e}"))?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        *got += n as u64;
    }
    if let Some(l) = len.filter(|&l| l != *got - start) {
        anyhow::bail!("{url}: got {} bytes, the server said {l}", *got - start);
    }
    Ok(true)
}
