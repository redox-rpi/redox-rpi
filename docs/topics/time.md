# Time

## State

- The Pi has no battery-backed clock. `tools/timesync` (experiment
  `timesync`, in v0.2.0) sets the time over NTP at boot; without a network
  it falls back to the last saved time or the build time. Tested on the
  Pi 3B+ and the Zero 2 W.

## Next

- `clock_settime` / `settimeofday` in relibc, so other programs can set the
  time.
- Move `timesync` from an experiment into the default images.

## Later

- An I2C clock module (DS3231) for boots without a network.
