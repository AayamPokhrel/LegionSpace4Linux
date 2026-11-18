use crate::error::{LegionError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const SYSFS_PLATFORM_PROFILE: &str = "/sys/firmware/acpi/platform_profile";
const SYSFS_PLATFORM_CHOICES: &str = "/sys/firmware/acpi/platform_profile_choices";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalStatus {
    pub current_profile: String,
    pub available_choices: Vec<String>,
    pub sysfs_path: String,
    pub power_led_color: String,
}

pub fn get_profile_path() -> Result<PathBuf> {
    let p = Path::new(SYSFS_PLATFORM_PROFILE);
    if p.exists() {
        return Ok(p.to_path_buf());
    }

    // Fallback: look directly in WMI bus
    let wmi_path = Path::new(
        "/sys/devices/platform/PNP0C14:02/wmi_bus/wmi_bus-PNP0C14:02/887B54E3-DDDC-4B2C-8B88-68A26A8835D0-3/platform-profile/platform-profile-0/profile",
    );
    if wmi_path.exists() {
        return Ok(wmi_path.to_path_buf());
    }

    Err(LegionError::UnsupportedFeature(
        "ACPI platform_profile interface not found. Ensure lenovo_wmi_gamezone or ideapad_laptop kernel module is loaded.".into(),
    ))
}

pub fn get_choices_path() -> Option<PathBuf> {
    let p = Path::new(SYSFS_PLATFORM_CHOICES);
    if p.exists() {
        return Some(p.to_path_buf());
    }
    let wmi_path = Path::new(
        "/sys/devices/platform/PNP0C14:02/wmi_bus/wmi_bus-PNP0C14:02/887B54E3-DDDC-4B2C-8B88-68A26A8835D0-3/platform-profile/platform-profile-0/choices",
    );
    if wmi_path.exists() {
        return Some(wmi_path.to_path_buf());
    }
    None
}

pub fn map_led_color(profile: &str) -> &'static str {
    match profile {
        "low-power" | "quiet" => "Blue",
        "balanced" => "White",
        "performance" => "Red",
        "max-power" | "extreme" => "Red (Turbo)",
        "custom" => "Purple",
        _ => "Unknown",
    }
}

pub fn normalize_profile_name(input: &str) -> String {
    let lower = input.to_lowercase();
    match lower.as_str() {
        "quiet" | "low" | "low-power" => "low-power".to_string(),
        "balance" | "balanced" => "balanced".to_string(),
        "perf" | "performance" => "performance".to_string(),
        "extreme" | "max" | "max-power" | "turbo" => "max-power".to_string(),
        "custom" => "custom".to_string(),
        _ => lower,
    }
}

pub fn get_thermal_status() -> Result<ThermalStatus> {
    let path = get_profile_path()?;
    let current = fs::read_to_string(&path)
        .map_err(|_e| LegionError::PermissionDenied {
            path: path.display().to_string(),
            action: "read",
        })?
        .trim()
        .to_string();

    let choices = if let Some(cp) = get_choices_path() {
        fs::read_to_string(cp)
            .ok()
            .map(|s| s.split_whitespace().map(|x| x.to_string()).collect())
            .unwrap_or_default()
    } else {
        vec![
            "low-power".to_string(),
            "balanced".to_string(),
            "performance".to_string(),
            "max-power".to_string(),
            "custom".to_string(),
        ]
    };

    let power_led_color = map_led_color(&current).to_string();

    Ok(ThermalStatus {
        current_profile: current,
        available_choices: choices,
        sysfs_path: path.display().to_string(),
        power_led_color,
    })
}

pub fn set_thermal_profile(target: &str) -> Result<ThermalStatus> {
    let path = get_profile_path()?;
    let normalized = normalize_profile_name(target);

    // Validate against choices if available
    let status = get_thermal_status()?;
    if !status.available_choices.is_empty()
        && !status.available_choices.iter().any(|c| c == &normalized)
    {
        return Err(LegionError::InvalidValue(format!(
            "Profile '{}' is not supported by this hardware. Available choices: {}",
            target,
            status.available_choices.join(", ")
        )));
    }

    fs::write(&path, normalized).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            LegionError::PermissionDenied {
                path: path.display().to_string(),
                action: "write",
            }
        } else {
            LegionError::Io(e)
        }
    })?;

    get_thermal_status()
}
