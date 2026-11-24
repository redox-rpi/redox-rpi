# Experiment: fmap-fault-fix, first fix for the scheme mmap fault (#678)

Kernel patch: the page fault path for a scheme mmap (`Provider::FmapBorrowed`
in `correct_inner`) held `context::current()`, a `Ref` on the per-CPU
current context, across `switch()`. `switch_to` then panicked with
`RefCell already borrowed` (`switch.rs:1391`). Any process that writes to a
`MAP_PRIVATE` + `PROT_WRITE` mapping of a file from a user scheme panicked
the kernel. Upstream master (73e92b4a) has the same code.

This only removes the panic. The thread still waits for the scheme to
answer `RequestMmap`, and the schemes do not answer it, so the write hangs.
The two `todo!`s of #678 (Stop and `NonfatalInternalError`) can then be
reached with `mmapstop stop` and `mmapstop kill` (experiment `mmap-stop`).

```bash
scripts/build-image.sh --experiment mmap-stop --experiment fmap-fault-fix
```
