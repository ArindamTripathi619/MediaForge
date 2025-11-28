#!/bin/bash

# MediaForge Setup Script for Arch Linux + Hyprland
# This script prepares your system to build and run MediaForge

set -e

echo "==================================="
echo "MediaForge Setup Script"
echo "==================================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running on Arch Linux
if [ ! -f /etc/arch-release ]; then
    echo -e "${RED}Error: This script is designed for Arch Linux${NC}"
    exit 1
fi

echo "Step 1: Checking disk space..."
AVAILABLE=$(df -BG /home | tail -1 | awk '{print $4}' | sed 's/G//')
if [ "$AVAILABLE" -lt 10 ]; then
    echo -e "${RED}Warning: Less than 10GB available on /home${NC}"
    echo "Cleaning up..."
    
    # Clean package cache
    echo "Cleaning package cache..."
    sudo pacman -Sc --noconfirm
    
    # Clean cargo
    if [ -d "$HOME/.cargo/registry/cache" ]; then
        echo "Cleaning cargo cache..."
        rm -rf ~/.cargo/registry/cache/*
    fi
    
    # Clean MediaForge build
    if [ -d "src-tauri/target" ]; then
        echo "Cleaning MediaForge build artifacts..."
        cargo clean --manifest-path=src-tauri/Cargo.toml
    fi
else
    echo -e "${GREEN}Disk space OK${NC}"
fi

echo ""
echo "Step 2: Checking system dependencies..."

# Check and install dependencies
DEPS=(
    "base-devel"
    "curl"
    "wget"
    "file"
    "openssl"
    "gtk3"
    "webkit2gtk"
    "librsvg"
    "yt-dlp"
    "ffmpeg"
    "imagemagick"
)

MISSING=()
for dep in "${DEPS[@]}"; do
    if ! pacman -Qi "$dep" &> /dev/null; then
        MISSING+=("$dep")
    fi
done

if [ ${#MISSING[@]} -gt 0 ]; then
    echo -e "${YELLOW}Missing packages: ${MISSING[*]}${NC}"
    echo "Installing missing packages..."
    sudo pacman -S --needed --noconfirm "${MISSING[@]}"
else
    echo -e "${GREEN}All system dependencies installed${NC}"
fi

echo ""
echo "Step 3: Checking Rust..."
if ! command -v rustc &> /dev/null; then
    echo -e "${YELLOW}Rust not found. Installing...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo -e "${GREEN}Rust installed: $(rustc --version)${NC}"
fi

echo ""
echo "Step 4: Checking Node.js..."
if ! command -v node &> /dev/null; then
    echo -e "${YELLOW}Node.js not found. Installing...${NC}"
    sudo pacman -S --needed --noconfirm nodejs npm
else
    echo -e "${GREEN}Node.js installed: $(node --version)${NC}"
fi

echo ""
echo "Step 5: Checking media tools..."

# Check yt-dlp
if ! command -v yt-dlp &> /dev/null; then
    echo -e "${RED}yt-dlp not found${NC}"
    exit 1
else
    echo -e "${GREEN}yt-dlp: $(which yt-dlp)${NC}"
fi

# Check ffmpeg
if ! command -v ffmpeg &> /dev/null; then
    echo -e "${RED}ffmpeg not found${NC}"
    exit 1
else
    echo -e "${GREEN}ffmpeg: $(which ffmpeg)${NC}"
fi

# Check ImageMagick
if ! command -v convert &> /dev/null; then
    echo -e "${RED}ImageMagick not found${NC}"
    exit 1
else
    echo -e "${GREEN}ImageMagick: $(which convert)${NC}"
fi

echo ""
echo "Step 6: Installing Node dependencies..."
if [ -f "package.json" ]; then
    npm install
    echo -e "${GREEN}Node dependencies installed${NC}"
else
    echo -e "${RED}package.json not found. Are you in the MediaForge directory?${NC}"
    exit 1
fi

echo ""
echo "Step 7: Checking current disk space..."
df -h /home

echo ""
echo "==================================="
echo -e "${GREEN}Setup Complete!${NC}"
echo "==================================="
echo ""
echo "Next steps:"
echo "1. To run in development mode:"
echo "   npm run tauri:dev"
echo ""
echo "2. To build for production:"
echo "   npm run tauri:build"
echo ""
echo "3. The built app will be in:"
echo "   src-tauri/target/release/bundle/"
echo ""
echo "==================================="
