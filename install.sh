#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Wayland Autoclicker Installer ===${NC}"

# 1. Install dependencies (Required for curl|bash to work on a fresh system)
if command -v pacman &> /dev/null; then
    echo -e "${GREEN}Installing dependencies (rust, gtk4, pkgconf, git)...${NC}"
    sudo pacman -S --needed --noconfirm rust gtk4 pkgconf git
else
    echo -e "${RED}This installer is designed for Arch Linux (pacman).${NC}"
    echo "Please install rust, gtk4, and pkgconf manually for your distro."
    exit 1
fi

# 2. Download the code
TMP_DIR=$(mktemp -d)
echo -e "${GREEN}Downloading source code to ${TMP_DIR}...${NC}"
git clone https://github.com/NikkyNikke/autoclicker.git "$TMP_DIR/autoclicker"
cd "$TMP_DIR/autoclicker"

# 3. Build
echo "Building Autoclicker..."
cargo build --release

# 4. Install binary
echo "Installing binary to /usr/local/bin/..."
sudo cp target/release/autoclicker /usr/local/bin/

# 5. Create KDE Menu entry
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

# 6. Set up uinput permissions
echo "Setting up uinput permissions..."
sudo modprobe uinput
echo uinput | sudo tee /etc/modules-load.d/uinput.conf > /dev/null
sudo usermod -aG input $USER
echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/80-uinput.rules > /dev/null
sudo udevadm control --reload-rules
sudo udevadm trigger /dev/uinput

# Cleanup
cd -
rm -rf "$TMP_DIR"

echo "=========================================="
echo "DONE! You can now find Autoclicker in your KDE menu."
echo "NOTE: If the clicker throws a permission error, you must log out and log back in once for the uinput group to take effect."
echo "=========================================="
