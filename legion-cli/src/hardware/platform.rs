use crate::error::{LegionError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const VPC_PATH: &str = "/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00";

fn get_vpc_node(name: &str) -> Option<PathBuf> {
    let p = Path::new(VPC_PATH).join(name);
    if p.exists() {
        return Some(p);
    }
    // Fallback: search /sys/bus/platform/devices/VPC2004:00/
    let alt = Path::new("/sys/bus/platform/devices/VPC2004:00").join(name);
    if alt.exists() {
        return Some(alt);
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformFeatures {
    pub camera_power: Option<bool>,
    pub fn_lock: Option<bool>,
    pub usb_charging: Option<bool>,
    pub conservation_mode: Option<bool>,
    pub fan_mode: Option<u32>,
}

pub fn get_platform_features() -> PlatformFeatures {
    let read_bool = |name: &str| -> Option<bool> {
        let node = get_vpc_node(name)?;
        let val = fs::read_to_string(node).ok()?;
        match val.trim() {
            "1" => Some(true),
            "0" => Some(false),
            _ => None,
        }
    };

    let read_u32 = |name: &str| -> Option<u32> {
        let node = get_vpc_node(name)?;
        let val = fs::read_to_string(node).ok()?;
        val.trim().parse::<u32>().ok()
    };

    PlatformFeatures {
        camera_power: read_bool("camera_power"),
        fn_lock: read_bool("fn_lock"),
        usb_charging: read_bool("usb_charging"),
        conservation_mode: read_bool("conservation_mode"),
        fan_mode: read_u32("fan_mode"),
    }
}

pub fn get_camera_power() -> Result<bool> {
    let node = get_vpc_node("camera_power").ok_or_else(|| {
        LegionError::UnsupportedFeature("VPC camera_power node not found".into())
    })?;

    let val = fs::read_to_string(&node).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            LegionError::PermissionDenied {
                path: node.display().to_string(),
                action: "read",
            }
        } else {
            LegionError::Io(e)
        }
    })?;

    Ok(val.trim() == "1")
}

pub fn set_camera_power(enable: bool) -> Result<bool> {
    let node = get_vpc_node("camera_power").ok_or_else(|| {
        LegionError::UnsupportedFeature("VPC camera_power node not found".into())
    })?;

    let val = if enable { "1" } else { "0" };
    fs::write(&node, val).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            LegionError::PermissionDenied {
                path: node.display().to_string(),
                action: "write",
            }
        } else {
            LegionError::Io(e)
        }
    })?;

    get_camera_power()
}

