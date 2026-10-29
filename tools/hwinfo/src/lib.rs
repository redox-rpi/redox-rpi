//! Shared code of `hwinfo` and `lsusb`: the USB device list of `dwc2d` and
//! names for the devices and classes found on a Raspberry Pi.

use std::fs;

/// `dwc2d`'s device list, see `usbinfo.rs` in `dwc2d`.
pub const USB_DEVICES: &str = "/scheme/usb.dwc2/devices";

pub struct UsbDevice {
    pub address: u8,
    /// Hub address and port; 0 and 0 on the root port.
    pub hub: u8,
    pub port: u8,
    pub vendor: u16,
    pub product: u16,
    pub class: u8,
    pub usb: u16,
    pub speed: String,
    /// Class, subclass and protocol of each interface.
    pub interfaces: Vec<(u8, u8, u8)>,
}

fn hex<T: TryFrom<u32>>(text: &str) -> Option<T> {
    u32::from_str_radix(text, 16).ok()?.try_into().ok()
}

fn parse_line(line: &str) -> Option<UsbDevice> {
    let f: Vec<&str> = line.split_whitespace().collect();
    let [address, hub, port, vendor, product, class, usb, speed, interfaces] = f[..] else {
        return None;
    };
    let interfaces = interfaces
        .split(',')
        .filter_map(|i| {
            let mut p = i.split('.').map(hex::<u8>);
            Some((p.next()??, p.next()??, p.next()??))
        })
        .collect();
    Some(UsbDevice {
        address: address.parse().ok()?,
        hub: hub.parse().ok()?,
        port: port.parse().ok()?,
        vendor: hex(vendor)?,
        product: hex(product)?,
        class: hex(class)?,
        usb: hex(usb)?,
        speed: speed.to_string(),
        interfaces,
    })
}

pub fn parse_devices(text: &str) -> Vec<UsbDevice> {
    text.lines().filter_map(parse_line).collect()
}

/// The USB devices, or why there are none.
pub fn usb_devices() -> Result<Vec<UsbDevice>, String> {
    fs::read_to_string(USB_DEVICES)
        .map(|text| parse_devices(&text))
        .map_err(|err| format!("{USB_DEVICES}: {err} (dwc2d without the device list, or no USB)"))
}

/// Names of devices found on and around the Pi. Linux's `lsusb` takes them
/// from the usb.ids database; this is a short list instead.
pub fn device_name(vendor: u16, product: u16) -> Option<&'static str> {
    Some(match (vendor, product) {
        (0x0424, 0x2514) => "Microchip USB2514 hub",
        (0x0424, 0x2513) => "Microchip USB2513 hub",
        (0x0424, 0x2512) => "Microchip USB2512 hub",
        (0x0424, 0x9514) => "Microchip LAN9514 hub and Ethernet (Pi 2B, 3B)",
        (0x0424, 0xec00) => "Microchip LAN9512/LAN9514 Ethernet",
        (0x0424, 0x7800) => "Microchip LAN7800 Gigabit Ethernet (Pi 3B+)",
        (0x0bda, 0x8152) => "Realtek RTL8152 Fast Ethernet",
        (0x0bda, 0x8153) => "Realtek RTL8153 Gigabit Ethernet",
        (0x1a40, 0x0101) => "Terminus FE1.1s hub",
        (0x0525, 0xa4a2) => "Linux CDC Ethernet gadget",
        (0x0627, 0x0001) => "QEMU USB device",
        (0x0409, 0x55aa) => "QEMU USB hub",
        _ => return None,
    })
}

/// Name of a device or interface class.
pub fn class_name(class: u8) -> &'static str {
    match class {
        0x00 => "per interface",
        0x01 => "audio",
        0x02 => "communications",
        0x03 => "HID",
        0x05 => "physical",
        0x06 => "image",
        0x07 => "printer",
        0x08 => "mass storage",
        0x09 => "hub",
        0x0a => "CDC data",
        0x0b => "smart card",
        0x0e => "video",
        0xe0 => "wireless",
        0xef => "miscellaneous",
        0xfe => "application specific",
        0xff => "vendor specific",
        _ => "unknown",
    }
}

/// A name for a device without a known name: its interface classes.
pub fn describe(device: &UsbDevice) -> String {
    if let Some(name) = device_name(device.vendor, device.product) {
        return name.to_string();
    }
    if device.class != 0 {
        return class_name(device.class).to_string();
    }
    let mut names: Vec<&str> = Vec::new();
    for &(class, subclass, protocol) in &device.interfaces {
        let name = match (class, subclass, protocol) {
            (0x03, 0x01, 0x01) => "keyboard",
            (0x03, 0x01, 0x02) => "mouse",
            (class, _, _) => class_name(class),
        };
        if !names.contains(&name) {
            names.push(name);
        }
    }
    if names.is_empty() { "unknown".to_string() } else { names.join(", ") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dwc2d_lines() {
        let text = "1 0 0 0424 2514 09 0200 high 09.00.02\n2 1 1 0424 7800 ff 0210 high ff.00.ff\n4 3 2 413c 2113 00 0110 low 03.01.01,03.00.00\n5 1 3 1234 5678 00 0200 full -\n";
        let devices = parse_devices(text);
        assert_eq!(devices.len(), 4);
        assert_eq!((devices[1].vendor, devices[1].product, devices[1].hub, devices[1].port), (0x0424, 0x7800, 1, 1));
        assert_eq!(devices[2].interfaces, vec![(3, 1, 1), (3, 0, 0)]);
        assert_eq!(describe(&devices[2]), "keyboard, HID");
        assert!(devices[3].interfaces.is_empty());
        assert_eq!(describe(&devices[0]), "Microchip USB2514 hub");
    }
}
