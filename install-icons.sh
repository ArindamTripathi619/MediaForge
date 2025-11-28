#!/bin/bash
# Install MediaForge icon for Hyprland/Wayland

ICON_DIR="$HOME/.local/share/icons/hicolor"
DESKTOP_DIR="$HOME/.local/share/applications"

echo "Installing MediaForge icons..."

# Create icon directories
mkdir -p "$ICON_DIR/32x32/apps"
mkdir -p "$ICON_DIR/128x128/apps"
mkdir -p "$ICON_DIR/256x256/apps"
mkdir -p "$ICON_DIR/512x512/apps"

# Copy icons
cp src-tauri/icons/32x32.png "$ICON_DIR/32x32/apps/mediaforge.png"
cp src-tauri/icons/128x128.png "$ICON_DIR/128x128/apps/mediaforge.png"
cp src-tauri/icons/256x256.png "$ICON_DIR/256x256/apps/mediaforge.png"
cp src-tauri/icons/icon.png "$ICON_DIR/512x512/apps/mediaforge.png"

# Update icon cache
if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t "$ICON_DIR" 2>/dev/null || true
fi

echo "✓ Icons installed to $ICON_DIR"

# Install desktop file (optional, for installed app)
if [ -f "src-tauri/mediaforge.desktop" ]; then
    mkdir -p "$DESKTOP_DIR"
    cp src-tauri/mediaforge.desktop "$DESKTOP_DIR/"
    echo "✓ Desktop file installed"
fi

echo ""
echo "Icon installation complete!"
echo "Restart your dev server: npm run tauri:dev"
