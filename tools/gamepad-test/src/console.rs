//! `gamepad-test --console`: the controller events on a text console (the
//! serial line or the HDMI console of an image without Orbital), from
//! inputd's `controllers` handle (redox-rpi base patch 0098, issue #320).
//! Unlike a window, it gets the events whichever VT is in front.

use std::fs::File;
use std::io::{ErrorKind, Read};
use std::mem::size_of;
use std::thread;
use std::time::Duration;

use orbclient::{Event, EventOption};

use crate::pad::{Pad, PadEvent, AXES, BUTTONS};

const PATH: &str = "/scheme/input/controllers";
/// inputd returns ESTALE once after a display handoff (not on this handle,
/// but older inputd builds may).
const ESTALE: i32 = 116;

pub fn run() -> ! {
    let mut file = File::open(PATH).unwrap_or_else(|err| {
        eprintln!("gamepad-test: cannot open {PATH}: {err}");
        eprintln!("gamepad-test: this inputd has no controllers handle (redox-rpi base patch 0098, #320)");
        std::process::exit(1)
    });
    println!("gamepad-test: waiting for controller events (Ctrl-C quits)");
    let mut pad = Pad::default();
    let mut buf = vec![0u8; 64 * size_of::<Event>()];
    loop {
        let n = match file.read(&mut buf) {
            Ok(0) => {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            Ok(n) => n,
            Err(err) if err.raw_os_error() == Some(ESTALE) || err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => {
                eprintln!("gamepad-test: read {PATH}: {err}");
                std::process::exit(1)
            }
        };
        for chunk in buf[..n].chunks_exact(size_of::<Event>()) {
            // SAFETY: inputd hands out whole orbclient events.
            let event: Event = unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const Event) };
            let pad_event = match event.to_option() {
                EventOption::ControllerButton(e) => PadEvent::Button { id: e.id, button: e.button, pressed: e.pressed },
                EventOption::ControllerAxis(e) => PadEvent::Axis { id: e.id, axis: e.axis, value: e.value },
                _ => continue,
            };
            pad.apply(pad_event);
            println!("gamepad-test: controller {}: {:<24} {}", pad.id.unwrap_or(0), pad.last, state(&pad));
        }
    }
}

/// The held buttons and the axes that are not 0, e.g. "held A START | X -32767".
pub fn state(pad: &Pad) -> String {
    let held: Vec<&str> = BUTTONS.iter().zip(pad.pressed).filter(|(_, p)| *p).map(|((_, name), _)| *name).collect();
    let axes: Vec<String> =
        AXES.iter().zip(pad.axes).filter(|(_, v)| *v != 0).map(|((_, name), v)| format!("{name} {v}")).collect();
    let held = if held.is_empty() { "-".to_string() } else { held.join(" ") };
    let axes = if axes.is_empty() { "-".to_string() } else { axes.join(" ") };
    format!("| held {held} | axes {axes}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_line() {
        let mut pad = Pad::default();
        assert_eq!(state(&pad), "| held - | axes -");
        pad.apply(PadEvent::Button { id: 4, button: 1, pressed: true });
        pad.apply(PadEvent::Button { id: 4, button: 8, pressed: true });
        pad.apply(PadEvent::Axis { id: 4, axis: 0x30, value: -32767 });
        assert_eq!(state(&pad), "| held A START | axes X -32767");
    }
}
