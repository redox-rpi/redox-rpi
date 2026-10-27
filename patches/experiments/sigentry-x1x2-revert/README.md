# Experiment: sigentry-x1x2-revert, negative control

Reverts relibc patch `0004`, so x1 and x2 are swapped again when a signal
handler returns. Only for checking that
[`sigregs`](../sigregs-test) finds that bug; never use it in an image for
anything else.

```bash
scripts/build-image.sh --experiment sigregs-test --experiment sigentry-x1x2-revert
```

## Results

QEMU, 2026-10-27: `sigregs` reports `x1 changed` after 2-33 signals in
both modes, as expected. Details in the
[test reports](../../../docs/test-reports.md).
