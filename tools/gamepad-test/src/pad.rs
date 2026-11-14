//! What the window shows, apart from drawing: the state of one controller
//! as Orbital's ControllerButton and ControllerAxis events change it. No
//! dependencies, so it tests on any host:
//! `rustc --edition 2021 --test src/pad.rs && ./pad`.
//!
//! Codes as in orbclient's event.rs (Xbox layout, D-pad from the generic
//! desktop page), which dwc2d (redox-rpi PR #378) and SDL's Redox joystick
//! driver use.

/// The 15 buttons SDL's "Redox OS Controller" has, in the order shown.
pub const BUTTONS: [(u16, &str); 15] = [
    (1, "A"),
    (2, "B"),
    (3, "X"),
    (4, "Y"),
    (5, "LB"),
    (6, "RB"),
    (7, "BACK"),
    (8, "START"),
    (9, "LS"),
    (10, "RS"),
    (11, "GUIDE"),
    (0x90, "UP"),
    (0x91, "DOWN"),
    (0x92, "RIGHT"),
    (0x93, "LEFT"),
];

/// The six axes: X, Y, RX, RY are -32768..32767 (sticks), Z and RZ 0..255
/// (triggers), as dwc2d sends them.
pub const AXES: [(u16, &str); 6] = [
    (0x30, "X"),
    (0x31, "Y"),
    (0x32, "Z"),
    (0x33, "RX"),
    (0x34, "RY"),
    (0x35, "RZ"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadEvent {
    Button { id: u32, button: u16, pressed: bool },
    Axis { id: u32, axis: u16, value: i32 },
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Pad {
    pub pressed: [bool; 15],
    pub axes: [i32; 6],
    /// Controller id of the last event (inputd fills it in).
    pub id: Option<u32>,
    pub events: u64,
    /// The last event as text, e.g. "button A pressed".
    pub last: String,
    /// Events with a code that is not in BUTTONS or AXES.
    pub unknown: u64,
}

pub fn button_index(code: u16) -> Option<usize> {
    BUTTONS.iter().position(|&(c, _)| c == code)
}

pub fn axis_index(code: u16) -> Option<usize> {
    AXES.iter().position(|&(c, _)| c == code)
}

impl Pad {
    pub fn apply(&mut self, event: PadEvent) {
        self.events += 1;
        match event {
            PadEvent::Button { id, button, pressed } => {
                self.id = Some(id);
                let state = if pressed { "pressed" } else { "released" };
                match button_index(button) {
                    Some(i) => {
                        self.pressed[i] = pressed;
                        self.last = format!("button {} {state}", BUTTONS[i].1);
                    }
                    None => {
                        self.unknown += 1;
                        self.last = format!("button {button:#x} {state} (unknown code)");
                    }
                }
            }
            PadEvent::Axis { id, axis, value } => {
                self.id = Some(id);
                match axis_index(axis) {
                    Some(i) => {
                        self.axes[i] = value;
                        self.last = format!("axis {} = {value}", AXES[i].1);
                    }
                    None => {
                        self.unknown += 1;
                        self.last = format!("axis {axis:#x} = {value} (unknown code)");
                    }
                }
            }
        }
    }

    pub fn axis(&self, code: u16) -> i32 {
        axis_index(code).map_or(0, |i| self.axes[i])
    }
}

/// A stick axis (-32768..32767) as an offset from the centre of a box
/// `half` pixels from centre to edge.
pub fn stick_offset(value: i32, half: i32) -> i32 {
    (value.clamp(-32767, 32767) as i64 * half as i64 / 32767) as i32
}

/// A trigger axis (0..255) as the filled length of a bar `len` long.
pub fn trigger_fill(value: i32, len: u32) -> u32 {
    (value.clamp(0, 255) as u32 * len) / 255
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_press_and_release() {
        let mut pad = Pad::default();
        pad.apply(PadEvent::Button { id: 1, button: 1, pressed: true });
        assert!(pad.pressed[0]);
        assert_eq!(pad.last, "button A pressed");
        assert_eq!(pad.id, Some(1));
        pad.apply(PadEvent::Button { id: 1, button: 0x93, pressed: true });
        assert!(pad.pressed[14]);
        pad.apply(PadEvent::Button { id: 1, button: 1, pressed: false });
        assert!(!pad.pressed[0] && pad.pressed[14]);
        assert_eq!(pad.last, "button A released");
        assert_eq!(pad.events, 3);
        assert_eq!(pad.unknown, 0);
    }

    #[test]
    fn axes_and_unknown_codes() {
        let mut pad = Pad::default();
        pad.apply(PadEvent::Axis { id: 2, axis: 0x31, value: -32767 });
        pad.apply(PadEvent::Axis { id: 2, axis: 0x35, value: 255 });
        assert_eq!((pad.axis(0x31), pad.axis(0x35), pad.axis(0x30)), (-32767, 255, 0));
        assert_eq!(pad.last, "axis RZ = 255");
        pad.apply(PadEvent::Button { id: 2, button: 12, pressed: true });
        pad.apply(PadEvent::Axis { id: 2, axis: 0x40, value: 3 });
        assert_eq!(pad.unknown, 2);
        assert_eq!(pad.last, "axis 0x40 = 3 (unknown code)");
        assert_eq!(pad.pressed, [false; 15]);
    }

    #[test]
    fn scaling() {
        assert_eq!(stick_offset(32767, 40), 40);
        assert_eq!(stick_offset(-32768, 40), -40);
        assert_eq!(stick_offset(0, 40), 0);
        assert_eq!(stick_offset(16384, 40), 20);
        assert_eq!(trigger_fill(255, 100), 100);
        assert_eq!(trigger_fill(0, 100), 0);
        assert_eq!(trigger_fill(300, 100), 100);
    }

    #[test]
    fn tables_have_distinct_codes() {
        for (i, (c, _)) in BUTTONS.iter().enumerate() {
            assert_eq!(button_index(*c), Some(i));
        }
        for (i, (c, _)) in AXES.iter().enumerate() {
            assert_eq!(axis_index(*c), Some(i));
        }
    }
}
