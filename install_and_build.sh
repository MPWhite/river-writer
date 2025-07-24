#!/bin/bash

echo "River Text Editor - Setup Script"
echo "================================"

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Rust is not installed. Installing Rust..."
    
    # Install Rust using rustup
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    
    # Source the cargo environment
    source "$HOME/.cargo/env"
    
    echo "Rust installed successfully!"
else
    echo "Rust is already installed."
fi

# Build the project
echo ""
echo "Building River..."
cargo build --release

if [ $? -eq 0 ]; then
    echo ""
    echo "Build successful!"
    echo ""
    echo "To run River:"
    echo "  ./target/release/river [filename]"
    echo ""
    echo "To install River system-wide (requires sudo):"
    echo "  sudo cp ./target/release/river /usr/local/bin/"
    echo ""
    echo "Key bindings:"
    echo "  Ctrl+Q - Quit"
    echo "  Ctrl+S - Save"
    echo "  Arrow keys - Navigate"
    echo "  Home/End - Beginning/end of line"
    echo "  Page Up/Down - Scroll"
else
    echo "Build failed. Please check the error messages above."
fi