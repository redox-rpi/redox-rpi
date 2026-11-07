//! pointer-bench: moves the mouse pointer in a circle through inputd, as a
//! mouse would, so the cost of pointer moves can be measured without a
//! person (with cputop running at the same time).
//!
//! pointer-bench [seconds] [--hz N] [--screen W H] [--center X Y] [--radius R]
//!               [--drag X Y] [--super-drag] [--type TEXT] [--super KEY]
//!
//! --type types TEXT first (lowercase letters, digits, space; `\\n` is
//! Enter, `\\t` is Tab), as a keyboard would: to log in at orblogin.
//! --super KEY presses Super+KEY (Super+t opens a terminal). With --type
//! or --super, the default time is 0 s (no moves). --keys MODS+KEY presses
//! a combination, e.g. ctrl+shift+s (modifiers super, ctrl, shift, alt).
//!
//! Positions are screen pixels; Orbital takes absolute mouse events as
//! 0..65535 over the screen (as ps2d sends them), so they are scaled by
//! the screen size (default 1920x1080, the firmware mode on our NanoKVM).
//!
//! With --super-drag, Super is held during the drag, which moves the
//! window under the pointer from anywhere inside it (Orbital).
//! With --drag, it first moves to (X, Y), presses the left button there
//! (on a window title bar, that window is dragged), circles with the
//! button held, and lets go at the end.

use std::env;
use std::thread;
use std::time::{Duration, Instant};

use inputd::ProducerHandle;
use orbclient::{ButtonEvent, KeyEvent, MouseEvent};

/// PC set 1 scancodes (what inputd expects) for the characters --type knows.
fn scancode(c: char) -> Option<u8> {
    const ROWS: [(&str, u8); 4] = [("1234567890", 0x02), ("qwertyuiop", 0x10), ("asdfghjkl", 0x1e), ("zxcvbnm", 0x2c)];
    match c {
        '\n' => Some(orbclient::K_ENTER),
        '\t' => Some(orbclient::K_TAB),
        ' ' => Some(orbclient::K_SPACE),
        _ => ROWS.iter().find_map(|(row, first)| row.find(c).map(|i| first + i as u8)),
    }
}

fn main() {
    let mut seconds: Option<f64> = None;
    let mut text: Option<String> = None;
    let mut combo: Option<String> = None;
    let mut super_drag = false;
    let mut hz = 60.0f64;
    let (mut cx, mut cy, mut radius) = (960.0f64, 540.0f64, 300.0f64);
    let mut drag: Option<(i32, i32)> = None;
    let (mut sw, mut sh) = (1920.0f64, 1080.0f64);
    let args: Vec<String> = env::args().skip(1).collect();
    let num = |i: usize| -> f64 {
        args.get(i).and_then(|a| a.parse().ok()).unwrap_or_else(|| {
            eprintln!("pointer-bench: a number is missing after {}", args[i - 1]);
            std::process::exit(2)
        })
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--hz" => { hz = num(i + 1); i += 1; }
            "--radius" => { radius = num(i + 1); i += 1; }
            "--screen" => { sw = num(i + 1); sh = num(i + 2); i += 2; }
            "--center" => { cx = num(i + 1); cy = num(i + 2); i += 2; }
            "--type" => { text = Some(args.get(i + 1).cloned().unwrap_or_default().replace("\\n", "\n").replace("\\t", "\t")); i += 1; }
            "--super" => { combo = args.get(i + 1).map(|a| format!("super+{a}")); i += 1; }
            "--keys" => { combo = args.get(i + 1).cloned(); i += 1; }
            "--super-drag" => super_drag = true,
            "--drag" => { drag = Some((num(i + 1) as i32, num(i + 2) as i32)); i += 2; }
            other => seconds = Some(other.parse().unwrap_or_else(|_| {
                eprintln!("usage: pointer-bench [seconds] [--hz N] [--screen W H] [--center X Y] [--radius R] [--drag X Y] [--super-drag] [--type TEXT] [--super KEY] [--keys MODS+KEY]");
                std::process::exit(2)
            })),
        }
        i += 1;
    }

    let to_event = |x: f64, y: f64| {
        MouseEvent { x: (x * 65536.0 / sw) as i32, y: (y * 65536.0 / sh) as i32 }.to_event()
    };
    let mut input = ProducerHandle::new().expect("pointer-bench: no inputd producer");
    let mut send = |event: orbclient::Event| {
        if let Err(err) = input.write_event(event) {
            eprintln!("pointer-bench: write_event: {err}");
        }
    };
    let pause = Duration::from_millis(100);
    let key = |scancode: u8, pressed: bool| KeyEvent { character: '\0', scancode, pressed }.to_event();
    let seconds = seconds.unwrap_or(if text.is_some() || combo.is_some() { 0.0 } else { 20.0 });
    if let Some(text) = &text {
        for c in text.chars() {
            let Some(code) = scancode(c) else {
                eprintln!("pointer-bench: cannot type {c:?}");
                continue;
            };
            send(key(code, true));
            thread::sleep(Duration::from_millis(30));
            send(key(code, false));
            thread::sleep(Duration::from_millis(70));
        }
        println!("pointer-bench: typed {} keys", text.chars().count());
    }
    if let Some(spec) = &combo {
        // MODS+KEY, e.g. super+t or ctrl+shift+s: modifiers down, key, up.
        let mut parts: Vec<&str> = spec.split('+').collect();
        let last = parts.pop().unwrap_or_default();
        let code = last.chars().next().filter(|_| last.chars().count() == 1).and_then(scancode)
            .unwrap_or_else(|| { eprintln!("pointer-bench: no key {last:?}"); std::process::exit(2) });
        let mods: Vec<u8> = parts.iter().map(|m| match *m {
            "super" => orbclient::K_SUPER,
            "ctrl" => orbclient::K_CTRL,
            "shift" => orbclient::K_LEFT_SHIFT,
            "alt" => orbclient::K_ALT,
            _ => { eprintln!("pointer-bench: no modifier {m:?}"); std::process::exit(2) }
        }).collect();
        for &m in &mods {
            send(key(m, true));
            thread::sleep(pause);
        }
        send(key(code, true));
        thread::sleep(Duration::from_millis(30));
        send(key(code, false));
        thread::sleep(pause);
        for &m in mods.iter().rev() {
            send(key(m, false));
        }
        println!("pointer-bench: {spec}");
    }
    // Where the circle starts, relative to the drag point: the window moves
    // by as much as the pointer does.
    let (ox, oy) = match drag {
        Some((x, y)) => {
            send(to_event(f64::from(x), f64::from(y)));
            thread::sleep(pause);
            if super_drag {
                send(key(orbclient::K_SUPER, true));
                thread::sleep(pause);
            }
            send(ButtonEvent { left: true, middle: false, right: false }.to_event());
            thread::sleep(pause);
            (f64::from(x) - (cx + radius), f64::from(y) - cy)
        }
        None => (0.0, 0.0),
    };

    let step = Duration::from_secs_f64(1.0 / hz);
    let start = Instant::now();
    let mut moves = 0u64;
    while start.elapsed().as_secs_f64() < seconds {
        let angle = moves as f64 * std::f64::consts::TAU / (hz * 2.0);
        let x = cx + radius * angle.cos() + ox;
        let y = cy + radius * angle.sin() + oy;
        send(to_event(x, y));
        moves += 1;
        let next = step * moves as u32;
        if let Some(wait) = next.checked_sub(start.elapsed()) {
            thread::sleep(wait);
        }
    }
    if drag.is_some() {
        send(ButtonEvent { left: false, middle: false, right: false }.to_event());
        if super_drag {
            thread::sleep(pause);
            send(key(orbclient::K_SUPER, false));
        }
    }
    if seconds <= 0.0 {
        return;
    }
    let took = start.elapsed().as_secs_f64();
    println!("pointer-bench: {moves} moves in {took:.1} s ({:.1}/s){}", moves as f64 / took,
        if drag.is_some() { ", dragging" } else { "" });
}
