//! serverfs: a directory on a home server as a Redox scheme, read-only
//! (spike for redox-rpi#713, design in docs/home-server-storage.md).
//!
//!     serverfs [-d] [--auth <file>] <url> <scheme>
//!     serverfs http://192.0.2.10:8080/files/ server &
//!     ls /scheme/server
//!
//! It speaks WebDAV over HTTP/1.1: `PROPFIND` (Depth 0 and 1) for stat and
//! directory listings, `GET` for file contents. Nothing else of the design
//! note's subset yet: no writes, no cache on disk, no TLS. A listing is kept
//! for a few seconds; a file is fetched whole at its first read and kept
//! while it is open. `--auth` names a file with one line `user:password`
//! for HTTP Basic authentication (only over a trusted network until TLS).
//! With `-d` serverfs goes to the background once the scheme is registered
//! (as Redox drivers do), so the next command can use it at once; without
//! it serverfs stays in the foreground.

use std::collections::{BTreeMap, HashMap};
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::process;
use std::sync::Arc;
use std::time::{Duration, Instant};

use redox_scheme::scheme::{register_sync_scheme, SchemeState, SchemeSync};
use redox_scheme::{CallerCtx, OpenResult, RequestKind, Response, SignalBehavior, Socket};
use syscall::data::Stat;
use syscall::dirent::{DirEntry, DirentBuf, DirentKind};
use syscall::error::{
    Error, Result, EACCES, EBADF, EFBIG, EHOSTUNREACH, EINVAL, EIO, EISDIR, ENOENT, ENOTDIR, EROFS,
};
use syscall::flag::{MODE_DIR, MODE_FILE, O_CREAT, O_DIRECTORY, O_STAT, O_TRUNC, O_WRONLY};
use syscall::schemev2::NewFdFlags;

/// How long a directory listing or a stat result is used without asking
/// the server again.
const FRESH: Duration = Duration::from_secs(5);
/// Files are held in memory while open; bigger ones are refused until
/// `GET` with `Range` exists.
const MAX_FILE: u64 = 128 << 20;
const TIMEOUT: Duration = Duration::from_secs(15);

// ---------------------------------------------------------------- HTTP

struct Server {
    host: String,
    port: u16,
    /// Path of the shared directory on the server, starting and ending with '/'.
    base: String,
    /// `Authorization` header value.
    auth: Option<String>,
}

struct HttpResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpResponse {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }
}

fn parse_url(url: &str) -> std::result::Result<(String, u16, String), String> {
    let rest = if let Some(rest) = url.strip_prefix("http://") {
        rest
    } else if url.starts_with("https://") {
        return Err("https comes with the TLS step of #713; use http:// on a trusted network".into());
    } else {
        return Err("the URL must start with http://".into());
    };
    let (hostport, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) if !h.ends_with(']') || hostport.starts_with('[') => {
            (h.trim_matches(|c| c == '[' || c == ']').to_string(), p.parse().map_err(|_| "bad port")?)
        }
        _ => (hostport.to_string(), 80),
    };
    if host.is_empty() {
        return Err("no host in the URL".into());
    }
    let mut base = path.to_string();
    if !base.ends_with('/') {
        base.push('/');
    }
    Ok((host, port, base))
}

/// Percent-encodes a path for the request line; '/' stays.
fn encode_path(path: &str) -> String {
    let mut out = String::new();
    for &b in path.as_bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/!$&'()*+,;=:@".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn decode_percent(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(v) = u8::from_str_radix(hex, 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn base64(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn dechunk(mut body: &[u8]) -> io::Result<Vec<u8>> {
    let bad = || io::Error::new(io::ErrorKind::InvalidData, "bad chunked body");
    let mut out = Vec::new();
    loop {
        let line_end = body.windows(2).position(|w| w == b"\r\n").ok_or_else(bad)?;
        let line = std::str::from_utf8(&body[..line_end]).map_err(|_| bad())?;
        let size = usize::from_str_radix(line.split(';').next().unwrap_or("").trim(), 16).map_err(|_| bad())?;
        body = &body[line_end + 2..];
        if size == 0 {
            return Ok(out);
        }
        if body.len() < size + 2 {
            return Err(bad());
        }
        out.extend_from_slice(&body[..size]);
        body = &body[size + 2..];
    }
}

impl Server {
    /// One request on its own connection (`Connection: close`).
    fn request(&self, method: &str, path: &str, headers: &[(&str, &str)], body: &[u8]) -> io::Result<HttpResponse> {
        let addr = (self.host.as_str(), self.port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no address for the host"))?;
        let mut stream = TcpStream::connect_timeout(&addr, TIMEOUT)?;
        let _ = stream.set_read_timeout(Some(TIMEOUT));
        let _ = stream.set_write_timeout(Some(TIMEOUT));

        let mut req = format!(
            "{method} {} HTTP/1.1\r\nHost: {}:{}\r\nUser-Agent: serverfs/0.1 (Redox)\r\nConnection: close\r\nContent-Length: {}\r\n",
            encode_path(path),
            self.host,
            self.port,
            body.len()
        );
        if let Some(auth) = &self.auth {
            req.push_str(&format!("Authorization: {auth}\r\n"));
        }
        for (name, value) in headers {
            req.push_str(&format!("{name}: {value}\r\n"));
        }
        req.push_str("\r\n");
        stream.write_all(req.as_bytes())?;
        stream.write_all(body)?;

        let mut raw = Vec::new();
        stream.read_to_end(&mut raw)?;
        let bad = |what: &str| io::Error::new(io::ErrorKind::InvalidData, format!("bad HTTP response: {what}"));
        let head_end = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or_else(|| bad("no header end"))?;
        let head = std::str::from_utf8(&raw[..head_end]).map_err(|_| bad("header not UTF-8"))?;
        let mut lines = head.split("\r\n");
        let status_line = lines.next().unwrap_or("");
        let status = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| bad("status line"))?;
        let headers: Vec<(String, String)> = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(n, v)| (n.trim().to_ascii_lowercase(), v.trim().to_string()))
            .collect();
        let mut response = HttpResponse { status, headers, body: raw[head_end + 4..].to_vec() };
        if response.header("transfer-encoding").is_some_and(|v| v.eq_ignore_ascii_case("chunked")) {
            response.body = dechunk(&response.body)?;
        } else if let Some(len) = response.header("content-length").and_then(|v| v.parse::<usize>().ok()) {
            if response.body.len() < len {
                return Err(bad("body shorter than Content-Length"));
            }
            response.body.truncate(len);
        }
        Ok(response)
    }

    fn url_path(&self, path: &str, dir: bool) -> String {
        let mut p = format!("{}{}", self.base, path);
        if dir && !p.ends_with('/') {
            p.push('/');
        }
        p
    }

    /// `PROPFIND` on `path`: the entries, the first one being `path` itself
    /// for Depth 0.
    fn propfind(&self, path: &str, depth: u8, dir_hint: bool) -> Result<Vec<Entry>> {
        const BODY: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\
<D:propfind xmlns:D=\"DAV:\"><D:prop><D:resourcetype/><D:getcontentlength/>\
<D:getlastmodified/><D:getetag/></D:prop></D:propfind>";
        let depth = depth.to_string();
        let headers = [("Depth", depth.as_str()), ("Content-Type", "application/xml; charset=utf-8")];
        let mut target = self.url_path(path, dir_hint);
        let mut res = self.request("PROPFIND", &target, &headers, BODY.as_bytes()).map_err(net_err)?;
        // A directory asked for without its trailing '/'.
        if matches!(res.status, 301 | 302 | 307 | 308) && !target.ends_with('/') {
            target.push('/');
            res = self.request("PROPFIND", &target, &headers, BODY.as_bytes()).map_err(net_err)?;
        }
        check_status(res.status, "PROPFIND", &target)?;
        if res.status != 207 {
            eprintln!("serverfs: PROPFIND {target}: status {} instead of 207", res.status);
            return Err(Error::new(EIO));
        }
        let xml = String::from_utf8_lossy(&res.body);
        Ok(parse_multistatus(&xml)
            .into_iter()
            .filter_map(|mut e| {
                e.path = self.rel_path(&e.path)?;
                Some(e)
            })
            .collect())
    }

    /// The path below `base` of an `href` from a multistatus answer.
    fn rel_path(&self, href: &str) -> Option<String> {
        let mut p = href;
        if let Some(rest) = p.strip_prefix("http://").or_else(|| p.strip_prefix("https://")) {
            p = &rest[rest.find('/').unwrap_or(rest.len())..];
        }
        let p = decode_percent(p);
        let base = self.base.trim_end_matches('/');
        let rest = p.strip_prefix(base)?;
        if !rest.is_empty() && !rest.starts_with('/') {
            return None;
        }
        Some(rest.trim_matches('/').to_string())
    }

    fn get(&self, path: &str) -> Result<Vec<u8>> {
        let target = self.url_path(path, false);
        let res = self.request("GET", &target, &[], &[]).map_err(net_err)?;
        check_status(res.status, "GET", &target)?;
        if res.status != 200 {
            eprintln!("serverfs: GET {target}: status {}", res.status);
            return Err(Error::new(EIO));
        }
        Ok(res.body)
    }
}

fn net_err(err: io::Error) -> Error {
    eprintln!("serverfs: server not reachable: {err}");
    Error::new(match err.kind() {
        io::ErrorKind::InvalidData => EIO,
        _ => EHOSTUNREACH,
    })
}

fn check_status(status: u16, method: &str, target: &str) -> Result<()> {
    match status {
        200..=299 => Ok(()),
        404 | 410 => Err(Error::new(ENOENT)),
        401 | 403 => {
            eprintln!("serverfs: {method} {target}: status {status} (login refused)");
            Err(Error::new(EACCES))
        }
        _ => {
            eprintln!("serverfs: {method} {target}: status {status}");
            Err(Error::new(EIO))
        }
    }
}

// ---------------------------------------------------------------- WebDAV XML

#[derive(Clone, Debug, Default)]
struct Entry {
    /// From the `href`; after `rel_path`, the path below the share root.
    path: String,
    is_dir: bool,
    size: u64,
    mtime: u64,
    etag: String,
}

fn decode_entities(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';') else { break };
        let ent = &rest[1..end];
        let ch = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if ent.starts_with("#x") => u32::from_str_radix(&ent[2..], 16).ok().and_then(char::from_u32),
            _ if ent.starts_with('#') => ent[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The `response` elements of a WebDAV multistatus body. A small tolerant
/// reader, not a full XML parser: it matches local names and ignores
/// namespace prefixes, which is enough for the four properties asked for.
fn parse_multistatus(xml: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut cur: Option<Entry> = None;
    let mut in_resourcetype = false;
    let mut text = String::new();
    let mut rest = xml;
    while let Some(lt) = rest.find('<') {
        text.push_str(&rest[..lt]);
        rest = &rest[lt + 1..];
        let Some(gt) = rest.find('>') else { break };
        let tag = &rest[..gt];
        rest = &rest[gt + 1..];
        if tag.starts_with('?') || tag.starts_with('!') {
            continue;
        }
        let closing = tag.starts_with('/');
        let self_closing = tag.ends_with('/');
        let name = tag.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");
        let local = name.rsplit(':').next().unwrap_or(name);
        if !closing {
            match local {
                "response" => cur = Some(Entry::default()),
                "resourcetype" if !self_closing => in_resourcetype = true,
                "collection" if in_resourcetype => {
                    if let Some(e) = cur.as_mut() {
                        e.is_dir = true;
                    }
                }
                _ => {}
            }
            text.clear();
            continue;
        }
        let value = decode_entities(text.trim());
        text.clear();
        let Some(e) = cur.as_mut() else { continue };
        match local {
            "href" if e.path.is_empty() => e.path = value,
            "getcontentlength" => e.size = value.parse().unwrap_or(0),
            "getlastmodified" => e.mtime = parse_http_date(&value).unwrap_or(0),
            "getetag" => e.etag = value,
            "resourcetype" => in_resourcetype = false,
            "response" => entries.extend(cur.take()),
            _ => {}
        }
    }
    entries
}

/// "Tue, 24 Nov 2026 17:21:34 GMT" → seconds since 1970.
fn parse_http_date(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }
    let day: i64 = parts[1].parse().ok()?;
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let month = MONTHS.iter().position(|m| *m == parts[2])? as i64 + 1;
    let year: i64 = parts[3].parse().ok()?;
    let hms: Vec<i64> = parts[4].split(':').filter_map(|p| p.parse().ok()).collect();
    if hms.len() != 3 {
        return None;
    }
    // Days from civil (Howard Hinnant's algorithm).
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    u64::try_from(days * 86400 + hms[0] * 3600 + hms[1] * 60 + hms[2]).ok()
}

// ---------------------------------------------------------------- scheme

enum Handle {
    Dir(String),
    /// The content is fetched at the first read and kept while open.
    File { path: String, data: Option<Arc<Vec<u8>>> },
}

struct ServerScheme {
    server: Server,
    name: String,
    handles: BTreeMap<usize, Handle>,
    next_id: usize,
    /// path → (when, entry), from PROPFIND answers.
    stats: HashMap<String, (Instant, Entry)>,
    /// directory path → (when, children).
    listings: HashMap<String, (Instant, Vec<Entry>)>,
}

/// Joins `path` to the directory `base` and removes ".", ".." and empty parts.
fn join(base: &str, path: &str) -> String {
    let mut parts: Vec<&str> =
        if path.starts_with('/') { Vec::new() } else { base.split('/').filter(|p| !p.is_empty()).collect() };
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

impl ServerScheme {
    fn insert(&mut self, handle: Handle) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.handles.insert(id, handle);
        id
    }

    /// Lists `dir` with `PROPFIND Depth: 1` unless a fresh listing is kept.
    fn list(&mut self, dir: &str) -> Result<Vec<Entry>> {
        if let Some((when, children)) = self.listings.get(dir) {
            if when.elapsed() < FRESH {
                return Ok(children.clone());
            }
        }
        let now = Instant::now();
        let mut found_self = false;
        let mut children = Vec::new();
        for e in self.server.propfind(dir, 1, true)? {
            if e.path == dir {
                found_self = true;
                if !e.is_dir {
                    return Err(Error::new(ENOTDIR));
                }
                self.stats.insert(e.path.clone(), (now, e));
            } else {
                self.stats.insert(e.path.clone(), (now, e.clone()));
                children.push(e);
            }
        }
        if !found_self && !dir.is_empty() {
            eprintln!("serverfs: PROPFIND {dir:?}: the answer does not list the directory itself");
        }
        self.listings.insert(dir.to_string(), (now, children.clone()));
        Ok(children)
    }

    /// The entry for `path`: from a fresh listing or stat, else `PROPFIND Depth: 0`.
    fn lookup(&mut self, path: &str) -> Result<Entry> {
        if path.is_empty() {
            return Ok(Entry { is_dir: true, ..Default::default() });
        }
        if let Some((when, e)) = self.stats.get(path) {
            if when.elapsed() < FRESH {
                return Ok(e.clone());
            }
        }
        let parent = path.rsplit_once('/').map_or("", |(p, _)| p);
        if let Some((when, children)) = self.listings.get(parent) {
            if when.elapsed() < FRESH {
                return children.iter().find(|e| e.path == path).cloned().ok_or(Error::new(ENOENT));
            }
        }
        let entry = self
            .server
            .propfind(path, 0, false)?
            .into_iter()
            .find(|e| e.path == path)
            .ok_or(Error::new(ENOENT))?;
        self.stats.insert(path.to_string(), (Instant::now(), entry.clone()));
        Ok(entry)
    }

    fn path(&self, id: usize) -> Result<&str> {
        match self.handles.get(&id).ok_or(Error::new(EBADF))? {
            Handle::Dir(path) | Handle::File { path, .. } => Ok(path),
        }
    }

    fn data(&mut self, id: usize) -> Result<Arc<Vec<u8>>> {
        let path = match self.handles.get(&id).ok_or(Error::new(EBADF))? {
            Handle::Dir(_) => return Err(Error::new(EISDIR)),
            Handle::File { data: Some(data), .. } => return Ok(data.clone()),
            Handle::File { path, .. } => path.clone(),
        };
        let entry = self.lookup(&path)?;
        if entry.size > MAX_FILE {
            eprintln!("serverfs: {path}: {} bytes, more than this spike holds in memory", entry.size);
            return Err(Error::new(EFBIG));
        }
        let data = Arc::new(self.server.get(&path)?);
        if let Some(Handle::File { data: slot, .. }) = self.handles.get_mut(&id) {
            *slot = Some(data.clone());
        }
        Ok(data)
    }
}

impl SchemeSync for ServerScheme {
    fn scheme_root(&mut self) -> Result<usize> {
        Ok(self.insert(Handle::Dir(String::new())))
    }

    fn openat(&mut self, dirfd: usize, path: &str, flags: usize, _fcntl_flags: u32, _ctx: &CallerCtx) -> Result<OpenResult> {
        let path = join(self.path(dirfd)?, path);
        // On Redox O_RDONLY is a bit of its own and O_RDWR = O_RDONLY | O_WRONLY;
        // stat and directory opens come without any access bit.
        if flags & (O_WRONLY | O_CREAT | O_TRUNC) != 0 {
            return Err(Error::new(EROFS));
        }
        let entry = self.lookup(&path)?;
        let handle = if entry.is_dir {
            Handle::Dir(path)
        } else if flags & O_DIRECTORY != 0 && flags & O_STAT == 0 {
            return Err(Error::new(ENOTDIR));
        } else {
            Handle::File { path, data: None }
        };
        Ok(OpenResult::ThisScheme { number: self.insert(handle), flags: NewFdFlags::POSITIONED })
    }

    fn read(&mut self, id: usize, buf: &mut [u8], offset: u64, _fcntl_flags: u32, _ctx: &CallerCtx) -> Result<usize> {
        let data = self.data(id)?;
        let start = usize::try_from(offset).map_err(|_| Error::new(EINVAL))?.min(data.len());
        let n = buf.len().min(data.len() - start);
        buf[..n].copy_from_slice(&data[start..start + n]);
        Ok(n)
    }

    fn write(&mut self, _id: usize, _buf: &[u8], _offset: u64, _fcntl_flags: u32, _ctx: &CallerCtx) -> Result<usize> {
        Err(Error::new(EROFS))
    }

    fn fsize(&mut self, id: usize, _ctx: &CallerCtx) -> Result<u64> {
        if let Some(Handle::File { data: Some(data), .. }) = self.handles.get(&id) {
            return Ok(data.len() as u64);
        }
        let path = self.path(id)?.to_string();
        Ok(self.lookup(&path)?.size)
    }

    fn fstat(&mut self, id: usize, stat: &mut Stat, _ctx: &CallerCtx) -> Result<()> {
        let path = self.path(id)?.to_string();
        let e = self.lookup(&path)?;
        let size = match self.handles.get(&id) {
            Some(Handle::File { data: Some(data), .. }) => data.len() as u64,
            _ => e.size,
        };
        *stat = Stat {
            st_mode: if e.is_dir { MODE_DIR | 0o555 } else { MODE_FILE | 0o444 },
            st_nlink: 1,
            st_size: size,
            st_blksize: 4096,
            st_blocks: size.div_ceil(512),
            st_mtime: e.mtime,
            st_atime: e.mtime,
            st_ctime: e.mtime,
            ..Default::default()
        };
        Ok(())
    }

    fn fpath(&mut self, id: usize, buf: &mut [u8], _ctx: &CallerCtx) -> Result<usize> {
        let full = format!("/scheme/{}/{}", self.name, self.path(id)?);
        let n = full.len().min(buf.len());
        buf[..n].copy_from_slice(&full.as_bytes()[..n]);
        Ok(n)
    }

    fn fsync(&mut self, id: usize, _ctx: &CallerCtx) -> Result<()> {
        self.path(id).map(|_| ())
    }

    fn getdents<'buf>(&mut self, id: usize, mut buf: DirentBuf<&'buf mut [u8]>, opaque_offset: u64) -> Result<DirentBuf<&'buf mut [u8]>> {
        let path = match self.handles.get(&id).ok_or(Error::new(EBADF))? {
            Handle::Dir(path) => path.clone(),
            Handle::File { .. } => return Err(Error::new(ENOTDIR)),
        };
        let children = self.list(&path)?;
        for (idx, e) in children.iter().enumerate().skip(opaque_offset as usize) {
            let name = e.path.rsplit('/').next().unwrap_or(&e.path);
            let result = buf.entry(DirEntry {
                inode: idx as u64 + 1,
                next_opaque_id: idx as u64 + 1,
                name,
                kind: if e.is_dir { DirentKind::Directory } else { DirentKind::Regular },
            });
            if let Err(err) = result {
                // A full buffer ends the call; the caller asks again.
                if err.errno == EINVAL && idx > opaque_offset as usize {
                    break;
                }
                return Err(err);
            }
        }
        Ok(buf)
    }

    fn on_close(&mut self, id: usize) {
        self.handles.remove(&id);
    }
}

/// `-d`: forks; the parent waits until the child has registered the scheme
/// (one byte through a pipe), then exits 0, or 1 if the child failed. The
/// child gets the write end.
fn daemonize() -> Option<libc::c_int> {
    let mut fds = [0 as libc::c_int; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        eprintln!("serverfs: pipe: {}", io::Error::last_os_error());
        process::exit(1);
    }
    match unsafe { libc::fork() } {
        -1 => {
            eprintln!("serverfs: fork: {}", io::Error::last_os_error());
            process::exit(1);
        }
        0 => {
            unsafe { libc::close(fds[0]) };
            Some(fds[1])
        }
        _ => {
            unsafe { libc::close(fds[1]) };
            let mut b = 0u8;
            let n = unsafe { libc::read(fds[0], &mut b as *mut u8 as *mut libc::c_void, 1) };
            process::exit(if n == 1 { 0 } else { 1 });
        }
    }
}

fn run(server: Server, name: &str, ready: Option<libc::c_int>) -> io::Result<()> {
    let socket = Socket::create()?;
    let label = format!("http://{}:{}{}", server.host, server.port, server.base);
    let mut scheme = ServerScheme {
        server,
        name: name.to_string(),
        handles: BTreeMap::new(),
        next_id: 1,
        stats: HashMap::new(),
        listings: HashMap::new(),
    };
    // A first listing, so a wrong URL or login shows at the start.
    match scheme.list("") {
        Ok(children) => eprintln!("serverfs: {label}: {} entries at the top", children.len()),
        Err(err) => eprintln!("serverfs: {label}: first listing failed ({err}); serving anyway"),
    }
    let mut state = SchemeState::new();
    register_sync_scheme(&socket, name, &mut scheme).map_err(io::Error::from)?;
    eprintln!("serverfs: serving /scheme/{name} (read-only)");
    if let Some(fd) = ready {
        unsafe {
            libc::write(fd, b"r".as_ptr() as *const libc::c_void, 1);
            libc::close(fd);
        }
    }

    loop {
        let req = match socket.next_request(SignalBehavior::Restart)? {
            None => return Ok(()),
            Some(req) => match req.kind() {
                RequestKind::Call(req) => req,
                RequestKind::OnClose { id } => {
                    scheme.on_close(id);
                    state.on_close(id);
                    continue;
                }
                _ => continue,
            },
        };
        let response: Response = req.handle_sync(&mut scheme, &mut state);
        if !socket.write_response(response, SignalBehavior::Restart)? {
            return Ok(());
        }
    }
}

fn usage() -> ! {
    eprintln!("usage: serverfs [-d] [--auth <file>] <url> <scheme>   e.g. serverfs -d http://192.0.2.10:8080/files/ server");
    process::exit(1);
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut auth = None;
    let mut background = false;
    if args.first().map(String::as_str) == Some("-d") {
        background = true;
        args.remove(0);
    }
    if args.first().map(String::as_str) == Some("--auth") {
        if args.len() < 2 {
            usage();
        }
        let file = args.remove(1);
        args.remove(0);
        match std::fs::read_to_string(&file) {
            Ok(s) => auth = Some(format!("Basic {}", base64(s.trim_end_matches(['\r', '\n']).as_bytes()))),
            Err(err) => {
                eprintln!("serverfs: {file}: {err}");
                process::exit(1);
            }
        }
    }
    if args.len() != 2 {
        usage();
    }
    let (host, port, base) = match parse_url(&args[0]) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("serverfs: {}: {err}", args[0]);
            process::exit(1);
        }
    };
    let ready = if background { daemonize() } else { None };
    if let Err(err) = run(Server { host, port, base, auth }, &args[1], ready) {
        eprintln!("serverfs: {err}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multistatus() {
        let xml = r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:"><d:response><d:href>/files/</d:href><d:propstat><d:prop>
<d:resourcetype><d:collection/></d:resourcetype><d:getlastmodified>Tue, 24 Nov 2026 17:21:34 GMT</d:getlastmodified>
</d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>
<d:response><d:href>/files/a%20b%C3%A4.txt</d:href><d:propstat><d:prop><d:resourcetype/>
<d:getcontentlength>12</d:getcontentlength><d:getetag>"x&amp;y"</d:getetag></d:prop></d:propstat></d:response>
</d:multistatus>"#;
        let s = Server { host: "h".into(), port: 80, base: "/files/".into(), auth: None };
        let e: Vec<Entry> = parse_multistatus(xml)
            .into_iter()
            .map(|mut e| {
                e.path = s.rel_path(&e.path).unwrap();
                e
            })
            .collect();
        assert_eq!(e.len(), 2);
        assert!(e[0].is_dir && e[0].path.is_empty());
        assert_eq!(e[0].mtime, 1795540894);
        assert_eq!(e[1].path, "a bä.txt");
        assert_eq!(e[1].size, 12);
        assert_eq!(e[1].etag, "\"x&y\"");
        assert!(!e[1].is_dir);
    }

    #[test]
    fn url_and_base64() {
        assert_eq!(parse_url("http://10.0.2.2:8080/f").unwrap(), ("10.0.2.2".into(), 8080, "/f/".into()));
        assert_eq!(parse_url("http://nas").unwrap(), ("nas".into(), 80, "/".into()));
        assert!(parse_url("https://nas/").is_err());
        assert_eq!(base64(b"user:pass"), "dXNlcjpwYXNz");
        assert_eq!(encode_path("/a b/ä"), "/a%20b/%C3%A4");
        assert_eq!(decode_percent("/a%20b/%C3%A4"), "/a b/ä");
    }
}
