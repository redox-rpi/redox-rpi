# Experiment: t1-memmap, memory map on an unhandled exception

Diagnostics for [#1](https://github.com/redox-rpi/redox-rpi/issues/1): a
rare data abort at program start, with `ELR` around `0xC0_0000`.

When a process dies of an exception it does not handle, the kernel prints
every grant (mapped memory range) of its address space after the usual
register dump:

```text
  grant 0xc00000..0xc25000 PageFlags { ... } Allocated { ... }
```

That shows which object the faulting code belongs to, and whether the
address it read (`FAR_EL1`) was mapped at all.

## Build

```bash
scripts/build-image.sh --experiment t1-memmap
```

## Result

Not run yet.
