#!/usr/bin/env python3
"""Pre-sort a Redox serial or QEMU log with the local model on homenodellm.

The log is cleaned (ANSI codes, prompt redraws, kernel debug noise), split
into boots, and each boot is reduced to the lines that matter: errors,
panics, faults, daemon messages, the version banner and the last lines.
Regexes find the facts; the model (gpt-oss-20b on vLLM, OpenAI API) reads
the reduced lines in chunks that fit its 16k context and writes a short
structured summary. Chunk summaries are merged by the model as well.

Standard library only. See README.md.
"""

import argparse
import concurrent.futures
import gzip
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request

DEFAULT_URL = os.environ.get("LOG_TRIAGE_URL", "http://127.0.0.1:8000/v1")
DEFAULT_MODEL = os.environ.get("LOG_TRIAGE_MODEL", "gpt-oss-20b")

# A chunk of log lines sent in one request. gpt-oss reads log text at about
# 3 characters per token, so 18000 characters stay near 6k tokens and leave
# room for the instructions, the reasoning and the answer in 16k.
CHUNK_CHARS = 18000
TAIL_LINES = 40
REPEAT_KEEP = 3
# A boot that still needs more chunks: the first and the last ones are sent.
MAX_CHUNKS = 4

ANSI_RE = re.compile(r"\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b\[[0-?]*[ -/]*[@-~]|\x1b[@-Z\\-_]")
CTRL_RE = re.compile(r"[\x00-\x08\x0b-\x1f\x7f]")
# Ion's prompt and getty's login prompt are redrawn on every key press.
PROMPT_RE = re.compile(r"(?:redox login: |[a-z_][\w-]*:[~/][^\s#$]*[#$] )")
FISH_MARK_RE = re.compile(r"⏎\s{20,}")
FDREAD_RE = re.compile(r"kernel::scheme::user:DEBUG -- call_fdread: payload: \d+ metadata: \d+")

BOOT_RE = re.compile(r"^U-Boot 20\d\d\.")
BANNER_RE = re.compile(r"^redox-rpi v\S+.*$")
LOGIN_RE = re.compile(r"redox login:")
WELCOME_RE = re.compile(r"Welcome to Redox OS")

PANIC_RE = re.compile(r"panicked at|kernel panic|\bPANIC\b|RELIBC PANIC", re.I)
FAULT_RE = re.compile(r"FATAL|UNHANDLED EXCEPTION|Not an SVC induced|page fault|data abort|"
                      r"\bOOM\b|out of memory", re.I)
ERROR_RE = re.compile(r"\bERROR\b|\berror\b|\bfailed\b|\bfailure\b|\btimed? ?out\b|"
                      r"\bAbort\b|\bWARN\b|exited with (?!0\b)|\bkilled\b|cannot|unable to|"
                      r"\bhang|not responding|WouldBlock|EFAULT|EIO|ENOMEM", re.I)
DAEMON_RE = re.compile(r"\b[a-z][\w-]*d\b(?::|\[| \()|^\S+d: |init: |restart", re.I)

# Lines that show up in every good netboot and say nothing on their own.
KNOWN_NOISE = [re.compile(p) for p in (
    r"kernel::scheme::user:DEBUG -- call_fdread",
    r"^removing handle: id:",
    r"Cannot persist EFI variables",
    r"Missing RNG device for EFI_RNG_PROTOCOL",
    r"Loading Boot0000 .* failed",
    r"EFI boot manager: Cannot load any image",
    r"Boot failed \(err=-14\)",
    r"No EFI variables loaded",
    r"\[ERROR redoxfs\] failed to open filesystem /scheme/disk\.mmc/0(p0)?: No such file",
    r"Unable to use mmc 0:1",
    r"Slirp: Failed to send packet",
    r"kernel::scheme::irq:DEBUG",
)]
# Register dumps after a fault: one line instead of 31.
REG_RE = re.compile(r"^X\d+:\s+[0-9A-Fa-f]{16}$")
OWNER_RE = re.compile(r"UNHANDLED EXCEPTION.*PID (\d+), NAME ([^,\s]+)")
RESET_CMD_RE = re.compile(r"^(kreset|reboot|shutdown|exit)\b|/scheme/sys/kstop|kreset$")
SHELL_PROMPT_RE = re.compile(r"^[a-z_][\w-]*:[~/][^\s#$]*[#$] (.*)$")


def read_lines(path, line_range=None):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rb") as f:
        raw = f.read().decode("utf-8", "replace").split("\n")
    first = 1
    if line_range:
        a, b = line_range
        raw = raw[a - 1:b]
        first = a
    return [(first + i, l) for i, l in enumerate(raw)]


def clean_line(text):
    text = text.rstrip("\r")
    if "\r" in text:
        parts = [p for p in text.split("\r") if p.strip()]
        text = parts[-1] if parts else ""
    text = ANSI_RE.sub("", text)
    text = FISH_MARK_RE.sub("", text)
    text = CTRL_RE.sub("", text)
    # Kernel debug output lands in the middle of other lines, often the login prompt.
    text = FDREAD_RE.sub("", text)
    prompts = list(PROMPT_RE.finditer(text))
    if len(prompts) > 1:
        text = text[prompts[-1].start():]
    return text.rstrip()


def is_noise(text):
    return any(p.search(text) for p in KNOWN_NOISE)


def clean(lines):
    """Clean lines; collapse runs of the same line. Returns (lineno, text, count)."""
    out = []
    noise = 0
    for n, raw in lines:
        t = clean_line(raw)
        if not t.strip():
            continue
        if is_noise(t):
            noise += 1
            continue
        if REG_RE.match(t):
            if out and out[-1][1].startswith("(registers"):
                out[-1][2] += 1
                continue
            t = "(registers X0-X30)"
        if out and out[-1][1] == t and not t.startswith("(registers"):
            out[-1][2] += 1
            continue
        out.append([n, t, 1])
    return out, noise


def split_boots(lines, boot_re):
    boots, cur = [], []
    for item in lines:
        if boot_re.search(item[1]) and cur:
            boots.append(cur)
            cur = []
        cur.append(item)
    if cur:
        boots.append(cur)
    # Text before the first U-Boot line is the end of an earlier boot.
    return boots


def signature(text):
    t = re.sub(r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b", "U", text)
    t = re.sub(r"0x[0-9a-fA-F]+|\b[0-9a-fA-F]{8,}\b", "H", t)
    t = re.sub(r"\d+", "N", t)
    return t.strip()


def facts(boot):
    f = {"first_line": boot[0][0], "last_line": boot[-1][0], "version": None,
         "reached_login": False, "logged_in": False, "panics": [], "faults": [],
         "errors": [], "daemon_lines": 0}
    for n, t, c in boot:
        if f["version"] is None and BANNER_RE.match(t):
            f["version"] = t
        if LOGIN_RE.search(t):
            f["reached_login"] = True
        if WELCOME_RE.search(t):
            f["logged_in"] = True
        if PANIC_RE.search(t):
            f["panics"].append({"line": n, "text": t[:300]})
        elif FAULT_RE.search(t):
            # The kernel's UNHANDLED EXCEPTION line belongs to the FATAL dump before it.
            prev = f["faults"][-1]["line"] if f["faults"] else None
            if not (OWNER_RE.search(t) and prev is not None and n - prev < 200):
                f["faults"].append({"line": n, "text": t[:300]})
        elif ERROR_RE.search(t):
            f["errors"].append({"line": n, "text": t[:300], "count": c})
        if DAEMON_RE.search(t):
            f["daemon_lines"] += 1
    # The kernel names the program of a user-space fault some lines later.
    for e in f["faults"] + f["panics"]:
        for n, t, c in boot:
            if n >= e["line"]:
                m = OWNER_RE.search(t)
                if m and n - e["line"] < 200:
                    e["program"] = f"{m.group(2)} (pid {m.group(1)})"
                    break
    f["hang_hint"] = hang_hint(boot)
    f["first_error"] = min(f["panics"] + f["faults"] + f["errors"], key=lambda e: e["line"], default=None)
    return f


def hang_hint(boot):
    """A shell command that never got back to the prompt, or a ping that never finished."""
    last_cmd = None
    for i, (n, t, c) in enumerate(boot):
        m = SHELL_PROMPT_RE.match(t)
        if m:
            last_cmd = (i, n, m.group(1).strip())
    hints = []
    # kreset and reboot end a boot on purpose.
    if last_cmd and last_cmd[2] and not RESET_CMD_RE.search(last_cmd[2]):
        i, n, cmd = last_cmd
        after = boot[i + 1:]
        hints.append(f"the last shell command (L{n}: {cmd[:80]}) never returned to the prompt; "
                     f"{len(after)} lines after it, last: {after[-1][1][:80] if after else 'none'}")
    for i, (n, t, c) in enumerate(boot):
        if t.startswith("PING "):
            done = False
            for n2, t2, c2 in boot[i + 1:i + 40]:
                if "packets transmitted" in t2:
                    done = True
                    break
                if SHELL_PROMPT_RE.match(t2):
                    break
            if not done:
                hints.append(f"ping at L{n} never printed its statistics")
    if boot and BOOT_RE.search(boot[0][1]) and not any(LOGIN_RE.search(t) for _, t, _ in boot):
        n, t, _ = boot[-1]
        hints.append(f"the boot stopped before the login prompt (reset, power cut or hang); last line L{n}: {t[:80]}")
    return "; ".join(hints) or None


def interesting(boot, baseline_sigs):
    """Lines the model should see, each marked '+' (not in the good log) or '='."""
    keep = set()
    for i, (n, t, c) in enumerate(boot):
        if PANIC_RE.search(t) or FAULT_RE.search(t):
            keep.update(range(i, min(i + 15, len(boot))))  # register dump / stack walk
            keep.update(range(max(0, i - 3), i))
        elif ERROR_RE.search(t):
            keep.update(range(max(0, i - 2), min(i + 3, len(boot))))
        elif DAEMON_RE.search(t) or BANNER_RE.match(t) or LOGIN_RE.search(t) or WELCOME_RE.search(t):
            keep.add(i)
    tail = set(range(max(0, len(boot) - TAIL_LINES), len(boot)))
    keep |= tail
    # A fault or error that repeats thousands of times: the first ones, then a count.
    total = {}
    for i in keep - tail:
        s = signature(boot[i][1])
        total[s] = total.get(s, 0) + 1
    seen = {}
    out = []
    for i in sorted(keep):
        n, t, c = boot[i]
        s = signature(t)
        if i not in tail:
            seen[s] = seen.get(s, 0) + 1
            if seen[s] == REPEAT_KEEP + 1:
                out.append(f"~L{n}: ({total[s] - REPEAT_KEEP} more lines like this one follow, left out: {t[:200]})")
            if seen[s] > REPEAT_KEEP:
                continue
        mark = "=" if baseline_sigs is not None and s in baseline_sigs else "+"
        rep = f" (x{c})" if c > 1 else ""
        out.append(f"{mark}L{n}: {t[:400]}{rep}")
    return out


def chunks(lines, size):
    cur, n = [], 0
    for l in lines:
        if cur and n + len(l) + 1 > size:
            yield cur
            cur, n = [], 0
        cur.append(l)
        n += len(l) + 1
    if cur:
        yield cur


SYSTEM = """You triage serial console logs of Redox OS on a Raspberry Pi 3B+ (or QEMU raspi3b).
You get the important lines of one boot, already filtered. Each line starts with a mark and its line number:
'+L123:' means the line does not appear in a known-good log, '=L123:' means it does (so it is normal).
If no good log was given, every line is marked '+'. The last lines of the boot are always included.
U-Boot loads Redox; 'redox login:' means the boot reached login; daemons are programs like dwc2d,
netstack (smolnetd), devshd, bcm2835-sdhostd, pcid, inputd, getty, timesync.
'panicked at' is a Rust panic; 'FATAL', 'UNHANDLED EXCEPTION', ESR_EL1/FAR_EL1 dumps are CPU faults.
A hang shows as commands that print nothing or only their first line before the log ends or the board resets;
the regex facts give a hang_hint when they see one. Name the program of a fault from the
'UNHANDLED EXCEPTION ... NAME' line or the regex facts, not from lines printed before the fault.
Only report what the lines show. Quote line numbers exactly as given. Answer with one JSON object:
{"reached_login": true|false|null, "logged_in": true|false|null,
 "panics": [{"line": N, "program": "...", "text": "..."}],
 "faults": [{"line": N, "program": "...", "text": "..."}],
 "failed_daemons": [{"name": "...", "line": N, "what": "failed|restarted|crashed|exited"}],
 "first_error": {"line": N, "text": "..."} or null,
 "hang": "short description or null",
 "new_vs_good": ["at most 5 short items: '+' lines that are problems, not ordinary device or boot messages"],
 "summary": "one or two plain sentences"}"""

MERGE = """You merge partial triage results of one Redox boot log, given in order, into one JSON object
with the same keys: reached_login, logged_in, panics, faults, failed_daemons, first_error (the earliest),
hang, new_vs_good, summary. Remove duplicates, keep line numbers, write one summary of the whole boot."""


def ask(url, model, system, user, timeout=600):
    t0 = time.time()
    tokens = [0, 0]
    # At temperature 0 the model sometimes reasons in a loop until max_tokens and
    # gives no answer. Answers need 100-700 tokens, so the first try stops early
    # and a second one with more randomness and room gets out of the loop.
    for temperature, max_tokens in ((0.2, 1500), (0.7, 3000)):
        body = json.dumps({
            "model": model,
            "messages": [{"role": "system", "content": system}, {"role": "user", "content": user}],
            "max_tokens": max_tokens,
            "temperature": temperature,
            "reasoning_effort": "low",
            "response_format": {"type": "json_object"},
        }).encode()
        req = urllib.request.Request(url.rstrip("/") + "/chat/completions", data=body,
                                     headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req, timeout=timeout) as r:
            resp = json.load(r)
        usage = resp.get("usage", {})
        tokens[0] += usage.get("prompt_tokens") or 0
        tokens[1] += usage.get("completion_tokens") or 0
        content = resp["choices"][0]["message"].get("content") or ""
        m = re.search(r"\{.*\}", content, re.S)
        try:
            result = json.loads(m.group(0)) if m else None
        except ValueError:
            result = None
        if result is not None:
            break
    if result is None:
        result = {"summary": "model gave no JSON", "raw": content[:500]}
    return result, {"seconds": round(time.time() - t0, 1), "tries": 1 if max_tokens == 1500 else 2,
                    "prompt_tokens": tokens[0], "completion_tokens": tokens[1]}


def ask_chunk(args, header, lines, stats):
    """Ask about one chunk; split it in two if the server says it is too long."""
    try:
        r, s = ask(args.url, args.model, SYSTEM, header + "\n".join(lines))
        stats.append(s)
        return [r]
    except urllib.error.HTTPError as e:
        if e.code == 400 and len(lines) > 1:
            half = len(lines) // 2
            return ask_chunk(args, header, lines[:half], stats) + ask_chunk(args, header, lines[half:], stats)
        raise


def triage_boot(args, idx, boot, baseline_sigs):
    f = facts(boot)
    sel = interesting(boot, baseline_sigs)
    res = {"boot": idx, "facts": f, "lines_total": f["last_line"] - f["first_line"] + 1,
           "lines_sent": len(sel), "model": None, "requests": []}
    clean_boot = f["reached_login"] and not (f["panics"] or f["faults"] or f["hang_hint"])
    # Text before the first U-Boot line is the tail of a boot outside the log.
    preamble = idx == 1 and not BOOT_RE.search(boot[0][1]) and not (f["panics"] or f["faults"])
    res["preamble"] = preamble
    if args.no_model or preamble or (clean_boot and not args.all and not args.boot):
        return res
    header = (f"Boot {idx}, log lines {f['first_line']}-{f['last_line']}"
              f"{', compared with a good log' if baseline_sigs is not None else ''}.\n"
              f"Regex facts (check them against the lines): {regex_facts(f)}\n\n")
    parts = []
    cs = list(chunks(sel, args.chunk_chars))
    res["chunks"] = len(cs)
    if len(cs) > args.max_chunks:
        half = args.max_chunks // 2
        dropped = sum(len(c) for c in cs[half:len(cs) - (args.max_chunks - half)])
        cs = cs[:half] + cs[len(cs) - (args.max_chunks - half):]
        header += f"({dropped} selected lines in the middle of the boot were left out.)\n"
        res["lines_dropped"] = dropped
    for i, c in enumerate(cs):
        h = header + (f"Part {i + 1} of {len(cs)}.\n" if len(cs) > 1 else "")
        parts += ask_chunk(args, h, c, res["requests"])
    if len(parts) == 1:
        res["model"] = parts[0]
    else:
        r, s = ask(args.url, args.model, MERGE, json.dumps(parts, ensure_ascii=False))
        res["requests"].append(s)
        res["model"] = r
    return res


def triage_boot_safe(args, idx, boot, baseline_sigs):
    """One boot the server could not answer must not lose the others."""
    try:
        return triage_boot(args, idx, boot, baseline_sigs)
    except (urllib.error.URLError, TimeoutError, ValueError) as e:
        res = triage_boot(argparse.Namespace(**{**vars(args), "no_model": True}), idx, boot, baseline_sigs)
        res["model"] = {"summary": f"model request failed: {e}"}
        return res


def regex_facts(f):
    out = {"reached_login": f["reached_login"], "logged_in": f["logged_in"],
           "panics": [(e["line"], e.get("program")) for e in f["panics"][:5]],
           "faults": [(e["line"], e.get("program")) for e in f["faults"][:5]],
           "panic_count": len(f["panics"]), "fault_count": len(f["faults"]),
           "hang_hint": f["hang_hint"]}
    return json.dumps(out)


def path_and_range(spec):
    m = re.match(r"^(.*?)(?::(\d+):(\d+))?$", spec)
    return m.group(1), ((int(m.group(2)), int(m.group(3))) if m.group(2) else None)


def baseline_signatures(spec):
    path, rng = path_and_range(spec)
    lines, _ = clean(read_lines(path, rng))
    return {signature(t) for _, t, _ in lines}


def yn(v):
    return {True: "yes", False: "no"}.get(v, "?")


def markdown(report):
    out = [f"# Log triage: {report['log']}", ""]
    meta = report["meta"]
    out.append(f"{meta['lines']} lines, {meta['noise_lines']} noise lines dropped, "
               f"{len(report['boots'])} boots; model {meta['model']}, "
               f"{meta['requests']} requests, {meta['seconds']} s.")
    if meta.get("baseline"):
        out.append(f"Compared with the good log `{meta['baseline']}`.")
    out += ["", "| Boot | Lines | Version | Login | Panics | Faults | Errors | Summary |",
            "|---|---|---|---|---|---|---|---|"]
    for b in report["boots"]:
        if b.get("preamble"):
            continue
        f, m = b["facts"], b["model"] or {}
        ver = (f["version"] or "").replace("redox-rpi ", "").split(" (")[0]
        summ = (m.get("summary") or ("clean (regex only)" if not b["model"] else "")).replace("|", "/")
        out.append(f"| {b['boot']} | {f['first_line']}-{f['last_line']} | {ver} | {yn(f['reached_login'])} | "
                   f"{len(f['panics'])} | {len(f['faults'])} | {len(f['errors'])} | {summ} |")
    for b in report["boots"]:
        m = b["model"]
        if not m:
            continue
        f = b["facts"]
        out += ["", f"## Boot {b['boot']} (lines {f['first_line']}-{f['last_line']})", ""]
        out.append(f"- Reached login: {yn(m.get('reached_login'))} (regex: {yn(f['reached_login'])}), "
                   f"logged in: {yn(m.get('logged_in'))}")
        for key, name in (("panics", "Panic"), ("faults", "Fault")):
            for e in m.get(key) or []:
                out.append(f"- {name} at line {e.get('line')} ({e.get('program', '?')}): `{e.get('text', '')}`")
        for d in m.get("failed_daemons") or []:
            out.append(f"- Daemon {d.get('name')} {d.get('what')} (line {d.get('line')})")
        fe = m.get("first_error")
        if fe:
            out.append(f"- First error, line {fe.get('line')}: `{fe.get('text')}`")
        if m.get("hang"):
            out.append(f"- Hang: {m['hang']}")
        if f.get("hang_hint"):
            out.append(f"- Hang hint (regex): {f['hang_hint']}")
        for x in m.get("new_vs_good") or []:
            out.append(f"- New against the good log: {x}")
        out.append(f"- Summary: {m.get('summary')}")
        secs = sum(r["seconds"] for r in b["requests"])
        out.append(f"- {b['lines_sent']} of {b['lines_total']} lines sent, {len(b['requests'])} requests, {secs:.1f} s")
    return "\n".join(out) + "\n"


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("log", help="log file (.gz is fine), optionally FILE:FIRST:LAST for a line range")
    p.add_argument("--baseline", help="known-good log, FILE or FILE:FIRST:LAST")
    p.add_argument("--boot", type=int, action="append", help="only this boot number (repeatable)")
    p.add_argument("--all", action="store_true", help="also send clean boots to the model")
    p.add_argument("--no-model", action="store_true", help="regex facts only")
    p.add_argument("--boot-re", default=BOOT_RE.pattern, help="regex for the first line of a boot")
    p.add_argument("--url", default=DEFAULT_URL)
    p.add_argument("--model", default=DEFAULT_MODEL)
    p.add_argument("--chunk-chars", type=int, default=CHUNK_CHARS)
    p.add_argument("--max-chunks", type=int, default=MAX_CHUNKS)
    p.add_argument("--jobs", type=int, default=4, help="boots sent to the model at the same time")
    p.add_argument("--json", help="write the JSON report here")
    p.add_argument("--md", help="write the Markdown report here (default: stdout)")
    args = p.parse_args()

    path, rng = path_and_range(args.log)
    t0 = time.time()
    raw = read_lines(path, rng)
    lines, noise = clean(raw)
    boots = split_boots(lines, re.compile(args.boot_re))
    base = baseline_signatures(args.baseline) if args.baseline else None
    todo = [(i, b) for i, b in enumerate(boots, 1) if not args.boot or i in args.boot]
    # vLLM batches parallel requests, so a few boots at once are nearly free.
    with concurrent.futures.ThreadPoolExecutor(args.jobs) as pool:
        results = list(pool.map(lambda ib: triage_boot_safe(args, ib[0], ib[1], base), todo))
    report = {"log": args.log, "boots": results, "meta": {
        "lines": len(raw), "noise_lines": noise, "model": args.model, "baseline": args.baseline,
        "requests": sum(len(r["requests"]) for r in results),
        "seconds": round(time.time() - t0, 1)}}
    if args.json:
        with open(args.json, "w") as f:
            json.dump(report, f, indent=1, ensure_ascii=False)
    md = markdown(report)
    if args.md:
        with open(args.md, "w") as f:
            f.write(md)
    else:
        sys.stdout.write(md)


if __name__ == "__main__":
    main()
