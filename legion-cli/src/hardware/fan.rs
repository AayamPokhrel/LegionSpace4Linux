use crate::error::{LegionError, Result};
use crate::hardware::probe::find_lenovo_hwmon;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const VPC_FAN_MODE_PATH: &str =
    "/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00/fan_mode";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleFanInfo {
    pub id: u32,
    pub label: String,
    pub current_rpm: u32,
    pub min_rpm: u32,
    pub max_rpm: u32,
    pub target_rpm: u32,
    pub is_manual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanSystemStatus {
    pub hwmon_driver: String,
    pub hwmon_path: String,
    pub fans: Vec<SingleFanInfo>,
    pub vpc_fan_mode: Option<u32>,
}

fn read_u32(path: impl AsRef<Path>) -> Option<u32> {
    fs::read_to_string(path).ok().and_then(|s| s.trim().parse::<u32>().ok())
}

fn write_u32(path: impl AsRef<Path>, val: u32) -> Result<()> {
    let p = path.as_ref();
    fs::write(p, val.to_string()).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            LegionError::PermissionDenied {
                path: p.display().to_string(),
                action: "write",
            }
        } else {
            LegionError::Io(e)
        }
    })
}

pub fn get_fan_status() -> Result<FanSystemStatus> {
    let hwmon_dir = find_lenovo_hwmon().ok_or_else(|| {
        LegionError::DeviceNotFound("No Lenovo HWMON device found (lenovo_wmi_other/yogafan)".into())
    })?;

    let hwmon_driver = fs::read_to_string(hwmon_dir.join("name"))
        .unwrap_or_else(|_| "unknown".into())
        .trim()
        .to_string();

    let mut fans = Vec::new();

    // Fan 1 (CPU)
    let f1_cur = read_u32(hwmon_dir.join("fan1_input")).unwrap_or(0);
    let f1_min = read_u32(hwmon_dir.join("fan1_min")).unwrap_or(1700);
    let f1_max = read_u32(hwmon_dir.join("fan1_max")).unwrap_or(5100);
    let f1_tgt = read_u32(hwmon_dir.join("fan1_target")).unwrap_or(0);

    fans.push(SingleFanInfo {
        id: 1,
        label: "Fan 1 (CPU)".to_string(),
        current_rpm: f1_cur,
        min_rpm: f1_min,
        max_rpm: f1_max,
        target_rpm: f1_tgt,
        is_manual: f1_tgt > 0,
    });

    // Fan 2 (GPU)
    let f2_cur = read_u32(hwmon_dir.join("fan2_input")).unwrap_or(0);
    let f2_min = read_u32(hwmon_dir.join("fan2_min")).unwrap_or(1700);
    let f2_max = read_u32(hwmon_dir.join("fan2_max")).unwrap_or(5100);
    let f2_tgt = read_u32(hwmon_dir.join("fan2_target")).unwrap_or(0);

    fans.push(SingleFanInfo {
        id: 2,
        label: "Fan 2 (GPU)".to_string(),
        current_rpm: f2_cur,
        min_rpm: f2_min,
        max_rpm: f2_max,
        target_rpm: f2_tgt,
        is_manual: f2_tgt > 0,
    });

    let vpc_fan_mode = read_u32(VPC_FAN_MODE_PATH);

    Ok(FanSystemStatus {
        hwmon_driver,
        hwmon_path: hwmon_dir.display().to_string(),
        fans,
        vpc_fan_mode,
    })
}

pub fn set_fan_target(fan_id: u32, target_rpm: u32) -> Result<()> {
    let hwmon_dir = find_lenovo_hwmon().ok_or_else(|| {
        LegionError::DeviceNotFound("No Lenovo HWMON device found".into())
    })?;

    let file_name = match fan_id {
        1 => "fan1_target",
        2 => "fan2_target",
        other => return Err(LegionError::InvalidValue(format!("Invalid fan ID: {}. Expected 1 or 2", other))),
    };

    let target_path = hwmon_dir.join(file_name);
    if !target_path.exists() {
        return Err(LegionError::UnsupportedFeature(format!(
            "Target speed control '{}' is not supported by driver",
            file_name
        )));
    }

    // If setting target > 0, clamp to min/max
    if target_rpm > 0 {
        let max_path = hwmon_dir.join(format!("fan{}_max", fan_id));
        let max_rpm = read_u32(max_path).unwrap_or(5100);
        let min_path = hwmon_dir.join(format!("fan{}_min", fan_id));
        let min_rpm = read_u32(min_path).unwrap_or(1700);

        if target_rpm < min_rpm || target_rpm > max_rpm {
            return Err(LegionError::InvalidValue(format!(
                "Target RPM {} is outside safety boundary [{}-{}]",
                target_rpm, min_rpm, max_rpm
            )));
        }
    }

    write_u32(&target_path, target_rpm)
}

pub fn set_both_fans(target1: u32, target2: u32) -> Result<FanSystemStatus> {
    set_fan_target(1, target1)?;
    set_fan_target(2, target2)?;
    get_fan_status()
}

pub fn set_full_speed() -> Result<FanSystemStatus> {
    let status = get_fan_status()?;
    let f1_max = status.fans.get(0).map(|f| f.max_rpm).unwrap_or(5100);
    let f2_max = status.fans.get(1).map(|f| f.max_rpm).unwrap_or(5100);
    set_both_fans(f1_max, f2_max)
}

pub fn set_auto() -> Result<FanSystemStatus> {
    set_both_fans(0, 0)
}

pub fn set_vpc_fan_mode(mode: u32) -> Result<()> {
    let p = Path::new(VPC_FAN_MODE_PATH);
    if !p.exists() {
        return Err(LegionError::UnsupportedFeature("VPC fan_mode interface not found".into()));
    }
    write_u32(p, mode)
}
