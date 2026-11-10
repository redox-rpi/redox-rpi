# Graphics (`GFX`)

Topic: [graphics](../topics/graphics.md). Trace: [trace-v0.4.0.md](trace-v0.4.0.md).

In the release image since v0.4.0 (PR #255, rc2).

### REQ-GFX-1 Optional desktop from the console (beta)

The release image shall boot to the text console; `desktop` as root shall
start the Orbital desktop, and the user shall switch between console and
desktop.

| Field | Value |
|---|---|
| Rationale | The maintainer wants a desktop in the release, but not as the default (decision for rc2). |
| Priority | should |
| Kind | functional |
| Level | beta (one resolution, Orbital runs until reboot, window dragging costs a core) |
| PBS | 2.5.4, 2.3.6 |
| Status | proposed |
| Since | v0.4.0 |
| Implemented by | #33, PR #255 (draft), PR #113 |
| Verification | test, person: checklist v0.4.0 test 22, row 286 (rc2 visit, pass) and row 292 (22 A, automated part); rows 287 and 288 were exploratory (`--allow-dirty`) |

Acceptance criteria:

1. Without `desktop`, the Pi boots to the text console as without the
   desktop ([REQ-CON-1](console.md#req-con-1-hdmi-console-with-login)).
2. `desktop` as root shows the Orbital login on VT 3 within 30 s
   (proposed); `user` with an empty password logs in to the launcher.
3. `Super`+`F2` goes to the text console and `Super`+`F3` back; both keep
   working.
4. Mouse and keyboard work in the desktop.

### REQ-GFX-2 Desktop apps (beta)

The desktop shall offer a terminal, a text editor that saves files, and a
file manager.

| Field | Value |
|---|---|
| Rationale | A desktop without apps is not usable. |
| Priority | may |
| Kind | functional |
| Level | beta (editor about 40 s to start, save dialog up to 30 s) |
| PBS | 2.5.4 |
| Status | proposed |
| Since | v0.4.0 |
| Implemented by | #33, PR #255 |
| Verification | test, person: checklist v0.4.0 test 22, row 286 (rc2 visit): orbterm and Files pass, the editor cannot save (#33) |

Acceptance criteria:

1. `Super`+`T` opens orbterm, and a command runs in it.
2. COSMIC Text Editor opens within 60 s (proposed), takes typed text and
   saves it to a file that `cat` shows on the console.
3. COSMIC Files opens and lists the home directory.
