#!/bin/bash

# MediaForge Icon Installation Script
# Ensures proper icon installation for Rofi and desktop environment integration

set -e

echo "🎨 Installing MediaForge icons for system integration..."

# Define paths
ICON_SIZES=("16" "22" "24" "32" "48" "64" "128" "256" "512")
ICON_DIR="/usr/share/icons/hicolor"
LOCAL_ICON_DIR="$HOME/.local/share/icons/hicolor"
PIXMAPS_DIR="/usr/share/pixmaps"
LOCAL_PIXMAPS_DIR="$HOME/.local/share/pixmaps"

# Function to install icons
install_icon() {
    local size=$1
    local target_dir=$2
    local is_local=$3
    
    if [ -f "src-tauri/icons/${size}x${size}.png" ]; then
        if [ "$is_local" = true ]; then
            mkdir -p "$target_dir/${size}x${size}/apps"
            cp "src-tauri/icons/${size}x${size}.png" "$target_dir/${size}x${size}/apps/mediaforge.png"
            echo "  ✅ Installed ${size}x${size} icon to user directory"
        else
            sudo mkdir -p "$target_dir/${size}x${size}/apps"
            sudo cp "src-tauri/icons/${size}x${size}.png" "$target_dir/${size}x${size}/apps/mediaforge.png"
            echo "  ✅ Installed ${size}x${size} icon to system directory"
        fi
    elif [ -f "src-tauri/icons/icon.png" ]; then
        # Fallback to main icon if specific size not found
        if [ "$is_local" = true ]; then
            mkdir -p "$target_dir/${size}x${size}/apps"
            cp "src-tauri/icons/icon.png" "$target_dir/${size}x${size}/apps/mediaforge.png"
            echo "  ✅ Installed fallback ${size}x${size} icon to user directory"
        else
            sudo mkdir -p "$target_dir/${size}x${size}/apps"
            sudo cp "src-tauri/icons/icon.png" "$target_dir/${size}x${size}/apps/mediaforge.png"
            echo "  ✅ Installed fallback ${size}x${size} icon to system directory"
        fi
    fi
}

# Function to install desktop file
install_desktop_file() {
    local is_local=$1
    
    if [ "$is_local" = true ]; then
        mkdir -p "$HOME/.local/share/applications"
        cp "src-tauri/mediaforge.desktop" "$HOME/.local/share/applications/"
        echo "  ✅ Installed desktop file to user directory"
    else
        sudo cp "src-tauri/mediaforge.desktop" "/usr/share/applications/"
        echo "  ✅ Installed desktop file to system directory"
    fi
}

# Check if we're in the right directory
if [ ! -d "src-tauri/icons" ] || [ ! -f "src-tauri/mediaforge.desktop" ]; then
    echo "❌ Error: Please run this script from the project root directory"
    echo "   Expected files: src-tauri/icons/ directory and src-tauri/mediaforge.desktop"
    exit 1
fi

# Ask user preference
echo "Choose installation type:"
echo "1) System-wide (requires sudo) - Recommended for single-user systems"
echo "2) User-only - Better for multi-user systems or no sudo access"
echo "3) Both - Maximum compatibility"
read -p "Enter choice (1/2/3): " choice

case $choice in
    1)
        echo "📦 Installing icons system-wide..."
        # Install to system directories
        for size in "${ICON_SIZES[@]}"; do
            install_icon "$size" "$ICON_DIR" false
        done
        
        # Install main icon to pixmaps
        if [ -f "src-tauri/icons/icon.png" ]; then
            sudo cp "src-tauri/icons/icon.png" "$PIXMAPS_DIR/mediaforge.png"
            echo "  ✅ Installed main icon to pixmaps"
        fi
        
        install_desktop_file false
        
        # Update icon cache
        echo "🔄 Updating system icon cache..."
        sudo gtk-update-icon-cache -f -t "$ICON_DIR" 2>/dev/null || true
        ;;
        
    2)
        echo "👤 Installing icons for current user..."
        # Install to user directories
        for size in "${ICON_SIZES[@]}"; do
            install_icon "$size" "$LOCAL_ICON_DIR" true
        done
        
        # Install main icon to local pixmaps
        if [ -f "src-tauri/icons/icon.png" ]; then
            mkdir -p "$LOCAL_PIXMAPS_DIR"
            cp "src-tauri/icons/icon.png" "$LOCAL_PIXMAPS_DIR/mediaforge.png"
            echo "  ✅ Installed main icon to local pixmaps"
        fi
        
        install_desktop_file true
        
        # Update user icon cache
        echo "🔄 Updating user icon cache..."
        gtk-update-icon-cache -f -t "$LOCAL_ICON_DIR" 2>/dev/null || true
        ;;
        
    3)
        echo "🌐 Installing icons both system-wide and for user..."
        # System installation
        for size in "${ICON_SIZES[@]}"; do
            install_icon "$size" "$ICON_DIR" false
        done
        if [ -f "src-tauri/icons/icon.png" ]; then
            sudo cp "src-tauri/icons/icon.png" "$PIXMAPS_DIR/mediaforge.png"
        fi
        install_desktop_file false
        
        # User installation
        for size in "${ICON_SIZES[@]}"; do
            install_icon "$size" "$LOCAL_ICON_DIR" true
        done
        if [ -f "src-tauri/icons/icon.png" ]; then
            mkdir -p "$LOCAL_PIXMAPS_DIR"
            cp "src-tauri/icons/icon.png" "$LOCAL_PIXMAPS_DIR/mediaforge.png"
        fi
        install_desktop_file true
        
        # Update both caches
        echo "🔄 Updating icon caches..."
        sudo gtk-update-icon-cache -f -t "$ICON_DIR" 2>/dev/null || true
        gtk-update-icon-cache -f -t "$LOCAL_ICON_DIR" 2>/dev/null || true
        ;;
        
    *)
        echo "❌ Invalid choice. Exiting."
        exit 1
        ;;
esac

# Update desktop database
echo "📱 Updating desktop database..."
if command -v update-desktop-database >/dev/null 2>&1; then
    if [ "$choice" = "1" ] || [ "$choice" = "3" ]; then
        sudo update-desktop-database /usr/share/applications/ 2>/dev/null || true
    fi
    if [ "$choice" = "2" ] || [ "$choice" = "3" ]; then
        update-desktop-database "$HOME/.local/share/applications/" 2>/dev/null || true
    fi
fi

# Rofi-specific instructions
echo ""
echo "🚀 Installation complete!"
echo ""
echo "📋 For optimal Rofi integration:"
echo "   • Icons should now appear in Rofi when using icon themes"
echo "   • Desktop file is registered with proper MIME types"
echo "   • Multiple icon sizes ensure crisp display at any resolution"
echo ""
echo "🔧 Rofi Configuration Tips:"
echo "   • Add 'show-icons: true' to your Rofi config"
echo "   • Use 'drun' mode for application launching: rofi -show drun"
echo "   • Icons will scale properly in both fullscreen and tiled modes"
echo ""
echo "🎯 Hyprland Integration:"
echo "   • MediaForge will appear in application launchers"
echo "   • Proper window class for window rules: 'MediaForge'"
echo "   • Desktop actions available for quick mode switching"
echo ""
echo "✨ Test the installation:"
echo "   rofi -show drun -filter 'MediaForge'"
echo ""
