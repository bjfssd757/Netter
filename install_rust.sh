#!/bin/bash

echo "======================================================="
echo "Install Rust and Cargo"
echo "======================================================="

if ! command -v curl &> /dev/null; then
    echo "Error: curl not installed. Please install curl."
    exit 1
fi

echo "Downloading and installing rustup..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

source "$HOME/.cargo/env"

if command -v rustc &> /dev/null && command -v cargo &> /dev/null; then
    echo "======================================================="
    echo "Install Rust and Cargo successfully!"
    echo "======================================================="
    echo
    echo "Rust version:"
    rustc --version
    echo
    echo "Cargo version:"
    cargo --version
    echo
    echo "Path to Rust add to PATH env to current session"
    echo "Add setting in ~/.profile for Rust."
else
    echo "======================================================="
    echo "Error while installing Rust и Cargo"
    echo "======================================================="
    exit 1
fi