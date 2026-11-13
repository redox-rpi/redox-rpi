# Experiment: scheme-dead-reply, log scheme replies that reach a caller that is gone

Issue [#401](https://github.com/redox-rpi/redox-rpi/issues/401). Debug only.

A scheme reply with data for a caller that has exited, or is being
SIGKILLed, is lost. For a pty, that is a typed line: after `nc` exits in a
devsh session, the next line can disappear. Kernel `0054` sends a Cancel
for a killed caller, which fixed most cases, but Pi rows 394 and 395 still
lost one line in about 20 `nc` rounds.

Kernel `0001` here logs each such reply:

- `scheme N: reply of B bytes for a caller that is gone (tag T, cancel sent X)`:
  `cancel sent true` means the scheme answered with data despite the
  Cancel (a ptyd ordering problem); `false` means the thread died on a
  path that sends no Cancel.
- `scheme N: reply of B bytes for a caller being killed (tag T)`: the reply
  arrived while the thread was being killed.

`ls /scheme` order or `/scheme/sys/...` does not give the scheme number;
compare it with the numbers of the other lines, or with a reply logged on
purpose.

```bash
scripts/build-image.sh --experiment scheme-dead-reply
```
