# Wayland-Native Autoclicker 🖱️

> Built out of spite because existing autoclickers on Linux either did not exist, did not work on Wayland, or were not good enough.

A blazingly fast, fully native autoclicker built for **Arch Linux (CachyOS)** and **KDE Plasma Wayland**. It works by creating a kernel-level virtual input device via `/dev/uinput`, meaning clicks are delivered to **all Wayland apps, XWayland apps, and games** identically to a physical mouse. No compositor-specific protocols needed.

## ✨ Features
- **Wayland & XWayland Native**: Works everywhere a real mouse works.
- **Global Hotkeys**: Binds directly to `/dev/input/eventX` to capture hotkeys at the kernel level. No KDE System Settings configuration required. Supports complex combinations (e.g., `Ctrl + Shift + E`) and even mouse buttons!
- **Rust + GTK4**: Fast, memory-safe, and integrates perfectly with KDE.
- **Custom Intervals**: Default 15ms, clamped to 1ms minimum (anything lower is unrealistic for the kernel/libinput pipeline).
- **System Tray**: Close the window and it hides in the tray. Left-click to show, right-click for options.
- **Remembers Settings**: Saves your MS interval and hotkey combination automatically.

## 🚀 One-Line Install (Arch Linux)
Run this command in your terminal to automatically install dependencies, compile, and set up the app:

    curl -fsSL https://raw.githubusercontent.com/NikkyNikke/autoclicker/main/install.sh | bash

*(Note: You will need to log out and log back in once for the `input` group permissions to take effect).*

## 🛠️ Manual Build (For Developers)

    git clone https://github.com/NikkyNikke/autoclicker.git
    cd autoclicker
    cargo build --release

## 📖 How to Use
1. Launch "Autoclicker" from your KDE app menu.
2. Click **"Configure Hotkey"** to safely record a key combination.
3. Focus your game/app and press your hotkey to start/stop clicking!

## Why not `xdotool` or `ydotool`?
`ydotool` requires running a background daemon and managing sockets. This app handles everything natively in one lightweight 3MB binary. It does not rely on compositor-specific `wlr` protocols, making it universally compatible.
