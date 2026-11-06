# log-triage

Pre-sorts a Redox serial or QEMU log with the local model on homenodellm
(gpt-oss-20b on vLLM, `http://127.0.0.1:8000/v1`), so a session reads a
short summary instead of thousands of lines. Python 3, standard library only.

```
tools/log-triage/log-triage.py LOG [--baseline GOOD] [--json out.json] [--md out.md]
```

- `LOG` is a file, a `.gz` file, or `FILE:FIRST:LAST` for a line range, for
  example `~/redox-rpi/logs/serial.log:78212:78800`.
- `--baseline GOOD` (same forms) is a known-good log. Lines that also appear
  there (numbers and hex masked) count as normal.
- `--boot N` looks at boot N only; `--all` also sends clean boots to the
  model; `--no-model` gives the regex facts only (a 120k-line day log takes
  about 6 s).
- `--url`, `--model` (or `LOG_TRIAGE_URL`, `LOG_TRIAGE_MODEL`) point at
  another OpenAI-compatible server.

What it does:

1. Cleans the log: ANSI and OSC codes, carriage-return redraws, the prompt
   that ion and getty redraw on every key, `call_fdread` debug lines and
   other lines every good netboot has. Repeated lines become `(xN)`, a
   register dump becomes one line.
2. Splits it into boots at each `U-Boot 20xx` line (`--boot-re` for others).
3. Regex facts per boot: version banner, reached `redox login:`, logged in,
   panics, faults (with the program from the kernel's `UNHANDLED EXCEPTION
   ... NAME` line), errors, the first error, and a hang hint (the last shell
   command never got back to the prompt, or a ping never printed its
   statistics).
4. A boot that reached login without panic, fault or hang hint is reported
   as clean without asking the model. For the others it sends the important
   lines (errors with context, fault dumps, daemon lines, the last 40 lines),
   marked `+` (not in the good log) or `=`, plus the regex facts, in chunks
   of about 6k tokens, and merges the chunk answers with one more request.
   A chunk the server finds too long is split in two.
5. Writes Markdown (table of boots, details for each boot the model saw) and
   JSON (regex facts, model answer, tokens and seconds per request).

The model's answer is a first sort, not a result: its line numbers and
program names are right most of the time, its reasons ("leading to") are
often guesses. The regex facts next to it are exact. Accuracy and speed of
the first tests are on [docs/topics/ci.md](../../docs/topics/ci.md#log-triage).
