# Experiment: app-log, desktop app errors on the serial console

Test only ([#33](https://github.com/redox-rpi/redox-rpi/issues/33)), with
`orbital-apps`. The launcher entries of COSMIC Text Editor and COSMIC Files
start small ion wrappers in `/usr/libexec/app-log/`. Each writes to
`/scheme/debug` (the serial console):

    app-log: start cosmic-edit <args>
    <the app's stderr, with RUST_BACKTRACE=1>
    app-log: cosmic-edit exited with <status>

In Pi test #204 the editor closed by itself with nothing on serial. The
Pi test agent keeps the serial log; `/tmp` is gone after a power cycle.

Only root may write /scheme/debug. For any other user (`$USER` is not `root`, e.g. `user` at the Orbital login) the wrappers start the app without the log; before that fix the apps did not start at all for `user` (ion: failed to redirect stdout to file /scheme/debug, Pi test #275).
