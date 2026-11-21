# Experiment pointer-bench

For [#188](https://github.com/redox-rpi/redox-rpi/issues/188) and the slow
window moves of #203: measures what pointer and window moves cost, without
a person. Adds `tools/pointer-bench`, which moves the pointer in a circle
through inputd at a fixed rate, as a mouse would, optionally with the left
button held on a window (a drag).

```
scripts/build-image.sh --experiment pointer-bench --experiment cputop \
    --experiment orbital [--experiment vc4d | --experiment rpi-fb]
```

On the Pi, over devsh, while Orbital runs:

```
pointer-bench 30 --hz 60 & cputop -n 1 -l 8 20
```

`cputop` shows how much CPU Orbital and the display daemon (`vc4d`,
`rpi-fbd` or `vesad`) use while the pointer moves.
