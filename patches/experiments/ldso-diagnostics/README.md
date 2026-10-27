# Experiment: ld.so diagnostics

Relibc patch that checks every `PT_DYNAMIC` for missing entries and every
fresh memory block `ld.so` maps for nonzero bytes, and prints what it finds
(`[ld.so diag] …`).

```bash
scripts/build-image.sh --experiment ldso-diagnostics
```

**Result:** found no broken `PT_DYNAMIC` and no dirty memory. Because the
check reads every byte of the fresh memory, it hid the second start-crash
cause (0 of 80 starts instead of about 5 of 80). That led to relibc patch
`0005`. See [debugging-random-crashes.md](../../../docs/debugging-random-crashes.md),
"Second cause".

The patch was first applied on top of relibc patches 0001–0004 of commit
`0d365d2` (netboot image `sig-x1x2`, 2026-10-26). Rebased onto the current
patch set; here the diagnostic check replaces the calls to `populate()`.
