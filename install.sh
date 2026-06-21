#!/bin/bash
set -e

echo "Building Autoclicker..."
cargo build --release

echo "Installing binary to /usr/local/bin/..."
sudo cp target/release/autoclicker /usr/local/bin/

echo "Creating KDE Menu entry..."
mkdir -p ~/.local/share/applications
cat << 'EOF' > ~/.local/share/applications/autoclicker.desktop
[Desktop Entry]
Name=Autoclicker
Comment=Wayland-Native Autoclicker
Exec=autoclicker
Icon=media-playback-start
Terminal=false
Type=Application
Categories=Utility;
EOF
chmod +x ~/.local/share/applications/autoclicker.desktop

echo "Setting up uinput permissions..."
sudo modprobe uinput
echo uinput | sudo tee /etc/modules-load.d/uinput.conf > /dev/null
sudo usermod -aG input $USER
echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/80-uinput.rules > /dev/null
sudo udevadm control --reload-rules
sudo udevadm trigger /dev/uinput

echo "=========================================="
echo "DONE! You can now find Autoclicker in your KDE menu."
echo "NOTE: If the clicker throws a permission error, you must log out and log back in once for the uinput group to take effect."
echo "=========================================="
