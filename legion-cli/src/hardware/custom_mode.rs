use crate::error::{LegionError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const FW_ATTR_BASE: &str = "/sys/class/firmware-attributes/lenovo-wmi-other-0/attributes";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeInfo {
    pub name: String,
    pub display_name: String,
    pub current_value: i64,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
    pub default_value: Option<i64>,
    pub attr_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomModeConfig {
    pub supported: bool,
    pub attributes: Vec<AttributeInfo>,
}

fn read_string(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_i64(path: impl AsRef<Path>) -> Option<i64> {
    read_string(path).and_then(|s| s.parse::<i64>().ok())
}

pub fn get_custom_mode_config() -> Result<CustomModeConfig> {
    let base = Path::new(FW_ATTR_BASE);
    if !base.exists() {
        return Ok(CustomModeConfig {
            supported: false,
            attributes: Vec::new(),
        });
    }

    let mut attributes = Vec::new();
    let entries = fs::read_dir(base).map_err(|e| LegionError::Io(e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let display_name = read_string(path.join("display_name")).unwrap_or_else(|| name.clone());
        let current_value = read_i64(path.join("current_value")).unwrap_or(0);
        let min_value = read_i64(path.join("min_value"));
        let max_value = read_i64(path.join("max_value"));
        let default_value = read_i64(path.join("default_value"));
        let attr_type = read_string(path.join("type")).unwrap_or_else(|| "integer".into());

        attributes.push(AttributeInfo {
            name,
            display_name,
            current_value,
            min_value,
            max_value,
            default_value,
            attr_type,
        });
    }

    attributes.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(CustomModeConfig {
        supported: true,
        attributes,
    })
}

pub fn set_firmware_attribute(attr_name: &str, value: i64) -> Result<()> {
    let attr_dir = Path::new(FW_ATTR_BASE).join(attr_name);
    if !attr_dir.exists() {
        return Err(LegionError::UnsupportedFeature(format!(
            "Firmware attribute '{}' does not exist",
            attr_name
        )));
    }

    let min_val = read_i64(attr_dir.join("min_value"));
    let max_val = read_i64(attr_dir.join("max_value"));

    if let (Some(min), Some(max)) = (min_val, max_val) {
        if value < min || value > max {
            return Err(LegionError::InvalidValue(format!(
                "Value {} for '{}' is out of bounds [{}-{}]",
                value, attr_name, min, max
            )));
        }
    }

    let target_file = attr_dir.join("current_value");
    fs::write(&target_file, value.to_string()).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            LegionError::PermissionDenied {
                path: target_file.display().to_string(),
                action: "write",
            }
        } else {
            LegionError::Io(e)
        }
    })
}
