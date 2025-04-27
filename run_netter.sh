#!/bin/bash

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
NETTER_PATH="$SCRIPT_DIR/target/release"
echo "Add $NETTER_PATH in PATH..."
export PATH="$PATH:$NETTER_PATH"

if ! grep -q "$NETTER_PATH" ~/.bashrc; then
    echo "export PATH=\"\$PATH:$NETTER_PATH\"" >> ~/.bashrc
    echo "Update PATH in ~/.bashrc"
fi

BUILD_DIR="$SCRIPT_DIR/build"
if [ ! -d "$BUILD_DIR" ]; then
    echo "Creating directory for build and building Netter..."
    mkdir -p "$BUILD_DIR"
    cd "$BUILD_DIR"
    cmake .. -G "Ninja"
    ninja
else
    echo "Directory found successfully"
fi

if [ -f "$BUILD_DIR/Netter" ]; then
    echo "Starting Netter..."
    "$BUILD_DIR/Netter"
else
    echo "Building Netter..."
    cd "$BUILD_DIR"
    cmake .. -G "Ninja"
    ninja
    
    if [ -f "$BUILD_DIR/Netter" ]; then
        echo "Starting Netter..."
        "$BUILD_DIR/Netter"
    else
        echo "Failed to build Netter."
    fi
fi