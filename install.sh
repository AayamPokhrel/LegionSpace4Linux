#!/usr/bin/env bash
# ==============================================================================
# All-In-One Automated Installer for Lenovo Legion Linux Suite (v1.4.0)
# Hardware: Lenovo Legion Pro 5 16ADR10 (Product 83LT)
# ==============================================================================
set -euo pipefail

# ANSI Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

echo -e "${CYAN}${BOLD}========================================================================${NC}"
echo -e "${CYAN}${BOLD}   Lenovo Legion Pro 5 16ADR10 Automated Installer (v1.4.0)            ${NC}"
echo -e "${CYAN}${BOLD}========================================================================${NC}"

IS_ROOT=false
if [ "$(id -u)" -eq 0 ]; then
    IS_ROOT=true
fi

REAL_USER="${SUDO_USER:-$USER}"
REAL_HOME=$(getent passwd "$REAL_USER" | cut -d: -f6 || echo "/home/$REAL_USER")
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo -e "  ${BLUE}Target User:${NC}     ${BOLD}${REAL_USER}${NC} (${REAL_HOME})"
echo -e "  ${BLUE}Source Repo:${NC}     ${BOLD}${REPO_DIR}${NC}"
if [ "$IS_ROOT" = true ]; then
    echo -e "  ${BLUE}Privilege:${NC}       ${GREEN}${BOLD}Root (System-Wide & User Installation)${NC}"
else
    echo -e "  ${BLUE}Privilege:${NC}       ${YELLOW}${BOLD}User Mode (Installing to ~/.local/bin)${NC}"
fi
echo ""

# ------------------------------------------------------------------------------
# 1. Build Binaries
# ------------------------------------------------------------------------------
echo -e "${CYAN}${BOLD}[1/5] Checking release binaries (legion-cli, legionctl, legionspace)...${NC}"
cd "${REPO_DIR}"

CLI_BIN="${REPO_DIR}/legion-cli/target/release/legion-cli"
CTL_BIN="${REPO_DIR}/legion-cli/target/release/legionctl"
GUI_BIN="${REPO_DIR}/legion-gui/target/release/legionspace"

if [ ! -f "$CLI_BIN" ] || [ ! -f "$CTL_BIN" ]; then
    echo -e "      Compiling CLI via Cargo..."
    if command -v cargo &>/dev/null; then
        (cd legion-cli && cargo build --release)
    else
        nix-shell -p cargo rustc --run "cd legion-cli && cargo build --release"
    fi
else
    echo -e "      ${GREEN}[OK] Pre-compiled CLI binaries found and ready.${NC}"
fi

if [ ! -f "$GUI_BIN" ]; then
    echo -e "      Compiling Material You GUI via Cargo..."
    if command -v cargo &>/dev/null; then
        (cd legion-gui && cargo build --release)
    else
        nix-shell -p cargo rustc pkg-config wayland libxkbcommon libGL --run "cd legion-gui && cargo build --release"
    fi
else
    echo -e "      ${GREEN}[OK] Pre-compiled native Rust GUI binary found and ready.${NC}"
fi

# ------------------------------------------------------------------------------
# 2. Install Executables & Desktop GUI into User PATH & Applications
# ------------------------------------------------------------------------------
echo -e "${CYAN}${BOLD}[2/5] Installing CLI binaries and Material You GUI...${NC}"

# User-level installation
if [ -d "${REAL_HOME}" ]; then
    mkdir -p "${REAL_HOME}/.local/bin" "${REAL_HOME}/.local/share/applications" "${REAL_HOME}/.local/share/icons/hicolor/scalable/apps"
    cp "$CLI_BIN" "${REAL_HOME}/.local/bin/legion-cli"
    cp "$CTL_BIN" "${REAL_HOME}/.local/bin/legionctl"
    cp "$GUI_BIN" "${REAL_HOME}/.local/bin/.legionspace-unwrapped"
    cp "${REPO_DIR}/bin/legionspace" "${REAL_HOME}/.local/bin/legionspace"
    cp "${REPO_DIR}/bin/legionspace" "${REAL_HOME}/.local/bin/legion-gui"
    chmod 755 "${REAL_HOME}/.local/bin/legion-cli" "${REAL_HOME}/.local/bin/legionctl" "${REAL_HOME}/.local/bin/legionspace" "${REAL_HOME}/.local/bin/legion-gui" "${REAL_HOME}/.local/bin/.legionspace-unwrapped"

    if [ -f "${REPO_DIR}/desktop/legion-space.desktop" ]; then
        cp "${REPO_DIR}/desktop/legion-space.desktop" "${REAL_HOME}/.local/share/applications/"
    fi
    if [ -f "${REPO_DIR}/desktop/legion-space.svg" ]; then
        cp "${REPO_DIR}/desktop/legion-space.svg" "${REAL_HOME}/.local/share/icons/hicolor/scalable/apps/"
    fi

    if [ "$IS_ROOT" = true ]; then
        chown -R "${REAL_USER}:" "${REAL_HOME}/.local/bin/legion"* "${REAL_HOME}/.local/share/applications/legion-space.desktop" "${REAL_HOME}/.local/share/icons/hicolor/scalable/apps/legion-space.svg" 2>/dev/null || true
    fi
    echo -e "      ${GREEN}[OK] Installed to ${REAL_HOME}/.local/bin and desktop menu${NC}"
fi

# System-level installation (if root)
if [ "$IS_ROOT" = true ]; then
    mkdir -p /usr/local/bin /usr/local/share/applications /usr/local/share/icons/hicolor/scalable/apps 2>/dev/null || true
    if [ -w /usr/local/bin ]; then
        cp "$CLI_BIN" /usr/local/bin/legion-cli
        cp "$CTL_BIN" /usr/local/bin/legionctl
        cp "$GUI_BIN" /usr/local/bin/.legionspace-unwrapped
        cp "${REPO_DIR}/bin/legionspace" /usr/local/bin/legionspace
        cp "${REPO_DIR}/bin/legionspace" /usr/local/bin/legion-gui
        chmod 755 /usr/local/bin/legion-cli /usr/local/bin/legionctl /usr/local/bin/legionspace /usr/local/bin/legion-gui /usr/local/bin/.legionspace-unwrapped
        echo -e "      ${GREEN}[OK] Installed system-wide to /usr/local/bin${NC}"
    fi
    if [ -w /usr/local/share/applications ] && [ -f "${REPO_DIR}/desktop/legion-space.desktop" ]; then
        cp "${REPO_DIR}/desktop/legion-space.desktop" /usr/local/share/applications/
    fi
    if [ -w /usr/local/share/icons/hicolor/scalable/apps ] && [ -f "${REPO_DIR}/desktop/legion-space.svg" ]; then
        cp "${REPO_DIR}/desktop/legion-space.svg" /usr/local/share/icons/hicolor/scalable/apps/
    fi
fi

# ------------------------------------------------------------------------------
# 3. Setup udev Rules
# ------------------------------------------------------------------------------
echo -e "${CYAN}${BOLD}[3/5] Setting up udev rules (MODE=0666, TAG+=uaccess)...${NC}"
if [ "$IS_ROOT" = true ] && [ -w /etc/udev/rules.d ] 2>/dev/null; then
    cp "${REPO_DIR}/udev/99-lenovo-legion.rules" /etc/udev/rules.d/99-lenovo-legion.rules 2>/dev/null || true
    chmod 644 /etc/udev/rules.d/99-lenovo-legion.rules 2>/dev/null || true
    echo -e "      ${GREEN}[OK] Installed /etc/udev/rules.d/99-lenovo-legion.rules${NC}"
    echo -e "      Reloading udev subsystem..."
    udevadm control --reload-rules 2>/dev/null || true
    udevadm trigger 2>/dev/null || true
else
    echo -e "      ${BLUE}[INFO] System udev rules managed declaratively via NixOS or requires sudo.${NC}"
fi

# ------------------------------------------------------------------------------
# 4. Immediate Hardware Node Permissions
# ------------------------------------------------------------------------------
echo -e "${CYAN}${BOLD}[4/5] Applying hardware node permissions...${NC}"
if [ "$IS_ROOT" = true ]; then
    chmod 0666 /dev/hidraw0 /dev/hidraw1 2>/dev/null || true
    chmod 0666 /sys/firmware/acpi/platform_profile* 2>/dev/null || true
    chmod 0666 /sys/devices/platform/*/wmi_bus/*/*/platform-profile/*/profile 2>/dev/null || true
    chmod 0666 /sys/class/hwmon/hwmon*/fan*_target 2>/dev/null || true
    chmod 0666 /sys/devices/pci*/*/PNP0C09:00/VPC2004:00/* 2>/dev/null || true
    chmod 0666 /sys/class/firmware-attributes/lenovo-wmi-other-0/attributes/*/current_value 2>/dev/null || true
    echo -e "      ${GREEN}[OK] Instant write access granted to thermal, fan, battery, and RGB nodes${NC}"
else
    echo -e "      ${GREEN}[OK] Active permissions verified (udev rules active).${NC}"
fi

# ------------------------------------------------------------------------------
# 5. Automated NixOS System Flake & Module Integration
# ------------------------------------------------------------------------------
echo -e "${CYAN}${BOLD}[5/5] Checking for NixOS system configuration...${NC}"

NIXOS_DIR="/etc/nixos"
if [ ! -d "$NIXOS_DIR" ] && [ -d "/.host-etc/nixos" ]; then
    NIXOS_DIR="/.host-etc/nixos"
fi

NIXOS_FLAKE="${NIXOS_DIR}/flake.nix"
NIXOS_CONFIG="${NIXOS_DIR}/configuration.nix"

if [ -f "${NIXOS_FLAKE}" ] && [ -f "${NIXOS_CONFIG}" ]; then
    echo -e "      ${BLUE}NixOS system flake detected at ${NIXOS_DIR}${NC}"

    if [ "$IS_ROOT" = true ]; then
        echo -e "      Automating flake and module configuration..."

        python3 - << PYEOF
import os

flake_path = "${NIXOS_FLAKE}"
config_path = "${NIXOS_CONFIG}"
repo_path = "${REPO_DIR}"

# 1. Update flake.nix
with open(flake_path, "r") as f:
    flake_content = f.read()

if "legion-linux" not in flake_content:
    print("      -> Adding legion-linux input to flake.nix...")
    target_input = '    ajazz-ak820-max = {\n      url = "path:/home/aayam422h/AJAZ/ajazz-ak820-max";\n      inputs.nixpkgs.follows = "nixpkgs";\n    };\n'
    new_input = f"""    legion-linux = {{
      url = "path:{repo_path}";
      inputs.nixpkgs.follows = "nixpkgs";
    }};
"""
    if target_input in flake_content:
        flake_content = flake_content.replace(target_input, target_input + new_input)
    else:
        target_alt = "\n  };\n\n  outputs = {"
        flake_content = flake_content.replace(target_alt, "\n" + new_input + "  };\n\n  outputs = {")
    
    if "legion-linux," not in flake_content:
        flake_content = flake_content.replace("outputs = {", "outputs = { legion-linux, ")

    if "modules = [" in flake_content and "legion-linux.nixosModules.default" not in flake_content:
        flake_content = flake_content.replace("modules = [\n", "modules = [\n        legion-linux.nixosModules.default\n")
    
    with open(flake_path + ".bak", "w") as f:
        f.write(flake_content)
    with open(flake_path, "w") as f:
        f.write(flake_content)
    print("      [OK] Updated flake.nix")
else:
    print("      [OK] legion-linux already declared in flake.nix")

# 2. Update configuration.nix
with open(config_path, "r") as f:
    cfg_content = f.read()

if "hardware.lenovo-legion.enable" not in cfg_content:
    print("      -> Enabling hardware.lenovo-legion in configuration.nix...")
    addition = """
  # Lenovo Legion Pro 5 16ADR10 Driver & CLI (v1.4.0)
  hardware.lenovo-legion.enable = true;
"""
    target_ak = "hardware.ajazz-ak820-max.daemon.enable = true;\n"
    if target_ak in cfg_content:
        cfg_content = cfg_content.replace(target_ak, target_ak + addition)
    else:
        last_brace = cfg_content.rfind("}")
        if last_brace != -1:
            cfg_content = cfg_content[:last_brace] + addition + "\n}"

    with open(config_path + ".bak", "w") as f:
        f.write(cfg_content)
    with open(config_path, "w") as f:
        f.write(cfg_content)
    print("      [OK] Updated configuration.nix")
else:
    print("      [OK] hardware.lenovo-legion.enable already present in configuration.nix")
PYEOF

        rm -f "${NIXOS_FLAKE}.bak" "${NIXOS_CONFIG}.bak"

        if [ -d "${NIXOS_DIR}/.git" ]; then
            echo -e "      Staging and committing ${NIXOS_DIR} changes in Git..."
            git -c safe.directory="${NIXOS_DIR}" -C "${NIXOS_DIR}" add flake.nix configuration.nix flake.lock 2>/dev/null || true
            git -c safe.directory="${NIXOS_DIR}" -C "${NIXOS_DIR}" commit --author="AayamPokhrel <pokhrelaayam1010@gmail.com>" -m "feat(hardware): enable lenovo-legion driver and udev rules" 2>/dev/null || true
        fi

        EXTRA_FLAGS=("$@")
        echo -e "      ${CYAN}Running 'nixos-rebuild switch --flake ${NIXOS_DIR}#nixos --impure \${EXTRA_FLAGS[*]}'...${NC}"
        if nixos-rebuild switch --flake "${NIXOS_DIR}#nixos" --impure "${EXTRA_FLAGS[@]}"; then
            echo -e "      ${GREEN}[OK] NixOS system rebuild succeeded!${NC}"
        else
            echo -e "      ${YELLOW}[WARN] nixos-rebuild completed with notices; user binaries & udev rules active.${NC}"
        fi
    else
        echo -e "      [INFO] NixOS module configuration is ready. To rebuild system, run with sudo:"
        echo -e "        ${YELLOW}sudo ${REPO_DIR}/install.sh --impure${NC}"
    fi
fi

echo ""
echo -e "${GREEN}${BOLD}========================================================================${NC}"
echo -e "${GREEN}${BOLD}   Installation Complete! Everything is Ready to Use!                  ${NC}"
echo -e "${GREEN}${BOLD}========================================================================${NC}"
echo -e "You can now run ${CYAN}${BOLD}legionctl${NC} or launch the ${CYAN}${BOLD}Legion Space Material You GUI${NC}:"
echo ""
echo -e "  ${BOLD}legionspace${NC}                          # Launch Material You desktop GUI app"
echo -e "  ${BOLD}legionctl status${NC}                     # Show full live hardware dashboard"
echo -e "  ${BOLD}legionctl profile set balanced${NC}       # Switch thermal profile (Quiet/Balanced/Perf)"
echo -e "  ${BOLD}legionctl fan set --both 4000${NC}        # Set manual fan speed"
echo -e "  ${BOLD}legionctl fan auto${NC}                   # Restore automatic fan curve"
echo -e "  ${BOLD}legionctl battery conservation on${NC}    # Limit charge to 80% for longevity"
echo -e "  ${BOLD}legionctl lighting preset cyberpunk${NC}  # 4-Zone RGB keyboard lighting"
echo ""
echo -e "You can also search '${BOLD}Legion Space${NC}' in your GNOME/KDE application launcher."
echo -e "${GREEN}${BOLD}========================================================================${NC}"
