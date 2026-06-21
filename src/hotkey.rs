use evdev::EventType;
use async_channel::Sender;
use std::collections::HashSet;

const MODIFIERS: [u16; 8] = [
    29, 97,   // Left/Right Ctrl
42, 54,   // Left/Right Shift
56, 100,  // Left/Right Alt
125, 126  // Left/Right Meta (Super/Windows)
];

pub fn start_key_listener(tx: Sender<Vec<u16>>) {
    std::thread::spawn(move || {
        for (_path, device) in evdev::enumerate() {
            // CRITICAL: Prevent infinite feedback loops by ignoring our own virtual device
            let dev_name = device.name().unwrap_or_default().to_string();
            if dev_name == "Autoclicker Virtual Mouse" {
                continue;
            }

            // We only care about devices that have keys (keyboards, mice with buttons, etc.)
            if device.supported_keys().is_some() {
                let tx_clone = tx.clone();
                std::thread::spawn(move || {
                    let mut device = device;
                    let mut pressed: HashSet<u16> = HashSet::new();
                    loop {
                        match device.fetch_events() {
                            Ok(events) => {
                                for event in events {
                                    if event.event_type() == EventType::KEY {
                                        let code = event.code();
                                        if event.value() == 1 { // Key pressed
                                            pressed.insert(code);
                                            // If it's NOT a modifier, it's the trigger key!
                                            if !MODIFIERS.contains(&code) {
                                                let mut combo: Vec<u16> = pressed.iter().copied().collect();
                                                combo.sort();
                                                let _ = tx_clone.send_blocking(combo);
                                            }
                                        } else if event.value() == 0 { // Key released
                                            pressed.remove(&code);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Input listener error: {}", e);
                                std::thread::sleep(std::time::Duration::from_secs(1));
                            }
                        }
                    }
                });
            }
        }
    });
}
