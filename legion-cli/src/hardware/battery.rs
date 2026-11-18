use crate::error::{LegionError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const CONSERVATION_MODE_PATH: &str =
    "/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00/conservation_mode";
const USB_CHARGING_PATH: &str =
    "/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00/usb_charging";
const BAT0_PATH: &str = "/sys/class/power_supply/BAT0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryDetails {
    pub present: bool,
    pub conservation_mode: bool,
    pub usb_charging: Option<bool>,
    pub capacity_percent: u32,
    pub status: String,
    pub health_percent: Option<f64>,
    pub energy_now_wh: f64,
    pub energy_full_wh: f64,
    pub energy_design_wh: f64,
    pub power_now_w: f64,
    pub voltage_now_v: f64,
    pub cycle_count: Option<u32>,
    pub technology: String,
}

fn read_string(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_u32(path: impl AsRef<Path>) -> Option<u32> {
    read_string(path).and_then(|s| s.parse::<u32>().ok())
}

fn read_f64(path: impl AsRef<Path>) -> Option<f64> {
    read_string(path).and_then(|s| s.parse::<f64>().ok())
}

pub fn get_battery_details() -> Result<BatteryDetails> {
    let bat_dir = Path::new(BAT0_PATH);
    if !bat_dir.exists() {
        return Err(LegionError::DeviceNotFound("BAT0 battery power supply not found".into()));
    }

    let cm_val = read_u32(CONSERVATION_MODE_PATH).unwrap_or(0);
    let conservation_mode = cm_val == 1;

    let usb_charging = read_u32(USB_CHARGING_PATH).map(|v| v == 1);

    let capacity_percent = read_u32(bat_dir.join("capacity")).unwrap_or(0);
    let status = read_string(bat_dir.join("status")).unwrap_or_else(|| "Unknown".into());
    let cycle_count = read_u32(bat_dir.join("cycle_count"));
    let technology = read_string(bat_dir.join("technology")).unwrap_or_else(|| "Li-poly".into());

    let energy_now_wh = read_f64(bat_dir.join("energy_now")).unwrap_or(0.0) / 1_000_000.0;
    let energy_full_wh = read_f64(bat_dir.join("energy_full")).unwrap_or(0.0) / 1_000_000.0;
    let energy_design_wh = read_f64(bat_dir.join("energy_full_design")).unwrap_or(0.0) / 1_000_000.0;
    let power_now_w = read_f64(bat_dir.join("power_now")).unwrap_or(0.0) / 1_000_000.0;
    let voltage_now_v = read_f64(bat_dir.join("voltage_now")).unwrap_or(0.0) / 1_000_000.0;

    let health_percent = if energy_design_wh > 0.0 {
        Some((energy_full_wh / energy_design_wh) * 100.0)
    } else {
        None
    };

    Ok(BatteryDetails {
        present: true,
        conservation_mode,
        usb_charging,
        capacity_percent,
        status,
        health_percent,
        energy_now_wh,
        energy_full_wh,
        energy_design_wh,
        power_now_w,
        voltage_now_v,
        cycle_count,
        technology,
    })
}

pub fn set_conservation_mode(enable: bool) -> Result<BatteryDetails> {
    let p = Path::new(CONSERVATION_MODE_PATH);
    if !p.exists() {
        return Err(LegionError::UnsupportedFeature(
            "Lenovo conservation_mode sysfs interface not found".into(),
        ));
    }

    let val = if enable { "1" } else { "0" };
    fs::write(p, val).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            LegionError::PermissionDenied {
                path: p.display().to_string(),
                action: "write",
            }
        } else {
            LegionError::Io(e)
        }
    })?;

    get_battery_details()
}

pub fn set_usb_charging(enable: bool) -> Result<BatteryDetails> {
    let p = Path::new(USB_CHARGING_PATH);
    if !p.exists() {
        return Err(LegionError::UnsupportedFeature(
            "Lenovo usb_charging sysfs interface not found".into(),
        ));
    }

    let val = if enable { "1" } else { "0" };
    fs::write(p, val).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            LegionError::PermissionDenied {
                path: p.display().to_string(),
                action: "write",
            }
        } else {
            LegionError::Io(e)
        }
    })?;

    get_battery_details()
}

