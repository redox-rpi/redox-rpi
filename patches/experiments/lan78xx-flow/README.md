# LAN7800 flow control (experiment `lan78xx-flow`)

Issue [#661](https://github.com/redox-rpi/redox-rpi/issues/661), see
`lan78xx-rearm` for the problem.

dwc2d turns flow control off (`FLOW 0`, `FCT_FLOW 0`), although the PHY
advertises pause. A full receive FIFO then drops frames.

- **base `0119`** (dwc2d): flow control as Linux sets it for a
  high-speed USB link: a pause frame when the FIFO holds 8.5 KiB
  (`FCT_FLOW` on 17, off 2, in 512-byte units), pause time 0xFFFF, and
  the MAC obeys pause frames from the link partner. It is turned on
  whatever the link partner advertised; a final version should use the
  negotiated result.

Test: as `lan78xx-rearm`, alone and together with it.
