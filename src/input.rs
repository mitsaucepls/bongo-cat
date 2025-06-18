use async_channel::Sender;
use evdev::{Device, EventSummary, KeyCode};
use std::fs;
use std::thread;

pub fn start_input_monitoring(sender: Sender<()>) -> Vec<thread::JoinHandle<()>> {
    let mut handles = Vec::new();

    if let Ok(entries) = fs::read_dir("/dev/input") {
        for entry in entries {
            let path = match entry {
                Ok(e) => e.path(),
                Err(_) => continue,
            };

            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.starts_with("event") {
                    if let Ok(dev) = Device::open(&path) {
                        if has_keyboard_keys(&dev) {
                            let handle =
                                spawn_key_listener(&path.to_string_lossy(), sender.clone());
                            handles.push(handle);
                        }
                    }
                }
            }
        }
    }

    handles
}

fn has_keyboard_keys(device: &Device) -> bool {
    device
        .supported_keys()
        .map_or(false, |keys| keys.contains(KeyCode::KEY_A))
}

fn spawn_key_listener(device_path: &str, sender: Sender<()>) -> thread::JoinHandle<()> {
    let dev_path = device_path.to_string();

    thread::spawn(move || {
        let mut dev = match Device::open(&dev_path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!(
                    "Failed to open device {}: {}. Add your user to the 'input' group.",
                    dev_path, e
                );
                return;
            }
        };

        loop {
            match dev.fetch_events() {
                Ok(events) => {
                    for ev in events {
                        if let EventSummary::Key(_, _key, value) = ev.destructure() {
                            if value == 1 {
                                let _ = sender.try_send(());
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading events from {}: {}", dev_path, e);
                    break;
                }
            }
        }
    })
}
