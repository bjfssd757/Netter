#!/usr/bin/env bash

error(){ 
	printf '%s' "$*"
	exit 1 
}

if command -v rustc &> /dev/null && command -v cargo &> /dev/null; then
    echo "======================================================="
    echo "Rust and Cargo are already installed"
    echo "======================================================="
    printf "The rust version: $(rustc -V)\n" 
    printf "The cargo version: $(cargo -V)\n"
    echo "======================================================="
    exit 0
else
    echo "======================================================="
    echo "Installation of Rust and Cargo"
    echo "======================================================="
    command -v curl &> /dev/null || error "Error: curl is not installed. Please install curl."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    echo "Sourcing the env file under $HOME./cargo/ directory"
    user_shell=$(awk -F: -v user="$USER" '$1 == user {print $NF}' /etc/passwd)
    [[ user_shell == "nushell" ]] && source "$HOME/.cargo/env.nu" && exit 0
    [[ user_shell == "fish" ]] && source "$HOME/.cargo/env.fish && exit 0
    source "$HOME/.cargo/env" && exit 0
fi
