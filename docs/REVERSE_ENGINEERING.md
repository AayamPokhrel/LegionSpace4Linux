# Reverse Engineering Analysis (REA): Lenovo Legion Pro 5 16ADR10 Hardware Controls

## Executive Summary
This document provides the reverse engineering analysis of Lenovo's proprietary hardware management software (`legionspace-1.9.11.5_x64_rtm.exe`) and maps vendor protocols to native Linux kernel subsystems, ACPI methods, WMI interfaces, and USB HID reports for the **Lenovo Legion Pro 5 16ADR10 (Type 83LT)** laptop.

---

## 1. Target Acquisition & Ingestion

- **Source Package**: `legionspace-1.9.11.5_x64_rtm.exe`
- **Format**: PE32 executable containing an embedded LZMA2 solid 7z archive at resource `.rsrc/ZIPRES/108`.
- **Primary Reverse-Engineered Binaries**:
  - `se/plugins/SEThermalModePlugin/SEThermalModePlugin64.dll` (Thermal management, SmartFan, power profiles, custom TDP)
  - `se/plugins/SEPowerBatteryPlugin/SEPowerBatteryPlugin64.dll` (Battery conservation mode, rapid charge telemetry)
  - `se/plugins/SYSCore/SYSCore64.dll` (System core events, platform capabilities)
  - `se/lgtplugins/SEBasicLighting/SEBasicLighting64.dll` (Embedded HIDAPI, 4-zone RGB protocol, lighting effects)
  - `se/plugins/SELuminousNB/SELuminousNB64.dll` (Advanced lighting engine, LampArray integration)
  - `se/lgtplugins/SEBasicLighting/NBLightingConfig/*.json` (Hardware layout, key masks, zone definitions)

---

## 2. Hardware Subsystems & Control Protocols

### 2.1 Thermal Modes & Power Profiles

#### Windows Vendor Implementation
In `SEThermalModePlugin64.dll`, thermal modes are commanded via Windows Management Instrumentation (WMI):
- **WMI Namespace**: `root\wmi`
- **WMI Classes**: `LENOVO_GAMEZONE_DATA`, `LENOVO_OTHER_METHOD`
- **WMI Methods**:
  - `GetSmartFanMode()` -> Returns current mode ID
  - `SetSmartFanMode(mode_id)` -> Sets mode
  - `GetSupportThermalMode()` -> Bitmask of supported modes
- **Mode Values**:
  - `1`: Quiet (`low-power`, Blue LED)
  - `2`: Balanced (`balanced`, White LED)
  - `3`: Performance (`performance`, Red LED)
  - `4`: Extreme / Turbo (`max-power`)
  - `5`: Custom (`custom`, Purple LED)

#### Linux Hardware Grounding & Implementation
In the Linux kernel (upstream modules `lenovo_wmi_gamezone` and `lenovo_wmi_other`, GUID `887B54E3-DDDC-4B2C-8B88-68A26A8835D0`):
- **Sysfs Node**: `/sys/firmware/acpi/platform_profile`
- **Choices**: `/sys/firmware/acpi/platform_profile_choices`
  - Values: `low-power`, `balanced`, `performance`, `max-power`, `custom`
- **Direct WMI Bus Node**:
  `/sys/devices/platform/PNP0C14:02/wmi_bus/wmi_bus-PNP0C14:02/887B54E3-DDDC-4B2C-8B88-68A26A8835D0-3/platform-profile/platform-profile-0/profile`

#### Custom Mode Power & Thermal Limits (Firmware Attributes)
When in `custom` mode, the EC and BIOS expose granular power limit controls via `lenovo-wmi-other-0`:
- **Sysfs Path**: `/sys/class/firmware-attributes/lenovo-wmi-other-0/attributes/`
- **Controllable Attributes**:
  | Attribute | Description | Min | Max | Default | Physical Measured |
  |-----------|-------------|-----|-----|---------|-------------------|
  | `ppt_pl1_spl` | CPU Sustained Power Limit (SPL) | 50 W | 85 W | 70 W | 70 W |
  | `ppt_pl2_sppt` | CPU Slow Package Power Tracking (sPPT) | 60 W | 130 W | 85 W | 85 W |
  | `ppt_pl3_fppt` | CPU Fast Package Power Tracking (fPPT) | 70 W | 150 W | 102 W | 102 W |
  | `ppt_cpu_cl` | CPU Cross Loading Limit | 30 W | 70 W | 70 W | 70 W |
  | `cpu_temp` | CPU Thermal Load Limit | 85 °C | 100 °C | 100 °C | 100 °C |
  | `gpu_temp` | GPU Thermal Load Limit | 75 °C | 87 °C | 87 °C | 87 °C |
  | `gpu_nv_ac_offset` | NVIDIA GPU AC baseline offset | 10 W | 85 W | 55 W | 55 W |
  | `gpu_nv_ctgp` | NVIDIA Configurable TGP | 0 W | - | 80 W | 80 W |
  | `gpu_nv_ppab` | NVIDIA Dynamic Boost (PPAB) | 0 W | - | 15 W | 15 W |
  | `gpu_nv_cpu_boost` | NVIDIA GPU-to-CPU Boost | 0 W | - | 15 W | 15 W |

---

### 2.2 Fan Monitoring & Manual Control

#### Windows Vendor Implementation
Vendor binaries query and set fan speeds through:
- `CFanManagerHelper`
- `LENOVO_FAN_METHOD`:
  - `Fan_Get_Table`
  - `Fan_Set_Table`
  - `GetFanSpeed`

#### Linux Hardware Grounding & Implementation
- Upstream driver `lenovo-wmi-other` registers an HWMON device (`hwmon5` on 16ADR10):
  - Path: `/sys/class/hwmon/hwmon5/` (device: `DC2A8805-3A8C-41BA-A6F7-092E0089CD3B-6`)
  - **Sensors & Targets**:
    - `fan1_input`: Current Fan 1 (CPU) RPM (range: 1700 - 5100 RPM)
    - `fan1_target`: Target RPM for Fan 1 (`0` = automatic firmware control)
    - `fan2_input`: Current Fan 2 (GPU) RPM (range: 1700 - 5100 RPM)
    - `fan2_target`: Target RPM for Fan 2 (`0` = automatic firmware control)
- ACPI Fan Mode:
  - Path: `/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00/fan_mode`
  - Mode: `0` = Auto / Normal

---

### 2.3 Battery Conservation Mode & Telemetry

#### Windows Vendor Implementation
In `SEPowerBatteryPlugin64.dll`, the battery mode watcher manages `BatteryChargeMode`:
- Value `Storage` (Conservation Mode): Maintains 75% - 80% charge ceiling.
- Controlled via Lenovo ACPI device `VPC2004`.

#### Linux Hardware Grounding & Implementation
- **Conservation Mode Control**:
  - Path: `/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00/conservation_mode`
  - Values:
    - `1`: Enabled (Stops charging at ~80% capacity to maximize battery longevity)
    - `0`: Disabled (Charges fully to 100%)
- **Battery Telemetry**:
  - Path: `/sys/class/power_supply/BAT0/`
  - Parameters:
    - `capacity`: Current percentage (%)
    - `status`: Charging / Discharging / Full / Not charging
    - `energy_now`: Current charge (µWh)
    - `energy_full`: Full charge capacity (µWh)
    - `energy_full_design`: Factory design capacity (µWh)
    - `power_now`: Current power draw/charge rate (µW)
    - `voltage_now`: Voltage (µV)
    - `cycle_count`: Charge cycles elapsed

---

### 2.4 Lighting & Keyboard Backlight

#### Hardware Architecture
The Legion Pro 5 16ADR10 features:
1. **ACPI Keyboard Backlight**:
   - `/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00/leds/platform::kbd_backlight/brightness`
   - Levels: `0` (Off), `1` (Low), `2` (High)
2. **ITE 8258 RGB Microcontroller**:
   - USB Device: `048D:C195` (ITE Tech. Inc. ITE Device(8258))
   - Interface 0 (`/dev/hidraw0`): Vendor Feature Report Protocol (Usage Page `0xFF89`)
   - Interface 1 (`/dev/hidraw1`): USB-IF / Windows 11 LampArray (`0x59`)

#### Disassembled USB HID Feature Report Protocol (`048D:C195`)
Reversed from `SEBasicLighting64.dll` (`0x1800B6CD2`, `0x1800B7260`):
- **Transport**: HID Feature Report (`hid_send_feature_report` / `ioctl(fd, HIDIOCSFEATURE)`)
- **Report ID**: `0xCC`
- **Report Length**: 32 bytes (minimum) / 200 bytes (`0xC8`)
- **Packet Structure**:
  | Offset | Field | Value / Meaning |
  |--------|-------|-----------------|
  | `0x00` | Report ID | `0xCC` (Mandatory HID report ID) |
  | `0x01` | Subcommand / Zone | `0x01` (Zone 1: Left)<br>`0x02` (Zone 2: Center-Left)<br>`0x03` (Zone 3: Center-Right)<br>`0x04` (Zone 4: Right / Numpad)<br>`0x18` (Commit / Profile Switch) |
  | `0x02` | Effect Mode | `0x00` = Off<br>`0x01` = Static Color<br>`0x03` = Breathing<br>`0x09` = Wave / Spectrum |
  | `0x03` | Speed / Direction | `0x01` (Slow) .. `0x04` (Fast) |
  | `0x04` | Brightness | `0x00` (Off) .. `0x03` (High) |
  | `0x05` | Red | `0x00` - `0xFF` (0 - 255) |
  | `0x06` | Green | `0x00` - `0xFF` (0 - 255) |
  | `0x07` | Blue | `0x00` - `0xFF` (0 - 255) |
  | `0x08..0xC7` | Padding | Zero-filled (`0x00`) |

- **Profile Activation Command**:
  - `[0xCC, 0x18, <profile_index: 0x01..0x04>, <flag: 0x01>, <brightness: 0x01..0x03>, 0x00...]`

---

### 2.5 GPU Working Modes & Display MUX Switch

#### Windows Vendor Implementation (`SEGSync64.dll`)
Disassembled methods and contracts:
- `HybridMode`, `HybridDGPUMode`, `HybridIGPUMode`, `HybridAutoMode`
- `Get-MDMMode`, `Set-MDMMode`, `DisplayMuxDevice` (Windows.Devices.Display.Core)
- `get_app_run_gpu`, `close_all_apps_gpu`

#### Linux Hardware Grounding & Implementation
- **Hardware MUX**: Parade `PS8371` eDP MUX switch (`MMID` package in ACPI `SSDT4.dsl`).
  - `card1-eDP-2`: Direct NVIDIA RTX 5060 dGPU pipeline.
  - `card2-eDP-1`: Integrated AMD Raphael iGPU pipeline.
- **Operating Modes**:
  1. **Hybrid (Optimus)**: PCI runtime power management set to `auto` (`/sys/bus/pci/devices/0000:01:00.0/power/control`), dynamic on-demand render offload.
  2. **dGPU Mode**: Hardware MUX forced direct to NVIDIA RTX 5060, power control `on`, PowerMizer mode 1 (Prefer Maximum Performance).
  3. **Hybrid-iGPU Mode (Eco)**: Discrete GPU powered down into `D3cold` (`/sys/bus/pci/devices/0000:01:00.0/d3cold_allowed = 1`), internal display driven exclusively by AMD Radeon 610M iGPU.
  4. **Hybrid-Auto Mode**: Dynamic power sensing via AC adapter status (`/sys/class/power_supply/ADP0/online`). Automatically engages Hybrid/dGPU on AC and forces Eco mode on DC battery.
- **PowerMizer Tuning**: Configured via `nvidia-settings -a [gpu:0]/GPUPowerMizerMode=N` (0 = Adaptive, 1 = Max Performance, 2 = Auto).
- **Process Telemetry**: Active GPU compute/graphics processes parsed from `nvidia-smi --query-compute-apps=pid,process_name,used_gpu_memory`.

---

### 2.6 OLED Protection & Display Care Suite

#### Windows Vendor Implementation (`SEOledAntiBurnt64.dll`, `SEBiosODPlugin64.dll`, `SEAdaptiveRefreshRatePlugin64.dll`)
Disassembled routines and symbols:
- `auto_shift_wallpapers` (`AutoShiftWallPaper`)
- `default_dark_Mode` (`DefaultDarkMode`)
- `get_oled_anti_burnt_settings`, `set_oled_anti_burnt_settings`, `oledCheck`
- `get_OD_status`, `set_OD_status` (LCD OverDrive response time accelerator)
- `FnR2Status` (Fn+R dynamic refresh rate switching)

#### Linux Hardware Grounding & Implementation
- **Pixel Shift**: Sub-pixel micro-translations every 2-3 minutes to prevent static burn-in on fixed UI elements.
- **Default Dark Mode**: GNOME desktop sync via `gsettings set org.gnome.desktop.interface color-scheme prefer-dark`.
- **Static Taskbar Dimming**: Dimming persistent desktop elements and static icons.
- **4-Hour Clean Cycle Tracker**: Tracks panel active on-time (`screen_on_seconds >= 14400`) and reminds user to perform panel calibration refresh.
- **LCD OverDrive (OD)**: Panel response time overdrive booster for gaming.
- **Adaptive Refresh Rate**: Dynamic mode switching between 60 Hz on battery and native 165/240 Hz on AC power.

---

### 2.7 Rapid Charge & Fn-Lock Controls

#### ACPI EC Control Method (`Method SBMC`)
From decompiled `DSDT.dsl`:
```asl
Method (SBMC, 1, Serialized)
{
    If ((Arg0 == 0x03)) { BTSM = 0x01 } // Conservation Mode ON (80% charge limit)
    If ((Arg0 == 0x05)) { BTSM = 0x00 } // Conservation Mode OFF
    If ((Arg0 == 0x07)) { QCHG = 0x01; BTSM = 0x00 } // Rapid Charge ON (clears Conservation)
    If ((Arg0 == 0x08)) { QCHG = 0x00 } // Rapid Charge OFF
    If ((Arg0 == 0x09)) { CAMM = 0x01 } // Camera E-Shutter ON
    If ((Arg0 == 0x10)) { CAMM = 0x00 } // Camera E-Shutter OFF
}
```
- **Rapid Charge**: Managed via `/sys/class/power_supply/BAT0/charge_types` (`Fast` vs `Normal`).
- **Fn-Lock**: Managed via `/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00/fn_lock` (1 = F1-F12 direct, 0 = hotkeys direct).

---

## 3. Physical Hardware Verification

Verification performed on physical machine:
- **DMI sys_vendor**: `LENOVO`
- **DMI product_name**: `83LT`
- **DMI product_family**: `Legion Pro 5 16ADR10`
- **DMI product_version**: `Legion Pro 5 16ADR10`
- **Active Profile**: `performance` (Choices: `low-power`, `balanced`, `performance`, `max-power`, `custom`)
- **Battery**: `BAT0` at 80% with `conservation_mode = 1` active (Not charging)
- **Fans**: `hwmon5` Fan 1 @ 2400 RPM, Fan 2 @ 2100 RPM
- **RGB Chip**: `048D:C195` present on USB bus 3-2.4

