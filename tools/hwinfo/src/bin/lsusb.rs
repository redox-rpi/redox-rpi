//! lsusb: list the USB devices that `dwc2d` found at boot.
//!
//! `lsusb` prints one line per device like Linux's `lsusb`; `-t` prints the
//! hub tree with the speed of each device; `-v` adds the interfaces.

use hwinfo::{class_name, describe, usb_devices, UsbDevice};

fn tree(devices: &[UsbDevice], hub: u8, depth: usize, verbose: bool) {
    let mut children: Vec<&UsbDevice> = devices.iter().filter(|d| d.hub == hub).collect();
    children.sort_by_key(|d| d.port);
    for device in children {
        let place = if depth == 0 { "root port".to_string() } else { format!("port {}", device.port) };
        println!(
            "{}{place}: address {}, {:04x}:{:04x} {}, {} speed",
            "    ".repeat(depth),
            device.address,
            device.vendor,
            device.product,
            describe(device),
            device.speed
        );
        if verbose {
            interfaces(device, depth + 1);
        }
        // Address 0 is never a hub; the root port's device has hub 0.
        if device.address != 0 {
            tree(devices, device.address, depth + 1, verbose);
        }
    }
}

fn interfaces(device: &UsbDevice, depth: usize) {
    for (i, &(class, subclass, protocol)) in device.interfaces.iter().enumerate() {
        println!(
            "{}interface {i}: class {class:02x} ({}), subclass {subclass:02x}, protocol {protocol:02x}",
            "    ".repeat(depth),
            class_name(class)
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let tree_view = args.iter().any(|a| a == "-t");
    let verbose = args.iter().any(|a| a == "-v");
    if let Some(arg) = args.iter().find(|a| *a != "-t" && *a != "-v") {
        eprintln!("lsusb: unknown argument {arg}; usage: lsusb [-t] [-v]");
        std::process::exit(2);
    }
    let devices = match usb_devices() {
        Ok(devices) => devices,
        Err(err) => {
            eprintln!("lsusb: {err}");
            std::process::exit(1);
        }
    };
    if tree_view {
        tree(&devices, 0, 0, verbose);
        return;
    }
    for device in &devices {
        println!(
            "Bus 001 Device {:03}: ID {:04x}:{:04x} {} (USB {:x}.{:02x}, {} speed)",
            device.address,
            device.vendor,
            device.product,
            describe(device),
            device.usb >> 8,
            device.usb & 0xff,
            device.speed
        );
        if verbose {
            interfaces(device, 1);
        }
    }
}
