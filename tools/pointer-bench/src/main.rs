//! pointer-bench: moves the mouse pointer in a circle through inputd, as a
//! mouse would, so the cost of pointer moves can be measured without a
//! person (with cputop running at the same time).
//!
//! pointer-bench [seconds] [--hz N] [--center X Y] [--radius R] [--drag X Y]
//!
//! With --drag, it first moves to (X, Y), presses the left button there
//! (on a window title bar, that window is dragged), circles with the
//! button held, and lets go at the end.

use std::env;
use std::thread;
use std::time::{Duration, Instant};

use inputd::ProducerHandle;
use orbclient::{ButtonEvent, MouseEvent};

fn main() {
    let mut seconds = 20.0f64;
    let mut hz = 60.0f64;
    let (mut cx, mut cy, mut radius) = (960.0f64, 540.0f64, 300.0f64);
    let mut drag: Option<(i32, i32)> = None;
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
            "--center" => { cx = num(i + 1); cy = num(i + 2); i += 2; }
            "--drag" => { drag = Some((num(i + 1) as i32, num(i + 2) as i32)); i += 2; }
            other => seconds = other.parse().unwrap_or_else(|_| {
                eprintln!("usage: pointer-bench [seconds] [--hz N] [--center X Y] [--radius R] [--drag X Y]");
                std::process::exit(2)
            }),
        }
        i += 1;
    }

    let mut input = ProducerHandle::new().expect("pointer-bench: no inputd producer");
    let mut send = |event: orbclient::Event| {
        if let Err(err) = input.write_event(event) {
            eprintln!("pointer-bench: write_event: {err}");
        }
    };
    let pause = Duration::from_millis(100);
    // Where the circle starts, relative to the drag point: the window moves
    // by as much as the pointer does.
    let (ox, oy) = match drag {
        Some((x, y)) => {
            send(MouseEvent { x, y }.to_event());
            thread::sleep(pause);
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
        send(MouseEvent { x: x as i32, y: y as i32 }.to_event());
        moves += 1;
        let next = step * moves as u32;
        if let Some(wait) = next.checked_sub(start.elapsed()) {
            thread::sleep(wait);
        }
    }
    if drag.is_some() {
        send(ButtonEvent { left: false, middle: false, right: false }.to_event());
    }
    let took = start.elapsed().as_secs_f64();
    println!("pointer-bench: {moves} moves in {took:.1} s ({:.1}/s){}", moves as f64 / took,
        if drag.is_some() { ", dragging" } else { "" });
}
