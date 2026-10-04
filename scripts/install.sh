#!/bin/bash

# Lale Compiler Installation Script
# Installs the lale compiler, language server, validator, and standard library
# to the user's LALE_HOME directory.
# Updates shell configuration files to add LALE_HOME to PATH and environment.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Detect project root (script is in scripts/ subdirectory)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo -e "${GREEN}Lale Compiler Installation${NC}"
echo "================================"
echo ""

# 1. Prompt for installation directory
echo "Where would you like to install Lale?"
read -p "Installation directory [~/.lale]: " INSTALL_DIR
INSTALL_DIR="${INSTALL_DIR:-$HOME/.lale}"

# Expand ~ to home directory
INSTALL_DIR="${INSTALL_DIR/#\~/$HOME}"

# Make absolute path
if [[ ! "$INSTALL_DIR" = /* ]]; then
  INSTALL_DIR="$PWD/$INSTALL_DIR"
fi

echo -e "${YELLOW}Installing to: $INSTALL_DIR${NC}"
echo ""

# 2. Create LALE_HOME environment variable
LALE_HOME="$INSTALL_DIR"
export LALE_HOME

# 3. Build the release binary
echo -e "${YELLOW}Step 1: Building Lale compiler and tooling (release mode)...${NC}"
cd "$PROJECT_ROOT"
cargo build --release 2>&1 | grep -E "Compiling|Finished|error" || true

for bin in lale lale-lsp lale-validate; do
  if [ ! -f "target/release/$bin" ]; then
    echo -e "${RED}ERROR: Build failed. Could not find target/release/$bin${NC}"
    exit 1
  fi
done
echo -e "${GREEN}✓ Build complete${NC}"
echo ""

# 4. Create directory structure
echo -e "${YELLOW}Step 2: Creating directory structure...${NC}"
mkdir -p "$LALE_HOME/bin"
mkdir -p "$LALE_HOME/lib/std/src"
echo -e "${GREEN}✓ Directories created${NC}"
echo ""

# 5. Copy binaries
echo -e "${YELLOW}Step 3: Installing binaries...${NC}"
cp "target/release/lale" "$LALE_HOME/bin/lale"
cp "target/release/lale-lsp" "$LALE_HOME/bin/lale-lsp"
cp "target/release/lale-validate" "$LALE_HOME/bin/lale-validate"
chmod +x "$LALE_HOME/bin/lale" "$LALE_HOME/bin/lale-lsp" "$LALE_HOME/bin/lale-validate"
echo -e "${GREEN}✓ Binaries installed to $LALE_HOME/bin/{lale, lale-lsp, lale-validate}${NC}"
echo ""

# 6. Copy stdlib sources
echo -e "${YELLOW}Step 4: Installing standard library sources...${NC}"
cp "stdlib/src"/*.lale "$LALE_HOME/lib/std/src/"
echo -e "${GREEN}✓ Stdlib sources installed${NC}"
echo ""

# 7. Update shell configuration files
echo -e "${YELLOW}Step 5: Updating shell configuration...${NC}"

# Detect shell config files
SHELL_CONFIGS=()

# Check for bash
if [ -f "$HOME/.bashrc" ]; then
  SHELL_CONFIGS+=("$HOME/.bashrc")
fi

# Check for zsh
if [ -f "$HOME/.zshrc" ]; then
  SHELL_CONFIGS+=("$HOME/.zshrc")
fi

# Check for fish
if [ -f "$HOME/.config/fish/config.fish" ]; then
  SHELL_CONFIGS+=("$HOME/.config/fish/config.fish")
fi

# Check for ksh
if [ -f "$HOME/.kshrc" ]; then
  SHELL_CONFIGS+=("$HOME/.kshrc")
fi

if [ ${#SHELL_CONFIGS[@]} -eq 0 ]; then
  echo -e "${RED}WARNING: Could not find shell configuration files${NC}"
  echo "Please manually add the following to your shell config:"
  echo ""
  echo "export LALE_HOME=\"$LALE_HOME\""
  echo "export PATH=\"\$LALE_HOME/bin:\$PATH\""
  echo ""
else
  # Add environment setup to each config file
  for config_file in "${SHELL_CONFIGS[@]}"; do
    shell_name=$(basename "$config_file")

    # Check if already configured
    if grep -q "LALE_HOME" "$config_file"; then
      echo -e "${YELLOW}  - $shell_name: Already configured${NC}"
      continue
    fi

    # Determine comment character based on shell
    if [ "$shell_name" = "config.fish" ]; then
      # Fish has different syntax
      cat >> "$config_file" << EOF

# Lale Compiler Configuration
set -gx LALE_HOME "$LALE_HOME"
set -gx PATH "\$LALE_HOME/bin" \$PATH
EOF
    else
      # Bash, Zsh, Ksh use same syntax
      cat >> "$config_file" << EOF

# Lale Compiler Configuration
export LALE_HOME="$LALE_HOME"
export PATH="\$LALE_HOME/bin:\$PATH"
EOF
    fi

    echo -e "${GREEN}  ✓ $shell_name updated${NC}"
  done
fi

echo ""
echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}Installation complete!${NC}"
echo -e "${GREEN}================================${NC}"
echo ""
echo "To start using Lale, please run one of the following:"
echo ""
echo "  # For bash:"
echo "  source ~/.bashrc"
echo ""
echo "  # For zsh:"
echo "  source ~/.zshrc"
echo ""
echo "  # For fish:"
echo "  source ~/.config/fish/config.fish"
echo ""
echo "Then verify installation with:"
echo "  lale --version"
echo "  lale-validate --version"
echo ""
echo -e "${YELLOW}LALE_HOME is set to: $LALE_HOME${NC}"
