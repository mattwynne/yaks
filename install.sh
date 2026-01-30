#!/usr/bin/env bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Installing yx (yaks CLI)..."

# Determine install location
if [ -w "/usr/local/bin" ]; then
    BIN_DIR="/usr/local/bin"
elif [ -d "$HOME/.local/bin" ]; then
    BIN_DIR="$HOME/.local/bin"
else
    mkdir -p "$HOME/.local/bin"
    BIN_DIR="$HOME/.local/bin"
fi

# Detect user's shell
DETECTED_SHELL=""
if [[ "$SHELL" == *"zsh"* ]]; then
    DETECTED_SHELL="zsh"
elif [[ "$SHELL" == *"bash"* ]]; then
    DETECTED_SHELL="bash"
else
    DETECTED_SHELL="bash"  # Default to bash
fi

# Prompt user to confirm or choose shell
echo ""
echo "Detected shell: $DETECTED_SHELL"
echo "Install completions for:"
echo "  1) zsh"
echo "  2) bash"
if [ "$DETECTED_SHELL" = "zsh" ]; then
    DEFAULT_CHOICE="1"
else
    DEFAULT_CHOICE="2"
fi
if [ -n "$YX_SHELL_CHOICE" ]; then
    SHELL_CHOICE="$YX_SHELL_CHOICE"
else
    read -p "Choice [$DEFAULT_CHOICE]: " SHELL_CHOICE </dev/tty
    SHELL_CHOICE="${SHELL_CHOICE:-$DEFAULT_CHOICE}"
fi

if [ "$SHELL_CHOICE" = "1" ]; then
    INSTALL_SHELL="zsh"
else
    INSTALL_SHELL="bash"
fi

# Determine completion location based on shell
if [ "$INSTALL_SHELL" = "zsh" ]; then
    COMPLETION_DIR="$HOME/.zsh/completions"
    mkdir -p "$COMPLETION_DIR"
    COMPLETION_FILE="yx.zsh"
    SHELL_CONFIG="$HOME/.zshrc"
else
    if [ -d "/usr/local/etc/bash_completion.d" ] && [ -w "/usr/local/etc/bash_completion.d" ]; then
        COMPLETION_DIR="/usr/local/etc/bash_completion.d"
    else
        COMPLETION_DIR="$HOME/.bash_completion.d"
        mkdir -p "$COMPLETION_DIR"
    fi
    COMPLETION_FILE="yx.bash"
    SHELL_CONFIG="$HOME/.bashrc"
fi

# Download or copy files
if [ -f "bin/yx" ]; then
    # Running from repo
    echo "Installing from local repository..."
    cp bin/yx "$BIN_DIR/yx"
    chmod +x "$BIN_DIR/yx"
    cp "completions/$COMPLETION_FILE" "$COMPLETION_DIR/yx"
else
    # Download from GitHub
    echo "Downloading from GitHub..."
    REPO_URL="https://raw.githubusercontent.com/mattwynne/yaks/main"
    curl -fsSL "$REPO_URL/bin/yx" -o "$BIN_DIR/yx"
    chmod +x "$BIN_DIR/yx"
    curl -fsSL "$REPO_URL/completions/$COMPLETION_FILE" -o "$COMPLETION_DIR/yx"
fi

echo -e "${GREEN}✓${NC} Installed yx to $BIN_DIR/yx"
echo -e "${GREEN}✓${NC} Installed completion to $COMPLETION_DIR/yx"

# Check if completion is already sourced
if [ -f "$SHELL_CONFIG" ]; then
    if ! grep -q "source.*completions.*yx\|source.*yx" "$SHELL_CONFIG" 2>/dev/null; then
        echo ""
        echo -e "${YELLOW}To enable tab completion, add this to $SHELL_CONFIG:${NC}"
        echo ""
        echo "    source $COMPLETION_DIR/yx"
        echo ""
        if [ -n "$YX_AUTO_COMPLETE" ]; then
            REPLY="$YX_AUTO_COMPLETE"
        else
            read -p "Add it now? [y/N] " -n 1 -r </dev/tty
        fi
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo "" >> "$SHELL_CONFIG"
            echo "# yx completion" >> "$SHELL_CONFIG"
            echo "source $COMPLETION_DIR/yx" >> "$SHELL_CONFIG"
            echo -e "${GREEN}✓${NC} Added completion to $SHELL_CONFIG"
            echo "Restart your shell or run: source $SHELL_CONFIG"
        fi
    fi
fi

# Check PATH
if ! echo "$PATH" | grep -q "$BIN_DIR"; then
    echo ""
    echo -e "${YELLOW}Warning: $BIN_DIR is not in your PATH${NC}"
    echo "Add it to your shell config:"
    echo "    export PATH=\"$BIN_DIR:\$PATH\""
fi

echo ""
echo -e "${GREEN}Installation complete!${NC}"
echo "Try: yx --help"
