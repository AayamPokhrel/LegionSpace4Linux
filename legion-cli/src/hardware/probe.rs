use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmiInfo {
    pub sys_vendor: String,
    pub product_name: String,
    pub product_family: String,
    pub product_version: String,
    pub bios_version: String,
    pub is_lenovo_legion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalProbe {
    pub supported: bool,
    pub current_profile: Option<String>,
    pub available_choices: Vec<String>,
    pub sysfs_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanProbe {
    pub hwmon_name: String,
    pub hwmon_path: String,
    pub fan1_rpm: Option<u32>,
    pub fan1_min: Option<u32>,
    pub fan1_max: Option<u32>,
    pub fan1_target: Option<u32>,
    pub fan2_rpm: Option<u32>,
    pub fan2_min: Option<u32>,
    pub fan2_max: Option<u32>,
    pub fan2_target: Option<u32>,
    pub vpc_fan_mode: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryProbe {
    pub present: bool,
    pub capacity_percent: Option<u32>,
    pub status: Option<String>,
    pub conservation_mode: Option<bool>,
    pub energy_now_wh: Option<f64>,
    pub energy_full_wh: Option<f64>,
    pub energy_design_wh: Option<f64>,
    pub power_now_w: Option<f64>,
    pub cycle_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgbControllerProbe {
    pub detected: bool,
    pub vendor_id: String,
    pub product_id: String,
    pub device_name: String,
    pub hidraw_nodes: Vec<String>,
    pub can_write: bool,
    pub acpi_kbd_backlight: Option<u32>,
    pub fn_lock: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareAttributesProbe {
    pub supported: bool,
    pub attribute_count: usize,
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemProbeReport {
    pub dmi: DmiInfo,
    pub thermal: ThermalProbe,
    pub fans: Option<FanProbe>,
    pub battery: BatteryProbe,
    pub rgb: RgbControllerProbe,
    pub firmware_attributes: FirmwareAttributesProbe,
}

fn read_sysfs_string(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_sysfs_u32(path: impl AsRef<Path>) -> Option<u32> {
    read_sysfs_string(path).and_then(|s| s.parse::<u32>().ok())
}

pub fn probe_dmi() -> DmiInfo {
    let sys_vendor = read_sysfs_string("/sys/class/dmi/id/sys_vendor").unwrap_or_default();
    let product_name = read_sysfs_string("/sys/class/dmi/id/product_name").unwrap_or_default();
    let product_family = read_sysfs_string("/sys/class/dmi/id/product_family").unwrap_or_default();
    let product_version = read_sysfs_string("/sys/class/dmi/id/product_version").unwrap_or_default();
    let bios_version = read_sysfs_string("/sys/class/dmi/id/bios_version").unwrap_or_default();

    let is_lenovo = sys_vendor.to_uppercase().contains("LENOVO");
    let is_legion = product_name.to_uppercase().contains("LEGION")
        || product_family.to_uppercase().contains("LEGION")
        || product_version.to_uppercase().contains("LEGION")
        || product_name == "83LT";

    DmiInfo {
        sys_vendor,
        product_name,
        product_family,
        product_version,
        bios_version,
        is_lenovo_legion: is_lenovo && is_legion,
    }
}

pub fn probe_thermal() -> ThermalProbe {
    let profile_path = "/sys/firmware/acpi/platform_profile";
    let choices_path = "/sys/firmware/acpi/platform_profile_choices";

    if Path::new(profile_path).exists() {
        let current_profile = read_sysfs_string(profile_path);
        let choices = read_sysfs_string(choices_path)
            .map(|s| s.split_whitespace().map(|x| x.to_string()).collect())
            .unwrap_or_default();

        ThermalProbe {
            supported: true,
            current_profile,
            available_choices: choices,
            sysfs_path: profile_path.to_string(),
        }
    } else {
        ThermalProbe {
            supported: false,
            current_profile: None,
            available_choices: Vec::new(),
            sysfs_path: String::new(),
        }
    }
}

pub fn find_lenovo_hwmon() -> Option<PathBuf> {
    let mut fallback = None;
    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = read_sysfs_string(path.join("name")) {
                if name == "lenovo_wmi_other" {
                    return Some(path);
                } else if name == "yogafan" {
                    fallback = Some(path);
                }
            }
        }
    }
    fallback
}

pub fn probe_fans() -> Option<FanProbe> {
    let hwmon_dir = find_lenovo_hwmon()?;
    let name = read_sysfs_string(hwmon_dir.join("name")).unwrap_or_default();

    let fan1_rpm = read_sysfs_u32(hwmon_dir.join("fan1_input"));
    let fan1_min = read_sysfs_u32(hwmon_dir.join("fan1_min"));
    let fan1_max = read_sysfs_u32(hwmon_dir.join("fan1_max"));
    let fan1_target = read_sysfs_u32(hwmon_dir.join("fan1_target"));

    let fan2_rpm = read_sysfs_u32(hwmon_dir.join("fan2_input"));
    let fan2_min = read_sysfs_u32(hwmon_dir.join("fan2_min"));
    let fan2_max = read_sysfs_u32(hwmon_dir.join("fan2_max"));
    let fan2_target = read_sysfs_u32(hwmon_dir.join("fan2_target"));

    let vpc_fan_mode = read_sysfs_u32(
        "/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00/fan_mode",
    );

    Some(FanProbe {
        hwmon_name: name,
        hwmon_path: hwmon_dir.display().to_string(),
        fan1_rpm,
        fan1_min,
        fan1_max,
        fan1_target,
        fan2_rpm,
        fan2_min,
        fan2_max,
        fan2_target,
        vpc_fan_mode,
    })
}

pub fn probe_battery() -> BatteryProbe {
    let bat_dir = Path::new("/sys/class/power_supply/BAT0");
    if !bat_dir.exists() {
        return BatteryProbe {
            present: false,
            capacity_percent: None,
            status: None,
            conservation_mode: None,
            energy_now_wh: None,
            energy_full_wh: None,
            energy_design_wh: None,
            power_now_w: None,
            cycle_count: None,
        };
    }

    let capacity_percent = read_sysfs_u32(bat_dir.join("capacity"));
    let status = read_sysfs_string(bat_dir.join("status"));
    let cycle_count = read_sysfs_u32(bat_dir.join("cycle_count"));

    let energy_now_wh = read_sysfs_string(bat_dir.join("energy_now"))
        .and_then(|s| s.parse::<f64>().ok())
        .map(|uwh| uwh / 1_000_000.0);

    let energy_full_wh = read_sysfs_string(bat_dir.join("energy_full"))
        .and_then(|s| s.parse::<f64>().ok())
        .map(|uwh| uwh / 1_000_000.0);

    let energy_design_wh = read_sysfs_string(bat_dir.join("energy_full_design"))
        .and_then(|s| s.parse::<f64>().ok())
        .map(|uwh| uwh / 1_000_000.0);

    let power_now_w = read_sysfs_string(bat_dir.join("power_now"))
        .and_then(|s| s.parse::<f64>().ok())
        .map(|uw| uw / 1_000_000.0);

    let conservation_mode = read_sysfs_u32(
        "/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00/conservation_mode",
    )
    .map(|v| v == 1);

    BatteryProbe {
        present: true,
        capacity_percent,
        status,
        conservation_mode,
        energy_now_wh,
        energy_full_wh,
        energy_design_wh,
        power_now_w,
        cycle_count,
    }
}

pub fn probe_rgb() -> RgbControllerProbe {
    let mut hidraw_nodes = Vec::new();
    let mut can_write = false;

    // Scan for 048d:c195
    if let Ok(entries) = fs::read_dir("/sys/class/hidraw") {
        for entry in entries.flatten() {
            let path = entry.path();
            let dev_link = path.join("device");
            if let Ok(target) = fs::read_link(&dev_link) {
                let target_str = target.to_string_lossy();
                if target_str.contains("048D:C195") || target_str.contains("048d:c195") {
                    let node_name = entry.file_name().to_string_lossy().to_string();
                    let dev_node = format!("/dev/{}", node_name);
                    
                    // Check write permission
                    if let Ok(meta) = fs::metadata(&dev_node) {
                        use std::os::unix::fs::PermissionsExt;
                        let mode = meta.permissions().mode();
                        // check if user or world writable
                        if (mode & 0o002) != 0 || (mode & 0o020) != 0 || (mode & 0o200) != 0 {
                            // try opening for write
                            if std::fs::OpenOptions::new().write(true).open(&dev_node).is_ok() {
                                can_write = true;
                            }
                        }
                    }
                    hidraw_nodes.push(dev_node);
                }
            }
        }
    }

    let detected = !hidraw_nodes.is_empty();

    let acpi_kbd_backlight = read_sysfs_u32(
        "/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00/leds/platform::kbd_backlight/brightness",
    );
    let fn_lock = read_sysfs_u32(
        "/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00/fn_lock",
    )
    .map(|v| v == 1);

    RgbControllerProbe {
        detected,
        vendor_id: "048d".to_string(),
        product_id: "c195".to_string(),
        device_name: "ITE Tech. Inc. ITE Device(8258) RGB Controller".to_string(),
        hidraw_nodes,
        can_write,
        acpi_kbd_backlight,
        fn_lock,
    }
}

pub fn probe_firmware_attributes() -> FirmwareAttributesProbe {
    let attr_base = Path::new("/sys/class/firmware-attributes/lenovo-wmi-other-0/attributes");
    if !attr_base.exists() {
        return FirmwareAttributesProbe {
            supported: false,
            attribute_count: 0,
            attributes: Vec::new(),
        };
    }

    let mut attrs = Vec::new();
    if let Ok(entries) = fs::read_dir(attr_base) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            attrs.push(name);
        }
    }
    attrs.sort();
    let count = attrs.len();

    FirmwareAttributesProbe {
        supported: true,
        attribute_count: count,
        attributes: attrs,
    }
}

pub fn run_full_probe() -> SystemProbeReport {
    SystemProbeReport {
        dmi: probe_dmi(),
        thermal: probe_thermal(),
        fans: probe_fans(),
        battery: probe_battery(),
        rgb: probe_rgb(),
        firmware_attributes: probe_firmware_attributes(),
    }
}

use colored::*;

pub fn print_probe_report(report: &SystemProbeReport) {
    println!("{}", "========================================================".cyan().bold());
    println!("{}", format!("  Lenovo Legion Hardware Verification Report (v{})   ", env!("CARGO_PKG_VERSION")).cyan().bold());
    println!("{}", "========================================================".cyan().bold());
    println!();

    // DMI
    println!("{}", "[System Identification]".yellow().bold());
    println!("  Vendor:          {}", report.dmi.sys_vendor);
    println!("  Product:         {}", report.dmi.product_name);
    println!("  Family:          {}", report.dmi.product_family);
    println!("  Version:         {}", report.dmi.product_version);
    println!("  BIOS Version:    {}", report.dmi.bios_version);
    if report.dmi.is_lenovo_legion {
        println!("  Legion Verified: {}", "YES (Target Hardware Confirmed)".green().bold());
    } else {
        println!("  Legion Verified: {}", "NO (Generic/Unverified System)".red().bold());
    }
    println!();

    // Thermal
    println!("{}", "[Thermal Profiles & ACPI/WMI]".yellow().bold());
    println!("  Supported:       {}", if report.thermal.supported { "YES".green() } else { "NO".red() });
    if report.thermal.supported {
        println!(
            "  Active Profile:  {}",
            report.thermal.current_profile.as_deref().unwrap_or("unknown").cyan().bold()
        );
        println!(
            "  Available:       {}",
            report.thermal.available_choices.join(", ").white()
        );
        println!("  Sysfs Node:      {}", report.thermal.sysfs_path);
    }
    println!();

    // Fans
    println!("{}", "[Fan Telemetry & HWMON]".yellow().bold());
    if let Some(fans) = &report.fans {
        println!("  HWMON Driver:    {}", fans.hwmon_name.green());
        println!("  HWMON Path:      {}", fans.hwmon_path);
        if let Some(rpm) = fans.fan1_rpm {
            println!(
                "  Fan 1 (CPU):     {} RPM (min: {}, max: {})",
                format!("{}", rpm).green().bold(),
                fans.fan1_min.unwrap_or(0),
                fans.fan1_max.unwrap_or(0)
            );
        }
        if let Some(rpm) = fans.fan2_rpm {
            println!(
                "  Fan 2 (GPU):     {} RPM (min: {}, max: {})",
                format!("{}", rpm).green().bold(),
                fans.fan2_min.unwrap_or(0),
                fans.fan2_max.unwrap_or(0)
            );
        }
        if let Some(mode) = fans.vpc_fan_mode {
            println!("  ACPI Fan Mode:   {}", mode);
        }
    } else {
        println!("  Status:          {}", "No Lenovo fan hwmon interface detected".red());
    }
    println!();

    // Battery
    println!("{}", "[Battery & Conservation Mode]".yellow().bold());
    if report.battery.present {
        if let Some(pct) = report.battery.capacity_percent {
            println!("  Capacity:        {}%", format!("{}", pct).green().bold());
        }
        if let Some(status) = &report.battery.status {
            println!("  Status:          {}", status);
        }
        if let Some(cm) = report.battery.conservation_mode {
            let cm_str = if cm {
                "ENABLED (Capped at 80% for longevity)".green().bold()
            } else {
                "DISABLED (Charging to 100%)".white()
            };
            println!("  Conservation:    {}", cm_str);
        }
        if let Some(w) = report.battery.power_now_w {
            println!("  Power Rate:      {:.2} W", w);
        }
        if let Some(cycles) = report.battery.cycle_count {
            println!("  Cycle Count:     {}", cycles);
        }
    } else {
        println!("  Status:          {}", "No BAT0 battery device found".red());
    }
    println!();

    // RGB & Lighting
    println!("{}", "[Lighting & RGB Controller]".yellow().bold());
    if report.rgb.detected {
        println!(
            "  Controller:      {} [{}:{}]",
            report.rgb.device_name.green(),
            report.rgb.vendor_id,
            report.rgb.product_id
        );
        println!("  HID Nodes:       {}", report.rgb.hidraw_nodes.join(", "));
        println!(
            "  Write Access:    {}",
            if report.rgb.can_write {
                "PERMITTED (Ready for RGB control)".green().bold()
            } else {
                "RESTRICTED (Install udev rules for unprivileged access)".yellow()
            }
        );
    } else {
        println!("  Controller:      {}", "ITE 8258 RGB USB device not detected".yellow());
    }
    if let Some(lvl) = report.rgb.acpi_kbd_backlight {
        println!("  ACPI Backlight:  Level {}", lvl);
    }
    if let Some(fn_l) = report.rgb.fn_lock {
        println!("  Fn Lock:         {}", if fn_l { "ON" } else { "OFF" });
    }
    println!();

    // Firmware attributes
    println!("{}", "[Custom Mode & Firmware Attributes]".yellow().bold());
    if report.firmware_attributes.supported {
        println!(
            "  Tuning Knobs:    {} attributes available",
            report.firmware_attributes.attribute_count
        );
        println!("  Key Attributes:  {}", report.firmware_attributes.attributes.join(", ").white());
    } else {
        println!("  Status:          {}", "Firmware attributes interface not found".yellow());
    }
    println!("{}", "========================================================".cyan().bold());
}
