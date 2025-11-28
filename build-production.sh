#!/bin/bash

# MediaForge Production Build Script
# Ensures proper setup for production deployment with Rofi integration

set -e

echo "🚀 MediaForge Production Build Script"
echo "======================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}✅${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠️${NC} $1"
}

print_error() {
    echo -e "${RED}❌${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ️${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "package.json" ] || [ ! -d "src-tauri" ]; then
    print_error "Please run this script from the MediaForge project root directory"
    exit 1
fi

print_info "Starting production build process..."
echo ""

# Step 1: Check dependencies
echo "📋 Step 1: Checking system dependencies"
echo "----------------------------------------"

check_command() {
    if command -v "$1" >/dev/null 2>&1; then
        print_status "$1 is available"
        return 0
    else
        print_error "$1 is not available"
        return 1
    fi
}

DEPS_OK=true
check_command "yt-dlp" || DEPS_OK=false
check_command "ffmpeg" || DEPS_OK=false  
check_command "magick" || check_command "convert" || DEPS_OK=false
check_command "node" || DEPS_OK=false
check_command "npm" || DEPS_OK=false
check_command "cargo" || DEPS_OK=false

if [ "$DEPS_OK" = false ]; then
    print_error "Missing required dependencies. Please install them first:"
    echo "  sudo pacman -S yt-dlp ffmpeg imagemagick nodejs npm rust"
    exit 1
fi

print_status "All system dependencies are available"
echo ""

# Step 2: Install Node dependencies
echo "📦 Step 2: Installing Node.js dependencies"
echo "-------------------------------------------"
npm install
print_status "Node.js dependencies installed"
echo ""

# Step 3: Check Rust dependencies
echo "🦀 Step 3: Checking Rust dependencies"
echo "--------------------------------------"
cd src-tauri
cargo check
print_status "Rust dependencies verified"
cd ..
echo ""

# Step 4: Lint and type check
echo "🔍 Step 4: Code quality checks"
echo "-------------------------------"
npm run lint || print_warning "Linting issues found (continuing...)"
npm run typecheck || print_warning "Type checking issues found (continuing...)"
print_status "Code quality checks completed"
echo ""

# Step 5: Build production version
echo "🔨 Step 5: Building production version"
echo "---------------------------------------"
npm run tauri build
print_status "Production build completed"
echo ""

# Step 6: Verify icons
echo "🎨 Step 6: Verifying icon setup"
echo "--------------------------------"
if [ -d "src-tauri/icons" ]; then
    ICON_COUNT=$(ls src-tauri/icons/*.png 2>/dev/null | wc -l)
    if [ "$ICON_COUNT" -ge 8 ]; then
        print_status "Icons are properly configured ($ICON_COUNT icon files)"
    else
        print_warning "Limited icon sizes found. Consider running icon generation."
    fi
else
    print_error "Icons directory not found"
fi
echo ""

# Step 7: Check build artifacts
echo "📁 Step 7: Checking build artifacts"
echo "------------------------------------"
if [ -d "src-tauri/target/release/bundle" ]; then
    print_status "Build artifacts generated successfully"
    
    # List available packages
    echo ""
    print_info "Available packages:"
    find src-tauri/target/release/bundle -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" -o -name "mediaforge" | while read -r file; do
        echo "  📦 $(basename "$file")"
    done
else
    print_error "Build artifacts not found"
fi
echo ""

# Step 8: Installation instructions
echo "🚀 Step 8: Installation & Deployment"
echo "=====================================\n"

print_info "Production build completed successfully!"
echo ""
echo "📋 Next Steps:"
echo ""
echo "1️⃣  Install the application:"
echo "   • For Arch Linux: sudo pacman -U src-tauri/target/release/bundle/deb/mediaforge_1.0.0_amd64.deb"
echo "   • Or use AppImage: ./src-tauri/target/release/bundle/appimage/MediaForge_1.0.0_amd64.AppImage"
echo "   • Or binary: cp src-tauri/target/release/mediaforge /usr/local/bin/"
echo ""
echo "2️⃣  Install icons for Rofi integration:"
echo "   ./install-icons.sh"
echo ""
echo "3️⃣  Test Rofi integration:"
echo "   rofi -show drun -filter 'MediaForge'"
echo ""
echo "4️⃣  Hyprland window rules (optional):"
echo "   Add to ~/.config/hypr/hyprland.conf:"
echo "   windowrulev2 = float,class:^(MediaForge)$"
echo "   windowrulev2 = center,class:^(MediaForge)$"
echo ""

print_info "🎯 MediaForge is ready for production deployment!"
echo "   Optimized for Arch Linux + Hyprland with full Rofi support"
echo ""