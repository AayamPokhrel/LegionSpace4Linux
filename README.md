# Legion Linux (`legion-cli` / `legionctl`)

Native Linux hardware driver and command-line control utility for the **Lenovo Legion Pro 5 16ADR10** (Product Code `83LT`, BIOS `RLCN33WW`).

Completely reverse-engineered from the vendor Windows package (`legionspace-1.9.11.5_x64_rtm.exe`) using the Reverse Engineering Anything (REA) methodology. Provides full hardware control over thermal profiles, cooling fans, battery conservation thresholds, firmware power envelopes, and 4-zone RGB keyboard lighting without requiring any proprietary Windows background services.

---

## Key Hardware Capabilities

| Subsystem | Linux Kernel Node / Protocol | Capabilities |
| :--- | :--- | :--- |
| **Thermal Profiles** | `/sys/firmware/acpi/platform_profile` via `lenovo_wmi_gamezone` | Quiet (Blue LED), Balanced (White LED), Performance (Red LED), Max-Power (Turbo), Custom (Purple LED). |
| **Cooling Fans** | `/sys/class/hwmon/hwmon5/` via `lenovo_wmi_other` | Independent CPU (`fan1`) & GPU (`fan2`) RPM telemetry, min/max constraints (1700–5100 RPM), manual target speeds, and automatic curve restore. |
| **Battery Conservation** | `/sys/devices/.../VPC2004:00/conservation_mode` | Capping charge threshold at 80% for battery longevity, status monitoring, battery health, and always-on USB charging. |
| **Custom Mode Tuning** | `/sys/class/firmware-attributes/lenovo-wmi-other-0/attributes/` | CPU Sustained Limit (PL1 / SPL), Short-Term (PL2 / SPPT), Peak (PL3 / FPPT), CPU Temp Limit, GPU Temp Limit, GPU TGP Offset. |
| **RGB Keyboard Lighting** | `/dev/hidrawX` direct USB HID Feature Report `0xCC` on ITE 8258 (`048d:c195`) | 4-Zone static colors, dynamic effects (Breath, Wave), speeds (1–4), brightness (1–3), and profile commit (`0x18`). |
| **ACPI Backlight & Hotkeys**| `/sys/devices/.../VPC2004:00/leds/platform::kbd_backlight/` | 3-level white keyboard backlight (`0`=Off, `1`=Low, `2`=High) and Fn-Lock toggle. |

---

## Installation & Setup

### Option 1: Standard Build & Install (Makefile)

Requirements: Rust toolchain (`cargo`, `rustc`).

```bash
# Build release binaries (target/release/legion-cli and legionctl)
make build

# Install binaries to /usr/local/bin and udev rules to /etc/udev/rules.d/
sudo make install
```

Or for user-only installation without root:
```bash
make install-user   # Installs to ~/.local/bin
```

### Option 2: Nix Flake & NixOS Integration

The repository includes complete Nix flake and NixOS module support:

#### Running directly with `nix run`:
```bash
# Run status probe
nix run .# -- status

# Run hardware probe
nix run .# -- probe
```

#### NixOS Configuration (`configuration.nix`):
Add the flake to your inputs and import the module:

```nix
{ inputs, ... }:
{
  imports = [
    inputs.legion-linux.nixosModules.default
  ];

  # Enables legion-cli package and installs required udev rules automatically
  hardware.lenovo-legion.enable = true;
}
```

Or import `nix/nixos-module.nix` directly:
```nix
{ pkgs, ... }:
{
  imports = [ ./nix/nixos-module.nix ];
  hardware.lenovo-legion.enable = true;
}
```

---

## udev Rules for Unprivileged Control

By default, Linux limits write access on `/dev/hidraw*`, sysfs thermal profiles, and HWMON target files to `root`. 

Installing `udev/99-lenovo-legion.rules` grants standard logged-in users permission (`MODE="0666", TAG+="uaccess"`):

```bash
sudo cp udev/99-lenovo-legion.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
```

---

## Material You Desktop Application (`legionspace`)

Version 1.0.0 introduces **Legion Space**, a standalone desktop GUI application designed with Google's **Material 3 Expressive (Material You)** design philosophy.

- **Real-time Live Telemetry**: Live CPU, GPU, and DDR5 RAM utilization, clock frequencies (GHz), temperatures (°C), and dual-fan RPM telemetry.
- **ACPI Thermal Profile Selector**: Segmented pill selectors for Quiet, Balanced, Performance, and Custom modes with physical power button LED synchronization.
- **Cooling Fans Management**: Automatic firmware curve restore, 5100 RPM full turbo boost, and manual RPM slider (1700–5100 RPM).
- **Battery Care**: One-click toggles for Battery Conservation Mode (80% charge threshold limiter) and Always-On USB charging.
- **Interactive 4-Zone RGB Lighting**: Interactive 4-zone keyboard representation, preset switcher (Legion Blue, Cyberpunk, Crimson, Spectrum, Stealth), custom hex color picker, brightness, and Fn-Lock toggles.
- **Advanced Power Tuning**: Custom PL1/PL2/PL3 power limit sliders and thermal throttle adjustments.

To launch the desktop GUI:
```bash
# Launch directly from terminal:
legionspace

# Or search "Legion Space" in GNOME / KDE Application launcher
```

---

## Command Reference & Usage

Both binary names `legion-cli` and `legionctl` are available.

### 1. System Status & Verification

```bash
# Display comprehensive live hardware status dashboard (default)
legion-cli status
# or
legionctl

# Full hardware capability verification and DMI identification
legion-cli probe

# Structured JSON output for scripts, waybar, or polybar integration
legion-cli status --json
```

---

### 2. Thermal Profile Switching

```bash
# Show active profile and power button LED indicator
legion-cli profile get

# List all available profiles
legion-cli profile list

# Switch thermal profile
legion-cli profile set quiet        # or low-power (Blue Power LED)
legion-cli profile set balanced     # (White Power LED)
legion-cli profile set performance  # (Red Power LED)
legion-cli profile set max-power    # Extreme Turbo (Red Power LED)
legion-cli profile set custom       # Firmware tuning envelope (Purple Power LED)
```

---

### 3. Cooling Fan Management

```bash
# Query fan RPM, min/max boundaries, and current mode
legion-cli fan status

# Set both fans to a manual target speed (1700 - 5100 RPM)
legion-cli fan set --both 4000

# Set CPU and GPU fans independently
legion-cli fan set --fan1 4200 --fan2 3800

# Set fans to maximum rated speed (5100 RPM)
legion-cli fan full

# Return fans to automatic firmware thermal curve
legion-cli fan auto
```

---

### 4. Battery Conservation & Power Telemetry

```bash
# Inspect battery health, charge capacity, power draw, and cycle count
legion-cli battery status

# Enable battery conservation mode (caps charging at 80% to maximize lifespan)
legion-cli battery conservation on

# Disable conservation mode (allow full 100% charging)
legion-cli battery conservation off

# Toggle always-on USB charging while laptop is off/sleeping
legion-cli battery usb-charge on
legion-cli battery usb-charge off
```

---

### 5. Custom Mode Firmware Power & Thermal Limits

```bash
# List all configurable firmware attributes with current values and safe bounds
legion-cli custom status

# Set custom CPU power limits (Watts)
legion-cli custom set --pl1 75 --pl2 85 --pl3 105

# Set thermal limits (°C)
legion-cli custom set --cpu-temp 95 --gpu-temp 87

# Set GPU TGP power offset (Watts)
legion-cli custom set --tgp-offset 25

# Set arbitrary firmware attribute by name
legion-cli custom set --name ppt_pl1_spl --value 70
```

---

### 6. Keyboard Lighting & RGB Controller

```bash
# Inspect lighting status, ACPI backlight level, and RGB USB HID device
legion-cli lighting status

# Set white ACPI keyboard backlight level (0 = Off, 1 = Low, 2 = High)
legion-cli lighting kbd-backlight 2

# Toggle Fn-Lock state
legion-cli lighting fn-lock on

# Set solid static RGB color on all 4 keyboard zones
legion-cli lighting static --zone all --color "#00d3fe" --brightness 3
legion-cli lighting static --zone 1 --color red
legion-cli lighting static --zone 2 --color green
legion-cli lighting static --zone 3 --color blue
legion-cli lighting static --zone 4 --color cyan

# Set dynamic effect (wave, breath, static)
legion-cli lighting effect --mode wave --speed 3 --brightness 3
legion-cli lighting effect --mode breath --color magenta --speed 2

# Apply built-in lighting presets
legion-cli lighting preset legion-blue
legion-cli lighting preset cyberpunk
legion-cli lighting preset crimson
legion-cli lighting preset spectrum
legion-cli lighting preset stealth     # turns off lighting

# Turn off all RGB lighting
legion-cli lighting off
```

---

## Semantic Versioning History

| Version | Milestone | Changes & Scope |
| :--- | :--- | :--- |
| **`v0.1.0`** | Baseline Hardware Probe | Hardware probing, DMI matching (`83LT`), HWMON detection, ACPI profile detection, and baseline verification CLI. |
| **`v0.2.0`** | Thermal Profiles | ACPI platform profile switching (`/sys/firmware/acpi/platform_profile`) and power LED mappings. |
| **`v0.3.0`** | Cooling Fans | Independent CPU/GPU fan RPM telemetry, min/max boundaries (1700–5100 RPM), target speeds, auto curve. |
| **`v0.4.0`** | Battery Conservation | Battery conservation mode control (80% charge threshold), health metrics, and always-on USB charging. |
| **`v0.5.0`** | Custom Mode Tuning | Custom mode firmware power limits (PL1/PL2/PL3) and CPU/GPU thermal throttle thresholds. |
| **`v0.6.0`** | 4-Zone RGB Lighting | ITE 8258 USB HID (`048d:c195`) 32-byte feature report `0xCC` ioctl protocol, static colors, effects, and presets. |
| **`v0.7.0`** | Unified CLI & Telemetry | Comprehensive status overview dashboard, interactive CLI commands, and machine-readable JSON output. |
| **`v0.8.0`** | udev Permissions | udev rules (`MODE="0666", TAG+="uaccess"`) for unprivileged non-root hardware access. |
| **`v0.9.0`** | Packaging & Automation | Nix flake, NixOS module, build scripts, and automated root installer with `--impure` support. |
| **`v1.0.0`** | Material You Desktop GUI | Complete Material 3 Expressive standalone desktop GUI app (`legionspace`), real-time hardware telemetry gauges, interactive 4-zone RGB visualizer, and desktop launcher. |

---

## Reverse Engineering Architecture

For disassembly details, Ghidra/PE analysis, WMI GUIDs, and USB HID byte packet layouts, see [docs/REVERSE_ENGINEERING.md](docs/REVERSE_ENGINEERING.md).

---

## License

MIT License. Developed for the Linux hardware community.
