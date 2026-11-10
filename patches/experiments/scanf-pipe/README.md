# Experiment: scanf-pipe, scanf keeps its look-ahead character on a pipe

For [#338](https://github.com/redox-rpi/redox-rpi/issues/338) (topic
relibc). Found with qbe, which read cproc's piped `data` as `dta`
([#327](https://github.com/redox-rpi/redox-rpi/issues/327)).

**The bug:** relibc's scanf reads a `FILE` through `FileReader`
(`src/header/stdio/reader.rs`), which returns each character and then
`fseek`s back to it, so the character scanf stops at stays in the stream.
On a pipe `fseek` drops the `FILE`'s read buffer and fails: the look-ahead
character and all input already buffered from the pipe are lost, and every
later read returns EOF. On a regular file it works. Upstream relibc master
(3e4160f6, 2026-11-10) has the same code; upstream has no issue or merge
request about it.

**The fix** (`relibc/0001`): no seeking. Each character's bytes go back
into the `FILE`'s `unget` buffer (as with `ungetc`) and are taken out when
the next character is read, so the stream ends where it did before on a
file, and now also on a pipe. `wscanf` uses the same reader. The patch adds
relibc test `stdio/fscanf_pipe` (expected output from glibc).

**Check:** build with `--experiment tcc --experiment scanf-pipe` (tcc
compiles the test program on the Pi), then

    ion /usr/share/scanf-pipe/check.ion

runs `scanf-pipe.c` on its own `pipe()`, on a file and on a shell pipe;
each must print `expected.txt` (glibc's output).

## Results

None yet.
