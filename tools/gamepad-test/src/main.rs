//! gamepad-test: a window that shows a game controller's buttons and axes
//! live, from Orbital's ControllerButton and ControllerAxis events (issue
//! #320; dwc2d sends them for USB gamepads since PR #378).
//!
//! gamepad-test           the window; every event is also printed to stdout
//! gamepad-test --send    no window: sends a fixed sequence of controller
//!                        events through inputd, as dwc2d would (all
//!                        buttons, then the axes), to check the window
//!                        without a pad (QEMU has none)
//!
//! The window takes the controller events only while it is the front
//! window. Esc or closing the window quits.

mod pad;

use std::env;
use std::thread;
use std::time::Duration;

use orbclient::{Color, EventOption, Renderer, Window, WindowFlag, K_ESC};

use crate::pad::{stick_offset, trigger_fill, Pad, PadEvent, AXES, BUTTONS};

const W: u32 = 440;
const H: u32 = 280;
const BG: Color = Color { data: 0xff20_2228 };
const FG: Color = Color { data: 0xffe0_e0e0 };
const DIM: Color = Color { data: 0xff50_5560 };
const LIT: Color = Color { data: 0xff3c_c864 };
const DOT: Color = Color { data: 0xfff0_b030 };

fn main() {
    if env::args().nth(1).as_deref() == Some("--send") {
        send();
        return;
    }
    if let Some(arg) = env::args().nth(1) {
        eprintln!("usage: gamepad-test [--send]   (unknown argument {arg})");
        std::process::exit(2);
    }
    let mut window = Window::new_flags(-1, -1, W, H, "Gamepad test", &[]).unwrap_or_else(|| {
        eprintln!("gamepad-test: no window (is Orbital running?)");
        std::process::exit(1)
    });
    let mut pad = Pad::default();
    draw(&mut window, &pad);
    'run: loop {
        let mut changed = false;
        for event in window.events() {
            let pad_event = match event.to_option() {
                EventOption::ControllerButton(e) => {
                    Some(PadEvent::Button { id: e.id, button: e.button, pressed: e.pressed })
                }
                EventOption::ControllerAxis(e) => Some(PadEvent::Axis { id: e.id, axis: e.axis, value: e.value }),
                EventOption::Key(k) if k.scancode == K_ESC && k.pressed => break 'run,
                EventOption::Quit(_) => break 'run,
                _ => None,
            };
            if let Some(e) = pad_event {
                pad.apply(e);
                println!("gamepad-test: controller {}: {}", pad.id.unwrap_or(0), pad.last);
                changed = true;
            }
        }
        if changed {
            draw(&mut window, &pad);
        }
    }
}

fn text(window: &mut Window, x: i32, y: i32, s: &str, color: Color) {
    for (i, c) in s.chars().enumerate() {
        window.char(x + 8 * i as i32, y, c, color);
    }
}

fn frame(window: &mut Window, x: i32, y: i32, w: u32, h: u32, color: Color) {
    let (x2, y2) = (x + w as i32 - 1, y + h as i32 - 1);
    window.line(x, y, x2, y, color);
    window.line(x, y2, x2, y2, color);
    window.line(x, y, x, y2, color);
    window.line(x2, y, x2, y2, color);
}

/// A stick: a square with a cross and a dot where the stick points. Up is a
/// positive Y value (dwc2d inverts the HID value, SDL inverts it back).
fn stick(window: &mut Window, x: i32, y: i32, size: i32, ax: i32, ay: i32, name: &str) {
    let half = size / 2;
    let (cx, cy) = (x + half, y + half);
    frame(window, x, y, size as u32, size as u32, DIM);
    window.line(cx, y, cx, y + size - 1, DIM);
    window.line(x, cy, x + size - 1, cy, DIM);
    let (dx, dy) = (stick_offset(ax, half - 5), stick_offset(ay, half - 5));
    window.rect(cx + dx - 4, cy - dy - 4, 9, 9, DOT);
    text(window, x, y + size + 4, name, FG);
}

fn draw(window: &mut Window, pad: &Pad) {
    window.set(BG);
    let id = pad.id.map_or("-".to_string(), |id| id.to_string());
    text(window, 8, 6, &format!("controller {id}   events {}", pad.events), FG);
    let last = if pad.last.is_empty() { "press a button on the pad".to_string() } else { pad.last.clone() };
    text(window, 8, 24, &format!("last: {last}"), FG);

    // 15 buttons, 5 per row.
    for (i, (_, name)) in BUTTONS.iter().enumerate() {
        let x = 8 + (i % 5) as i32 * 86;
        let y = 48 + (i / 5) as i32 * 28;
        if pad.pressed[i] {
            window.rect(x, y, 80, 22, LIT);
        }
        frame(window, x, y, 80, 22, if pad.pressed[i] { LIT } else { DIM });
        let tx = x + 40 - 4 * name.len() as i32;
        text(window, tx, y + 3, name, if pad.pressed[i] { BG } else { FG });
    }

    // Sticks and triggers.
    let top = 140;
    let (x, y, rx, ry) = (pad.axis(0x30), pad.axis(0x31), pad.axis(0x33), pad.axis(0x34));
    stick(window, 8, top, 100, x, y, &format!("X {x} Y {y}"));
    stick(window, 160, top, 100, rx, ry, &format!("RX {rx} RY {ry}"));
    for (row, code) in [0x32u16, 0x35].iter().enumerate() {
        let value = pad.axis(*code);
        let name = AXES.iter().find(|a| a.0 == *code).map_or("?", |a| a.1);
        let by = top + 10 + row as i32 * 50;
        text(window, 300, by, &format!("{name} {value}"), FG);
        frame(window, 300, by + 18, 130, 14, DIM);
        let fill = trigger_fill(value, 128);
        if fill > 0 {
            window.rect(301, by + 19, fill, 12, LIT);
        }
    }
    if pad.unknown > 0 {
        text(window, 300, top + 110, &format!("{} unknown", pad.unknown), DOT);
    }
    window.sync();
}

/// Sends a test sequence through inputd, as dwc2d sends a pad's reports.
fn send() {
    use orbclient::{ControllerAxisEvent, ControllerButtonEvent};
    let mut input = inputd::ProducerHandle::new().unwrap_or_else(|err| {
        eprintln!("gamepad-test: no inputd producer: {err}");
        std::process::exit(1)
    });
    let mut write = |event: orbclient::Event, what: String| {
        match input.write_event(event) {
            Ok(_) => println!("gamepad-test: sent {what}"),
            Err(err) => eprintln!("gamepad-test: write_event: {err}"),
        }
        thread::sleep(Duration::from_millis(120));
    };
    for (code, name) in BUTTONS {
        for pressed in [true, false] {
            let e = ControllerButtonEvent { id: 0, button: code, pressed };
            write(e.to_event(), format!("button {name} {}", if pressed { "pressed" } else { "released" }));
        }
    }
    // Sticks to the corners and back, triggers up and down; ends with the
    // left stick up-right and RZ half way, so a screenshot shows them.
    let moves: [(u16, i32); 14] = [
        (0x30, -32767), (0x31, 32767), (0x33, 32767), (0x34, -32767),
        (0x32, 255), (0x35, 255), (0x30, 0), (0x31, 0), (0x33, 0), (0x34, 0),
        (0x32, 0), (0x30, 20000), (0x31, 20000), (0x35, 128),
    ];
    for (axis, value) in moves {
        let name = AXES.iter().find(|a| a.0 == axis).map_or("?", |a| a.1);
        write(ControllerAxisEvent { id: 0, axis, value }.to_event(), format!("axis {name} = {value}"));
    }
    // Leave A held, so a screenshot shows a lit button.
    write(ControllerButtonEvent { id: 0, button: 1, pressed: true }.to_event(), "button A pressed".into());
}
