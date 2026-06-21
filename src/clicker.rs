use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
use evdev::{AttributeSet, EventType, InputEvent, Key, RelativeAxisType};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Raw evdev button codes (from linux/input-event-codes.h)
const BTN_LEFT: u16   = 0x110;
const BTN_RIGHT: u16  = 0x111;
const BTN_MIDDLE: u16 = 0x112;
const BTN_SIDE: u16   = 0x113;
const BTN_EXTRA: u16  = 0x114;

const SYN_REPORT: u16 = 0x00;

pub struct Clicker {
    running: Arc<AtomicBool>,
    interval_ms: Arc<AtomicU64>,
    button: Arc<AtomicU8>,
    device: Arc<Mutex<VirtualDevice>>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl Clicker {
    pub fn new() -> Result<Self, String> {
        let mut keys = AttributeSet::<Key>::new();
        keys.insert(Key::BTN_LEFT);
        keys.insert(Key::BTN_RIGHT);
        keys.insert(Key::BTN_MIDDLE);
        keys.insert(Key::BTN_SIDE);
        keys.insert(Key::BTN_EXTRA);

        let mut axes = AttributeSet::<RelativeAxisType>::new();
        axes.insert(RelativeAxisType::REL_X);
        axes.insert(RelativeAxisType::REL_Y);
        axes.insert(RelativeAxisType::REL_WHEEL);

        let device = VirtualDeviceBuilder::new()
        .map_err(|e| format!(
            "Failed to open /dev/uinput: {}\n\n\
===================== SETUP REQUIRED =====================\n\
1. Load the uinput kernel module:\n\
sudo modprobe uinput\n\n\
2. Make it persistent across reboots:\n\
echo uinput | sudo tee /etc/modules-load.d/uinput.conf\n\n\
3. Grant your user access to /dev/uinput:\n\
sudo usermod -aG input $USER\n\n\
4. Create a udev rule for reliable access:\n\
echo 'KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\"' | sudo tee /etc/udev/rules.d/80-uinput.rules\n\
sudo udevadm control --reload-rules\n\
sudo udevadm trigger /dev/uinput\n\n\
5. Log out and log back in (or reboot).\n\
=========================================================",
e
        ))?
        .name("Autoclicker Virtual Mouse")
        .with_keys(&keys)
        .map_err(|e| format!("Failed to configure device keys: {}", e))?
        .with_relative_axes(&axes)
        .map_err(|e| format!("Failed to configure device axes: {}", e))?
        .build()
        .map_err(|e| format!("Failed to create virtual device: {}", e))?;

        Ok(Clicker {
            running: Arc::new(AtomicBool::new(false)),
           interval_ms: Arc::new(AtomicU64::new(100)),
           button: Arc::new(AtomicU8::new(1)),
           device: Arc::new(Mutex::new(device)),
           thread_handle: None,
        })
    }

    pub fn set_interval(&self, ms: u64) {
        // Clamp to 1ms minimum
        self.interval_ms.store(ms.max(1), Ordering::SeqCst);
    }

    pub fn set_button(&self, button: u8) {
        self.button.store(button, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn start(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            return;
        }
        self.running.store(true, Ordering::SeqCst);

        let running = self.running.clone();
        let interval_ms = self.interval_ms.clone();
        let button = self.button.clone();
        let device = self.device.clone();

        self.thread_handle = Some(thread::spawn(move || {
            click_loop(running, interval_ms, button, device);
        }));
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Clicker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn click_loop(
    running: Arc<AtomicBool>,
    interval_ms: Arc<AtomicU64>,
    button: Arc<AtomicU8>,
    device: Arc<Mutex<VirtualDevice>>,
) {
    while running.load(Ordering::SeqCst) {
        let btn_code = button_to_code(button.load(Ordering::SeqCst));

        // --- Press event + sync ---
        {
            let mut dev = device.lock().unwrap_or_else(|e| e.into_inner());
            let events = [
                InputEvent::new(EventType::KEY, btn_code, 1),
                InputEvent::new(EventType::SYNCHRONIZATION, SYN_REPORT, 0),
            ];
            if dev.emit(&events).is_err() {
                eprintln!("Warning: failed to send press event");
            }
        }

        // Minimum gap between press and release so apps register it as a click.
        thread::sleep(Duration::from_millis(1));

        // --- Release event + sync ---
        {
            let mut dev = device.lock().unwrap_or_else(|e| e.into_inner());
            let events = [
                InputEvent::new(EventType::KEY, btn_code, 0),
                InputEvent::new(EventType::SYNCHRONIZATION, SYN_REPORT, 0),
            ];
            if dev.emit(&events).is_err() {
                eprintln!("Warning: failed to send release event");
            }
        }

        // --- Wait for the configured interval ---
        let ms = interval_ms.load(Ordering::SeqCst);
        let mut remaining = ms;
        while remaining > 0 && running.load(Ordering::SeqCst) {
            let step = remaining.min(5);
            thread::sleep(Duration::from_millis(step));
            remaining = remaining.saturating_sub(step);
        }
    }
}

fn button_to_code(button: u8) -> u16 {
    match button {
        1 => BTN_LEFT,
        2 => BTN_RIGHT,
        3 => BTN_MIDDLE,
        4 => BTN_SIDE,
        5 => BTN_EXTRA,
        _ => BTN_LEFT,
    }
}

/// Fire a single click without needing the GUI running.
pub fn single_click(button: u8) -> Result<(), String> {
    let mut clicker = Clicker::new()?;
    clicker.set_button(button);
    clicker.start();
    thread::sleep(Duration::from_millis(20));
    clicker.stop();
    Ok(())
}
