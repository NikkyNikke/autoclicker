use ksni::TrayMethods;
use ksni::menu::*;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, ComboBoxText, Label, SpinButton,
    Orientation,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::clicker::Clicker;
use crate::ipc;
use crate::hotkey;
use crate::keymap;

// --- Config Save/Load ---
fn config_dir() -> std::path::PathBuf {
    let config_dir = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{}/.config", home)
    });
    std::path::PathBuf::from(config_dir)
}

fn load_ms() -> u64 {
    std::fs::read_to_string(config_dir().join("autoclicker_ms.conf"))
    .ok()
    .and_then(|s| s.trim().parse::<u64>().ok())
    .filter(|&v| v >= 1 && v <= 60000)
    .unwrap_or(15) // Default to 15ms
}

fn save_ms(ms: u64) {
    let _ = std::fs::write(config_dir().join("autoclicker_ms.conf"), ms.to_string());
}

fn load_hotkey() -> Vec<u16> {
    let mut v = std::fs::read_to_string(config_dir().join("autoclicker_hotkey.conf"))
    .ok()
    .and_then(|s| {
        let v: Vec<u16> = s.split(',').filter_map(|p| p.trim().parse().ok()).collect();
        if v.is_empty() { None } else { Some(v) }
    })
    .unwrap_or_default();
    v.sort();
    v
}

fn save_hotkey(combo: &Vec<u16>) {
    let s = combo.iter().map(|c| c.to_string()).collect::<Vec<String>>().join(",");
    let _ = std::fs::write(config_dir().join("autoclicker_hotkey.conf"), s);
}

// --- KDE System Tray (ksni 0.3.x API) ---
struct AutoclickerTray {
    tx: async_channel::Sender<String>,
    running: Arc<AtomicBool>,
}

impl ksni::Tray for AutoclickerTray {
    fn id(&self) -> String { "autoclicker".into() }
    fn icon_name(&self) -> String { "media-playback-start".into() }
    fn title(&self) -> String { "Autoclicker".into() }

    fn tool_tip(&self) -> ksni::ToolTip {
        let mut tt = ksni::ToolTip::default();
        if self.running.load(Ordering::SeqCst) {
            tt.title = "Autoclicker: Running".into();
            tt.description = "Left-click to show window\nRight-click for options".into();
        } else {
            tt.title = "Autoclicker: Stopped".into();
            tt.description = "Left-click to show window\nRight-click for options".into();
        }
        tt
    }

    // Left-clicking the tray icon triggers this
    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send_blocking("show".to_string());
    }

    // Middle-clicking the tray icon triggers this
    fn secondary_activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send_blocking("show".to_string());
    }

    // RIGHT-CLICK MENU: Only safe options
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Show Window".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send_blocking("show".to_string());
                }),
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send_blocking("quit".to_string());
                }),
                ..Default::default()
            }.into(),
        ]
    }
}

// --- Main UI ---
pub fn run() {
    let app = Application::builder()
    .application_id("com.autoclicker.app")
    .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let clicker = match Clicker::new() {
        Ok(c) => Rc::new(RefCell::new(c)),
        Err(e) => {
            let window = ApplicationWindow::builder()
            .application(app)
            .title("Autoclicker — Setup Required")
            .default_width(550)
            .default_height(350)
            .build();

            let label = Label::new(Some(&e));
            label.set_margin_top(20);
            label.set_margin_bottom(20);
            label.set_margin_start(20);
            label.set_margin_end(20);
            label.set_wrap(true);
            label.set_max_width_chars(70);
            label.set_halign(Align::Start);
            label.set_valign(Align::Start);
            label.set_use_markup(true);

            window.set_child(Some(&label));
            window.present();
            return;
        }
    };

    // --- Channels ---
    let (ipc_tx, ipc_rx) = async_channel::unbounded::<String>();
    ipc::start_listener(ipc_tx);

    let (tray_tx, tray_rx) = async_channel::unbounded::<String>();
    let tray_running = Arc::new(AtomicBool::new(false));

    let (hotkey_tx, hotkey_rx) = async_channel::unbounded::<Vec<u16>>();
    hotkey::start_key_listener(hotkey_tx);

    // Spawn tray icon in a dedicated Tokio runtime (required by zbus/ksni 0.3)
    {
        let tray_running = tray_running.clone();
        let tray = AutoclickerTray { tx: tray_tx, running: tray_running };

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let _ = tray.spawn().await;
                // Block this thread forever so the tray handle isn't dropped
                std::future::pending::<()>().await;
            });
        });
    }

    // --- Build the UI ---
    let window = ApplicationWindow::builder()
    .application(app)
    .title("Wayland Autoclicker")
    .default_width(340)
    .default_height(420)
    .resizable(false)
    .build();

    window.set_icon_name(Some("media-playback-start"));

    let main_box = GtkBox::builder()
    .orientation(Orientation::Vertical)
    .spacing(10)
    .margin_top(20)
    .margin_bottom(20)
    .margin_start(20)
    .margin_end(20)
    .build();

    let interval_label = Label::new(Some("Interval (milliseconds):"));
    interval_label.set_halign(Align::Start);
    main_box.append(&interval_label);

    let initial_ms = load_ms();
    let interval_spin = SpinButton::with_range(1.0, 60000.0, 1.0);
    interval_spin.set_value(initial_ms as f64);
    main_box.append(&interval_spin);

    let button_label = Label::new(Some("Mouse Button:"));
    button_label.set_halign(Align::Start);
    button_label.set_margin_top(6);
    main_box.append(&button_label);

    let button_combo = ComboBoxText::new();
    button_combo.append_text("Left Click");
    button_combo.append_text("Right Click");
    button_combo.append_text("Middle Click");
    button_combo.set_active(Some(0));
    main_box.append(&button_combo);

    let toggle_btn = Button::builder()
    .label("Start Clicking")
    .margin_top(10)
    .height_request(44)
    .build();
    main_box.append(&toggle_btn);

    let status_label = Label::new(Some("● Stopped"));
    status_label.set_margin_top(4);
    main_box.append(&status_label);

    // --- Hotkey UI ---
    let hotkey_btn = Button::builder()
    .label("Configure Hotkey")
    .height_request(36)
    .margin_top(10)
    .build();
    main_box.append(&hotkey_btn);

    let hotkey_label = Label::new(Some(&format!("Current: {}", keymap::combo_name(&load_hotkey()))));
    hotkey_label.set_halign(Align::Center);
    hotkey_label.set_margin_top(5);
    main_box.append(&hotkey_label);

    let help_label = Label::new(Some(
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
Click 'Configure Hotkey' to safely record\n\
a key combination (e.g., Ctrl + E).\n\
You can also bind mouse buttons!\n\
━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
Closing this window hides it to the tray.\n\
Left-click the tray icon to show this window.\n\
Right-click the tray icon for options."
    ));
    help_label.set_margin_top(10);
    help_label.set_wrap(true);
    help_label.set_max_width_chars(42);
    help_label.set_justify(gtk4::Justification::Center);
    main_box.append(&help_label);

    let quit_btn = Button::builder()
    .label("Quit App Completely")
    .margin_top(10)
    .build();
    main_box.append(&quit_btn);

    clicker.borrow().set_interval(initial_ms);

    // --- Dialog & Recording State ---
    let is_dialog_open = Rc::new(AtomicBool::new(false));
    let is_recording = Rc::new(AtomicBool::new(false));
    let temp_hotkey = Rc::new(RefCell::new(Vec::<u16>::new()));
    let dialog_label = Rc::new(RefCell::new(Option::<Label>::None));
    let dialog_record_btn = Rc::new(RefCell::new(Option::<Button>::None));

    // --- Signal Handlers ---
    {
        let clicker = clicker.clone();
        interval_spin.connect_value_changed(move |spin| {
            let ms = spin.value() as u64;
            clicker.borrow().set_interval(ms);
            save_ms(ms);
        });
    }

    {
        let clicker = clicker.clone();
        button_combo.connect_changed(move |combo| {
            if let Some(idx) = combo.active() {
                clicker.borrow().set_button((idx + 1) as u8);
            }
        });
    }

    {
        let clicker = clicker.clone();
        let toggle_btn_clone = toggle_btn.clone();
        let status_label_clone = status_label.clone();
        let tray_running_clone = tray_running.clone();

        toggle_btn.connect_clicked(move |_| {
            let mut c = clicker.borrow_mut();
            if c.is_running() {
                c.stop();
                toggle_btn_clone.set_label("Start Clicking");
                status_label_clone.set_text("● Stopped");
                tray_running_clone.store(false, Ordering::SeqCst);
            } else {
                c.start();
                toggle_btn_clone.set_label("Stop Clicking");
                status_label_clone.set_text("● Running");
                tray_running_clone.store(true, Ordering::SeqCst);
            }
        });
    }

    {
        let clicker_clone = clicker.clone();
        let app_clone = app.clone();
        quit_btn.connect_clicked(move |_| {
            clicker_clone.borrow_mut().stop();
            app_clone.quit();
        });
    }

    // Open Dialog Logic
    {
        let app_clone = app.clone();
        let window_clone = window.clone();
        let is_dialog_open = is_dialog_open.clone();
        let is_recording = is_recording.clone();
        let temp_hotkey = temp_hotkey.clone();
        let dialog_label = dialog_label.clone();
        let dialog_record_btn = dialog_record_btn.clone();
        let hotkey_label = hotkey_label.clone();

        hotkey_btn.connect_clicked(move |_| {
            *temp_hotkey.borrow_mut() = load_hotkey();

            let dialog = ApplicationWindow::builder()
            .application(&app_clone)
            .title("Set Hotkey")
            .modal(true)
            .transient_for(&window_clone)
            .icon_name("media-playback-start") // Set icon for the dialog
            .default_width(350)
            .default_height(150)
            .resizable(false)
            .build();

            let d_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(15)
            .margin_top(20)
            .margin_bottom(20)
            .margin_start(20)
            .margin_end(20)
            .build();

            let info_label = Label::new(Some("Press the button below, then press your desired key combination."));
            info_label.set_wrap(true);
            d_box.append(&info_label);

            let current_label = Label::new(Some(&format!("Current: {}", keymap::combo_name(&temp_hotkey.borrow()))));
            current_label.set_margin_top(10);
            d_box.append(&current_label);
            *dialog_label.borrow_mut() = Some(current_label.clone());

            let record_btn = Button::builder()
            .label("Start Recording")
            .height_request(40)
            .margin_top(10)
            .build();
            d_box.append(&record_btn);
            *dialog_record_btn.borrow_mut() = Some(record_btn.clone());

            let btn_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .margin_top(10)
            .build();

            let ok_btn = Button::builder().label("OK").hexpand(true).build();
            btn_box.append(&ok_btn);

            let cancel_btn = Button::builder().label("Cancel").hexpand(true).build();
            btn_box.append(&cancel_btn);

            d_box.append(&btn_box);
            dialog.set_child(Some(&d_box));

            is_dialog_open.store(true, Ordering::SeqCst);

            // Record Button
            {
                let is_recording = is_recording.clone();
                let record_btn_clone = record_btn.clone();
                record_btn.connect_clicked(move |_| {
                    if is_recording.load(Ordering::SeqCst) {
                        is_recording.store(false, Ordering::SeqCst);
                        record_btn_clone.set_label("Start Recording");
                    } else {
                        is_recording.store(true, Ordering::SeqCst);
                        record_btn_clone.set_label("Press your hotkey...");
                    }
                });
            }

            // OK Button
            {
                let temp_hotkey = temp_hotkey.clone();
                let hotkey_label = hotkey_label.clone();
                let dialog = dialog.clone();
                let is_dialog_open = is_dialog_open.clone();

                ok_btn.connect_clicked(move |_| {
                    save_hotkey(&temp_hotkey.borrow());
                    hotkey_label.set_text(&format!("Current: {}", keymap::combo_name(&temp_hotkey.borrow())));
                    is_dialog_open.store(false, Ordering::SeqCst);
                    dialog.close();
                });
            }

            // Cancel Button
            {
                let dialog = dialog.clone();
                let is_dialog_open = is_dialog_open.clone();
                cancel_btn.connect_clicked(move |_| {
                    is_dialog_open.store(false, Ordering::SeqCst);
                    dialog.close();
                });
            }

            // Close Request (X button)
            {
                let is_dialog_open = is_dialog_open.clone();
                dialog.connect_close_request(move |_| {
                    is_dialog_open.store(false, Ordering::SeqCst);
                    glib::Propagation::Proceed
                });
            }

            dialog.present();
        });
    }

    // Prevent the MS spinbox from auto-highlighting text when the app opens.
    {
        let toggle_btn_clone = toggle_btn.clone();
        window.connect_realize(move |_| {
            toggle_btn_clone.grab_focus();
        });
    }

    // --- IPC Receiver ---
    {
        let clicker = clicker.clone();
        let toggle_btn_clone = toggle_btn.clone();
        let status_label_clone = status_label.clone();
        let tray_running_clone = tray_running.clone();

        glib::spawn_future_local(async move {
            while let Ok(cmd) = ipc_rx.recv().await {
                let mut c = clicker.borrow_mut();
                match cmd.as_str() {
                    "toggle" => {
                        if c.is_running() {
                            c.stop();
                            toggle_btn_clone.set_label("Start Clicking");
                            status_label_clone.set_text("● Stopped");
                            tray_running_clone.store(false, Ordering::SeqCst);
                        } else {
                            c.start();
                            toggle_btn_clone.set_label("Stop Clicking");
                            status_label_clone.set_text("● Running");
                            tray_running_clone.store(true, Ordering::SeqCst);
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    // --- Tray Receiver ---
    {
        let clicker = clicker.clone();
        let toggle_btn_clone = toggle_btn.clone();
        let status_label_clone = status_label.clone();
        let tray_running_clone = tray_running.clone();
        let window_clone = window.clone();
        let app_clone = app.clone();

        glib::spawn_future_local(async move {
            while let Ok(cmd) = tray_rx.recv().await {
                let mut c = clicker.borrow_mut();
                match cmd.as_str() {
                    "toggle" => {
                        if c.is_running() {
                            c.stop();
                            toggle_btn_clone.set_label("Start Clicking");
                            status_label_clone.set_text("● Stopped");
                            tray_running_clone.store(false, Ordering::SeqCst);
                        } else {
                            c.start();
                            toggle_btn_clone.set_label("Stop Clicking");
                            status_label_clone.set_text("● Running");
                            tray_running_clone.store(true, Ordering::SeqCst);
                        }
                    }
                    "show" => {
                        drop(c);
                        window_clone.present();
                    }
                    "quit" => {
                        c.stop();
                        app_clone.quit();
                    }
                    _ => {}
                }
            }
        });
    }

    // --- Global Key Listener Receiver ---
    {
        let clicker = clicker.clone();
        let toggle_btn_clone = toggle_btn.clone();
        let status_label_clone = status_label.clone();
        let tray_running_clone = tray_running.clone();

        let is_recording_clone = is_recording.clone();
        let is_dialog_open_clone = is_dialog_open.clone();
        let temp_hotkey_clone = temp_hotkey.clone();
        let dialog_label_clone = dialog_label.clone();
        let dialog_record_btn_clone = dialog_record_btn.clone();

        glib::spawn_future_local(async move {
            while let Ok(combo) = hotkey_rx.recv().await {
                if is_recording_clone.load(Ordering::SeqCst) {
                    // We are recording! Save to temp and update dialog
                    is_recording_clone.store(false, Ordering::SeqCst);
                    *temp_hotkey_clone.borrow_mut() = combo.clone();
                    if let Some(lbl) = dialog_label_clone.borrow().as_ref() {
                        lbl.set_text(&format!("Current: {}", keymap::combo_name(&combo)));
                    }
                    if let Some(btn) = dialog_record_btn_clone.borrow().as_ref() {
                        btn.set_label("Start Recording");
                    }
                } else if !is_dialog_open_clone.load(Ordering::SeqCst) {
                    // Dialog is closed, check if pressed combo matches saved hotkey
                    let saved_hotkey = load_hotkey();
                    if !saved_hotkey.is_empty() && combo == saved_hotkey {
                        let mut c = clicker.borrow_mut();
                        if c.is_running() {
                            c.stop();
                            toggle_btn_clone.set_label("Start Clicking");
                            status_label_clone.set_text("● Stopped");
                            tray_running_clone.store(false, Ordering::SeqCst);
                        } else {
                            c.start();
                            toggle_btn_clone.set_label("Stop Clicking");
                            status_label_clone.set_text("● Running");
                            tray_running_clone.store(true, Ordering::SeqCst);
                        }
                    }
                }
            }
        });
    }

    // Close to tray
    {
        let window_clone = window.clone();
        window.connect_close_request(move |_| {
            window_clone.hide();
            glib::Propagation::Stop
        });
    }

    window.set_child(Some(&main_box));
    window.present();
}
