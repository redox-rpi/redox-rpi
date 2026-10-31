# Experiment: timerd test timers

Two test timers for [`timerd`](../timerd/README.md), only for testing:
`test-minute` runs `date` every minute, `test-catchup` runs `uname -a`
every two minutes and catches up a missed run after a restart
(`persistent = true`).

```bash
scripts/build-image.sh --sdcard --experiment timesync --experiment timerd --experiment timerd-test
```
